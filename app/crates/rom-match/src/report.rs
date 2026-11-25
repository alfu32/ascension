//! The match report (task 117): the players ranked across several aspects, their history over
//! game time and the facts of the match, read from a world. The app shows it, the server writes
//! it as `report.json` and `rom report` prints it.

use rom_save::{Controller, MatchInfo};
use rom_sim::{CityArea, EndReason, MatchEnd, Outcome, PlayerId, PlayerStatus, World};

/// An aspect the players are ranked in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Aspect {
    /// The score ([`World::score`]).
    Overall,
    /// Kills, then fewer losses.
    Military,
    /// Resources gathered, then spent.
    Economy,
    /// Cities founded and captured, then the most land held.
    Expansion,
    /// Technologies researched, then the highest age and how early it was reached.
    Technology,
    /// Ascensions.
    Ascension,
}

impl Aspect {
    /// Every aspect, in the report's column order.
    pub const ALL: [Self; 6] = [
        Self::Overall,
        Self::Military,
        Self::Economy,
        Self::Expansion,
        Self::Technology,
        Self::Ascension,
    ];

    /// The column heading.
    pub fn label(self) -> &'static str {
        match self {
            Self::Overall => "Overall",
            Self::Military => "Military",
            Self::Economy => "Economy",
            Self::Expansion => "Expansion",
            Self::Technology => "Technology",
            Self::Ascension => "Ascension",
        }
    }
}

/// How a player stands at the report's tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Its team completed the first victory.
    Won,
    /// Defeated at a tick, by the player that took its capital when one did.
    Defeated {
        /// The tick.
        tick: u64,
        /// The capturer.
        by: Option<PlayerId>,
    },
    /// Out of the material plane.
    Ascended,
    /// Still playing.
    Playing,
}

/// One player's line of the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportPlayer {
    /// The player.
    pub id: PlayerId,
    /// Its seat's name, or "Player n".
    pub name: String,
    /// Its starting nation's name.
    pub nation: String,
    /// Its race's name.
    pub race: String,
    /// Whether people play it.
    pub human: bool,
    /// Its team.
    pub team: u32,
    /// How it stands.
    pub standing: Standing,
    /// Its score.
    pub score: i64,
    /// Enemies it destroyed.
    pub kills: u64,
    /// What it lost in combat.
    pub losses: u64,
    /// Resources gathered (income and trade).
    pub gathered: i64,
    /// Resources paid for orders.
    pub spent: i64,
    /// Units trained.
    pub units_trained: u64,
    /// Buildings completed.
    pub buildings: u64,
    /// Cities founded.
    pub cities_founded: u64,
    /// Enemy cities captured.
    pub cities_captured: u64,
    /// Own cities lost.
    pub cities_lost: u64,
    /// The most tiles it held.
    pub peak_land: u64,
    /// The largest army it had.
    pub peak_army: u64,
    /// Technologies researched.
    pub techs: u64,
    /// The highest age reached: its name and when.
    pub age: (String, u64),
    /// Ordinal of that age.
    pub age_ordinal: i64,
    /// Ascensions.
    pub ascensions: u32,
    /// The name of its capital, while the capital stands (task 144).
    pub capital: Option<String>,
    /// The names of the cities it holds at the report's tick, in founding (id) order (task 144).
    pub cities: Vec<String>,
    /// Its rank in every aspect, in [`Aspect::ALL`] order: 1 for the leaders.
    pub ranks: Vec<u32>,
}

/// One player's curve of a chart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Series {
    /// The player.
    pub player: PlayerId,
    /// (tick, value) points in tick order.
    pub points: Vec<(u64, i64)>,
}

/// A chart over game time from the history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chart {
    /// Its title.
    pub title: &'static str,
    /// One curve per player.
    pub series: Vec<Series>,
}

/// The whole report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchReport {
    /// The tick the report describes.
    pub tick: u64,
    /// Simulation ticks per game second.
    pub ticks_per_second: u64,
    /// The map style's key.
    pub map_style: String,
    /// The seed.
    pub seed: u64,
    /// The setup's city area option (task 158).
    pub city_area: CityArea,
    /// The end, once reached.
    pub end: Option<MatchEnd>,
    /// The first victory: team, rule and tick.
    pub victory: Option<(u32, String, u64)>,
    /// The players, best overall score first.
    pub players: Vec<ReportPlayer>,
    /// Score, land, army and resources over time.
    pub charts: Vec<Chart>,
}

impl MatchReport {
    /// The report of `world` as it stands, named from the save's or setup's seats.
    pub fn of(world: &World, info: &MatchInfo) -> Self {
        let rules = world.rules();
        let seat = |id: PlayerId| info.seats.iter().find(|s| s.player == id);
        // Saves made before the match knew its humans say it in the seats.
        let known = world.humans().next().is_some();
        let mut players: Vec<ReportPlayer> = world
            .players()
            .filter(|p| p.id != PlayerId(0))
            .map(|p| {
                let tally = world.tally(p.id).cloned().unwrap_or_default();
                let origin = world.origin_faction(p.id).unwrap_or(p.faction);
                let (age, reached) = tally
                    .ages
                    .iter()
                    .chain([(&p.age, &world.tick())])
                    .filter_map(|(age, tick)| rules.ages.get(age).map(|a| (a, *tick)))
                    .max_by_key(|(a, tick)| (a.ordinal, std::cmp::Reverse(*tick)))
                    .expect("a player has an age");
                let standing = match world.player_status(p.id) {
                    Some(PlayerStatus::Won) => Standing::Won,
                    Some(PlayerStatus::Ascended) => Standing::Ascended,
                    Some(PlayerStatus::Defeated) => {
                        let (tick, by) = tally.defeated.unwrap_or((world.tick(), None));
                        Standing::Defeated { tick, by }
                    }
                    _ => Standing::Playing,
                };
                ReportPlayer {
                    id: p.id,
                    name: seat(p.id)
                        .map(|s| s.name.clone())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| format!("Player {}", p.id.0)),
                    nation: rules.def(origin).name.clone(),
                    race: rules.def(rules.race_of(origin)).name.clone(),
                    human: if known {
                        world.is_human(p.id)
                    } else {
                        seat(p.id).is_some_and(|s| {
                            matches!(s.controller, Controller::Human | Controller::Remote)
                        })
                    },
                    team: p.team,
                    standing,
                    score: world.score(p.id),
                    kills: world.stats().kills.get(&p.id).copied().unwrap_or(0),
                    losses: world.stats().losses.get(&p.id).copied().unwrap_or(0),
                    gathered: tally.gathered_total(),
                    spent: tally.spent,
                    units_trained: tally.units_trained,
                    buildings: tally.buildings_completed,
                    cities_founded: tally.cities_founded,
                    cities_captured: tally.cities_captured,
                    cities_lost: tally.cities_lost,
                    peak_land: tally.peak_land,
                    peak_army: tally.peak_army,
                    techs: tally.techs_researched,
                    age: (rules.def(age.id).name.clone(), reached),
                    age_ordinal: age.ordinal,
                    ascensions: world.ascensions(p.id),
                    capital: world
                        .capital(p.id)
                        .and_then(|c| world.city_name(c))
                        .map(str::to_owned),
                    cities: world
                        .city_names()
                        .filter(|(id, _)| world.entity(*id).is_some_and(|e| e.owner == p.id))
                        .map(|(_, name)| name.to_owned())
                        .collect(),
                    ranks: Vec::new(),
                }
            })
            .collect();
        let keys: Vec<Vec<[i128; 3]>> = Aspect::ALL
            .iter()
            .map(|aspect| players.iter().map(|p| key(*aspect, p)).collect())
            .collect();
        for (index, player) in players.iter_mut().enumerate() {
            player.ranks = keys
                .iter()
                .map(|column| 1 + column.iter().filter(|k| **k > column[index]).count() as u32)
                .collect();
        }
        players.sort_by_key(|p| (p.ranks[0], p.id));
        let victory = match world.outcome() {
            Outcome::Won { team, rule, tick } => {
                Some((*team, rom_sim::victory_rule_key(*rule).to_owned(), *tick))
            }
            Outcome::Playing => None,
        };
        let chart = |title, value: fn(&rom_sim::HistorySample) -> i64| Chart {
            title,
            series: players
                .iter()
                .map(|p| Series {
                    player: p.id,
                    points: world
                        .history()
                        .iter()
                        .filter(|s| s.player == p.id)
                        .map(|s| (s.tick, value(s)))
                        .collect(),
                })
                .collect(),
        };
        let charts = vec![
            chart("Score", |s| s.score),
            chart("Land (tiles)", |s| s.land as i64),
            chart("Army", |s| s.army as i64),
            chart("Resources in store", |s| s.stock),
        ];
        Self {
            tick: world.tick(),
            ticks_per_second: rules.constant("sim.ticks_per_second").unwrap_or(20).max(1) as u64,
            map_style: info.map_style.clone(),
            seed: info.seed,
            city_area: world.match_options().city_area,
            end: world.match_end(),
            victory,
            players,
            charts,
        }
    }

    /// Whether `p` leads the aspect of [`Aspect::ALL`] at `index`: ranked first while someone
    /// ranks below it (nobody leads when everyone is equal).
    pub fn leads(&self, p: &ReportPlayer, index: usize) -> bool {
        p.ranks[index] == 1 && self.players.iter().any(|q| q.ranks[index] > 1)
    }

    /// The map style's key without its prefix, in words.
    pub fn map_style_name(&self) -> String {
        match self.map_style.rsplit('.').next() {
            Some(name) if !name.is_empty() => name.replace('_', " "),
            _ => "default".to_owned(),
        }
    }

    /// The city area option in words: "limits building" or "building anywhere in the borders".
    pub fn city_area_text(&self) -> &'static str {
        match self.city_area {
            CityArea::LimitsBuilding => "limits building",
            CityArea::Free => "building anywhere in the borders",
        }
    }

    /// A player's line.
    pub fn player(&self, id: PlayerId) -> Option<&ReportPlayer> {
        self.players.iter().find(|p| p.id == id)
    }

    /// Game ticks as hours, minutes and seconds: "1:02:05" or "02:05".
    pub fn clock(&self, ticks: u64) -> String {
        let seconds = ticks / self.ticks_per_second.max(1);
        let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m:02}:{s:02}")
        }
    }

    /// The length of the match: to its end, or to the report's tick.
    pub fn length(&self) -> u64 {
        self.end.map_or(self.tick, |end| end.tick)
    }

    /// One line on why the match ended, for `me` (a player of the report, or nobody).
    pub fn end_line(&self, me: Option<PlayerId>) -> Option<String> {
        let end = self.end?;
        let mine = me.and_then(|id| self.player(id));
        Some(match end.reason {
            EndReason::NoHumanLeft => match mine {
                Some(p) if matches!(p.standing, Standing::Defeated { .. }) => match &p.standing {
                    Standing::Defeated { by: Some(by), .. } => format!(
                        "You were defeated by {}.",
                        self.player(*by)
                            .map_or_else(|| "an enemy".to_owned(), ReportPlayer::label)
                    ),
                    _ => "You were defeated.".to_owned(),
                },
                _ => "No human player is left.".to_owned(),
            },
            EndReason::LastStanding { team } => match mine {
                Some(p) if p.team == team => "Your team is the last standing.".to_owned(),
                _ => format!("Team {team} is the last standing."),
            },
        })
    }

    /// Who won, for the dialog: the first victory's team, or the last standing team.
    pub fn winner_line(&self) -> Option<String> {
        let team = match (&self.victory, self.end.map(|e| e.reason)) {
            (Some((team, rule, tick)), _) => {
                return Some(format!(
                    "{} won by {} at {}.",
                    self.team_names(*team),
                    rule.replace('_', " "),
                    self.clock(*tick)
                ));
            }
            (None, Some(EndReason::LastStanding { team })) => team,
            _ => return None,
        };
        Some(format!("{} remain.", self.team_names(team)))
    }

    fn team_names(&self, team: u32) -> String {
        let names: Vec<String> = self
            .players
            .iter()
            .filter(|p| p.team == team)
            .map(ReportPlayer::label)
            .collect();
        names.join(", ")
    }

    /// The standing of a player as a word or two.
    pub fn standing_text(&self, p: &ReportPlayer) -> String {
        match &p.standing {
            Standing::Won => "won".to_owned(),
            Standing::Defeated { tick, .. } => format!("defeated at {}", self.clock(*tick)),
            Standing::Ascended => "ascended".to_owned(),
            Standing::Playing => "playing".to_owned(),
        }
    }

    /// The end reason as text.
    pub fn end_text(&self) -> String {
        match self.end.map(|e| e.reason) {
            None => "not ended".to_owned(),
            Some(EndReason::NoHumanLeft) => "no human player is left".to_owned(),
            Some(EndReason::LastStanding { team }) => format!("team {team} is the last standing"),
        }
    }

    /// What a cell of the ranking table shows for an aspect.
    pub fn cell(&self, p: &ReportPlayer, aspect: Aspect) -> String {
        match aspect {
            Aspect::Overall => p.score.to_string(),
            Aspect::Military => format!("{} kills, {} lost", p.kills, p.losses),
            Aspect::Economy => format!("{} gathered, {} spent", p.gathered, p.spent),
            Aspect::Expansion => format!(
                "{} cities, {} land",
                p.cities_founded + p.cities_captured,
                p.peak_land
            ),
            Aspect::Technology => {
                format!("{} techs, {} at {}", p.techs, p.age.0, self.clock(p.age.1))
            }
            Aspect::Ascension => p.ascensions.to_string(),
        }
    }

    /// The ranking table and the facts as text (`rom report`).
    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(out, "match report at {}", self.clock(self.tick));
        let _ = writeln!(
            out,
            "map style {} · seed {} · city area: {} · length {} · end: {}",
            self.map_style_name(),
            self.seed,
            self.city_area_text(),
            self.clock(self.length()),
            self.end_text()
        );
        let _ = writeln!(
            out,
            "first victory: {}",
            self.victory.as_ref().map_or_else(
                || "none".to_owned(),
                |(team, rule, tick)| format!("team {team}, {rule}, at {}", self.clock(*tick))
            )
        );
        let mut rows = vec![{
            let mut head = vec![
                "player".to_owned(),
                "race".to_owned(),
                "who".to_owned(),
                "standing".to_owned(),
            ];
            head.extend(Aspect::ALL.iter().map(|a| a.label().to_owned()));
            head
        }];
        for p in &self.players {
            let mut row = vec![
                format!("{} ({})", p.name, p.nation),
                p.race.clone(),
                if p.human { "human" } else { "computer" }.to_owned(),
                self.standing_text(p),
            ];
            for (i, aspect) in Aspect::ALL.iter().enumerate() {
                let rank = p.ranks[i];
                let lead = if self.leads(p, i) { "*" } else { "" };
                row.push(format!("{lead}{} {}", ordinal(rank), self.cell(p, *aspect)));
            }
            rows.push(row);
        }
        let widths: Vec<usize> = (0..rows[0].len())
            .map(|c| rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0))
            .collect();
        for row in rows {
            let line: Vec<String> = row
                .iter()
                .zip(&widths)
                .map(|(cell, w)| format!("{cell:<w$}"))
                .collect();
            let _ = writeln!(out, "{}", line.join(" | ").trim_end());
        }
        let _ = writeln!(out, "cities:");
        for p in &self.players {
            let _ = writeln!(out, "  {}: {}", p.label(), self.cities_text(p));
        }
        out
    }

    /// The cities a player holds, its capital first and marked: "Roma (capital), Ostia"; "none".
    pub fn cities_text(&self, p: &ReportPlayer) -> String {
        let capital = p.capital.as_ref().filter(|c| p.cities.contains(c));
        let mut names: Vec<String> = capital
            .map(|c| format!("{c} (capital)"))
            .into_iter()
            .collect();
        names.extend(p.cities.iter().filter(|c| Some(*c) != capital).cloned());
        if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(", ")
        }
    }
}

impl ReportPlayer {
    /// "name (nation)".
    pub fn label(&self) -> String {
        format!("{} ({})", self.name, self.nation)
    }
}

/// 1st, 2nd, 3rd, 4th … 11th, 12th, 13th, 21st.
pub fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// What ranks a player in an aspect: bigger is better, compared in order.
fn key(aspect: Aspect, p: &ReportPlayer) -> [i128; 3] {
    let n = |v: u64| i128::from(v);
    match aspect {
        Aspect::Overall => [i128::from(p.score), 0, 0],
        Aspect::Military => [n(p.kills), -n(p.losses), 0],
        Aspect::Economy => [i128::from(p.gathered), i128::from(p.spent), 0],
        Aspect::Expansion => [n(p.cities_founded + p.cities_captured), n(p.peak_land), 0],
        Aspect::Technology => [n(p.techs), i128::from(p.age_ordinal), -n(p.age.1)],
        Aspect::Ascension => [i128::from(p.ascensions), 0, 0],
    }
}

#[cfg(test)]
mod tests {
    use super::ordinal;

    #[test]
    fn ordinals_read_as_english() {
        let words: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 103, 111]
            .map(ordinal)
            .into();
        assert_eq!(
            words,
            [
                "1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "103rd",
                "111th"
            ]
        );
    }
}

//! The live rankings and the age on the top bar (task 160): every player's colour, name, age,
//! score and the parts of its score, read from the score the match report uses
//! ([`World::score`], [`World::score_parts`]). Only public facts: what the report shows.

use std::sync::Mutex;

use rom_render::{HudRanking, RankStanding};
use rom_sim::{PlayerId, PlayerStatus, World};

use crate::ascension_view::{colour_of, player_name};

/// The rankings, worked out once per game second and kept between the HUD's publications.
#[derive(Debug, Default)]
pub struct RankingBoard {
    /// The game second the rows are of, and the rows.
    rows: Mutex<Option<(u64, Vec<HudRanking>)>>,
}

impl RankingBoard {
    /// The rows for `me` as of the current game second: worked out again only when the second
    /// changed since the last call.
    pub fn rows(&self, world: &World, me: PlayerId) -> Vec<HudRanking> {
        let second = world.tick() / u64::from(crate::sim_thread::TICKS_PER_SECOND);
        let Ok(mut slot) = self.rows.lock() else {
            return rankings(world, me);
        };
        match &*slot {
            Some((at, rows)) if *at == second => rows.clone(),
            _ => {
                let rows = rankings(world, me);
                *slot = Some((second, rows.clone()));
                rows
            }
        }
    }
}

/// Every player but nature, best score first (ties by id): colour, name, age, score and the
/// military, economy and research parts as shares of the best player's part.
pub fn rankings(world: &World, me: PlayerId) -> Vec<HudRanking> {
    let rules = world.rules();
    let mut players: Vec<_> = world
        .players()
        .filter(|p| p.id != PlayerId(0))
        .map(|p| (p, world.score(p.id), world.score_parts(p.id)))
        .collect();
    players.sort_by_key(|(p, score, _)| (std::cmp::Reverse(*score), p.id));
    let best = |part: fn(&rom_sim::ScoreParts) -> i64| {
        players
            .iter()
            .map(|(_, _, parts)| part(parts))
            .max()
            .unwrap_or(0)
            .max(1)
    };
    let bests = [
        best(|s| s.military),
        best(|s| s.economy),
        best(|s| s.research),
    ];
    let share = |value: i64, best: i64| {
        (i128::from(value.max(0)) * 1000 / i128::from(best)).clamp(0, 1000) as u16
    };
    players
        .iter()
        .map(|(p, score, parts)| {
            let ordinal = rules.ages.get(&p.age).map_or(0, |a| a.ordinal);
            HudRanking {
                colour: colour_of(p.id),
                name: player_name(world, me, p.id),
                age: crate::ages::age(ordinal)
                    .map_or_else(|| rules.def(p.age).name.clone(), |a| a.name.clone()),
                score: thousands(*score),
                bars: [
                    share(parts.military, bests[0]),
                    share(parts.economy, bests[1]),
                    share(parts.research, bests[2]),
                ],
                me: p.id == me,
                standing: match world.player_status(p.id) {
                    Some(PlayerStatus::Defeated) => RankStanding::Defeated,
                    Some(PlayerStatus::Ascended) => RankStanding::Ascended,
                    _ => RankStanding::Playing,
                },
            }
        })
        .collect()
}

/// The person's age as the top bar names it: "AGE 10 OF 15 · FUSION AGE", counted from one
/// as the new-age dialog counts.
pub fn age_label(world: &World, me: PlayerId) -> String {
    let rules = world.rules();
    let Some(age) = world
        .players()
        .find(|p| p.id == me)
        .and_then(|p| rules.ages.get(&p.age))
    else {
        return String::new();
    };
    let last = rules.ages.values().map(|a| a.ordinal).max().unwrap_or(0);
    format!(
        "AGE {} OF {}  ·  {}",
        age.ordinal + 1,
        last + 1,
        rules.def(age.id).name.to_uppercase()
    )
}

/// A score with thousands separators: "12,340".
fn thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut text = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        text.push('-');
    }
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            text.push(',');
        }
        text.push(ch);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_sim::{MapGrid, Player, Pos};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn world(players: u32) -> World {
        let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
        let id = |key: &str| rules.id_of(key).expect("definition");
        let players = (1..=players)
            .map(|n| Player {
                id: PlayerId(n),
                team: n,
                age: id("age.ancient"),
                faction: id("nation.romans"),
                resources: rules.resources.keys().map(|id| (*id, 10000)).collect(),
                techs: BTreeMap::new(),
            })
            .collect();
        let map = MapGrid::new(96, 96, vec![id("ter.plain"); 96 * 96]).expect("map");
        World::new(rules.clone(), map, 1, players).expect("world")
    }

    #[test]
    fn every_player_is_ranked_by_the_report_score_with_its_parts() {
        let mut w = world(4);
        let id = |w: &World, key: &str| w.rules().id_of(key).unwrap();
        for (n, x) in [(1u32, 10_000), (2, 40_000), (3, 70_000), (4, 40_000)] {
            let y = if n == 4 { 80_000 } else { 20_000 };
            w.spawn(id(&w, "bld.city"), PlayerId(n), Pos::flat(x, y))
                .unwrap();
        }
        for _ in 0..40 {
            w.step();
        }
        let rows = rankings(&w, PlayerId(2));
        assert_eq!(rows.len(), 4);
        let scores: Vec<i64> = rows
            .iter()
            .map(|r| r.score.replace(',', "").parse().unwrap())
            .collect();
        assert!(scores.windows(2).all(|p| p[0] >= p[1]), "{scores:?}");
        let mut expected: Vec<i64> = w
            .players()
            .filter(|p| p.id.0 > 0)
            .map(|p| w.score(p.id))
            .collect();
        expected.sort_by_key(|s| std::cmp::Reverse(*s));
        assert_eq!(scores, expected);
        assert_eq!(rows.iter().filter(|r| r.me).count(), 1);
        assert_eq!(rows.iter().find(|r| r.me).unwrap().name, "You");
        assert!(rows.iter().all(|r| r.age == "Ancient"));
        // Every bar is a share of the best part: the best player's part is full.
        let parts: Vec<_> = w
            .players()
            .filter(|p| p.id.0 > 0)
            .map(|p| w.score_parts(p.id))
            .collect();
        for (n, part) in [
            |s: &rom_sim::ScoreParts| s.military,
            |s: &rom_sim::ScoreParts| s.economy,
            |s: &rom_sim::ScoreParts| s.research,
        ]
        .into_iter()
        .enumerate()
        {
            assert!(rows.iter().all(|r| r.bars[n] <= 1000));
            if parts.iter().any(|p| part(p) > 0) {
                assert_eq!(rows.iter().map(|r| r.bars[n]).max(), Some(1000));
            }
        }
        assert!(rows.iter().all(|r| r.standing == RankStanding::Playing));
        // The board keeps one game second's rows.
        let board = RankingBoard::default();
        assert_eq!(board.rows(&w, PlayerId(2)), rows);
        assert_eq!(board.rows(&w, PlayerId(2)), rows);
    }

    #[test]
    fn the_top_bar_names_the_age_counted_from_one() {
        let w = world(1);
        let ancient = w
            .rules()
            .def(w.rules().id_of("age.ancient").unwrap())
            .name
            .to_uppercase();
        let count = w.rules().ages.len();
        assert_eq!(
            age_label(&w, PlayerId(1)),
            format!("AGE 1 OF {count}  ·  {ancient}")
        );
        assert_eq!(age_label(&w, PlayerId(9)), "");
    }

    #[test]
    fn scores_read_with_thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(12_345_678), "12,345,678");
        assert_eq!(thousands(-4_500), "-4,500");
    }
}

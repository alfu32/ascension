//! Match options, player defeat and team victory timers.

use crate::{PlayerId, SimError, World};
use rom_rules::{DefId, EntityClass, Rules, VictoryRule};
use std::collections::{BTreeMap, BTreeSet};

/// The match's capital or total-conquest elimination rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Elimination {
    /// Lose only after the last city and unit are gone.
    Conquest,
    /// Lose after an uninterrupted enemy capital occupation.
    #[default]
    Capital,
    /// Lose as soon as an enemy captures the capital.
    SuddenDeath,
}

/// Match settings supplied during world construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchOptions {
    /// The elimination choice from the match setup.
    pub elimination: Elimination,
    /// Enabled rule families; their thresholds and durations come from victory rows.
    pub victories: BTreeSet<VictoryRule>,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            elimination: Elimination::Capital,
            victories: [VictoryRule::Conquest, VictoryRule::Territory].into(),
        }
    }
}

/// Current participation status, separate from immutable player setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    /// Still participating in the match.
    Playing,
    /// Eliminated and unable to issue orders.
    Defeated,
    /// Member of the winning team.
    Won,
}

/// The first completed victory, retained while subsequent ticks continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// No enabled victory has completed.
    Playing,
    /// A team completed this rule at the supplied simulation tick.
    Won {
        /// Winning team number.
        team: u32,
        /// Rule family from the victory row.
        rule: VictoryRule,
        /// Completed simulation tick.
        tick: u64,
    },
}

#[derive(Debug)]
pub(crate) struct VictoryState {
    pub options: MatchOptions,
    pub statuses: BTreeMap<PlayerId, PlayerStatus>,
    pub timers: BTreeMap<(DefId, u32), u64>,
    pub outcome: Outcome,
    /// Ground tiles eligible for the territory victory, across all surfaces.
    pub ownable_tiles: u64,
    pub land_owned: BTreeMap<PlayerId, u64>,
}

impl VictoryState {
    pub fn new(
        rules: &Rules,
        players: &BTreeMap<PlayerId, crate::Player>,
    ) -> Result<Self, SimError> {
        let elimination = match rules
            .match_options
            .get("elimination")
            .map(|row| row.default_value.as_str())
        {
            Some("conquest") => Elimination::Conquest,
            Some("capital") => Elimination::Capital,
            Some("sudden_death") => Elimination::SuddenDeath,
            _ => return Err(SimError::InvalidOptions),
        };
        Ok(Self {
            options: MatchOptions {
                elimination,
                ..MatchOptions::default()
            },
            statuses: players
                .keys()
                .map(|id| (*id, PlayerStatus::Playing))
                .collect(),
            timers: BTreeMap::new(),
            outcome: Outcome::Playing,
            ownable_tiles: 0,
            land_owned: BTreeMap::new(),
        })
    }
}

impl World {
    /// Selects match settings during construction, before any tick or accepted order.
    /// Works with flat, spherical and universe constructors alike.
    pub fn with_match_options(mut self, options: MatchOptions) -> Result<Self, SimError> {
        if self.tick != 0
            || !self.pending.is_empty()
            || options.victories.iter().any(|rule| {
                !matches!(rule, VictoryRule::Conquest | VictoryRule::Territory)
                    || !self.rules.victories.values().any(|row| row.rule == *rule)
            })
        {
            return Err(SimError::InvalidOptions);
        }
        self.victory.options = options;
        Ok(self)
    }

    /// Settings selected when the world was built.
    pub fn match_options(&self) -> &MatchOptions {
        &self.victory.options
    }

    /// Current status in player id order; neutral player 0 never participates in victory.
    pub fn player_status(&self, player: PlayerId) -> Option<PlayerStatus> {
        self.victory.statuses.get(&player).copied()
    }

    /// The first victory, or Playing while the match is undecided.
    pub fn outcome(&self) -> &Outcome {
        &self.victory.outcome
    }

    /// Active territory timers keyed by victory definition and team, storing end ticks.
    pub fn victory_timers(&self) -> &BTreeMap<(DefId, u32), u64> {
        &self.victory.timers
    }

    fn defeat(&mut self, player: PlayerId, capturer: Option<PlayerId>) {
        self.victory.statuses.insert(player, PlayerStatus::Defeated);
        let entities: Vec<_> = self
            .entities()
            .filter(|e| e.owner == player)
            .map(|e| e.id)
            .collect();
        for id in entities {
            if let Some(owner) = capturer {
                self.change_owner(id, owner);
            } else {
                self.cancel_entity_orders(id);
            }
        }
        // Ships between surfaces are still this player's units and change hands too.
        let pct = self
            .rules
            .constant("economy.cancel_refund_pct")
            .expect("refund");
        for transit in self
            .space
            .transits
            .values_mut()
            .filter(|t| t.entity.owner == player)
        {
            let e = &mut transit.entity;
            if let Some(owner) = capturer {
                e.owner = owner;
            }
            for order in std::mem::take(&mut e.orders) {
                self.stats.orders_cancelled += 1;
                if matches!(order.kind.as_str(), "train" | "research") {
                    for (resource, amount) in order.payment {
                        let balance = self
                            .players
                            .get_mut(&player)
                            .expect("defeated owner")
                            .resources
                            .entry(resource)
                            .or_default();
                        *balance = balance
                            .saturating_add((i128::from(amount) * i128::from(pct) / 100) as i64);
                    }
                }
            }
            let max = self
                .rules
                .entity_stat(
                    "hp",
                    e.type_id,
                    self.rules.entity_types[&e.type_id].hp,
                    &self.players[&e.owner].active_sources(),
                    &rom_rules::Context::default(),
                )
                .max(1);
            e.hp = (i128::from(e.hp) * i128::from(max) / i128::from(e.max_hp.max(1))) as i64;
            e.max_hp = max;
        }
    }

    fn eliminate_players(&mut self) {
        let mut defeated = Vec::new();
        for player in self.players.values() {
            if player.id == PlayerId(0)
                || self.player_status(player.id) != Some(PlayerStatus::Playing)
            {
                continue;
            }
            if self.victory.options.elimination == Elimination::Conquest {
                let has_force = self
                    .entities()
                    .chain(self.transits().map(|t| &t.entity))
                    .any(|e| {
                        e.owner == player.id
                            && (self.is_city_type(e.type_id)
                                || self.rules.entity_types[&e.type_id].class == EntityClass::Unit)
                    });
                if !has_force {
                    defeated.push((player.id, None));
                }
                continue;
            }
            let capturer = self
                .capital(player.id)
                .and_then(|id| self.entity(id))
                .filter(|capital| self.enemies(player.id, capital.owner))
                .map(|e| e.owner);
            let Some(capturer) = capturer else {
                self.capture.holds.remove(&player.id);
                continue;
            };
            let team = self.players[&capturer].team;
            let hold = self
                .capture
                .holds
                .entry(player.id)
                .or_insert((team, self.tick));
            if hold.0 != team {
                *hold = (team, self.tick);
            }
            let duration = self
                .rules
                .constant("victory.capital_hold_ticks")
                .expect("capital hold") as u64;
            if self.victory.options.elimination == Elimination::SuddenDeath
                || self.tick.saturating_sub(hold.1) >= duration
            {
                defeated.push((player.id, Some(capturer)));
            }
        }
        for (player, capturer) in defeated {
            self.defeat(player, capturer);
        }
    }

    fn win(&mut self, team: u32, rule: VictoryRule) {
        self.victory.outcome = Outcome::Won {
            team,
            rule,
            tick: self.tick,
        };
        for player in self
            .players
            .values()
            .filter(|p| p.id != PlayerId(0) && p.team == team)
        {
            if self.victory.statuses[&player.id] == PlayerStatus::Playing {
                self.victory.statuses.insert(player.id, PlayerStatus::Won);
            }
        }
    }

    pub(crate) fn update_victory(&mut self) {
        if self.victory.outcome != Outcome::Playing {
            return;
        }
        self.eliminate_players();
        // Solo benchmarks and cooperative scenarios have no opposing team to defeat.
        let initial: BTreeSet<_> = self
            .players
            .values()
            .filter(|p| p.id != PlayerId(0))
            .map(|p| p.team)
            .collect();
        let alive: BTreeSet<_> = self
            .players
            .values()
            .filter(|p| {
                p.id != PlayerId(0) && self.player_status(p.id) == Some(PlayerStatus::Playing)
            })
            .map(|p| p.team)
            .collect();
        let rows: Vec<_> = self
            .rules
            .victories
            .values()
            .filter(|row| self.victory.options.victories.contains(&row.rule))
            .cloned()
            .collect();
        for row in rows {
            match row.rule {
                VictoryRule::Conquest if initial.len() > 1 && alive.len() == 1 => {
                    self.win(*alive.first().expect("one team"), row.rule);
                    return;
                }
                VictoryRule::Territory => {
                    let mut shares: BTreeMap<u32, u64> = BTreeMap::new();
                    for (player, tiles) in &self.victory.land_owned {
                        if *player != PlayerId(0)
                            && alive.contains(&self.players[player].team)
                            && self.player_status(*player) != Some(PlayerStatus::Defeated)
                        {
                            *shares.entry(self.players[player].team).or_default() += tiles;
                        }
                    }
                    self.victory.timers.retain(|(victory, team), _| {
                        *victory != row.id
                            || shares.get(team).is_some_and(|tiles| {
                                self.victory.ownable_tiles > 0
                                    && i128::from(*tiles) * 100
                                        >= i128::from(self.victory.ownable_tiles)
                                            * i128::from(row.threshold)
                            })
                    });
                    for (team, tiles) in shares {
                        if self.victory.ownable_tiles == 0
                            || i128::from(tiles) * 100
                                < i128::from(self.victory.ownable_tiles) * i128::from(row.threshold)
                        {
                            continue;
                        }
                        let end = self
                            .victory
                            .timers
                            .entry((row.id, team))
                            .or_insert_with(|| {
                                self.tick.saturating_add(row.timer_ticks.max(0) as u64)
                            });
                        if self.tick >= *end {
                            self.win(team, row.rule);
                            return;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

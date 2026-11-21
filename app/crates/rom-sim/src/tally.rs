//! Per-player statistics and their history for the match report (task 117): counters updated
//! where the event happens, peaks and ages after every tick, and one sample per player every
//! `stats.history_period_ticks`. Integers only, in player id order; hashed and saved.

use crate::{PlayerId, PlayerStatus, World};
use rom_rules::{DefId, EntityClass};
use std::collections::BTreeMap;

/// What one player did in the match.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlayerTally {
    /// Resources credited by income and trade deliveries, per resource.
    pub gathered: BTreeMap<DefId, i64>,
    /// Resources paid for orders when they were given (refunds are not subtracted).
    pub spent: i64,
    /// Units that left a producer.
    pub units_trained: u64,
    /// Building sites finished, cities included.
    pub buildings_completed: u64,
    /// Cities among them.
    pub cities_founded: u64,
    /// Enemy cities taken.
    pub cities_captured: u64,
    /// Own cities taken by an enemy.
    pub cities_lost: u64,
    /// Research orders completed (the ascension project included).
    pub techs_researched: u64,
    /// The first tick each age was held; the starting age at tick 0.
    pub ages: BTreeMap<DefId, u64>,
    /// The most tiles owned at the end of any tick.
    pub peak_land: u64,
    /// The largest army at the end of any tick.
    pub peak_army: u64,
    /// The tick the player was defeated, and by whom when its capital was taken.
    pub defeated: Option<(u64, Option<PlayerId>)>,
}

impl PlayerTally {
    /// Everything gathered, all resources together.
    pub fn gathered_total(&self) -> i64 {
        self.gathered
            .values()
            .fold(0i64, |sum, amount| sum.saturating_add(*amount))
    }
}

/// One player's standing at a sampled tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySample {
    /// The tick sampled (a multiple of the period).
    pub tick: u64,
    /// The player.
    pub player: PlayerId,
    /// Its score then ([`World::score`]).
    pub score: i64,
    /// Tiles it owned.
    pub land: u64,
    /// Its army ([`World::army_size`]).
    pub army: u64,
    /// Its cities.
    pub cities: u64,
    /// The sum of its resource stores.
    pub stock: i64,
    /// Its age.
    pub age: DefId,
}

/// The counters of every player and the sampled history.
#[derive(Debug, Default)]
pub(crate) struct TallyState {
    pub players: BTreeMap<PlayerId, PlayerTally>,
    pub history: Vec<HistorySample>,
}

impl World {
    /// What `player` did in the match so far.
    pub fn tally(&self, player: PlayerId) -> Option<&PlayerTally> {
        self.tally.players.get(&player)
    }

    /// Every sample taken, in tick then player order.
    pub fn history(&self) -> &[HistorySample] {
        &self.tally.history
    }

    /// A counter to change, for a player other than nature.
    pub(crate) fn tally_mut(&mut self, player: PlayerId) -> Option<&mut PlayerTally> {
        (player != PlayerId(0) && self.players.contains_key(&player))
            .then(|| self.tally.players.entry(player).or_default())
    }

    /// Counts `amount` of `resource` gathered by `player` (income and trade).
    pub(crate) fn count_gathered(&mut self, player: PlayerId, resource: DefId, amount: i64) {
        if amount > 0
            && let Some(t) = self.tally_mut(player)
        {
            let total = t.gathered.entry(resource).or_default();
            *total = total.saturating_add(amount);
        }
    }

    /// Whether units of `type_id` are fighters: units with a weapon that neither build nor
    /// gather.
    pub fn is_army_type(&self, type_id: DefId) -> bool {
        self.rules.entity_types.get(&type_id).is_some_and(|et| {
            et.class == EntityClass::Unit
                && !et.weapons.is_empty()
                && et.builder.is_none()
                && et.harvester.is_empty()
        })
    }

    /// How many fighters `player` has, ships in transit included.
    pub fn army_size(&self, player: PlayerId) -> u64 {
        self.types
            .of(player)
            .filter(|(type_id, _)| self.is_army_type(*type_id))
            .map(|(_, ids)| ids.len() as u64)
            .sum()
    }

    /// The match score of `player`: a weighted sum of its counters with the `score.*`
    /// constants as weights (`docs/ascension.md`, "How a match ends"): gathered resources per
    /// `score.gathered_per`, units trained, buildings completed, cities founded and
    /// captured, kills, technologies, the ordinal of the highest age reached and ascensions.
    pub fn score(&self, player: PlayerId) -> i64 {
        let weight = |key: &str| self.rules.constant(key).unwrap_or(0).max(0);
        let Some(t) = self.tally(player) else {
            return 0;
        };
        let age = t
            .ages
            .keys()
            .chain(self.players.get(&player).map(|p| &p.age))
            .filter_map(|age| self.rules.ages.get(age))
            .map(|age| age.ordinal)
            .max()
            .unwrap_or(0);
        let kills = self.stats.kills.get(&player).copied().unwrap_or(0);
        let per = weight("score.gathered_per").max(1);
        let terms: [(i128, i64); 8] = [
            (i128::from(t.units_trained), weight("score.unit_trained")),
            (
                i128::from(t.buildings_completed),
                weight("score.building_completed"),
            ),
            (i128::from(t.cities_founded), weight("score.city_founded")),
            (i128::from(t.cities_captured), weight("score.city_captured")),
            (i128::from(kills), weight("score.kill")),
            (i128::from(t.techs_researched), weight("score.tech")),
            (i128::from(age), weight("score.age")),
            (
                i128::from(self.ascensions(player)),
                weight("score.ascension"),
            ),
        ];
        let sum = terms
            .iter()
            .fold(i128::from(t.gathered_total() / per), |sum, (n, w)| {
                sum + n * i128::from(*w)
            });
        sum.clamp(0, i128::from(i64::MAX)) as i64
    }

    /// The ages the players hold now, where first held; before the first tick, the ages the
    /// match starts in.
    pub(crate) fn record_ages(&mut self) {
        let tick = self.tick;
        for p in self.players.values().filter(|p| p.id != PlayerId(0)) {
            if self.victory.statuses.get(&p.id) != Some(&PlayerStatus::Ascended) {
                let t = self.tally.players.entry(p.id).or_default();
                t.ages.entry(p.age).or_insert(tick);
            }
        }
    }

    /// After every tick: the ages held, the peaks, and every period a sample per player.
    pub(crate) fn record_tally(&mut self) {
        self.record_ages();
        let tick = self.tick;
        let players: Vec<(PlayerId, DefId)> = self
            .players
            .values()
            .filter(|p| p.id != PlayerId(0))
            .map(|p| (p.id, p.age))
            .collect();
        for (player, _) in &players {
            let land = self.territory.owned.get(player).copied().unwrap_or(0);
            let army = self.army_size(*player);
            let t = self.tally.players.entry(*player).or_default();
            t.peak_land = t.peak_land.max(land);
            t.peak_army = t.peak_army.max(army);
        }
        let period = self
            .rules
            .constant("stats.history_period_ticks")
            .unwrap_or(0);
        if period <= 0 || tick % period as u64 != 0 {
            return;
        }
        for (player, age) in players {
            let sample = HistorySample {
                tick,
                player,
                score: self.score(player),
                land: self.territory.owned.get(&player).copied().unwrap_or(0),
                army: self.army_size(player),
                cities: self.cities(player).len() as u64,
                stock: self.players[&player]
                    .resources
                    .values()
                    .fold(0i64, |sum, amount| sum.saturating_add(*amount)),
                age,
            };
            self.tally.history.push(sample);
        }
    }
}

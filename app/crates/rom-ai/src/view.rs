//! Perception: what the player has and what it is allowed to know, gathered once per
//! thought through the simulation's public queries.

use crate::{catalogue::Catalogue, memory::EnemySighting};
use rom_rules::{ActiveSources, DefId, EntityClass, Owned, ProductionMode, RampMode, Rules};
use rom_sim::{Entity, EntityId, PlayerId, Pos, Sighting, World};
use std::collections::{BTreeMap, BTreeSet};

/// Side of a threat-map cell in milli-tiles (8×8 tiles).
pub(crate) const CELL: i32 = 8000;
/// Ticks after which an enemy unit that was not seen again is forgotten.
pub(crate) const UNIT_MEMORY_TICKS: u64 = 2400;

/// One own city.
#[derive(Debug, Clone, Copy)]
pub(crate) struct City {
    pub id: EntityId,
    pub pos: Pos,
    /// Built, standing and not being assimilated.
    pub complete: bool,
    /// Economic radius in milli-tiles.
    pub radius: i64,
    /// Border radius of the city type in milli-tiles.
    pub border: i64,
}

/// A coarse map cell: body, face and cell coordinates.
pub(crate) type Cell = (u32, u8, i32, i32);

/// The cell a position falls in.
pub(crate) fn cell_of(pos: Pos) -> Cell {
    (
        pos.body.0,
        pos.face,
        pos.x.div_euclid(CELL),
        pos.y.div_euclid(CELL),
    )
}

/// The player's own state and the public facts around it at one tick.
pub(crate) struct View<'a> {
    pub world: &'a World,
    pub rules: &'a Rules,
    pub cat: &'a Catalogue,
    pub me: PlayerId,
    pub tick: u64,
    pub faction: DefId,
    pub age: DefId,
    pub active: ActiveSources,
    pub owned: Owned,
    /// Resource balances, reduced as the plan spends them.
    pub purse: BTreeMap<DefId, i64>,
    pub pop_used: i64,
    pub pop_cap: i64,
    /// Population of units still in training queues.
    pub queued_pop: i64,
    pub cities: Vec<City>,
    /// Own entities in id order.
    pub own: Vec<&'a Entity>,
    /// Own entities plus queued trainings, by type.
    pub have: BTreeMap<DefId, i64>,
    /// The same, by the first type of the upgrade line.
    pub line_have: BTreeMap<DefId, i64>,
    /// Technologies in a research queue.
    pub researching: BTreeSet<DefId>,
    /// Orders in each own producer's queue.
    pub queue_len: BTreeMap<EntityId, usize>,
    /// Unfinished own buildings a builder is assigned to.
    pub tended: BTreeSet<EntityId>,
    /// Workers walking to each gather site.
    pub inbound: BTreeMap<EntityId, i64>,
    /// Own entities already given an order during this thought.
    pub busy: BTreeSet<EntityId>,
}

/// Whether an entity can take part in the economy or produce now.
pub(crate) fn operational(e: &Entity) -> bool {
    e.build_progress.is_none() && !e.neutralised && e.assimilation_until.is_none()
}

impl<'a> View<'a> {
    /// Gathers the view, or `None` when the player is not in the world.
    pub fn new(world: &'a World, cat: &'a Catalogue, me: PlayerId) -> Option<Self> {
        let rules: &Rules = world.rules();
        let player = world.players().find(|p| p.id == me)?;
        let active = player.active_sources();
        let own: Vec<&Entity> = world.entities().filter(|e| e.owner == me).collect();
        let owned: Owned = own
            .iter()
            .filter(|e| e.build_progress.is_none())
            .map(|e| e.type_id)
            .collect();
        let mut have: BTreeMap<DefId, i64> = BTreeMap::new();
        let mut researching = BTreeSet::new();
        let mut queue_len = BTreeMap::new();
        let mut tended = BTreeSet::new();
        let mut inbound: BTreeMap<EntityId, i64> = BTreeMap::new();
        let mut queued_pop = 0;
        for e in &own {
            *have.entry(e.type_id).or_default() += 1;
            if !e.orders().is_empty() {
                queue_len.insert(e.id, e.orders().len());
            }
            for order in e.orders() {
                match (order.kind.as_str(), order.subject, order.target_entity) {
                    ("train", Some(subject), _) => {
                        *have.entry(subject).or_default() += 1;
                        queued_pop += rules.entity_types.get(&subject).map_or(0, |t| t.pop_cost);
                    }
                    ("research", Some(subject), _) => {
                        researching.insert(subject);
                    }
                    ("construct" | "repair", _, Some(site)) => {
                        tended.insert(site);
                    }
                    ("harvest", _, Some(site)) if e.work_site != Some(site) => {
                        *inbound.entry(site).or_default() += 1;
                    }
                    _ => {}
                }
            }
        }
        let mut line_have: BTreeMap<DefId, i64> = BTreeMap::new();
        for (type_id, count) in &have {
            *line_have.entry(cat.root_of(*type_id)).or_default() += count;
        }
        let cities: Vec<City> = world
            .cities(me)
            .into_iter()
            .filter_map(|id| world.entity(id))
            .map(|e| City {
                id: e.id,
                pos: e.pos,
                complete: operational(e),
                radius: world.city_economic_radius(e.id),
                border: rules.entity_types[&e.type_id]
                    .territory
                    .as_ref()
                    .map_or(0, |t| t.border_radius),
            })
            .collect();
        Some(Self {
            world,
            rules,
            cat,
            me,
            tick: world.tick(),
            faction: player.faction,
            age: player.age,
            purse: player.resources.clone(),
            pop_used: world.population_used(me),
            pop_cap: world.population_cap(me),
            queued_pop,
            cities,
            own,
            owned,
            active,
            have,
            line_have,
            researching,
            queue_len,
            tended,
            inbound,
            busy: BTreeSet::new(),
        })
    }

    /// Ordinal of the player's age.
    pub fn age_ordinal(&self) -> i64 {
        self.rules.ages.get(&self.age).map_or(0, |a| a.ordinal)
    }

    /// Whether the roster lists a definition and its prerequisites are met.
    pub fn unlocked(&self, def: DefId) -> bool {
        self.rules.is_available(def, self.faction)
            && self.rules.is_unlocked(def, &self.active, &self.owned)
    }

    /// Whether a successor in the type's upgrade line is already unlocked.
    pub fn obsolete(&self, def: DefId) -> bool {
        self.cat
            .successors
            .get(&def)
            .into_iter()
            .flatten()
            .any(|next| self.rules.is_unlocked(*next, &self.active, &self.owned))
    }

    /// Whether a technology has been researched.
    pub fn researched(&self, tech: DefId) -> bool {
        self.active.contains(&tech)
    }

    /// Entities and queued trainings of a type.
    pub fn count(&self, type_id: DefId) -> i64 {
        self.have.get(&type_id).copied().unwrap_or(0)
    }

    /// Entities and queued trainings of a whole upgrade line.
    pub fn line_count(&self, type_id: DefId) -> i64 {
        self.line_have
            .get(&self.cat.root_of(type_id))
            .copied()
            .unwrap_or(0)
    }

    /// Whether a type is a city.
    pub fn is_city(&self, type_id: DefId) -> bool {
        self.rules
            .entity_types
            .get(&type_id)
            .and_then(|t| t.territory.as_ref())
            .is_some_and(|t| t.is_city)
    }

    /// Whether an entity type is a building.
    pub fn is_building(&self, type_id: DefId) -> bool {
        self.rules
            .entity_types
            .get(&type_id)
            .is_some_and(|t| t.class == EntityClass::Building)
    }

    /// Own working entities whose type has a production row for `product` in `mode`.
    pub fn producers(&self, mode: ProductionMode, product: DefId) -> Vec<&'a Entity> {
        self.own
            .iter()
            .copied()
            .filter(|e| operational(e) && e.container_id.is_none())
            .filter(|e| {
                self.rules
                    .production
                    .get(&e.type_id)
                    .into_iter()
                    .flatten()
                    .any(|p| p.mode == mode && p.product_def_id == product)
            })
            .collect()
    }

    /// Whether an own unit has nothing to do.
    pub fn is_idle(&self, e: &Entity) -> bool {
        e.orders().is_empty()
            && e.work_site.is_none()
            && e.container_id.is_none()
            && e.build_progress.is_none()
            && !self.busy.contains(&e.id)
    }

    /// Records one more planned entity of a type, with its population.
    pub fn note_planned(&mut self, type_id: DefId) {
        *self.have.entry(type_id).or_default() += 1;
        *self.line_have.entry(self.cat.root_of(type_id)).or_default() += 1;
    }

    /// The price of a definition as the simulation asks it now: base cost raised by the
    /// ramp for what the player owns or has queued of the type or its ramp group, then the
    /// player's cost modifiers.
    pub fn price(&self, def: DefId) -> Vec<(DefId, i64)> {
        let owned = self.rules.entity_type(def).map_or(0, |et| {
            self.have
                .iter()
                .filter(|(other, _)| match et.ramp_mode {
                    RampMode::None => false,
                    RampMode::PerType => **other == def,
                    RampMode::PerGroup => self
                        .rules
                        .entity_type(**other)
                        .is_some_and(|o| o.ramp_group == et.ramp_group),
                })
                .map(|(_, count)| *count)
                .sum::<i64>()
        });
        self.rules
            .cost_of_ramped(def, &self.active, u64::try_from(owned).unwrap_or(0))
    }

    /// The player's commerce cap: the most a capped resource pays per income period.
    pub fn commerce_cap(&self) -> i64 {
        let base = self.rules.constant("player.commerce_cap.base").unwrap_or(0);
        self.rules.player_stat("commerce_cap", base, &self.active)
    }

    /// How many cities the player may have.
    pub fn city_limit(&self) -> i64 {
        let base = self.rules.constant("player.city_limit.base").unwrap_or(0);
        self.rules.player_stat("city_limit", base, &self.active)
    }

    /// How many trade routes the player may run.
    pub fn caravan_limit(&self) -> i64 {
        let base = self
            .rules
            .constant("player.caravan_limit.base")
            .unwrap_or(0);
        self.rules.player_stat("caravan_limit", base, &self.active)
    }

    /// Distance between two positions in milli-tiles, or a large number when out of reach.
    pub fn distance(&self, a: Pos, b: Pos) -> u64 {
        self.world.distance(a, b).unwrap_or(u64::MAX / 4)
    }
}

/// Entities of other players the brain may know about now, in a stable order.
pub(crate) fn sightings(
    world: &World,
    me: PlayerId,
    sees_all: bool,
) -> Vec<(EntityId, DefId, PlayerId, Pos)> {
    let hostile = |owner: PlayerId| owner != PlayerId(0) && world.enemies(me, owner);
    if sees_all {
        return world
            .entities()
            .filter(|e| hostile(e.owner) && e.container_id.is_none())
            .map(|e| (e.id, e.type_id, e.owner, e.pos))
            .collect();
    }
    world
        .visible_entities(me)
        .into_iter()
        .filter_map(|s| match s {
            Sighting::Live(e) if hostile(e.owner) => Some((e.id, e.type_id, e.owner, e.pos)),
            Sighting::Remembered(b) if hostile(b.owner) => Some((b.id, b.type_id, b.owner, b.pos)),
            _ => None,
        })
        .collect()
}

/// Updates the memory of enemies: what is seen now is refreshed, units not seen for a
/// while fade, and buildings last as long as the simulation's own fog memory keeps them.
pub(crate) fn remember(
    enemies: &mut BTreeMap<u32, EnemySighting>,
    world: &World,
    me: PlayerId,
    sees_all: bool,
) {
    let rules = world.rules();
    let tick = world.tick();
    let seen = sightings(world, me, sees_all);
    let mut now = BTreeSet::new();
    for (id, type_id, owner, pos) in seen {
        now.insert(id.0);
        enemies.insert(
            id.0,
            EnemySighting {
                type_id,
                owner,
                pos,
                tick,
                building: rules
                    .entity_types
                    .get(&type_id)
                    .is_some_and(|t| t.class == EntityClass::Building),
            },
        );
    }
    enemies.retain(|id, e| {
        now.contains(id) || (!e.building && tick.saturating_sub(e.tick) < UNIT_MEMORY_TICKS)
    });
}

/// Fighting strength of a type: hit points times damage per second, scaled down.
pub(crate) fn strength(rules: &Rules, type_id: DefId) -> i64 {
    let Some(et) = rules.entity_types.get(&type_id) else {
        return 0;
    };
    let dps: i64 = et
        .weapons
        .iter()
        .map(|w| w.damage * w.shots.max(1) * 20 / w.cooldown_ticks.max(1))
        .sum();
    et.hp * dps / 100
}

/// Known enemy strength per coarse cell.
pub(crate) fn threat_map(
    rules: &Rules,
    enemies: &BTreeMap<u32, EnemySighting>,
) -> BTreeMap<Cell, i64> {
    let mut map = BTreeMap::new();
    for enemy in enemies.values() {
        let value = strength(rules, enemy.type_id);
        if value > 0 {
            *map.entry(cell_of(enemy.pos)).or_default() += value;
        }
    }
    map
}

//! Owned simulation state and the fixed tick pipeline.

use crate::{
    SimError,
    avoidance::{Avoidance, Motion},
    construction::footprint,
    entity::{Entity, EntityId, Player, PlayerId},
    income::EconomyIndex,
    map::MapGrid,
    math::Pos,
    order::{Command, Order, OrderId, OrderStatus, has_component, target_matches},
    pathing::{PathStats, Pathing},
    rng::SimRng,
    spatial::SpatialGrid,
};
use rom_rules::{DefId, Rules};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

/// Aggregate terminal-order counts; no finished order history is retained.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    /// Successfully completed orders, including stop.
    pub orders_done: u64,
    /// Failed orders of all kinds.
    pub orders_failed: u64,
    /// Orders cancelled by replacement, stop, or despawning.
    pub orders_cancelled: u64,
    /// Orders failed because the next position was inaccessible.
    pub orders_no_path: u64,
    /// Orders failed because their behavior is not implemented yet.
    pub orders_unsupported: u64,
    /// Harvest orders failed because no slot was free on arrival.
    pub orders_no_slot: u64,
    /// Orders failed because their target disappeared.
    pub orders_target_gone: u64,
}

/// Deterministic match state, mutated through setup operations and commands.
#[derive(Debug)]
pub struct World {
    pub(crate) tick: u64,
    pub(crate) rng: SimRng,
    pub(crate) rules: Arc<Rules>,
    pub(crate) map: MapGrid,
    pub(crate) players: BTreeMap<PlayerId, Player>,
    pub(crate) entities: Vec<Option<Entity>>,
    pub(crate) pending: Vec<(Order, bool)>,
    pub(crate) next_entity_id: u32,
    pub(crate) next_order_id: u64,
    pub(crate) stats: Stats,
    pub(crate) income_remainders: BTreeMap<(PlayerId, DefId), i64>,
    pub(crate) pathing: Pathing,
    pub(crate) avoid: Avoidance,
    pub(crate) economy: EconomyIndex,
    spatial: SpatialGrid,
}

impl World {
    /// Establishes a checked map and players before the first tick.
    pub fn new(
        rules: Arc<Rules>,
        map: MapGrid,
        seed: u64,
        players: Vec<Player>,
    ) -> Result<Self, SimError> {
        map.validate()?;
        for terrain in &map.terrain {
            if !rules.terrains.contains_key(terrain) {
                return Err(SimError::UnknownTerrain(*terrain));
            }
        }
        if rules
            .constant("sim.ticks_per_second")
            .is_none_or(|v| v <= 0)
            || rules
                .entity_types
                .values()
                .filter_map(|e| e.movement.as_ref())
                .any(|m| m.speed < 0)
            || rules
                .movement_class_terrain
                .values()
                .flatten()
                .any(|row| row.speed_pct < 0)
        {
            return Err(SimError::InvalidRules(
                "tick rate must be positive and movement nonnegative",
            ));
        }
        for (key, minimum, maximum) in [
            ("player.pop_cap.base", 0, i64::MAX),
            ("build.site_start_hp_pct", 0, 100),
            ("build.reach", 0, i64::MAX),
            ("economy.cancel_refund_pct", 0, 100),
            ("economy.income_period_ticks", 1, i64::MAX),
            ("player.commerce_cap.base", 0, i64::MAX),
            ("citizen.auto_work_delay_ticks", 0, i64::MAX),
        ] {
            if rules
                .constant(key)
                .is_none_or(|v| v < minimum || v > maximum)
            {
                return Err(SimError::InvalidRules(key));
            }
        }
        if rules.constant("path.nodes_per_tick").is_some_and(|v| v < 1) {
            return Err(SimError::InvalidRules("path.nodes_per_tick"));
        }
        let mut by_id = BTreeMap::new();
        for player in players {
            if !rules.factions.contains_key(&player.faction)
                || !rules.ages.contains_key(&player.age)
                || by_id.insert(player.id, player).is_some()
            {
                return Err(SimError::InvalidPlayer);
            }
        }
        let pathing = Pathing::new(&rules, &map);
        let avoid = Avoidance::new(&rules, map.width, map.height);
        Ok(Self {
            tick: 0,
            rng: SimRng::new(seed),
            rules,
            map,
            pathing,
            avoid,
            economy: EconomyIndex::default(),
            players: by_id,
            entities: vec![None],
            pending: Vec::new(),
            next_entity_id: 1,
            next_order_id: 1,
            stats: Stats::default(),
            income_remainders: BTreeMap::new(),
            spatial: SpatialGrid::default(),
        })
    }

    /// Returns the number of completed simulation ticks.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns the immutable rules catalogue.
    pub fn rules(&self) -> &Arc<Rules> {
        &self.rules
    }

    /// Returns the generated map.
    pub fn map(&self) -> &MapGrid {
        &self.map
    }

    /// Iterates players in ascending id order.
    pub fn players(&self) -> impl Iterator<Item = &Player> {
        self.players.values()
    }

    /// Returns the complete random generator state without advancing it.
    pub fn rng(&self) -> &SimRng {
        &self.rng
    }

    /// Returns the next entity id, which never decreases.
    pub fn next_entity_id(&self) -> EntityId {
        EntityId(self.next_entity_id)
    }

    /// Returns the next order id, which never decreases.
    pub fn next_order_id(&self) -> OrderId {
        OrderId(self.next_order_id)
    }

    /// Looks up an entity in constant time.
    pub fn entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(id.0 as usize).and_then(Option::as_ref)
    }

    /// Iterates live entities in ascending id order, skipping vacant slots.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter().flatten()
    }

    /// Returns terminal-order counters.
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// Returns path-search counters.
    pub fn path_stats(&self) -> PathStats {
        self.pathing.stats
    }

    /// Spawns a checked entity with its type's full hit points.
    pub fn spawn(
        &mut self,
        type_id: DefId,
        owner: PlayerId,
        pos: Pos,
    ) -> Result<EntityId, SimError> {
        let hp = self
            .rules
            .entity_type(type_id)
            .ok_or(SimError::UnknownType(type_id))?
            .hp;
        if !self.players.contains_key(&owner) {
            return Err(SimError::UnknownPlayer(owner));
        }
        if self.map.terrain_at_pos(pos).is_none() {
            return Err(SimError::OutsideMap);
        }
        let next = self
            .next_entity_id
            .checked_add(1)
            .ok_or(SimError::IdExhausted)?;
        let id = EntityId(self.next_entity_id);
        let gather_slots = self.gather_slots(type_id, pos);
        self.entities.push(Some(Entity {
            id,
            type_id,
            owner,
            pos,
            facing: 0,
            hp,
            build_progress: None,
            move_remainder: 0,
            gather_slots,
            work_site: None,
            idle_ticks: 0,
            orders: VecDeque::new(),
        }));
        self.next_entity_id = next;
        self.spatial.insert(id, pos);
        let et = &self.rules.entity_types[&type_id];
        self.pathing.occupy(et, footprint(et, pos), true);
        self.avoid.set(id, Motion::of(&self.rules, type_id), pos);
        if et.gather_site.is_some() {
            self.economy.sites.insert(id);
        }
        self.refresh_income(id);
        self.economy.dirty.insert(id);
        Ok(id)
    }

    /// Updates grids and indexes after an entity's type changed from `old`.
    pub(crate) fn retyped(&mut self, id: EntityId, old: DefId) {
        let entity = self.entity(id).expect("retyped entity");
        let (pos, new) = (entity.pos, entity.type_id);
        let before = &self.rules.entity_types[&old];
        self.pathing.occupy(before, footprint(before, pos), false);
        let after = &self.rules.entity_types[&new];
        self.pathing.occupy(after, footprint(after, pos), true);
        self.avoid.set(id, Motion::of(&self.rules, new), pos);
        if after.gather_site.is_some() {
            self.economy.sites.insert(id);
        } else {
            self.economy.sites.remove(&id);
        }
        self.refresh_site(id);
        self.economy.dirty.insert(id);
    }

    /// Moves an entity between spatial cells after its position changed.
    pub(crate) fn relocate(&mut self, id: EntityId, from: Pos, to: Pos) {
        self.spatial.relocate(id, from, to);
        self.avoid.pos[id.0 as usize] = to;
    }

    /// Removes an entity and cancels both its live and pending orders.
    pub fn despawn(&mut self, id: EntityId) -> Result<(), SimError> {
        let entity = self
            .entities
            .get_mut(id.0 as usize)
            .and_then(Option::take)
            .ok_or(SimError::UnknownExecutor(id))?;
        self.spatial.remove(id, entity.pos);
        let et = &self.rules.entity_types[&entity.type_id];
        self.pathing.occupy(et, footprint(et, entity.pos), false);
        self.avoid.set(id, None, entity.pos);
        self.economy.idle.remove(&id);
        self.economy.dirty.remove(&id);
        self.economy.sites.remove(&id);
        self.refresh_income(id);
        if let Some(site) = entity.work_site
            && let Some(set) = self.economy.occupants.get_mut(&site)
        {
            set.remove(&id);
            if set.is_empty() {
                self.economy.occupants.remove(&site);
            }
        }
        self.release_site_workers(id);
        self.stats.orders_cancelled += entity.orders.len() as u64;
        self.pending.retain(|(order, _)| {
            if order.executor == id {
                self.stats.orders_cancelled += 1;
                false
            } else {
                true
            }
        });
        Ok(())
    }

    /// Returns exact radius matches in ascending id order, including the boundary.
    pub fn entities_in_radius(&self, center: Pos, radius: u32) -> Vec<EntityId> {
        let squared = u128::from(radius) * u128::from(radius);
        let mut ids: Vec<_> = self
            .spatial
            .candidates(center, radius)
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|e| e.pos.distance_squared_wide(center) <= squared)
            })
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Validates now and defers queue mutation until the next step.
    pub fn submit(&mut self, mut command: Command) -> Result<OrderId, SimError> {
        let entity = self
            .entity(command.executor)
            .ok_or(SimError::UnknownExecutor(command.executor))?;
        if entity.owner != command.player {
            return Err(SimError::ForeignOwner(command.executor));
        }
        let kind = self
            .rules
            .order_kinds
            .get(&command.kind)
            .ok_or_else(|| SimError::UnknownKind(command.kind.clone()))?;
        let et = self
            .rules
            .entity_type(entity.type_id)
            .expect("spawn validates type");
        if !has_component(et, &kind.needs) {
            return Err(SimError::MissingComponent(kind.needs.clone()));
        }
        if !target_matches(&command, kind.target) {
            return Err(SimError::WrongTargetShape);
        }
        if command.kind == "harvest" {
            self.validate_harvest(&command)?;
        }
        let payment = self.validate_economy_order(&mut command)?;
        let next = self
            .next_order_id
            .checked_add(1)
            .ok_or(SimError::IdExhausted)?;
        let id = OrderId(self.next_order_id);
        self.entities[command.executor.0 as usize]
            .as_mut()
            .expect("checked executor")
            .idle_ticks = 0;
        self.economy.dirty.insert(command.executor);
        for (resource, amount) in &payment {
            *self
                .players
                .get_mut(&command.player)
                .expect("checked owner")
                .resources
                .entry(*resource)
                .or_default() -= amount;
        }
        let total_ticks = if command.kind == "repair" {
            command
                .target_entity
                .and_then(|id| self.entity(id))
                .filter(|site| site.owner == command.player && site.build_progress.is_some())
                .map(|_| 0)
        } else {
            None
        };
        self.pending.push((
            Order {
                id,
                player: command.player,
                executor: command.executor,
                kind: command.kind,
                subject: command.subject,
                target_entity: command.target_entity,
                target_pos: command.target_pos,
                status: OrderStatus::Queued,
                progress_ticks: 0,
                issued_tick: self.tick,
                movement: None,
                payment,
                total_ticks,
                build_remainder: 0,
            },
            command.queue,
        ));
        self.next_order_id = next;
        Ok(id)
    }

    /// Finds idle work, applies inputs, advances entities by id, runs path searches,
    /// separates overlapping units, and distributes income.
    pub fn step(&mut self) {
        self.find_idle_work();
        for (mut order, append) in std::mem::take(&mut self.pending) {
            if order.kind == "construct" {
                self.create_site(&mut order);
            }
            if !append {
                self.cancel_orders(order.executor);
            } else {
                self.interrupt_harvest(order.executor);
            }
            self.entities[order.executor.0 as usize]
                .as_mut()
                .expect("despawn cancels pending orders")
                .orders
                .push_back(order);
        }
        let count = self.entities.len();
        let mut moved = Vec::new();
        for index in 1..count {
            let id = EntityId(index as u32);
            let Some(old) = self.entity(id).map(|e| e.pos) else {
                continue;
            };
            self.advance_executor(id);
            let new = self.entity(id).expect("executor persists").pos;
            if old != new {
                self.relocate(id, old, new);
                moved.push(id);
            }
        }
        self.run_path_searches();
        self.separate_units(&moved);
        self.pay_income();
        self.pathing.end_tick();
        self.tick = self.tick.checked_add(1).expect("simulation tick exhausted");
    }
}

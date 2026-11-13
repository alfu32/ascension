//! Owned simulation state and the fixed tick pipeline.

use crate::{
    SimError,
    avoidance::{Avoidance, Motion},
    construction::footprint,
    entity::{Entity, EntityId, Player, PlayerId},
    geometry::{Geometry, Topology},
    income::EconomyIndex,
    map::MapGrid,
    math::Pos,
    order::{Command, Order, OrderId, OrderStatus, has_component, target_matches},
    pathing::{PathStats, Pathing},
    rng::SimRng,
    spatial::SpatialGrid,
    territory::Territory,
    vision::Vision,
};
use rom_geo::{CubeSurface, SurfacePos};
use rom_rules::{DefId, Rules};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

/// Aggregate order counts and per-player combat totals.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stats {
    /// Successfully completed orders, including stop.
    pub orders_done: u64,
    /// Enemy kills credited to each player.
    pub kills: BTreeMap<PlayerId, u64>,
    /// Combat deaths, including consumed attackers, for each player.
    pub losses: BTreeMap<PlayerId, u64>,
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
    pub(crate) hits: Vec<crate::combat::PendingHit>,
    pub(crate) shots: Vec<crate::combat::PendingShot>,
    pub(crate) tick: u64,
    pub(crate) rng: SimRng,
    pub(crate) rules: Arc<Rules>,
    pub(crate) map: MapGrid,
    /// The surface's shape; every spatial question goes through it.
    pub(crate) geo: Geometry,
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
    /// Positions by entity id as they stood when the order phase began; empty
    /// between ticks. Combat reads other entities here so update order cannot matter.
    pub(crate) seen: Vec<Pos>,
    /// Attackers that spent themselves this tick; emptied when hits land.
    pub(crate) spent: Vec<EntityId>,
    pub(crate) territory: Territory,
    pub(crate) vision: Vision,
    spatial: SpatialGrid,
}

impl World {
    /// Establishes a checked flat map and players before the first tick.
    pub fn new(
        rules: Arc<Rules>,
        map: MapGrid,
        seed: u64,
        players: Vec<Player>,
    ) -> Result<Self, SimError> {
        let topology = Topology::Flat {
            width: map.width,
            height: map.height,
        };
        Self::with_topology(rules, map, topology, seed, players)
    }

    /// Establishes a cube-sphere planet: `terrain` holds one definition per tile in the
    /// surface's dense order. Each node is spawned for the neutral player 0, which must
    /// then be among `players`; see `spawn_nodes`.
    pub fn from_sphere(
        rules: Arc<Rules>,
        surface: CubeSurface,
        terrain: Vec<DefId>,
        nodes: &[(DefId, SurfacePos)],
        seed: u64,
        players: Vec<Player>,
    ) -> Result<Self, SimError> {
        if !(4..=4096).contains(&surface.n) {
            return Err(SimError::InvalidMap);
        }
        let map = MapGrid::new(surface.n, surface.n * 6, terrain)?;
        let mut world = Self::with_topology(rules, map, Topology::Cube(surface), seed, players)?;
        world.spawn_nodes(nodes)?;
        Ok(world)
    }

    /// Spawns map nodes (a type and a tile position each) for the neutral player 0.
    pub fn spawn_nodes(
        &mut self,
        nodes: &[(DefId, SurfacePos)],
    ) -> Result<Vec<EntityId>, SimError> {
        nodes
            .iter()
            .map(|(type_id, at)| self.spawn(*type_id, PlayerId(0), Self::pos_of(*at)?))
            .collect()
    }

    /// A simulation position for a `rom_geo` surface position.
    pub fn pos_of(at: SurfacePos) -> Result<Pos, SimError> {
        Ok(Pos {
            x: i32::try_from(at.x).map_err(|_| SimError::OutsideMap)?,
            y: i32::try_from(at.y).map_err(|_| SimError::OutsideMap)?,
            face: at.face,
        })
    }

    fn with_topology(
        rules: Arc<Rules>,
        map: MapGrid,
        topology: Topology,
        seed: u64,
        players: Vec<Player>,
    ) -> Result<Self, SimError> {
        map.validate()?;
        let geo = Geometry::new(topology).ok_or(SimError::InvalidMap)?;
        if geo.tiles() != map.terrain.len() {
            return Err(SimError::InvalidMap);
        }
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
            ("combat.min_damage", 1, i64::MAX),
            ("unit.respond_range", 0, i64::MAX),
            ("player.pop_cap.base", 0, i64::MAX),
            ("build.site_start_hp_pct", 0, 100),
            ("build.reach", 0, i64::MAX),
            ("economy.cancel_refund_pct", 0, 100),
            ("economy.income_period_ticks", 1, i64::MAX),
            ("player.commerce_cap.base", 0, i64::MAX),
            ("citizen.auto_work_delay_ticks", 0, i64::MAX),
            ("city.spacing", 0, i64::MAX),
            ("player.city_limit.base", 0, i64::MAX),
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
        if !rules
            .stances
            .values()
            .any(|s| s.family == rom_rules::StanceFamily::Combat && s.behavior == "aggressive")
        {
            return Err(SimError::InvalidRules("aggressive combat stance"));
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
        let pathing = Pathing::new(&rules, &map, geo.clone());
        let avoid = Avoidance::new(&rules, geo.clone());
        let territory = Territory::new(&rules, &map);
        let mut world = Self {
            hits: Vec::new(),
            shots: Vec::new(),
            tick: 0,
            rng: SimRng::new(seed),
            rules,
            map,
            geo,
            pathing,
            avoid,
            economy: EconomyIndex::default(),
            seen: Vec::new(),
            spent: Vec::new(),
            players: by_id,
            entities: vec![None],
            pending: Vec::new(),
            next_entity_id: 1,
            next_order_id: 1,
            stats: Stats::default(),
            income_remainders: BTreeMap::new(),
            territory,
            vision: Vision::default(),
            spatial: SpatialGrid::default(),
        };
        let ids: Vec<PlayerId> = world.players.keys().copied().collect();
        for id in ids {
            world.ensure_vision(id);
        }
        Ok(world)
    }

    /// Returns the number of completed simulation ticks.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns the immutable rules catalogue.
    pub fn rules(&self) -> &Arc<Rules> {
        &self.rules
    }

    /// Returns the terrain storage; a sphere's faces are stacked as one strip.
    pub fn map(&self) -> &MapGrid {
        &self.map
    }

    /// Returns the surface's shape.
    pub fn topology(&self) -> Topology {
        self.geo.topology
    }

    /// Returns the terrain under a position, or None off the surface.
    pub fn terrain_at(&self, pos: Pos) -> Option<DefId> {
        self.geo.pos_index(pos).map(|index| self.map.terrain[index])
    }

    /// Distance between two positions in milli-tiles, measured across face edges on a
    /// sphere; None there when they are farther apart than half a face.
    pub fn distance(&self, a: Pos, b: Pos) -> Option<u64> {
        self.geo.distance(a, b)
    }

    /// Steps a position by a local vector in milli-tiles, crossing face edges.
    pub fn offset(&self, pos: Pos, dx: i64, dy: i64) -> Pos {
        self.geo.offset(pos, dx, dy).0
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

    /// Returns terminal-order counters and per-player kills and losses.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Returns path-search counters.
    pub fn path_stats(&self) -> PathStats {
        self.pathing.stats
    }

    /// Spawns a checked, complete entity with its type's full hit points.
    pub fn spawn(
        &mut self,
        type_id: DefId,
        owner: PlayerId,
        pos: Pos,
    ) -> Result<EntityId, SimError> {
        self.spawn_entity(type_id, owner, pos, false)
    }

    /// Spawns a complete entity, or an unfinished building site when `site` is true.
    pub(crate) fn spawn_entity(
        &mut self,
        type_id: DefId,
        owner: PlayerId,
        pos: Pos,
        site: bool,
    ) -> Result<EntityId, SimError> {
        let hp = self
            .rules
            .entity_type(type_id)
            .ok_or(SimError::UnknownType(type_id))?
            .hp;
        if !self.players.contains_key(&owner) {
            return Err(SimError::UnknownPlayer(owner));
        }
        if !self.geo.contains(pos) {
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
            max_hp: hp,
            los: 0,
            stance: self.default_stance(),
            cooldowns: BTreeMap::new(),
            energy: self.rules.entity_types[&type_id]
                .energy
                .as_ref()
                .map_or(0, |e| e.max),
            energy_remainder: 0,
            heal_remainder: 0,
            killer: None,
            build_progress: site.then_some(0),
            move_remainder: 0,
            gather_slots,
            work_site: None,
            city_id: None,
            idle_ticks: 0,
            orders: VecDeque::new(),
        }));
        self.refresh_entity_stats(id);
        self.next_entity_id = next;
        self.spatial.insert(id, pos);
        let et = &self.rules.entity_types[&type_id];
        self.pathing.occupy(et, pos.face, footprint(et, pos), true);
        self.avoid.set(id, Motion::of(&self.rules, type_id), pos);
        if et.gather_site.is_some() {
            self.economy.sites.insert(id);
        }
        self.refresh_income(id);
        self.economy.dirty.insert(id);
        self.assign_city(id);
        self.refresh_territory(id);
        if self.is_building(type_id) {
            self.index_building(id, pos, true);
        }
        Ok(id)
    }

    /// Updates grids and indexes after an entity's type changed from `old`.
    pub(crate) fn retyped(&mut self, id: EntityId, old: DefId) {
        let entity = self.entity(id).expect("retyped entity");
        let (pos, new) = (entity.pos, entity.type_id);
        let before = &self.rules.entity_types[&old];
        self.pathing
            .occupy(before, pos.face, footprint(before, pos), false);
        let after = &self.rules.entity_types[&new];
        self.pathing
            .occupy(after, pos.face, footprint(after, pos), true);
        self.avoid.set(id, Motion::of(&self.rules, new), pos);
        if after.gather_site.is_some() {
            self.economy.sites.insert(id);
        } else {
            self.economy.sites.remove(&id);
        }
        self.refresh_site(id);
        self.economy.dirty.insert(id);
        if self.is_building(old) {
            self.index_building(id, pos, false);
        }
        if self.is_building(new) {
            self.index_building(id, pos, true);
        }
        if self.is_city_type(old) && !self.is_city_type(new) {
            self.release_city(id);
        }
        self.refresh_territory(id);
    }

    /// Moves an entity between spatial cells after its position changed.
    pub(crate) fn relocate(&mut self, id: EntityId, from: Pos, to: Pos) {
        self.spatial.relocate(id, from, to);
        self.avoid.pos[id.0 as usize] = to;
        if self.geo.tile_of(from) != self.geo.tile_of(to) {
            self.refresh_viewer(id);
        }
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
        self.pathing
            .occupy(et, entity.pos.face, footprint(et, entity.pos), false);
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
        self.refresh_viewer(id);
        self.refresh_territory(id);
        if self.is_city_type(entity.type_id) {
            self.release_city(id);
        }
        if self.is_building(entity.type_id) {
            self.index_building(id, entity.pos, false);
        }
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

    /// Returns exact radius matches in ascending id order, including the boundary; on a
    /// sphere the radius is measured across face edges and limited to half a face.
    pub fn entities_in_radius(&self, center: Pos, radius: u32) -> Vec<EntityId> {
        let radius = self.geo.clamp_radius(i64::from(radius)) as u32;
        let mut ids: Vec<_> = self
            .spatial
            .candidates(&self.geo, center, radius)
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|e| self.geo.within(e.pos, center, i64::from(radius)))
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
        // A flat map clamps far goals onto itself; a sphere has no outside to clamp from.
        if !self.geo.is_flat()
            && command
                .target_pos
                .is_some_and(|pos| !self.geo.contains(pos))
        {
            return Err(SimError::OutsideMap);
        }
        if command.kind == "harvest" {
            self.validate_harvest(&command)?;
        }
        self.validate_combat_order(&command)?;
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
                automatic: false,
                home: None,
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

    /// Finds idle work, applies inputs, chooses attack-move targets, advances entities
    /// by id, runs path searches, separates overlapping units, lands every hit of the
    /// tick together, and distributes income. Combat decisions see other entities as
    /// they were when the order phase began, so the id order of updates cannot decide
    /// who strikes first.
    pub fn step(&mut self) {
        self.find_idle_work();
        self.acquire_idle();
        self.regenerate_energy();
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
        self.acquire_attack_moves();
        // `avoid.pos` mirrors every entity's position until the first move below.
        if self.has_enemies() {
            self.seen.clone_from(&self.avoid.pos);
        }
        let count = self.entities.len();
        let mut moved = Vec::new();
        for index in 1..count {
            let id = EntityId(index as u32);
            let Some(old) = self.entity(id).map(|e| e.pos) else {
                continue;
            };
            self.advance_executor(id);
            self.heal_idle(id);
            let new = self.entity(id).expect("executor persists").pos;
            if old != new {
                self.relocate(id, old, new);
                moved.push(id);
            }
        }
        self.run_path_searches();
        self.separate_units(&moved);
        self.seen.clear();
        self.fire_shots();
        self.land_projectiles();
        self.remove_dead();
        self.remember_touched();
        self.pay_income();
        self.pathing.end_tick();
        self.tick = self.tick.checked_add(1).expect("simulation tick exhausted");
    }
}

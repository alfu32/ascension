//! Owned simulation state and the fixed tick pipeline.

use crate::{
    BodyId, SimError,
    avoidance::{Avoidance, Motion},
    construction::footprint,
    entity::{Entity, EntityId, Player, PlayerId},
    geometry::{Geometry, Topology},
    income::EconomyIndex,
    map::MapGrid,
    math::Pos,
    order::{Command, Order, OrderId, OrderStatus, has_component, target_matches},
    pathing::PathStats,
    rng::SimRng,
    surface::Surface,
    territory::Territory,
    universe::UniverseData,
    vision::Vision,
};
use rom_geo::{CubeSurface, SurfacePos};
use rom_rules::{DefId, Rules};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
};

/// Aggregate order counts and per-player combat totals.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stats {
    /// Paid trade arrivals, excluding the initial walk to the home city.
    pub trade_trips: u64,
    /// Enemy cities captured during the match.
    pub cities_captured: u64,
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
    pub(crate) victory: crate::victory::VictoryState,
    pub(crate) capture: crate::capture::CaptureState,
    pub(crate) hits: Vec<crate::combat::PendingHit>,
    pub(crate) shots: Vec<crate::combat::PendingShot>,
    pub(crate) tick: u64,
    pub(crate) rng: SimRng,
    pub(crate) rules: Arc<Rules>,
    /// Every generated surface, in body order; transit ships stand on none.
    pub(crate) surfaces: Vec<Surface>,
    /// Index into `surfaces` by body id; `u32::MAX` where a body has no surface.
    pub(crate) slots: Vec<u32>,
    /// Star systems, bodies and routes; empty for a world built from one map.
    pub(crate) universe: UniverseData,
    /// System positions, markers, claims and ships between surfaces.
    pub(crate) space: crate::space::Space,
    /// Each player's home body.
    pub(crate) homes: BTreeMap<PlayerId, BodyId>,
    /// Simulation ticks per second.
    pub(crate) tick_rate: i64,
    pub(crate) players: BTreeMap<PlayerId, Player>,
    pub(crate) entities: Vec<Option<Entity>>,
    pub(crate) pending: Vec<(Order, bool)>,
    pub(crate) next_entity_id: u32,
    pub(crate) next_order_id: u64,
    pub(crate) stats: Stats,
    pub(crate) income_remainders: BTreeMap<(PlayerId, DefId), i64>,
    pub(crate) avoid: Avoidance,
    pub(crate) economy: EconomyIndex,
    /// Positions by entity id as they stood when the order phase began; empty
    /// between ticks. Combat reads other entities here so update order cannot matter.
    pub(crate) seen: Vec<Pos>,
    /// Attackers that spent themselves this tick; emptied when hits land.
    pub(crate) spent: Vec<EntityId>,
    pub(crate) territory: Territory,
    pub(crate) vision: Vision,
    /// Rally points of producers (`set_rally`), by producer.
    pub(crate) rallies: BTreeMap<EntityId, crate::rally::Rally>,
    /// Automated colonies whose owner paused the governor (`pause_governor`).
    pub(crate) governors_paused: BTreeSet<EntityId>,
    /// Legs of space trade routes queued this tick in front of their routes; empty between
    /// ticks.
    pub(crate) legs: Vec<Order>,
    /// Units whose queued `disband` came up this tick; empty between ticks.
    pub(crate) disbanded: Vec<EntityId>,
    /// Each player's latest failures of accepted orders, oldest first (a report, not hashed).
    pub(crate) failures: BTreeMap<PlayerId, std::collections::VecDeque<crate::OrderFailure>>,
    /// Entities by owner and type, sites and producers; derived, not hashed.
    pub(crate) types: crate::index::TypeIndex,
    /// The largest building footprint in the rules, in tiles across and down.
    pub(crate) max_footprint: (i64, i64),
    /// Recent blasts of area weapons, for the views (task 75; not hashed, not saved).
    pub(crate) blasts: Vec<crate::combat::Blast>,
    /// Buildings that fell lately, for the views (task 75; not hashed, not saved).
    pub(crate) wrecks: Vec<crate::combat::Wreck>,
    /// Stain lifetime and cap per surface, from the rules (task 80).
    pub(crate) stain_rules: crate::stains::StainRules,
    /// Buildings pulled down by their owners this tick (no stain); empty between ticks.
    pub(crate) razed: Vec<EntityId>,
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

    /// Spawns map nodes (a type and a tile position each) on body 0 for the neutral
    /// player 0.
    pub fn spawn_nodes(
        &mut self,
        nodes: &[(DefId, SurfacePos)],
    ) -> Result<Vec<EntityId>, SimError> {
        nodes
            .iter()
            .map(|(type_id, at)| self.spawn(*type_id, PlayerId(0), Self::pos_of(*at)?))
            .collect()
    }

    /// A simulation position on body 0 for a `rom_geo` surface position.
    pub fn pos_of(at: SurfacePos) -> Result<Pos, SimError> {
        Ok(Pos {
            x: i32::try_from(at.x).map_err(|_| SimError::OutsideMap)?,
            y: i32::try_from(at.y).map_err(|_| SimError::OutsideMap)?,
            face: at.face,
            body: BodyId(0),
        })
    }

    /// A one-surface world: the map is body 0 and every player's home.
    fn with_topology(
        rules: Arc<Rules>,
        map: MapGrid,
        topology: Topology,
        seed: u64,
        players: Vec<Player>,
    ) -> Result<Self, SimError> {
        let mut world = Self::empty(rules, seed, players)?;
        world.insert_map(BodyId(0), map, topology)?;
        world.homes = world.players.keys().map(|id| (*id, BodyId(0))).collect();
        Ok(world)
    }

    /// Checks the rules and players and builds a world without surfaces.
    pub(crate) fn empty(
        rules: Arc<Rules>,
        seed: u64,
        players: Vec<Player>,
    ) -> Result<Self, SimError> {
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
            ("combat.attrition_per_period", 0, i64::MAX),
            ("combat.attrition_floor_hp", 0, 1),
            ("city.assimilate_ticks", 0, i64::MAX),
            ("city.capture_hold_ticks", 0, i64::MAX),
            ("victory.capital_hold_ticks", 0, i64::MAX),
            ("unit.respond_range", 0, i64::MAX),
            ("player.pop_cap.base", 0, i64::MAX),
            ("build.site_start_hp_pct", 0, 100),
            ("build.reach", 0, i64::MAX),
            ("economy.cancel_refund_pct", 0, 100),
            ("economy.income_period_ticks", 1, i64::MAX),
            ("player.commerce_cap.base", 0, i64::MAX),
            ("citizen.auto_work_delay_ticks", 0, i64::MAX),
            ("player.caravan_limit.base", 0, i64::MAX),
            ("trade.route_unit", 1, i64::MAX),
            ("trade.min_route", 1, i64::MAX),
            ("city.spacing", 0, i64::MAX),
            ("player.city_limit.base", 0, i64::MAX),
            ("world.stain_days", 0, i64::MAX),
            ("world.stain_cap", 0, i64::MAX),
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
        let avoid = Avoidance::new(&rules);
        let tick_rate = rules.constant("sim.ticks_per_second").unwrap_or(1).max(1);
        let victory = crate::victory::VictoryState::new(&rules, &by_id)?;
        let max_footprint = rules
            .entity_types
            .values()
            .filter(|et| et.class == rom_rules::EntityClass::Building)
            .fold((0, 0), |(w, h), et| {
                (w.max(et.footprint_w.max(0)), h.max(et.footprint_h.max(0)))
            });
        let stain_rules = crate::stains::StainRules::of(&rules);
        Ok(Self {
            victory,
            capture: crate::capture::CaptureState::default(),
            hits: Vec::new(),
            shots: Vec::new(),
            tick: 0,
            rng: SimRng::new(seed),
            rules,
            surfaces: Vec::new(),
            slots: Vec::new(),
            universe: UniverseData::default(),
            homes: BTreeMap::new(),
            space: crate::space::Space::default(),
            tick_rate,
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
            territory: Territory::default(),
            vision: Vision::default(),
            rallies: BTreeMap::new(),
            governors_paused: BTreeSet::new(),
            disbanded: Vec::new(),
            legs: Vec::new(),
            failures: BTreeMap::new(),
            types: crate::index::TypeIndex::default(),
            max_footprint,
            blasts: Vec::new(),
            wrecks: Vec::new(),
            stain_rules,
            razed: Vec::new(),
        })
    }

    /// The index of a body's surface, if it has one.
    #[inline]
    pub(crate) fn slot_of(&self, body: BodyId) -> Option<usize> {
        match self.slots.get(body.0 as usize) {
            Some(&slot) if slot != u32::MAX => Some(slot as usize),
            _ => None,
        }
    }

    /// The index of the surface of a body that entities stand on.
    #[inline]
    pub(crate) fn slot(&self, body: BodyId) -> usize {
        self.slot_of(body).expect("body has a surface")
    }

    /// The surface of a body that entities stand on.
    #[inline]
    pub(crate) fn surf(&self, body: BodyId) -> &Surface {
        &self.surfaces[self.slot(body)]
    }

    /// The geometry of a body that entities stand on.
    #[inline]
    pub(crate) fn geo(&self, body: BodyId) -> &Geometry {
        &self.surf(body).geo
    }

    /// Returns the number of completed simulation ticks.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Returns the immutable rules catalogue.
    pub fn rules(&self) -> &Arc<Rules> {
        &self.rules
    }

    /// Returns the first surface's terrain storage (the only one of a one-surface
    /// world); a sphere's faces are stacked as one strip.
    pub fn map(&self) -> &MapGrid {
        &self.surfaces[0].map
    }

    /// Returns the first surface's shape (the only one of a one-surface world).
    pub fn topology(&self) -> Topology {
        self.surfaces[0].geo.topology
    }

    /// Returns the terrain under a position on its body, or None off every surface.
    pub fn terrain_at(&self, pos: Pos) -> Option<DefId> {
        let surface = self.surface(pos.body)?;
        surface
            .geo
            .pos_index(pos)
            .map(|index| surface.map.terrain[index])
    }

    /// Returns the terrain of a tile on a face of a body, or None off its surface.
    pub fn terrain_on_body(
        &self,
        body: BodyId,
        face: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<DefId> {
        let surface = self.surface(body)?;
        surface
            .geo
            .index_of(crate::geometry::Tile::new(face, tile_x, tile_y))
            .map(|index| surface.map.terrain[index])
    }

    /// Distance between two positions in milli-tiles, measured across face edges on a
    /// sphere; None there when they are farther apart than half a face, and always None
    /// between different bodies.
    pub fn distance(&self, a: Pos, b: Pos) -> Option<u64> {
        self.surface(a.body)?.geo.distance(a, b)
    }

    /// Steps a position by a local vector in milli-tiles, crossing face edges; a position
    /// on a body without a surface is returned unchanged.
    pub fn offset(&self, pos: Pos, dx: i64, dy: i64) -> Pos {
        self.surface(pos.body)
            .map_or(pos, |surface| surface.geo.offset(pos, dx, dy).0)
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

    /// Returns path-search counters summed over every surface.
    pub fn path_stats(&self) -> PathStats {
        self.surfaces
            .iter()
            .fold(PathStats::default(), |sum, s| PathStats {
                searches: sum.searches + s.pathing.stats.searches,
                nodes: sum.nodes + s.pathing.stats.nodes,
                repaths: sum.repaths + s.pathing.stats.repaths,
            })
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
        let Some(slot) = self
            .slot_of(pos.body)
            .filter(|slot| self.surfaces[*slot].geo.contains(pos))
        else {
            return Err(SimError::OutsideMap);
        };
        self.check_space_placement(type_id, owner, pos)?;
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
            attrition_remainder: 0,
            neutralised: false,
            assimilation_until: None,
            killer: None,
            build_progress: site.then_some(0),
            move_remainder: 0,
            gather_slots,
            work_site: None,
            city_id: None,
            container_id: None,
            idle_ticks: 0,
            aim: None,
            orders: VecDeque::new(),
        }));
        self.index_entity(owner, type_id, id, site);
        self.note_capital(id);
        self.refresh_entity_stats(id);
        self.next_entity_id = next;
        let surface = &mut self.surfaces[slot];
        surface.spatial.insert(id, pos);
        let et = &self.rules.entity_types[&type_id];
        surface
            .pathing
            .occupy(et, pos.face, footprint(et, pos), true);
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
        let (owner, site) = (entity.owner, entity.build_progress.is_some());
        self.types.remove(owner, old, id, site);
        self.index_entity(owner, new, id, site);
        let slot = self.slot(pos.body);
        let pathing = &mut self.surfaces[slot].pathing;
        let before = &self.rules.entity_types[&old];
        pathing.occupy(before, pos.face, footprint(before, pos), false);
        let after = &self.rules.entity_types[&new];
        pathing.occupy(after, pos.face, footprint(after, pos), true);
        let motion = self
            .walks(id)
            .then(|| Motion::of(&self.rules, new))
            .flatten();
        self.avoid.set(id, motion, pos);
        if after.gather_site.is_some() {
            self.economy.sites.insert(id);
        } else {
            self.economy.sites.remove(&id);
        }
        let slots = if self
            .entity(id)
            .expect("retyped entity")
            .build_progress
            .is_none()
        {
            self.gather_slots(new, pos)
        } else {
            0
        };
        if slots == 0 {
            self.release_site_workers(id);
        }
        self.entities[id.0 as usize]
            .as_mut()
            .expect("retyped entity")
            .gather_slots = slots;
        self.refresh_site(id);
        self.refresh_trade_income(id);
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

    /// Moves an entity between spatial cells of its surface after its position changed.
    pub(crate) fn relocate(&mut self, id: EntityId, from: Pos, to: Pos) {
        let slot = self.slot(to.body);
        if from.body != to.body {
            let from_slot = self.slot(from.body);
            self.surfaces[from_slot].spatial.remove(id, from);
            self.surfaces[slot].spatial.insert(id, to);
            let entity = self.entity(id).expect("relocated entity");
            let motion = self
                .walks(id)
                .then(|| Motion::of(&self.rules, entity.type_id))
                .flatten();
            self.avoid.set(id, motion, to);
            self.refresh_viewer(id);
            return;
        }
        let surface = &mut self.surfaces[slot];
        surface.spatial.relocate(id, from, to);
        self.avoid.pos[id.0 as usize] = to;
        if surface.geo.tile_of(from) != surface.geo.tile_of(to) {
            self.refresh_viewer(id);
        }
    }

    /// Removes an entity and cancels both its live and pending orders.
    pub fn despawn(&mut self, id: EntityId) -> Result<(), SimError> {
        let (pos, type_id) = self
            .entity(id)
            .map(|e| (e.pos, e.type_id))
            .ok_or(SimError::UnknownExecutor(id))?;
        if self.may_hold_nodes(type_id) {
            self.drop_nodes(id, pos);
        }
        // Units aboard a carrier go down with it.
        for passenger in self.passengers(id) {
            self.despawn(passenger)?;
        }
        let entity = self
            .entities
            .get_mut(id.0 as usize)
            .and_then(Option::take)
            .ok_or(SimError::UnknownExecutor(id))?;
        self.types.remove(
            entity.owner,
            entity.type_id,
            id,
            entity.build_progress.is_some(),
        );
        self.types.producers.remove(&id);
        self.rallies.remove(&id);
        self.governors_paused.remove(&id);
        self.space.orbits.remove(&id);
        self.space.landed.remove(&id);
        self.space.arrivals.remove(&id);
        self.capture.neutralised.remove(&id);
        self.capture.assimilating.remove(&id);
        self.capture.sieges.remove(&id);
        let slot = self.slot(entity.pos.body);
        let surface = &mut self.surfaces[slot];
        surface.spatial.remove(id, entity.pos);
        let et = &self.rules.entity_types[&entity.type_id];
        surface
            .pathing
            .occupy(et, entity.pos.face, footprint(et, entity.pos), false);
        self.avoid.set(id, None, entity.pos);
        self.economy.idle.remove(&id);
        self.economy.dirty.remove(&id);
        self.economy.sites.remove(&id);
        self.refresh_income(id);
        if let Some(container) = entity.container_id {
            self.refresh_income(container);
        }
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

    /// Returns exact radius matches on the centre's body in ascending id order, including
    /// the boundary; on a sphere the radius is measured across face edges and limited to
    /// half a face.
    pub fn entities_in_radius(&self, center: Pos, radius: u32) -> Vec<EntityId> {
        let Some(surface) = self.surface(center.body) else {
            return Vec::new();
        };
        let geo = &surface.geo;
        let radius = geo.clamp_radius(i64::from(radius)) as u32;
        let mut ids: Vec<_> = surface
            .spatial
            .candidates(geo, center, radius)
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|e| geo.within(e.pos, center, i64::from(radius)))
            })
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Validates now and defers queue mutation until the next step.
    pub fn submit(&mut self, mut command: Command) -> Result<OrderId, SimError> {
        self.check_orders_open(command.player)?;
        let entity = self
            .entity(command.executor)
            .ok_or(SimError::UnknownExecutor(command.executor))?;
        if entity.neutralised {
            return Err(SimError::InactiveExecutor);
        }
        if entity.owner != command.player {
            return Err(SimError::ForeignOwner(command.executor));
        }
        // A unit aboard acts only once its carrier stands on a planet; it then steps out.
        if self.aboard_in_space(entity) {
            return Err(SimError::Aboard);
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
        self.validate_craft(&command)?;
        self.validate_space_order(&command)?;
        self.validate_transport(&command)?;
        // Orbit can name a body's surface; an orbiting ship can attack that surface, and what
        // stands on a planet can attack what orbits it.
        let body = entity.pos.body;
        let orbital_attack = command.kind == "attack"
            && command
                .target_entity
                .is_some_and(|target| self.across_orbit(command.executor, target));
        // Within the orbit it holds a ship moves, attack-moves and patrols over its planet.
        let orbit_move = matches!(command.kind.as_str(), "move" | "attack_move" | "patrol")
            && self.orbit(command.executor).is_some_and(|orbit| {
                command
                    .target_pos
                    .is_some_and(|p| p.body == orbit.body && self.geo(orbit.body).contains(p))
            });
        // Boarding, landing, unloading and trade check their own bodies.
        let cross_body = matches!(
            command.kind.as_str(),
            "travel" | "orbit" | "enter" | "land" | "unload" | "trade"
        ) || orbital_attack
            || orbit_move;
        if !cross_body
            && (command.target_pos.is_some_and(|pos| pos.body != body)
                || command
                    .target_entity
                    .and_then(|id| self.entity(id))
                    .is_some_and(|target| target.pos.body != body))
        {
            return Err(SimError::OtherBody);
        }
        // A flat map clamps far goals onto itself; a sphere has no outside to clamp from.
        let geo = self.geo(body);
        if !cross_body && !geo.is_flat() && command.target_pos.is_some_and(|pos| !geo.contains(pos))
        {
            return Err(SimError::OutsideMap);
        }
        if command.kind == "harvest" {
            self.validate_harvest(&command)?;
        }
        if command.kind == "carry" {
            self.validate_carry(&command)?;
        }
        let route = if command.kind == "trade" {
            Some(self.validate_trade(&command)?)
        } else {
            None
        };
        self.validate_combat_order(&command)?;
        self.validate_rally_and_disband(&command)?;
        self.validate_tactics(&command)?;
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
                pace: None,
                engaged: None,
                movement: None,
                route: route.map(Box::new),
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
        let mut applied = Vec::new();
        for (mut order, append) in std::mem::take(&mut self.pending) {
            // An executor disbanded earlier in this loop takes no more orders.
            if self.entity(order.executor).is_none() {
                self.stats.orders_cancelled += 1;
                continue;
            }
            // A passenger given an order steps out of its landed carrier to obey it.
            if !self.step_out(order.executor) {
                self.stats.orders_cancelled += 1;
                continue;
            }
            // A rally point is a setting: it does not replace or join the producer's queue.
            if order.kind == "set_rally" {
                self.apply_rally(&order);
                continue;
            }
            // So are a combat stance and the order to pull down an own building.
            if order.kind == "stance" {
                self.apply_stance(&order);
                continue;
            }
            if order.kind == "raze" {
                self.apply_raze(&order);
                continue;
            }
            // A colony's governor switch is a setting too.
            if matches!(order.kind.as_str(), "pause_governor" | "resume_governor") {
                self.apply_governor(&order);
                continue;
            }
            if order.kind == "disband" && !append {
                self.despawn(order.executor).expect("checked executor");
                self.stats.orders_done += 1;
                continue;
            }
            if order.kind == "construct" {
                self.create_site(&mut order);
            }
            if !append {
                self.cancel_orders(order.executor);
            } else {
                self.interrupt_harvest(order.executor);
            }
            if order.subject.is_some() {
                applied.push((order.executor, order.id));
            }
            self.entities[order.executor.0 as usize]
                .as_mut()
                .expect("despawn cancels pending orders")
                .orders
                .push_back(order);
        }
        if !applied.is_empty() {
            self.pace_formations(&applied);
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
            if self.entity(id).is_some_and(|e| e.container_id.is_some()) {
                continue;
            }
            self.advance_executor(id);
            self.fire_turret(id);
            self.heal_idle(id);
            let new = self.entity(id).expect("executor persists").pos;
            if old != new {
                self.relocate(id, old, new);
                moved.push(id);
            }
        }
        if !self.disbanded.is_empty() {
            self.remove_disbanded();
            moved.retain(|id| self.entity(*id).is_some());
        }
        self.run_path_searches();
        self.separate_units(&moved);
        self.depart_ships();
        self.seen.clear();
        self.fire_shots();
        self.land_projectiles();
        self.apply_attrition();
        self.remove_dead();
        self.fade_stains();
        self.forget_blasts();
        self.update_captures();
        self.remember_touched();
        self.update_claims();
        self.reveal_routes();
        self.pay_income();
        for surface in &mut self.surfaces {
            surface.pathing.end_tick();
        }
        self.tick = self.tick.checked_add(1).expect("simulation tick exhausted");
        self.arrive_transits();
        self.update_victory();
    }
}

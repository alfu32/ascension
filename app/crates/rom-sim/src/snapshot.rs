//! The complete simulation state as plain data: [`World::snapshot`] exports it and
//! [`World::restore`] rebuilds a world from it and the rules.
//!
//! Everything the next tick reads is carried. Caches that follow from it are rebuilt on
//! restore (spatial buckets, path occupancy, viewer counts, income rates, motion cache,
//! the economic radius index of a surface's slots); the restored world's `hash()` equals
//! the exported world's, which the save tests check.

use crate::tiles::Tiles;
use crate::{
    BodyId, BodyInfo, Entity, EntityId, MapGrid, MatchOptions, Order, OrderId, OrderStatus,
    Outcome, Player, PlayerId, PlayerStatus, Pos, RememberedBuilding, RouteInfo, SimError, SimRng,
    Stats, SystemInfo, Topology, TradeRoute, World,
    avoidance::Motion,
    capture::CaptureState,
    combat::{PendingHit, PendingShot},
    construction::footprint,
    order::MoveState,
    pathing::{PathRequest, Search},
    space::{Orbit, OrbitalShot, Space, Transit},
    territory::{Emitter, Territory},
    victory::VictoryState,
    vision::{PlayerVision, Viewer, Vision},
    wealth::RouteState,
};
use rom_geo::CubeSurface;
use rom_rules::{DefId, Rules, VictoryRule};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
    sync::Arc,
};

/// One income contribution: player and resource, and the amount per period.
pub type IncomeTerm = ((PlayerId, DefId), i64);

/// A tile reference: face and tile coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileRef {
    /// Cube face; 0 on a flat map.
    pub face: u8,
    /// Tile column.
    pub x: i32,
    /// Tile row.
    pub y: i32,
}

/// The shape of a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceShape {
    /// A rectangle of tiles.
    Flat {
        /// Width in tiles.
        width: u32,
        /// Height in tiles.
        height: u32,
    },
    /// A cube sphere with `n` tiles along each face edge.
    Cube {
        /// Tiles along a face edge.
        n: u32,
    },
}

/// A route-driven trader order's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSnapshot {
    /// The accepted route.
    pub info: TradeRoute,
    /// Resource paid per trip.
    pub resource: DefId,
    /// Whether the trader is walking to the target city.
    pub to_target: bool,
    /// Whether the first delivery trip has started.
    pub started: bool,
}

/// Walking progress of an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveSnapshot {
    /// Where the current segment started.
    pub origin: Pos,
    /// Length of the current segment.
    pub distance: u64,
    /// Distance covered along the segment.
    pub traveled: u64,
    /// Final point of this walker.
    pub goal: Pos,
    /// Intermediate points.
    pub waypoints: Vec<Pos>,
    /// Segments completed.
    pub index: usize,
    /// Search the walker waits for.
    pub request: Option<u64>,
    /// Repaths since the last waypoint.
    pub repaths: u32,
}

/// One live or pending order with all its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSnapshot {
    /// Order identity.
    pub id: OrderId,
    /// Issuing player.
    pub player: PlayerId,
    /// Executor.
    pub executor: EntityId,
    /// Order vocabulary key.
    pub kind: String,
    /// Subject definition.
    pub subject: Option<DefId>,
    /// Entity target.
    pub target_entity: Option<EntityId>,
    /// Point target.
    pub target_pos: Option<Pos>,
    /// Lifecycle state.
    pub status: OrderStatus,
    /// Ticks advanced.
    pub progress_ticks: u64,
    /// Tick of submission.
    pub issued_tick: u64,
    /// Captured duration.
    pub total_ticks: Option<u64>,
    /// Resources paid at submission.
    pub payment: Vec<(DefId, i64)>,
    /// Fractional construction work.
    pub build_remainder: i64,
    /// Created by automatic acquisition.
    pub automatic: bool,
    /// Position to return to; a patrol's other end.
    pub home: Option<Pos>,
    /// Formation pace: walking speed cap.
    pub pace: Option<i64>,
    /// The enemy a guard is fighting off.
    pub engaged: Option<EntityId>,
    /// Trade route state.
    pub route: Option<RouteSnapshot>,
    /// Walking state.
    pub movement: Option<MoveSnapshot>,
}

/// An entity with all its component state and its order queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitySnapshot {
    /// Entity identity.
    pub id: EntityId,
    /// Type definition.
    pub type_id: DefId,
    /// Owner.
    pub owner: PlayerId,
    /// Position.
    pub pos: Pos,
    /// Facing in degrees.
    pub facing: u16,
    /// Hit points.
    pub hp: i64,
    /// Maximum hit points.
    pub max_hp: i64,
    /// Line of sight.
    pub los: i64,
    /// Stance definition.
    pub stance: DefId,
    /// Weapon id and the tick it is ready.
    pub cooldowns: Vec<(u32, u64)>,
    /// Energy.
    pub energy: i64,
    /// Energy remainder.
    pub energy_remainder: i64,
    /// Heal remainder.
    pub heal_remainder: i64,
    /// Attrition remainder.
    pub attrition_remainder: i64,
    /// Neutralised building.
    pub neutralised: bool,
    /// Assimilation end tick.
    pub assimilation_until: Option<u64>,
    /// Credited killer.
    pub killer: Option<PlayerId>,
    /// Construction ticks done; None when complete.
    pub build_progress: Option<u32>,
    /// Movement remainder.
    pub move_remainder: i128,
    /// Gather capacity.
    pub gather_slots: i64,
    /// Occupied gather site.
    pub work_site: Option<EntityId>,
    /// Container.
    pub container_id: Option<EntityId>,
    /// City.
    pub city_id: Option<EntityId>,
    /// Idle ticks.
    pub idle_ticks: u64,
    /// The enemy a defence keeps firing at.
    pub aim: Option<EntityId>,
    /// Orders from active to last.
    pub orders: Vec<OrderSnapshot>,
}

/// A ship between systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitSnapshot {
    /// The ship.
    pub entity: EntitySnapshot,
    /// Origin system.
    pub origin: u32,
    /// Destination system.
    pub destination: u32,
    /// Departure tick.
    pub departed: u64,
    /// Arrival tick.
    pub arrives: u64,
}

/// A ship's orbit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitSnapshot {
    /// The ship.
    pub entity: EntityId,
    /// Orbited body.
    pub body: BodyId,
    /// Surface subpoint.
    pub pos: Pos,
    /// Facing.
    pub facing: u16,
    /// Movement remainder.
    pub move_remainder: i64,
    /// Last orbital shot tick and target.
    pub last_shot: Option<(u64, Pos)>,
}

/// System space: body positions, markers, claims, orbits, transits and landed ships.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpaceSnapshot {
    /// Orbiting ships.
    pub orbits: Vec<OrbitSnapshot>,
    /// Body positions in system space.
    pub positions: Vec<(BodyId, Pos)>,
    /// Marker entity per body.
    pub markers: Vec<(BodyId, EntityId)>,
    /// Claimant per resource body.
    pub claims: Vec<(BodyId, PlayerId)>,
    /// Ships between systems.
    pub transits: Vec<TransitSnapshot>,
    /// Completed route legs.
    pub transits_done: u64,
    /// Landed ships.
    pub landed: Vec<EntityId>,
    /// Routes each player has charted or had revealed: player, then the two systems (lower
    /// id first), ascending.
    pub charted: Vec<(PlayerId, u32, u32)>,
    /// The system each ship last arrived from, by ship id.
    pub arrivals: Vec<(EntityId, u32)>,
}

/// An in-flight path request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRequestSnapshot {
    /// Request id.
    pub id: u64,
    /// Movement class.
    pub class: u32,
    /// Start tile.
    pub start: TileRef,
    /// Goal tile.
    pub goal: TileRef,
    /// Walkers waiting for the result.
    pub waiters: Vec<(EntityId, OrderId)>,
}

/// A search node of the active search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchNodeSnapshot {
    /// Tile index.
    pub tile: u32,
    /// Cost so far.
    pub g: u64,
    /// Parent tile index.
    pub parent: u32,
    /// Heuristic.
    pub h: u64,
    /// Whether the node is closed.
    pub closed: bool,
}

/// The search in progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSnapshot {
    /// Request being searched.
    pub request: u64,
    /// Target tile index.
    pub target: u32,
    /// Anchor tile.
    pub anchor: TileRef,
    /// Open list entries `(f, g, tile)`, sorted ascending.
    pub open: Vec<(u64, u64, u32)>,
    /// Closest expanded tile: squared distance and index.
    pub best: (u64, u32),
    /// Nodes expanded.
    pub expanded: u64,
    /// Generation stamp.
    pub generation: u32,
    /// Nodes stamped with the generation, ascending by tile.
    pub nodes: Vec<SearchNodeSnapshot>,
}

/// A surface's path-search state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PathSnapshot {
    /// Next request id.
    pub next_request: u64,
    /// Search generation counter.
    pub generation: u32,
    /// Searches started.
    pub searches: u64,
    /// Nodes expanded.
    pub nodes: u64,
    /// Repaths.
    pub repaths: u64,
    /// Queued requests.
    pub queue: Vec<PathRequestSnapshot>,
    /// Active search.
    pub search: Option<SearchSnapshot>,
}

/// One player's fog on one surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVisionSnapshot {
    /// The player.
    pub player: PlayerId,
    /// One bit per tile; empty when no tile is explored (task 99).
    pub explored: Vec<u64>,
    /// Remembered buildings by position tile.
    pub memory: Vec<(u32, RememberedBuilding)>,
}

/// One body's surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceSnapshot {
    /// The body.
    pub body: BodyId,
    /// Its shape.
    pub shape: SurfaceShape,
    /// Terrain storage width.
    pub width: u32,
    /// Terrain storage height.
    pub height: u32,
    /// Terrain per tile.
    pub terrain: Vec<DefId>,
    /// Winning emitter per tile; empty when no tile is owned (task 99).
    pub winner: Vec<u32>,
    /// Path search state.
    pub path: PathSnapshot,
    /// Fog per player.
    pub vision: Vec<PlayerVisionSnapshot>,
    /// Tiles newly seen this tick.
    pub touched: Vec<u32>,
    /// Buildings by position tile.
    pub buildings: Vec<(u32, EntityId)>,
    /// Stains of what was destroyed there, oldest first (task 80).
    pub stains: Vec<crate::Stain>,
}

/// A viewer record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewerSnapshot {
    /// Body.
    pub body: BodyId,
    /// Centre tile index.
    pub tile: u32,
    /// Radius in milli-tiles.
    pub los: i64,
    /// Owner.
    pub owner: PlayerId,
}

/// A border emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitterSnapshot {
    /// Position.
    pub pos: Pos,
    /// Radius in milli-tiles.
    pub radius: i64,
    /// Owner.
    pub owner: PlayerId,
    /// Completion tick.
    pub completed: u64,
}

/// A player's setup and per-player counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSnapshot {
    /// Identity, team, faction, age, resources and technologies.
    pub player: Player,
    /// Home body.
    pub home: Option<BodyId>,
    /// Victory status.
    pub status: PlayerStatus,
    /// Land tiles owned (territory victory).
    pub land_owned: Option<u64>,
    /// Tiles owned over all surfaces.
    pub owned_tiles: Option<u64>,
    /// First founded city.
    pub capital: Option<EntityId>,
    /// Enemy team and start tick of a capital occupation.
    pub hold: Option<(u32, u64)>,
    /// The faction the player started with (task 76); a descent restores it.
    pub origin: Option<DefId>,
    /// The tick the player ascended, while it is ascended.
    pub ascended: Option<u64>,
    /// How many times the player has ascended.
    pub ascensions: u32,
}

/// A weapon hit waiting to land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitSnapshot {
    /// Landing tick.
    pub land_tick: u64,
    /// Attacker.
    pub attacker: EntityId,
    /// Attacker's owner.
    pub owner: PlayerId,
    /// Attacker's type.
    pub source_type: DefId,
    /// Weapon id.
    pub weapon_id: u32,
    /// Target.
    pub target: EntityId,
    /// Damage.
    pub damage: i64,
}

/// A firing decision not yet rolled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotSnapshot {
    /// Firing position.
    pub origin: Pos,
    /// Aimed position.
    pub target_pos: Pos,
    /// Accuracy.
    pub accuracy: u32,
    /// The hit if it lands.
    pub hit: HitSnapshot,
}

/// The victory state beyond per-player status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VictorySnapshot {
    /// Match options.
    pub options: MatchOptions,
    /// Timers by victory definition and team, with end ticks.
    pub timers: Vec<((DefId, u32), u64)>,
    /// First completed victory.
    pub outcome: Outcome,
    /// Ground tiles eligible for territory victory.
    pub ownable_tiles: u64,
}

/// Complete simulation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Completed ticks.
    pub tick: u64,
    /// Random generator state.
    pub rng_state: u64,
    /// Next entity id.
    pub next_entity_id: u32,
    /// Next order id.
    pub next_order_id: u64,
    /// Counters.
    pub stats: Stats,
    /// Players in id order.
    pub players: Vec<PlayerSnapshot>,
    /// Fractional income per player and resource.
    pub income_remainders: Vec<((PlayerId, DefId), i64)>,
    /// Victory state.
    pub victory: VictorySnapshot,
    /// Star systems.
    pub systems: Vec<SystemInfo>,
    /// Bodies.
    pub bodies: Vec<BodyInfo>,
    /// Routes.
    pub routes: Vec<RouteInfo>,
    /// Constellation figure lines between systems.
    pub figure: Vec<(u32, u32)>,
    /// Surfaces in body order.
    pub surfaces: Vec<SurfaceSnapshot>,
    /// Entities standing on a surface, in id order.
    pub entities: Vec<EntitySnapshot>,
    /// Orders submitted and not yet applied, with their append flag.
    pub pending: Vec<(OrderSnapshot, bool)>,
    /// Hits waiting to land.
    pub hits: Vec<HitSnapshot>,
    /// Firing decisions not yet rolled.
    pub shots: Vec<ShotSnapshot>,
    /// System space.
    pub space: SpaceSnapshot,
    /// Per entity id: what it reveals; the vector's length is part of the state.
    pub viewers: Vec<Option<ViewerSnapshot>>,
    /// What orbiting ships reveal below them.
    pub orbit_viewers: Vec<(EntityId, ViewerSnapshot)>,
    /// Border emitters.
    pub emitters: Vec<(EntityId, EmitterSnapshot)>,
    /// City entities.
    pub cities: Vec<EntityId>,
    /// Economic radius per city.
    pub city_radii: Vec<(EntityId, i64)>,
    /// Non-city buildings widening cities: owner and position.
    pub widening: Vec<(EntityId, PlayerId, Pos)>,
    /// Income terms each entity contributes: player, resource, amount.
    pub income_terms: Vec<(EntityId, Vec<IncomeTerm>)>,
    /// Harvesters with nothing to do.
    pub idle: Vec<EntityId>,
    /// Entities whose idle status is re-evaluated.
    pub dirty: Vec<EntityId>,
    /// Ground units to check for separation next tick.
    pub awake: Vec<EntityId>,
    /// Neutralised capturable buildings.
    pub neutralised: Vec<EntityId>,
    /// Buildings being assimilated.
    pub assimilating: Vec<EntityId>,
    /// Neutralised cities held down by an attacking team: city, team, start tick.
    pub sieges: Vec<(EntityId, u32, u64)>,
    /// Rally points of producers, by producer id.
    pub rallies: Vec<(EntityId, crate::Rally)>,
    /// Each player's latest order failures, oldest first, in player order.
    pub failures: Vec<(PlayerId, Vec<crate::OrderFailure>)>,
    /// Automated colonies whose governor is paused, in id order.
    pub governors_paused: Vec<EntityId>,
    /// Dormant surfaces of resource bodies not yet brought to life, in body order (task 77).
    pub dormant: Vec<(BodyId, crate::DormantSurface)>,
    /// Bodies brought to life, in body order (task 77).
    pub living: Vec<(BodyId, crate::Living)>,
}

const RULES: [VictoryRule; 8] = [
    VictoryRule::Conquest,
    VictoryRule::Capital,
    VictoryRule::Territory,
    VictoryRule::Wonder,
    VictoryRule::Economic,
    VictoryRule::Score,
    VictoryRule::Time,
    VictoryRule::TechRace,
];

/// The rules vocabulary key of a victory rule family.
pub fn victory_rule_key(rule: VictoryRule) -> &'static str {
    match rule {
        VictoryRule::Conquest => "conquest",
        VictoryRule::Capital => "capital",
        VictoryRule::Territory => "territory",
        VictoryRule::Wonder => "wonder",
        VictoryRule::Economic => "economic",
        VictoryRule::Score => "score",
        VictoryRule::Time => "time",
        VictoryRule::TechRace => "tech_race",
    }
}

/// The victory rule family for a vocabulary key.
pub fn victory_rule_from_key(key: &str) -> Option<VictoryRule> {
    RULES
        .into_iter()
        .find(|rule| victory_rule_key(*rule) == key)
}

fn tile_ref(tile: crate::geometry::Tile) -> TileRef {
    TileRef {
        face: tile.face,
        x: tile.x,
        y: tile.y,
    }
}

fn tile_of(tile: TileRef) -> crate::geometry::Tile {
    crate::geometry::Tile::new(tile.face, tile.x, tile.y)
}

fn viewer_snapshot(v: &Viewer) -> ViewerSnapshot {
    ViewerSnapshot {
        body: v.body,
        tile: v.tile,
        los: v.los,
        owner: v.owner,
    }
}

fn viewer_from(v: ViewerSnapshot) -> Viewer {
    Viewer {
        body: v.body,
        tile: v.tile,
        los: v.los,
        owner: v.owner,
    }
}

fn hit_snapshot(h: &PendingHit) -> HitSnapshot {
    HitSnapshot {
        land_tick: h.land_tick,
        attacker: h.attacker,
        owner: h.owner,
        source_type: h.source_type,
        weapon_id: h.weapon_id,
        target: h.target,
        damage: h.damage,
    }
}

fn hit_from(h: &HitSnapshot) -> PendingHit {
    PendingHit {
        land_tick: h.land_tick,
        attacker: h.attacker,
        owner: h.owner,
        source_type: h.source_type,
        weapon_id: h.weapon_id,
        target: h.target,
        damage: h.damage,
    }
}

fn order_snapshot(o: &Order) -> OrderSnapshot {
    OrderSnapshot {
        id: o.id,
        player: o.player,
        executor: o.executor,
        kind: o.kind.clone(),
        subject: o.subject,
        target_entity: o.target_entity,
        target_pos: o.target_pos,
        status: o.status.clone(),
        progress_ticks: o.progress_ticks,
        issued_tick: o.issued_tick,
        total_ticks: o.total_ticks,
        payment: o.payment.clone(),
        build_remainder: o.build_remainder,
        automatic: o.automatic,
        home: o.home,
        pace: o.pace,
        engaged: o.engaged,
        route: o.route.as_ref().map(|r| RouteSnapshot {
            info: r.info.clone(),
            resource: r.resource,
            to_target: r.to_target,
            started: r.started,
        }),
        movement: o.movement.as_ref().map(|m| MoveSnapshot {
            origin: m.origin,
            distance: m.distance,
            traveled: m.traveled,
            goal: m.goal,
            waypoints: m.waypoints.clone(),
            index: m.index,
            request: m.request,
            repaths: m.repaths,
        }),
    }
}

fn order_from(o: OrderSnapshot) -> Order {
    Order {
        id: o.id,
        player: o.player,
        executor: o.executor,
        kind: o.kind,
        subject: o.subject,
        target_entity: o.target_entity,
        target_pos: o.target_pos,
        status: o.status,
        progress_ticks: o.progress_ticks,
        issued_tick: o.issued_tick,
        total_ticks: o.total_ticks,
        payment: o.payment,
        build_remainder: o.build_remainder,
        automatic: o.automatic,
        home: o.home,
        pace: o.pace,
        engaged: o.engaged,
        route: o.route.map(|r| {
            Box::new(RouteState {
                info: r.info,
                resource: r.resource,
                to_target: r.to_target,
                started: r.started,
            })
        }),
        movement: o.movement.map(|m| MoveState {
            origin: m.origin,
            distance: m.distance,
            traveled: m.traveled,
            goal: m.goal,
            waypoints: m.waypoints,
            index: m.index,
            request: m.request,
            repaths: m.repaths,
        }),
    }
}

fn entity_snapshot(e: &Entity) -> EntitySnapshot {
    EntitySnapshot {
        id: e.id,
        type_id: e.type_id,
        owner: e.owner,
        pos: e.pos,
        facing: e.facing,
        hp: e.hp,
        max_hp: e.max_hp,
        los: e.los,
        stance: e.stance,
        cooldowns: e.cooldowns.iter().map(|(w, t)| (*w, *t)).collect(),
        energy: e.energy,
        energy_remainder: e.energy_remainder,
        heal_remainder: e.heal_remainder,
        attrition_remainder: e.attrition_remainder,
        neutralised: e.neutralised,
        assimilation_until: e.assimilation_until,
        killer: e.killer,
        build_progress: e.build_progress,
        move_remainder: e.move_remainder,
        gather_slots: e.gather_slots,
        work_site: e.work_site,
        container_id: e.container_id,
        city_id: e.city_id,
        idle_ticks: e.idle_ticks,
        aim: e.aim,
        orders: e.orders.iter().map(order_snapshot).collect(),
    }
}

fn entity_from(e: EntitySnapshot) -> Entity {
    Entity {
        id: e.id,
        type_id: e.type_id,
        owner: e.owner,
        pos: e.pos,
        facing: e.facing,
        hp: e.hp,
        max_hp: e.max_hp,
        los: e.los,
        stance: e.stance,
        cooldowns: e.cooldowns.into_iter().collect(),
        energy: e.energy,
        energy_remainder: e.energy_remainder,
        heal_remainder: e.heal_remainder,
        attrition_remainder: e.attrition_remainder,
        neutralised: e.neutralised,
        assimilation_until: e.assimilation_until,
        killer: e.killer,
        build_progress: e.build_progress,
        move_remainder: e.move_remainder,
        gather_slots: e.gather_slots,
        work_site: e.work_site,
        container_id: e.container_id,
        city_id: e.city_id,
        idle_ticks: e.idle_ticks,
        aim: e.aim,
        orders: e
            .orders
            .into_iter()
            .map(order_from)
            .collect::<VecDeque<_>>(),
    }
}

impl World {
    /// The hits on their way, oldest first, each with the tick it lands in (task 102: the views
    /// draw a flight ending then).
    pub fn hits_in_flight(&self) -> impl Iterator<Item = HitSnapshot> + '_ {
        self.hits.iter().map(hit_snapshot)
    }

    /// Exports the complete simulation state. Call between ticks.
    pub fn snapshot(&self) -> Snapshot {
        let players = self
            .players
            .values()
            .map(|p| PlayerSnapshot {
                player: p.clone(),
                home: self.homes.get(&p.id).copied(),
                status: self
                    .victory
                    .statuses
                    .get(&p.id)
                    .copied()
                    .unwrap_or(PlayerStatus::Playing),
                land_owned: self.victory.land_owned.get(&p.id).copied(),
                owned_tiles: self.territory.owned.get(&p.id).copied(),
                capital: self.capture.capitals.get(&p.id).copied(),
                hold: self.capture.holds.get(&p.id).copied(),
                origin: self.ascension.origins.get(&p.id).copied(),
                ascended: self.ascension.ascended.get(&p.id).copied(),
                ascensions: self.ascensions(p.id),
            })
            .collect();
        let surfaces = self
            .surfaces
            .iter()
            .map(|s| {
                let pathing = &s.pathing;
                let search = pathing.search.as_ref().map(|search| SearchSnapshot {
                    request: search.request,
                    target: search.target,
                    anchor: tile_ref(search.anchor),
                    open: search
                        .open
                        .clone()
                        .into_sorted_vec()
                        .into_iter()
                        .map(|Reverse(entry)| entry)
                        .collect(),
                    best: search.best,
                    expanded: search.expanded,
                    generation: search.generation,
                    nodes: pathing
                        .stamp
                        .iter()
                        .enumerate()
                        .filter(|(_, stamp)| **stamp == search.generation)
                        .map(|(index, _)| SearchNodeSnapshot {
                            tile: index as u32,
                            g: pathing.g[index],
                            parent: pathing.parent[index],
                            h: pathing.h[index],
                            closed: pathing.closed[index] == search.generation,
                        })
                        .collect(),
                });
                SurfaceSnapshot {
                    body: s.body(),
                    shape: match s.geo.topology {
                        Topology::Flat { width, height } => SurfaceShape::Flat { width, height },
                        Topology::Cube(cube) => SurfaceShape::Cube { n: cube.n },
                    },
                    width: s.map.width,
                    height: s.map.height,
                    terrain: s.map.terrain.clone(),
                    winner: if s.territory.winner.holds_only(0) {
                        Vec::new()
                    } else {
                        s.territory.winner.to_vec()
                    },
                    path: PathSnapshot {
                        next_request: pathing.next_request,
                        generation: pathing.generation,
                        searches: pathing.stats.searches,
                        nodes: pathing.stats.nodes,
                        repaths: pathing.stats.repaths,
                        queue: pathing
                            .queue
                            .iter()
                            .map(|r| PathRequestSnapshot {
                                id: r.id,
                                class: r.class,
                                start: tile_ref(r.start),
                                goal: tile_ref(r.goal),
                                waiters: r.waiters.clone(),
                            })
                            .collect(),
                        search,
                    },
                    vision: s
                        .vision
                        .players
                        .iter()
                        .map(|(player, v)| PlayerVisionSnapshot {
                            player: *player,
                            explored: if v.explored.holds_only(0) {
                                Vec::new()
                            } else {
                                v.explored.to_vec()
                            },
                            memory: v.memory.iter().map(|((tile, _), b)| (*tile, *b)).collect(),
                        })
                        .collect(),
                    touched: s.vision.touched.clone(),
                    buildings: s
                        .vision
                        .buildings
                        .iter()
                        .flat_map(|(tile, ids)| ids.iter().map(move |id| (*tile, *id)))
                        .collect(),
                    stains: s.stains.iter().copied().collect(),
                }
            })
            .collect();
        let space = &self.space;
        Snapshot {
            tick: self.tick,
            rng_state: self.rng.state(),
            next_entity_id: self.next_entity_id,
            next_order_id: self.next_order_id,
            stats: self.stats.clone(),
            players,
            income_remainders: self
                .income_remainders
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            victory: VictorySnapshot {
                options: self.victory.options.clone(),
                timers: self.victory.timers.iter().map(|(k, v)| (*k, *v)).collect(),
                outcome: self.victory.outcome.clone(),
                ownable_tiles: self.victory.ownable_tiles,
            },
            systems: self.universe.systems.clone(),
            bodies: self.universe.bodies.clone(),
            routes: self.universe.routes.clone(),
            figure: self.universe.figure.clone(),
            surfaces,
            entities: self.entities().map(entity_snapshot).collect(),
            pending: self
                .pending
                .iter()
                .map(|(o, append)| (order_snapshot(o), *append))
                .collect(),
            hits: self.hits.iter().map(hit_snapshot).collect(),
            shots: self
                .shots
                .iter()
                .map(|s| ShotSnapshot {
                    origin: s.origin,
                    target_pos: s.target_pos,
                    accuracy: s.accuracy,
                    hit: hit_snapshot(&s.hit),
                })
                .collect(),
            space: SpaceSnapshot {
                orbits: space
                    .orbits
                    .iter()
                    .map(|(id, o)| OrbitSnapshot {
                        entity: *id,
                        body: o.body,
                        pos: o.pos,
                        facing: o.facing,
                        move_remainder: o.move_remainder,
                        last_shot: o.last_shot.map(|s| (s.tick, s.target)),
                    })
                    .collect(),
                positions: space.positions.iter().map(|(b, p)| (*b, *p)).collect(),
                markers: space.markers.iter().map(|(b, e)| (*b, *e)).collect(),
                claims: space.claims.iter().map(|(b, p)| (*b, *p)).collect(),
                transits: space
                    .transits
                    .values()
                    .map(|t| TransitSnapshot {
                        entity: entity_snapshot(&t.entity),
                        origin: t.origin,
                        destination: t.destination,
                        departed: t.departed,
                        arrives: t.arrives,
                    })
                    .collect(),
                transits_done: space.transits_done,
                landed: space.landed.iter().copied().collect(),
                charted: space
                    .charted
                    .iter()
                    .flat_map(|(player, routes)| routes.iter().map(|(a, b)| (*player, *a, *b)))
                    .collect(),
                arrivals: space.arrivals.iter().map(|(id, s)| (*id, *s)).collect(),
            },
            viewers: self
                .vision
                .viewers
                .iter()
                .map(|v| v.as_ref().map(viewer_snapshot))
                .collect(),
            orbit_viewers: self
                .vision
                .orbit_viewers
                .iter()
                .map(|(id, v)| (*id, viewer_snapshot(v)))
                .collect(),
            emitters: self
                .territory
                .emitters
                .iter()
                .map(|(id, e)| {
                    (
                        *id,
                        EmitterSnapshot {
                            pos: e.pos,
                            radius: e.radius,
                            owner: e.owner,
                            completed: e.completed,
                        },
                    )
                })
                .collect(),
            cities: self.territory.cities.iter().copied().collect(),
            city_radii: self.territory.radii.iter().map(|(k, v)| (*k, *v)).collect(),
            widening: self
                .territory
                .widening
                .iter()
                .map(|(id, (owner, pos))| (*id, *owner, *pos))
                .collect(),
            income_terms: self
                .economy
                .terms
                .iter()
                .map(|(id, terms)| (*id, terms.clone()))
                .collect(),
            idle: self.economy.idle.iter().copied().collect(),
            dirty: self.economy.dirty.iter().copied().collect(),
            awake: self.avoid.awake.clone(),
            neutralised: self.capture.neutralised.iter().copied().collect(),
            assimilating: self.capture.assimilating.iter().copied().collect(),
            sieges: self
                .capture
                .sieges
                .iter()
                .map(|(id, (team, since))| (*id, *team, *since))
                .collect(),
            rallies: self.rallies().collect(),
            failures: self
                .failures
                .iter()
                .map(|(player, list)| (*player, list.iter().cloned().collect()))
                .collect(),
            governors_paused: self.governors_paused.iter().copied().collect(),
            dormant: self.life.dormant.iter().map(|(b, d)| (*b, *d)).collect(),
            living: self
                .life
                .living
                .iter()
                .map(|(b, l)| (*b, l.clone()))
                .collect(),
        }
    }

    /// Rebuilds a world from a snapshot and the rules it was exported under.
    pub fn restore(rules: Arc<Rules>, snapshot: Snapshot) -> Result<Self, SimError> {
        let bad = SimError::InvalidSnapshot;
        let Snapshot {
            tick,
            rng_state,
            next_entity_id,
            next_order_id,
            stats,
            players,
            income_remainders,
            victory,
            systems,
            bodies,
            routes,
            figure,
            surfaces,
            entities,
            pending,
            hits,
            shots,
            space,
            viewers,
            orbit_viewers,
            emitters,
            cities,
            city_radii,
            widening,
            income_terms,
            idle,
            dirty,
            awake,
            neutralised,
            assimilating,
            sieges,
            rallies,
            failures,
            governors_paused,
            dormant,
            living,
        } = snapshot;
        let mut world = Self::empty(
            rules.clone(),
            0,
            players.iter().map(|p| p.player.clone()).collect(),
        )?;
        world.tick = tick;
        world.rng = SimRng::new(rng_state);
        world.next_entity_id = next_entity_id;
        world.next_order_id = next_order_id;
        world.stats = stats;
        world.income_remainders = income_remainders.into_iter().collect();
        world.universe = crate::universe::UniverseData {
            systems,
            bodies,
            routes,
            figure,
        };
        for p in &players {
            if let Some(home) = p.home {
                world.homes.insert(p.player.id, home);
            }
        }
        // Surfaces: terrain and shape first, then the per-surface state.
        for s in surfaces {
            let map = MapGrid::new(s.width, s.height, s.terrain)?;
            let topology = match s.shape {
                SurfaceShape::Flat { width, height } => Topology::Flat { width, height },
                SurfaceShape::Cube { n } => Topology::Cube(CubeSurface { n }),
            };
            world.insert_map(s.body, map, topology)?;
            let slot = world.slot(s.body);
            let surface = &mut world.surfaces[slot];
            let tiles = surface.map.terrain.len();
            if !s.winner.is_empty() {
                if s.winner.len() != tiles {
                    return Err(bad("territory grid length"));
                }
                surface.territory.winner = Tiles::from_vec(s.winner);
            }
            surface.vision.players.clear();
            for v in s.vision {
                let words = tiles.div_ceil(64);
                let explored = if v.explored.is_empty() {
                    Tiles::uniform(0, words)
                } else if v.explored.len() == words {
                    Tiles::from_vec(v.explored)
                } else {
                    return Err(bad("explored length"));
                };
                surface.vision.players.insert(
                    v.player,
                    PlayerVision {
                        count: Tiles::uniform(0, tiles),
                        explored,
                        memory: v
                            .memory
                            .into_iter()
                            .map(|(tile, b)| ((tile, b.id), b))
                            .collect(),
                    },
                );
            }
            surface.vision.touched = s.touched;
            if s.stains.iter().any(|stain| stain.pos.body != s.body)
                || s.stains.windows(2).any(|w| w[0].tick > w[1].tick)
            {
                return Err(bad("stain list"));
            }
            surface.stains = s.stains.into();
            for (tile, id) in s.buildings {
                surface.vision.buildings.entry(tile).or_default().insert(id);
            }
            let pathing = &mut surface.pathing;
            pathing.next_request = s.path.next_request;
            pathing.generation = s.path.generation;
            pathing.stats = crate::PathStats {
                searches: s.path.searches,
                nodes: s.path.nodes,
                repaths: s.path.repaths,
            };
            pathing.queue = s
                .path
                .queue
                .into_iter()
                .map(|r| PathRequest {
                    id: r.id,
                    class: r.class,
                    start: tile_of(r.start),
                    goal: tile_of(r.goal),
                    waiters: r.waiters,
                })
                .collect();
            pathing.search = None;
            if let Some(search) = s.path.search {
                pathing.workspace();
                for node in &search.nodes {
                    let index = node.tile as usize;
                    if index >= tiles {
                        return Err(bad("search node outside the surface"));
                    }
                    pathing.stamp[index] = search.generation;
                    pathing.g[index] = node.g;
                    pathing.parent[index] = node.parent;
                    pathing.h[index] = node.h;
                    if node.closed {
                        pathing.closed[index] = search.generation;
                    }
                }
                pathing.search = Some(Search {
                    request: search.request,
                    target: search.target,
                    anchor: tile_of(search.anchor),
                    open: search
                        .open
                        .into_iter()
                        .map(Reverse)
                        .collect::<BinaryHeap<_>>(),
                    best: search.best,
                    expanded: search.expanded,
                    generation: search.generation,
                });
            }
        }
        // Space first: whether an entity walks depends on its orbit and landing.
        world.space = Space {
            orbits: space
                .orbits
                .into_iter()
                .map(|o| {
                    (
                        o.entity,
                        Orbit {
                            body: o.body,
                            pos: o.pos,
                            facing: o.facing,
                            last_shot: o
                                .last_shot
                                .map(|(tick, target)| OrbitalShot { tick, target }),
                            move_remainder: o.move_remainder,
                        },
                    )
                })
                .collect(),
            positions: space.positions.into_iter().collect(),
            markers: space.markers.into_iter().collect(),
            claims: space.claims.into_iter().collect(),
            transits: space
                .transits
                .into_iter()
                .map(|t| {
                    (
                        t.entity.id,
                        Transit {
                            entity: entity_from(t.entity),
                            origin: t.origin,
                            destination: t.destination,
                            departed: t.departed,
                            arrives: t.arrives,
                        },
                    )
                })
                .collect(),
            transits_done: space.transits_done,
            landed: space.landed.into_iter().collect(),
            charted: space.charted.into_iter().fold(
                BTreeMap::new(),
                |mut charted, (player, a, b)| {
                    charted
                        .entry(player)
                        .or_insert_with(BTreeSet::new)
                        .insert(crate::jumps::route_key(a, b));
                    charted
                },
            ),
            arrivals: space.arrivals.into_iter().collect(),
        };
        let slots = next_entity_id as usize;
        world.entities = vec![None; slots];
        let mut previous = 0;
        for e in entities {
            let index = e.id.0 as usize;
            if index == 0 || index >= slots || index <= previous {
                return Err(bad("entity ids"));
            }
            previous = index;
            if !rules.entity_types.contains_key(&e.type_id) {
                return Err(SimError::UnknownType(e.type_id));
            }
            world.entities[index] = Some(entity_from(e));
        }
        if world
            .space
            .transits
            .keys()
            .any(|id| id.0 == 0 || id.0 as usize >= slots)
        {
            return Err(bad("transit ids"));
        }
        // Rebuild the caches that follow from entity state, in id order.
        world.avoid.motion.resize(slots, None);
        world.avoid.pos.resize(slots, Pos::default());
        let ids: Vec<EntityId> = world.entities().map(|e| e.id).collect();
        for id in &ids {
            let e = world.entity(*id).expect("restored entity");
            let (type_id, pos, container) = (e.type_id, e.pos, e.container_id);
            let Some(slot) = world.slot_of(pos.body) else {
                return Err(bad("entity on a body without a surface"));
            };
            if !world.surfaces[slot].geo.contains(pos) {
                return Err(bad("entity outside its surface"));
            }
            if container.is_none() {
                world.surfaces[slot].spatial.insert(*id, pos);
            }
            let et = &rules.entity_types[&type_id];
            world.surfaces[slot]
                .pathing
                .occupy(et, pos.face, footprint(et, pos), true);
            let motion = (container.is_none() && world.walks(*id))
                .then(|| Motion::of(&rules, type_id))
                .flatten();
            world.avoid.set(*id, motion, pos);
            if et.gather_site.is_some() {
                world.economy.sites.insert(*id);
            }
            if let Some(site) = world.entity(*id).and_then(|e| e.work_site) {
                world.economy.occupants.entry(site).or_default().insert(*id);
            }
        }
        let transit_entities: Vec<(EntityId, Pos)> = world
            .space
            .transits
            .values()
            .map(|t| (t.entity.id, t.entity.pos))
            .collect();
        for (id, pos) in transit_entities {
            world.avoid.set(id, None, pos);
        }
        world.avoid.awake = awake;
        for (id, terms) in income_terms {
            for (key, amount) in &terms {
                let rate = world.economy.rates.entry(*key).or_default();
                *rate += i128::from(*amount);
            }
            world.economy.terms.insert(id, terms);
        }
        world.economy.rates.retain(|_, rate| *rate != 0);
        world.economy.idle = idle.into_iter().collect();
        world.economy.dirty = dirty.into_iter().collect();
        // Vision: viewers as recorded; the counts follow from them.
        world.vision = Vision::default();
        world.vision.viewers = viewers
            .into_iter()
            .map(|v| v.map(viewer_from))
            .collect::<Vec<_>>();
        world.vision.orbit_viewers = orbit_viewers
            .into_iter()
            .map(|(id, v)| (id, viewer_from(v)))
            .collect::<BTreeMap<_, _>>();
        let all: Vec<Viewer> = world
            .vision
            .viewers
            .iter()
            .flatten()
            .chain(world.vision.orbit_viewers.values())
            .copied()
            .collect();
        for viewer in all {
            let Some(slot) = world.slot_of(viewer.body) else {
                return Err(bad("viewer on a body without a surface"));
            };
            let surface = &mut world.surfaces[slot];
            if viewer.tile as usize >= surface.map.terrain.len() {
                return Err(bad("viewer outside its surface"));
            }
            let centre = surface.geo.index_centre(viewer.tile as usize);
            let counts = &mut surface
                .vision
                .players
                .get_mut(&viewer.owner)
                .ok_or(bad("viewer without vision"))?
                .count;
            for run in surface.geo.disc(centre, viewer.los) {
                counts.update(*run.start()..*run.end() + 1, |_, count| *count += 1);
            }
        }
        world.territory = Territory {
            emitters: emitters
                .into_iter()
                .map(|(id, e)| {
                    (
                        id,
                        Emitter {
                            pos: e.pos,
                            radius: e.radius,
                            owner: e.owner,
                            completed: e.completed,
                        },
                    )
                })
                .collect(),
            owned: players
                .iter()
                .filter_map(|p| p.owned_tiles.map(|n| (p.player.id, n)))
                .collect(),
            cities: cities.into_iter().collect(),
            radii: city_radii.into_iter().collect(),
            widening: widening
                .into_iter()
                .map(|(id, owner, pos)| (id, (owner, pos)))
                .collect(),
        };
        world.capture = CaptureState {
            neutralised: neutralised.into_iter().collect(),
            assimilating: assimilating.into_iter().collect(),
            capitals: players
                .iter()
                .filter_map(|p| p.capital.map(|c| (p.player.id, c)))
                .collect(),
            holds: players
                .iter()
                .filter_map(|p| p.hold.map(|h| (p.player.id, h)))
                .collect(),
            sieges: sieges
                .into_iter()
                .map(|(id, team, since)| (id, (team, since)))
                .collect(),
        };
        world.victory = VictoryState {
            options: victory.options,
            statuses: players.iter().map(|p| (p.player.id, p.status)).collect(),
            timers: victory.timers.into_iter().collect(),
            outcome: victory.outcome,
            ownable_tiles: victory.ownable_tiles,
            land_owned: players
                .iter()
                .filter_map(|p| p.land_owned.map(|n| (p.player.id, n)))
                .collect(),
        };
        world.ascension = crate::ascension::AscensionState {
            origins: players
                .iter()
                .map(|p| (p.player.id, p.origin.unwrap_or(p.player.faction)))
                .collect(),
            ascended: players
                .iter()
                .filter_map(|p| p.ascended.map(|t| (p.player.id, t)))
                .collect(),
            times: players
                .iter()
                .filter(|p| p.ascensions > 0)
                .map(|p| (p.player.id, p.ascensions))
                .collect(),
            completing: Vec::new(),
        };
        if players
            .iter()
            .any(|p| (p.status == PlayerStatus::Ascended) != p.ascended.is_some())
        {
            return Err(bad("ascended status without its tick"));
        }
        world.pending = pending
            .into_iter()
            .map(|(o, append)| (order_from(o), append))
            .collect();
        world.rallies = rallies.into_iter().collect();
        world.governors_paused = governors_paused.into_iter().collect();
        for (body, d) in &dormant {
            if world
                .body(*body)
                .is_none_or(|b| b.kind != rom_rules::BodyKind::Resource)
                || !rules.terrains.contains_key(&d.terrain)
            {
                return Err(bad("dormant surface of a body that is not a resource body"));
            }
        }
        for (body, l) in &living {
            if world
                .surface(*body)
                .is_none_or(|s| s.tiles() != l.heights.len())
                || world
                    .body(*body)
                    .is_none_or(|b| b.kind != rom_rules::BodyKind::Habitable)
            {
                return Err(bad("living body without its surface"));
            }
        }
        world.life = crate::life::LifeState {
            dormant: dormant.into_iter().collect(),
            living: living.into_iter().collect(),
        };
        world.failures = failures
            .into_iter()
            .map(|(player, list)| (player, list.into_iter().collect()))
            .collect();
        world.rebuild_type_index();
        world.hits = hits.iter().map(hit_from).collect();
        world.shots = shots
            .iter()
            .map(|s| PendingShot {
                origin: s.origin,
                target_pos: s.target_pos,
                accuracy: s.accuracy,
                hit: hit_from(&s.hit),
            })
            .collect();
        Ok(world)
    }
}

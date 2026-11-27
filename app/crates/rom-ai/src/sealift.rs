//! The sea lift (task 167): a main army that knows enemy buildings on its own planet but can
//! walk to none of them (they stand on an island, or on another continent) is carried over by
//! sea transports trained at the dock. The transports wait at the harbour, the main army boards
//! them there, they sail to the coast nearest the enemy city and put the troops ashore, then
//! come home for the next wave. Troops ashore attack what they can reach there. While no enemy
//! is known anywhere the army could walk or sail to, a transport looks for one: it sails in turn
//! to the water nearest eight points two faces of the planet from the capital (where a planet's
//! other starts lie), then eight points one face away.
//!
//! A sea transport is a unit whose container takes units and whose movement keeps to a planet's
//! surface (`World::is_sea_carrier_type`); who may board is the container's tag
//! (`World::may_ride`). Nothing names a definition.

use crate::{
    memory::{Army, EnemySighting, Job, Memory, Phase, Role, Voyage},
    plan::{Act, Plan, Wish},
    view::{View, operational},
};
use rom_rules::{AiDifficulty, DefId, Layer, ProductionMode};
use rom_sim::{Entity, EntityId, Pos};
use std::collections::BTreeMap;

/// Score of the wish for a transport while the army waits for one: above most soldiers, below
/// the economy and the age path.
const TRANSPORT_SCORE: i64 = 610;
/// Score of the wish for a dock while the army waits for transports and the player has none.
const DOCK_SCORE: i64 = 640;
/// The fewest units of the main army that make a wave worth ferrying.
const MIN_FERRY: usize = 6;
/// Ticks a loading transport waits at the harbour before it sails with what it has.
const LOAD_TICKS: u64 = 1800;
/// Ticks a voyage stage may take before it is given up.
const STAGE_TICKS: u64 = 9600;
/// Tiles around a dock searched for the water where transports load.
const HARBOUR_TILES: i32 = 5;
/// How near the harbour a transport counts as there, in milli-tiles.
const HARBOUR_SLACK: u64 = 2500;
/// Tiles around an enemy building searched for water off a beach troops can walk on from.
const BEACH_TILES: i32 = 30;
/// Tiles from the water to the land of a beach.
const BEACH_STEP: i32 = 2;
/// The most transports kept.
const MAX_TRANSPORTS: i64 = 3;
/// Enemy buildings looked at for a beach in one thought, cities and the nearest first.
const BEACH_LOOKS: usize = 8;
/// Tiles around a search point searched for water.
const SEARCH_WATER_TILES: i32 = 24;
/// How near its search point a transport counts as there, in milli-tiles.
const SEARCH_SLACK: u64 = 6000;
/// Eight directions, × 1000.
const DIRECTIONS: [(i64, i64); 8] = [
    (1000, 0),
    (707, 707),
    (0, 1000),
    (-707, 707),
    (-1000, 0),
    (-707, -707),
    (0, -1000),
    (707, -707),
];

/// What the sea lift knows in one thought.
#[derive(Debug, Default)]
pub(crate) struct Lift {
    /// The transport type to train: the trainable one with the most room.
    pub transport: Option<DefId>,
    /// A transport type the player has or could train: what the sea is measured for.
    pub probe: Option<DefId>,
    /// Water beside an own dock where transports load, or before a dock the water nearest
    /// the capital.
    pub harbour: Option<Pos>,
    /// Whether an own dock stands beside the harbour.
    pub dock: bool,
    /// A building type the player could construct that trains transports.
    pub dock_type: Option<DefId>,
    /// The enemy building the army is carried to, and the land it is put ashore on.
    pub target: Option<(u32, Pos)>,
    /// No known enemy building on the home planet can be reached on foot or by sea: the sea is
    /// explored.
    pub search: bool,
}

/// Own sea transports that can take orders, in id order.
fn transports<'a>(view: &View<'a>) -> Vec<&'a Entity> {
    view.own
        .iter()
        .copied()
        .filter(|e| {
            view.world.is_sea_carrier_type(e.type_id)
                && operational(e)
                && e.hp > 0
                && e.container_id.is_none()
        })
        .collect()
}

/// Sea transport types trainable now at an own producer, in id order.
fn trainable(view: &View<'_>) -> Vec<DefId> {
    let rules = view.rules;
    let mut types: Vec<DefId> = view
        .owned
        .iter()
        .flat_map(|p| rules.production.get(p).into_iter().flatten())
        .filter(|row| row.mode == ProductionMode::Train)
        .map(|row| row.product_def_id)
        .filter(|t| {
            view.world.is_sea_carrier_type(*t)
                && rules.is_available(*t, view.faction)
                && view.unlocked(*t)
                && !view.obsolete(*t)
        })
        .collect();
    types.sort_unstable();
    types.dedup();
    types
}

/// The room of a transport type, in population.
fn room_of(view: &View<'_>, type_id: DefId) -> i64 {
    view.rules.entity_types[&type_id]
        .container
        .as_ref()
        .map_or(0, |c| c.capacity)
}

/// Whether the tile at a position is water a ship of `probe` sails on.
fn is_water(view: &View<'_>, probe: DefId, pos: Pos) -> bool {
    let rules = view.rules;
    let Some(class) = rules.entity_types[&probe]
        .movement
        .as_ref()
        .map(|m| m.movement_class_id)
    else {
        return false;
    };
    view.world.terrain_at(pos).is_some_and(|t| {
        rules
            .terrains
            .get(&t)
            .is_some_and(|t| t.layer == Layer::Fluid)
            && rules
                .movement_class_terrain
                .get(&class)
                .into_iter()
                .flatten()
                .any(|row| row.terrain_id == t && row.speed_pct > 0)
    })
}

/// The centre of the tile a position lies in.
fn tile_centre(pos: Pos) -> Pos {
    Pos {
        x: pos.x.div_euclid(1000) * 1000 + 500,
        y: pos.y.div_euclid(1000) * 1000 + 500,
        ..pos
    }
}

/// The nearest water to `centre` within `tiles`, as a tile centre.
fn water_near(view: &View<'_>, probe: DefId, centre: Pos, tiles: i32) -> Option<Pos> {
    view.cat
        .spiral
        .iter()
        .take_while(|(dx, dy)| dx * dx + dy * dy <= tiles * tiles)
        .map(|(dx, dy)| {
            view.world
                .offset(centre, i64::from(*dx) * 1000, i64::from(*dy) * 1000)
        })
        .find(|at| is_water(view, probe, *at))
        .map(tile_centre)
}

/// Land a unit of `walker` can stand on and walk from to `goal`, next to water `probe` sails
/// on that a ship can reach from `harbour`: the beach nearest `goal` within `BEACH_TILES`.
fn beach(view: &View<'_>, probe: DefId, walker: DefId, harbour: Pos, goal: Pos) -> Option<Pos> {
    let world = view.world;
    view.cat
        .spiral
        .iter()
        .take_while(|(dx, dy)| dx * dx + dy * dy <= BEACH_TILES * BEACH_TILES)
        .map(|(dx, dy)| world.offset(goal, i64::from(*dx) * 1000, i64::from(*dy) * 1000))
        .filter(|land| world.reachable(walker, *land, goal) == Some(true))
        .find(|land| {
            [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (-1, -1),
                (1, -1),
                (-1, 1),
            ]
            .iter()
            .flat_map(|(dx, dy)| (1..=BEACH_STEP).map(move |k| (dx * k, dy * k)))
            .map(|(dx, dy)| world.offset(*land, i64::from(dx) * 1000, i64::from(dy) * 1000))
            .any(|water| {
                is_water(view, probe, water) && world.reachable(probe, harbour, water) == Some(true)
            })
        })
        .map(tile_centre)
}

/// The main army's units at home that may board a transport of `probe`, in id order; while it
/// is out (`gathered`), none.
fn waiting<'a>(view: &View<'a>, memory: &Memory, probe: DefId, gathered: bool) -> Vec<&'a Entity> {
    let Some(army) = memory.armies.get(&Role::Attack) else {
        return Vec::new();
    };
    if gathered && army.phase != Phase::Gather {
        return Vec::new();
    }
    let home = view.cities.first().map(|c| c.pos.body);
    view.own
        .iter()
        .copied()
        .filter(|e| {
            army.members.contains(&e.id.0)
                && Some(e.pos.body) == home
                && e.container_id.is_none()
                && operational(e)
                && e.hp > 0
                && view.world.may_ride(probe, e.type_id)
        })
        .collect()
}

/// The most common type among `units`, ties to the lower id.
fn walker(units: &[&Entity]) -> Option<DefId> {
    let mut count: BTreeMap<DefId, usize> = BTreeMap::new();
    for e in units {
        *count.entry(e.type_id).or_default() += 1;
    }
    count
        .into_iter()
        .max_by_key(|(t, n)| (*n, std::cmp::Reverse(*t)))
        .map(|(t, _)| t)
}

/// Whether most of `units` (each type counted once, from where its first member stands) could
/// walk to `to`.
fn walkable(view: &View<'_>, units: &[&Entity], to: Pos) -> bool {
    let mut kinds: BTreeMap<DefId, (Pos, usize)> = BTreeMap::new();
    for u in units {
        kinds.entry(u.type_id).or_insert((u.pos, 0)).1 += 1;
    }
    let (mut yes, mut all) = (0, 0);
    for (type_id, (from, count)) in kinds {
        all += count;
        if view.world.reachable(type_id, from, to) == Some(true) {
            yes += count;
        }
    }
    all > 0 && yes * 2 > all
}

impl Lift {
    /// Reads the transports, the harbour and whether the army needs to be carried over.
    pub fn new(view: &View<'_>, memory: &Memory) -> Self {
        let rules = view.rules;
        let mut lift = Lift::default();
        let Some(capital) = view.cities.first() else {
            return lift;
        };
        let types = trainable(view);
        lift.transport = types
            .iter()
            .copied()
            .max_by_key(|t| (room_of(view, *t), std::cmp::Reverse(*t)));
        // A type the player could train some day when none is trainable yet: the dock waits
        // for an age, but the plan is made.
        lift.probe = lift
            .transport
            .or_else(|| transports(view).first().map(|e| e.type_id))
            .or_else(|| {
                rules
                    .entity_types
                    .values()
                    .filter(|t| {
                        view.world.is_sea_carrier_type(t.id)
                            && rules.is_available(t.id, view.faction)
                            && view.unlocked(t.id)
                    })
                    .map(|t| t.id)
                    .min()
            });
        let Some(probe) = lift.probe else {
            return lift;
        };
        lift.dock_type = crate::construction::constructible(view)
            .into_iter()
            .filter(|b| {
                rules.production.get(b).into_iter().flatten().any(|row| {
                    row.mode == ProductionMode::Train
                        && view.world.is_sea_carrier_type(row.product_def_id)
                })
            })
            .min();
        let docks = view.own.iter().copied().filter(|e| {
            operational(e)
                && e.pos.body == capital.pos.body
                && rules
                    .production
                    .get(&e.type_id)
                    .into_iter()
                    .flatten()
                    .any(|row| {
                        row.mode == ProductionMode::Train
                            && view.world.is_sea_carrier_type(row.product_def_id)
                    })
        });
        let harbour = docks
            .filter_map(|d| water_near(view, probe, d.pos, HARBOUR_TILES))
            .next();
        lift.dock = harbour.is_some();
        lift.harbour = harbour.or_else(|| {
            let tiles = ((capital.border / 1000) as i32).clamp(1, crate::catalogue::SEARCH_TILES);
            water_near(view, probe, capital.pos, tiles)
        });
        let Some(harbour) = lift.harbour else {
            return lift;
        };
        let army = waiting(view, memory, probe, false);
        let ferried = memory.armies.get(&Role::Ferry);
        let Some(walker) = walker(&army) else {
            // The army is over the water already: the target stays.
            lift.target = ferried.and_then(|f| f.target.zip(f.goal));
            return lift;
        };
        let known: Vec<(u32, &EnemySighting)> = memory
            .enemies
            .iter()
            .filter(|(_, e)| e.building && e.pos.body == capital.pos.body)
            .map(|(id, e)| (*id, e))
            .collect();
        // The army walks to anything it can reach: that is the military manager's.
        if known.iter().any(|(_, e)| walkable(view, &army, e.pos)) {
            return lift;
        }
        // Cities first, the nearest to the harbour; the beach found for the last target is kept.
        let mut candidates: Vec<(bool, u64, u32, Pos)> = known
            .iter()
            .map(|(id, e)| {
                (
                    !view.is_city(e.type_id),
                    view.distance(harbour, e.pos),
                    *id,
                    e.pos,
                )
            })
            .collect();
        candidates.sort_unstable_by_key(|(city, distance, id, _)| (*city, *distance, *id));
        for (_, _, id, at) in candidates.into_iter().take(BEACH_LOOKS) {
            if let Some(f) = ferried.filter(|f| f.target == Some(id))
                && let Some(goal) = f.goal
            {
                lift.target = Some((id, goal));
                return lift;
            }
            if let Some(land) = beach(view, probe, walker, harbour, at) {
                lift.target = Some((id, land));
                return lift;
            }
        }
        // Only once the home land is known to its last corner does the enemy lie over the sea.
        lift.search = known_land(view, walker, capital.pos);
        lift
    }
}

/// Whether the player has seen every cell of the map (`view::CELL` square) whose centre a unit
/// of `walker` could walk to from `home`.
fn known_land(view: &View<'_>, walker: DefId, home: Pos) -> bool {
    let world = view.world;
    let topology = world
        .surface(home.body)
        .map(|s| s.topology())
        .unwrap_or_else(|| world.topology());
    let (faces, width, height) = match topology {
        rom_sim::Topology::Flat { width, height } => (1_u8, width as i32, height as i32),
        rom_sim::Topology::Cube(cube) => (6, cube.n as i32, cube.n as i32),
    };
    let cell = crate::view::CELL;
    let tiles = cell / 1000;
    (0..faces).all(|face| {
        (0..(height + tiles - 1) / tiles).all(|y| {
            (0..(width + tiles - 1) / tiles).all(|x| {
                let at = Pos::on(home.body, face, x * cell + cell / 2, y * cell + cell / 2);
                world.reachable(walker, home, at) != Some(true)
                    || world.is_explored_on_body(
                        view.me,
                        home.body,
                        face,
                        at.x.div_euclid(1000),
                        at.y.div_euclid(1000),
                    )
            })
        })
    })
}

/// Wishes of the sea lift: a dock while there is none, and transports for the waiting army.
pub(crate) fn wishes(view: &View<'_>, lift: &Lift, memory: &Memory, wishes: &mut Vec<Wish>) {
    if lift.target.is_none() && !lift.search {
        return;
    }
    if !lift.dock {
        if let Some(dock) = lift.dock_type
            && view.line_count(dock) == 0
        {
            wishes.push(Wish {
                score: DOCK_SCORE,
                act: Act::Construct(dock),
                reserve: false,
                opening: false,
            });
        }
        return;
    }
    let Some(transport) = lift.transport else {
        return;
    };
    let room = room_of(view, transport).max(1);
    let army: i64 = memory
        .armies
        .get(&Role::Attack)
        .map_or(0, |a| a.members.len() as i64);
    // One explores; for an army, enough for the wave, three at most.
    let want = if lift.target.is_some() {
        ((army + room - 1) / room).clamp(1, MAX_TRANSPORTS)
    } else {
        1
    };
    if view.line_count(transport) < want {
        wishes.push(Wish {
            score: TRANSPORT_SCORE,
            act: Act::Train(transport),
            reserve: false,
            opening: false,
        });
    }
}

/// Leads the transports and the units they carried for one thought.
pub(crate) fn command(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    lift: &Lift,
    difficulty: &AiDifficulty,
) {
    let tick = plan.view.tick;
    let mut ferry = memory.armies.remove(&Role::Ferry).unwrap_or_default();
    let alive = |plan: &Plan<'_>, id: u32| {
        plan.view
            .world
            .entity(EntityId(id))
            .is_some_and(|e| e.owner == plan.view.me && e.hp > 0)
    };
    ferry.members.retain(|id| alive(plan, *id));
    if let Some((target, goal)) = lift.target {
        ferry.target = Some(target);
        ferry.goal = Some(goal);
    }
    let ships = transports(&plan.view);
    let ids: Vec<u32> = ships.iter().map(|e| e.id.0).collect();
    memory
        .voyages
        .retain(|id, v| v.job != Job::Ferry || ids.contains(id));
    ashore(plan, memory, &mut ferry);
    let Some(harbour) = lift.harbour else {
        keep(memory, ferry);
        return;
    };
    // During the grace period no wave sets off.
    let grace = tick < difficulty.grace_ticks.max(0) as u64;
    for ship in ships {
        let voyage = memory
            .voyages
            .get(&ship.id.0)
            .copied()
            .filter(|v| v.job == Job::Ferry);
        let Some(voyage) = voyage else {
            match lift.target {
                Some((target, _)) if !grace => {
                    memory.voyages.insert(
                        ship.id.0,
                        Voyage {
                            job: Job::Ferry,
                            body: target,
                            stage: 0,
                            since: tick,
                        },
                    );
                }
                _ if lift.search => search(plan, &mut ferry, ship, harbour),
                _ if ship.orders().is_empty() => home(plan, ship, harbour),
                _ => {}
            }
            continue;
        };
        let stale = tick >= voyage.since + STAGE_TICKS;
        let next = |memory: &mut Memory, stage: u8| {
            memory.voyages.insert(
                ship.id.0,
                Voyage {
                    stage,
                    since: tick,
                    ..voyage
                },
            );
        };
        let aboard = plan.view.world.passengers(ship.id);
        match voyage.stage {
            0 => {
                if plan.view.distance(ship.pos, harbour) > HARBOUR_SLACK {
                    if stale {
                        memory.voyages.remove(&ship.id.0);
                    } else {
                        home(plan, ship, harbour);
                    }
                    continue;
                }
                let room =
                    plan.view.world.carry_capacity(ship.id) - plan.view.world.carried_load(ship.id);
                let army = waiting(&plan.view, memory, ship.type_id, true);
                let walkers: Vec<&Entity> = plan
                    .view
                    .own
                    .iter()
                    .copied()
                    .filter(|e| boarding(e, ship.id))
                    .collect();
                let walking = walkers.len();
                // Room left for those not yet on their way.
                let free = room - walkers.iter().map(|e| pop(plan, e)).sum::<i64>();
                let loaded = !aboard.is_empty()
                    && walking == 0
                    && (room <= 0 || tick >= voyage.since + LOAD_TICKS || army.is_empty());
                if loaded && lift.target.is_some() {
                    next(memory, 1);
                    continue;
                }
                // The wave waits for enough of the army, unless it fills the ship.
                let enough = army.len() + aboard.len() + walking >= MIN_FERRY
                    || army.iter().map(|e| pop(plan, e)).sum::<i64>() >= free;
                if lift.target.is_some() && enough && free > 0 {
                    board(plan, memory, &mut ferry, ship, &army, free);
                }
            }
            1 => {
                if aboard.is_empty() {
                    next(memory, 2);
                    continue;
                }
                let Some(goal) = ferry.goal else {
                    next(memory, 2);
                    continue;
                };
                if stale {
                    // Stuck: home with them.
                    next(memory, 2);
                    continue;
                }
                let unloading = ship.orders().front().is_some_and(|o| o.kind == "unload");
                if !unloading && !plan.view.busy.contains(&ship.id) {
                    plan.order(ship.id, "unload", None, None, Some(goal));
                }
            }
            _ => {
                if plan.view.distance(ship.pos, harbour) > HARBOUR_SLACK && !stale {
                    home(plan, ship, harbour);
                    continue;
                }
                if !aboard.is_empty() {
                    // Brought back: they step ashore at home and join the army again.
                    if plan.view.world.shore_in_reach(ship.id) && !plan.view.busy.contains(&ship.id)
                    {
                        plan.order(ship.id, "unload", None, Some(ship.id), None);
                    }
                    continue;
                }
                memory.voyages.remove(&ship.id.0);
            }
        }
    }
    keep(memory, ferry);
}

/// Puts the sea lift's army back into memory while it holds anything.
fn keep(memory: &mut Memory, ferry: Army) {
    if ferry != Army::default() {
        memory.armies.insert(Role::Ferry, ferry);
    }
}

/// Sends a transport to look for the enemy: to the water nearest the next search point it can
/// sail to (`Army::peak` of the sea lift counts the points visited), once it is there or idle.
fn search(plan: &mut Plan<'_>, ferry: &mut Army, ship: &Entity, harbour: Pos) {
    let view = &plan.view;
    if view.busy.contains(&ship.id) {
        return;
    }
    let goal = ship
        .orders()
        .front()
        .filter(|o| o.kind == "move")
        .and_then(|o| o.target_pos);
    if goal.is_some_and(|g| view.distance(ship.pos, g) > SEARCH_SLACK) {
        return;
    }
    let Some(capital) = view.cities.first().map(|c| c.pos) else {
        return;
    };
    let Some(n) = view.world.body(capital.body).and_then(|b| b.n) else {
        return;
    };
    let n = i64::from(n) * 1000;
    for step in 0..16 {
        let k = (ferry.peak.max(0) + step) as usize % 16;
        let (dx, dy) = DIRECTIONS[k % 8];
        let far = if k < 8 { 2 * n } else { n };
        let at = view.world.offset(capital, dx * far / 1000, dy * far / 1000);
        let Some(water) = water_near(view, ship.type_id, at, SEARCH_WATER_TILES) else {
            continue;
        };
        if view.world.reachable(ship.type_id, harbour, water) != Some(true)
            || view.distance(ship.pos, water) <= SEARCH_SLACK
        {
            continue;
        }
        ferry.peak = (k + 1) as i64;
        plan.order(ship.id, "move", None, None, Some(water));
        return;
    }
}

/// Population of a unit.
fn pop(plan: &Plan<'_>, e: &Entity) -> i64 {
    plan.view.rules.entity_types[&e.type_id].pop_cost.max(0)
}

/// Whether a unit is walking to board `ship`.
fn boarding(e: &Entity, ship: EntityId) -> bool {
    e.orders()
        .front()
        .is_some_and(|o| o.kind == "enter" && o.target_entity == Some(ship))
}

/// Sends a transport to the harbour unless it is there or on its way.
fn home(plan: &mut Plan<'_>, ship: &Entity, harbour: Pos) {
    let view = &plan.view;
    let going = ship.orders().front().is_some_and(|o| {
        o.kind == "move"
            && o.target_pos
                .is_some_and(|p| view.distance(p, harbour) <= 1000)
    });
    if going || view.busy.contains(&ship.id) || view.distance(ship.pos, harbour) <= HARBOUR_SLACK {
        return;
    }
    plan.order(ship.id, "move", None, None, Some(harbour));
}

/// Orders main army units at home aboard a transport at the harbour, as many as it has room
/// for; they leave the main army for the sea lift.
fn board(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    ferry: &mut Army,
    ship: &Entity,
    army: &[&Entity],
    room: i64,
) {
    let mut room = room;
    for e in army {
        let need = pop(plan, e);
        if need > room {
            continue;
        }
        if boarding(e, ship.id) || plan.view.busy.contains(&e.id) {
            continue;
        }
        if !plan.order(e.id, "enter", None, Some(ship.id), None) {
            return;
        }
        room -= need;
        if let Some(main) = memory.armies.get_mut(&Role::Attack) {
            main.members.remove(&e.id.0);
        }
        ferry.members.insert(e.id.0);
    }
}

/// The units the sea lift carried: those ashore over the water attack the nearest enemy
/// building they can walk to (else the target); those back at home with nothing to do rejoin
/// the main army.
fn ashore(plan: &mut Plan<'_>, memory: &mut Memory, ferry: &mut Army) {
    let rally = memory
        .rally
        .or_else(|| plan.view.cities.first().map(|c| c.pos));
    let target = ferry
        .target
        .and_then(|id| memory.enemies.get(&id))
        .map(|e| e.pos);
    let members: Vec<&Entity> = ferry
        .members
        .iter()
        .filter_map(|id| plan.view.world.entity(EntityId(*id)))
        .filter(|e| e.container_id.is_none() && e.orders().is_empty())
        .collect();
    let mut orders: Vec<(EntityId, Pos)> = Vec::new();
    for e in members {
        let view = &plan.view;
        let at_home =
            rally.is_some_and(|r| view.world.reachable(e.type_id, e.pos, r) == Some(true));
        if at_home {
            ferry.members.remove(&e.id.0);
            memory
                .armies
                .entry(Role::Attack)
                .or_default()
                .members
                .insert(e.id.0);
            continue;
        }
        let near = memory
            .enemies
            .values()
            .filter(|x| x.building && x.pos.body == e.pos.body)
            .filter(|x| view.world.reachable(e.type_id, e.pos, x.pos) == Some(true))
            .min_by_key(|x| (view.distance(e.pos, x.pos), x.pos.face, x.pos.x, x.pos.y))
            .map(|x| x.pos)
            .or(target);
        if let Some(goal) = near {
            orders.push((e.id, goal));
        }
    }
    for (id, goal) in orders {
        if !plan.order(id, "attack_move", None, None, Some(goal)) {
            return;
        }
    }
}

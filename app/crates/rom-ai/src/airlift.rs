//! Buildings that lift off (task 170). A computer player lifts a production building (one that
//! trains units) only when it is under siege: enemy fighters it has seen lately stand near it
//! and outweigh its own fighters there, and another of its cities on the same planet stands
//! clear of enemies. The building lands beside that city. It never flies a building toward
//! enemies: a building in the air with nowhere to land is set down where it is, or as near as it
//! fits. The engines are a side research of small worth.

use crate::{army, memory::Memory, plan::Plan, view::View};
use rom_rules::{ProductionMode, StatScope};
use rom_sim::{Entity, EntityId, Pos};

/// Worth of the research that fits a line of buildings with lift-off engines, as a side
/// research (before its price).
pub(crate) const LIFT_WORTH: i64 = 40;
/// How near a building enemy fighters lay siege to it, in milli-tiles.
const SIEGE_RADIUS: u64 = 16_000;
/// How far from a city no enemy may stand for the city to be safe, in milli-tiles.
const SAFE_RADIUS: u64 = 30_000;
/// How long a sighting of an enemy unit counts, in ticks.
const RECENT_TICKS: u64 = 400;
/// Rings around a city where a landing is looked for: the nearest first, in milli-tiles.
const RINGS: [i64; 4] = [9_000, 13_000, 17_000, 21_000];
/// Points looked at on each ring.
const RING_POINTS: i64 = 16;

/// The worth of a modifier on `stat` when it fits lift-off engines: `Some(LIFT_WORTH)` for the
/// stat `lift_off`, `None` for any other.
pub(crate) fn stat_worth(stat: &str, scope: Option<StatScope>) -> Option<i64> {
    (stat == "lift_off" && scope == Some(StatScope::Entity)).then_some(LIFT_WORTH)
}

/// The power of the enemy fighters seen lately within `radius` of `at`.
fn enemy_power_near(view: &View<'_>, memory: &Memory, at: Pos, radius: u64) -> i64 {
    memory
        .enemies
        .values()
        .filter(|e| !e.building && e.tick + RECENT_TICKS >= view.tick)
        .filter(|e| e.pos.body == at.body && view.distance(e.pos, at) <= radius)
        .map(|e| crate::view::strength(view.rules, e.type_id))
        .sum()
}

/// Whether `e` is a production building: it trains units.
fn trains(view: &View<'_>, e: &Entity) -> bool {
    view.rules
        .production
        .get(&e.type_id)
        .into_iter()
        .flatten()
        .any(|p| p.mode == ProductionMode::Train)
}

/// The nearest own finished city on the planet of `from` clear of enemies, other than one
/// within the siege around `from`.
fn safe_city(view: &View<'_>, memory: &Memory, from: Pos) -> Option<Pos> {
    view.cities
        .iter()
        .filter(|c| c.complete && c.pos.body == from.body)
        .filter(|c| view.distance(c.pos, from) > SIEGE_RADIUS + SAFE_RADIUS / 2)
        .filter(|c| enemy_power_near(view, memory, c.pos, SAFE_RADIUS) == 0)
        .min_by_key(|c| (view.distance(c.pos, from), c.id))
        .map(|c| c.pos)
}

/// The point `k` of `RING_POINTS` on the square ring `r` milli-tiles about the origin, walked
/// clockwise from its north-west corner, as an offset.
fn ring_point(r: i64, k: i64) -> (i64, i64) {
    let side = 2 * r;
    let step = side * 4 * k / RING_POINTS;
    match step / side {
        0 => (-r + step, -r),
        1 => (r, -r + (step - side)),
        2 => (r - (step - 2 * side), r),
        _ => (-r, r - (step - 3 * side)),
    }
}

/// Where building `id` in the air lands near `around`: `around` itself or the first point on
/// the rings about it, nearest ring first, where its footprint passes the landing check.
fn landing_near(view: &View<'_>, id: EntityId, around: Pos) -> Option<Pos> {
    let world = view.world;
    std::iter::once(around)
        .chain(RINGS.iter().flat_map(|r| {
            (0..RING_POINTS).map(move |k| {
                let (dx, dy) = ring_point(*r, k);
                world.offset(around, dx, dy)
            })
        }))
        .find_map(|at| world.check_touch_down(id, at, 0).ok())
}

/// Lifts besieged production buildings and lands buildings in the air (at most one new lift a
/// thought).
pub(crate) fn command(plan: &mut Plan<'_>, memory: &Memory) {
    let view = &plan.view;
    let world = view.world;
    // Buildings in the air without a landing come down beside a safe city, or where they are.
    let mut landings: Vec<(EntityId, Pos)> = Vec::new();
    for (id, flight) in world.building_flights() {
        let Some(e) = world.entity(id).filter(|e| e.owner == view.me) else {
            continue;
        };
        if flight.landing.is_some() {
            continue;
        }
        let target = safe_city(view, memory, e.pos)
            .and_then(|city| landing_near(view, id, city))
            .or_else(|| landing_near(view, id, e.pos));
        if let Some(at) = target {
            landings.push((id, at));
        }
    }
    // A besieged production building with somewhere safe to go lifts off.
    let lift = view
        .own
        .iter()
        .filter(|e| e.build_progress.is_none() && !world.airborne(e.id) && trains(view, e))
        .filter(|e| {
            let enemies = enemy_power_near(view, memory, e.pos, SIEGE_RADIUS);
            if enemies <= 0 || world.check_lift_off(e.id).is_err() {
                return false;
            }
            let own: i64 = army::fighters(view)
                .iter()
                .filter(|f| f.pos.body == e.pos.body && view.distance(f.pos, e.pos) <= SIEGE_RADIUS)
                .map(|f| army::power(view, f))
                .sum();
            enemies > own
        })
        .find(|e| safe_city(view, memory, e.pos).is_some())
        .map(|e| e.id);
    for (id, at) in landings {
        plan.order(id, "touch_down", None, None, Some(at));
    }
    if let Some(id) = lift {
        plan.order(id, "lift_off", None, None, None);
    }
}

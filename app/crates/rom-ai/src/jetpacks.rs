//! Jetpacks and orbital drop (task 169). Researches that teach a jump (stat `jump_range`) or a
//! drop from orbit (stat `orbital_drop`) are weighed at a fixed worth, not by the size of their
//! modifiers. A soldier that can jump uses it when attacking: when something it cannot walk
//! through lies on its straight way to the army's goal within a jump (a wall, water, cliffs),
//! or its walk there found no way, it jumps toward the goal onto land at least two tiles nearer
//! from which the goal can be walked to. The army counts a gap narrower than a jump as crossed ([`crosses`]). A
//! transport over an enemy world whose orbit no enemy ship holds drops its infantry near the
//! nearest enemy building it remembers there before it lands.

use crate::{memory::Memory, plan::Plan, view::View};
use rom_sim::{Entity, EntityId, Pos};

/// Worth of a research that teaches a jump or a drop, as a side research (before its price).
pub(crate) const LEAP_WORTH: i64 = 200;
/// How long ago a failed walk still counts as the way being closed, in ticks.
const STUCK_TICKS: u64 = 400;
/// The least a jump must bring a soldier nearer its goal, in milli-tiles.
const GAIN: i64 = 2_000;
/// Steps between the points looked at along the way to the goal, in milli-tiles.
const STEP: i64 = 500;
/// How far from the enemy building it lands at troops are dropped, in milli-tiles.
const DROP_STAND_OFF: i64 = 8_000;

/// The worth of a modifier on `stat` when it is one of the leap stats: `Some(LEAP_WORTH)` for
/// the stat that grants the jump or the drop, `Some(0)` for its cooldown, `None` otherwise.
pub(crate) fn stat_worth(stat: &str) -> Option<i64> {
    match stat {
        "jump_range" | "orbital_drop" => Some(LEAP_WORTH),
        "jump_cooldown" => Some(0),
        _ => None,
    }
}

/// Whether units of `type_id` of this player jump across from the land `from` stands on to the
/// land `to` stands on, near `to` (a gap narrower than their jump).
pub(crate) fn crosses(view: &View<'_>, type_id: rom_rules::DefId, from: Pos, to: Pos) -> bool {
    let range = view.world.jump_range_for(view.me, type_id);
    range >= 2_000
        && crate::swimming::short_crossing(view, type_id, from, to, (range / 1000 - 1) as i32)
}

/// The point `distance` from `from` on the straight way to `to`; `None` beyond reach.
fn toward(view: &View<'_>, from: Pos, to: Pos, distance: i64) -> Option<Pos> {
    let (dx, dy) = view.world.vector(from, to)?;
    let length = rom_sim::integer_sqrt((dx * dx + dy * dy) as u128) as i64;
    (length > 0).then(|| {
        view.world
            .offset(from, dx * distance / length, dy * distance / length)
    })
}

/// Where `e` would jump to get nearer `goal`, if a jump helps: the farthest point on its way
/// within its range where it may come down, at least [`GAIN`] nearer the goal and on land the
/// goal can be walked to from; unless the way is known closed (`stuck`), only over something
/// it could not walk straight across.
fn vault_point(view: &View<'_>, e: &Entity, goal: Pos, stuck: bool) -> Option<Pos> {
    let world = view.world;
    let range = world.jump_range(e.id);
    if range <= 0 || world.jump_cooldown_left(e.id) > 0 || world.leaping(e.id) {
        return None;
    }
    let distance = view.distance(e.pos, goal) as i64;
    if distance <= range {
        return None;
    }
    let mut reach = range.min(distance) - STEP;
    while reach >= GAIN {
        if let Some(at) = toward(view, e.pos, goal, reach)
            && let Ok(landing) = world.check_jump(e.id, at)
            && (view.distance(landing, goal) as i64) + GAIN <= distance
            && world
                .distance(e.pos, landing)
                .is_some_and(|d| d as i64 <= range)
            && world.reachable_for(view.me, e.type_id, landing, goal) == Some(true)
            && (stuck || !world.walkable_line(e.id, landing))
        {
            return Some(landing);
        }
        reach -= STEP;
    }
    None
}

/// Marching soldiers jump what stands in their way to `goal` (walls, gaps), and those whose
/// walk there lately found no way jump nearer it.
pub(crate) fn vault(plan: &mut Plan<'_>, units: &[&Entity], goal: Pos) {
    let view = &plan.view;
    let tick = view.tick;
    let stuck: std::collections::BTreeSet<EntityId> = view
        .world
        .order_failures(view.me)
        .filter(|f| {
            f.tick + STUCK_TICKS >= tick
                && f.reason == "no_path"
                && matches!(f.kind.as_str(), "attack_move" | "move")
        })
        .map(|f| f.executor)
        .collect();
    let jumps: Vec<(EntityId, Pos)> = units
        .iter()
        .filter(|e| !view.busy.contains(&e.id) && e.pos.body == goal.body)
        .filter_map(|e| {
            let closed = stuck.contains(&e.id)
                || view.world.reachable_for(view.me, e.type_id, e.pos, goal) == Some(false);
            vault_point(view, e, goal, closed).map(|at| (e.id, at))
        })
        .collect();
    for (id, at) in jumps {
        if !plan.order(id, "jump", None, None, Some(at)) {
            return;
        }
    }
}

/// Drops the infantry aboard `ship`, in orbit over an enemy world whose orbit no enemy ship
/// holds, near the enemy building nearest the point below it. True while it drops: when it gave
/// the order, or one is under way.
pub(crate) fn drop_troops(plan: &mut Plan<'_>, memory: &Memory, ship: &Entity) -> bool {
    let view = &plan.view;
    let world = view.world;
    let Some(below) = world.orbit(ship.id).map(|o| o.pos) else {
        return false;
    };
    if ship.orders().front().is_some_and(|o| o.kind == "drop") {
        return true;
    }
    if world.droppable_passengers(ship.id).is_empty() || view.busy.contains(&ship.id) {
        return false;
    }
    // An enemy ship in this orbit: the orbit is not held.
    let contested = memory.enemies.keys().any(|id| {
        world
            .orbit(EntityId(*id))
            .is_some_and(|o| o.body == below.body)
            && world
                .entity(EntityId(*id))
                .is_some_and(|e| world.enemies(view.me, e.owner))
    });
    if contested {
        return false;
    }
    let Some(target) = memory
        .enemies
        .values()
        .filter(|e| e.building && e.pos.body == below.body)
        .min_by_key(|e| (view.distance(below, e.pos), e.pos.face, e.pos.x, e.pos.y))
        .map(|e| e.pos)
    else {
        return false;
    };
    // Short of the building on the side of the point below, at the first spot it may drop on.
    let length = world.distance(target, below).map_or(0, |d| d as i64);
    let mut stand_off = DROP_STAND_OFF.min(length);
    let mut at = None;
    while length > 0 && stand_off <= length {
        let p = if stand_off == length {
            Some(below)
        } else {
            toward(view, target, below, stand_off)
        };
        if let Some(p) = p
            && world.check_drop(ship.id, p).is_ok()
        {
            at = Some(p);
            break;
        }
        stand_off += 2 * STEP;
    }
    let Some(at) = at.or_else(|| world.check_drop(ship.id, below).is_ok().then_some(below)) else {
        return false;
    };
    plan.order(ship.id, "drop", None, None, Some(at))
}

/// Troops a transport takes to an invasion, before the profile's share kept at home.
pub(crate) const INVASION_TROOPS: usize = 12;

/// Whether some infantry type of this player can be dropped from orbit now.
pub(crate) fn drops(view: &View<'_>) -> bool {
    view.own
        .iter()
        .any(|e| view.world.droppable_for(view.me, e.type_id))
}

/// Enemy worlds to invade, in reach and with a remembered enemy building, without an own city:
/// lowest body id first.
pub(crate) fn invasion_targets(
    view: &View<'_>,
    memory: &Memory,
    systems: &std::collections::BTreeSet<u32>,
) -> Vec<rom_sim::BodyId> {
    let mut out: Vec<rom_sim::BodyId> = memory
        .enemies
        .values()
        .filter(|e| e.building)
        .map(|e| e.pos.body)
        .filter(|b| {
            view.world.body(*b).is_some_and(|info| {
                info.kind == rom_rules::BodyKind::Habitable && systems.contains(&info.system)
            }) && !view.cities.iter().any(|c| c.pos.body == *b)
        })
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

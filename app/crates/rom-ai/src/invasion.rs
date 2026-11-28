//! Invasions of other worlds (task 173). An enemy world the player knows (a remembered enemy
//! building on a habitable planet in reach, other than its home) is weighed like a ground
//! target: its cities, capitals first, by price over the danger around them, and over the jumps
//! to get there. An invasion sets off only with enough force: the main army's troops at home
//! that a transport takes must exceed what was seen (the enemy army known, and the defence
//! around the target, or a wave lost there) by the profile's attack ratio, and be able to take
//! the target, as a ground wave must (`army::can_take`). Only what can fire on troops on the
//! ground counts in that defence: a battery that fires only at the orbit does not. It holds the
//! orbit, though, as armed enemy ships there do: while either is known the troops wait and the
//! warships strike the world first. Over the world the transport moves within its orbit to above
//! the target, then drops its infantry when the player can (task 169), else lands beside the
//! target and sets them down (task 72).
//!
//! The troops carried leave the main army for the invasion's own (`Role::Invade`): on the enemy
//! world they attack the nearest enemy building they can walk to; back at home unused they
//! rejoin the main army. A wave lost to its last unit raises the defence measured at its target
//! to the strength it set off with, so the next one waits for more.

use crate::{
    Weights,
    army::{self, fighters, power},
    fleet,
    memory::{Army, EnemySighting, Memory, Phase, Role},
    plan::Plan,
    view::{self, View},
};
use rom_rules::{BodyKind, DefId, Layer};
use rom_sim::{BodyId, Entity, EntityId, Pos};
use std::collections::BTreeSet;

/// An enemy world worth invading and the building the invasion is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Invasion {
    /// The world.
    pub body: BodyId,
    /// The enemy building the invasion is for.
    pub target: u32,
    /// Where it stood.
    pub at: Pos,
    /// The strength a wave must exceed by the attack ratio.
    pub needed: i64,
    /// Armed enemy ships hold the world's orbit: the warships go first.
    pub held: bool,
}

/// Whether a remembered enemy is a spacecraft or an orbital craft.
fn flies(view: &View<'_>, e: &EnemySighting) -> bool {
    view.world.is_spacecraft_type(e.type_id) || view.world.is_craft_type(e.type_id)
}

/// Armed enemy ships or craft remembered in the orbit of `body`.
pub(crate) fn orbit_held(view: &View<'_>, memory: &Memory, body: BodyId) -> bool {
    memory.enemies.iter().any(|(id, e)| {
        !e.building
            && flies(view, e)
            && view::strength(view.rules, e.type_id) > 0
            && view
                .world
                .orbit(EntityId(*id))
                .is_some_and(|o| o.body == body)
    })
}

/// Whether a remembered enemy battery (a building that fires up at the orbit) reaches the orbit
/// above `at`, or above the point a transport holds over to drop or land beside it.
fn covered(view: &View<'_>, memory: &Memory, at: Pos) -> bool {
    memory.enemies.values().any(|e| {
        e.building
            && e.pos.body == at.body
            && view::guards_orbit(view.rules, e.type_id)
            && view.rules.entity_types.get(&e.type_id).is_some_and(|t| {
                let reach = t
                    .weapons
                    .iter()
                    .filter(|w| !w.orbit_only && w.targets.iter().any(|l| l.layer == Layer::Orbit))
                    .map(|w| w.range_max)
                    .max()
                    .unwrap_or(0);
                view.distance(e.pos, at)
                    <= (reach.max(0) as u64).saturating_add(crate::fleet::APPROACH)
            })
    })
}

/// Whether a type has a weapon that strikes troops on the ground from where it stands.
fn fights_ground(view: &View<'_>, type_id: DefId) -> bool {
    view.rules.entity_types.get(&type_id).is_some_and(|t| {
        t.weapons
            .iter()
            .any(|w| !w.orbit_only && w.targets.iter().any(|l| l.layer == Layer::Ground))
    })
}

/// The enemy's ground army as remembered: every enemy unit seen lately that fights on the
/// ground and is neither a ship nor an orbital craft.
fn ground_army(view: &View<'_>, memory: &Memory) -> i64 {
    memory
        .enemies
        .values()
        .filter(|e| {
            !e.building
                && !flies(view, e)
                && !view.cat.is_sea_unit(e.type_id)
                && fights_ground(view, e.type_id)
        })
        .map(|e| view::strength(view.rules, e.type_id))
        .sum()
}

/// What fights troops on the ground around `at`: buildings remembered and units seen lately,
/// at full strength (as `army::defence_now` measures a city's defence, without what fires only
/// at the orbit).
fn ground_defence(view: &View<'_>, memory: &Memory, at: Pos) -> i64 {
    memory
        .enemies
        .values()
        .filter(|e| {
            e.pos.body == at.body
                && (e.building || e.tick + army::RECENT_TICKS >= view.tick)
                && view.distance(e.pos, at) <= army::THREAT_REACH
                && fights_ground(view, e.type_id)
        })
        .map(|e| view::strength(view.rules, e.type_id))
        .sum()
}

/// The enemy world best worth invading, if one is known: the building scored highest, those no
/// known battery covers first, then its cities (four times their price, a capital twice that)
/// before any other building that holds land, before any other building, over the danger around
/// it and the jumps to its system. When batteries cover every target, or armed enemy ships hold
/// the orbit, the world is `held`: the warships go first.
pub(crate) fn choose(view: &View<'_>, memory: &Memory) -> Option<Invasion> {
    let home = fleet::home(view)?;
    let legs = fleet::legs(view);
    let own: i64 = memory
        .armies
        .get(&Role::Attack)
        .map_or(0, |a| {
            fighters(view)
                .into_iter()
                .filter(|e| a.members.contains(&e.id.0) && e.pos.body == home)
                .map(|e| power(view, e))
                .sum::<i64>()
        })
        .max(1);
    let tick = view.tick;
    let ((shot, ..), id, seen) = memory
        .enemies
        .iter()
        .filter(|(id, e)| {
            e.building
                && e.pos.body != home
                && memory
                    .unreachable
                    .get(id)
                    .is_none_or(|until| *until <= tick)
        })
        .filter_map(|(id, e)| {
            let body = view.world.body(e.pos.body)?;
            if body.kind != BodyKind::Habitable {
                return None;
            }
            let jumps = i64::from(*legs.get(&body.system)?);
            let kind = if army::is_city(view, e.type_id) {
                0
            } else if army::holds_land(view, e.type_id) {
                1
            } else {
                2
            };
            let mut value = army::worth(view, e.type_id);
            if kind == 0 {
                value *= 4;
                if view.world.capital(e.owner) == Some(EntityId(*id)) {
                    value *= 2;
                }
            }
            let threat = army::threat_near(view, memory, e.pos, army::THREAT_REACH);
            let score = value * 1000 / (100 + threat * 100 / own) / (jumps + 1);
            let shot = covered(view, memory, e.pos);
            Some(((shot, kind, std::cmp::Reverse(score), *id), *id, e))
        })
        .min_by_key(|(key, _, _)| *key)?;
    // A wave lost there raised the defence measured above what stands there now: it counts.
    let measured = army::measured(memory, id, tick);
    let lost = if measured > army::defence_now(view, memory, seen.pos) {
        measured
    } else {
        0
    };
    let needed = (ground_army(view, memory) + ground_defence(view, memory, seen.pos)).max(lost);
    Some(Invasion {
        body: seen.pos.body,
        target: id,
        at: seen.pos,
        needed,
        held: shot || orbit_held(view, memory, seen.pos.body),
    })
}

/// Whether a unit is walking to board a ship.
fn boarding(e: &Entity) -> bool {
    e.orders().front().is_some_and(|o| o.kind == "enter")
}

/// The wave a transport of `carrier` with `room` free would take now: members of the gathered
/// main army at home that may ride it and are not boarding another ship, in id order, as many
/// as fit.
pub(crate) fn wave<'a>(
    view: &View<'a>,
    memory: &Memory,
    carrier: DefId,
    room: i64,
) -> Vec<&'a Entity> {
    let Some(army) = memory
        .armies
        .get(&Role::Attack)
        .filter(|a| a.phase == Phase::Gather)
    else {
        return Vec::new();
    };
    let Some(home) = fleet::home(view) else {
        return Vec::new();
    };
    let mut room = room;
    let mut out = Vec::new();
    for e in fighters(view) {
        if !army.members.contains(&e.id.0)
            || e.pos.body != home
            || boarding(e)
            || view.busy.contains(&e.id)
            || !view.world.may_ride(carrier, e.type_id)
        {
            continue;
        }
        let pop = view.rules.entity_types[&e.type_id].pop_cost.max(0);
        if pop > room {
            continue;
        }
        room -= pop;
        out.push(e);
    }
    out
}

/// Whether `units` are enough for the invasion: at least a wave's worth (or all a ship holds),
/// strong enough against what was seen there by the attack ratio, and able to take the target.
pub(crate) fn enough(
    view: &View<'_>,
    memory: &Memory,
    weights: &Weights,
    invasion: &Invasion,
    units: &[&Entity],
) -> bool {
    let Some(seen) = memory.enemies.get(&invasion.target) else {
        return false;
    };
    let own: i64 = units.iter().map(|e| power(view, e)).sum();
    let ratio = army::wave_ratio(weights, view.tick, false);
    units.len() >= army::MIN_WAVE
        && own * 100 >= ratio * invasion.needed
        && army::can_take_from(view, memory, units, seen, &|e| {
            fights_ground(view, e.type_id)
        })
}

/// Orders the wave aboard `ship`; its units leave the main army for the invasion's, which takes
/// the invasion's target and the strength that sets off.
pub(crate) fn board(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    ship: &Entity,
    room: i64,
    invasion: &Invasion,
) {
    let wave = wave(&plan.view, memory, ship.type_id, room);
    let mut invade = memory.armies.remove(&Role::Invade).unwrap_or_default();
    for e in wave {
        let strength = power(&plan.view, e);
        if !plan.order(e.id, "enter", None, Some(ship.id), None) {
            break;
        }
        if let Some(main) = memory.armies.get_mut(&Role::Attack) {
            main.members.remove(&e.id.0);
        }
        if invade.phase != Phase::Advance || invade.target != Some(invasion.target) {
            invade.phase = Phase::Advance;
            invade.target = Some(invasion.target);
            invade.goal = Some(invasion.at);
            invade.since = plan.view.tick;
            invade.peak = 0;
        }
        invade.members.insert(e.id.0);
        invade.peak += strength;
    }
    memory.armies.insert(Role::Invade, invade);
}

/// The strength of the units aboard a ship.
pub(crate) fn aboard(view: &View<'_>, ship: EntityId) -> i64 {
    view.world
        .passengers(ship)
        .iter()
        .filter_map(|id| view.world.entity(*id))
        .map(|e| power(view, e))
        .sum()
}

/// Leads the invasion's troops for one thought: on another world, those with nothing to do go
/// for the nearest enemy building they can walk to there (the target first when they can) and
/// fire on the weakest armed enemy in reach; back at home with nothing to do they rejoin the
/// main army. A wave lost to its last unit leaves its strength as the defence measured there.
pub(crate) fn lead(plan: &mut Plan<'_>, memory: &mut Memory) {
    let Some(mut invade) = memory.armies.remove(&Role::Invade) else {
        return;
    };
    let view = &plan.view;
    let tick = view.tick;
    invade.members.retain(|id| {
        view.world
            .entity(EntityId(*id))
            .is_some_and(|e| e.owner == view.me && e.hp > 0)
    });
    if invade.members.is_empty() {
        if invade.phase == Phase::Advance
            && let Some(target) = invade.target
        {
            army::note_defence(memory, target, invade.peak, tick);
        }
        return;
    }
    let home = fleet::home(view);
    let target = invade
        .target
        .and_then(|id| memory.enemies.get(&id).map(|e| (id, e.pos)));
    let mut ashore: Vec<&Entity> = Vec::new();
    let mut orders: Vec<(EntityId, Pos)> = Vec::new();
    let mut back: BTreeSet<u32> = BTreeSet::new();
    for id in &invade.members {
        let Some(e) = view.world.entity(EntityId(*id)) else {
            continue;
        };
        if e.container_id.is_some() || !view::operational(e) {
            continue;
        }
        if Some(e.pos.body) == home {
            if e.orders().is_empty() && !view.busy.contains(&e.id) {
                back.insert(*id);
            }
            continue;
        }
        ashore.push(e);
        if !e.orders().is_empty() || view.busy.contains(&e.id) {
            continue;
        }
        let reaches = |to: Pos| crate::swimming::reaches(view, e.type_id, e.pos, to) == Some(true);
        let goal = target
            .filter(|(_, at)| at.body == e.pos.body && reaches(*at))
            .map(|(_, at)| at)
            .or_else(|| {
                memory
                    .enemies
                    .values()
                    .filter(|x| x.building && x.pos.body == e.pos.body && reaches(x.pos))
                    .min_by_key(|x| (view.distance(e.pos, x.pos), x.pos.face, x.pos.x, x.pos.y))
                    .map(|x| x.pos)
            });
        if let Some(goal) = goal {
            orders.push((e.id, goal));
        }
    }
    for id in back {
        invade.members.remove(&id);
        memory
            .armies
            .entry(Role::Attack)
            .or_default()
            .members
            .insert(id);
    }
    for (id, goal) in orders {
        if !plan.order(id, "attack_move", None, None, Some(goal)) {
            break;
        }
    }
    army::focus(plan, memory, &ashore);
    if invade != Army::default() {
        memory.armies.insert(Role::Invade, invade);
    }
}

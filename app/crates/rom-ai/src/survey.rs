//! The space scout (task 173). From the first age whose map scale reaches beyond its planet, a
//! computer player keeps one cheap ship looking over the other worlds: the planets of its own
//! system first, the nearest first, then, once it may jump, the systems along its routes, those
//! fewest jumps away first; a planet, then a system, where its research places an enemy capital
//! comes before the rest. Over each world the scout holds orbit and moves the point it holds over through a
//! tour (a capital its research places there first, then the six face centres, then the eight
//! corners), so what stands below comes into sight and is remembered as any sighting is. The
//! tour ends once an enemy city is seen there. A world looked over is noted with the tick, and
//! looked at again after a while.
//! A lost scout is replaced after a pause that stretches with the pace (as task 166 does for the
//! ground scouts); the next one goes on with the tour where the lost one stopped.
//!
//! The scout is a cheap spacecraft the player can train that may leave its planet's orbit and
//! neither carries troops, launches craft nor trades, the farthest-seeing of the cheap ones, told
//! apart by its rows; nothing names a definition.

use crate::{
    fleet,
    memory::{Army, Memory, Role},
    plan::{Act, Plan, Wish},
    view::{View, operational},
};
use rom_rules::{BodyKind, DefId, ProductionMode};
use rom_sim::{BodyId, Entity, EntityId, Pos};
use std::cmp::Reverse;

/// Score of the wish for a scout when none is out: as the ground scout's (`army::SCOUT_SCORE`),
/// above the spaceport and the ships that claim bodies.
const SURVEY_SCORE: i64 = 700;
/// The pause before a lost scout is replaced, at the default pace; it stretches with the pace.
const SURVEY_WAIT_TICKS: u64 = 1200;
/// A world looked over this long ago is looked at again.
const REVISIT_TICKS: u64 = 36_000;
/// The longest the scout spends on one world, getting there included: past it, the world counts
/// as looked at (a route that closed, an orbit it cannot reach).
const WORLD_TICKS: u64 = 24_000;
/// The points of a tour, at most: the bits of `Army::peak`.
const TOUR_POINTS: usize = 15;

/// Whether a spacecraft type can scout: it leaves its planet's orbit, and is no carrier of
/// troops, launcher of craft or trader (those have work of their own).
fn suits(view: &View<'_>, type_id: DefId) -> bool {
    let world = view.world;
    let et = &view.rules.entity_types[&type_id];
    world.is_spacecraft_type(type_id)
        && !world.is_orbit_bound(type_id)
        && !world.is_craft_type(type_id)
        && et.trader.is_none()
        && et.container.as_ref().is_none_or(|c| !c.launches)
        && !(et.weapons.is_empty() && world.is_carrier_type(type_id))
}

/// The scout type: among the cheap suitable spacecraft trainable now, the one that sees
/// farthest. Cheap is at most twice the lowest price (before ramps: a type the player owns many
/// of stays cheap) and twice the lowest population of them, so in the ages where nothing costs
/// anything the population still tells. Equals by price, then id.
pub(crate) fn scout_type(view: &View<'_>) -> Option<DefId> {
    let rules = view.rules;
    let mut types: Vec<DefId> = view
        .owned
        .iter()
        .flat_map(|p| rules.production.get(p).into_iter().flatten())
        .filter(|row| row.mode == ProductionMode::Train)
        .map(|row| row.product_def_id)
        .filter(|t| {
            suits(view, *t)
                && rules.is_available(*t, view.faction)
                && view.unlocked(*t)
                && !view.obsolete(*t)
        })
        .collect();
    types.sort_unstable();
    types.dedup();
    let price = |t: &DefId| crate::army::worth(view, *t);
    let pop = |t: &DefId| rules.entity_types[t].pop_cost.max(1);
    let cheapest = types.iter().map(price).min()?;
    let fewest = types.iter().map(pop).min()?;
    types
        .into_iter()
        .filter(|t| price(t) <= 2 * cheapest && pop(t) <= 2 * fewest)
        .max_by_key(|t| (rules.entity_types[t].los, Reverse(price(t)), Reverse(*t)))
}

/// Whether a scout may be found now: always for the first; after a lost one, once the pause
/// has passed.
fn due(view: &View<'_>, army: Option<&Army>) -> bool {
    army.is_none_or(|a| a.since == 0 || view.tick >= a.since + view.paced(SURVEY_WAIT_TICKS))
}

/// An own ship of `type_id` free to become the scout: whole, on no errand, lowest id first.
fn spare(view: &View<'_>, memory: &Memory, type_id: DefId) -> Option<u32> {
    view.own
        .iter()
        .find(|e| {
            e.type_id == type_id
                && operational(e)
                && e.hp > 0
                && e.container_id.is_none()
                && !memory.voyages.contains_key(&e.id.0)
        })
        .map(|e| e.id.0)
}

/// Whether an enemy city is remembered on `body`.
fn known_city(view: &View<'_>, memory: &Memory, body: BodyId) -> bool {
    memory
        .enemies
        .values()
        .any(|e| e.building && e.pos.body == body && view.is_city(e.type_id))
}

/// The live scout, if one is out.
pub(crate) fn scout(view: &View<'_>, memory: &Memory) -> Option<u32> {
    memory
        .armies
        .get(&Role::Survey)?
        .members
        .iter()
        .copied()
        .find(|id| fleet::exists(view, *id))
}

/// Where in its system a ship stands: its own place in space, else the place of the body it
/// orbits or stands on.
fn place(view: &View<'_>, ship: &Entity) -> Option<Pos> {
    let body = view.world.orbit(ship.id).map_or(ship.pos.body, |o| o.body);
    if view
        .world
        .body(body)
        .is_some_and(|b| b.kind == BodyKind::Star)
    {
        Some(ship.pos)
    } else {
        view.world.body_position(body)
    }
}

/// The planets and the star systems where the player's research places an enemy capital
/// (`World::known_capitals`: the system once the enemy is met, the planet and the city with
/// further research).
fn capital_places(view: &View<'_>) -> (Vec<BodyId>, Vec<u32>) {
    let capitals = view.world.known_capitals(view.me);
    let enemy = || {
        capitals
            .iter()
            .filter(|c| view.world.enemies(view.me, c.owner))
    };
    (
        enemy().filter_map(|c| c.body).collect(),
        enemy().filter_map(|c| c.system).collect(),
    )
}

/// The worlds left to look at, in the order the scout visits them: those never looked over
/// first, then the longest ago; a planet where research places an enemy capital first, then
/// the planets of a system where it places one; then the fewest jumps from home; then the
/// nearest to `from`; then by id. Habitable bodies in reach other than the home planet (a
/// colony of its own sees little of its planet).
pub(crate) fn worlds(view: &View<'_>, memory: &Memory, from: Option<Pos>) -> Vec<(BodyId, bool)> {
    let home = fleet::home(view);
    let legs = fleet::legs(view);
    let (capital_bodies, capital_systems) = capital_places(view);
    let from = from.or_else(|| home.and_then(|b| view.world.body_position(b)));
    let mut out: Vec<(u64, bool, bool, u32, u64, BodyId)> = view
        .world
        .bodies()
        .iter()
        .filter(|b| b.kind == BodyKind::Habitable && Some(b.id) != home)
        .filter_map(|b| {
            let jumps = *legs.get(&b.system)?;
            let last = memory.surveyed.get(&b.id.0).copied();
            if last.is_some_and(|t| view.tick < t + REVISIT_TICKS) {
                return None;
            }
            let far = from
                .zip(view.world.body_position(b.id))
                .and_then(|(a, b)| view.world.distance(a, b))
                .unwrap_or(u64::MAX / 4);
            Some((
                last.map_or(0, |t| t + 1),
                !capital_bodies.contains(&b.id),
                !capital_systems.contains(&b.system),
                jumps,
                far,
                b.id,
            ))
        })
        .collect();
    out.sort_unstable();
    out.into_iter()
        .map(|(_, body, system, .., b)| (b, !body || !system))
        .collect()
}

/// Wishes of the scout: a ship to scout with while none is out, there is a world to look at,
/// and no ship of the type stands free or is in training.
pub(crate) fn wishes(view: &View<'_>, memory: &Memory, wishes: &mut Vec<Wish>) {
    if scout(view, memory).is_some() || !due(view, memory.armies.get(&Role::Survey)) {
        return;
    }
    let Some(type_id) = scout_type(view) else {
        return;
    };
    let live = view.own.iter().filter(|e| e.type_id == type_id).count() as i64;
    if view.have.get(&type_id).copied().unwrap_or(0) > live
        || spare(view, memory, type_id).is_some()
        || worlds(view, memory, None).is_empty()
    {
        return;
    }
    wishes.push(Wish {
        score: SURVEY_SCORE,
        act: Act::Train(type_id),
        reserve: false,
        opening: false,
    });
}

/// Leads the scout for one thought. Returns its id, which the rest of the fleet leaves alone.
pub(crate) fn command(plan: &mut Plan<'_>, memory: &mut Memory) -> Option<u32> {
    let tick = plan.view.tick;
    let mut army = memory.armies.remove(&Role::Survey).unwrap_or_default();
    let had = !army.members.is_empty();
    army.members.retain(|id| fleet::exists(&plan.view, *id));
    if had && army.members.is_empty() {
        // Lost: the next scout goes on with the tour where this one stopped, unless an enemy
        // city is known there by now.
        if let Some(body) = army.target
            && known_city(&plan.view, memory, BodyId(body))
        {
            memory.surveyed.insert(body, tick);
            army.target = None;
            army.peak = 0;
        }
        army.goal = None;
        army.since = tick;
    }
    if army.members.is_empty()
        && due(&plan.view, Some(&army))
        && let Some(type_id) = scout_type(&plan.view)
        && let Some(ship) = spare(&plan.view, memory, type_id)
    {
        army.members.insert(ship);
        army.since = tick;
        if let Some(escort) = memory.armies.get_mut(&Role::Escort) {
            escort.members.remove(&ship);
        }
    }
    let id = army.members.iter().next().copied();
    let world = plan.view.world;
    // Between systems it is out of reach of orders.
    if let Some(ship) = id
        .and_then(|id| world.entity(EntityId(id)))
        .filter(|e| operational(e))
    {
        tour(plan, memory, &mut army, ship);
    }
    if army != Army::default() {
        memory.armies.insert(Role::Survey, army);
    }
    id
}

/// The points of a world's tour with their rank: where the player's research places an enemy
/// capital there (0), then the face centres (1), then the corners, each a sight radius in from
/// it (2); at most [`TOUR_POINTS`].
fn points(view: &View<'_>, body: BodyId, inset: i32) -> Vec<(u8, Pos)> {
    let Some(n) = view.world.body(body).and_then(|b| b.n) else {
        return Vec::new();
    };
    let side = n as i32 * 1000;
    let inset = inset.clamp(500, side / 4);
    let mut out: Vec<(u8, Pos)> = view
        .world
        .known_capitals(view.me)
        .iter()
        .filter(|c| view.world.enemies(view.me, c.owner) && c.body == Some(body))
        .filter_map(|c| c.pos)
        .map(|p| (0, p))
        .collect();
    out.extend((0..6).map(|face| (1, Pos::on(body, face, side / 2, side / 2))));
    // The cube's eight corners, each once: a corner of a later face that lies near one already
    // taken is the same corner.
    let near = 3 * inset as u64;
    let mut corners: Vec<Pos> = Vec::new();
    for face in 0..6 {
        for (x, y) in [
            (inset, inset),
            (side - inset, inset),
            (inset, side - inset),
            (side - inset, side - inset),
        ] {
            let at = Pos::on(body, face, x, y);
            if !corners
                .iter()
                .any(|p| view.world.distance(*p, at).is_some_and(|d| d <= near))
            {
                corners.push(at);
            }
        }
    }
    out.extend(corners.into_iter().map(|p| (2, p)));
    out.truncate(TOUR_POINTS);
    out
}

/// Takes the scout to its world and over it: a new world when it has none, then orbit, then
/// the point of the tour not yet in sight of the lowest rank, the nearest of it first. A world
/// whose tour is done, or where an enemy city is seen, is noted.
fn tour(plan: &mut Plan<'_>, memory: &mut Memory, army: &mut Army, ship: &Entity) {
    let tick = plan.view.tick;
    if let Some(body) = army.target
        && tick >= army.since + WORLD_TICKS
    {
        memory.surveyed.insert(body, tick);
        army.target = None;
    }
    // Research placed an enemy capital since the scout set off: it goes there first.
    let list = worlds(&plan.view, memory, place(&plan.view, ship));
    let first = list.first().copied();
    if let (Some(body), Some((next, true))) = (army.target, first)
        && next.0 != body
        && !list.iter().any(|(b, capital)| b.0 == body && *capital)
    {
        army.target = None;
    }
    if army.target.is_none() {
        let Some((body, _)) = first else {
            return;
        };
        army.target = Some(body.0);
        army.goal = None;
        army.peak = 0;
        army.since = tick;
    }
    let Some(body) = army.target.map(BodyId) else {
        return;
    };
    if !fleet::fly(plan, ship, body, 0, None) {
        return;
    }
    let view = &plan.view;
    let Some(below) = view.world.orbit(ship.id).map(|o| o.pos) else {
        return;
    };
    let sight = ship.los.max(1000);
    let tour = points(view, body, (sight / 2) as i32);
    // Points in sight now are done.
    for (i, (_, p)) in tour.iter().enumerate() {
        if view.distance(below, *p) <= sight as u64 / 2 {
            army.peak |= 1 << i;
        }
    }
    // The lowest rank first, the nearest of it first.
    let next = tour
        .iter()
        .enumerate()
        .filter(|(i, _)| army.peak & (1 << i) == 0)
        .min_by_key(|(i, (rank, p))| (*rank, view.far(below, *p), *i))
        .map(|(_, (_, p))| *p);
    // An enemy city seen there: the world is known, the scout goes on to the next.
    let found = known_city(view, memory, body);
    let Some(next) = next.filter(|_| !found) else {
        memory.surveyed.insert(body.0, tick);
        army.target = None;
        army.goal = None;
        return;
    };
    let heading = ship
        .orders()
        .front()
        .is_some_and(|o| o.kind == "move" && o.target_pos == Some(next));
    if heading || view.busy.contains(&ship.id) {
        return;
    }
    if plan.order(ship.id, "move", None, None, Some(next)) {
        army.goal = Some(next);
    }
}

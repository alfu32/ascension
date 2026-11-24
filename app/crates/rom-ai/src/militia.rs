//! Militia (task 152): when an own city is raided, workers inside its economic radius are called
//! up in proportion to the threat, held at that city (never enlisted in an army), and stood
//! down once its borders have been quiet for a while. Nothing here names a definition: who can
//! be called up is the simulation's `et_call_up` data (`World::militia_change`).

use crate::{
    army,
    memory::{Memory, Raid},
    plan::Plan,
    view::{City, View, operational, strength},
};
use rom_rules::AiDifficulty;
use rom_sim::{Entity, EntityId, Pos};
use std::collections::{BTreeMap, BTreeSet};

/// Militia are called up until the defence of a raided city is this share of the strength of
/// the intruders in it.
const CALL_UP_PCT: i64 = 150;
/// At most this share of the workers inside a city's economic radius is called up: the rest
/// keep the economy going.
const MAX_SHARE_PCT: i64 = 50;
/// Ticks without an armed enemy seen in a city, and without the city losing hit points, before
/// its militia stand down: half a game minute.
pub(crate) const STAND_DOWN_TICKS: u64 = 600;
/// Ticks an enemy unit counts as present after it was last seen.
const RECENT_TICKS: u64 = 400;
/// How far beyond a city's economic radius its town reaches, in milli-tiles: sites drawing from
/// the forest and rock beside it, and the workers going there.
const NEAR: u64 = 4000;

/// How far beyond its weapons' reach an armed building counts against an intruder: a
/// building's footprint, in milli-tiles.
const GUN_SLACK: u64 = 2000;

/// How far a city's town reaches from its centre, in milli-tiles: its economic radius, where
/// its workers are, and a margin. Not the border: a city's border reaches 40 tiles, the whole
/// realm, and workers sent after raiders that far from their town were lost (traced, duel
/// seed 9: workers fell from 38 to 24).
fn reach(city: &City) -> u64 {
    city.radius.max(0) as u64 + NEAR
}

/// Armed enemy units seen lately in a city's town (`reach`), nearest to its centre first: where
/// each stood and its strength.
fn intruders(view: &View<'_>, memory: &Memory, city: &City) -> Vec<(u64, Pos, i64)> {
    let mut out: Vec<(u64, Pos, i64)> = memory
        .enemies
        .values()
        .filter(|e| {
            !e.building
                && e.tick + RECENT_TICKS >= view.tick
                && e.pos.body == city.pos.body
                && !view.cat.is_sea_unit(e.type_id)
        })
        .filter_map(|e| {
            let d = view.distance(e.pos, city.pos);
            let value = strength(view.rules, e.type_id);
            (d <= reach(city) && value > 0).then_some((d, e.pos, value))
        })
        .collect();
    out.sort_by_key(|(d, pos, _)| (*d, pos.face, pos.x, pos.y));
    out
}

/// Strength of an own unit now, scaled by its health.
fn power(view: &View<'_>, e: &Entity) -> i64 {
    army::power(view, e)
}

/// Calls up, leads and stands down the militia for one thought. `hurt` are the own cities
/// that lost hit points since the last thought.
pub(crate) fn command(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    difficulty: &AiDifficulty,
    hurt: &BTreeSet<u32>,
) {
    let tick = plan.view.tick;
    let cities: Vec<City> = plan
        .view
        .cities
        .iter()
        .filter(|c| c.complete)
        .copied()
        .collect();
    // Militia that fell or stood down are off the roll; own militia not on it (a brain
    // restored from an older save) belong to the nearest city.
    let militia: Vec<&Entity> = plan
        .view
        .own
        .iter()
        .copied()
        .filter(|e| plan.view.cat.is_militia(e.type_id) && operational(e))
        .collect();
    let alive: BTreeSet<u32> = militia.iter().map(|e| e.id.0).collect();
    memory.militia.retain(|id, _| alive.contains(id));
    for e in &militia {
        if !memory.militia.contains_key(&e.id.0)
            && let Some(city) = cities
                .iter()
                .filter(|c| c.pos.body == e.pos.body)
                .min_by_key(|c| (plan.view.distance(c.pos, e.pos), c.id))
        {
            memory.militia.insert(e.id.0, city.id.0);
        }
    }

    // Which cities are raided: an armed enemy in the town, or hit points lost.
    let mut threats: BTreeMap<u32, Vec<(u64, Pos, i64)>> = BTreeMap::new();
    for city in &cities {
        let seen = intruders(&plan.view, memory, city);
        if !seen.is_empty() || hurt.contains(&city.id.0) {
            memory
                .raids
                .entry(city.id.0)
                .or_insert(Raid {
                    since: tick,
                    last: tick,
                })
                .last = tick;
        }
        threats.insert(city.id.0, seen);
    }
    memory
        .raids
        .retain(|city, raid| threats.contains_key(city) && tick < raid.last + STAND_DOWN_TICKS);

    let reaction = difficulty.reaction_ticks.max(0) as u64;
    let fighters = army::fighters(&plan.view);
    for city in &cities {
        let Some(raid) = memory.raids.get(&city.id.0).copied() else {
            continue;
        };
        let seen = &threats[&city.id.0];
        let threat: i64 = seen.iter().map(|(_, _, s)| *s).sum();
        // Called up only after the difficulty's reaction time, and only against intruders.
        if threat > 0 && tick >= raid.since + reaction {
            call_up(plan, memory, city, seen, threat, &fighters);
        }
    }
    lead(plan, memory, &cities, &threats);
}

/// Calls up workers inside `city`'s economic radius, nearest to the intruders first, until the
/// defence of the town (fighters in it, its armed buildings, the city's guns, its militia) is
/// `CALL_UP_PCT` of the threat or `MAX_SHARE_PCT` of those workers are called up.
fn call_up(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    city: &City,
    seen: &[(u64, Pos, i64)],
    threat: i64,
    fighters: &[&Entity],
) {
    let view = &plan.view;
    let near =
        |pos: Pos, reach: u64| pos.body == city.pos.body && view.distance(pos, city.pos) <= reach;
    let mine: Vec<u32> = memory
        .militia
        .iter()
        .filter(|(_, c)| **c == city.id.0)
        .map(|(id, _)| *id)
        .collect();
    // Armed buildings count where an intruder is within their reach.
    let guns: i64 = view
        .own
        .iter()
        .filter(|e| view.is_building(e.type_id) && operational(e) && near(e.pos, reach(city)))
        .filter(|e| {
            let range = view.rules.entity_types[&e.type_id]
                .weapons
                .iter()
                .map(|w| w.range_max.max(0) as u64)
                .max()
                .unwrap_or(0);
            range > 0
                && seen
                    .iter()
                    .any(|(_, at, _)| view.distance(e.pos, *at) <= range + GUN_SLACK)
        })
        .map(|e| strength(view.rules, e.type_id) * e.hp.max(0) / e.max_hp.max(1))
        .sum();
    let mut defence: i64 = guns
        + fighters
            .iter()
            .filter(|e| near(e.pos, reach(city)))
            .map(|e| power(view, e))
            .sum::<i64>()
        + mine
            .iter()
            .filter_map(|id| view.world.entity(EntityId(*id)))
            .map(|e| power(view, e))
            .sum::<i64>();
    if defence * 100 >= threat * CALL_UP_PCT {
        return;
    }
    let radius = city.radius.max(0) as u64;
    let mut workers: Vec<&Entity> = view
        .own
        .iter()
        .copied()
        .filter(|e| {
            view.world.calls_up(e.type_id)
                && operational(e)
                && e.container_id.is_none()
                && near(e.pos, radius)
        })
        .collect();
    let room = (workers.len() as i64 + mine.len() as i64) * MAX_SHARE_PCT / 100 - mine.len() as i64;
    if room <= 0 {
        return;
    }
    let toward = seen.first().map_or(city.pos, |(_, pos, _)| *pos);
    workers.sort_by_key(|e| (view.distance(e.pos, toward), e.id));
    let mut orders: Vec<EntityId> = Vec::new();
    for e in workers {
        if orders.len() as i64 >= room || defence * 100 >= threat * CALL_UP_PCT {
            break;
        }
        if view.busy.contains(&e.id) {
            continue;
        }
        // What it would become here; a worker outside the own territory cannot be called up.
        let Ok(militia) = view.world.militia_change(e.id, rom_sim::CALL_UP) else {
            continue;
        };
        orders.push(e.id);
        defence += strength(view.rules, militia) * e.hp.max(0) / e.max_hp.max(1);
    }
    for id in orders {
        if !plan.order(id, rom_sim::CALL_UP, None, None, None) {
            return;
        }
        memory.militia.insert(id.0, city.id.0);
    }
}

/// Holds every militia at its city: against the intruder nearest the city while it is raided,
/// back home when it strayed, and stood down once the city is quiet (inside the own territory,
/// where the simulation allows it; one outside walks home first).
fn lead(
    plan: &mut Plan<'_>,
    memory: &Memory,
    cities: &[City],
    threats: &BTreeMap<u32, Vec<(u64, Pos, i64)>>,
) {
    for (id, home) in &memory.militia {
        let Some(e) = plan.view.world.entity(EntityId(*id)) else {
            continue;
        };
        if plan.view.busy.contains(&e.id) || !plan.view.cat.is_militia(e.type_id) {
            continue;
        }
        let city = cities.iter().find(|c| c.id.0 == *home).or_else(|| {
            cities
                .iter()
                .filter(|c| c.pos.body == e.pos.body)
                .min_by_key(|c| (plan.view.distance(c.pos, e.pos), c.id))
        });
        let Some(city) = city else {
            continue;
        };
        let raided = memory.raids.contains_key(&city.id.0);
        let order = e.orders().front().map(|o| o.kind.as_str());
        if raided {
            match threats.get(&city.id.0).and_then(|seen| seen.first()) {
                Some((_, at, _)) => {
                    if order != Some("attack_move") && order != Some("attack") {
                        plan.order(e.id, "attack_move", None, None, Some(*at));
                    }
                }
                None => {
                    if order.is_none()
                        && plan.view.distance(e.pos, city.pos) > city.radius.max(0) as u64 / 2
                    {
                        plan.order(e.id, "move", None, None, Some(city.pos));
                    }
                }
            }
            continue;
        }
        let inside = plan.view.world.territory_owner_on_body(
            e.pos.body,
            e.pos.face,
            e.pos.x.div_euclid(1000),
            e.pos.y.div_euclid(1000),
        ) == Some(plan.view.me);
        if inside {
            plan.order(e.id, rom_sim::STAND_DOWN, None, None, None);
        } else if order != Some("move") {
            plan.order(e.id, "move", None, None, Some(city.pos));
        }
    }
}

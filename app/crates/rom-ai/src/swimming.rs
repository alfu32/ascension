//! Swimming (task 168): a research that teaches swimming (a modifier on the stat `swim`) is
//! worth its price only where shallow water parts the player from land it wants: land within
//! its cities' reach that its workers cannot walk to, or a remembered enemy building its
//! soldiers cannot walk to across a short swim. Once learnt, workers go to sites they can swim
//! to like anywhere else (`World::reachable_for`), and the army swims to an enemy across a
//! short stretch of shallow water ([`reaches`]); a long one is left to the sea lift.

use crate::{memory::EnemySighting, view::View};
use rom_rules::{DefId, Layer, Modifier};
use rom_sim::Pos;
use std::collections::{BTreeMap, BTreeSet};

/// Score of a swimming research that opens land: above most side research.
pub(crate) const SWIM_WORTH: i64 = 200;
/// Tiles between the points of a city's reach that are looked at.
const STEP: i32 = 2;
/// The most tiles of water an army swims rather than waiting for a sea transport.
const SHORT_SWIM_TILES: i32 = 8;
/// How far around a target the shore an army would swim from is looked for, in tiles.
const LANDING_TILES: i32 = 30;

/// Whether the land `from` stands on comes within a short swim of the land `to` stands on,
/// near `to`: the tile of `from`'s land nearest `to` (within [`LANDING_TILES`]) has a tile of
/// `to`'s land within [`SHORT_SWIM_TILES`]. Lands are what units of `type_id` walk over; the
/// water between is not looked at.
fn short_crossing(view: &View<'_>, type_id: DefId, from: Pos, to: Pos) -> bool {
    let world = view.world;
    let around = |centre: Pos, tiles: i32| {
        view.cat
            .spiral
            .iter()
            .take_while(move |(dx, dy)| dx * dx + dy * dy <= tiles * tiles)
            .map(move |(dx, dy)| world.offset(centre, i64::from(*dx) * 1000, i64::from(*dy) * 1000))
    };
    around(to, LANDING_TILES)
        .find(|at| world.reachable(type_id, from, *at) == Some(true))
        .is_some_and(|shore| {
            around(shore, SHORT_SWIM_TILES).any(|at| world.reachable(type_id, at, to) == Some(true))
        })
}

/// Whether units of `type_id` of this player can get from `from` to `to`: on foot, or once
/// they swim across a short swim. `None` as for `World::reachable`.
pub(crate) fn reaches(view: &View<'_>, type_id: DefId, from: Pos, to: Pos) -> Option<bool> {
    let walk = view.world.reachable(type_id, from, to)?;
    Some(
        walk || (view.world.reachable_for(view.me, type_id, from, to) == Some(true)
            && short_crossing(view, type_id, from, to)),
    )
}

/// The swim percentages `tech` grants, with the modifier rows that grant them.
fn swim_rows<'a>(view: &View<'a>, tech: DefId) -> Vec<&'a Modifier> {
    view.rules
        .modifiers_from(tech)
        .iter()
        .filter(|m| m.stat_key == "swim" && m.value > 0)
        .collect()
}

/// Whether `tech` teaches swimming.
pub(crate) fn teaches(view: &View<'_>, tech: DefId) -> bool {
    !swim_rows(view, tech).is_empty()
}

/// Whether a modifier row applies to entities of `type_id`.
fn targets(view: &View<'_>, row: &Modifier, type_id: DefId) -> bool {
    match (row.target_def_id, row.target_tag_id) {
        (Some(def), _) => def == type_id,
        (None, Some(tag)) => view.rules.has_tag(type_id, tag),
        (None, None) => true,
    }
}

/// Whether researching `tech` would let some own unit reach land it cannot walk to now.
pub(crate) fn opens_land(
    view: &View<'_>,
    enemies: &BTreeMap<u32, EnemySighting>,
    tech: DefId,
) -> bool {
    let world = view.world;
    let rules = view.rules;
    for row in swim_rows(view, tech) {
        // One unit of each own type the row teaches and that cannot swim yet, where it stands.
        let mut seen = BTreeSet::new();
        for unit in view.own.iter().filter(|e| {
            e.container_id.is_none()
                && targets(view, row, e.type_id)
                && !world.can_swim(view.me, e.type_id)
        }) {
            if !seen.insert(unit.type_id) {
                continue;
            }
            let et = &rules.entity_types[&unit.type_id];
            let from = unit.pos;
            let opens = |to| {
                world.reachable_for(view.me, unit.type_id, from, to) == Some(false)
                    && world.reachable_swimming(unit.type_id, row.value, from, to) == Some(true)
            };
            let works = !et.harvester.is_empty() || et.builder.is_some();
            if works
                && view
                    .cities
                    .iter()
                    .filter(|c| c.complete && c.pos.body == from.body)
                    .any(|city| {
                        let reach = (city.radius / 1000) as i32;
                        view.cat
                            .spiral
                            .iter()
                            .take_while(|(dx, dy)| dx * dx + dy * dy <= reach * reach)
                            .filter(|(dx, dy)| dx % STEP == 0 && dy % STEP == 0)
                            .map(|(dx, dy)| {
                                world.offset(city.pos, i64::from(*dx) * 1000, i64::from(*dy) * 1000)
                            })
                            .any(|at| {
                                world.terrain_at(at).is_some_and(|t| {
                                    let terrain = &rules.terrains[&t];
                                    terrain.buildable && terrain.layer == Layer::Ground
                                }) && opens(at)
                            })
                    })
            {
                return true;
            }
            if !et.weapons.is_empty()
                && enemies
                    .values()
                    .filter(|x| x.building && x.pos.body == from.body)
                    .any(|x| opens(x.pos) && short_crossing(view, unit.type_id, from, x.pos))
            {
                return true;
            }
        }
    }
    false
}

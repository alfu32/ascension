//! Fortification lines and minefields (task 147b). Once enemy soldiers have come inside a
//! town's borders, the town is given one short line across the way they came, beyond its
//! economic radius, with a gate in the middle: a line open at both ends, clear of every own
//! building, resource and gather site, so the player never walls itself in. A town with its
//! line gets a small minefield before the gate once mines can be laid. What is a segment, a
//! gate or a mine comes from the `et_fortification` and `et_minefield` rows; how keen the
//! player is from `mil.defense_share_pct`.

use crate::{
    Weights,
    memory::Memory,
    plan::{Act, Outcome, Plan, RETRY_TICKS, Wish},
    view::{City, View},
};
use rom_rules::{DefId, ProductionMode};
use rom_sim::{EntityId, LineSite, Pos, integer_sqrt};

/// Score of a town's line: below every building the construction manager wants, above
/// nothing at all; the profile's defence share adds to it.
const LINE_SCORE: i64 = 150;
/// Score of a town's minefield, a little below its line.
const MINES_SCORE: i64 = 140;
/// Tiles of the line either side of its gate's centre: nine tiles across the way.
const LINE_HALF: i64 = 4;
/// How far beyond a town's economic radius its line is tried, tiles, in order.
const LINE_BEYOND: [i64; 4] = [3, 5, 2, 7];
/// Milli-tiles a line or minefield keeps from own buildings, resources and gather sites.
const CLEAR: u32 = 2500;
/// How far from a town its line and mines are looked for, milli-tiles beyond the radius.
const NEAR: i64 = 12_000;
/// Builders that share a line.
const LINE_BUILDERS: usize = 2;

/// The kind of work.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Work {
    Line,
    Mines,
}

/// The segment type, then the mine type, the player's builders can lay now.
fn kinds(view: &View<'_>) -> (Option<DefId>, Option<DefId>) {
    let rules = view.rules;
    let (mut segment, mut mine) = (None, None);
    for e in &view.own {
        if rules.entity_types[&e.type_id].builder.is_none() {
            continue;
        }
        for row in rules.production.get(&e.type_id).into_iter().flatten() {
            let def = row.product_def_id;
            if row.mode != ProductionMode::Construct
                || !view.unlocked(def)
                || view.obsolete(def)
                || !rules.is_available(def, view.faction)
            {
                continue;
            }
            let Some(et) = rules.entity_types.get(&def) else {
                continue;
            };
            if et.fortification.as_ref().is_some_and(|f| !f.gate) {
                segment = Some(segment.map_or(def, |s: DefId| s.min(def)));
            } else if et.minefield.is_some() {
                mine = Some(mine.map_or(def, |m: DefId| m.min(def)));
            }
        }
    }
    (segment, mine)
}

/// Whether the own entity `e`, or a construction ordered of `subject`, is a fortification
/// (`Work::Line`) or a mine (`Work::Mines`).
fn is_work(view: &View<'_>, type_id: DefId, work: Work) -> bool {
    view.rules
        .entity_types
        .get(&type_id)
        .is_some_and(|t| match work {
            Work::Line => t.fortification.is_some(),
            Work::Mines => t.minefield.is_some(),
        })
}

/// Whether the town has its line (or mines), standing or ordered, near it.
fn has(view: &View<'_>, city: &City, work: Work) -> bool {
    let near = |p: Pos| {
        p.body == city.pos.body
            && view
                .world
                .distance(city.pos, p)
                .is_some_and(|d| d as i64 <= city.radius + NEAR)
    };
    view.own.iter().any(|e| {
        (is_work(view, e.type_id, work) && near(e.pos))
            || e.orders().iter().any(|o| {
                o.kind == "construct"
                    && o.subject.is_some_and(|s| is_work(view, s, work))
                    && o.target_pos.is_some_and(near)
            })
    })
}

/// The unit vector (per mille) from a town towards the nearest enemy soldier it remembers
/// inside its borders: a town lays its line once the enemy has come, across the way it came.
fn approach(view: &View<'_>, memory: &Memory, city: &City) -> Option<(i64, i64)> {
    let (dx, dy) = memory
        .enemies
        .iter()
        .filter(|(_, s)| {
            !s.building
                && s.pos.body == city.pos.body
                && crate::production::is_military(view.rules, s.type_id)
        })
        .filter_map(|(id, s)| {
            let v = view.world.vector(city.pos, s.pos)?;
            let squared = v.0 * v.0 + v.1 * v.1;
            (squared <= city.border * city.border).then_some((squared, *id, v))
        })
        .min()
        .map(|(_, _, v)| v)?;
    let length = integer_sqrt((dx * dx + dy * dy) as u128) as i64;
    (length > 0).then(|| (dx * 1000 / length, dy * 1000 / length))
}

/// The own gate nearest a town, on its planet.
fn own_gate(view: &View<'_>, city: &City) -> Option<Pos> {
    view.own
        .iter()
        .filter(|e| {
            e.pos.body == city.pos.body
                && view
                    .rules
                    .entity_types
                    .get(&e.type_id)
                    .and_then(|t| t.fortification.as_ref())
                    .is_some_and(|f| f.gate)
        })
        .filter_map(|e| Some((view.world.distance(city.pos, e.pos)?, e.id)))
        .min()
        .and_then(|(_, id)| view.world.entity(id))
        .map(|e| e.pos)
}

/// The first complete town (in id order) that wants `work`, and its way to the enemy: for a
/// line the way the enemy came, for mines the way through its line's gate.
fn town<'v>(view: &'v View<'_>, memory: &Memory, work: Work) -> Option<(&'v City, (i64, i64))> {
    view.cities.iter().filter(|c| c.complete).find_map(|city| {
        let way = match work {
            Work::Line if !has(view, city, Work::Line) => approach(view, memory, city)?,
            Work::Mines if !has(view, city, Work::Mines) => {
                let gate = own_gate(view, city)?;
                let (dx, dy) = view.world.vector(city.pos, gate)?;
                let length = integer_sqrt((dx * dx + dy * dy) as u128) as i64;
                if length == 0 || length as i64 > city.radius + NEAR {
                    return None;
                }
                (dx * 1000 / length, dy * 1000 / length)
            }
            _ => return None,
        };
        Some((city, way))
    })
}

/// Wishes for a town's line or minefield.
pub(crate) fn wishes(view: &View<'_>, weights: &Weights, memory: &Memory, wishes: &mut Vec<Wish>) {
    if weights.defense_share <= 0 {
        return;
    }
    let (segment, mine) = kinds(view);
    if let Some(segment) = segment
        && town(view, memory, Work::Line).is_some()
    {
        wishes.push(Wish {
            score: LINE_SCORE + weights.defense_share,
            act: Act::Line(segment),
            reserve: false,
            opening: false,
        });
    }
    if let Some(mine) = mine
        && town(view, memory, Work::Mines).is_some()
    {
        wishes.push(Wish {
            score: MINES_SCORE + weights.defense_share,
            act: Act::Minefield(mine),
            reserve: false,
            opening: false,
        });
    }
}

/// Whether every site can be placed and keeps clear of own buildings (other than the
/// fortifications), resources and gather sites: a line never shuts a way the player uses.
fn clear(view: &View<'_>, sites: &[LineSite]) -> bool {
    let rules = view.rules;
    !sites.is_empty()
        && sites.iter().all(|s| {
            s.check.is_ok()
                && view
                    .world
                    .entities_in_radius(s.pos, CLEAR)
                    .into_iter()
                    .filter_map(|id| view.world.entity(id))
                    .all(|e| {
                        let et = &rules.entity_types[&e.type_id];
                        let mine = e.owner == view.me;
                        let resource = et.node.is_some() || et.gather_site.is_some();
                        !resource
                            && !(mine
                                && et.class == rom_rules::EntityClass::Building
                                && et.fortification.is_none()
                                && et.minefield.is_none())
                    })
        })
}

/// The sites of the town's line, if one fits.
fn line_sites(
    view: &View<'_>,
    city: &City,
    (ux, uy): (i64, i64),
    segment: DefId,
) -> Option<Vec<LineSite>> {
    let world = view.world;
    let (px, py) = (-uy, ux);
    LINE_BEYOND.iter().find_map(|beyond| {
        let reach = city.radius + beyond * 1000;
        if reach + 2000 > city.border {
            return None;
        }
        let middle = world.offset(city.pos, ux * reach / 1000, uy * reach / 1000);
        let end = |sign: i64| world.offset(middle, sign * px * LINE_HALF, sign * py * LINE_HALF);
        let (a, b) = (end(1), end(-1));
        if a.face != middle.face || b.face != middle.face {
            return None;
        }
        let sites = world
            .line_sites(view.me, segment, &[a, b], &[middle])
            .ok()?;
        clear(view, &sites).then_some(sites)
    })
}

/// The sites of the town's minefield before the gate of its line nearest the way in.
fn mine_sites(
    view: &View<'_>,
    city: &City,
    (ux, uy): (i64, i64),
    mine: DefId,
) -> Option<Vec<LineSite>> {
    let world = view.world;
    let gate = own_gate(view, city)?;
    let (px, py) = (-uy, ux);
    let corner = |ahead: i64, side: i64| {
        world.offset(
            gate,
            (ux * ahead + px * side) / 1000,
            (uy * ahead + py * side) / 1000,
        )
    };
    let (a, b) = (corner(2500, -2000), corner(3500, 2000));
    if a.face != gate.face || b.face != gate.face {
        return None;
    }
    let sites = world.minefield_sites(view.me, mine, a, b).ok()?;
    (sites.len() <= 15 && clear(view, &sites)).then_some(sites)
}

/// Lays the first wanting town's line (`Work::Line`, of `def` segments) or minefield.
fn lay(plan: &mut Plan<'_>, memory: &mut Memory, def: DefId, work: Work) -> Outcome {
    let Some((city, way)) = town(&plan.view, memory, work) else {
        return Outcome::Impossible;
    };
    let city = *city;
    let sites = match work {
        Work::Line => line_sites(&plan.view, &city, way, def),
        Work::Mines => mine_sites(&plan.view, &city, way, def),
    };
    let Some(sites) = sites else {
        memory.blocked.insert(def.0, plan.view.tick + RETRY_TICKS);
        return Outcome::Impossible;
    };
    let mut price: Vec<(DefId, i64)> = Vec::new();
    for site in &sites {
        for (resource, amount) in plan.price(site.subject) {
            match price.iter_mut().find(|(r, _)| *r == resource) {
                Some((_, n)) => *n += amount,
                None => price.push((resource, amount)),
            }
        }
    }
    price.sort();
    if !plan.affordable(&price) {
        plan.lack(&price, false);
        return Outcome::Unaffordable;
    }
    if plan.orders_left < sites.len() as i64 {
        return Outcome::Impossible;
    }
    let first = sites[0].pos;
    let mut builders: Vec<(bool, u64, EntityId)> = plan
        .builders(def)
        .into_iter()
        .filter(|e| e.pos.body == first.body)
        .map(|e| {
            (
                !plan.view.is_idle(e),
                plan.view.distance(e.pos, first),
                e.id,
            )
        })
        .collect();
    builders.sort();
    builders.truncate(LINE_BUILDERS);
    if builders.is_empty() {
        return Outcome::Impossible;
    }
    // The builders take the line in order, one stretch each.
    for (n, chunk) in sites
        .chunks(sites.len().div_ceil(builders.len()))
        .enumerate()
    {
        let builder = builders[n].2;
        for (k, site) in chunk.iter().enumerate() {
            if !plan.order(
                builder,
                "construct",
                Some(site.subject),
                None,
                Some(site.pos),
            ) {
                return Outcome::Impossible;
            }
            let command = plan.out.last_mut().expect("order written");
            command.queue = k > 0;
            command.facing = Some(site.facing);
        }
    }
    plan.pay(&price);
    plan.view.note_planned(def);
    plan.started += 1;
    Outcome::Ordered
}

/// Serves `Act::Line`.
pub(crate) fn lay_line(plan: &mut Plan<'_>, memory: &mut Memory, segment: DefId) -> Outcome {
    lay(plan, memory, segment, Work::Line)
}

/// Serves `Act::Minefield`.
pub(crate) fn lay_mines(plan: &mut Plan<'_>, memory: &mut Memory, mine: DefId) -> Outcome {
    lay(plan, memory, mine, Work::Mines)
}

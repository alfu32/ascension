//! The production manager: keeps the army share, picks unit types from what the player can
//! make and what the enemies it has seen are weak to, and chooses the rally point.

use crate::{
    Weights,
    economy::can_trade,
    memory::{EnemySighting, Memory},
    plan::{Act, Wish},
    view::View,
};
use rom_rules::{DefId, EntityClass, Layer, ProductionMode, Rules};
use rom_sim::Pos;
use std::collections::BTreeMap;

/// Distance of the rally point from the capital, in milli-tiles.
const RALLY_REACH: i64 = 9000;

/// Whether a type fights for a living: an armed, moving unit that neither works, builds,
/// trades nor carries.
pub(crate) fn is_military(rules: &Rules, type_id: DefId) -> bool {
    rules.entity_types.get(&type_id).is_some_and(|t| {
        t.class == EntityClass::Unit
            && !t.weapons.is_empty()
            && t.movement.is_some()
            && t.harvester.is_empty()
            && t.builder.is_none()
            && t.trader.is_none()
            && t.container.is_none()
    })
}

/// Whether a unit type can move on the ground its player's capital stands on (orbital craft,
/// which the space manager leads, do not count).
pub(crate) fn walks_at_home(view: &View<'_>, type_id: DefId) -> bool {
    // Orbital craft belong to the space manager.
    if view.world.is_craft_type(type_id) {
        return false;
    }
    let Some(class) = view.rules.entity_types[&type_id]
        .movement
        .as_ref()
        .map(|m| m.movement_class_id)
    else {
        return false;
    };
    let Some(terrain) = view
        .cities
        .first()
        .and_then(|c| view.world.terrain_at(c.pos))
    else {
        return false;
    };
    if view.rules.terrains[&terrain].layer != Layer::Ground {
        return false;
    }
    view.rules
        .movement_class_terrain
        .get(&class)
        .into_iter()
        .flatten()
        .any(|row| row.terrain_id == terrain && row.speed_pct > 0)
}

/// Damage per second of `attacker` against an armour class, with the damage matrix.
pub(crate) fn dps_against(
    rules: &Rules,
    attacker: DefId,
    armor_class: Option<u32>,
    armor: i64,
) -> i64 {
    rules.entity_types[&attacker]
        .weapons
        .iter()
        .map(|w| {
            let pct = armor_class.map_or(100, |a| rules.damage_pct(w.damage_class_id, a));
            let hit = (w.damage * pct / 100 - armor).max(1);
            hit * w.shots.max(1) * 2000 / w.cooldown_ticks.max(1)
        })
        .sum()
}

/// How good a unit type is for its price against the enemies seen, or against every
/// armour class alike when none has been seen.
fn value(view: &View<'_>, enemies: &BTreeMap<DefId, i64>, unit: DefId, price: i64) -> i64 {
    let rules = view.rules;
    let et = &rules.entity_types[&unit];
    let mut damage = 0;
    let mut weight = 0;
    if enemies.is_empty() {
        for class in rules.armor_classes.keys() {
            damage += dps_against(rules, unit, Some(*class), 0);
            weight += 1;
        }
    } else {
        for (enemy, count) in enemies {
            let target = &rules.entity_types[enemy];
            damage += dps_against(rules, unit, target.armor_class_id, target.armor) * count;
            weight += count;
        }
    }
    let reach = et.weapons.iter().map(|w| w.range_max).max().unwrap_or(0);
    let damage = damage / weight.max(1);
    // Staying power and reach count as well as damage; all of it per resource spent.
    damage * (et.hp + reach / 50) / price.max(1)
}

/// Population of own military units, with those in training.
pub(crate) fn army_pop(view: &View<'_>) -> i64 {
    view.have
        .iter()
        .filter(|(t, _)| is_military(view.rules, **t))
        .map(|(t, n)| view.rules.entity_types[t].pop_cost * n)
        .sum()
}

/// The unit type to train next, best first; `second_best` takes the runner-up.
fn choice(view: &View<'_>, memory: &Memory, second_best: bool) -> Option<DefId> {
    let rules = view.rules;
    let mut seen: BTreeMap<DefId, i64> = BTreeMap::new();
    for EnemySighting { type_id, .. } in memory.enemies.values() {
        if rules
            .entity_types
            .get(type_id)
            .is_some_and(|t| !t.weapons.is_empty())
        {
            *seen.entry(*type_id).or_default() += 1;
        }
    }
    let mut ranked: Vec<(i64, DefId)> = Vec::new();
    for producer in &view.owned {
        for row in rules.production.get(producer).into_iter().flatten() {
            let unit = row.product_def_id;
            if row.mode != ProductionMode::Train
                || !is_military(rules, unit)
                || ranked.iter().any(|(_, u)| *u == unit)
                || !view.unlocked(unit)
                || view.obsolete(unit)
                || !walks_at_home(view, unit)
                || memory
                    .blocked
                    .get(&unit.0)
                    .is_some_and(|until| *until > view.tick)
            {
                continue;
            }
            let price: i64 = view.price(unit).iter().map(|(_, a)| *a).sum();
            ranked.push((value(view, &seen, unit, price), unit));
        }
    }
    ranked.sort_unstable_by_key(|(value, unit)| (std::cmp::Reverse(*value), *unit));
    ranked
        .get(usize::from(second_best && ranked.len() > 1))
        .map(|(_, unit)| *unit)
}

/// Wishes for military units and traders.
pub(crate) fn wishes(
    view: &View<'_>,
    weights: &Weights,
    memory: &Memory,
    alarmed: bool,
    second_best: bool,
    wishes: &mut Vec<Wish>,
) {
    let target = view.pop_cap * weights.army_share / 100;
    let army = army_pop(view);
    if army < target
        && let Some(unit) = choice(view, memory, second_best)
    {
        let score = if alarmed {
            880
        } else {
            440 + weights.army_share + ((target - army) * 4).min(160)
        };
        for _ in 0..2 {
            wishes.push(Wish {
                score,
                act: Act::Train(unit),
                reserve: false,
                opening: false,
            });
        }
    }
    // A trader for every free route once two cities stand far enough apart.
    if can_trade(view) {
        let have: i64 = view
            .have
            .iter()
            .filter(|(t, _)| {
                view.rules.entity_types[*t]
                    .trader
                    .as_ref()
                    .is_some_and(|t| t.runs_routes)
            })
            .map(|(_, n)| *n)
            .sum();
        if have < view.caravan_limit() {
            let trader = view
                .owned
                .iter()
                .flat_map(|p| view.rules.production.get(p).into_iter().flatten())
                .filter(|row| row.mode == ProductionMode::Train)
                .map(|row| row.product_def_id)
                .filter(|unit| {
                    view.rules.entity_types.get(unit).is_some_and(|t| {
                        t.trader
                            .as_ref()
                            .is_some_and(|t| t.runs_routes && t.route_resource_id.is_some())
                    }) && view.unlocked(*unit)
                        && !view.obsolete(*unit)
                })
                .max_by_key(|unit| {
                    (
                        view.rules.entity_types[unit]
                            .trader
                            .as_ref()
                            .map_or(0, |t| t.route_income),
                        std::cmp::Reverse(*unit),
                    )
                });
            if let Some(trader) = trader {
                wishes.push(Wish {
                    score: 560,
                    act: Act::Train(trader),
                    reserve: false,
                    opening: false,
                });
            }
        }
    }
}

/// Chooses the rally point once: open ground of the player beside the capital.
pub(crate) fn rally_point(view: &View<'_>) -> Option<Pos> {
    let capital = view.cities.first()?;
    let steps = [
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (1, 1),
        (-1, 1),
        (-1, -1),
        (1, -1),
    ];
    steps
        .iter()
        .map(|(dx, dy)| {
            view.world
                .offset(capital.pos, dx * RALLY_REACH, dy * RALLY_REACH)
        })
        .find(|at| {
            view.world.terrain_at(*at).is_some_and(|t| {
                let terrain = &view.rules.terrains[&t];
                terrain.buildable && terrain.layer == Layer::Ground
            })
        })
}

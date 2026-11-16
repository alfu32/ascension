//! The production manager: keeps the army share, picks unit types from what the player can
//! make and what the enemies it has seen are weak to, and chooses the rally point.

use crate::{
    Weights,
    economy::{can_trade, can_trade_in_space},
    memory::{EnemySighting, Memory},
    plan::{Act, Wish},
    view::View,
};
use rom_rules::{AiDifficulty, DefId, EntityClass, Layer, ProductionMode, Rules};
use rom_sim::Pos;
use std::collections::BTreeMap;

/// Distance of the rally point beyond the capital's economic radius, in milli-tiles: at the
/// edge of the town, where buildings box the army in less.
const RALLY_MARGIN: i64 = 3000;
/// Tiles around the rally point that no building of the player takes.
pub(crate) const RALLY_CLEAR: i32 = 3;

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

/// The price of a definition weighed by how scarce each resource is for the player: a
/// resource it has little of and earns little of counts up to eight times its amount, an
/// abundant one down to a quarter. Units are compared by this, so an army is not built of
/// what the player cannot pay for (a metal-starved player trains bowmen, not pikemen).
pub(crate) fn dearness(view: &View<'_>, def: DefId) -> i64 {
    let price = view.price(def);
    // What each resource of the price amounts to: the store and a few periods of income.
    let supply: Vec<(i64, i64)> = price
        .iter()
        .map(|(resource, amount)| {
            let stock = view.purse.get(resource).copied().unwrap_or(0).max(0);
            let income = view.world.income(view.me, *resource).effective.max(0);
            (*amount, stock + 4 * income + 1)
        })
        .collect();
    let base: i64 = price.iter().map(|(_, a)| *a).sum::<i64>().max(1);
    let supplies: Vec<i64> = view
        .rules
        .resources
        .values()
        // Resources the player earns: a starting store nobody works for says nothing.
        .filter(|r| {
            r.kind == rom_rules::ResourceKind::Base
                && view.world.income(view.me, r.id).effective > 0
        })
        .map(|r| {
            view.purse.get(&r.id).copied().unwrap_or(0).max(0)
                + 4 * view.world.income(view.me, r.id).effective.max(0)
                + 1
        })
        .collect();
    let mean = supplies.iter().sum::<i64>() / supplies.len().max(1) as i64;
    let weighed: i64 = supply
        .iter()
        .map(|(amount, have)| amount * (mean.max(1) * 100 / have).clamp(25, 800) / 100)
        .sum();
    weighed.max(base / 4).max(1)
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
    // Armed enemy units only: buildings are the siege wish's business, and counting them
    // here turned whole armies into siege engines that could not clear a city.
    let mut seen: BTreeMap<DefId, i64> = BTreeMap::new();
    for EnemySighting { type_id, .. } in memory.enemies.values() {
        if rules
            .entity_types
            .get(type_id)
            .is_some_and(|t| !t.weapons.is_empty() && t.class == EntityClass::Unit)
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
            ranked.push((value(view, &seen, unit, dearness(view, unit)), unit));
        }
    }
    ranked.sort_unstable_by_key(|(value, unit)| (std::cmp::Reverse(*value), *unit));
    ranked
        .get(usize::from(second_best && ranked.len() > 1))
        .map(|(_, unit)| *unit)
}

/// The fewest population an army limited by the enemy it has seen may still hold.
const ARMY_FLOOR: i64 = 8;
/// The largest enemy army seen counts half after this long (ten game minutes).
const PEAK_HALF_TICKS: u64 = 12_000;

/// Population of the armed enemy units the brain remembers.
fn enemy_army_pop(view: &View<'_>, memory: &Memory) -> i64 {
    memory
        .enemies
        .values()
        .filter(|e| !e.building && is_military(view.rules, e.type_id))
        .map(|e| view.rules.entity_types[&e.type_id].pop_cost.max(0))
        .sum()
}

/// The military population to keep: the profile's share of the cap; with a surplus in
/// every store, whatever room the cap leaves; at an easy difficulty no more than its share
/// of the enemy army seen.
pub(crate) fn army_target(
    view: &View<'_>,
    weights: &Weights,
    difficulty: &AiDifficulty,
    memory: &Memory,
) -> i64 {
    let mut target = view.pop_cap * weights.army_share / 100;
    if view.surplus() > 0 {
        let others = view.pop_used + view.queued_pop - army_pop(view);
        target = target.max(view.pop_cap - others.max(0));
    }
    if difficulty.army_vs_enemy_pct > 0 {
        let seen = enemy_peak(view, memory) * difficulty.army_vs_enemy_pct / 100;
        target = target.min(seen.max(ARMY_FLOOR));
    }
    target
}

/// The largest enemy army seen, halved for every `PEAK_HALF_TICKS` since.
pub(crate) fn enemy_peak(view: &View<'_>, memory: &Memory) -> i64 {
    let (pop, since) = memory.enemy_peak;
    let now = enemy_army_pop(view, memory);
    now.max(pop >> (view.tick.saturating_sub(since) / PEAK_HALF_TICKS).min(62))
}

/// Notes the enemy army seen now when it is the largest lately.
pub(crate) fn note_enemy_peak(view: &View<'_>, memory: &mut Memory) {
    let now = enemy_army_pop(view, memory);
    if now >= enemy_peak(view, memory) {
        memory.enemy_peak = (now, view.tick);
    }
}

/// Wishes for military units and traders.
pub(crate) fn wishes(
    view: &View<'_>,
    weights: &Weights,
    difficulty: &AiDifficulty,
    memory: &Memory,
    alarmed: bool,
    second_best: bool,
    wishes: &mut Vec<Wish>,
) {
    let target = army_target(view, weights, difficulty, memory);
    let army = army_pop(view);
    if army < target
        && let Some(unit) = choice(view, memory, second_best)
    {
        // An army below half its target comes before other research and buildings.
        let score = if alarmed {
            880
        } else if army * 2 < target {
            700
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
    // A trader for every free route once two cities stand far enough apart: one that walks
    // for cities of one planet, one that flies (the space freighter) for cities on two
    // planets of a star system.
    let walking = can_trade(view);
    let flying = can_trade_in_space(view);
    if walking || flying {
        let is_trader = |unit: &DefId| {
            view.rules.entity_types.get(unit).is_some_and(|t| {
                t.trader
                    .as_ref()
                    .is_some_and(|t| t.runs_routes && t.route_resource_id.is_some())
            })
        };
        let have = |flies: bool| -> i64 {
            view.have
                .iter()
                .filter(|(t, _)| is_trader(t) && view.world.is_spacecraft_type(**t) == flies)
                .map(|(_, n)| *n)
                .sum()
        };
        let limit = view.caravan_limit();
        let total = have(false) + have(true);
        // Freighters take up to half the routes; the rest walk.
        let want_flying = flying && total < limit && have(true) * 2 < limit.max(2);
        let want_walking = walking && total < limit && !want_flying;
        let pick = |flies: bool| {
            view.owned
                .iter()
                .flat_map(|p| view.rules.production.get(p).into_iter().flatten())
                .filter(|row| row.mode == ProductionMode::Train)
                .map(|row| row.product_def_id)
                .filter(|unit| {
                    is_trader(unit)
                        && view.world.is_spacecraft_type(*unit) == flies
                        && view.unlocked(*unit)
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
                })
        };
        let trader = if want_flying {
            pick(true)
        } else if want_walking {
            pick(false)
        } else {
            None
        };
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

/// Chooses the rally point: open ground of the player's own land just beyond the capital's
/// economic radius, which the town's buildings keep clear of (`RALLY_CLEAR`). Each move
/// (`moves`, when units there are boxed in) tries the next direction.
pub(crate) fn rally_point(view: &View<'_>, moves: u32) -> Option<Pos> {
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
    let shift = moves as usize % steps.len();
    let reach = capital.radius.max(0) + RALLY_MARGIN;
    // Somewhere the player's own walkers can get to over the terrain.
    let walker = view
        .own
        .iter()
        .find(|e| {
            e.pos.body == capital.pos.body
                && view.rules.entity_types[&e.type_id].movement.is_some()
                && view.rules.entity_types[&e.type_id].class == EntityClass::Unit
        })
        .copied();
    steps
        .iter()
        .cycle()
        .skip(shift)
        .take(steps.len())
        .map(|(dx, dy)| {
            // Diagonals at the same distance as the straight steps, near enough.
            let (dx, dy) = if dx * dy != 0 {
                (dx * 707, dy * 707)
            } else {
                (dx * 1000, dy * 1000)
            };
            view.world
                .offset(capital.pos, dx * reach / 1000, dy * reach / 1000)
        })
        .find(|at| {
            view.world.terrain_at(*at).is_some_and(|t| {
                let terrain = &view.rules.terrains[&t];
                terrain.buildable && terrain.layer == Layer::Ground
            }) && walker.is_none_or(|w| view.world.reachable(w.type_id, w.pos, *at) != Some(false))
                // Inside own borders: enemy land wears waiting units down.
                && view.world.territory_owner_on_body(
                    at.body,
                    at.face,
                    at.x.div_euclid(1000),
                    at.y.div_euclid(1000),
                ) == Some(view.me)
        })
}

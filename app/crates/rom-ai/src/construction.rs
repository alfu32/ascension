//! The construction manager: buildings other than gather sites, valued by what they
//! produce, research or unlock, and new cities when the limit allows.

use crate::{
    Weights,
    catalogue::Catalogue,
    economy::{Economy, can_trade},
    plan::{Act, Wish},
    production::{army_pop, is_military},
    view::View,
};
use rom_rules::{DefId, EntityClass, LimitScope, ProductionMode};
use std::collections::BTreeSet;

/// Score of another city while a resource has no room for another gather site: above the
/// way to the next age and every soldier, below the first site of a resource and workers.
const STUCK_CITY_SCORE: i64 = 850;
/// Score of an automated colony: below the next age and the economy, above most buildings.
const COLONY_SCORE: i64 = 520;
/// Score of a ground battery while an enemy bombards a planet of the player's cities (task
/// 75): above soldiers and most buildings, below the age path.
const BATTERY_SCORE: i64 = 780;
/// Ground batteries wanted per city on a bombarded planet.
const BATTERIES_PER_CITY: i64 = 2;

/// What a building type is good for, read from its component and production rows.
struct Role {
    researches_ages: bool,
    researches: bool,
    trains_military: bool,
    trains_traders: bool,
    defends: bool,
    unlocks: bool,
}

fn role(view: &View<'_>, building: DefId) -> Role {
    let rules = view.rules;
    let et = &rules.entity_types[&building];
    let rows = || rules.production.get(&building).into_iter().flatten();
    let trains = |test: &dyn Fn(DefId) -> bool| {
        rows().any(|p| {
            p.mode == ProductionMode::Train
                && rules.is_available(p.product_def_id, view.faction)
                && test(p.product_def_id)
        })
    };
    Role {
        researches_ages: Catalogue::researches_ages(rules, building),
        researches: rows().any(|p| p.mode == ProductionMode::Research),
        trains_military: trains(&|unit| is_military(rules, unit)),
        trains_traders: trains(&|unit| {
            rules.entity_types[&unit]
                .trader
                .as_ref()
                .is_some_and(|t| t.runs_routes)
        }),
        defends: !et.weapons.is_empty() || et.territory.as_ref().is_some_and(|t| !t.is_city),
        unlocks: view
            .cat
            .dependents
            .get(&building)
            .is_some_and(|d| d.iter().any(|d| rules.is_available(*d, view.faction))),
    }
}

/// Building types an own builder can construct now, one per upgrade line.
fn constructible(view: &View<'_>) -> Vec<DefId> {
    let rules = view.rules;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for producer in &view.owned {
        if rules.entity_types[producer].builder.is_none() {
            continue;
        }
        for row in rules.production.get(producer).into_iter().flatten() {
            let building = row.product_def_id;
            if row.mode == ProductionMode::Construct
                && rules
                    .entity_types
                    .get(&building)
                    .is_some_and(|t| t.class == EntityClass::Building)
                && seen.insert(building)
                && view.unlocked(building)
                && !view.obsolete(building)
            {
                out.push(building);
            }
        }
    }
    out
}

/// Wishes for buildings and cities. `needed` are the buildings the age path waits for.
pub(crate) fn wishes(
    view: &View<'_>,
    weights: &Weights,
    eco: &Economy,
    needed: &BTreeSet<DefId>,
    wishes: &mut Vec<Wish>,
) {
    let rules = view.rules;
    let cities = view.cities.iter().filter(|c| c.complete).count() as i64;
    let army_target = view.pop_cap * weights.army_share / 100;
    let army = army_pop(view);
    let bombarded = crate::view::bombarded_bodies(view);
    let threatened = view
        .cities
        .iter()
        .filter(|c| c.complete && bombarded.contains(&c.pos.body))
        .count() as i64;
    for building in constructible(view) {
        let et = &rules.entity_types[&building];
        let have = view.line_count(building);
        // Batteries that fire up at the orbit answer a bombardment (task 75).
        if threatened > 0
            && crate::view::guards_orbit(rules, building)
            && have < threatened * BATTERIES_PER_CITY
        {
            wishes.push(Wish {
                score: BATTERY_SCORE,
                act: Act::Construct(building),
                reserve: true,
                opening: false,
            });
            continue;
        }
        if view.is_colony(building) {
            // An automated colony outside the city limit, one per town, while it pays: its
            // governor then keeps its sites standing.
            let towns = view.cities.iter().filter(|c| c.complete).count() as i64;
            if have < towns {
                wishes.push(Wish {
                    score: COLONY_SCORE,
                    act: Act::Construct(building),
                    reserve: false,
                    opening: false,
                });
            }
            continue;
        }
        if view.is_city(building) {
            // Cities are where the land, the farms and the food are: the main way to grow.
            // A resource whose sites are full with no room for another around the cities
            // (a town by a small wood, five farms) waits for one: it is saved for ahead of
            // soldiers and the next age, else a town lives on forty timber a period all match.
            let stuck = eco.stuck();
            if have < view.city_limit() && (weights.expand > 0 || stuck) {
                wishes.push(Wish {
                    score: if stuck {
                        STUCK_CITY_SCORE
                    } else {
                        450 + weights.expand * 4
                    },
                    act: Act::Construct(building),
                    reserve: stuck || weights.expand >= 50,
                    opening: false,
                });
            }
            continue;
        }
        if et.gather_site.is_some() {
            // The economy manager places gather sites.
            continue;
        }
        let limit = match et.limit_scope {
            LimitScope::PerCity => et.limit_count * cities,
            LimitScope::PerPlayer | LimitScope::PerMatch => et.limit_count,
            _ => i64::MAX,
        };
        if have >= limit {
            continue;
        }
        let role = role(view, building);
        let need = needed
            .iter()
            .any(|n| view.cat.root_of(*n) == view.cat.root_of(building));
        let score = if need && have == 0 {
            950
        } else if role.researches_ages && have < cities {
            if have == 0 { 950 } else { 380 }
        } else if role.trains_military {
            if have == 0 {
                430 + weights.army_share * 2
            } else if army_target - army >= 10 && have < 1 + army_target / 25 {
                300
            } else {
                continue;
            }
        } else if role.trains_traders {
            if have == 0 && can_trade(view) {
                580
            } else {
                continue;
            }
        } else if role.researches && have == 0 {
            520
        } else if role.defends {
            if have < weights.defense_share * cities / 25 {
                260 + weights.defense_share
            } else {
                continue;
            }
        } else if have == 0 {
            if role.unlocks { 360 } else { 220 }
        } else {
            continue;
        };
        wishes.push(Wish {
            score,
            act: Act::Construct(building),
            reserve: score >= 900,
            opening: false,
        });
    }
}

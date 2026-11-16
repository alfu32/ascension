//! The construction manager: buildings other than gather sites, valued by what they
//! produce, research or unlock, and new cities when the limit allows.

use crate::{
    Weights,
    catalogue::Catalogue,
    economy::can_trade,
    plan::{Act, Wish},
    production::{army_pop, is_military},
    view::View,
};
use rom_rules::{DefId, EntityClass, LimitScope, ProductionMode};
use std::collections::BTreeSet;

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
    needed: &BTreeSet<DefId>,
    wishes: &mut Vec<Wish>,
) {
    let rules = view.rules;
    let cities = view.cities.iter().filter(|c| c.complete).count() as i64;
    let army_target = view.pop_cap * weights.army_share / 100;
    let army = army_pop(view);
    for building in constructible(view) {
        let et = &rules.entity_types[&building];
        let have = view.line_count(building);
        if view.is_city(building) {
            // Cities are where the land, the farms and the food are: the main way to grow.
            if have < view.city_limit() && weights.expand > 0 {
                wishes.push(Wish {
                    score: 450 + weights.expand * 4,
                    act: Act::Construct(building),
                    reserve: weights.expand >= 50,
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

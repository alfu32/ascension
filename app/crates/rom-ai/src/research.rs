//! The research manager: the way to the next age first, then whatever improves the most.

use crate::{
    Weights,
    catalogue::Catalogue,
    economy::Economy,
    plan::{Act, Wish},
    view::{View, operational},
};
use rom_rules::{DefId, EntityClass, ProductionMode, StatScope};
use std::collections::BTreeSet;

/// Technologies still to research before the next age can be: the age technology itself
/// and, through prerequisites, everything it waits for.
pub(crate) fn age_path(view: &View<'_>) -> BTreeSet<DefId> {
    let mut path = BTreeSet::new();
    let Some(next) = view.cat.next_age_tech(view.rules, view.age) else {
        return path;
    };
    let mut open = vec![next];
    while let Some(tech) = open.pop() {
        if view.researched(tech) || !path.insert(tech) {
            continue;
        }
        for row in view.rules.prerequisites.get(&tech).into_iter().flatten() {
            if view.rules.techs.contains_key(&row.requires_def_id) {
                open.push(row.requires_def_id);
            }
        }
    }
    path
}

/// Building types the age path needs standing: prerequisites of its technologies and the
/// producers that research them, when the player has none.
pub(crate) fn age_buildings(view: &View<'_>, path: &BTreeSet<DefId>) -> BTreeSet<DefId> {
    let mut needed = BTreeSet::new();
    for tech in path {
        for row in view.rules.prerequisites.get(tech).into_iter().flatten() {
            if view
                .rules
                .entity_types
                .get(&row.requires_def_id)
                .is_some_and(|t| t.class == EntityClass::Building)
                && !view.owned.contains(&row.requires_def_id)
            {
                needed.insert(row.requires_def_id);
            }
        }
        let producers = view.rules.production_by_product.get(tech);
        let have = producers
            .into_iter()
            .flatten()
            .any(|p| p.mode == ProductionMode::Research && view.count(p.producer_type_id) > 0);
        if !have {
            needed.extend(
                producers
                    .into_iter()
                    .flatten()
                    .filter(|p| p.mode == ProductionMode::Research)
                    .map(|p| p.producer_type_id)
                    .filter(|t| view.is_building(*t)),
            );
        }
    }
    needed
}

/// How much a technology is worth besides the age path: its modifiers, the types it
/// upgrades and what it unlocks, weighed by the profile and the state of the economy.
fn worth(view: &View<'_>, weights: &Weights, eco: &Economy, tech: DefId) -> i64 {
    let rules = view.rules;
    let crowded = (view.pop_used + view.queued_pop) * 100 >= view.pop_cap * 80;
    let capped = eco
        .resources
        .keys()
        .any(|r| view.world.income(view.me, *r).capped);
    let mut value = 0;
    for modifier in rules.modifiers_from(tech) {
        let size = modifier.value.abs();
        let scope = rules.stats.get(&modifier.stat_key).map(|s| s.scope);
        value += match (modifier.stat_key.as_str(), scope) {
            ("pop_cap", _) => size * if crowded { 12 } else { 2 },
            ("city_limit", _) => size * weights.expand,
            ("commerce_cap", _) => size * if capped { 4 } else { 1 },
            ("caravan_limit", _) => size * 10,
            (key, _) if key.starts_with("income.") => size * 6,
            (_, Some(StatScope::Player)) => size * 3,
            _ => size * weights.army_share / 20,
        };
    }
    for (from, _) in view.cat.upgrades.get(&tech).into_iter().flatten() {
        let owned = view.count(*from);
        let military = crate::production::is_military(rules, *from);
        value += if military {
            20 + owned.min(10) * weights.army_share / 10
        } else {
            40 + owned.min(10) * 8
        };
    }
    let unlocks = view
        .cat
        .dependents
        .get(&tech)
        .into_iter()
        .flatten()
        .filter(|d| rules.is_available(**d, view.faction) && !view.unlocked(**d))
        .count() as i64;
    value += unlocks.min(5) * 25;
    let cost: i64 = view.price(tech).iter().map(|(_, a)| *a).sum();
    value - cost / 20
}

/// Wishes for technologies; returns the buildings the age path needs.
pub(crate) fn wishes(
    view: &View<'_>,
    weights: &Weights,
    eco: &Economy,
    second_best: bool,
    wishes: &mut Vec<Wish>,
) -> BTreeSet<DefId> {
    let rules = view.rules;
    let path = age_path(view);
    let mut side: Vec<(i64, DefId)> = Vec::new();
    let mut seen = BTreeSet::new();
    // Only producers with nothing to research take part: the rest cannot start anything.
    let free: BTreeSet<DefId> = view
        .own
        .iter()
        .filter(|e| operational(e) && !view.queue_len.contains_key(&e.id))
        .map(|e| e.type_id)
        .collect();
    for producer in &free {
        for row in rules.production.get(producer).into_iter().flatten() {
            let tech = row.product_def_id;
            if row.mode != ProductionMode::Research || !seen.insert(tech) {
                continue;
            }
            let Some(info) = rules.techs.get(&tech) else {
                continue;
            };
            if (view.researched(tech) && !info.repeatable)
                || view.researching.contains(&tech)
                || !view.unlocked(tech)
            {
                continue;
            }
            if path.contains(&tech) {
                let age = Catalogue::is_age_tech(rules, tech);
                wishes.push(Wish {
                    score: 560 + weights.age_priority * 2 + i64::from(age),
                    act: Act::Research(tech),
                    reserve: true,
                    opening: false,
                });
            } else if !info.repeatable {
                side.push((worth(view, weights, eco, tech), tech));
            }
        }
    }
    side.sort_unstable_by_key(|(value, tech)| (std::cmp::Reverse(*value), *tech));
    let pick = usize::from(second_best && side.len() > 1);
    if let Some((value, tech)) = side.get(pick) {
        wishes.push(Wish {
            score: 300 + (*value).clamp(0, 250),
            act: Act::Research(*tech),
            reserve: false,
            opening: false,
        });
    }
    age_buildings(view, &path)
}

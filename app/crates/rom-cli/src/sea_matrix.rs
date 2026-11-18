//! The sea rows of `rom fight-matrix` (task 92): warships on the water, and what stands or flies
//! on the shore beside it.
//!
//! The owner's rule holds at sea as on land: whatever outranges and beats a target type has a
//! countermeasure in the same age, effective now or after one research. A fight here is the
//! ground matrix's fight (equal cost, the same seeds and measure) on a field whose halves are
//! each side's own ground: open water for a warship, the plain shore for everything else. A row
//! is a strength in which a warship takes part: a warship that outranges and beats anything of
//! its age (a ship, a shore defence, a unit ashore), or anything ashore that outranges and beats
//! a warship. Its answer may be any mobile type of the age, and, for a warship, a building on
//! the shore. Everything is read from the rules: a warship is a unit whose movement crosses the
//! water and not the shore. No definition is named.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use rom_rules::{DefId, EntityClass, Layer, Rules};

use crate::fight_matrix_cmd::{
    AgeMatrix, Pairing, Setup, Side, Strength, extent, fight_on, reach_at, researches_for, roster,
};

/// The water of the sea rows: a fluid terrain nothing can be built on (open water), the one
/// the most movement classes cross.
pub fn water(rules: &Rules) -> Option<DefId> {
    rules
        .terrains
        .values()
        .filter(|t| t.layer == Layer::Fluid && !t.buildable)
        .max_by_key(|t| {
            let crossers = rules
                .movement_class_terrain
                .values()
                .flatten()
                .filter(|row| row.terrain_id == t.id && row.speed_pct > 0)
                .count();
            (crossers, std::cmp::Reverse(t.id))
        })
        .map(|t| t.id)
}

/// Whether a movement class crosses a terrain.
fn crosses(rules: &Rules, class: u32, terrain: DefId) -> bool {
    rules
        .movement_class_terrain
        .get(&class)
        .into_iter()
        .flatten()
        .any(|row| row.terrain_id == terrain && row.speed_pct > 0)
}

/// Whether a type is a warship: a unit that moves on the water and cannot go ashore.
fn is_ship(rules: &Rules, setup: &Setup, water: DefId, unit: DefId) -> bool {
    rules.entity_types[&unit]
        .movement
        .as_ref()
        .is_some_and(|m| {
            m.climb_ticks == 0
                && !rules.movement_classes[&m.movement_class_id].flies
                && crosses(rules, m.movement_class_id, water)
                && !crosses(rules, m.movement_class_id, setup.terrain)
        })
}

/// The layer a type is hit on in a sea fight.
fn layer(rules: &Rules, setup: &Setup, water: DefId, unit: DefId) -> Layer {
    let flies = rules.entity_types[&unit]
        .movement
        .as_ref()
        .is_some_and(|m| rules.movement_classes[&m.movement_class_id].flies);
    if flies {
        Layer::Air
    } else if is_ship(rules, setup, water, unit) {
        Layer::Fluid
    } else {
        Layer::Ground
    }
}

/// How far `a` reaches `b`, centre to centre, in a sea fight; `None` when it cannot hit it.
fn reach(rules: &Rules, setup: &Setup, water: DefId, a: DefId, b: DefId) -> Option<i64> {
    reach_at(rules, a, layer(rules, setup, water, b)).map(|r| r + extent(rules, b))
}

/// Whether `a` outranges `b` at sea: it hits `b` from farther than `b` can hit it, or `b`
/// cannot hit it at all.
fn outranges(rules: &Rules, setup: &Setup, water: DefId, a: DefId, b: DefId) -> bool {
    let Some(ra) = reach(rules, setup, water, a, b) else {
        return false;
    };
    reach(rules, setup, water, b, a).is_none_or(|rb| ra > rb)
}

/// The warships of an age: armed units that move on the water, buildable by the faction, the
/// newest of each class whose first age is this one or earlier.
pub fn ships(rules: &Rules, setup: &Setup, water: DefId, age: i64) -> Vec<DefId> {
    let ordinal = |id: Option<DefId>| id.and_then(|a| rules.ages.get(&a)).map_or(0, |a| a.ordinal);
    let reached: BTreeSet<DefId> = rules
        .entity_types
        .values()
        .filter(|et| ordinal(et.first_age_id) <= age)
        .map(|et| et.id)
        .collect();
    rules
        .entity_types
        .values()
        .filter(|et| {
            reached.contains(&et.id)
                && et.class == EntityClass::Unit
                && et.container.is_none()
                && rules.production_by_product.contains_key(&et.id)
                && rules.is_available(et.id, setup.faction)
                && is_ship(rules, setup, water, et.id)
                && reach_at(rules, et.id, Layer::Fluid).is_some()
                && !rules
                    .entity_types
                    .values()
                    .any(|next| next.upgrades_from_id == Some(et.id) && reached.contains(&next.id))
        })
        .map(|et| et.id)
        .collect()
}

/// One fight of `first` against `second` at sea on a seed: each side on its own ground (a
/// warship on the water, anything else ashore), the value each keeps per mille.
fn fight_at_sea(
    rules: &Arc<Rules>,
    setup: &Setup,
    water: DefId,
    age: i64,
    first: Side,
    second: Side,
    seed: u64,
) -> Result<(i64, i64), String> {
    let ground = |unit| {
        if is_ship(rules, setup, water, unit) {
            water
        } else {
            setup.terrain
        }
    };
    let gap = reach(rules, setup, water, first.unit, second.unit)
        .unwrap_or(0)
        .max(reach(rules, setup, water, second.unit, first.unit).unwrap_or(0));
    fight_on(
        rules,
        setup,
        age,
        [first, second],
        [ground(first.unit), ground(second.unit)],
        gap,
        seed,
    )
}

/// One sea fight, as `rom fight-matrix --sea-fight a,b` plays it.
pub fn fight(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    first: Side,
    second: Side,
    seed: u64,
) -> Result<(i64, i64), String> {
    let water = water(rules).ok_or("the rules have no open water")?;
    fight_at_sea(rules, setup, water, age, first, second, seed)
}

/// Fights `first` against `second` at sea on `seeds` seeds.
fn pairing(
    rules: &Arc<Rules>,
    setup: &Setup,
    water: DefId,
    age: i64,
    first: Side,
    second: Side,
    seeds: u64,
) -> Result<Pairing, String> {
    let mut result = Pairing::default();
    for seed in 1..=seeds {
        let (a, b) = fight_at_sea(rules, setup, water, age, first, second, seed)?;
        result.seeds += 1;
        result.first_left += a;
        result.second_left += b;
        if a > b {
            result.wins += 1;
        }
    }
    Ok(result)
}

/// Measures the sea rows of one age: every strength a warship takes part in, and its answer.
pub fn measure_age(
    rules: &Arc<Rules>,
    setup: &Setup,
    water: DefId,
    age: i64,
    seeds: u64,
    all: bool,
) -> Result<AgeMatrix, String> {
    let age_id = rules
        .ages
        .values()
        .find(|a| a.ordinal == age)
        .ok_or("no such age")?
        .id;
    let fleet = ships(rules, setup, water, age);
    let shore = roster(rules, setup, age);
    let mut strengths = Vec::new();
    if fleet.is_empty() {
        return Ok(AgeMatrix {
            age: age_id,
            roster: fleet,
            strengths,
        });
    }
    let everyone: Vec<DefId> = fleet.iter().chain(shore.iter()).copied().collect();
    let mobile: Vec<DefId> = everyone
        .iter()
        .copied()
        .filter(|u| rules.entity_types[u].movement.is_some())
        .collect();
    let plain = |unit| Side { unit, tech: None };
    for &strong in &everyone {
        let at_sea = fleet.contains(&strong);
        let targets: &[DefId] = if at_sea { &everyone } else { &fleet };
        let mut beaten = Vec::new();
        for &target in targets {
            if target == strong || !outranges(rules, setup, water, strong, target) {
                continue;
            }
            if pairing(
                rules,
                setup,
                water,
                age,
                plain(strong),
                plain(target),
                seeds,
            )?
            .first_wins()
            {
                beaten.push(target);
                if !all {
                    break;
                }
            }
        }
        if beaten.is_empty() {
            continue;
        }
        // A shore defence answers a warship; anything that moves answers anything.
        let counters: Vec<DefId> = if at_sea {
            everyone.clone()
        } else {
            mobile.clone()
        };
        let answer = best_answer(rules, setup, water, age, strong, &counters, seeds)?;
        strengths.push(Strength {
            strong,
            beaten,
            answer,
        });
    }
    Ok(AgeMatrix {
        age: age_id,
        roster: fleet,
        strengths,
    })
}

/// The best answer to `strong` among `counters`, without research and then with one research.
fn best_answer(
    rules: &Arc<Rules>,
    setup: &Setup,
    water: DefId,
    age: i64,
    strong: DefId,
    counters: &[DefId],
    seeds: u64,
) -> Result<Option<(Side, Pairing)>, String> {
    let plain = |unit| Side { unit, tech: None };
    let better = |best: &Option<(Side, Pairing)>, p: &Pairing| {
        best.is_none_or(|(_, b)| {
            (p.wins, p.first_left - p.second_left) > (b.wins, b.first_left - b.second_left)
        })
    };
    let mut best: Option<(Side, Pairing)> = None;
    for &counter in counters.iter().filter(|c| **c != strong) {
        let p = pairing(
            rules,
            setup,
            water,
            age,
            plain(counter),
            plain(strong),
            seeds,
        )?;
        if better(&best, &p) {
            best = Some((plain(counter), p));
        }
        // Every seed won outright: nothing can do better than that.
        if p.wins == p.seeds && p.second_left == 0 {
            return Ok(best);
        }
    }
    if !best.is_some_and(|(_, p)| p.first_wins()) {
        for &counter in counters.iter().filter(|c| **c != strong) {
            if rules.entity_types[&counter].movement.is_none() {
                continue;
            }
            for tech in researches_for(rules, age, counter) {
                let side = Side {
                    unit: counter,
                    tech: Some(tech),
                };
                let p = pairing(rules, setup, water, age, side, plain(strong), seeds)?;
                if p.first_wins() && better(&best, &p) {
                    best = Some((side, p));
                }
            }
        }
    }
    Ok(best)
}

/// Measures the sea rows of the ages in `ages` on `jobs` threads, in age order; none when the
/// rules have no open water.
pub fn measure(
    rules: &Arc<Rules>,
    ages: (i64, i64),
    seeds: u64,
    jobs: usize,
    all: bool,
) -> Result<Vec<AgeMatrix>, String> {
    let setup = crate::fight_matrix_cmd::setup(rules)?;
    let Some(water) = water(rules) else {
        return Ok(Vec::new());
    };
    let list: Vec<i64> = (ages.0..=ages.1)
        .filter(|age| !ships(rules, &setup, water, *age).is_empty())
        .collect();
    let next = AtomicUsize::new(0);
    let results: Mutex<BTreeMap<i64, Result<AgeMatrix, String>>> = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(age) = list.get(i) else {
                        break;
                    };
                    let result = measure_age(rules, &setup, water, *age, seeds, all);
                    results.lock().expect("results lock").insert(*age, result);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("results lock")
        .into_values()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight_matrix_cmd::{SEEDS, setup, unanswered};
    use rom_rules::build_rules_db;

    fn rules() -> Arc<Rules> {
        let db = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        Arc::new(Rules::load(&db).unwrap())
    }

    /// The owner's rule at sea (task 92): every warship that outranges and beats something,
    /// and everything ashore that outranges and beats a warship, has an answer in its age.
    /// Classes coexist: from the modern age frigates, destroyers and line ships sail together.
    #[test]
    fn every_sea_strength_has_an_answer() {
        let rules = rules();
        let last = rules.ages.values().map(|a| a.ordinal).max().unwrap();
        let jobs = std::thread::available_parallelism().map_or(1, |n| n.get());
        let matrices = measure(&rules, (0, last), SEEDS, jobs, false).unwrap();
        assert!(!matrices.is_empty(), "no age has a warship");
        let missing: Vec<String> = matrices
            .iter()
            .filter(|m| unanswered(&m.strengths) > 0)
            .map(|m| rules.key(m.age).to_string())
            .collect();
        assert!(
            missing.is_empty(),
            "sea strengths without an answer in {missing:?}"
        );
        assert!(matrices.iter().any(|m| m.roster.len() >= 3));
        // Warships shell what stands ashore and something ashore answers them.
        assert!(matrices.iter().any(|m| !m.strengths.is_empty()));
    }

    #[test]
    fn a_sea_fight_repeats() {
        let rules = rules();
        let setup = setup(&rules).unwrap();
        let water = water(&rules).unwrap();
        let fleet = ships(&rules, &setup, water, 6);
        assert!(fleet.len() >= 3, "classes side by side in the modern age");
        let side = |unit| Side { unit, tech: None };
        let (a, b) = (side(fleet[0]), side(*fleet.last().unwrap()));
        assert_eq!(
            fight(&rules, &setup, 6, a, b, 3).unwrap(),
            fight(&rules, &setup, 6, a, b, 3).unwrap()
        );
    }
}

//! The `rom fight-matrix` subcommand: every strength has an answer.
//!
//! The owner's rule (2026-11-16): a unit or armed building that outranges and beats a target
//! type must have a countermeasure, available in the same age, that is effective against it
//! now or after one research of that age. This command measures it with scripted fights at
//! equal resource cost, per age, and prints the table; the test below fails when a strength
//! has no answer.
//!
//! Everything comes from the rules: the units of an age are the newest of each upgrade line
//! whose first age it is or an earlier one, armed buildings are buildings with a weapon, reach
//! is the weapon's range, effectiveness is whatever the fight shows. No definition is named.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use rom_rules::{AppliesTo, DefId, EntityClass, Layer, Rules, TechKind};
use rom_sim::{Command, MapGrid, MatchOptions, Player, PlayerId, Pos, World};

const USAGE: &str = "usage: rom fight-matrix --db <rules.db> [--ages <first>..<last>] [--seeds <n>] [--jobs <n>] [--all] [--orbit-only] [--no-orbit] [--no-sea] [--fight a,b | --orbit-fight a,b | --sea-fight a,b]";
/// Each side spends this many times the dearer side's price.
const BUDGET_UNITS: i64 = 3;
/// The most of one type on a side.
const MAX_COUNT: i64 = 24;
/// A fight that has not ended after this long is judged as it stands (five game minutes).
const FIGHT_TICKS: u64 = 6000;
/// Seeds of each fight; a side wins the pairing when it wins most of them.
pub const SEEDS: u64 = 3;
/// The map of a fight: plain ground, in tiles.
const MAP_W: u32 = 112;
const MAP_H: u32 = 64;
/// Space between the nearer edges of the two sides, beyond both reaches, in milli-tiles.
const GAP_SLACK: i64 = 6000;
/// Entity stats a research may change to make a countermeasure effective.
const COMBAT_STATS: [&str; 5] = ["attack", "range", "hp", "armor", "speed"];

/// What the fights are fought with: the faction, the ground and the observer that lets both
/// sides see the whole field (a real army brings scouts; siege sees little on its own).
#[derive(Debug, Clone)]
pub struct Setup {
    /// The faction both sides play.
    pub faction: DefId,
    /// Plain ground every fighter in the matrix can stand on.
    pub terrain: DefId,
    /// The unarmed type that sees farthest, placed far off each side; none when the rules
    /// have no unarmed unit.
    pub observer: Option<DefId>,
}

/// One side of a fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Side {
    /// The type fielded.
    pub unit: DefId,
    /// A research the side has, beyond the age.
    pub tech: Option<DefId>,
}

/// The result of one pairing over the seeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pairing {
    /// Seeds the first side won.
    pub wins: u64,
    /// Seeds fought.
    pub seeds: u64,
    /// Share of its starting value the first side still had, per mille, summed over seeds.
    pub first_left: i64,
    /// The same for the second side.
    pub second_left: i64,
}

impl Pairing {
    /// Whether the first side won more than half of the seeds.
    pub fn first_wins(&self) -> bool {
        self.wins * 2 > self.seeds
    }
}

/// A strength found in an age and the answer to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strength {
    /// The unit or armed building that outranges and beats.
    pub strong: DefId,
    /// What it outranges and beats.
    pub beaten: Vec<DefId>,
    /// The best countermeasure: its side, and its fight against the strong one.
    pub answer: Option<(Side, Pairing)>,
}

/// One age of the matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgeMatrix {
    /// The age.
    pub age: DefId,
    /// Its fighters, in id order.
    pub roster: Vec<DefId>,
    /// Every strength of the age.
    pub strengths: Vec<Strength>,
}

/// The ordinal of an age definition.
fn ordinal(rules: &Rules, age: Option<DefId>) -> i64 {
    age.and_then(|a| rules.ages.get(&a))
        .map_or(0, |a| a.ordinal)
}

/// The price of a type: its cost, or, for a building made by upgrading another in place, the
/// price of what it is made from.
pub fn price(rules: &Rules, unit: DefId) -> i64 {
    let mut current = Some(unit);
    let mut seen = BTreeSet::new();
    while let Some(id) = current {
        if !seen.insert(id) {
            break;
        }
        let total: i64 = rules
            .costs
            .get(&id)
            .into_iter()
            .flatten()
            .map(|c| c.amount)
            .sum();
        if total > 0 {
            return total;
        }
        current = rules.entity_types.get(&id).and_then(|t| t.upgrades_from_id);
    }
    1
}

/// Whether a type can stand on the fight's ground: buildings always, units whose movement
/// class crosses it or flies (orbital craft, which climb, do not count).
fn stands_on(rules: &Rules, unit: DefId, terrain: DefId) -> bool {
    let et = &rules.entity_types[&unit];
    match &et.movement {
        None => et.class == EntityClass::Building,
        Some(m) => {
            if m.climb_ticks > 0 {
                return false;
            }
            let class = &rules.movement_classes[&m.movement_class_id];
            class.flies
                || rules
                    .movement_class_terrain
                    .get(&m.movement_class_id)
                    .into_iter()
                    .flatten()
                    .any(|row| row.terrain_id == terrain && row.speed_pct > 0)
        }
    }
}

/// The layer a type stands on in a fight.
fn layer_of(rules: &Rules, unit: DefId) -> Layer {
    let flies = rules.entity_types[&unit]
        .movement
        .as_ref()
        .is_some_and(|m| rules.movement_classes[&m.movement_class_id].flies);
    if flies { Layer::Air } else { Layer::Ground }
}

/// The farthest a type's weapons reach a target on `layer`, in milli-tiles; `None` when no
/// weapon can hit that layer (orbital strikes do not count on the ground).
pub(crate) fn reach_at(rules: &Rules, unit: DefId, layer: Layer) -> Option<i64> {
    rules.entity_types[&unit]
        .weapons
        .iter()
        .filter(|w| !w.orbit_only && w.targets.iter().any(|t| t.layer == layer))
        .map(|w| w.range_max.max(0))
        .max()
}

/// How far from its centre a type is hit: half its footprint for a building, its radius for
/// a unit (weapons measure to the edge).
pub(crate) fn extent(rules: &Rules, unit: DefId) -> i64 {
    let et = &rules.entity_types[&unit];
    if et.class == EntityClass::Building {
        et.footprint_w.max(et.footprint_h) * 500
    } else {
        et.radius.max(0)
    }
}

/// Whether `a` outranges `b`: `a` can hit `b` from farther, centre to centre, than `b` can
/// hit `a` (not at all, when no weapon of `b` reaches `a`'s layer).
pub fn outranges(rules: &Rules, a: DefId, b: DefId) -> bool {
    let Some(reach_a) = reach_at(rules, a, layer_of(rules, b)) else {
        return false;
    };
    match reach_at(rules, b, layer_of(rules, a)) {
        None => true,
        Some(reach_b) => reach_a + extent(rules, b) > reach_b + extent(rules, a),
    }
}

/// Whether a type is a fighter of the matrix: armed, buildable by the faction, able to stand
/// on the ground of the fight, and a unit that only fights (no worker, builder, trader or
/// carrier) or a building.
fn fighter(rules: &Rules, setup: &Setup, unit: DefId) -> bool {
    let et = &rules.entity_types[&unit];
    let fights = match et.class {
        EntityClass::Unit => {
            et.movement.is_some()
                && et.harvester.is_empty()
                && et.builder.is_none()
                && et.trader.is_none()
                && et.container.is_none()
        }
        EntityClass::Building => true,
        _ => false,
    };
    fights
        && rules.production_by_product.contains_key(&unit)
        && rules.is_available(unit, setup.faction)
        && stands_on(rules, unit, setup.terrain)
        && [Layer::Ground, Layer::Air]
            .iter()
            .any(|layer| reach_at(rules, unit, *layer).is_some())
}

/// The fighters of an age: of every upgrade line, the newest whose first age is this one or
/// earlier.
pub fn roster(rules: &Rules, setup: &Setup, age: i64) -> Vec<DefId> {
    let reached: BTreeSet<DefId> = rules
        .entity_types
        .values()
        .filter(|et| ordinal(rules, et.first_age_id) <= age)
        .map(|et| et.id)
        .collect();
    rules
        .entity_types
        .values()
        .filter(|et| {
            reached.contains(&et.id)
                && fighter(rules, setup, et.id)
                && !rules
                    .entity_types
                    .values()
                    .any(|next| next.upgrades_from_id == Some(et.id) && reached.contains(&next.id))
        })
        .map(|et| et.id)
        .collect()
}

/// The age a technology needs: the latest age among its prerequisites.
fn tech_age(rules: &Rules, tech: DefId) -> i64 {
    rules
        .prerequisites
        .get(&tech)
        .into_iter()
        .flatten()
        .filter_map(|p| rules.ages.get(&p.requires_def_id))
        .map(|a| a.ordinal)
        .max()
        .unwrap_or(0)
}

/// Researches of an age that change a combat stat of `unit` for its owner.
pub fn researches_for(rules: &Rules, age: i64, unit: DefId) -> Vec<DefId> {
    rules
        .techs
        .values()
        .filter(|t| {
            !matches!(
                t.kind,
                TechKind::Age | TechKind::Handicap | TechKind::Project | TechKind::Evolution
            ) && tech_age(rules, t.id) <= age
                && rules.modifiers_from(t.id).iter().any(|m| {
                    m.applies_to == AppliesTo::Owner
                        && COMBAT_STATS.contains(&m.stat_key.as_str())
                        && m.aura_radius.is_none()
                        && m.duration_ticks.is_none()
                        && (m.target_def_id == Some(unit)
                            || m.target_tag_id.is_some_and(|tag| rules.has_tag(unit, tag))
                            || (m.target_def_id.is_none() && m.target_tag_id.is_none()))
                })
        })
        .map(|t| t.id)
        .collect()
}

/// The setup of the matrix from the rules: the first faction of the first race (as in a
/// duel), the buildable ground terrain the most movement classes cross, and the observer.
pub fn setup(rules: &Rules) -> Result<Setup, String> {
    let faction = crate::duel_cmd::duel_faction(rules, 0)?;
    let terrain = rules
        .terrains
        .values()
        .filter(|t| t.buildable && t.layer == Layer::Ground)
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
        .ok_or("the rules have no buildable ground")?;
    let observer = rules
        .entity_types
        .values()
        .filter(|et| et.class == EntityClass::Unit && et.weapons.is_empty())
        .max_by_key(|et| (et.los, std::cmp::Reverse(et.id)))
        .map(|et| et.id);
    Ok(Setup {
        faction,
        terrain,
        observer,
    })
}

/// A small deterministic generator for the jitter of the starting places.
fn jitter(state: &mut u64) -> i32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 33) % 801) as i32 - 400
}

/// How many of a type a side fields for `budget`.
fn count_for(rules: &Rules, unit: DefId, budget: i64) -> i64 {
    (budget / price(rules, unit)).clamp(1, MAX_COUNT)
}

/// Plays one fight of `first` against `second` at equal cost on a seed: the shares of their
/// starting value each side still has at the end, per mille.
pub fn fight(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    first: Side,
    second: Side,
    seed: u64,
) -> Result<(i64, i64), String> {
    let reach =
        |a: DefId, b: DefId| reach_at(rules, a, layer_of(rules, b)).unwrap_or(0) + extent(rules, b);
    let gap = reach(first.unit, second.unit).max(reach(second.unit, first.unit));
    fight_on(
        rules,
        setup,
        age,
        [first, second],
        [setup.terrain; 2],
        gap,
        seed,
    )
}

/// Plays one fight at equal cost on a seed with each side's half of the field of its own
/// terrain (`terrains`; the sea rows put ships on water beside the shore), the sides `gap`
/// milli-tiles of reach apart besides the slack: the shares of their starting value each
/// side still has at the end, per mille.
pub(crate) fn fight_on(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    [first, second]: [Side; 2],
    terrains: [DefId; 2],
    gap: i64,
    seed: u64,
) -> Result<(i64, i64), String> {
    let age_id = rules
        .ages
        .values()
        .find(|a| a.ordinal == age)
        .ok_or("no such age")?
        .id;
    let players: Vec<Player> = [first, second]
        .iter()
        .enumerate()
        .map(|(i, side)| Player {
            id: PlayerId(i as u32 + 1),
            team: i as u32 + 1,
            faction: setup.faction,
            age: age_id,
            resources: BTreeMap::new(),
            techs: side.tech.map(|t| (t, 1)).into_iter().collect(),
        })
        .collect();
    let gap = gap + GAP_SLACK;
    let front = 40_000;
    // The halves meet midway between the two fronts.
    let shore = ((front + gap as i32 / 2) / 1000).max(0) as u32;
    let map = MapGrid::new(
        MAP_W,
        MAP_H,
        (0..MAP_W * MAP_H)
            .map(|i| terrains[usize::from(i % MAP_W >= shore)])
            .collect(),
    )
    .map_err(|e| e.to_string())?;
    let mut world = World::new(Arc::clone(rules), map, seed, players)
        .map_err(|e| e.to_string())?
        .with_match_options(MatchOptions {
            victories: BTreeSet::new(),
            ..MatchOptions::default()
        })
        .map_err(|e| e.to_string())?;
    let budget = BUDGET_UNITS * price(rules, first.unit).max(price(rules, second.unit));
    let centre_y = MAP_H as i32 * 500;
    let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    let mut spawned: [Vec<(rom_sim::EntityId, i64)>; 2] = [Vec::new(), Vec::new()];
    for (i, side) in [first, second].iter().enumerate() {
        let owner = PlayerId(i as u32 + 1);
        let et = &rules.entity_types[&side.unit];
        let count = count_for(rules, side.unit, budget);
        let building = et.class == EntityClass::Building;
        // Rows of five, in columns going back from the front; buildings two tiles apart.
        let spacing = if building {
            (et.footprint_w.max(et.footprint_h) + 2) * 1000
        } else {
            1500
        } as i32;
        let edge = if i == 0 {
            front - extent(rules, side.unit) as i32
        } else {
            front + gap as i32 + extent(rules, side.unit) as i32
        };
        let back = if i == 0 { -1 } else { 1 };
        for n in 0..count as i32 {
            let (column, row) = (n / 5, n % 5);
            let mut x = edge + back * column * spacing;
            let mut y = centre_y + (row - 2) * spacing;
            if !building {
                x += jitter(&mut state);
                y += jitter(&mut state);
            }
            let id = world
                .spawn(side.unit, owner, Pos::flat(x, y))
                .map_err(|e| format!("{}: {e}", rules.key(side.unit)))?;
            spawned[i].push((id, price(rules, side.unit)));
        }
        if let Some(observer) = setup.observer {
            // Far off to one side of the field, out of everyone's way.
            let x = if i == 0 {
                2_000
            } else {
                MAP_W as i32 * 1000 - 2_000
            };
            world
                .spawn(observer, owner, Pos::flat(x, 2_000))
                .map_err(|e| e.to_string())?;
        }
    }
    // Every unit goes for the other side's middle, fighting what it meets; a unit left idle
    // (its target fell or changed hands) is sent on again, as a player would.
    let goals = [
        Pos::flat(front + gap as i32 + 4000, centre_y),
        Pos::flat(front - 4000, centre_y),
    ];
    let send = |world: &mut World| -> Result<(), String> {
        for (i, list) in spawned.iter().enumerate() {
            for (id, _) in list {
                let idle = world.entity(*id).is_some_and(|e| {
                    e.hp > 0
                        && e.owner == PlayerId(i as u32 + 1)
                        && e.orders().is_empty()
                        && rules.entity_types[&e.type_id].movement.is_some()
                });
                if idle {
                    world
                        .submit(Command {
                            player: PlayerId(i as u32 + 1),
                            executor: *id,
                            kind: "attack_move".into(),
                            subject: None,
                            target_entity: None,
                            target_pos: Some(goals[i]),
                            queue: false,
                        })
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    };
    send(&mut world)?;
    // What a side still holds: its own entities standing (a captured city is lost).
    let left = |world: &World, side: usize| -> (i64, bool) {
        let owner = PlayerId(side as u32 + 1);
        let mut value = 0_i64;
        let mut start = 0_i64;
        let mut alive = false;
        for (id, worth) in &spawned[side] {
            start += worth;
            if let Some(e) = world
                .entity(*id)
                .filter(|e| e.hp > 0 && !e.neutralised && e.owner == owner)
            {
                value += worth * e.hp / e.max_hp.max(1);
                alive = true;
            }
        }
        (value * 1000 / start.max(1), alive)
    };
    let trace = std::env::var("ROM_FIGHT_TRACE").is_ok();
    while world.tick() < FIGHT_TICKS {
        world.step();
        if trace && world.tick() % 200 == 0 {
            for list in &spawned {
                let line: Vec<String> = list
                    .iter()
                    .filter_map(|(id, _)| world.entity(*id))
                    .map(|e| {
                        format!(
                            "{}:{},{}:{}:{}",
                            e.id.0,
                            e.pos.x / 100,
                            e.pos.y / 100,
                            e.hp,
                            e.orders().front().map_or("-", |o| o.kind.as_str())
                        )
                    })
                    .collect();
                eprintln!("t={} {}", world.tick(), line.join(" "));
            }
        }
        if world.tick() % 100 == 0 {
            send(&mut world)?;
        }
        if world.tick() % 20 == 0 {
            let (_, a) = left(&world, 0);
            let (_, b) = left(&world, 1);
            if !a || !b {
                break;
            }
        }
    }
    Ok((left(&world, 0).0, left(&world, 1).0))
}

/// Fights `first` against `second` on `SEEDS` seeds.
pub fn pairing(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    first: Side,
    second: Side,
    seeds: u64,
) -> Result<Pairing, String> {
    let mut result = Pairing::default();
    for seed in 1..=seeds {
        let (a, b) = fight(rules, setup, age, first, second, seed)?;
        result.seeds += 1;
        result.first_left += a;
        result.second_left += b;
        if a > b {
            result.wins += 1;
        }
    }
    Ok(result)
}

/// Mobile fighters of a roster: what can sally out against a strength.
fn mobile(rules: &Rules, roster: &[DefId]) -> Vec<DefId> {
    roster
        .iter()
        .copied()
        .filter(|u| rules.entity_types[u].movement.is_some())
        .collect()
}

/// Measures one age: every strength and the best answer to it.
pub fn measure_age(
    rules: &Arc<Rules>,
    setup: &Setup,
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
    let roster = roster(rules, setup, age);
    let plain = |unit| Side { unit, tech: None };
    let mut strengths = Vec::new();
    for &strong in &roster {
        let mut beaten = Vec::new();
        for &target in &roster {
            if target == strong || !outranges(rules, strong, target) {
                continue;
            }
            if pairing(rules, setup, age, plain(strong), plain(target), seeds)?.first_wins() {
                beaten.push(target);
                if !all {
                    break;
                }
            }
        }
        if beaten.is_empty() {
            continue;
        }
        // The best answer without research; failing that, with one research of the age.
        let mut best: Option<(Side, Pairing)> = None;
        let better = |best: &Option<(Side, Pairing)>, p: &Pairing| {
            best.is_none_or(|(_, b)| {
                (p.wins, p.first_left - p.second_left) > (b.wins, b.first_left - b.second_left)
            })
        };
        let counters = mobile(rules, &roster);
        for &counter in counters.iter().filter(|c| **c != strong) {
            let p = pairing(rules, setup, age, plain(counter), plain(strong), seeds)?;
            if better(&best, &p) {
                best = Some((plain(counter), p));
            }
        }
        if !best.is_some_and(|(_, p)| p.first_wins()) {
            for &counter in counters.iter().filter(|c| **c != strong) {
                for tech in researches_for(rules, age, counter) {
                    let side = Side {
                        unit: counter,
                        tech: Some(tech),
                    };
                    let p = pairing(rules, setup, age, side, plain(strong), seeds)?;
                    if p.first_wins() && better(&best, &p) {
                        best = Some((side, p));
                    }
                }
            }
        }
        strengths.push(Strength {
            strong,
            beaten,
            answer: best,
        });
    }
    Ok(AgeMatrix {
        age: age_id,
        roster,
        strengths,
    })
}

/// Measures the ages in `ages` on `jobs` threads, in age order.
pub fn measure(
    rules: &Arc<Rules>,
    ages: (i64, i64),
    seeds: u64,
    jobs: usize,
    all: bool,
) -> Result<Vec<AgeMatrix>, String> {
    let setup = setup(rules)?;
    let list: Vec<i64> = (ages.0..=ages.1).collect();
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
                    let result = measure_age(rules, &setup, *age, seeds, all);
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

/// Measures the orbit rows of the ages in `ages` that have an orbital attacker, on `jobs`
/// threads, in age order.
pub fn measure_orbit(
    rules: &Arc<Rules>,
    ages: (i64, i64),
    seeds: u64,
    jobs: usize,
    all: bool,
) -> Result<Vec<crate::orbit_matrix::OrbitMatrix>, String> {
    let setup = setup(rules)?;
    let Some(first) = crate::orbit_matrix::first_orbit_age(rules, &setup) else {
        return Ok(Vec::new());
    };
    let list: Vec<i64> = (ages.0.max(first)..=ages.1).collect();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(age) = list.get(i) else {
                        break;
                    };
                    let result = crate::orbit_matrix::measure_age(rules, &setup, *age, seeds, all);
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

/// Measures ship against ship in orbit (task 92) for the ages in `ages` that have an orbital
/// attacker, on `jobs` threads, in age order.
pub fn measure_space(
    rules: &Arc<Rules>,
    ages: (i64, i64),
    seeds: u64,
    jobs: usize,
    all: bool,
) -> Result<Vec<crate::orbit_matrix::OrbitMatrix>, String> {
    let setup = setup(rules)?;
    let Some(first) = crate::orbit_matrix::first_orbit_age(rules, &setup) else {
        return Ok(Vec::new());
    };
    let list: Vec<i64> = (ages.0.max(first)..=ages.1).collect();
    let next = AtomicUsize::new(0);
    let results = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(age) = list.get(i) else {
                        break;
                    };
                    let result =
                        crate::orbit_matrix::measure_space_age(rules, &setup, *age, seeds, all);
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

/// The orbit rows of one age, as Markdown rows.
pub fn orbit_table(
    rules: &Rules,
    matrix: &crate::orbit_matrix::OrbitMatrix,
    seeds: u64,
) -> Vec<String> {
    let strengths: Vec<Strength> = matrix
        .strengths
        .iter()
        .map(|s| Strength {
            strong: s.strong,
            beaten: s.beaten.clone(),
            answer: s.answer,
        })
        .collect();
    table(
        rules,
        &AgeMatrix {
            age: matrix.age,
            roster: matrix.roster.clone(),
            strengths,
        },
        seeds,
    )
}

/// Strengths of a matrix without an answer that wins.
pub fn unanswered(strengths: &[Strength]) -> usize {
    strengths
        .iter()
        .filter(|s| !s.answer.is_some_and(|(_, p)| p.first_wins()))
        .count()
}

/// A type's key without its kind prefix.
fn short(rules: &Rules, id: DefId) -> &str {
    let key = rules.key(id);
    key.split_once('.').map_or(key, |(_, rest)| rest)
}

/// The table of one age, as Markdown rows.
pub fn table(rules: &Rules, matrix: &AgeMatrix, seeds: u64) -> Vec<String> {
    let age = rules.key(matrix.age);
    let mut rows = Vec::new();
    for s in &matrix.strengths {
        let beaten: Vec<&str> = s.beaten.iter().map(|b| short(rules, *b)).collect();
        let (answer, research, result) = match &s.answer {
            Some((side, p)) => (
                short(rules, side.unit).to_string(),
                side.tech
                    .map_or("-".to_string(), |t| short(rules, t).to_string()),
                format!(
                    "{}/{} ({}‰ left vs {}‰){}",
                    p.wins,
                    seeds,
                    p.first_left / p.seeds.max(1) as i64,
                    p.second_left / p.seeds.max(1) as i64,
                    if p.first_wins() { "" } else { " NO ANSWER" }
                ),
            ),
            None => ("-".into(), "-".into(), "NO ANSWER".into()),
        };
        rows.push(format!(
            "| {age} | {} | {} | {answer} | {research} | {result} |",
            short(rules, s.strong),
            beaten.join(", ")
        ));
    }
    rows
}

/// Parses `first..last` (both included) or a single age ordinal.
fn parse_range(value: &str) -> Result<(i64, i64), String> {
    let (a, b) = value.split_once("..").unwrap_or((value, value));
    let first = a.parse::<i64>().map_err(|e| format!("bad ages: {e}"))?;
    let last = b.parse::<i64>().map_err(|e| format!("bad ages: {e}"))?;
    if last < first {
        return Err("bad ages: the last is before the first".into());
    }
    Ok((first, last))
}

/// Runs the subcommand with the arguments after `fight-matrix`.
pub fn run_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut db = None;
    let mut ages = None;
    let mut one: Option<(String, String)> = None;
    let mut seeds = SEEDS;
    let mut all = false;
    let (mut orbit_only, mut no_orbit, mut no_sea) = (false, false, false);
    // Where a single fight is fought: on the ground, across the orbit or at sea.
    let mut field = "--fight";
    let mut jobs = std::thread::available_parallelism().map_or(1, |n| n.get().min(4));
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--all" => all = true,
            "--orbit-only" => orbit_only = true,
            "--no-orbit" => no_orbit = true,
            "--no-sea" => no_sea = true,
            _ => {}
        }
        if matches!(
            flag.as_str(),
            "--all" | "--orbit-only" | "--no-orbit" | "--no-sea"
        ) {
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--ages" => ages = Some(parse_range(&value)?),
            "--seeds" => seeds = value.parse::<u64>()?.max(1),
            "--jobs" => jobs = value.parse::<usize>()?.max(1),
            "--fight" | "--orbit-fight" | "--sea-fight" => {
                let (a, b) = value.split_once(',').ok_or("--fight a,b")?;
                one = Some((a.to_string(), b.to_string()));
                field = match flag.as_str() {
                    "--orbit-fight" => "--orbit-fight",
                    "--sea-fight" => "--sea-fight",
                    _ => "--fight",
                };
            }
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or(USAGE)?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    let last = rules.ages.values().map(|a| a.ordinal).max().unwrap_or(0);
    let ages = ages.unwrap_or((0, last));
    if let Some((a, b)) = one {
        let setup = setup(&rules)?;
        let side = |key: &str| -> Result<Side, String> {
            let (unit, tech) = key
                .split_once('+')
                .map_or((key, None), |(u, t)| (u, Some(t)));
            Ok(Side {
                unit: rules.id_of(unit).ok_or(format!("unknown {unit}"))?,
                tech: match tech {
                    Some(t) => Some(rules.id_of(t).ok_or(format!("unknown {t}"))?),
                    None => None,
                },
            })
        };
        for seed in 1..=seeds {
            let (x, y) = match field {
                "--orbit-fight" => {
                    crate::orbit_matrix::fight(&rules, &setup, ages.0, side(&a)?, side(&b)?, seed)?
                }
                "--sea-fight" => {
                    crate::sea_matrix::fight(&rules, &setup, ages.0, side(&a)?, side(&b)?, seed)?
                }
                _ => fight(&rules, &setup, ages.0, side(&a)?, side(&b)?, seed)?,
            };
            println!("fight seed={seed} {a}={x} {b}={y}");
        }
        return Ok(());
    }
    let matrices = if orbit_only {
        Vec::new()
    } else {
        measure(&rules, ages, seeds, jobs, all)?
    };
    let seas = if orbit_only || no_sea {
        Vec::new()
    } else {
        crate::sea_matrix::measure(&rules, ages, seeds, jobs, all)?
    };
    let orbits = if no_orbit {
        Vec::new()
    } else {
        measure_orbit(&rules, ages, seeds, jobs, all)?
    };
    let spaces = if no_orbit {
        Vec::new()
    } else {
        measure_space(&rules, ages, seeds, jobs, all)?
    };
    let mut missing = 0;
    println!(
        "| Age | Strength | Outranges and beats | Answer | Research | Answer's fights (won, value left) |"
    );
    println!("| --- | --- | --- | --- | --- | --- |");
    for matrix in &matrices {
        for row in table(&rules, matrix, seeds) {
            println!("{row}");
        }
        missing += matrix
            .strengths
            .iter()
            .filter(|s| !s.answer.is_some_and(|(_, p)| p.first_wins()))
            .count();
    }
    for matrix in &matrices {
        let roster: Vec<&str> = matrix.roster.iter().map(|u| short(&rules, *u)).collect();
        println!("roster {}: {}", rules.key(matrix.age), roster.join(", "));
    }
    if !seas.is_empty() {
        println!();
        println!(
            "| Age | At sea | Outranges and beats | Answer | Research | Answer's fights (won, value left) |"
        );
        println!("| --- | --- | --- | --- | --- | --- |");
        for matrix in &seas {
            for row in table(&rules, matrix, seeds) {
                println!("{row}");
            }
            missing += unanswered(&matrix.strengths);
        }
        for matrix in &seas {
            let roster: Vec<&str> = matrix.roster.iter().map(|u| short(&rules, *u)).collect();
            println!(
                "sea roster {}: {}",
                rules.key(matrix.age),
                roster.join(", ")
            );
        }
    }
    if !orbits.is_empty() {
        println!();
        println!(
            "| Age | Across the orbit | Outranges and beats | Answer | Research | Answer's fights (won, value left) |"
        );
        println!("| --- | --- | --- | --- | --- | --- |");
        for matrix in &orbits {
            for row in orbit_table(&rules, matrix, seeds) {
                println!("{row}");
            }
            missing += matrix
                .strengths
                .iter()
                .filter(|s| !s.answer.is_some_and(|(_, p)| p.first_wins()))
                .count();
        }
        for matrix in &orbits {
            let roster: Vec<&str> = matrix.roster.iter().map(|u| short(&rules, *u)).collect();
            println!(
                "orbit roster {}: {}",
                rules.key(matrix.age),
                roster.join(", ")
            );
        }
        println!();
        println!(
            "| Age | Ship against ship | Outranges and beats | Answer | Research | Answer's fights (won, value left) |"
        );
        println!("| --- | --- | --- | --- | --- | --- |");
        for matrix in &spaces {
            for row in orbit_table(&rules, matrix, seeds) {
                println!("{row}");
            }
            missing += matrix
                .strengths
                .iter()
                .filter(|s| !s.answer.is_some_and(|(_, p)| p.first_wins()))
                .count();
        }
        for matrix in &spaces {
            let roster: Vec<&str> = matrix.roster.iter().map(|u| short(&rules, *u)).collect();
            println!(
                "space roster {}: {}",
                rules.key(matrix.age),
                roster.join(", ")
            );
        }
        let setup = setup(&rules)?;
        println!();
        println!(
            "| Age | Ship | Volleys to level the town | Volleys to the city | Buildings | Reach of the town (tiles) | Soldiers left |"
        );
        println!("| --- | --- | --- | --- | --- | --- | --- |");
        for row in crate::orbit_matrix::levelling_table(&rules, &setup)?
            .iter()
            .filter(|r| (ages.0..=ages.1).contains(&rules.ages[&r.age].ordinal))
        {
            println!(
                "| {} | {} | {} | {} | {} | {:.1} | {}/{} |",
                rules.key(row.age),
                short(&rules, row.ship),
                row.volleys
                    .map_or("not levelled".to_string(), |v| v.to_string()),
                row.city_volleys.map_or("-".to_string(), |v| v.to_string()),
                row.buildings,
                row.spread as f64 / 1000.0,
                row.units.1,
                row.units.0
            );
        }
    }
    println!(
        "fight-matrix strengths={} unanswered={missing}",
        matrices
            .iter()
            .chain(seas.iter())
            .map(|m| m.strengths.len())
            .sum::<usize>()
            + orbits
                .iter()
                .chain(spaces.iter())
                .map(|m| m.strengths.len())
                .sum::<usize>()
    );
    if missing > 0 {
        return Err(format!("{missing} strengths have no answer").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::build_rules_db;

    fn rules() -> Arc<Rules> {
        let db = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        Arc::new(Rules::load(&db).unwrap())
    }

    fn jobs() -> usize {
        std::thread::available_parallelism().map_or(1, |n| n.get())
    }

    /// The owner's rule: every unit or armed building that outranges and beats something
    /// has an answer in its age, now or after one research of that age.
    #[test]
    fn every_strength_has_an_answer() {
        let rules = rules();
        let last = rules.ages.values().map(|a| a.ordinal).max().unwrap();
        let matrices = measure(&rules, (0, last), SEEDS, jobs(), false).unwrap();
        assert_eq!(matrices.len() as i64, last + 1);
        let unanswered: Vec<String> = matrices
            .iter()
            .flat_map(|m| {
                m.strengths
                    .iter()
                    .filter(|s| !s.answer.is_some_and(|(_, p)| p.first_wins()))
                    .map(|s| format!("{}: {}", rules.key(m.age), rules.key(s.strong)))
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(
            unanswered.is_empty(),
            "strengths without an answer: {unanswered:?}"
        );
        // Every age has fighters, and siege (a weapon that outranges the city's) is found
        // to be a strength wherever it exists.
        assert!(matrices.iter().all(|m| !m.roster.is_empty()));
        assert!(matrices.iter().map(|m| m.strengths.len()).sum::<usize>() > 0);
    }

    #[test]
    fn a_fight_repeats_and_reads_reach_from_the_weapons() {
        let rules = rules();
        let setup = setup(&rules).unwrap();
        let roster = roster(&rules, &setup, 1);
        // Something in the second age outranges something else there.
        let (a, b) = roster
            .iter()
            .flat_map(|a| roster.iter().map(move |b| (*a, *b)))
            .find(|(a, b)| a != b && outranges(&rules, *a, *b))
            .expect("an outranging pair");
        assert!(!outranges(&rules, b, a));
        let side = |unit| Side { unit, tech: None };
        let first = fight(&rules, &setup, 1, side(a), side(b), 7).unwrap();
        let second = fight(&rules, &setup, 1, side(a), side(b), 7).unwrap();
        assert_eq!(first, second);
        assert!((0..=1000).contains(&first.0) && (0..=1000).contains(&first.1));
    }
}

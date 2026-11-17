//! The orbit rows of `rom fight-matrix` (task 75): ships and craft that bombard the ground from
//! orbit, the ground batteries that fire up at them, and how many volleys of one ship level a
//! town of its age.
//!
//! The owner's rule applies across the orbit as on the ground: whatever outranges and beats a
//! target type has a countermeasure in the same age, effective now or after one research. The
//! fights are played on a small planet of plain ground in a star system of its own, at equal
//! cost, with the same measure as the ground matrix (a side wins a seed when it keeps a larger
//! share of its starting value). Everything is read from the rules: an orbital attacker is a
//! type with an `orbit_only` weapon, a ground defence against orbit a type standing on the
//! ground with a weapon at the `orbit` layer. No definition is named.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, EntityClass, Layer, Rules};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, MatchOptions, PlanetSurface, Player, PlayerId, Pos,
    SystemInfo, UniverseSetup, World,
};

use crate::fight_matrix_cmd::{Pairing, Setup, Side, price, researches_for, roster};

/// Tiles across a face of the fight's planet.
const N: u32 = 96;
/// The planet the fights are fought over.
const PLANET: BodyId = BodyId(2);
/// Each side spends this many times the dearer side's price (as on the ground).
const BUDGET_UNITS: i64 = 3;
/// The most of one type on a side.
const MAX_COUNT: i64 = 24;
/// A fight that has not ended after this long is judged as it stands (five game minutes).
const FIGHT_TICKS: u64 = 6000;
/// Space between the two sides beyond both reaches, in milli-tiles.
const GAP_SLACK: i64 = 6000;
/// Ticks one ship may take to level a town before the count gives up.
const LEVEL_TICKS: u64 = 12_000;
/// Tiles of lane kept free around every building of a measured town.
const LANE: i32 = 1;
/// Soldiers standing in a measured town, on four rings out to its edge.
const SOLDIERS: usize = 20;

/// How a type takes the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// Stands on the ground: a building or a ground unit.
    Ground,
    /// Flies from system space into the planet's orbit (a spacecraft).
    Ship,
    /// Climbs from the ground into the planet's orbit (an orbital craft).
    Craft,
}

fn place_of(world: &World, unit: DefId) -> Place {
    if world.is_spacecraft_type(unit) {
        Place::Ship
    } else if world.is_craft_type(unit) {
        Place::Craft
    } else {
        Place::Ground
    }
}

/// The ordinal of a type's first age.
fn first_age(rules: &Rules, unit: DefId) -> i64 {
    rules.entity_types[&unit]
        .first_age_id
        .and_then(|a| rules.ages.get(&a))
        .map_or(0, |a| a.ordinal)
}

/// Whether a type has a weapon that strikes the ground from orbit.
fn bombards(rules: &Rules, unit: DefId) -> bool {
    rules.entity_types[&unit]
        .weapons
        .iter()
        .any(|w| w.orbit_only)
}

/// Whether a type has a weapon that reaches the `orbit` layer.
fn reaches_orbit(rules: &Rules, unit: DefId) -> bool {
    rules.entity_types[&unit]
        .weapons
        .iter()
        .any(|w| !w.orbit_only && w.targets.iter().any(|t| t.layer == Layer::Orbit))
}

/// The farthest a weapon of `unit` reaches across the orbit: down from orbit (`down`), or up
/// at the orbit, in milli-tiles of ground.
fn orbit_reach(rules: &Rules, unit: DefId, down: bool) -> i64 {
    rules.entity_types[&unit]
        .weapons
        .iter()
        .filter(|w| {
            if down {
                w.orbit_only
            } else {
                !w.orbit_only && w.targets.iter().any(|t| t.layer == Layer::Orbit)
            }
        })
        .map(|w| w.range_max.max(0))
        .max()
        .unwrap_or(0)
}

/// Half the footprint of a building, else the unit's radius.
fn extent(rules: &Rules, unit: DefId) -> i64 {
    let et = &rules.entity_types[&unit];
    if et.class == EntityClass::Building {
        et.footprint_w.max(et.footprint_h) * 500
    } else {
        et.radius.max(0)
    }
}

/// Types available to the faction and reached in `age` that are the newest of their upgrade
/// line, filtered by `keep`.
fn newest(rules: &Rules, setup: &Setup, age: i64, keep: impl Fn(DefId) -> bool) -> Vec<DefId> {
    let reached: BTreeSet<DefId> = rules
        .entity_types
        .values()
        .filter(|et| first_age(rules, et.id) <= age)
        .map(|et| et.id)
        .collect();
    rules
        .entity_types
        .values()
        .filter(|et| {
            reached.contains(&et.id)
                && rules.production_by_product.contains_key(&et.id)
                && rules.is_available(et.id, setup.faction)
                && keep(et.id)
                && !rules
                    .entity_types
                    .values()
                    .any(|next| next.upgrades_from_id == Some(et.id) && reached.contains(&next.id))
        })
        .map(|et| et.id)
        .collect()
}

/// Orbital attackers of an age: armed units that strike the ground from orbit.
pub fn attackers(rules: &Rules, setup: &Setup, age: i64) -> Vec<DefId> {
    newest(rules, setup, age, |u| {
        rules.entity_types[&u].class == EntityClass::Unit && bombards(rules, u)
    })
}

/// Ground defences of an age against orbit: buildings and ground units (not craft, not ships)
/// with a weapon at the `orbit` layer.
fn ground_defences(rules: &Rules, setup: &Setup, age: i64, world: &World) -> Vec<DefId> {
    newest(rules, setup, age, |u| {
        reaches_orbit(rules, u) && place_of(world, u) == Place::Ground && {
            let et = &rules.entity_types[&u];
            et.class == EntityClass::Building
                || (et.movement.is_some()
                    && et.container.is_none()
                    && et.builder.is_none()
                    && et.harvester.is_empty())
        }
    })
}

/// Interceptors of an age: craft and armed ships with a weapon at the `orbit` layer that are not
/// carriers.
fn interceptors(rules: &Rules, setup: &Setup, age: i64, world: &World) -> Vec<DefId> {
    newest(rules, setup, age, |u| {
        let et = &rules.entity_types[&u];
        et.class == EntityClass::Unit
            && et.container.is_none()
            && place_of(world, u) != Place::Ground
            && reaches_orbit(rules, u)
    })
}

/// The ordinal of the first age with an orbital attacker; `None` without one.
pub fn first_orbit_age(rules: &Rules, setup: &Setup) -> Option<i64> {
    let last = rules.ages.values().map(|a| a.ordinal).max()?;
    (0..=last).find(|age| !attackers(rules, setup, *age).is_empty())
}

/// A star system of its own with one planet of plain ground, both players in `age`.
fn planet_world(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    techs: [Vec<DefId>; 2],
    seed: u64,
) -> Result<World, String> {
    let age_id = rules
        .ages
        .values()
        .find(|a| a.ordinal == age)
        .ok_or("no such age")?
        .id;
    let players = (0..=2u32)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: setup.faction,
            age: age_id,
            resources: BTreeMap::new(),
            techs: if p == 0 {
                BTreeMap::new()
            } else {
                techs[p as usize - 1].iter().map(|t| (*t, 1)).collect()
            },
        })
        .collect();
    let body = |n, kind, radius| BodyInfo {
        id: BodyId(n),
        system: 1,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: 0,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: None,
    };
    let side = (N * N) as usize;
    let universe = UniverseSetup {
        figure: Vec::new(),
        systems: vec![SystemInfo {
            z: 0,
            constellation: String::new(),
            id: 1,
            x: 0,
            y: 0,
            name: "Proving Ground".into(),
        }],
        bodies: vec![
            body(1, BodyKind::Star, 0),
            body(PLANET.0, BodyKind::Habitable, 20),
        ],
        routes: Vec::new(),
        planets: BTreeMap::from([(
            PLANET,
            PlanetSurface {
                surface: CubeSurface { n: N },
                terrain: vec![setup.terrain; 6 * side],
                nodes: vec![],
            },
        )]),
    };
    World::from_universe(
        Arc::clone(rules),
        universe,
        seed,
        players,
        BTreeMap::from([(PlayerId(1), PLANET), (PlayerId(2), PLANET)]),
    )
    .map_err(|e| e.to_string())?
    .with_match_options(MatchOptions {
        victories: BTreeSet::new(),
        ..MatchOptions::default()
    })
    .map_err(|e| e.to_string())
}

/// A point on face 0 of the planet, in milli-tiles.
fn at(x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        body: PLANET,
    }
}

/// Puts one entity of `unit` for `owner` above or on `pos`: a building or a ground unit there,
/// a ship sent into the orbit above it, a craft on the ground there, climbing.
fn field(world: &mut World, unit: DefId, owner: PlayerId, pos: Pos) -> Result<EntityId, String> {
    let key = world.rules().key(unit).to_string();
    match place_of(world, unit) {
        Place::Ground => if world.rules().entity_types[&unit].class == EntityClass::Building {
            world.place_for_setup(unit, owner, pos)
        } else {
            world.spawn(unit, owner, pos)
        }
        .map_err(|e| format!("{key}: {e}")),
        Place::Ship => {
            let marker = world.body_position(PLANET).ok_or("no planet marker")?;
            let id = world
                .spawn(
                    unit,
                    owner,
                    Pos {
                        x: marker.x + 1000,
                        ..marker
                    },
                )
                .map_err(|e| format!("{key}: {e}"))?;
            order(world, id, "orbit", None, Some(pos))?;
            Ok(id)
        }
        Place::Craft => {
            let id = world
                .spawn(unit, owner, pos)
                .map_err(|e| format!("{key}: {e}"))?;
            order(world, id, "lift", None, None)?;
            Ok(id)
        }
    }
}

fn order(
    world: &mut World,
    id: EntityId,
    kind: &str,
    target: Option<EntityId>,
    pos: Option<Pos>,
) -> Result<(), String> {
    let player = world.entity(id).ok_or("no executor")?.owner;
    world
        .submit(Command {
            player,
            executor: id,
            kind: kind.into(),
            subject: None,
            target_entity: target,
            target_pos: pos,
            queue: false,
        })
        .map(|_| ())
        .map_err(|e| format!("{kind}: {e}"))
}

/// Whether an entity is still getting into place: a ship on its way to orbit, a craft climbing.
fn arriving(world: &World, e: &rom_sim::Entity) -> bool {
    match place_of(world, e.type_id) {
        Place::Ground => false,
        Place::Ship | Place::Craft => world.orbit(e.id).is_none(),
    }
}

/// Whether an entity waits for an order: nothing queued, or only the orbit it holds.
fn idle(world: &World, e: &rom_sim::Entity) -> bool {
    match e.orders().front() {
        None => true,
        Some(o) => o.kind == "orbit" && world.orbit(e.id).is_some() && e.orders().len() == 1,
    }
}

/// Where an entity is measured from: its ground point in orbit, else where it stands.
fn ground_point(world: &World, id: EntityId) -> Option<Pos> {
    world
        .orbit(id)
        .map(|o| o.pos)
        .or_else(|| world.entity(id).map(|e| e.pos))
        .filter(|p| p.body == PLANET)
}

/// Orders every idle armed unit of `owner` among `ids` to attack the nearest of `foes` it can
/// aim at; units that cannot aim at anything stay.
fn send(world: &mut World, ids: &[EntityId], owner: PlayerId, foes: &[EntityId]) {
    let enemies: Vec<EntityId> = foes
        .iter()
        .copied()
        .filter(|id| {
            world.entity(*id).is_some_and(|e| {
                e.owner != owner && e.hp > 0 && !e.neutralised && e.container_id.is_none()
            })
        })
        .collect();
    for id in ids {
        let Some(e) = world.entity(*id) else {
            continue;
        };
        if e.hp <= 0
            || e.owner != owner
            || world.rules().entity_types[&e.type_id].movement.is_none()
            || world.rules().entity_types[&e.type_id].weapons.is_empty()
            || arriving(world, e)
            || !idle(world, e)
        {
            continue;
        }
        let Some(from) = ground_point(world, *id) else {
            continue;
        };
        let target = enemies
            .iter()
            .filter(|t| world.can_target(*id, **t))
            .filter_map(|t| {
                let to = ground_point(world, *t)?;
                Some((world.distance(from, to)?, *t))
            })
            .min();
        if let Some((_, target)) = target {
            // A target out of sight is left for later.
            if let Err(e) = order(world, *id, "attack", Some(target), None)
                && std::env::var("ROM_FIGHT_TRACE").is_ok()
            {
                eprintln!("t={} {} -> {}: {e}", world.tick(), id.0, target.0);
            }
        }
    }
}

/// The share of their starting value `owner`'s entities among `list` still have, per mille.
fn value_left(world: &World, list: &[(EntityId, i64)], owner: PlayerId) -> (i64, bool) {
    let (mut value, mut start, mut alive) = (0_i64, 0_i64, false);
    for (id, worth) in list {
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
}

/// The farthest `a` reaches `b` across the planet and its orbit, centre to centre.
fn reach(world: &World, a: DefId, b: DefId) -> i64 {
    let rules = world.rules();
    let (pa, pb) = (place_of(world, a), place_of(world, b));
    let aloft = |p: Place| p != Place::Ground;
    let range = if aloft(pa) && !aloft(pb) {
        orbit_reach(rules, a, true)
    } else if aloft(pb) {
        orbit_reach(rules, a, false)
    } else {
        rules.entity_types[&a]
            .weapons
            .iter()
            .filter(|w| !w.orbit_only)
            .map(|w| w.range_max.max(0))
            .max()
            .unwrap_or(0)
    };
    range + extent(rules, b)
}

/// One fight of `first` against `second` at equal cost over the planet: the share of their
/// starting value each side still has at the end, per mille.
pub fn fight(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    first: Side,
    second: Side,
    seed: u64,
) -> Result<(i64, i64), String> {
    let techs = [first, second].map(|s| s.tech.into_iter().collect());
    let mut world = planet_world(rules, setup, age, techs, seed)?;
    let budget = BUDGET_UNITS * price(rules, first.unit).max(price(rules, second.unit));
    let gap = reach(&world, first.unit, second.unit).max(reach(&world, second.unit, first.unit))
        + GAP_SLACK;
    let centre = N as i32 * 500;
    let front = centre - gap as i32 / 2;
    let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    let mut spawned: [Vec<(EntityId, i64)>; 2] = [Vec::new(), Vec::new()];
    for (i, side) in [first, second].iter().enumerate() {
        let owner = PlayerId(i as u32 + 1);
        let et = &rules.entity_types[&side.unit];
        let count = (budget / price(rules, side.unit)).clamp(1, MAX_COUNT);
        let building = et.class == EntityClass::Building;
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
            let mut y = centre + (row - 2) * spacing;
            if !building {
                x += jitter(&mut state);
                y += jitter(&mut state);
            }
            let id = field(&mut world, side.unit, owner, at(x, y))?;
            spawned[i].push((id, price(rules, side.unit)));
        }
        if let Some(observer) = setup.observer {
            // Off to one side of the middle of the field, out of everyone's way.
            let y = if i == 0 {
                8_000
            } else {
                N as i32 * 1000 - 8_000
            };
            world
                .spawn(observer, owner, at(centre, y))
                .map_err(|e| e.to_string())?;
        }
    }
    let ids: [Vec<EntityId>; 2] = [0, 1].map(|i| spawned[i].iter().map(|(id, _)| *id).collect());
    let trace = std::env::var("ROM_FIGHT_TRACE").is_ok();
    while world.tick() < FIGHT_TICKS {
        if trace && world.tick() % 200 == 0 {
            for list in &spawned {
                let line: Vec<String> = list
                    .iter()
                    .filter_map(|(id, _)| world.entity(*id))
                    .map(|e| {
                        let p = ground_point(&world, e.id).unwrap_or(e.pos);
                        format!(
                            "{}:{},{}{}:{}:{}",
                            e.id.0,
                            p.x / 100,
                            p.y / 100,
                            if world.orbit(e.id).is_some() { "^" } else { "" },
                            e.hp,
                            e.orders().front().map_or("-", |o| o.kind.as_str())
                        )
                    })
                    .collect();
                eprintln!("t={} {}", world.tick(), line.join(" "));
            }
            for p in [PlayerId(1), PlayerId(2)] {
                for f in world
                    .order_failures(p)
                    .filter(|f| f.tick + 200 > world.tick())
                {
                    eprintln!("  failure {:?} {} {}", p, f.kind, f.reason);
                }
            }
        }
        if world.tick() % 20 == 0 {
            send(&mut world, &ids[0], PlayerId(1), &ids[1]);
            send(&mut world, &ids[1], PlayerId(2), &ids[0]);
        }
        world.step();
        if world.tick() % 20 == 0 {
            let (_, a) = value_left(&world, &spawned[0], PlayerId(1));
            let (_, b) = value_left(&world, &spawned[1], PlayerId(2));
            if !a || !b {
                break;
            }
        }
    }
    Ok((
        value_left(&world, &spawned[0], PlayerId(1)).0,
        value_left(&world, &spawned[1], PlayerId(2)).0,
    ))
}

fn jitter(state: &mut u64) -> i32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 33) % 801) as i32 - 400
}

/// Fights `first` against `second` on `seeds` seeds.
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

/// A strength across the orbit and the answer to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitStrength {
    /// The orbital attacker or ground defence that outranges and beats.
    pub strong: DefId,
    /// What it outranges and beats.
    pub beaten: Vec<DefId>,
    /// The best countermeasure: its side, and its fight against the strong one.
    pub answer: Option<(Side, Pairing)>,
}

/// The orbit rows of one age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitMatrix {
    /// The age.
    pub age: DefId,
    /// Orbital attackers, ground defences against orbit and interceptors of the age.
    pub roster: Vec<DefId>,
    /// Every strength across the orbit in the age.
    pub strengths: Vec<OrbitStrength>,
}

/// The best answer to `strong` among `counters`, without research and then with one research.
fn best_answer(
    rules: &Arc<Rules>,
    setup: &Setup,
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
        let p = pairing(rules, setup, age, plain(counter), plain(strong), seeds)?;
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
    Ok(best)
}

/// Measures the orbit rows of one age: every orbital attacker that levels what stands below it
/// and the answer to it, and every ground defence that outranges and beats the ships above it
/// and the answer to that.
pub fn measure_age(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    seeds: u64,
    all: bool,
) -> Result<OrbitMatrix, String> {
    let age_id = rules
        .ages
        .values()
        .find(|a| a.ordinal == age)
        .ok_or("no such age")?
        .id;
    let probe = planet_world(rules, setup, age, [Vec::new(), Vec::new()], 1)?;
    let attackers = attackers(rules, setup, age);
    let defences = ground_defences(rules, setup, age, &probe);
    let interceptors = interceptors(rules, setup, age, &probe);
    // Ground fighters that can walk up to a defence; the city type stands for the town.
    let ground: Vec<DefId> = roster(rules, setup, age)
        .into_iter()
        .filter(|u| {
            rules.entity_types[u].movement.is_some() && place_of(&probe, *u) == Place::Ground
        })
        .collect();
    let city = probe.city_type(PlayerId(1));
    let plain = |unit| Side { unit, tech: None };
    let mut strengths = Vec::new();
    for &strong in &attackers {
        // What stands below cannot fire back unless it reaches the orbit.
        let targets: Vec<DefId> = city
            .into_iter()
            .chain(ground.iter().copied())
            .filter(|t| !reaches_orbit(rules, *t))
            .collect();
        let mut beaten = Vec::new();
        for target in targets {
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
        let counters: Vec<DefId> = defences
            .iter()
            .chain(interceptors.iter())
            .copied()
            .collect();
        let answer = best_answer(rules, setup, age, strong, &counters, seeds)?;
        strengths.push(OrbitStrength {
            strong,
            beaten,
            answer,
        });
    }
    for &strong in &defences {
        let reach_up = orbit_reach(rules, strong, false);
        let mut beaten = Vec::new();
        for &ship in attackers.iter().chain(interceptors.iter()) {
            let reach_down = orbit_reach(rules, ship, true).max(orbit_reach(rules, ship, false));
            if reach_up <= reach_down + extent(rules, strong) || beaten.contains(&ship) {
                continue;
            }
            if pairing(rules, setup, age, plain(strong), plain(ship), seeds)?.first_wins() {
                beaten.push(ship);
                if !all {
                    break;
                }
            }
        }
        if beaten.is_empty() {
            continue;
        }
        let counters: Vec<DefId> = attackers
            .iter()
            .chain(interceptors.iter())
            .chain(ground.iter())
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let answer = best_answer(rules, setup, age, strong, &counters, seeds)?;
        strengths.push(OrbitStrength {
            strong,
            beaten,
            answer,
        });
    }
    let roster = attackers
        .iter()
        .chain(defences.iter())
        .chain(interceptors.iter())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(OrbitMatrix {
        age: age_id,
        roster,
        strengths,
    })
}

/// Researches a player of `age` has when its town stands for that age: every research whose
/// ages are reached, upgrades, tracks and building research alike (no age advance, project or
/// handicap).
fn town_research(rules: &Rules, age: i64) -> Vec<DefId> {
    rules
        .techs
        .values()
        .filter(|t| {
            matches!(
                t.kind,
                rom_rules::TechKind::Upgrade
                    | rom_rules::TechKind::Track
                    | rom_rules::TechKind::UnitUpgrade
            ) && rules
                .prerequisites
                .get(&t.id)
                .into_iter()
                .flatten()
                .filter_map(|p| rules.ages.get(&p.requires_def_id))
                .all(|a| a.ordinal <= age)
        })
        .map(|t| t.id)
        .collect()
}

/// The buildings of a town of `age`, the city first: the city type, then one of every other
/// building its builders construct in that age (the newest of each line), biggest first;
/// cities, gather sites and defences that reach the orbit left out.
pub fn town(world: &World, setup: &Setup, age: i64) -> Vec<DefId> {
    let rules = world.rules();
    let Some(city) = world.city_type(PlayerId(1)) else {
        return Vec::new();
    };
    let built: BTreeSet<DefId> = rules
        .production
        .iter()
        .filter(|(producer, _)| rules.entity_types[*producer].builder.is_some())
        .flat_map(|(_, rows)| rows.iter())
        .filter(|row| row.mode == rom_rules::ProductionMode::Construct)
        .map(|row| row.product_def_id)
        .collect();
    let mut others = newest(rules, setup, age, |b| {
        let et = &rules.entity_types[&b];
        built.contains(&b)
            && et.class == EntityClass::Building
            && et.territory.as_ref().is_none_or(|t| !t.is_city)
            && !reaches_orbit(rules, b)
            && b != city
    });
    others.sort_by_key(|b| {
        let et = &rules.entity_types[b];
        (std::cmp::Reverse(et.footprint_w * et.footprint_h), *b)
    });
    std::iter::once(city).chain(others).collect()
}

/// Lays a town out around the centre of face 0: the city in the middle, every other building on
/// the nearest free ground with `LANE` tiles between. Returns the buildings placed.
fn lay_out(world: &mut World, owner: PlayerId, buildings: &[DefId]) -> Vec<EntityId> {
    let mut taken: Vec<[i32; 4]> = Vec::new();
    let mut placed = Vec::new();
    let mid = N as i32 / 2;
    // Candidate corners nearest the centre first, in a fixed order.
    let mut corners: Vec<(i32, i32)> = (-30..=30)
        .flat_map(|dy| (-30..=30).map(move |dx| (dx, dy)))
        .collect();
    corners.sort_by_key(|(dx, dy)| (dx * dx + dy * dy, *dy, *dx));
    for &b in buildings {
        let et = &world.rules().entity_types[&b];
        let (w, h) = (et.footprint_w as i32, et.footprint_h as i32);
        for &(dx, dy) in &corners {
            // The footprint's top-left tile, centred on the offset.
            let (x0, y0) = (mid + dx - w / 2, mid + dy - h / 2);
            let rect = [x0, y0, x0 + w, y0 + h];
            if rect[0] < 1 || rect[1] < 1 || rect[2] >= N as i32 - 1 || rect[3] >= N as i32 - 1 {
                continue;
            }
            let clear = taken.iter().all(|t| {
                rect[0] >= t[2] + LANE
                    || t[0] >= rect[2] + LANE
                    || rect[1] >= t[3] + LANE
                    || t[1] >= rect[3] + LANE
            });
            if !clear {
                continue;
            }
            let pos = at(x0 * 1000 + w * 500, y0 * 1000 + h * 500);
            if let Ok(id) = world.place_for_setup(b, owner, pos) {
                taken.push(rect);
                placed.push(id);
                break;
            }
        }
    }
    placed
}

/// What one ship does to an undefended town of its age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Levelling {
    /// The age of the town and the ship.
    pub age: DefId,
    /// The ship (or craft).
    pub ship: DefId,
    /// Buildings in the town, the city among them.
    pub buildings: usize,
    /// How far the farthest building's footprint reaches from the city centre, milli-tiles.
    pub spread: i64,
    /// Volleys until every building was down; `None` when the town still stood at the limit.
    pub volleys: Option<u32>,
    /// Volleys until the city was down.
    pub city_volleys: Option<u32>,
    /// Units placed in the town and how many survived the bombardment.
    pub units: (usize, usize),
}

/// Plays one ship of `ship` against an undefended town of `age` on the planet: the ship aims
/// at the city first, then at the standing building nearest its ground point. Twenty of the
/// age's ground fighters stand in the town's streets, from the city's walls to the town's edge.
pub fn level_town(
    rules: &Arc<Rules>,
    setup: &Setup,
    age: i64,
    ship: DefId,
) -> Result<Levelling, String> {
    let research = town_research(rules, age);
    let mut world = planet_world(rules, setup, age, [Vec::new(), research], 1)?;
    let buildings = town(&world, setup, age);
    let placed = lay_out(&mut world, PlayerId(2), &buildings);
    let mid = N as i32 * 500;
    let city = *placed.first().ok_or("no city in the town")?;
    let spread = placed
        .iter()
        .filter_map(|id| {
            let e = world.entity(*id)?;
            let et = &world.rules().entity_types[&e.type_id];
            let dx = (e.pos.x - mid).abs() as i64 + et.footprint_w * 500;
            let dy = (e.pos.y - mid).abs() as i64 + et.footprint_h * 500;
            Some(rom_sim::integer_sqrt((dx * dx + dy * dy) as u128) as i64)
        })
        .max()
        .unwrap_or(0);
    // Soldiers in the streets: the age's ground fighters in turn, five on each of four rings
    // from beside the city's walls out to the edge of the town, holding fire where they stand.
    let soldiers: Vec<DefId> = roster(rules, setup, age)
        .into_iter()
        .filter(|u| {
            rules.entity_types[u].movement.is_some()
                && place_of(&world, *u) == Place::Ground
                && !reaches_orbit(rules, *u)
        })
        .collect();
    let hold = world
        .rules()
        .stances
        .values()
        .find(|s| s.behavior == "hold_fire")
        .map(|s| s.id);
    let mut units = Vec::new();
    for n in 0..SOLDIERS {
        let Some(&unit) = soldiers.get(n % soldiers.len().max(1)) else {
            break;
        };
        let ring = (4500 + (n / 5) as i64 * (spread - 4500).max(0) / 3) as f64;
        let angle = (n % 5) as f64 * std::f64::consts::TAU / 5.0 + (n / 5) as f64 * 0.6;
        let pos = at(
            mid + (ring * angle.cos()) as i32,
            mid + (ring * angle.sin()) as i32,
        );
        if let Ok(id) = world.spawn(unit, PlayerId(2), pos) {
            if let Some(stance) = hold {
                let _ = world.set_stance(PlayerId(2), id, stance);
            }
            units.push(id);
        }
    }
    let attacker = field(&mut world, ship, PlayerId(1), at(mid, mid))?;
    // A city knocked out is down even when its defenders restore it at once (task 67): the
    // ship moves on to the rest of the town.
    let mut knocked: BTreeSet<EntityId> = BTreeSet::new();
    let mut volleys = 0u32;
    let mut city_volleys = None;
    let mut last = None;
    while world.tick() < LEVEL_TICKS && world.entity(attacker).is_some() {
        knocked.extend(
            placed
                .iter()
                .copied()
                .filter(|id| world.entity(*id).is_none_or(|e| e.hp <= 0 || e.neutralised)),
        );
        let down = |id: &EntityId| knocked.contains(id);
        if placed.iter().all(down) {
            break;
        }
        let ready = world.orbit(attacker).is_some()
            && world.entity(attacker).is_some_and(|e| {
                idle(&world, e)
                    || e.orders()
                        .front()
                        .and_then(|o| o.target_entity)
                        .is_some_and(|t| down(&t))
            });
        if ready {
            let below = world.orbit(attacker).expect("in orbit").pos;
            let mark = if !down(&city) {
                Some(city)
            } else {
                placed
                    .iter()
                    .filter(|id| !down(id))
                    .filter_map(|id| Some((world.distance(below, world.entity(*id)?.pos)?, *id)))
                    .min()
                    .map(|(_, id)| id)
            };
            if let Some(mark) = mark {
                let _ = order(&mut world, attacker, "attack", Some(mark), None);
            }
        }
        world.step();
        let shot = world
            .orbit(attacker)
            .and_then(|o| o.last_shot)
            .map(|s| s.tick);
        if shot.is_some() && shot != last {
            last = shot;
            volleys += 1;
            if std::env::var("ROM_LEVEL_TRACE").is_ok() {
                let standing: Vec<String> = placed
                    .iter()
                    .filter_map(|id| world.entity(*id))
                    .filter(|e| e.hp > 0 && !e.neutralised)
                    .map(|e| format!("{}:{}", world.rules().key(e.type_id), e.hp))
                    .collect();
                eprintln!(
                    "t={} volley {volleys} target={:?} standing={}",
                    world.tick(),
                    world
                        .entity(attacker)
                        .and_then(|e| e.orders().front().and_then(|o| o.target_entity)),
                    standing.join(" ")
                );
            }
        }
        if city_volleys.is_none()
            && world
                .entity(city)
                .is_none_or(|e| e.hp <= 0 || e.neutralised || e.hp * 4 <= e.max_hp)
        {
            city_volleys = Some(volleys);
        }
    }
    knocked.extend(
        placed
            .iter()
            .copied()
            .filter(|id| world.entity(*id).is_none_or(|e| e.hp <= 0 || e.neutralised)),
    );
    let levelled = placed.iter().all(|id| knocked.contains(id));
    let survivors = units
        .iter()
        .filter(|id| world.entity(**id).is_some())
        .count();
    Ok(Levelling {
        age: rules
            .ages
            .values()
            .find(|a| a.ordinal == age)
            .ok_or("no such age")?
            .id,
        ship,
        buildings: placed.len(),
        spread,
        volleys: levelled.then_some(volleys),
        city_volleys,
        units: (units.len(), survivors),
    })
}

/// Every ship class with a bombardment weapon in every age from its first: the volleys one
/// of it takes to level an undefended town of that age.
pub fn levelling_table(rules: &Arc<Rules>, setup: &Setup) -> Result<Vec<Levelling>, String> {
    let Some(first) = first_orbit_age(rules, setup) else {
        return Ok(Vec::new());
    };
    let last = rules.ages.values().map(|a| a.ordinal).max().unwrap_or(0);
    let mut rows = Vec::new();
    for age in first..=last {
        let mut ships: Vec<DefId> = rules
            .entity_types
            .values()
            .filter(|et| {
                et.class == EntityClass::Unit
                    && bombards(rules, et.id)
                    && first_age(rules, et.id) <= age
                    && rules.production_by_product.contains_key(&et.id)
                    && rules.is_available(et.id, setup.faction)
            })
            .map(|et| et.id)
            .collect();
        ships.sort_by_key(|s| (first_age(rules, *s), *s));
        for ship in ships {
            rows.push(level_town(rules, setup, age, ship)?);
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight_matrix_cmd::{SEEDS, measure_orbit, setup};
    use rom_rules::build_rules_db;

    fn rules() -> Arc<Rules> {
        let db = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        Arc::new(Rules::load(&db).unwrap())
    }

    /// The owner's rule across the orbit: every ship that levels what stands below it is
    /// answered (by the ground battery), and every battery that holds ships off is answered too.
    #[test]
    fn every_orbit_strength_has_an_answer() {
        let rules = rules();
        let last = rules.ages.values().map(|a| a.ordinal).max().unwrap();
        let jobs = std::thread::available_parallelism().map_or(1, |n| n.get());
        let matrices = measure_orbit(&rules, (0, last), SEEDS, jobs, false).unwrap();
        assert!(!matrices.is_empty(), "no age has an orbital attacker");
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
            "orbit strengths without an answer: {unanswered:?}"
        );
        // Every ship that bombards is a strength, and a ground defence answers it.
        for m in &matrices {
            for s in &m.strengths {
                if bombards(&rules, s.strong) {
                    let (side, _) = s.answer.unwrap();
                    assert!(reaches_orbit(&rules, side.unit), "{}", rules.key(s.strong));
                }
            }
        }
    }

    /// The owner's target (2026-11-17): three to five volleys of one frigate or cruiser level a
    /// town of its age; the battleship needs fewer; some soldiers at the edge survive.
    #[test]
    fn a_frigate_or_cruiser_levels_a_town_in_three_to_five_volleys() {
        let rules = rules();
        let setup = setup(&rules).unwrap();
        let first_age_of = |key: &str| first_age(&rules, rules.id_of(key).unwrap());
        let mut volleys = BTreeMap::new();
        for key in [
            "unit.space_frigate",
            "unit.space_cruiser",
            "unit.battleship",
        ] {
            let ship = rules.id_of(key).unwrap();
            let row = level_town(&rules, &setup, first_age_of(key), ship).unwrap();
            assert!(
                row.buildings >= 20,
                "{key}: a town of {} buildings",
                row.buildings
            );
            assert!(row.units.1 > 0, "{key}: no soldier survived");
            assert!(
                row.units.1 * 2 < row.units.0,
                "{key}: most soldiers survived"
            );
            volleys.insert(key, row.volleys.expect("the town was levelled"));
        }
        for key in ["unit.space_frigate", "unit.space_cruiser"] {
            assert!((3..=5).contains(&volleys[key]), "{key}: {volleys:?}");
        }
        assert!(volleys["unit.battleship"] < volleys["unit.space_cruiser"]);
        // A town levelled the same way twice.
        let ship = rules.id_of("unit.space_frigate").unwrap();
        let age = first_age_of("unit.space_frigate");
        assert_eq!(
            level_town(&rules, &setup, age, ship).unwrap(),
            level_town(&rules, &setup, age, ship).unwrap()
        );
    }
}

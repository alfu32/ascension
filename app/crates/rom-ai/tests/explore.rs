//! Exploring other worlds (task 173) in a small universe: a computer player of the Starfaring
//! age keeps a scout ship looking over the planets of its system, nearest first, before the
//! systems along its routes; an enemy city it sees there is remembered across a save; and an
//! invasion of that world sets off only when the troops at home are enough for the defence seen.

mod common;

use rom_ai::{Brain, Job};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules};
use rom_sim::{
    BodyId, BodyInfo, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SystemInfo, UniverseSetup,
    World,
};
use std::{collections::BTreeMap, sync::Arc};

const ME: PlayerId = PlayerId(1);
const FOE: PlayerId = PlayerId(2);
const HOME: BodyId = BodyId(2);
/// The other habitable planets of the home system, nearest the home planet first.
const NEAR: BodyId = BodyId(5);
const OUTER: BodyId = BodyId(6);
/// The habitable planet of the second system.
const FAR: BodyId = BodyId(11);
const N: u32 = 48;

fn key(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap()
}

/// Every planet: plain ground.
fn planet(rules: &Rules) -> PlanetSurface {
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain: vec![key(rules, "ter.plain"); 6 * (N * N) as usize],
        nodes: vec![],
    }
}

/// Two systems on a route: the home system with the home planet, two more habitable planets and
/// an ore asteroid, and a second system with one planet. Player one has a city and a spaceport at
/// home, builders, every technology and `stock` of every resource; player two has nothing yet.
fn world(rules: &Arc<Rules>, stock: i64) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: key(rules, "nation.romans"),
            age: key(rules, "age.expansion"),
            resources: if p == 1 {
                rules.resources.keys().map(|r| (*r, stock)).collect()
            } else {
                BTreeMap::new()
            },
            techs: if p == 1 {
                rules.techs.keys().map(|k| (*k, 1)).collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let body = |n, system, kind, radius, angle| BodyInfo {
        id: BodyId(n),
        system,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: angle,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: (kind == BodyKind::Resource).then(|| key(rules, "node.ore_asteroid")),
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: [(1, 0, 0), (2, 100, 0)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
                z: 0,
                constellation: String::new(),
                cluster: None,
                id,
                x,
                y,
                name: format!("System {id}"),
            })
            .collect(),
        bodies: vec![
            body(1, 1, BodyKind::Star, 0, 0),
            body(2, 1, BodyKind::Habitable, 20, 0),
            body(3, 1, BodyKind::Resource, 30, 90_000),
            body(5, 1, BodyKind::Habitable, 30, 30_000),
            body(6, 1, BodyKind::Habitable, 45, 200_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        planets: [HOME, NEAR, OUTER, FAR]
            .iter()
            .map(|body| (*body, planet(rules)))
            .collect(),
    };
    let mut w = World::from_universe(
        Arc::clone(rules),
        setup,
        73,
        players,
        BTreeMap::from([(ME, HOME), (FOE, OUTER)]),
    )
    .unwrap();
    w.reveal_terrain(ME, HOME);
    let at = |x, y| Pos::on(HOME, 0, x, y);
    w.place_for_setup(key(rules, "bld.city"), ME, at(20_500, 32_500))
        .unwrap();
    w.place_for_setup(key(rules, "bld.spaceport"), ME, at(34_000, 32_000))
        .unwrap();
    let builder = rules
        .entity_types
        .values()
        .find(|et| et.builder.is_some() && et.movement.is_some())
        .unwrap()
        .id;
    for n in 0..5 {
        w.spawn(builder, ME, at(26_500 + n * 1200, 38_500)).unwrap();
    }
    w
}

fn brain(rules: &Arc<Rules>) -> Brain {
    Brain::new(Arc::clone(rules), ME, "balanced", "tougher", 73).unwrap()
}

/// The scout looks over both other planets of the home system, the nearer first, before the
/// planet of the other system; twice alike.
#[test]
fn a_scout_looks_over_every_planet_of_its_system() {
    let rules = common::rules();
    let play = || {
        let mut w = world(&rules, 1_000_000);
        let mut brain = brain(&rules);
        let done = |brain: &Brain| {
            let seen: BTreeMap<u32, u64> = brain.surveyed_worlds().collect();
            [NEAR, OUTER].iter().all(|b| seen.contains_key(&b.0))
        };
        while w.tick() < 60_000 && !done(&brain) {
            brain.step(&mut w);
            w.step();
        }
        let seen: BTreeMap<u32, u64> = brain.surveyed_worlds().collect();
        assert!(done(&brain), "surveyed {seen:?} by tick {}", w.tick());
        assert!(seen[&NEAR.0] < seen[&OUTER.0], "nearest first: {seen:?}");
        assert!(
            seen.get(&FAR.0).is_none_or(|far| *far > seen[&OUTER.0]),
            "the home system first: {seen:?}"
        );
        (w.tick(), w.hash())
    };
    assert_eq!(play(), play());
}

/// An enemy city on a planet of the home system is seen by the scout and remembered; a brain
/// restored from the save knows it too and gives the same orders as the one saved; twice alike.
#[test]
fn an_enemy_city_seen_by_the_scout_is_remembered_across_a_save() {
    let rules = common::rules();
    let play = || {
        let mut w = world(&rules, 1_000_000);
        let city = w
            .place_for_setup(
                key(&rules, "bld.city"),
                FOE,
                Pos::on(OUTER, 0, 24_500, 24_500),
            )
            .unwrap();
        let mut brain = brain(&rules);
        let knows = |brain: &Brain| {
            brain
                .known_enemies()
                .any(|(id, e)| id == city.0 && e.building && e.pos.body == OUTER)
        };
        while w.tick() < 60_000 && !knows(&brain) {
            brain.step(&mut w);
            w.step();
        }
        assert!(knows(&brain), "no sight of the city by tick {}", w.tick());
        let saved = brain.save();
        let mut restored = Brain::restore(Arc::clone(&rules), ME, &saved).unwrap();
        assert!(knows(&restored));
        assert_eq!(restored.save(), saved);
        for _ in 0..2000 {
            let tick = w.tick();
            let orders = brain.think(&w, tick);
            assert_eq!(restored.think(&w, tick), orders, "tick {tick}");
            for order in orders {
                match w.submit(order.clone()) {
                    Ok(_) => {
                        brain.accepted();
                        restored.accepted();
                    }
                    Err(error) => {
                        brain.rejected(&order, &error, tick);
                        restored.rejected(&order, &error, tick);
                    }
                }
            }
            w.step();
        }
        assert!(knows(&brain) && knows(&restored));
        w.hash()
    };
    assert_eq!(play(), play());
}

/// Rules in which every difficulty sees everything: the defence is known without a scout.
fn seeing() -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    Arc::new(rules)
}

/// Sixteen marines at home and a transport in orbit, no money; an enemy city on the nearest
/// planet with `defenders` marines beside it. Returns whether an invasion set off in the time
/// given, and the hash.
fn invade(rules: &Arc<Rules>, defenders: i32) -> (bool, u64) {
    let mut w = world(rules, 0);
    w.place_for_setup(
        key(rules, "bld.city"),
        FOE,
        Pos::on(NEAR, 0, 24_500, 24_500),
    )
    .unwrap();
    let marine = key(rules, "unit.marine");
    for n in 0..defenders {
        w.spawn(
            marine,
            FOE,
            Pos::on(NEAR, 0, 20_500 + (n % 8) * 1000, 30_500 + (n / 8) * 1000),
        )
        .unwrap();
    }
    for n in 0..16 {
        w.spawn(
            marine,
            ME,
            Pos::on(HOME, 0, 22_500 + (n % 8) * 1000, 26_500 + (n / 8) * 1000),
        )
        .unwrap();
    }
    w.launch_for_setup(
        key(rules, "unit.transport_ship"),
        ME,
        Pos::on(HOME, 0, 34_000, 34_000),
    )
    .unwrap();
    let mut brain = Brain::new(Arc::clone(rules), ME, "balanced", "tougher", 73).unwrap();
    let mut invaded = false;
    while w.tick() < 6000 && !invaded {
        brain.step(&mut w);
        w.step();
        invaded = brain.voyages().any(|(_, v)| v.job == Job::Invade);
    }
    (invaded, w.hash())
}

/// An invasion is planned only with enough force: against an undefended city sixteen marines
/// set off; against thirty-two defenders they stay home. Twice alike.
#[test]
fn an_invasion_is_planned_only_with_enough_force() {
    let rules = seeing();
    let open = invade(&rules, 0);
    assert!(open.0, "no invasion of an undefended world");
    let held = invade(&rules, 32);
    assert!(!held.0, "an invasion against a defence it cannot beat");
    assert_eq!(invade(&rules, 0), open);
    assert_eq!(invade(&rules, 32), held);
}

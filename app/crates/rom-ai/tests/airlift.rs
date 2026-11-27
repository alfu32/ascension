//! Buildings that lift off (task 170) on a hand-built field: a computer player in the Spacefaring
//! age with its barracks's lift-off engines has two cities far apart; an enemy army stands beside
//! the barracks of the western one. The barracks lifts off and lands beside the eastern city,
//! clear of the enemy. With the Starfaring age's research (task 170b) a barracks whose planet
//! has no safe city flies to the safe city on another planet of its star system. The same match
//! twice gives equal hashes.

mod common;

use rom_ai::Brain;
use rom_rules::BodyKind;
use rom_rules::{DefId, PlacementKind, ResourceKind, Rules};
use rom_sim::{
    BodyId, BodyInfo, MapGrid, MatchOptions, PlanetSurface, Player, PlayerId, Pos, SystemInfo,
    UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const ME: PlayerId = PlayerId(1);
const ENEMY: PlayerId = PlayerId(2);

fn seeing() -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    Arc::new(rules)
}

fn key(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap()
}

fn tile(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}

/// Player one's cities at (10, 24) and (84, 24) and its barracks beside the western one;
/// twelve enemy marines a dozen tiles east of the barracks.
fn field(rules: &Arc<Rules>, seed: u64) -> (World, Brain, rom_sim::EntityId) {
    let city = rules
        .entity_types
        .values()
        .find(|t| t.territory.as_ref().is_some_and(|t| t.is_city))
        .expect("city");
    let ground = city
        .placement_rules
        .iter()
        .find(|r| r.rule == PlacementKind::OnTerrain)
        .and_then(|r| r.ref_def_id)
        .expect("the city names its ground");
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 3000))
        .collect();
    let factions = common::factions(rules, 2);
    let age = key(rules, "age.starfaring");
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        faction: factions[(id as usize).saturating_sub(1).min(1)],
        age,
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: if id == 1 {
            BTreeMap::from([(key(rules, "tech.lift_engines.barracks"), 1)])
        } else {
            BTreeMap::new()
        },
    };
    let (w, h) = (96, 48);
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(w, h, vec![ground; (w * h) as usize]).unwrap(),
        seed,
        vec![player(0), player(1), player(2)],
    )
    .unwrap()
    .with_match_options(MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    })
    .unwrap();
    world.reveal_terrain(ME, BodyId(0));
    world.reveal_terrain(ENEMY, BodyId(0));
    world.place_for_setup(city.id, ME, tile(10, 24)).unwrap();
    world.place_for_setup(city.id, ME, tile(84, 24)).unwrap();
    let barracks = world
        .place_for_setup(key(rules, "bld.barracks"), ME, tile(20, 24))
        .unwrap();
    let marine = key(rules, "unit.marine");
    for k in 0..12 {
        world
            .spawn(marine, ENEMY, tile(32 + k % 4, 20 + k / 4 * 3))
            .unwrap();
    }
    let brain = Brain::new(Arc::clone(rules), ME, "balanced", "tough", seed).unwrap();
    (world, brain, barracks)
}

#[test]
fn a_besieged_barracks_flies_to_a_safe_city() {
    let rules = seeing();
    let run = || {
        let (mut world, mut brain, barracks) = field(&rules, 3);
        let mut lifted = None;
        let mut landed = None;
        for tick in 0..6_000_u32 {
            brain.step(&mut world);
            world.step();
            let Some(e) = world.entity(barracks) else {
                break;
            };
            if world.airborne(barracks) {
                lifted.get_or_insert(tick);
            } else if lifted.is_some() {
                landed = Some((tick, e.pos));
                break;
            }
        }
        assert!(lifted.is_some(), "the barracks never lifted off");
        let (_, at) = landed.expect("the barracks never landed");
        assert!(
            at.x > 60_000,
            "it landed beside the eastern city, clear of the enemy: {at:?}"
        );
        world.hash()
    };
    assert_eq!(run(), run());
}

/// Two planets of one star system, 2 and 3: player one's only city on planet 2 has its barracks
/// beside it and a dozen enemy marines at the door; its other city stands on planet 3.
fn system_field(rules: &Arc<Rules>, seed: u64) -> (World, Brain, rom_sim::EntityId) {
    let ground = key(rules, "ter.plain");
    let factions = common::factions(rules, 2);
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        faction: factions[(id as usize).saturating_sub(1).min(1)],
        age: key(rules, "age.expansion"),
        resources: rules
            .resources
            .values()
            .filter(|r| r.kind == ResourceKind::Base && id != 0)
            .map(|r| (r.id, 3000))
            .collect(),
        techs: if id == 1 {
            ["tech.lift_engines.barracks", "tech.age.expansion"]
                .into_iter()
                .map(|k| (key(rules, k), 1))
                .collect()
        } else {
            BTreeMap::new()
        },
    };
    let body = |n, kind, radius, angle| BodyInfo {
        id: BodyId(n),
        system: 1,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: angle,
        n: (kind == BodyKind::Habitable).then_some(48),
        home_race: None,
        node_type: None,
    };
    let planet = || PlanetSurface {
        surface: rom_geo::CubeSurface { n: 48 },
        terrain: vec![ground; 6 * 48 * 48],
        nodes: vec![],
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: vec![SystemInfo {
            id: 1,
            name: "System 1".into(),
            x: 0,
            y: 0,
            z: 0,
            constellation: String::new(),
            cluster: None,
        }],
        bodies: vec![
            body(1, BodyKind::Star, 0, 0),
            body(2, BodyKind::Habitable, 20, 0),
            body(3, BodyKind::Habitable, 40, 120_000),
        ],
        routes: Vec::new(),
        planets: [(BodyId(2), planet()), (BodyId(3), planet())].into(),
    };
    let mut world = World::from_universe(
        Arc::clone(rules),
        setup,
        seed,
        vec![player(0), player(1), player(2)],
        BTreeMap::from([(ME, BodyId(2)), (ENEMY, BodyId(2))]),
    )
    .unwrap()
    .with_match_options(MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    })
    .unwrap();
    for body in [BodyId(2), BodyId(3)] {
        world.reveal_terrain(ME, body);
    }
    let on = |body: u32, x: i32, y: i32| Pos::on(BodyId(body), 0, x * 1000 + 500, y * 1000 + 500);
    let city = key(rules, "bld.city");
    world.place_for_setup(city, ME, on(2, 10, 24)).unwrap();
    world.place_for_setup(city, ME, on(3, 24, 24)).unwrap();
    let barracks = world
        .place_for_setup(key(rules, "bld.barracks"), ME, on(2, 18, 24))
        .unwrap();
    let marine = key(rules, "unit.marine");
    for k in 0..12 {
        world
            .spawn(marine, ENEMY, on(2, 28 + k % 4, 20 + k / 4 * 3))
            .unwrap();
    }
    let brain = Brain::new(Arc::clone(rules), ME, "balanced", "tough", seed).unwrap();
    (world, brain, barracks)
}

#[test]
fn a_besieged_barracks_flies_to_a_safe_planet_of_its_system() {
    let rules = seeing();
    let run = || {
        let (mut world, mut brain, barracks) = system_field(&rules, 3);
        let mut landed = None;
        let mut lifted = false;
        for _ in 0..12_000_u32 {
            brain.step(&mut world);
            world.step();
            let Some(e) = world.entity(barracks) else {
                break;
            };
            if world.airborne(barracks) {
                lifted = true;
            } else if lifted {
                landed = Some(e.pos);
                break;
            }
        }
        assert!(lifted, "the barracks never lifted off");
        let at = landed.expect("the barracks never landed");
        assert_eq!(at.body, BodyId(3), "it landed on the safe planet: {at:?}");
        world.hash()
    };
    assert_eq!(run(), run());
}

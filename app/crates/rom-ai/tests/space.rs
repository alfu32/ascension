//! The space manager in a small universe: a player of the age of expansion with a city and
//! a spaceport claims the resource bodies of its system with its ships, and carries a
//! builder to the other planet of the system to found a colony there.

mod common;

use rom_ai::{Brain, Job};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules};
use rom_sim::{
    BodyId, BodyInfo, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SystemInfo, UniverseSetup,
    World,
};
use std::{collections::BTreeMap, sync::Arc};

const ME: PlayerId = PlayerId(1);
const HOME: BodyId = BodyId(2);
const NEIGHBOUR: BodyId = BodyId(5);
const N: u32 = 48;

/// Every planet: plain ground with a forest and a mountain block.
fn planet(rules: &Rules) -> PlanetSurface {
    let side = (N * N) as usize;
    let mut terrain = vec![rules.id_of("ter.plain").unwrap(); 6 * side];
    for y in 10..16 {
        for x in 6..12 {
            terrain[y * N as usize + x] = rules.id_of("ter.forest").unwrap();
            terrain[y * N as usize + x + 24] = rules.id_of("ter.mountain").unwrap();
        }
    }
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain,
        nodes: vec![],
    }
}

/// One system with the home planet, a second habitable planet and two ore asteroids, and a
/// second system on a route. Player one has a city, a spaceport, builders and plenty.
fn world(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.expansion").unwrap(),
            resources: if p == 1 {
                rules.resources.keys().map(|r| (*r, 1_000_000)).collect()
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
        node_type: (kind == BodyKind::Resource).then(|| rules.id_of("node.ore_asteroid").unwrap()),
    };
    let setup = UniverseSetup {
        systems: [(1, 0, 0), (2, 100, 0)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
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
            body(4, 1, BodyKind::Resource, 30, 270_000),
            body(5, 1, BodyKind::Habitable, 40, 180_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        // The host generates other planets when a ship first heads there; here they exist.
        planets: BTreeMap::from([(HOME, planet(rules)), (NEIGHBOUR, planet(rules))]),
    };
    let mut w = World::from_universe(
        Arc::clone(rules),
        setup,
        61,
        players,
        BTreeMap::from([(ME, HOME), (PlayerId(2), HOME)]),
    )
    .unwrap();
    w.reveal_terrain(ME, HOME);
    let at = |x, y| Pos {
        x,
        y,
        face: 0,
        body: HOME,
    };
    w.place_for_setup(rules.id_of("bld.city").unwrap(), ME, at(20_500, 32_500))
        .unwrap();
    w.place_for_setup(rules.id_of("bld.spaceport").unwrap(), ME, at(34_000, 32_000))
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

fn run(world: &mut World, brain: &mut Brain, ticks: u64) {
    for _ in 0..ticks {
        brain.step(world);
        world.step();
    }
}

#[test]
fn ships_claim_the_resource_bodies_and_a_transport_founds_a_colony() {
    let rules = common::rules();
    let mut w = world(&rules);
    let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
    let mut claimed = false;
    let mut colony = false;
    let mut carried = false;
    for _ in 0..80 {
        run(&mut w, &mut brain, 250);
        claimed |= w.claims().any(|(_, owner)| owner == ME);
        carried |= brain.voyages().any(|(_, v)| v.job == Job::Colonise && v.stage >= 1);
        colony = w
            .cities(ME)
            .iter()
            .any(|c| w.entity(*c).is_some_and(|e| e.pos.body == NEIGHBOUR));
        if claimed && colony {
            break;
        }
    }
    assert!(claimed, "no resource body was claimed by tick {}", w.tick());
    assert!(carried, "no transport set off by tick {}", w.tick());
    assert!(colony, "no colony stands on the second planet by tick {}", w.tick());
}

#[test]
fn the_space_age_brain_repeats() {
    let rules = common::rules();
    let play = || {
        let mut w = world(&rules);
        let mut brain = Brain::new(Arc::clone(&rules), ME, "builder", "tough", 61).unwrap();
        run(&mut w, &mut brain, 3000);
        (w.hash(), brain.save())
    };
    assert_eq!(play(), play());
}


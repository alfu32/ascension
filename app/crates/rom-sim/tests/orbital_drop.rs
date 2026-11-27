//! Orbital drop (task 169): infantry aboard a ship in orbit are dropped onto visible free land
//! of the planet below, within `space.drop_range` of the point under the ship; they fall for
//! `space.drop_ticks`, in reach of anti-air and planetary defences. Refused without the
//! research and from a ship that is not in orbit. Every scenario runs twice and compares state
//! hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, SimError,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const HOME: BodyId = BodyId(2);
const N: u32 = 64;

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

/// A plain planet.
fn planet(rules: &Rules) -> PlanetSurface {
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain: vec![rules.id_of("ter.plain").unwrap(); 6 * (N * N) as usize],
        nodes: vec![],
    }
}

/// One system with the home planet of both players, in the Starfaring age with every
/// technology but those in `without`; player one has a spaceport on face 5.
fn world(rules: &Arc<Rules>, without: &[&str]) -> World {
    let skip: Vec<DefId> = without.iter().map(|k| rules.id_of(k).unwrap()).collect();
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.expansion").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: if p == 0 {
                BTreeMap::new()
            } else {
                rules
                    .techs
                    .keys()
                    .filter(|k| !skip.contains(k))
                    .map(|k| (*k, 1))
                    .collect()
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
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: vec![SystemInfo {
            z: 0,
            constellation: String::new(),
            cluster: None,
            id: 1,
            x: 0,
            y: 0,
            name: "System 1".into(),
        }],
        bodies: vec![body(1, BodyKind::Star, 0), body(2, BodyKind::Habitable, 20)],
        routes: vec![],
        planets: BTreeMap::from([(HOME, planet(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        72,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap();
    w.place_for_setup(id(&w, "bld.spaceport"), P1, at(5, 32_000, 32_000))
        .unwrap();
    w
}

fn at(face: u8, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face,
        body: HOME,
    }
}

fn command(w: &World, executor: EntityId, kind: &str) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    }
}

fn drop_at(w: &mut World, ship: EntityId, to: Pos) -> Result<(), SimError> {
    let order = Command {
        target_pos: Some(to),
        ..command(w, ship, "drop")
    };
    w.submit(order).map(|_| ())
}

fn run_until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "not reached in {limit} ticks (tick {})", w.tick());
}

/// A transport lands on face 5, three marines and an armoured car board it (the car is no
/// infantry and stays aboard), it lifts off and holds above `above`. The drop is tried once
/// while it stands on the ground.
fn loaded_transport(w: &mut World, above: Pos) -> (EntityId, Vec<EntityId>, Result<(), SimError>) {
    let marker = w.body_position(HOME).unwrap();
    let ship = w
        .spawn(
            id(w, "unit.transport_ship"),
            P1,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    let pad = at(5, 54_500, 54_500);
    w.submit(Command {
        target_pos: Some(pad),
        ..command(w, ship, "orbit")
    })
    .unwrap();
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == HOME));
    w.submit(Command {
        target_pos: Some(pad),
        ..command(w, ship, "land")
    })
    .unwrap();
    run_until(w, 4000, |w| w.is_landed(ship));
    let mut riders: Vec<_> = (0..3)
        .map(|n| {
            w.spawn(id(w, "unit.marine"), P1, at(5, 44_500 + n * 1000, 54_500))
                .unwrap()
        })
        .collect();
    riders.push(
        w.spawn(id(w, "unit.armoured_car"), P1, at(5, 44_500, 52_500))
            .unwrap(),
    );
    for rider in &riders {
        w.submit(Command {
            target_entity: Some(ship),
            ..command(w, *rider, "enter")
        })
        .unwrap();
    }
    run_until(w, 3000, |w| w.passengers(ship).len() == riders.len());
    let landed = drop_at(w, ship, at(5, 50_500, 50_500));
    w.submit(command(w, ship, "lift")).unwrap();
    run_until(w, 400, |w| w.orbit(ship).is_some());
    w.submit(Command {
        target_pos: Some(above),
        ..command(w, ship, "move")
    })
    .unwrap();
    run_until(w, 8000, |w| w.orbit(ship).is_some_and(|o| o.pos == above));
    (ship, riders, landed)
}

#[test]
fn a_drop_needs_the_research_and_a_ship_in_orbit() {
    let rules = rules();
    let mut w = world(&rules, &["tech.orbital_drop"]);
    let (ship, _, landed) = loaded_transport(&mut w, at(5, 40_500, 40_500));
    assert_eq!(landed, Err(SimError::NotInOrbit), "landed, not in orbit");
    assert_eq!(
        drop_at(&mut w, ship, at(5, 34_500, 34_500)),
        Err(SimError::CannotDrop)
    );
    let mut w = world(&rules, &[]);
    let (ship, _, landed) = loaded_transport(&mut w, at(5, 40_500, 40_500));
    assert_eq!(landed, Err(SimError::NotInOrbit), "landed, not in orbit");
    // Land the player cannot see is no drop zone.
    assert_eq!(
        drop_at(&mut w, ship, at(2, 32_500, 32_500)),
        Err(SimError::DropZone)
    );
    assert_eq!(drop_at(&mut w, ship, at(5, 34_500, 34_500)), Ok(()));
}

/// The ship holds above face 5's far corner and is sent to drop near the spaceport, more than
/// the drop range away: it moves within its orbit, then drops the marines, which fall in reach
/// of player two's anti-air and planetary defences and land around the point, within range.
fn drop_scenario() -> (u64, Vec<u64>, u64, bool, bool, usize) {
    let rules = rules();
    let mut w = world(&rules, &[]);
    let flak = w
        .place_for_setup(id(&w, "unit.flak_gun"), P2, at(5, 22_500, 32_500))
        .unwrap();
    let battery = w
        .place_for_setup(id(&w, "bld.ground_battery"), P2, at(3, 20_500, 20_500))
        .unwrap();
    let (ship, riders, _) = loaded_transport(&mut w, at(5, 60_500, 60_500));
    let target = at(5, 30_500, 26_500);
    let range = w.rules().constant("space.drop_range").unwrap() as u64;
    let start = w.distance(w.orbit(ship).unwrap().pos, target);
    assert!(
        start.is_none_or(|d| d > range),
        "farther than the drop range: {start:?}"
    );
    drop_at(&mut w, ship, target).unwrap();
    let marines = &riders[..3];
    run_until(&mut w, 4000, |w| marines.iter().all(|m| w.leaping(*m)));
    let below = w.orbit(ship).unwrap().pos;
    let to_ship = w.distance(below, target).unwrap();
    assert!(to_ship <= range, "it moved within range: {to_ship}");
    // Falling, they are in reach of anti-air and of planetary defences.
    let aloft = marines
        .iter()
        .all(|m| w.can_target(flak, *m) && w.can_target(battery, *m));
    run_until(&mut w, 200, |w| marines.iter().all(|m| !w.leaping(*m)));
    let down = marines
        .iter()
        .any(|m| w.entity(*m).is_some_and(|_| w.can_target(flak, *m)));
    let mut spread = Vec::new();
    for marine in marines {
        let Some(e) = w.entity(*marine) else {
            continue;
        };
        assert!(e.container_id.is_none());
        let d = w.distance(below, e.pos).unwrap();
        assert!(d <= range, "landed within range of the ship: {d}");
        spread.push(w.distance(target, e.pos).unwrap());
    }
    // The armoured car is no infantry: it stays aboard.
    let aboard = w.passengers(ship).len();
    (w.hash(), spread, to_ship, aloft, down, aboard)
}

#[test]
fn dropped_infantry_fall_in_reach_of_anti_air_and_land_within_range() {
    let (first, spread, _, aloft, down, aboard) = drop_scenario();
    assert!(
        aloft,
        "falling troops are in reach of anti-air and batteries"
    );
    assert!(!down, "on the ground anti-air no longer reaches them");
    assert_eq!(aboard, 1, "the vehicle stays aboard");
    assert!(
        spread.iter().all(|d| *d <= 6_000),
        "around the point: {spread:?}"
    );
    let (second, ..) = drop_scenario();
    assert_eq!(first, second);
}

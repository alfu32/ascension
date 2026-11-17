//! Ships in orbit (task 68): a ship holds over one ground point, moves and patrols within its
//! orbit, lands straight down, takes troops aboard only on the ground; a freighter trades
//! between cities of two planets; a satellite sees most of the horizon and never leaves its
//! orbit. Every scenario runs twice and compares state hashes; several also go through a
//! snapshot and on from there.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, SimError,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const HOME: BodyId = BodyId(2);
const NEIGHBOUR: BodyId = BodyId(5);
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

/// Face 0 of every planet is plain; face 1 is deep sea, face 2 forest; the rest plain.
fn planet(rules: &Rules) -> PlanetSurface {
    let side = (N * N) as usize;
    let mut terrain = vec![rules.id_of("ter.plain").unwrap(); 6 * side];
    terrain[side..2 * side].fill(rules.id_of("ter.deep").unwrap());
    terrain[2 * side..3 * side].fill(rules.id_of("ter.forest").unwrap());
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain,
        nodes: vec![],
    }
}

/// One system with two habitable planets, both generated; player one has a city and a
/// spaceport on the home planet and a city on the neighbour.
fn world(rules: &Arc<Rules>) -> World {
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
                rules.techs.keys().map(|k| (*k, 1)).collect()
            },
        })
        .collect();
    let body = |n, kind, radius, angle| BodyInfo {
        id: BodyId(n),
        system: 1,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: angle,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: None,
    };
    let setup = UniverseSetup {
        figure: Vec::new(),
        systems: vec![SystemInfo {
            z: 0,
            constellation: String::new(),
            id: 1,
            x: 0,
            y: 0,
            name: "System 1".into(),
        }],
        bodies: vec![
            body(1, BodyKind::Star, 0, 0),
            body(2, BodyKind::Habitable, 20, 0),
            body(5, BodyKind::Habitable, 40, 90_000),
        ],
        routes: vec![],
        planets: BTreeMap::from([(HOME, planet(rules)), (NEIGHBOUR, planet(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        68,
        players,
        BTreeMap::from([(P1, HOME), (PlayerId(2), HOME)]),
    )
    .unwrap();
    let city = id(&w, "bld.city");
    w.place_for_setup(city, P1, at(HOME, 0, 20_500, 32_500))
        .unwrap();
    w.place_for_setup(id(&w, "bld.spaceport"), P1, at(HOME, 0, 36_000, 32_000))
        .unwrap();
    w.place_for_setup(city, P1, at(NEIGHBOUR, 0, 30_500, 30_500))
        .unwrap();
    w
}

fn at(body: BodyId, face: u8, x: i32, y: i32) -> Pos {
    Pos { x, y, face, body }
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
    }
}

fn submit(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    pos: Option<Pos>,
) -> Result<rom_sim::OrderId, SimError> {
    w.submit(Command {
        target_pos: pos,
        ..command(w, executor, kind)
    })
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

fn run(w: &mut World, ticks: u64) {
    for _ in 0..ticks {
        w.step();
    }
}

/// A ship of `key` in the orbit of `body` above `above`.
fn in_orbit(w: &mut World, key: &str, body: BodyId, above: Pos) -> EntityId {
    let marker = w.body_position(body).unwrap();
    let ship = w
        .spawn(
            id(w, key),
            P1,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    w.submit(Command {
        target_pos: Some(above),
        ..command(w, ship, "orbit")
    })
    .unwrap();
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == body));
    ship
}

/// A satellite trained at the spaceport and launched into orbit above it.
fn launch_satellite(w: &mut World) -> EntityId {
    let kind = id(w, "unit.satellite");
    let port = w
        .entities()
        .find(|e| e.type_id == id(w, "bld.spaceport"))
        .unwrap()
        .id;
    w.submit(Command {
        subject: Some(kind),
        ..command(w, port, "train")
    })
    .unwrap();
    run_until(w, 2000, |w| {
        w.entities()
            .any(|e| e.type_id == kind && w.orbit(e.id).is_some())
    });
    w.entities().find(|e| e.type_id == kind).unwrap().id
}

/// The state hash, also after a snapshot and a restore, and the hash of both run on.
fn checked(w: &World, rules: &Arc<Rules>, more: u64) -> u64 {
    let mut copy = World::restore(Arc::clone(rules), w.snapshot()).unwrap();
    assert_eq!(copy.hash(), w.hash());
    let mut on = World::restore(Arc::clone(rules), w.snapshot()).unwrap();
    run(&mut copy, more);
    run(&mut on, more);
    assert_eq!(copy.hash(), on.hash());
    w.hash()
}

#[test]
fn a_ship_in_orbit_holds_over_one_point() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let above = at(HOME, 0, 40_500, 20_500);
        let ship = in_orbit(&mut w, "unit.corvette", HOME, above);
        run(&mut w, 2000);
        assert_eq!(w.orbit(ship).unwrap().pos, above);
        checked(&w, &rules, 50)
    };
    assert_eq!(play(), play());
}

#[test]
fn a_ship_moves_between_orbit_points_without_descending() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let ship = in_orbit(&mut w, "unit.corvette", HOME, at(HOME, 0, 32_500, 32_500));
        // Near: straight there.
        let near = at(HOME, 0, 50_500, 40_500);
        submit(&mut w, ship, "move", Some(near)).unwrap();
        run_until(&mut w, 2000, |w| {
            w.orbit(ship).is_some_and(|o| o.pos == near)
        });
        assert!(w.entity(ship).unwrap().orders().is_empty());
        // Far: the other side of the planet, along a great circle, in orbit all the way.
        let far = at(HOME, 2, 20_500, 30_500);
        submit(&mut w, ship, "move", Some(far)).unwrap();
        let mut faces = std::collections::BTreeSet::new();
        for _ in 0..8000 {
            w.step();
            let orbit = w.orbit(ship).expect("still in orbit");
            assert!(!w.is_landed(ship));
            faces.insert(orbit.pos.face);
            if orbit.pos == far {
                break;
            }
        }
        assert_eq!(w.orbit(ship).unwrap().pos, far);
        assert!(faces.len() >= 3, "faces crossed: {faces:?}");
        checked(&w, &rules, 50)
    };
    assert_eq!(play(), play());
}

#[test]
fn an_orbit_patrol_circles_the_planet_on_a_great_circle() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let start = at(HOME, 0, 32_500, 32_500);
        let ship = in_orbit(&mut w, "unit.corvette", HOME, start);
        // A point straight east on the next face: the great circle is the equator of the net.
        let point = at(HOME, 1, 32_500, 32_500);
        submit(&mut w, ship, "patrol", Some(point)).unwrap();
        let mut seen = std::collections::BTreeSet::new();
        let mut back = false;
        let mut left = false;
        for _ in 0..4000 {
            w.step();
            let pos = w.orbit(ship).expect("in orbit").pos;
            seen.insert(pos.face);
            // Never far from the circle: the middle row of faces 0-3.
            assert!(pos.face < 4 && (24_000..41_000).contains(&pos.y), "{pos:?}");
            let d = w.distance(pos, start).unwrap_or(u64::MAX);
            left |= d > 40_000;
            if left && d < 2_000 {
                back = true;
                break;
            }
        }
        assert_eq!(seen.len(), 4, "faces {seen:?}");
        assert!(back, "the patrol did not come round");
        assert_eq!(w.entity(ship).unwrap().orders()[0].kind, "patrol");
        checked(&w, &rules, 50)
    };
    assert_eq!(play(), play());
}

#[test]
fn a_ship_lands_straight_down_and_is_refused_over_water() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        // Over the sea of face 1 the vertical landing is refused, with a reason.
        let sea = at(HOME, 1, 30_500, 30_500);
        let ship = in_orbit(&mut w, "unit.transport_ship", HOME, sea);
        assert_eq!(
            w.landing_problem(ship, sea),
            Some("too little ground below: the ship would stand in water")
        );
        assert_eq!(
            submit(&mut w, ship, "land", Some(sea)),
            Err(SimError::CannotLand)
        );
        // Over open ground it comes straight down onto the point below it.
        let ground = at(HOME, 0, 50_500, 50_500);
        submit(&mut w, ship, "move", Some(ground)).unwrap();
        run_until(&mut w, 4000, |w| {
            w.orbit(ship).is_some_and(|o| o.pos == ground)
        });
        assert_eq!(w.landing_problem(ship, ground), None);
        submit(&mut w, ship, "land", Some(ground)).unwrap();
        w.step();
        w.step();
        assert_eq!(w.orbit(ship).unwrap().pos, ground);
        assert!(w.altitude_pct(ship).unwrap() < 100);
        run_until(&mut w, 200, |w| w.is_landed(ship));
        assert_eq!(w.entity(ship).unwrap().pos, ground);
        // A satellite cannot land at all.
        let satellite = launch_satellite(&mut w);
        let below = w.orbit(satellite).unwrap().pos;
        assert_eq!(
            w.landing_problem(satellite, below),
            Some("this ship cannot land")
        );
        checked(&w, &rules, 50)
    };
    assert_eq!(play(), play());
}

#[test]
fn troops_board_only_a_landed_ship() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let ground = at(HOME, 0, 30_500, 40_500);
        let ship = in_orbit(&mut w, "unit.transport_ship", HOME, ground);
        let marine = w
            .spawn(id(&w, "unit.marine"), P1, at(HOME, 0, 34_500, 44_500))
            .unwrap();
        // In orbit, even above the own spaceport: refused, with the reason.
        assert_eq!(
            w.submit(Command {
                target_entity: Some(ship),
                ..command(&w, marine, "enter")
            }),
            Err(SimError::ShipNotLanded)
        );
        submit(&mut w, ship, "land", Some(ground)).unwrap();
        run_until(&mut w, 400, |w| w.is_landed(ship));
        w.submit(Command {
            target_entity: Some(ship),
            ..command(&w, marine, "enter")
        })
        .unwrap();
        run_until(&mut w, 400, |w| w.passengers(ship) == [marine]);
        submit(&mut w, ship, "lift", None).unwrap();
        run_until(&mut w, 400, |w| w.orbit(ship).is_some());
        // Nor do they leave it in orbit.
        assert_eq!(
            w.submit(Command {
                target_entity: Some(ship),
                ..command(&w, ship, "unload")
            }),
            Err(SimError::ShipNotLanded)
        );
        checked(&w, &rules, 50)
    };
    assert_eq!(play(), play());
}

#[test]
fn a_freighter_runs_a_route_between_two_planets_and_earns() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let wealth = id(&w, "res.wealth");
        let freighter = in_orbit(
            &mut w,
            "unit.space_freighter",
            HOME,
            at(HOME, 0, 20_500, 32_500),
        );
        let colony = *w
            .cities(P1)
            .iter()
            .find(|c| w.entity(**c).unwrap().pos.body == NEIGHBOUR)
            .unwrap();
        w.submit(Command {
            target_entity: Some(colony),
            ..command(&w, freighter, "trade")
        })
        .unwrap();
        w.step();
        let route = w.trade_routes(P1).next().unwrap();
        assert!(route.length > 20_000, "{}", route.length);
        let balance = |w: &World| w.players().find(|p| p.id == P1).unwrap().resources[&wealth];
        let start = balance(&w);
        // Down at home, up, across, down at the colony: the first paid arrival.
        let mut landed_on = Vec::new();
        for _ in 0..20_000 {
            w.step();
            let e = w.entity(freighter).unwrap();
            if w.is_landed(freighter) && landed_on.last() != Some(&e.pos.body) {
                landed_on.push(e.pos.body);
            }
            if w.stats().trade_trips >= 2 {
                break;
            }
        }
        assert_eq!(w.stats().trade_trips, 2, "landed on {landed_on:?}");
        assert_eq!(landed_on[..3], [HOME, NEIGHBOUR, HOME]);
        let paid = balance(&w) - start;
        assert!(paid >= 2 * route.per_trip_income, "{paid}");
        let mid = checked(&w, &rules, 300);
        (mid, w.hash())
    };
    assert_eq!(play(), play());
}

#[test]
fn a_satellite_sees_far_and_stays_in_its_orbit() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let tiles = |w: &World| {
            w.visibility_tiles(P1, HOME)
                .iter()
                .filter(|t| **t == 2)
                .count()
        };
        let before = tiles(&w);
        let satellite = launch_satellite(&mut w);
        let above = w.orbit(satellite).unwrap().pos;
        // 96 tiles, and Optics' 15 % on top (every technology is researched here).
        assert_eq!(w.entity(satellite).unwrap().los, 110_400);
        // Thirty tiles out, beyond every ground unit's and building's sight (half a face of
        // this small planet is as far as the sight reaches).
        assert!(w.is_visible_on_body(P1, HOME, 1, 2, 32));
        assert!(w.is_visible_on_body(P1, HOME, 0, 36, 2));
        assert!(tiles(&w) > before + 700, "{before} then {}", tiles(&w));
        let elsewhere = w.body_position(NEIGHBOUR).unwrap();
        for kind in ["orbit", "travel"] {
            assert_eq!(
                submit(&mut w, satellite, kind, Some(elsewhere)),
                Err(SimError::BoundToOrbit)
            );
        }
        assert_eq!(
            submit(&mut w, satellite, "land", Some(above)),
            Err(SimError::BoundToOrbit)
        );
        // It moves within its orbit.
        let next = at(HOME, 0, 16_500, 32_500);
        submit(&mut w, satellite, "move", Some(next)).unwrap();
        run_until(&mut w, 1000, |w| {
            w.orbit(satellite).is_some_and(|o| o.pos == next)
        });
        checked(&w, &rules, 50)
    };
    assert_eq!(play(), play());
}

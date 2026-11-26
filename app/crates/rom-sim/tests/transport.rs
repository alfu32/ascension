//! Carrying units in ships (task 54): boarding at a spaceport or beside a landed ship, travel,
//! landing on open ground of another planet, unloading, founding a city there and fighting
//! there. Every scenario runs twice and compares state hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SimError,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const HOME: BodyId = BodyId(2);
const NEIGHBOUR: BodyId = BodyId(5);
const FAR: BodyId = BodyId(11);
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

/// Face 0 of every planet is plain; face 1 is deep sea, face 2 forest.
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

/// Two systems on one route: the home planet and a neighbour in the first, a far planet in the
/// second. Only the home planet is generated at the start, as in a match.
fn world(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.expansion").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            // Technologies raise the city limit above the starting city.
            techs: if p == 0 {
                BTreeMap::new()
            } else {
                rules.techs.keys().map(|k| (*k, 1)).collect()
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
        node_type: None,
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
            body(5, 1, BodyKind::Habitable, 40, 90_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        planets: BTreeMap::from([(HOME, planet(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        54,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap();
    let city = id(&w, "bld.city");
    w.place_for_setup(city, P1, at(HOME, 20_500, 32_500))
        .unwrap();
    w.place_for_setup(id(&w, "bld.spaceport"), P1, at(HOME, 36_000, 32_000))
        .unwrap();
    w
}

fn at(body: BodyId, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        body,
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

fn order(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    entity: Option<EntityId>,
    pos: Option<Pos>,
) {
    w.submit(Command {
        target_entity: entity,
        target_pos: pos,
        ..command(w, executor, kind)
    })
    .unwrap_or_else(|e| panic!("{kind}: {e}"));
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

/// A transport in orbit of `body`, from system space beside its marker.
fn transport_in_orbit(w: &mut World, body: BodyId) -> EntityId {
    let marker = w.body_position(body).unwrap();
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
    orbit(w, ship, body);
    ship
}

fn orbit(w: &mut World, ship: EntityId, body: BodyId) {
    let marker = w.body_marker(body).unwrap();
    order(w, ship, "orbit", Some(marker), None);
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == body));
}

/// Ground units of player one beside the spaceport.
fn passengers(w: &mut World, citizens: u32, marines: u32) -> Vec<EntityId> {
    let mut out = Vec::new();
    for (key, count) in [("unit.citizen", citizens), ("unit.marine", marines)] {
        for n in 0..count {
            let pos = at(
                HOME,
                30_500 + n as i32 * 1000,
                38_500 + out.len() as i32 * 300,
            );
            out.push(w.spawn(id(w, key), P1, pos).unwrap());
        }
    }
    out
}

/// Units board only a landed ship (task 68): a ship in orbit comes down beside them first and
/// lifts off again once they are aboard.
fn board(w: &mut World, units: &[EntityId], ship: EntityId) {
    let in_orbit = w.orbit(ship).is_some();
    if in_orbit {
        let near = w.entity(units[0]).unwrap().pos;
        land(w, ship, near);
    }
    for unit in units {
        order(w, *unit, "enter", Some(ship), None);
    }
    let count = units.len();
    run_until(w, 2000, |w| w.passengers(ship).len() == count);
    if in_orbit {
        order(w, ship, "lift", None, None);
        run_until(w, 200, |w| w.orbit(ship).is_some());
    }
}

/// Lands on open ground near `near`: the ship first moves within its orbit above the site.
fn land(w: &mut World, ship: EntityId, near: Pos) -> Pos {
    let site = w.landing_site(ship, near).expect("open ground");
    order(w, ship, "land", None, Some(site));
    run_until(w, 4000, |w| w.is_landed(ship));
    site
}

fn add_neighbour(w: &mut World, rules: &Rules) {
    w.add_surface(NEIGHBOUR, &planet(rules)).unwrap();
}

#[test]
fn units_board_at_the_spaceport_land_on_another_planet_and_found_a_city() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let ship = transport_in_orbit(&mut w, HOME);
        let units = passengers(&mut w, 3, 2);
        board(&mut w, &units, ship);
        let boarded = w.hash();
        for unit in &units {
            let e = w.entity(*unit).unwrap();
            assert_eq!(e.container_id, Some(ship));
            assert!(w.entities_in_radius(e.pos, 0).iter().all(|id| id != unit));
        }
        assert_eq!(w.carried_load(ship), 5);
        assert!(w.visible_entities(P2).iter().all(|s| match s {
            rom_sim::Sighting::Live(e) => !units.contains(&e.id),
            rom_sim::Sighting::Remembered(_) => true,
        }));
        // Aboard a ship in space a unit takes no orders.
        assert_eq!(
            w.submit(Command {
                target_pos: Some(at(HOME, 30_000, 30_000)),
                ..command(&w, units[0], "move")
            }),
            Err(SimError::Aboard)
        );
        // The first landing on a planet asks for it to be generated (the app expands it).
        let marker = w.body_marker(NEIGHBOUR).unwrap();
        order(&mut w, ship, "orbit", Some(marker), None);
        run_until(&mut w, 4000, |w| {
            w.entity(ship).is_some_and(|e| {
                w.distance(e.pos, w.body_position(NEIGHBOUR).unwrap())
                    .unwrap()
                    < 6000
            })
        });
        assert!(
            w.orbit(ship).is_none(),
            "no surface yet: the ship holds beside the marker"
        );
        add_neighbour(&mut w, &rules);
        run_until(&mut w, 40, |w| {
            w.orbit(ship).is_some_and(|o| o.body == NEIGHBOUR)
        });
        let travelled = w.hash();
        // Landing: a descent of `space.land_ticks`, drawn by the altitude.
        let site = w.landing_site(ship, at(NEIGHBOUR, 40_500, 40_500)).unwrap();
        // Within its orbit above the site, then straight down.
        order(&mut w, ship, "move", None, Some(site));
        run_until(&mut w, 2000, |w| {
            w.orbit(ship).is_some_and(|o| o.pos == site)
        });
        order(&mut w, ship, "land", None, Some(site));
        w.step();
        w.step();
        let pct = w.altitude_pct(ship).unwrap();
        assert!((90..100).contains(&pct), "{pct}");
        assert_eq!(w.orbit(ship).unwrap().pos, site);
        run_until(&mut w, 200, |w| w.is_landed(ship));
        assert_eq!(w.entity(ship).unwrap().pos, site);
        assert_eq!(w.altitude_pct(ship), None);
        assert!(w.passengers(ship).is_empty());
        let landed = w.hash();
        // They come out around the hull (task 72), not under it.
        let hull = w.landing_radius(ship) as u64;
        assert!(hull > 0);
        for unit in &units {
            let e = w.entity(*unit).unwrap();
            assert_eq!(e.container_id, None);
            assert_eq!(e.pos.body, NEIGHBOUR);
            let d = w.distance(e.pos, site).unwrap();
            assert!(d > hull && d <= hull + 7_500, "{d}");
        }
        // Every passenger has its own tile.
        let mut tiles: Vec<_> = units
            .iter()
            .map(|u| {
                let p = w.entity(*u).unwrap().pos;
                (p.face, p.x / 1000, p.y / 1000)
            })
            .collect();
        tiles.sort();
        tiles.dedup();
        assert_eq!(tiles.len(), units.len());
        // The citizens found the first city of the planet, outside any border.
        let city = id(&w, "bld.city");
        let place = w.find_city_site(P1, NEIGHBOUR, site).unwrap();
        w.submit(Command {
            subject: Some(city),
            target_pos: Some(place),
            ..command(&w, units[0], "construct")
        })
        .unwrap();
        run_until(&mut w, 400, |w| {
            w.cities(P1)
                .iter()
                .any(|c| w.entity(*c).unwrap().pos.body == NEIGHBOUR)
        });
        let site = *w
            .cities(P1)
            .iter()
            .find(|c| w.entity(**c).unwrap().pos.body == NEIGHBOUR)
            .unwrap();
        for helper in &units[1..3] {
            order(&mut w, *helper, "repair", Some(site), None);
        }
        run_until(&mut w, 30_000, |w| {
            w.cities(P1).iter().any(|c| {
                w.entity(*c)
                    .is_some_and(|e| e.pos.body == NEIGHBOUR && e.build_progress.is_none())
            })
        });
        let colony = *w
            .cities(P1)
            .iter()
            .find(|c| w.entity(**c).unwrap().pos.body == NEIGHBOUR)
            .unwrap();
        // Its planet is the player's territory now.
        let c = w.entity(colony).unwrap().pos;
        assert_eq!(
            w.territory_owner_on_body(NEIGHBOUR, c.face, c.x / 1000, c.y / 1000),
            Some(P1)
        );
        (boarded, travelled, landed, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
}

#[test]
fn landed_troops_fight_and_take_a_city_on_another_planet() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        add_neighbour(&mut w, &rules);
        // A rival colony on the neighbour, with a defender.
        let city = id(&w, "bld.city");
        let rival = w
            .place_for_setup(city, P2, at(NEIGHBOUR, 20_500, 32_500))
            .unwrap();
        let defender = w
            .spawn(id(&w, "unit.marine"), P2, at(NEIGHBOUR, 26_500, 32_500))
            .unwrap();
        let ship = transport_in_orbit(&mut w, HOME);
        let troops = passengers(&mut w, 0, 6);
        board(&mut w, &troops, ship);
        orbit(&mut w, ship, NEIGHBOUR);
        // Inside the rival city's radius the ground is closed to a landing.
        assert_eq!(
            w.check_landing(ship, at(NEIGHBOUR, 24_500, 32_500)),
            Err(SimError::CannotLand)
        );
        // So is open water; forest is not (task 72: the ship stands on the trees).
        let water = Pos {
            face: 1,
            ..at(NEIGHBOUR, 32_500, 32_500)
        };
        assert_eq!(w.check_landing(ship, water), Err(SimError::CannotLand));
        let forest = Pos { face: 2, ..water };
        assert!(w.check_landing(ship, forest).is_ok());
        let site = land(&mut w, ship, at(NEIGHBOUR, 45_500, 32_500));
        assert!(
            w.distance(site, at(NEIGHBOUR, 20_500, 32_500))
                .is_none_or(|d| d > 16_000)
        );
        let landed = w.hash();
        for unit in &troops {
            order(
                &mut w,
                *unit,
                "attack_move",
                None,
                Some(at(NEIGHBOUR, 26_500, 32_500)),
            );
        }
        run_until(&mut w, 4000, |w| w.entity(defender).is_none());
        let fought = w.hash();
        assert!(w.stats().kills.get(&P1).copied().unwrap_or(0) >= 1);
        let alive: Vec<_> = troops
            .iter()
            .copied()
            .filter(|u| w.entity(*u).is_some())
            .collect();
        for unit in alive {
            let target = w.entity(rival).unwrap().pos;
            order(&mut w, unit, "attack_move", None, Some(target));
        }
        // Capturing works as on the home planet: the city falls to zero and changes hands.
        run_until(&mut w, 20_000, |w| {
            w.entity(rival).is_some_and(|c| c.owner == P1)
        });
        (landed, fought, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
}

#[test]
fn passengers_cross_a_route_to_another_system() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let ship = transport_in_orbit(&mut w, HOME);
        let units = passengers(&mut w, 2, 0);
        board(&mut w, &units, ship);
        w.submit(Command {
            target_pos: Some(Pos {
                body: w.space_of(2).unwrap(),
                ..Pos::default()
            }),
            ..command(&w, ship, "travel")
        })
        .unwrap();
        run_until(&mut w, 20_000, |w| w.transits().next().is_some());
        // Between systems the ship is off every surface; its passengers ride with it.
        assert_eq!(w.passengers(ship), units);
        let population = w.population_used(P1);
        run_until(&mut w, 20_000, |w| w.entity(ship).is_some());
        assert_eq!(w.population_used(P1), population);
        w.add_surface(FAR, &planet(&rules)).unwrap();
        orbit(&mut w, ship, FAR);
        let site = land(&mut w, ship, at(FAR, 32_500, 32_500));
        for unit in &units {
            assert_eq!(w.entity(*unit).unwrap().pos.body, FAR);
        }
        assert_eq!(site.body, FAR);
        w.hash()
    };
    assert_eq!(play(), play());
}

#[test]
fn a_landed_ship_takes_units_aboard_lifts_off_and_sets_them_down_only_on_the_ground() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        add_neighbour(&mut w, &rules);
        let ship = transport_in_orbit(&mut w, NEIGHBOUR);
        land(&mut w, ship, at(NEIGHBOUR, 32_500, 32_500));
        // Lifting off needs a landed ship; landing needs one in orbit.
        let other = transport_in_orbit(&mut w, HOME);
        assert_eq!(
            w.submit(command(&w, other, "lift")),
            Err(SimError::NotLanded)
        );
        assert_eq!(
            w.submit(command(&w, other, "unload")),
            Err(SimError::WrongTargetShape)
        );
        assert_eq!(
            w.submit(Command {
                target_entity: Some(other),
                ..command(&w, other, "unload")
            }),
            Err(SimError::CannotUnload)
        );
        let walkers: Vec<_> = (0..2)
            .map(|n| {
                w.spawn(
                    id(&w, "unit.citizen"),
                    P1,
                    at(NEIGHBOUR, 36_500 + n * 1000, 36_500),
                )
                .unwrap()
            })
            .collect();
        board(&mut w, &walkers, ship);
        // A passenger of a landed ship steps out when it is given an order.
        order(
            &mut w,
            walkers[1],
            "move",
            None,
            Some(at(NEIGHBOUR, 40_500, 40_500)),
        );
        w.step();
        assert_eq!(w.entity(walkers[1]).unwrap().container_id, None);
        assert_eq!(w.passengers(ship), vec![walkers[0]]);
        order(&mut w, ship, "lift", None, None);
        w.step();
        assert!(w.altitude_pct(ship).is_some_and(|p| p < 100));
        run_until(&mut w, 100, |w| w.orbit(ship).is_some());
        assert!(!w.is_landed(ship));
        orbit(&mut w, ship, HOME);
        let port = w
            .entities()
            .find(|e| e.type_id == id(&w, "bld.spaceport"))
            .unwrap()
            .id;
        // In orbit nobody gets off, not even at an own spaceport below (task 68).
        assert_eq!(
            w.submit(Command {
                target_pos: Some(at(HOME, 30_000, 30_000)),
                ..command(&w, ship, "unload")
            }),
            Err(SimError::ShipNotLanded)
        );
        assert_eq!(
            w.submit(Command {
                target_entity: Some(port),
                ..command(&w, ship, "unload")
            }),
            Err(SimError::ShipNotLanded)
        );
        // Landing sets them down.
        land(&mut w, ship, at(HOME, 30_500, 40_500));
        let e = w.entity(walkers[0]).unwrap();
        assert_eq!((e.container_id, e.pos.body), (None, HOME));
        w.hash()
    };
    assert_eq!(play(), play());
}

#[test]
fn capacity_and_who_may_board_come_from_the_rules() {
    let rules = rules();
    let mut w = world(&rules);
    let corvette = w
        .spawn(id(&w, "unit.corvette"), P1, w.body_position(HOME).unwrap())
        .unwrap();
    orbit(&mut w, corvette, HOME);
    let transport = transport_in_orbit(&mut w, HOME);
    assert_eq!(w.carry_capacity(corvette), 1);
    assert_eq!(w.carry_capacity(transport), 20);
    let units = passengers(&mut w, 2, 0);
    // Aboard a ship in orbit: refused until it lands (task 68).
    assert_eq!(
        w.submit(Command {
            target_entity: Some(corvette),
            ..command(&w, units[0], "enter")
        }),
        Err(SimError::ShipNotLanded)
    );
    land(&mut w, corvette, at(HOME, 30_500, 42_500));
    order(&mut w, units[0], "enter", Some(corvette), None);
    run_until(&mut w, 2000, |w| w.passengers(corvette).len() == 1);
    assert_eq!(
        w.submit(Command {
            target_entity: Some(corvette),
            ..command(&w, units[1], "enter")
        }),
        Err(SimError::NoRoomAboard)
    );
    // A ship does not board a ship; a rival's ship takes nobody.
    assert_eq!(
        w.submit(Command {
            target_entity: Some(transport),
            ..command(&w, corvette, "enter")
        }),
        Err(SimError::CannotBoard)
    );
    let foreign = w
        .spawn(id(&w, "unit.marine"), P2, at(HOME, 30_500, 40_500))
        .unwrap();
    assert_eq!(
        w.submit(Command {
            target_entity: Some(transport),
            ..command(&w, foreign, "enter")
        }),
        Err(SimError::CannotBoard)
    );
}

#[test]
fn a_destroyed_carrier_takes_its_passengers_with_it() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        add_neighbour(&mut w, &rules);
        let ship = transport_in_orbit(&mut w, HOME);
        let units = passengers(&mut w, 2, 0);
        board(&mut w, &units, ship);
        orbit(&mut w, ship, NEIGHBOUR);
        // Rival warships in orbit beside it shoot it down.
        for n in 0..4 {
            let marker = w.body_position(NEIGHBOUR).unwrap();
            let foe = w
                .spawn(
                    id(&w, "unit.corvette"),
                    P2,
                    Pos {
                        x: marker.x - 1000 - n * 300,
                        ..marker
                    },
                )
                .unwrap();
            order(&mut w, foe, "attack", Some(ship), None);
        }
        let losses = w.stats().losses.get(&P1).copied().unwrap_or(0);
        run_until(&mut w, 6000, |w| w.entity(ship).is_none());
        for unit in &units {
            assert!(w.entity(*unit).is_none());
        }
        assert_eq!(w.stats().losses.get(&P1).copied().unwrap_or(0), losses + 3);
        w.hash()
    };
    assert_eq!(play(), play());
}

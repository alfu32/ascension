//! Orbital combat (task 58): craft trained at an airfield climb to orbit by themselves and fight
//! there, bombers strike the ground from orbit, craft ride a carrier to another system and launch
//! there, and ground batteries fire on what orbits above them. Every scenario runs twice and
//! compares state hashes.

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

/// Every face plain except face 1, deep sea.
fn planet(rules: &Rules) -> PlanetSurface {
    let side = (N * N) as usize;
    let mut terrain = vec![rules.id_of("ter.plain").unwrap(); 6 * side];
    terrain[side..2 * side].fill(rules.id_of("ter.deep").unwrap());
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain,
        nodes: vec![],
    }
}

/// Two systems on one route: the home planet in the first, a far planet in the second. Both
/// players are in the age of expansion with every technology.
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
        figure: Vec::new(),
        systems: [(1, 0, 0), (2, 100, 0)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
                z: 0,
                constellation: String::new(),
                id,
                x,
                y,
                name: format!("System {id}"),
            })
            .collect(),
        bodies: vec![
            body(1, 1, BodyKind::Star, 0, 0),
            body(2, 1, BodyKind::Habitable, 20, 0),
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
        58,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P1, at(HOME, 0, 20_500, 32_500))
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

/// A ship of `owner` that enters the orbit of `body` above `over` (or the face centre).
fn ship_in_orbit(
    w: &mut World,
    key: &str,
    owner: PlayerId,
    body: BodyId,
    over: Option<Pos>,
) -> EntityId {
    let marker = w.body_position(body).unwrap();
    let ship = w
        .spawn(
            id(w, key),
            owner,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    match over {
        Some(pos) => order(w, ship, "orbit", None, Some(pos)),
        None => {
            let marker = w.body_marker(body).unwrap();
            order(w, ship, "orbit", Some(marker), None);
        }
    }
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == body));
    ship
}

/// Trains `count` craft of `key` at a fresh airfield of player one on the home planet.
fn train_craft(w: &mut World, key: &str, count: usize) -> Vec<EntityId> {
    let airfield = w
        .place_for_setup(id(w, "bld.airfield"), P1, at(HOME, 0, 30_000, 40_000))
        .unwrap();
    let kind = id(w, key);
    for _ in 0..count {
        w.submit(Command {
            subject: Some(kind),
            queue: true,
            ..command(w, airfield, "train")
        })
        .unwrap();
    }
    run_until(w, 4000, |w| {
        w.entities().filter(|e| e.type_id == kind).count() == count
    });
    let craft: Vec<_> = w
        .entities()
        .filter(|e| e.type_id == kind)
        .map(|e| e.id)
        .collect();
    // They take off beside the airfield, as aircraft in the planet's air.
    for c in &craft {
        let e = w.entity(*c).unwrap();
        assert_eq!(e.pos.body, HOME);
        assert!(w.orbit(*c).is_none());
        assert!(w.is_craft_type(e.type_id));
    }
    craft
}

/// Every craft lifts off; the climb is drawn by the altitude and ends in the planet's orbit.
fn climb(w: &mut World, craft: &[EntityId]) {
    for c in craft {
        order(w, *c, "lift", None, None);
    }
    w.step();
    w.step();
    w.step();
    let pct = w.altitude_pct(craft[0]).unwrap();
    assert!((0..100).contains(&pct) && pct > 0, "{pct}");
    run_until(w, 400, |w| {
        craft
            .iter()
            .all(|c| w.orbit(*c).is_some_and(|o| o.body == HOME))
    });
    assert_eq!(w.altitude_pct(craft[0]), Some(100));
}

#[test]
fn craft_take_off_from_an_airfield_climb_to_orbit_and_fight_a_ship() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let fighters = train_craft(&mut w, "unit.orbital_fighter", 3);
        let trained = w.hash();
        climb(&mut w, &fighters);
        let climbed = w.hash();
        // In orbit they hold it: no move order takes them into the system on their own.
        let space = w.body_position(HOME).unwrap();
        assert_eq!(
            w.submit(Command {
                target_pos: Some(Pos {
                    x: space.x + 9000,
                    ..space
                }),
                ..command(&w, fighters[0], "move")
            }),
            Err(SimError::CraftInOrbit)
        );
        assert_eq!(
            w.submit(command(&w, fighters[0], "lift")),
            Err(SimError::CannotClimb)
        );
        // An enemy corvette comes into the same orbit; the craft engage it there.
        let corvette = ship_in_orbit(&mut w, "unit.corvette", P2, HOME, None);
        let hp = w.entity(corvette).unwrap().max_hp;
        run_until(&mut w, 3000, |w| w.entity(corvette).is_none());
        assert!(hp > 0);
        assert!(w.stats().kills.get(&P1).copied().unwrap_or(0) >= 1);
        // Back down into the air straight below (task 68: it holds over one point).
        let survivor = *fighters.iter().find(|c| w.entity(**c).is_some()).unwrap();
        let below = w.orbit(survivor).unwrap().pos;
        order(&mut w, survivor, "land", None, Some(below));
        w.step();
        w.step();
        let pct = w.altitude_pct(survivor).unwrap();
        assert!((50..100).contains(&pct), "{pct}");
        run_until(&mut w, 400, |w| w.orbit(survivor).is_none());
        assert_eq!(w.entity(survivor).unwrap().pos, below);
        assert_eq!(w.altitude_pct(survivor), None);
        (trained, climbed, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!(
        "craft_take_off_from_an_airfield_climb_to_orbit_and_fight_a_ship hashes={first:016x?}"
    );
}

#[test]
fn bombers_strike_the_ground_from_orbit() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let target = w
            .place_for_setup(id(&w, "bld.factory"), P2, at(HOME, 0, 34_000, 46_000))
            .unwrap();
        let full = w.entity(target).unwrap().hp;
        let bombers = train_craft(&mut w, "unit.orbital_bomber", 2);
        climb(&mut w, &bombers);
        for b in &bombers {
            order(&mut w, *b, "attack", Some(target), None);
        }
        run_until(&mut w, 2000, |w| {
            w.entity(target).is_none_or(|t| t.hp < full)
                && bombers
                    .iter()
                    .any(|b| w.orbit(*b).is_some_and(|o| o.last_shot.is_some()))
        });
        let struck = w.hash();
        let shot = bombers
            .iter()
            .find_map(|b| w.orbit(*b).and_then(|o| o.last_shot))
            .unwrap();
        assert_eq!(shot.target.body, HOME);
        // The bombers stay in orbit while they strike.
        assert!(bombers.iter().all(|b| w.orbit(*b).is_some()));
        (struck, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!("bombers_strike_the_ground_from_orbit hashes={first:016x?}");
}

#[test]
fn craft_ride_a_carrier_to_another_system_and_launch_there() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let fighters = train_craft(&mut w, "unit.orbital_fighter", 2);
        let carrier = ship_in_orbit(&mut w, "unit.carrier", P1, HOME, None);
        // The carrier takes craft only; the transport takes no craft.
        let citizen = w
            .spawn(id(&w, "unit.citizen"), P1, at(HOME, 0, 24_000, 36_000))
            .unwrap();
        assert_eq!(
            w.submit(Command {
                target_entity: Some(carrier),
                ..command(&w, citizen, "enter")
            }),
            Err(SimError::CannotBoard)
        );
        let transport = ship_in_orbit(&mut w, "unit.transport_ship", P1, HOME, None);
        assert_eq!(
            w.submit(Command {
                target_entity: Some(transport),
                ..command(&w, fighters[0], "enter")
            }),
            Err(SimError::CannotBoard)
        );
        // One craft is in orbit already, the other climbs from the air to board.
        order(&mut w, fighters[0], "lift", None, None);
        run_until(&mut w, 400, |w| w.orbit(fighters[0]).is_some());
        for f in &fighters {
            order(&mut w, *f, "enter", Some(carrier), None);
        }
        run_until(&mut w, 400, |w| w.passengers(carrier).len() == 2);
        assert!(fighters.iter().all(|f| w.orbit(*f).is_none()));
        let boarded = w.hash();
        // Aboard they take no orders until launched.
        assert_eq!(
            w.submit(command(&w, fighters[0], "stop")),
            Err(SimError::Aboard)
        );
        // Across the route to the second system and into the orbit of its planet.
        w.submit(Command {
            target_pos: Some(Pos {
                body: w.space_of(2).unwrap(),
                ..Pos::default()
            }),
            ..command(&w, carrier, "travel")
        })
        .unwrap();
        run_until(&mut w, 20_000, |w| {
            w.entity(carrier)
                .is_some_and(|e| Some(e.pos.body) == w.space_of(2) && e.orders().is_empty())
        });
        w.add_surface(FAR, &planet(&rules)).unwrap();
        let enemy = ship_in_orbit(&mut w, "unit.corvette", P2, FAR, None);
        let marker = w.body_marker(FAR).unwrap();
        order(&mut w, carrier, "orbit", Some(marker), None);
        run_until(&mut w, 4000, |w| {
            w.orbit(carrier).is_some_and(|o| o.body == FAR)
        });
        let arrived = w.hash();
        // Launched into the carrier's orbit, the craft fight there.
        order(&mut w, carrier, "unload", Some(carrier), None);
        w.step();
        assert!(w.passengers(carrier).is_empty());
        for f in &fighters {
            assert_eq!(w.orbit(*f).map(|o| o.body), Some(FAR));
            assert_eq!(w.entity(*f).unwrap().container_id, None);
        }
        run_until(&mut w, 3000, |w| w.entity(enemy).is_none());
        (boarded, arrived, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!("craft_ride_a_carrier_to_another_system_and_launch_there hashes={first:016x?}");
}

#[test]
fn batteries_shoot_down_a_descending_transport_and_damage_a_ship_in_orbit() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        // Eight batteries of player two around the middle of face 3, far from where the ships
        // first enter the orbit.
        let battery = id(&w, "bld.ground_battery");
        let mut batteries = Vec::new();
        for (dx, dy) in [
            (-4, -4),
            (0, -4),
            (4, -4),
            (-4, 0),
            (4, 0),
            (-4, 4),
            (0, 4),
            (4, 4),
        ] {
            batteries.push(
                w.place_for_setup(
                    battery,
                    P2,
                    at(HOME, 3, 32_500 + dx * 1000, 32_500 + dy * 1000),
                )
                .unwrap(),
            );
        }
        let transport = ship_in_orbit(&mut w, "unit.transport_ship", P1, HOME, None);
        let troops: Vec<_> = (0..2)
            .map(|n| {
                w.spawn(
                    id(&w, "unit.marine"),
                    P1,
                    at(HOME, 0, 25_000 + n * 1000, 30_000),
                )
                .unwrap()
            })
            .collect();
        w.place_for_setup(id(&w, "bld.spaceport"), P1, at(HOME, 0, 28_000, 36_000))
            .unwrap();
        // Troops board only a landed ship (task 68): down beside them, aboard, up again.
        let pad = w
            .landing_site(transport, at(HOME, 0, 25_000, 34_000))
            .unwrap();
        order(&mut w, transport, "land", None, Some(pad));
        run_until(&mut w, 2000, |w| w.is_landed(transport));
        for t in &troops {
            order(&mut w, *t, "enter", Some(transport), None);
        }
        run_until(&mut w, 2000, |w| w.passengers(transport).len() == 2);
        order(&mut w, transport, "lift", None, None);
        run_until(&mut w, 200, |w| w.orbit(transport).is_some());
        // Out of their range nothing fires.
        let hp = w.entity(transport).unwrap().hp;
        assert_eq!(hp, w.entity(transport).unwrap().max_hp);
        let site = w
            .landing_site(transport, at(HOME, 3, 39_500, 32_500))
            .unwrap();
        order(&mut w, transport, "land", None, Some(site));
        run_until(&mut w, 2000, |w| {
            w.entity(transport).is_none() || w.is_landed(transport)
        });
        assert!(w.entity(transport).is_none(), "shot down while descending");
        assert!(troops.iter().all(|t| w.entity(*t).is_none()));
        let downed = w.hash();
        // A corvette orbiting over them is hit too.
        let corvette = ship_in_orbit(
            &mut w,
            "unit.corvette",
            P1,
            HOME,
            Some(at(HOME, 3, 26_000, 32_500)),
        );
        let full = w.entity(corvette).unwrap().max_hp;
        run_until(&mut w, 200, |w| {
            w.entity(corvette).is_none_or(|c| c.hp < full)
        });
        let damaged = w.hash();
        assert!(w.stats().kills.get(&P2).copied().unwrap_or(0) >= 3);
        (downed, damaged)
    };
    let first = play();
    assert_eq!(first, play());
    println!(
        "batteries_shoot_down_a_descending_transport_and_damage_a_ship_in_orbit hashes={first:016x?}"
    );
}

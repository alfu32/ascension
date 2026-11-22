//! Aircraft (task 123): jets and bombers come out of the airfield landed, take off before they
//! fly or fight, land on open ground or an own airfield, are ground targets that cannot fire
//! while they are down, and settle back when a take-off is called off. Every scenario runs twice
//! and compares state hashes.

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

/// One system with the home planet of both players, in `age` with every technology, and a city
/// of player one.
fn world(rules: &Arc<Rules>, age: &str) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of(age).unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: if p == 0 {
                BTreeMap::new()
            } else {
                rules.techs.keys().map(|k| (*k, 1)).collect()
            },
        })
        .collect();
    let body = |n, kind| BodyInfo {
        id: BodyId(n),
        system: 1,
        kind,
        name: format!("Body {n}"),
        orbit_radius: if kind == BodyKind::Star { 0 } else { 20 },
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
            id: 1,
            x: 0,
            y: 0,
            name: "System 1".into(),
        }],
        bodies: vec![body(1, BodyKind::Star), body(2, BodyKind::Habitable)],
        routes: vec![],
        planets: BTreeMap::from([(HOME, planet(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        123,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P1, at(0, 20_500, 32_500))
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
    // On a failure, every aircraft's place, climb and order help tell why.
    if !done(w) {
        for e in w.entities().filter(|e| w.is_aircraft_type(e.type_id)) {
            eprintln!(
                "{:?} {:?} {:?} {:?}",
                e.id,
                e.pos,
                w.flight_climbed(e.id),
                e.orders().front().map(|o| (
                    o.kind.clone(),
                    o.status.clone(),
                    o.progress_ticks,
                    o.home,
                    o.target_pos
                ))
            );
        }
        eprintln!("{:?}", w.order_failures(PlayerId(1)).collect::<Vec<_>>());
    }
    assert!(done(w), "not reached in {limit} ticks (tick {})", w.tick());
}

const AIRFIELD: (i32, i32) = (34_000, 40_000);

/// Trains `count` aircraft of `key` at a fresh airfield of player one; returns the airfield
/// and the aircraft.
fn train(w: &mut World, key: &str, count: usize) -> (EntityId, Vec<EntityId>) {
    let airfield = w
        .place_for_setup(id(w, "bld.airfield"), P1, at(0, AIRFIELD.0, AIRFIELD.1))
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
    run_until(w, 6000, |w| {
        w.entities().filter(|e| e.type_id == kind).count() == count
    });
    let planes = w
        .entities()
        .filter(|e| e.type_id == kind)
        .map(|e| e.id)
        .collect();
    (airfield, planes)
}

fn tile(p: Pos) -> (u8, i32, i32) {
    (p.face, p.x.div_euclid(1000), p.y.div_euclid(1000))
}

fn ground_tile(w: &World, p: Pos) -> bool {
    let terrain = w.terrain_at(p).unwrap();
    w.rules().terrains[&terrain].layer == rom_rules::Layer::Ground
}

#[test]
fn trained_aircraft_come_out_landed_and_take_off_to_fly() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules, "age.information");
        let jet = id(&w, "unit.jet_fighter");
        let ticks = w.takeoff_ticks(jet).unwrap();
        assert!((60..=100).contains(&ticks), "{ticks}");
        assert!(w.is_airfield_type(id(&w, "bld.airfield")));
        // Orbital craft and the hovering repair drone never land.
        assert_eq!(w.takeoff_ticks(id(&w, "unit.orbital_fighter")), None);
        assert_eq!(w.takeoff_ticks(id(&w, "unit.repair_drone")), None);
        let (_, jets) = train(&mut w, "unit.jet_fighter", 3);
        let trained = w.hash();
        // Each stands landed on a ground tile of its own beside (or on) the airfield.
        let mut tiles = Vec::new();
        for j in &jets {
            assert!(w.is_aircraft_landed(*j), "{j:?}");
            assert_eq!(w.flight_pct(*j), Some(0));
            let pos = w.entity(*j).unwrap().pos;
            assert!(ground_tile(&w, pos));
            assert!(w.distance(pos, at(0, AIRFIELD.0, AIRFIELD.1)).unwrap() < 12_000);
            tiles.push(tile(pos));
        }
        tiles.sort_unstable();
        tiles.dedup();
        assert_eq!(tiles.len(), jets.len());
        // A move takes it up first: it rolls forward along its heading while it climbs.
        let goal = at(0, 20_000, 20_000);
        let start = w.entity(jets[0]).unwrap().pos;
        order(&mut w, jets[0], "move", None, Some(goal));
        for _ in 0..ticks / 2 {
            w.step();
        }
        let pct = w.flight_pct(jets[0]).unwrap();
        assert!((30..70).contains(&pct), "{pct}");
        assert_ne!(w.entity(jets[0]).unwrap().pos, start);
        assert!(w.is_grounded(jets[0]));
        run_until(&mut w, ticks, |w| !w.is_grounded(jets[0]));
        assert_eq!(w.flight_pct(jets[0]), Some(100));
        let rolled = w.distance(start, w.entity(jets[0]).unwrap().pos).unwrap();
        assert!((2_000..=3_100).contains(&rolled), "{rolled}");
        run_until(&mut w, 2000, |w| {
            w.entity(jets[0]).unwrap().orders().is_empty()
        });
        assert!(w.distance(w.entity(jets[0]).unwrap().pos, goal).unwrap() < 1_500);
        // Idle in the air it stays there.
        for _ in 0..100 {
            w.step();
        }
        assert_eq!(w.flight_pct(jets[0]), Some(100));
        // The others are still down.
        assert!(w.is_aircraft_landed(jets[1]) && w.is_aircraft_landed(jets[2]));
        (trained, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!("trained_aircraft_come_out_landed_and_take_off_to_fly hashes={first:016x?}");
}

#[test]
fn a_landed_aircraft_is_a_ground_target_that_cannot_fire() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules, "age.information");
        let (_, jets) = train(&mut w, "unit.jet_fighter", 1);
        let jet = jets[0];
        let pos = w.entity(jet).unwrap().pos;
        // An enemy fighter overhead, out of reach of nothing: the landed jet does not fire at it.
        let enemy = w
            .spawn(
                id(&w, "unit.jet_fighter"),
                P2,
                Pos {
                    x: pos.x + 2_000,
                    ..pos
                },
            )
            .unwrap();
        // A tank of player two: its gun reaches the landed jet, not the airborne fighter.
        let tank = w
            .spawn(
                id(&w, "unit.tank"),
                P2,
                Pos {
                    x: pos.x - 3_000,
                    ..pos
                },
            )
            .unwrap();
        assert!(w.can_target(tank, jet));
        assert!(!w.can_target(tank, enemy));
        // Anti-air weapons still reach it on the ground.
        assert!(w.can_target(enemy, jet));
        let full = w.entity(jet).unwrap().hp;
        run_until(&mut w, 400, |w| w.entity(jet).is_none_or(|e| e.hp < full));
        assert!(w.entity(jet).is_none_or(|e| e.cooldowns.is_empty()));
        let shot = w.hash();
        // Sent up, it fights in the air; the tank no longer reaches it.
        if w.entity(jet).is_some() {
            order(&mut w, jet, "attack", Some(enemy), None);
            let ticks = w.takeoff_ticks(w.entity(jet).unwrap().type_id).unwrap();
            run_until(&mut w, ticks + 5, |w| {
                w.entity(jet).is_none_or(|_| !w.is_grounded(jet))
            });
            if w.entity(jet).is_some() {
                assert!(!w.can_target(tank, jet));
            }
        }
        (shot, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!("a_landed_aircraft_is_a_ground_target_that_cannot_fire hashes={first:016x?}");
}

#[test]
fn aircraft_land_on_open_ground_or_an_own_airfield() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules, "age.modern");
        let (airfield, bombers) = train(&mut w, "unit.bomber", 3);
        // Up and away from the airfield.
        let field = at(0, 15_000, 50_000);
        for b in &bombers {
            order(&mut w, *b, "move", None, Some(field));
            assert_eq!(
                w.submit(Command {
                    queue: true,
                    ..command(&w, *b, "lift")
                })
                .err(),
                None,
                "a landed aircraft may be told to take off"
            );
        }
        run_until(&mut w, 3000, |w| {
            bombers
                .iter()
                .all(|b| w.entity(*b).unwrap().orders().is_empty() && !w.is_grounded(*b))
        });
        // Airborne, there is nothing to take off from; the sea has no spot to land on.
        assert_eq!(
            w.submit(command(&w, bombers[0], "lift")),
            Err(SimError::NotLanded)
        );
        assert_eq!(
            w.submit(Command {
                target_pos: Some(at(1, 32_000, 32_000)),
                ..command(&w, bombers[0], "land")
            }),
            Err(SimError::CannotLand)
        );
        // Two land at one point of open ground: each on a ground tile of its own near it.
        let point = at(0, 10_500, 45_500);
        order(&mut w, bombers[0], "land", None, Some(point));
        order(&mut w, bombers[1], "land", None, Some(point));
        // The third is sent to its airfield.
        let field_pos = w.entity(airfield).unwrap().pos;
        order(&mut w, bombers[2], "land", None, Some(field_pos));
        w.step();
        w.step();
        run_until(&mut w, 3000, |w| {
            bombers.iter().all(|b| w.is_aircraft_landed(*b))
        });
        let landed = w.hash();
        let (a, b) = (
            w.entity(bombers[0]).unwrap().pos,
            w.entity(bombers[1]).unwrap().pos,
        );
        // Wing tip to wing tip apart: both reaches and the margin.
        let reach = w.aircraft_reach(id(&w, "unit.bomber"));
        assert!(reach >= 1_500, "{reach}");
        let apart = (2 * reach + rom_sim::LANDING_MARGIN) as u64;
        assert!(w.distance(a, b).unwrap() >= apart);
        for p in [a, b] {
            assert!(ground_tile(&w, p));
            assert!(w.distance(p, point).unwrap() <= apart + 1_500, "{p:?}");
        }
        // At the airfield it parks beside it, its wings clear of the footprint (hangar, tower).
        let on_field = w.entity(bombers[2]).unwrap().pos;
        let dx = (on_field.x - field_pos.x).abs();
        let dy = (on_field.y - field_pos.y).abs();
        assert!(i64::from(dx.max(dy)) >= 3_000 + reach, "{on_field:?}");
        assert!(dx.max(dy) <= 9_000, "{on_field:?}");
        (landed, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!("aircraft_land_on_open_ground_or_an_own_airfield hashes={first:016x?}");
}

#[test]
fn a_take_off_called_off_settles_back_down() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules, "age.industrial");
        let (_, planes) = train(&mut w, "unit.biplane", 1);
        let plane = planes[0];
        order(&mut w, plane, "lift", None, None);
        for _ in 0..20 {
            w.step();
        }
        let climbed = w.flight_climbed(plane).unwrap();
        assert!(climbed > 10, "{climbed}");
        order(&mut w, plane, "stop", None, None);
        run_until(&mut w, 100, |w| w.is_aircraft_landed(plane));
        let settled = w.hash();
        // Land where it stands when already down: done at once.
        let here = w.entity(plane).unwrap().pos;
        order(&mut w, plane, "land", None, Some(here));
        w.step();
        assert!(w.entity(plane).unwrap().orders().is_empty());
        assert!(w.is_aircraft_landed(plane));
        (settled, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!("a_take_off_called_off_settles_back_down hashes={first:016x?}");
}

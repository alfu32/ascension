//! Shared scenario helpers: rules, the save and load check, and a small universe.
#![allow(dead_code)]

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_save::{AiState, AiStates, Controller, MatchInfo, Seat};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Instant};

pub const P1: PlayerId = PlayerId(1);
pub const P2: PlayerId = PlayerId(2);
pub const HOME: BodyId = BodyId(2);
pub const NEIGHBOUR: BodyId = BodyId(5);
pub const N: u32 = 64;

pub fn rules_with(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../../db/schema.sql"),
        include_str!("../../../../db/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

pub fn rules() -> Arc<Rules> {
    rules_with("")
}

pub fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

pub fn at(body: BodyId, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        body,
    }
}

pub fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rom-save-test-{}-{name}.db", std::process::id()))
}

/// Seats for players 0, 1 and 2 (the neutral player included).
pub fn info(world: &World) -> MatchInfo {
    let mut info = all_seats();
    info.seats
        .retain(|seat| world.players().any(|p| p.id == seat.player));
    info
}

fn all_seats() -> MatchInfo {
    MatchInfo {
        seed: 7,
        map_style: "style.test".into(),
        speed_pct: 150,
        settings: BTreeMap::from([("map_size".into(), "small".into())]),
        seats: vec![
            Seat {
                player: PlayerId(0),
                slot: 2,
                name: String::new(),
                controller: Controller::Nature,
                ai_profile: String::new(),
                color: 0,
            },
            Seat {
                player: P1,
                slot: 0,
                name: "Alpha".into(),
                controller: Controller::Human,
                ai_profile: String::new(),
                color: 3,
            },
            Seat {
                player: P2,
                slot: 1,
                name: "Beta".into(),
                controller: Controller::Ai,
                ai_profile: "ai.test".into(),
                color: 5,
            },
        ],
    }
}

pub fn ai() -> AiStates {
    BTreeMap::from([(
        P2,
        AiState {
            profile: "ai.test".into(),
            difficulty: "diff.test".into(),
            version: 3,
            rng_state: u64::MAX - 5,
            opening_seq: 4,
            data: vec![1, 2, 3, 250, 0, 255],
        },
    )])
}

/// What one scenario measured, for the report.
#[derive(Debug)]
pub struct Outcome {
    pub name: String,
    pub tick: u64,
    pub hash_before: u64,
    pub hash_after_load: u64,
    pub hash_after_run: u64,
    pub bytes: u64,
    pub save_ms: u128,
    pub load_ms: u128,
}

/// The defining check. Saves `world` as it stands, loads it, and requires the same hash
/// and the same exported state; then runs both worlds `more` ticks with the same
/// `orders`, comparing hashes along the way and at the end, and finally saves and loads
/// the loaded world again.
pub fn check(
    name: &str,
    rules: &Arc<Rules>,
    mut world: World,
    more: u64,
    orders: impl Fn(&mut World),
) -> Outcome {
    let path = temp_path(name);
    let tick = world.tick();
    let hash_before = world.hash();
    let started = Instant::now();
    rom_save::save(&path, &world, &info(&world), &ai()).unwrap();
    let save_ms = started.elapsed().as_millis();
    let bytes = std::fs::metadata(&path).unwrap().len();
    let started = Instant::now();
    let (mut loaded, loaded_info, loaded_ai) = rom_save::load(&path, rules).unwrap();
    let load_ms = started.elapsed().as_millis();
    assert_eq!(loaded_info, info(&world));
    assert_eq!(loaded_ai, ai());
    let header = rom_save::read_header(&path).unwrap();
    assert_eq!(header.tick, tick);
    assert_eq!(header.info, info(&world));
    let hash_after_load = loaded.hash();
    assert_eq!(hash_after_load, hash_before, "{name}: hash after load");
    assert_eq!(loaded.tick(), tick);
    assert!(
        loaded.snapshot() == world.snapshot(),
        "{name}: exported state differs after load"
    );
    orders(&mut world);
    orders(&mut loaded);
    for step in 0..more {
        world.step();
        loaded.step();
        if step % 8 == 0 || step + 1 == more {
            assert_eq!(
                world.hash(),
                loaded.hash(),
                "{name}: diverged {} ticks after load",
                step + 1
            );
        }
    }
    let hash_after_run = world.hash();
    assert_eq!(loaded.hash(), hash_after_run, "{name}: hash after the run");
    assert!(
        loaded.snapshot() == world.snapshot(),
        "{name}: exported state differs after the run"
    );
    // A loaded world saves and loads again to the same state.
    rom_save::save(&path, &loaded, &info(&loaded), &ai()).unwrap();
    let (again, _, _) = rom_save::load(&path, rules).unwrap();
    assert_eq!(again.hash(), hash_after_run, "{name}: second round trip");
    std::fs::remove_file(&path).unwrap();
    let outcome = Outcome {
        name: name.into(),
        tick,
        hash_before,
        hash_after_load,
        hash_after_run,
        bytes,
        save_ms,
        load_ms,
    };
    println!("{outcome:?}");
    outcome
}

pub fn command(w: &World, executor: EntityId, kind: &str) -> Command {
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

pub fn order(
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

pub fn run_until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "not reached in {limit} ticks (tick {})", w.tick());
}

pub fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}

/// Face 0 of every planet is plain; face 1 is deep sea, face 2 forest.
pub fn planet(rules: &Rules) -> PlanetSurface {
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

/// Two systems on one route: the home planet and a neighbour in the first, a far planet in
/// the second. Only the home planet is generated; player one has a city and a spaceport.
pub fn universe(rules: &Arc<Rules>) -> World {
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

/// A transport in orbit of `body`, from system space beside its marker.
pub fn transport_in_orbit(w: &mut World, body: BodyId) -> EntityId {
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

pub fn orbit(w: &mut World, ship: EntityId, body: BodyId) {
    let marker = w.body_marker(body).unwrap();
    order(w, ship, "orbit", Some(marker), None);
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == body));
}

/// Ground units of player one beside the spaceport.
pub fn passengers(w: &mut World, citizens: u32, marines: u32) -> Vec<EntityId> {
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

pub fn board(w: &mut World, units: &[EntityId], ship: EntityId) {
    for unit in units {
        order(w, *unit, "enter", Some(ship), None);
    }
    let count = units.len();
    run_until(w, 2000, |w| w.passengers(ship).len() == count);
}

pub fn add_neighbour(w: &mut World, rules: &Rules) {
    w.add_surface(NEIGHBOUR, &planet(rules)).unwrap();
}

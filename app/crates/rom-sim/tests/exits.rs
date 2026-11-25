//! Units leave buildings: ships appear on water beside a dock (never stuck on land, never stuck
//! at 100 % in the queue), and builders are never shut inside a building they stand on.
//! Every scenario runs twice and compares state hashes.

use std::{collections::BTreeMap, sync::Arc};

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Layer, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, MapGrid, OrderStatus, PlanetSurface, Player, PlayerId,
    Pos, SystemInfo, UniverseSetup, World,
};

const P1: PlayerId = PlayerId(1);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn player(rules: &Rules, n: u32) -> Player {
    Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.classical").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
        techs: Default::default(),
    }
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn flat(pos: Pos) -> Pos {
    Pos { face: 0, ..pos }
}

fn command(player: PlayerId, executor: EntityId, kind: &str) -> Command {
    Command {
        player,
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

/// A 64 x 64 flat map: shallow water west of column 24, plain elsewhere (or all plain).
fn flat_world(with_water: bool) -> World {
    let rules = rules();
    let (plain, coast) = (
        rules.id_of("ter.plain").unwrap(),
        rules.id_of("ter.coast").unwrap(),
    );
    let terrain = (0..64 * 64)
        .map(|i| {
            if with_water && i % 64 < 24 {
                coast
            } else {
                plain
            }
        })
        .collect();
    let map = MapGrid::new(64, 64, terrain).unwrap();
    let players = vec![player(&rules, 1)];
    World::new(rules, map, 7, players).unwrap()
}

fn is_water(w: &World, pos: Pos) -> bool {
    let terrain = w.terrain_at(pos).expect("on the map");
    w.rules().terrains[&terrain].layer == Layer::Fluid
}

fn train(w: &mut World, dock: EntityId, ship: DefId) {
    w.submit(Command {
        subject: Some(ship),
        queue: true,
        ..command(P1, dock, "train")
    })
    .unwrap();
}

#[test]
fn a_ship_appears_on_the_water_behind_a_dock_whose_gate_faces_land() {
    let run = || {
        let mut w = flat_world(true);
        let dock = w
            .place_for_setup(id(&w, "bld.dock"), P1, flat(Pos::flat(28_000, 32_000)))
            .unwrap();
        let ship = id(&w, "unit.trireme");
        // Three ships in a row: the whole queue empties, nothing stays at 100 %.
        for _ in 0..3 {
            train(&mut w, dock, ship);
        }
        w.step();
        let mut guard = 0;
        while w
            .entity(dock)
            .unwrap()
            .orders()
            .iter()
            .any(|o| o.kind == "train")
        {
            w.step();
            guard += 1;
            assert!(guard < 20_000, "the queue never emptied");
        }
        let ships: Vec<_> = w.entities().filter(|e| e.type_id == ship).collect();
        assert_eq!(ships.len(), 3, "stats {:?}", w.stats());
        for e in ships {
            assert!(is_water(&w, e.pos), "ship at {:?} is on land", e.pos);
            assert!(
                e.pos.x < 24_000,
                "ship must be west of the shore: {:?}",
                e.pos
            );
        }
        // They can move: sail west for a while.
        let first = w.entities().find(|e| e.type_id == ship).unwrap().id;
        w.submit(Command {
            target_pos: Some(Pos::flat(4_500, 32_500)),
            ..command(P1, first, "move")
        })
        .unwrap();
        for _ in 0..400 {
            w.step();
        }
        assert!(w.entity(first).unwrap().pos.x < 20_000);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn without_water_in_reach_the_ship_waits_in_the_queue_and_says_why() {
    let run = || {
        let mut w = flat_world(false);
        let dock = w
            .place_for_setup(id(&w, "bld.dock"), P1, flat(Pos::flat(28_000, 32_000)))
            .unwrap();
        let ship = id(&w, "unit.trireme");
        train(&mut w, dock, ship);
        for _ in 0..6_000 {
            w.step();
        }
        assert!(
            w.entities().all(|e| e.type_id != ship),
            "a ship was spawned on land"
        );
        let order = w.entity(dock).unwrap().orders().front().cloned().unwrap();
        assert_eq!(order.status, OrderStatus::Blocked("no_spawn_tile".into()));
        w.hash()
    };
    assert_eq!(run(), run());
}

/// One system with a star and one home planet whose west strip is water.
fn universe() -> World {
    let rules = rules();
    let n = 32;
    let (plain, coast) = (
        rules.id_of("ter.plain").unwrap(),
        rules.id_of("ter.coast").unwrap(),
    );
    let terrain = (0..6 * n * n)
        .map(|i| if (i % n) < 10 { coast } else { plain })
        .collect();
    let body = |id: u32, kind: BodyKind| BodyInfo {
        id: BodyId(id),
        system: 1,
        kind,
        name: format!("Body {id}"),
        orbit_radius: id as i32 * 1000,
        orbit_angle_mdeg: id as i32 * 1000,
        n: (kind == BodyKind::Habitable).then_some(n),
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
            name: "Testar".into(),
            x: 0,
            y: 0,
        }],
        bodies: vec![body(1, BodyKind::Star), body(2, BodyKind::Habitable)],
        routes: Vec::new(),
        planets: BTreeMap::from([(
            BodyId(2),
            PlanetSurface {
                surface: CubeSurface { n },
                terrain,
                nodes: Vec::new(),
            },
        )]),
    };
    let players = vec![player(&rules, 0), player(&rules, 1)];
    World::from_universe(rules, setup, 7, players, BTreeMap::from([(P1, BodyId(2))])).unwrap()
}

#[test]
fn in_a_universe_a_dock_makes_ships_for_the_water_not_for_space() {
    // Before the fix a ship from a dock counted as a spacecraft to launch, found no room in
    // space and stayed at 100 % forever.
    let run = || {
        let mut w = universe();
        let at = Pos::on(BodyId(2), 0, 16_000, 16_000);
        let dock = w.place_for_setup(id(&w, "bld.dock"), P1, at).unwrap();
        let ship = id(&w, "unit.trireme");
        train(&mut w, dock, ship);
        w.step();
        let mut guard = 0;
        while w
            .entity(dock)
            .unwrap()
            .orders()
            .iter()
            .any(|o| o.kind == "train")
        {
            w.step();
            guard += 1;
            assert!(guard < 20_000, "the ship stayed in the queue");
        }
        let e = w.entities().find(|e| e.type_id == ship).unwrap();
        assert_eq!(e.pos.body, BodyId(2));
        assert!(is_water(&w, e.pos), "ship at {:?} is on land", e.pos);
        w.hash()
    };
    assert_eq!(run(), run());
}

fn inside(pos: Pos, centre: Pos, half: i32) -> bool {
    (pos.x - centre.x).abs() < half && (pos.y - centre.y).abs() < half
}

#[test]
fn builders_are_never_trapped_in_the_building_they_stand_on() {
    let run = || {
        let mut w = flat_world(false);
        let city = id(&w, "bld.city");
        w.place_for_setup(city, P1, flat(Pos::flat(20_500, 20_500)))
            .unwrap();
        let citizen = id(&w, "unit.citizen");
        let farm = id(&w, "bld.farm");
        let site = flat(Pos::flat(26_000, 26_000));
        // Four builders already stand on the spot when the site is placed over them.
        let builders: Vec<_> = (0..4)
            .map(|k| {
                w.spawn(citizen, P1, flat(Pos::flat(25_000 + k * 700, 26_000)))
                    .unwrap()
            })
            .collect();
        // The first builder orders the farm; the others join once the site exists.
        w.submit(Command {
            subject: Some(farm),
            target_pos: Some(site),
            ..command(P1, builders[0], "construct")
        })
        .unwrap();
        w.step();
        let building = w.entities().find(|e| e.type_id == farm).unwrap().id;
        for &b in &builders[1..] {
            w.submit(Command {
                target_entity: Some(building),
                ..command(P1, b, "repair")
            })
            .unwrap();
        }
        let building = w.entities().find(|e| e.type_id == farm).unwrap().id;
        for &b in &builders {
            let pos = w.entity(b).unwrap().pos;
            assert!(
                !inside(pos, site, 2_000),
                "builder still on the site: {pos:?}"
            );
        }
        // A latecomer steps onto the site while it is being built.
        let late = w.spawn(citizen, P1, site).unwrap();
        let mut guard = 0;
        while w.entity(building).unwrap().build_progress.is_some() {
            w.step();
            guard += 1;
            assert!(guard < 10_000, "the farm was never finished");
        }
        for &b in builders.iter().chain([&late]) {
            let pos = w.entity(b).unwrap().pos;
            assert!(
                !inside(pos, site, 2_000),
                "{b:?} is inside the finished farm: {pos:?}"
            );
            // ... and can walk away from it.
            w.submit(Command {
                target_pos: Some(flat(Pos::flat(40_500, 40_500))),
                ..command(P1, b, "move")
            })
            .unwrap();
        }
        // Everyone walks out of the farm's neighbourhood (idle citizens later return to work in
        // it, so look for the furthest point reached).
        let mut furthest = vec![0; 5];
        for _ in 0..1_500 {
            w.step();
            for (k, &b) in builders.iter().chain([&late]).enumerate() {
                let pos = w.entity(b).unwrap().pos;
                furthest[k] = furthest[k].max(pos.x.min(pos.y));
            }
        }
        assert!(
            furthest.iter().all(|&reach| reach > 36_000),
            "somebody stayed shut in: {furthest:?}"
        );
        w.hash()
    };
    assert_eq!(run(), run());
}

//! Rotorcraft (task 119): helicopters and the hovering craft that replace them are aircraft that
//! hold a point in the air, hit the ground only, and are answered by anti-air. Every scenario
//! runs twice and compares state hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, SystemInfo,
    UniverseSetup, World,
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

#[test]
fn rotorcraft_hover_in_place_and_shoot_the_ground_only() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules, "age.information");
        let (_, craft) = train(&mut w, "unit.gunship_helicopter", 1);
        let heli = craft[0];
        let ticks = w.takeoff_ticks(id(&w, "unit.gunship_helicopter")).unwrap();
        assert!(ticks < 60, "rotorcraft lift quickly: {ticks}");
        assert!(w.is_aircraft_landed(heli));
        let goal = at(0, 20_000, 20_000);
        order(&mut w, heli, "move", None, Some(goal));
        run_until(&mut w, 3000, |w| {
            w.entity(heli).unwrap().orders().is_empty() && !w.is_grounded(heli)
        });
        // Idle in the air it stays exactly where it stopped.
        let held = w.entity(heli).unwrap().pos;
        for _ in 0..200 {
            w.step();
        }
        assert_eq!(w.entity(heli).unwrap().pos, held);
        assert_eq!(w.flight_pct(heli), Some(100));
        // It hits what stands on the ground, not what flies.
        let tank = w
            .spawn(id(&w, "unit.tank"), P2, at(0, held.x + 3_000, held.y))
            .unwrap();
        let jet = w
            .spawn(
                id(&w, "unit.jet_fighter"),
                P2,
                at(0, held.x, held.y + 3_000),
            )
            .unwrap();
        assert!(w.can_target(heli, tank));
        assert!(!w.can_target(heli, jet));
        // Anti-air answers it.
        let flak = w
            .spawn(id(&w, "unit.flak_gun"), P2, at(0, held.x, held.y - 3_000))
            .unwrap();
        assert!(w.can_target(flak, heli));
        assert!(!w.can_target(tank, heli));
        let full = w.entity(tank).unwrap().hp;
        run_until(&mut w, 400, |w| w.entity(tank).is_none_or(|e| e.hp < full));
        (w.hash(), w.entity(heli).map(|e| e.hp))
    };
    let first = play();
    assert_eq!(first, play());
    println!("rotorcraft_hover_in_place_and_shoot_the_ground_only hashes={first:016x?}");
}

#[test]
fn the_line_is_one_class_each_member_replacing_the_one_before() {
    let rules = rules();
    let w = world(&rules, "age.automation");
    let line: Vec<_> = ["attack_helicopter", "gunship_helicopter", "hover_gunship"]
        .iter()
        .map(|k| rules.entity_type(id(&w, &format!("unit.{k}"))).unwrap())
        .collect();
    assert!(line.iter().all(|e| e.ui_group == "rotorcraft"));
    assert_eq!(line[0].upgrades_from_id, None);
    assert_eq!(line[1].upgrades_from_id, Some(line[0].id));
    assert_eq!(line[2].upgrades_from_id, Some(line[1].id));
    for e in &line {
        assert!(w.is_aircraft_type(e.id));
    }
    // The gunship's missiles hurt tanks; the first helicopter's bullets barely do.
    let tank = rules
        .entity_type(id(&w, "unit.tank"))
        .unwrap()
        .armor_class_id
        .unwrap();
    let vs_tank = |e: &rom_rules::EntityType| {
        let weapon = &e.weapons[0];
        weapon.damage * rules.damage_pct(weapon.damage_class_id, tank) / 100
    };
    assert!(vs_tank(line[1]) > vs_tank(line[0]) * 2);
}

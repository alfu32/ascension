//! Drone operators (task 120): one infantry unit whose cloud of kamikaze drones flies ahead of it,
//! strikes with one drone per blow, is replenished by the operator, and is thinned by weapons
//! that hit the air. Every scenario runs twice and compares state hashes.

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
const OPERATOR: &str = "unit.drone_operator";

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

/// Every face plain.
fn planet(rules: &Rules) -> PlanetSurface {
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain: vec![rules.id_of("ter.plain").unwrap(); 6 * (N * N) as usize],
        nodes: vec![],
    }
}

/// One system with the home planet of both players in the information age, without research
/// (base numbers).
fn world(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.information").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: BTreeMap::new(),
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
    World::from_universe(
        rules.clone(),
        setup,
        120,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap()
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        body: HOME,
    }
}

fn spawn(w: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    w.spawn(id(w, key), owner, pos).unwrap()
}

fn order(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    entity: Option<EntityId>,
    pos: Option<Pos>,
) {
    w.submit(Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: entity,
        target_pos: pos,
        queue: false,
    })
    .unwrap_or_else(|e| panic!("{kind}: {e}"));
}

/// Sets `unit`'s combat stance.
fn stance(w: &mut World, unit: EntityId, key: &str) {
    let owner = w.entity(unit).unwrap().owner;
    let stance = id(w, key);
    w.set_stance(owner, unit, stance).unwrap();
}

/// Keeps `unit` where it stands: it fires only at what comes into its reach.
fn hold(w: &mut World, unit: EntityId) {
    stance(w, unit, "stance.stand_ground");
}

/// Steps until `done`, at most `limit` ticks; fails when it is not reached.
fn run_until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "not reached in {limit} ticks (tick {})", w.tick());
}

fn drones(w: &World, operator: EntityId) -> u32 {
    w.drone_cloud(operator).unwrap().drones
}

fn distance(a: Pos, b: Pos) -> u64 {
    a.distance(b)
}

/// The owner's numbers: one drone kills an infantry unit of the operator's age, and about ten
/// destroy the main battle tank of that age. Counted by striking with real drones.
#[test]
fn one_drone_kills_infantry_and_about_ten_destroy_a_main_battle_tank() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        let capacity = w.cloud_rules(id(&w, OPERATOR)).unwrap().capacity as u32;
        assert_eq!(drones(&w, operator), capacity, "trained with a full cloud");
        let infantry = spawn(&mut w, "unit.assault_infantry", P2, at(18_000, 32_000));
        // Hold the infantry where it is: it would walk to the operator and fire back.
        hold(&mut w, infantry);
        order(&mut w, operator, "attack", Some(infantry), None);
        run_until(&mut w, 600, |w| w.entity(infantry).is_none());
        let for_infantry = capacity - drones(&w, operator);
        let tank = spawn(&mut w, "unit.main_battle_tank", P2, at(16_000, 38_000));
        let full = drones(&w, operator);
        order(&mut w, operator, "attack", Some(tank), None);
        run_until(&mut w, 1200, |w| w.entity(tank).is_none());
        let for_tank = full - drones(&w, operator);
        (for_infantry, for_tank, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    let (for_infantry, for_tank, _) = first;
    println!("drones: infantry {for_infantry}, main battle tank {for_tank}");
    assert_eq!(
        for_infantry, 1,
        "one drone kills an infantry unit of its age"
    );
    assert!(
        (9..=11).contains(&for_tank),
        "about ten for a tank: {for_tank}"
    );
}

/// On a move the cloud flies ahead and the operator follows; the cloud never leaves its leash.
#[test]
fn the_cloud_flies_ahead_on_its_leash() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        let leash = w.cloud_rules(id(&w, OPERATOR)).unwrap().leash as u64;
        let goal = at(40_000, 32_000);
        order(&mut w, operator, "move", None, Some(goal));
        let mut ahead = 0;
        for _ in 0..600 {
            w.step();
            let me = w.entity(operator).unwrap().pos;
            let cloud = w.drone_cloud(operator).unwrap().pos;
            assert!(
                distance(me, cloud) <= leash + 1,
                "the cloud keeps its leash"
            );
            if distance(cloud, goal) + 2_000 < distance(me, goal) {
                ahead += 1;
            }
        }
        assert!(ahead > 100, "the cloud flies ahead: {ahead} ticks");
        // Arrived and idle, it gathers over the operator again.
        run_until(&mut w, 2000, |w| {
            w.entity(operator).unwrap().orders().is_empty()
        });
        for _ in 0..100 {
            w.step();
        }
        let me = w.entity(operator).unwrap().pos;
        assert!(distance(me, w.drone_cloud(operator).unwrap().pos) < 500);
        w.hash()
    };
    assert_eq!(play(), play());
}

/// On an attack the operator stops at its reach from the target, short of the target's own
/// reach, while the cloud strikes; spent drones come back one by one.
#[test]
fn the_operator_keeps_its_distance_and_issues_new_drones() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let kind = id(&w, OPERATOR);
        let cloud_rules = w.cloud_rules(kind).unwrap().clone();
        let reach = w.rules().entity_type(kind).unwrap().weapons[0].range_max as u64;
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        // A scout that does not fight shows the operator its targets.
        let scout = spawn(&mut w, "unit.recon_drone", P1, at(20_000, 31_000));
        stance(&mut w, scout, "stance.hold_fire");
        let mut targets = Vec::new();
        for i in 0..3 {
            let t = spawn(
                &mut w,
                "unit.assault_infantry",
                P2,
                at(30_000, 30_000 + i * 2_000),
            );
            hold(&mut w, t);
            targets.push(t);
        }
        for t in &targets {
            w.submit(Command {
                player: P1,
                executor: operator,
                kind: "attack".into(),
                subject: None,
                target_entity: Some(*t),
                target_pos: None,
                queue: true,
            })
            .unwrap();
        }
        let infantry_reach = w
            .rules()
            .entity_type(id(&w, "unit.assault_infantry"))
            .unwrap()
            .weapons[0]
            .range_max as u64;
        run_until(&mut w, 2000, |w| {
            targets.iter().all(|t| w.entity(*t).is_none())
        });
        let me = w.entity(operator).unwrap();
        assert_eq!(me.hp, me.max_hp, "never in reach of the riflemen");
        let stood = me.pos;
        assert!(
            distance(stood, at(30_000, 30_000)) > infantry_reach,
            "the operator stayed beyond the infantry's reach"
        );
        assert!(distance(stood, at(30_000, 30_000)) <= reach + 1_000);
        let left = drones(&w, operator);
        assert!(left < cloud_rules.capacity as u32);
        // A new drone every rebuild_ticks until the cloud is full.
        let before = w.tick();
        run_until(&mut w, 4000, |w| {
            drones(w, operator) == cloud_rules.capacity as u32
        });
        let took = w.tick() - before;
        let missing = cloud_rules.capacity as u64 - u64::from(left);
        assert!(took >= (missing - 1) * cloud_rules.rebuild_ticks as u64);
        assert!(took <= missing * cloud_rules.rebuild_ticks as u64 + 1);
        w.hash()
    };
    assert_eq!(play(), play());
}

/// Weapons that hit the air thin the cloud, never the operator; weapons that hit only the ground
/// must find the operator; an empty cloud leaves anti-air nothing to aim at.
#[test]
fn anti_air_thins_the_cloud_and_ground_weapons_find_the_operator() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        stance(&mut w, operator, "stance.hold_fire");
        let launcher = spawn(&mut w, "unit.missile_launcher", P2, at(14_000, 32_000));
        let tank = spawn(&mut w, "unit.main_battle_tank", P2, at(10_000, 50_000));
        assert!(
            w.can_target(launcher, operator),
            "anti-air aims at the cloud"
        );
        assert!(
            w.can_target(tank, operator),
            "ground weapons aim at the operator"
        );
        order(&mut w, launcher, "attack", Some(operator), None);
        let full = drones(&w, operator);
        run_until(&mut w, 1200, |w| drones(w, operator) == 0);
        let me = w.entity(operator).unwrap();
        assert_eq!(me.hp, me.max_hp, "anti-air never touches the operator");
        assert!(full > 0);
        assert!(!w.can_target(launcher, operator), "nothing left to aim at");
        w.hash()
    };
    assert_eq!(play(), play());
}

/// An operator is spotted where its cloud flies even when the operator stands out of sight.
#[test]
fn an_operator_is_seen_where_its_cloud_is() {
    let rules = rules();
    let mut w = world(&rules);
    let operator = spawn(&mut w, OPERATOR, P1, at(5_000, 32_000));
    let watcher = spawn(&mut w, "unit.missile_launcher", P2, at(25_000, 32_000));
    stance(&mut w, watcher, "stance.hold_fire");
    let sight = w.entity(watcher).unwrap().los as u64;
    order(&mut w, operator, "move", None, Some(at(24_000, 32_000)));
    let seen = |w: &World| {
        w.visible_entities(P2)
            .iter()
            .any(|s| matches!(s, rom_sim::Sighting::Live(e) if e.id == operator))
    };
    run_until(&mut w, 1500, seen);
    let me = w.entity(operator).unwrap().pos;
    assert!(
        distance(me, at(25_000, 32_000)) > sight,
        "seen by its cloud alone"
    );
}

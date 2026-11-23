//! Drone operators (tasks 120 and 128): one infantry unit whose square grid of kamikaze drones
//! hovers over it, flies ahead of it and deploys on its target, sends one drone per enemy, is
//! replenished by the operator, and loses one drone per hit from weapons that hit the air while
//! the operator is a target of its own. Every scenario runs twice and compares state hashes.

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
    w.drone_cloud(operator).unwrap().drones()
}

fn distance(a: Pos, b: Pos) -> u64 {
    a.distance(b)
}

/// Steps until `done` (at most `limit` ticks) and counts the drones that leave their slots of
/// `operator`'s grid on the way: with nothing shooting at the grid, the drones it sent.
fn sent_until(w: &mut World, operator: EntityId, limit: u64, done: impl Fn(&World) -> bool) -> u32 {
    let mut sent = 0;
    for _ in 0..limit {
        if done(w) {
            return sent;
        }
        let before = w.drone_cloud(operator).unwrap().ready;
        w.step();
        let after = w.drone_cloud(operator).map_or(0, |c| c.ready);
        sent += (before & !after).count_ones();
    }
    assert!(done(w), "not reached in {limit} ticks (tick {})", w.tick());
    sent
}

/// The grid is square, ⌈√capacity⌉ a side, its slots `spacing` apart row by row from the front;
/// at rest it hovers over its operator and after a move it faces the way it flew.
#[test]
fn the_grid_is_a_square_hovering_over_its_operator() {
    let rules = rules();
    let cloud_rules = rules
        .entity_type(rules.id_of(OPERATOR).unwrap())
        .unwrap()
        .drone_cloud
        .clone()
        .unwrap();
    assert_eq!(cloud_rules.side(), 4);
    let nine = rom_rules::DroneCloud {
        capacity: 9,
        ..cloud_rules.clone()
    };
    assert_eq!(nine.side(), 3);
    assert_eq!(nine.slot_offset(0), (600, -600));
    assert_eq!(nine.slot_offset(4), (0, 0));
    assert_eq!(nine.slot_offset(8), (-600, 600));
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(20_000, 32_000));
        let me = w.entity(operator).unwrap().pos;
        let spacing = cloud_rules.spacing as u64;
        let check = |w: &World, centre: Pos| {
            let drones = w.drones_of(operator);
            assert_eq!(drones.len(), 16);
            for (slot, pos) in &drones {
                let (ahead, right) = cloud_rules.slot_offset(u32::from(*slot));
                let away = ((ahead * ahead + right * right) as f64).sqrt() as u64;
                assert!(distance(centre, *pos).abs_diff(away) <= 2, "slot {slot}");
            }
            // Neighbours along a row and down a column are `spacing` apart.
            for (slot, pos) in &drones {
                if slot % 4 != 3 {
                    let next = drones[usize::from(*slot) + 1].1;
                    assert!(distance(*pos, next).abs_diff(spacing) <= 2);
                }
                if *slot < 12 {
                    let below = drones[usize::from(*slot) + 4].1;
                    assert!(distance(*pos, below).abs_diff(spacing) <= 2);
                }
            }
            drones
        };
        assert_eq!(w.drone_cloud(operator).unwrap().pos, me);
        check(&w, me);
        // Moved east, it faces east: the front row (slots 0 to 3) is the easternmost.
        let goal = at(30_000, 32_000);
        order(&mut w, operator, "move", None, Some(goal));
        w.step();
        run_until(&mut w, 3000, |w| {
            w.entity(operator).unwrap().orders().is_empty()
        });
        for _ in 0..60 {
            w.step();
        }
        let cloud = *w.drone_cloud(operator).unwrap();
        let me = w.entity(operator).unwrap().pos;
        assert!(distance(me, cloud.pos) < 100, "at rest over the operator");
        assert!(
            cloud.facing <= 10 || cloud.facing >= 350,
            "{}",
            cloud.facing
        );
        let drones = check(&w, cloud.pos);
        assert!(drones[0].1.x > me.x + 800 && drones[0].1.y < me.y - 800);
        assert!(drones[15].1.x < me.x - 800 && drones[15].1.y > me.y + 800);
        w.hash()
    };
    assert_eq!(play(), play());
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
        assert_eq!(drones(&w, operator), capacity, "trained with a full grid");
        let infantry = spawn(&mut w, "unit.assault_infantry", P2, at(18_000, 32_000));
        // Hold the infantry where it is: it would walk to the operator and fire back.
        hold(&mut w, infantry);
        order(&mut w, operator, "attack", Some(infantry), None);
        let for_infantry = sent_until(&mut w, operator, 600, |w| w.entity(infantry).is_none());
        let tank = spawn(&mut w, "unit.main_battle_tank", P2, at(16_000, 38_000));
        order(&mut w, operator, "attack", Some(tank), None);
        let for_tank = sent_until(&mut w, operator, 1200, |w| w.entity(tank).is_none());
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

/// One drone per enemy: against a squad within reach of the grid, the drones in the air are
/// reserved for their targets, so each soldier gets one and no drone is wasted; several fly at
/// once.
#[test]
fn one_drone_goes_to_each_infantry_target() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        let squad: Vec<EntityId> = [
            (18_000, 31_600),
            (18_400, 32_000),
            (17_800, 32_400),
            (18_300, 32_500),
        ]
        .into_iter()
        .map(|(x, y)| {
            let soldier = spawn(&mut w, "unit.assault_infantry", P2, at(x, y));
            hold(&mut w, soldier);
            soldier
        })
        .collect();
        order(&mut w, operator, "attack", Some(squad[0]), None);
        let mut most_in_air = 0;
        let mut sent = 0;
        for _ in 0..1200 {
            if squad.iter().all(|s| w.entity(*s).is_none()) {
                break;
            }
            let before = w.drone_cloud(operator).unwrap().ready;
            w.step();
            sent += (before & !w.drone_cloud(operator).unwrap().ready).count_ones();
            let in_air = w
                .hits_in_flight()
                .filter(|h| h.attacker == operator)
                .count();
            most_in_air = most_in_air.max(in_air);
        }
        assert!(
            squad.iter().all(|s| w.entity(*s).is_none()),
            "the squad falls"
        );
        assert_eq!(sent, squad.len() as u32, "one drone for each soldier");
        assert!(
            most_in_air >= 2,
            "several drones in the air at once: {most_in_air}"
        );
        let me = w.entity(operator).unwrap();
        assert_eq!(me.hp, me.max_hp);
        w.hash()
    };
    assert_eq!(play(), play());
}

/// On a move the grid flies ahead and the operator follows; the grid never leaves its leash.
#[test]
fn the_grid_flies_ahead_on_its_leash() {
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
            assert!(distance(me, cloud) <= leash + 1, "the grid keeps its leash");
            if distance(cloud, goal) + 2_000 < distance(me, goal) {
                ahead += 1;
            }
        }
        assert!(ahead > 100, "the grid flies ahead: {ahead} ticks");
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
/// reach, while the grid strikes; spent drones come back one by one, each taking off from the
/// operator for the first empty slot.
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
        // A new drone every rebuild_ticks until the grid is full; each takes off from the
        // operator for the first empty slot and settles there.
        let capacity = cloud_rules.capacity as u32;
        let before = w.tick();
        let mut joined = 0;
        let mut was: Option<rom_sim::Joining> = None;
        let mut full_at = None;
        while full_at.is_none() || was.is_some() {
            assert!(w.tick() - before < 4000, "the grid fills");
            let me = w.entity(operator).unwrap().pos;
            w.step();
            let cloud = *w.drone_cloud(operator).unwrap();
            if let Some(j) = cloud.joining
                && was.is_none_or(|was| was.slot != j.slot)
            {
                assert_eq!(j.pos, me, "a new drone takes off from the operator");
                assert_eq!(
                    u32::from(j.slot),
                    (!cloud.ready).trailing_zeros(),
                    "for the first empty slot"
                );
                joined += 1;
            }
            if let Some(j) = was.filter(|j| cloud.joining.is_none_or(|n| n.slot != j.slot)) {
                assert!(cloud.ready >> j.slot & 1 == 1, "it settles in its slot");
            }
            was = cloud.joining;
            if full_at.is_none() && cloud.drones() == capacity {
                full_at = Some(w.tick());
            }
        }
        let took = full_at.unwrap() - before;
        let missing = u64::from(capacity - left);
        assert_eq!(joined, missing);
        assert!(took >= (missing - 1) * cloud_rules.rebuild_ticks as u64);
        assert!(took <= missing * cloud_rules.rebuild_ticks as u64 + 1);
        assert_eq!(
            w.drone_cloud(operator).unwrap().ready.count_ones(),
            capacity
        );
        w.hash()
    };
    assert_eq!(play(), play());
}

/// A weapon that hits only the air aims at the drones, one at a time, and each hit brings one
/// down whatever its damage; it never touches the operator, and an empty grid leaves it nothing
/// to aim at. Weapons that hit the ground reach the operator through its grid.
#[test]
fn one_hit_brings_down_one_drone_and_ground_weapons_reach_the_operator() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        stance(&mut w, operator, "stance.hold_fire");
        let launcher = spawn(&mut w, "unit.missile_launcher", P2, at(14_000, 32_000));
        let tank = spawn(&mut w, "unit.main_battle_tank", P2, at(10_000, 50_000));
        stance(&mut w, tank, "stance.hold_fire");
        assert!(
            w.can_target(launcher, operator),
            "anti-air aims at the drones"
        );
        assert!(
            w.can_target(tank, operator),
            "ground weapons aim at the operator"
        );
        order(&mut w, launcher, "attack", Some(operator), None);
        let mut downed = 0;
        for _ in 0..3000 {
            if drones(&w, operator) == 0 {
                break;
            }
            let cloud = *w.drone_cloud(operator).unwrap();
            w.step();
            let after = *w.drone_cloud(operator).unwrap();
            let lost =
                (cloud.ready & !after.ready).count_ones()
                    + u32::from(cloud.joining.is_some_and(|j| {
                        after.joining.is_none() && after.ready >> j.slot & 1 == 0
                    }));
            // One missile, one drone: the missile's 28 damage would have brought down three of
            // task 120's eight-hit-point drones.
            assert!(lost <= 1, "one hit, one drone: {lost}");
            downed += lost;
        }
        assert_eq!(drones(&w, operator), 0, "the grid is shot down");
        assert!(downed >= 16);
        let me = w.entity(operator).unwrap();
        assert_eq!(me.hp, me.max_hp, "anti-air never touches the operator");
        assert!(!w.can_target(launcher, operator), "nothing left to aim at");
        // The tank's gun reaches the operator, drones or none.
        order(&mut w, tank, "attack", Some(operator), None);
        run_until(&mut w, 1200, |w| {
            w.entity(operator).is_none_or(|e| e.hp < e.max_hp)
        });
        w.hash()
    };
    assert_eq!(play(), play());
}

/// A weapon that hits the ground and the air (a defence turret) firing by itself aims at the
/// nearer part: the drones flying ahead of an operator still out of its reach.
#[test]
fn a_weapon_that_hits_both_aims_at_the_nearer_part() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let turret = spawn(&mut w, "bld.defence_turret", P2, at(30_000, 32_000));
        let reach = w
            .rules()
            .entity_type(id(&w, "bld.defence_turret"))
            .unwrap()
            .weapons[0]
            .range_max as u64;
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        stance(&mut w, operator, "stance.hold_fire");
        order(&mut w, operator, "move", None, Some(at(40_000, 32_000)));
        let full = drones(&w, operator);
        run_until(&mut w, 400, |w| drones(w, operator) < full);
        let me = w.entity(operator).unwrap();
        let turret_pos = w.entity(turret).unwrap().pos;
        assert!(
            distance(me.pos, turret_pos) > reach,
            "the operator is out of its reach"
        );
        assert_eq!(me.hp, me.max_hp);
        w.hash()
    };
    assert_eq!(play(), play());
}

/// The operator's end (here disbanded; death takes the same way out) ends the unit, its grid
/// and the drones it sent.
#[test]
fn the_operators_end_ends_its_grid_and_its_drones_in_the_air() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let operator = spawn(&mut w, OPERATOR, P1, at(10_000, 32_000));
        let tank = spawn(&mut w, "unit.main_battle_tank", P2, at(17_000, 32_000));
        hold(&mut w, tank);
        order(&mut w, operator, "attack", Some(tank), None);
        run_until(&mut w, 600, |w| {
            w.hits_in_flight().any(|h| h.attacker == operator)
        });
        let hp = w.entity(tank).unwrap().hp;
        order(&mut w, operator, "disband", None, None);
        w.step();
        assert!(w.entity(operator).is_none());
        assert!(w.drone_cloud(operator).is_none(), "its grid is gone");
        assert!(w.hits_in_flight().all(|h| h.attacker != operator));
        for _ in 0..40 {
            w.step();
        }
        assert_eq!(w.entity(tank).unwrap().hp, hp, "no drone of it lands");
        w.hash()
    };
    assert_eq!(play(), play());
}

/// An operator is spotted where its grid flies even when the operator stands out of sight.
#[test]
fn an_operator_is_seen_where_its_grid_is() {
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
        "seen by its grid alone"
    );
}

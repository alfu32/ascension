//! The commands of task 142: `delete` (a construction site refunds the work not done, a queue
//! comes back, workers inside leave), `attack_ground` (weapons that throw a projectile fire at
//! a point) and a spaceport's rally point for the ships it launches. Each scenario runs twice
//! and compares state hashes; the long ones also restore a snapshot half-way.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, MapGrid, PlanetSurface, Player, PlayerId, Pos, Rally,
    SimError, SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const ENEMY: PlayerId = PlayerId(2);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// A 64 × 64 plain, with deep water from x = 30 to 33 when `water` is set.
fn world(rules: &Arc<Rules>, water: bool) -> World {
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        age: rules.id_of("age.ancient").unwrap(),
        faction: rules.id_of("nation.romans").unwrap(),
        resources: BTreeMap::new(),
        techs: BTreeMap::new(),
    };
    let plain = rules.id_of("ter.plain").unwrap();
    let deep = rules.id_of("ter.deep").unwrap();
    let terrain = (0..64 * 64)
        .map(|i| {
            if water && (30..34).contains(&(i % 64)) {
                deep
            } else {
                plain
            }
        })
        .collect();
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(64, 64, terrain).unwrap(),
        142,
        vec![player(0), player(1), player(2)],
    )
    .unwrap();
    for resource in rules.resources.keys().copied().collect::<Vec<_>>() {
        world.grant(OWNER, resource, 100_000).unwrap();
    }
    world
}

fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
}

fn spawn(world: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    world.spawn(id(world, key), owner, pos).unwrap()
}

fn order(executor: EntityId, kind: &str) -> Command {
    Command {
        player: OWNER,
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

fn at(executor: EntityId, kind: &str, pos: Pos) -> Command {
    Command {
        target_pos: Some(pos),
        ..order(executor, kind)
    }
}

fn stock(world: &World, resource: DefId) -> i64 {
    world
        .players()
        .find(|p| p.id == OWNER)
        .and_then(|p| p.resources.get(&resource).copied())
        .unwrap_or(0)
}

fn stocks(world: &World) -> Vec<i64> {
    let resources: Vec<DefId> = world.rules().resources.keys().copied().collect();
    resources.into_iter().map(|r| stock(world, r)).collect()
}

fn ticks(world: &mut World, n: u64) {
    for _ in 0..n {
        world.step();
    }
}

fn hp(world: &World, id: EntityId) -> i64 {
    world.entity(id).map_or(0, |e| e.hp)
}

/// Runs a scenario twice, and once more restoring a snapshot after `half` of its steps (the
/// scenario calls `restore` there); all three end in the same state.
fn thrice(run: impl Fn(&dyn Fn(World) -> World) -> World) -> World {
    let keep = |w: World| w;
    let restore = |w: World| World::restore(w.rules().clone(), w.snapshot()).unwrap();
    let (a, b, c) = (run(&keep), run(&keep), run(&restore));
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.hash(), c.hash());
    assert_eq!(a.snapshot(), b.snapshot());
    a
}

#[test]
fn deleting_a_site_refunds_its_price_for_the_work_not_done() {
    let r = rules("");
    thrice(|restore| {
        let mut w = world(&r, false);
        spawn(&mut w, "bld.city", OWNER, Pos::flat(20_500, 20_500));
        let builder = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(30_500, 20_500));
        let kind = id(&w, "bld.farm");
        let before = stocks(&w);
        let mut build = at(builder, "construct", Pos::flat(27_000, 26_000));
        build.subject = Some(kind);
        w.submit(build).unwrap();
        let paid: Vec<i64> = before.iter().zip(stocks(&w)).map(|(b, a)| b - a).collect();
        assert!(paid.iter().any(|p| *p > 0));
        ticks(&mut w, 60);
        let mut w = restore(w);
        ticks(&mut w, 60);
        let site = w.entities().find(|e| e.type_id == kind).unwrap();
        let (site, done) = (site.id, site.build_progress.expect("unfinished") as i64);
        let total = w.rules().build_ticks(kind, &w.active_sources(OWNER));
        assert!(done > 0 && done < total, "{done} of {total}");
        let promised = w.delete_refund(site);
        let held = stocks(&w);
        w.submit(order(site, "delete")).unwrap();
        w.step();
        assert!(w.entity(site).is_none());
        // Cost × (1 − progress), rounded down, per resource; what the views promised.
        let back: Vec<i64> = stocks(&w).iter().zip(&held).map(|(a, b)| a - b).collect();
        let expected: Vec<i64> = paid.iter().map(|p| p * (total - done) / total).collect();
        assert_eq!(back, expected);
        let resources: Vec<DefId> = w.rules().resources.keys().copied().collect();
        let promised: Vec<i64> = resources
            .iter()
            .map(|r| promised.iter().find(|(p, _)| p == r).map_or(0, |(_, a)| *a))
            .collect();
        assert_eq!(back, promised);
        // The builder's work ended with its site; nobody scored a kill.
        assert!(w.stats().kills.is_empty());
        w
    });
}

#[test]
fn deleting_a_finished_building_refunds_its_queue_and_lets_its_workers_go() {
    let r = rules("");
    let w = thrice(|restore| {
        let mut w = world(&r, false);
        let city = spawn(&mut w, "bld.city", OWNER, Pos::flat(20_500, 20_500));
        let barracks = spawn(&mut w, "bld.barracks", OWNER, Pos::flat(30_500, 20_500));
        let farm = spawn(&mut w, "bld.farm", OWNER, Pos::flat(20_500, 30_500));
        let workers: Vec<EntityId> = (0..3)
            .map(|i| {
                spawn(
                    &mut w,
                    "unit.citizen",
                    OWNER,
                    Pos::flat(22_500 + i * 500, 32_500),
                )
            })
            .collect();
        for worker in &workers {
            w.submit(Command {
                target_entity: Some(farm),
                ..order(*worker, "harvest")
            })
            .unwrap();
        }
        ticks(&mut w, 200);
        let mut w = restore(w);
        assert!(!w.occupants(farm).is_empty());
        let held = stocks(&w);
        for _ in 0..3 {
            let mut train = order(barracks, "train");
            train.subject = Some(id(&w, "unit.spearman"));
            train.queue = true;
            w.submit(train).unwrap();
        }
        assert_ne!(stocks(&w), held);
        ticks(&mut w, 20);
        let gathered_before = stocks(&w);
        w.submit(order(barracks, "delete")).unwrap();
        w.submit(order(farm, "delete")).unwrap();
        w.step();
        assert!(w.entity(barracks).is_none() && w.entity(farm).is_none());
        // The three trainings came back in full; a finished building gives nothing more.
        let spearman = w.price(OWNER, id(&w, "unit.spearman"));
        for (n, resource) in w.rules().resources.keys().enumerate() {
            let cost = spearman
                .iter()
                .find(|(r, _)| r == resource)
                .map_or(0, |(_, a)| *a);
            assert!(
                stocks(&w)[n] - gathered_before[n] >= 3 * cost,
                "{resource:?}: {} of {}",
                stocks(&w)[n] - gathered_before[n],
                3 * cost
            );
        }
        // The workers came out alive and idle.
        for worker in &workers {
            let e = w.entity(*worker).expect("worker lives");
            assert!(!e.is_inside());
        }
        // A finished city stays; a unit goes at once, several in one tick.
        assert_eq!(w.submit(order(city, "delete")), Err(SimError::CannotDelete));
        assert!(!w.can_delete(city));
        for worker in &workers {
            w.submit(order(*worker, "delete")).unwrap();
        }
        w.step();
        assert!(workers.iter().all(|u| w.entity(*u).is_none()));
        // Only the owner's: deleting is not a weapon.
        let foe = spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(50_500, 50_500));
        assert!(w.submit(order(foe, "delete")).is_err());
        w
    });
    assert!(w.stats().kills.is_empty());
    assert_eq!(w.stains(BodyId(0)).count(), 0);
}

#[test]
fn only_projectile_weapons_attack_the_ground_and_only_where_they_can_hit() {
    let r = rules("");
    let mut w = world(&r, true);
    let ground = Pos::flat(20_500, 20_500);
    let water = Pos::flat(31_500, 20_500);
    let catapult = spawn(&mut w, "unit.catapult", OWNER, Pos::flat(10_500, 20_500));
    let archer = spawn(&mut w, "unit.archer", OWNER, Pos::flat(10_500, 22_500));
    let spearman = spawn(&mut w, "unit.spearman", OWNER, Pos::flat(10_500, 24_500));
    let beam = spawn(
        &mut w,
        "unit.beam_trooper",
        OWNER,
        Pos::flat(10_500, 26_500),
    );
    let citizen = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(10_500, 28_500));
    assert!(w.submit(at(catapult, "attack_ground", ground)).is_ok());
    assert!(w.submit(at(catapult, "attack_ground", water)).is_ok());
    assert!(w.submit(at(archer, "attack_ground", ground)).is_ok());
    // An archer's arrows reach the ground layer, not ships on the water.
    assert_eq!(
        w.submit(at(archer, "attack_ground", water)),
        Err(SimError::NoGroundWeapon)
    );
    // A blow and an instant beam are not thrown.
    assert_eq!(
        w.submit(at(spearman, "attack_ground", ground)),
        Err(SimError::NoGroundWeapon)
    );
    assert_eq!(
        w.submit(at(beam, "attack_ground", ground)),
        Err(SimError::NoGroundWeapon)
    );
    assert!(w.submit(at(citizen, "attack_ground", ground)).is_err());
    assert!(w.attacks_ground(id(&w, "unit.catapult")));
    assert!(!w.attacks_ground(id(&w, "unit.spearman")));
    assert!(!w.attacks_ground(id(&w, "unit.beam_trooper")));
}

/// Catapults sent at a point out of their reach walk up and bombard it until stopped: an enemy
/// standing there is hit, one beside it is not (no splash), and the order never ends by itself.
#[test]
fn catapults_walk_into_range_and_bombard_the_point() {
    let r = rules("");
    thrice(|restore| {
        let mut w = world(&r, false);
        let point = Pos::flat(50_500, 20_500);
        let catapults: Vec<EntityId> = (0..2)
            .map(|i| {
                spawn(
                    &mut w,
                    "unit.catapult",
                    OWNER,
                    Pos::flat(8_500, 18_500 + i * 2_000),
                )
            })
            .collect();
        // Enemies that cannot strike back: a farm stands on the point, a citizen beside it.
        let farm = spawn(&mut w, "bld.farm", ENEMY, point);
        let aside = spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(50_500, 26_500));
        let full = (hp(&w, farm), hp(&w, aside));
        for c in &catapults {
            w.submit(at(*c, "attack_ground", point)).unwrap();
        }
        ticks(&mut w, 300);
        let mut w = restore(w);
        ticks(&mut w, 600);
        let reach = w.rules().entity_types[&id(&w, "unit.catapult")].weapons[0].range_max as u64;
        for c in &catapults {
            let e = w.entity(*c).unwrap();
            assert_eq!(e.orders()[0].kind, "attack_ground", "still at it");
            let d = w.distance(e.pos, point).unwrap();
            assert!(d <= reach, "{d} within {reach}");
        }
        assert!(hp(&w, farm) < full.0, "the farm on the point is hit");
        assert_eq!(hp(&w, aside), full.1, "the citizen beside it is not");
        for c in &catapults {
            w.submit(order(*c, "stop")).unwrap();
        }
        w.step();
        assert!(
            catapults
                .iter()
                .all(|c| w.entity(*c).unwrap().orders().is_empty())
        );
        w
    });
}

/// With a splash (a rules change for the test), a shot at the ground hurts every enemy around the
/// point and no own unit.
#[test]
fn a_shot_at_the_ground_splashes_enemies_and_spares_own_units() {
    let r = rules(
        "UPDATE et_weapon SET splash_radius=3000, splash_pct=100 \
         WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.catapult');",
    );
    thrice(|restore| {
        let mut w = world(&r, false);
        let point = Pos::flat(30_500, 30_500);
        let catapult = spawn(&mut w, "unit.catapult", OWNER, Pos::flat(24_500, 30_500));
        let foes: Vec<EntityId> = [(30_500, 31_500), (31_500, 30_500)]
            .into_iter()
            .map(|(x, y)| spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(x, y)))
            .collect();
        let own = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(29_500, 30_500));
        let far = spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(30_500, 40_500));
        // Nobody fights back: what hurts them is the catapult.
        let hold = id(&w, "stance.hold_fire");
        for foe in foes.iter().chain([&far]) {
            w.set_stance(ENEMY, *foe, hold).unwrap();
        }
        w.set_stance(OWNER, own, hold).unwrap();
        let full = hp(&w, own);
        w.submit(at(catapult, "attack_ground", point)).unwrap();
        ticks(&mut w, 100);
        let mut w = restore(w);
        ticks(&mut w, 200);
        assert!(foes.iter().all(|f| hp(&w, *f) < full), "both foes hurt");
        assert_eq!(hp(&w, own), full, "own unit spared");
        assert_eq!(hp(&w, far), full, "out of the splash");
        assert!(!w.blasts().is_empty());
        w
    });
}

const HOME: BodyId = BodyId(2);
const N: u32 = 96;

fn on(face: u8, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face,
        body: HOME,
    }
}

/// One system with a home planet of 96 tiles a face, everyone in the starfaring age.
fn universe(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.starfaring").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: BTreeMap::new(),
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
        planets: BTreeMap::from([(
            HOME,
            PlanetSurface {
                surface: CubeSurface { n: N },
                terrain: vec![rules.id_of("ter.plain").unwrap(); 6 * (N * N) as usize],
                nodes: vec![],
            },
        )]),
    };
    World::from_universe(
        rules.clone(),
        setup,
        142,
        players,
        BTreeMap::from([(OWNER, HOME), (ENEMY, HOME)]),
    )
    .unwrap()
}

/// Ships leaving a spaceport's queue go to its rally point as units leaving a barracks do: a
/// satellite (bound to its orbit) and a corvette (the survey ship until task 155 moved it one age
/// later) both move within the orbit to above the point on the ground, and stay in orbit there.
#[test]
fn ships_launched_from_a_spaceport_go_to_its_rally_point_in_orbit() {
    let r = rules("");
    thrice(|restore| {
        let mut w = universe(&r);
        let port = w
            .place_for_setup(id(&w, "bld.spaceport"), OWNER, on(0, 30_000, 30_000))
            .unwrap();
        let rally = on(0, 60_500, 50_500);
        w.submit(Command {
            target_pos: Some(rally),
            ..order(port, "set_rally")
        })
        .unwrap();
        for key in ["unit.satellite", "unit.corvette"] {
            let mut train = order(port, "train");
            train.subject = Some(id(&w, key));
            train.queue = true;
            w.submit(train).unwrap();
        }
        ticks(&mut w, 500);
        let mut w = restore(w);
        assert_eq!(w.rally(port), Some(Rally::Point(rally)));
        let mut arrived = 0;
        for _ in 0..4000 {
            w.step();
            arrived = ["unit.satellite", "unit.corvette"]
                .iter()
                .filter(|key| {
                    w.entities().any(|e| {
                        e.type_id == id(&w, key)
                            && w.orbit(e.id).is_some_and(|o| {
                                w.distance(o.pos, rally).is_some_and(|d| d <= 1_000)
                            })
                    })
                })
                .count();
            if arrived == 2 {
                break;
            }
        }
        assert_eq!(arrived, 2, "both ships hold orbit above the rally point");
        w
    });
}

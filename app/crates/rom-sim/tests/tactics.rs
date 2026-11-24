//! Orders added for computer players (task 65): `explore`, `patrol`, `guard`, `stance`,
//! `raze`, formation pace, the report of failed orders and terrain reachability. Each runs
//! twice from the same start and must end in the same state.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const ENEMY: PlayerId = PlayerId(2);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// A 64 × 64 plain with a strip of deep water from x = 30 to 33 when `water` is set.
fn world(rules: Arc<Rules>, water: bool) -> World {
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
        Arc::clone(&rules),
        MapGrid::new(64, 64, terrain).unwrap(),
        11,
        vec![player(0), player(1), player(2)],
    )
    .unwrap();
    let resources: Vec<_> = rules.resources.keys().copied().collect();
    for resource in resources {
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
    }
}

fn stock(world: &World, resource: DefId) -> i64 {
    world
        .players()
        .find(|p| p.id == OWNER)
        .and_then(|p| p.resources.get(&resource).copied())
        .unwrap_or(0)
}

fn ticks(world: &mut World, n: u64) {
    for _ in 0..n {
        world.step();
    }
}

/// Builds a scenario twice and checks both runs end alike.
fn twice(run: impl Fn() -> World) -> World {
    let (a, b) = (run(), run());
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.snapshot(), b.snapshot());
    a
}

fn explored(world: &World) -> usize {
    (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .filter(|(x, y)| world.is_explored_on_body(OWNER, Pos::flat(0, 0).body, 0, *x, *y))
        .count()
}

#[test]
fn explore_walks_to_unseen_ground_until_none_is_left() {
    let w = twice(|| {
        let mut w = world(rules(), false);
        let scout = spawn(&mut w, "unit.scout", OWNER, Pos::flat(8_500, 8_500));
        w.step();
        let before = explored(&w);
        w.submit(order(scout, "explore")).unwrap();
        ticks(&mut w, 400);
        assert!(explored(&w) > before + 200, "{before} → {}", explored(&w));
        let order = &w.entity(scout).unwrap().orders()[0];
        assert_eq!(order.kind, "explore");
        assert!(order.target_pos.is_some());
        ticks(&mut w, 6000);
        w
    });
    // The whole plain has been seen and the order is done.
    assert_eq!(explored(&w), 64 * 64);
    let scout = w.entities().find(|e| e.owner == OWNER).unwrap();
    assert!(scout.orders().is_empty());
}

#[test]
fn explore_keeps_to_ground_it_can_reach() {
    let w = twice(|| {
        let mut w = world(rules(), true);
        let scout = spawn(&mut w, "unit.scout", OWNER, Pos::flat(8_500, 30_500));
        w.submit(order(scout, "explore")).unwrap();
        ticks(&mut w, 6000);
        w
    });
    let scout = w.entities().find(|e| e.owner == OWNER).unwrap();
    assert!(scout.orders().is_empty(), "{:?}", scout.orders());
    assert!(scout.pos.x < 30_000);
    assert_eq!(w.order_failures(OWNER).count(), 0);
}

#[test]
fn reachability_follows_the_terrain() {
    let w = world(rules(), true);
    let spearman = id(&w, "unit.spearman");
    let west = Pos::flat(10_500, 10_500);
    assert_eq!(
        w.reachable(spearman, west, Pos::flat(25_500, 60_500)),
        Some(true)
    );
    assert_eq!(
        w.reachable(spearman, west, Pos::flat(40_500, 10_500)),
        Some(false)
    );
    // A building does not move: no answer.
    assert_eq!(w.reachable(id(&w, "bld.farm"), west, west), None);
}

#[test]
fn a_patrol_walks_back_and_forth_and_fights_on_the_way() {
    let w = twice(|| {
        let mut w = world(rules(), false);
        let unit = spawn(&mut w, "unit.spearman", OWNER, Pos::flat(10_500, 10_500));
        let far = Pos::flat(40_500, 10_500);
        let mut patrol = order(unit, "patrol");
        patrol.target_pos = Some(far);
        w.submit(patrol).unwrap();
        let mut ends = [false, false];
        for _ in 0..2400 {
            w.step();
            let at = w.entity(unit).unwrap().pos;
            ends[0] |= w.distance(at, Pos::flat(10_500, 10_500)).unwrap() < 1_500;
            ends[1] |= w.distance(at, far).unwrap() < 1_500;
        }
        assert!(ends[0] && ends[1], "{ends:?}");
        assert_eq!(w.entity(unit).unwrap().orders()[0].kind, "patrol");
        // An enemy beside the route is fought, and the patrol goes on afterwards.
        let enemy = spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(25_500, 12_500));
        ticks(&mut w, 1200);
        assert!(w.entity(enemy).is_none());
        assert_eq!(w.entity(unit).unwrap().orders()[0].kind, "patrol");
        w
    });
    assert!(w.stats().kills.get(&OWNER).copied().unwrap_or(0) >= 1);
}

/// The points of `spots` a unit comes within 1.5 tiles of, in the order it does, repeats
/// merged, for `steps` ticks.
fn visits(w: &mut World, unit: EntityId, spots: &[Pos], steps: u64) -> Vec<usize> {
    let mut seen: Vec<usize> = Vec::new();
    for _ in 0..steps {
        w.step();
        let at = w.entity(unit).unwrap().pos;
        if let Some(i) = spots
            .iter()
            .position(|p| w.distance(at, *p).unwrap() < 1_500)
            && seen.last() != Some(&i)
        {
            seen.push(i);
        }
    }
    seen
}

fn route_world() -> (World, EntityId, [Pos; 4]) {
    let mut w = world(rules(), false);
    let start = Pos::flat(10_500, 10_500);
    let unit = spawn(&mut w, "unit.spearman", OWNER, start);
    let spots = [
        start,
        Pos::flat(30_500, 10_500),
        Pos::flat(30_500, 30_500),
        Pos::flat(10_500, 30_500),
    ];
    for p in &spots[1..] {
        let mut patrol = order(unit, "patrol");
        patrol.target_pos = Some(*p);
        patrol.queue = true;
        w.submit(patrol).unwrap();
    }
    (w, unit, spots)
}

#[test]
fn queued_patrol_points_make_a_loop_walked_round_and_round() {
    let w = twice(|| {
        let (mut w, unit, spots) = route_world();
        let seen = visits(&mut w, unit, &spots, 6000);
        assert_eq!(seen[..9], [0, 1, 2, 3, 0, 1, 2, 3, 0], "{seen:?}");
        assert!(
            w.entity(unit)
                .unwrap()
                .orders()
                .iter()
                .all(|o| o.kind == "patrol")
        );
        assert_eq!(w.entity(unit).unwrap().orders().len(), 4);
        w
    });
    let _ = w;
}

#[test]
fn a_saved_loop_goes_on_from_the_middle() {
    let (mut w, unit, spots) = route_world();
    ticks(&mut w, 900);
    let mut restored = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(restored.hash(), w.hash());
    let (a, b) = (
        visits(&mut w, unit, &spots, 4000),
        visits(&mut restored, unit, &spots, 4000),
    );
    assert_eq!(a, b);
    assert!(a.len() >= 5, "{a:?}");
    assert_eq!(restored.hash(), w.hash());
}

#[test]
fn a_plain_order_replaces_the_route_and_a_queued_one_behind_it_is_refused() {
    let (mut w, unit, spots) = route_world();
    ticks(&mut w, 100);
    let mut late = order(unit, "move");
    late.target_pos = Some(Pos::flat(50_500, 50_500));
    late.queue = true;
    w.submit(late).unwrap();
    ticks(&mut w, 1);
    assert!(
        w.entity(unit)
            .unwrap()
            .orders()
            .iter()
            .all(|o| o.kind == "patrol")
    );
    let mut plain = order(unit, "move");
    plain.target_pos = Some(spots[2]);
    w.submit(plain).unwrap();
    ticks(&mut w, 2000);
    let e = w.entity(unit).unwrap();
    assert!(e.orders().is_empty());
    assert!(w.distance(e.pos, spots[2]).unwrap() < 1_500);
}

#[test]
fn a_guard_follows_its_ward_and_fights_off_what_comes_near() {
    let w = twice(|| {
        let mut w = world(rules(), false);
        let ward = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(10_500, 10_500));
        let guard = spawn(&mut w, "unit.spearman", OWNER, Pos::flat(14_500, 10_500));
        let mut command = order(guard, "guard");
        command.target_entity = Some(ward);
        w.submit(command).unwrap();
        let mut walk = order(ward, "move");
        walk.target_pos = Some(Pos::flat(30_500, 30_500));
        w.submit(walk).unwrap();
        ticks(&mut w, 900);
        let (g, c) = (w.entity(guard).unwrap().pos, w.entity(ward).unwrap().pos);
        assert!(w.distance(g, c).unwrap() <= 4_000, "{g:?} {c:?}");
        // What the guard can see coming near its ward is fought off.
        let enemy = spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(32_500, 30_500));
        ticks(&mut w, 600);
        assert!(w.entity(enemy).is_none());
        assert_eq!(w.entity(guard).unwrap().orders()[0].kind, "guard");
        // Guarding an enemy is refused.
        let other = spawn(&mut w, "unit.citizen", ENEMY, Pos::flat(50_500, 50_500));
        let mut bad = order(guard, "guard");
        bad.target_entity = Some(other);
        assert_eq!(w.submit(bad), Err(SimError::InvalidGuardTarget));
        w
    });
    assert!(w.stats().kills.get(&OWNER).copied().unwrap_or(0) >= 1);
}

#[test]
fn a_stance_is_an_order_applied_at_once() {
    let w = twice(|| {
        let mut w = world(rules(), false);
        let unit = spawn(&mut w, "unit.spearman", OWNER, Pos::flat(10_500, 10_500));
        let mut walk = order(unit, "move");
        walk.target_pos = Some(Pos::flat(40_500, 10_500));
        w.submit(walk).unwrap();
        w.step();
        let defensive = id(&w, "stance.defensive");
        let mut stance = order(unit, "stance");
        stance.subject = Some(defensive);
        w.submit(stance).unwrap();
        w.step();
        let e = w.entity(unit).unwrap();
        assert_eq!(e.stance, defensive);
        // The walk goes on.
        assert_eq!(e.orders()[0].kind, "move");
        let mut bad = order(unit, "stance");
        bad.subject = Some(id(&w, "stance.gather"));
        assert_eq!(w.submit(bad), Err(SimError::InvalidStance));
        w
    });
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn raze_gives_up_an_unfinished_building_with_a_refund() {
    twice(|| {
        let mut w = world(rules(), false);
        let city = spawn(&mut w, "bld.city", OWNER, Pos::flat(20_500, 20_500));
        let builder = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(30_500, 20_500));
        let farm = id(&w, "bld.farm");
        let timber = id(&w, "res.timber");
        let mut build = order(builder, "construct");
        build.subject = Some(farm);
        build.target_pos = Some(Pos::flat(32_000, 26_000));
        w.submit(build).unwrap();
        let paid = 100_000 - stock(&w, timber);
        assert!(paid > 0);
        w.step();
        let site = w
            .entities()
            .find(|e| e.type_id == farm)
            .map(|e| e.id)
            .unwrap();
        assert!(w.entity(site).unwrap().build_progress.is_some());
        w.submit(order(site, "raze")).unwrap();
        w.step();
        assert!(w.entity(site).is_none());
        // Hardly any work was done: about the whole price comes back.
        let back = stock(&w, timber) - (100_000 - paid);
        assert!(back * 100 >= paid * 95, "{back} of {paid}");
        // A finished city cannot be razed; it is captured instead.
        assert_eq!(w.submit(order(city, "raze")), Err(SimError::CannotRaze));
        // Neither can a unit.
        assert_eq!(w.submit(order(builder, "raze")), Err(SimError::CannotRaze));
        w
    });
}

#[test]
fn raze_pulls_down_a_finished_building() {
    twice(|| {
        let mut w = world(rules(), false);
        spawn(&mut w, "bld.city", OWNER, Pos::flat(20_500, 20_500));
        let tower = spawn(&mut w, "bld.tower", OWNER, Pos::flat(30_000, 30_000));
        w.submit(order(tower, "raze")).unwrap();
        w.step();
        assert!(w.entity(tower).is_none());
        w
    });
}

#[test]
fn a_formation_keeps_to_its_slowest_member() {
    let w = twice(|| {
        let mut w = world(rules(), false);
        let fast = spawn(&mut w, "unit.horseman", OWNER, Pos::flat(10_500, 10_500));
        let slow = spawn(&mut w, "unit.spearman", OWNER, Pos::flat(10_500, 12_500));
        let line = id(&w, "form.line");
        for (unit, y) in [(fast, 10_500), (slow, 12_500)] {
            let mut march = order(unit, "attack_move");
            march.subject = Some(line);
            march.target_pos = Some(Pos::flat(50_500, y));
            w.submit(march).unwrap();
        }
        ticks(&mut w, 200);
        let (a, b) = (w.entity(fast).unwrap().pos, w.entity(slow).unwrap().pos);
        // Side by side: the horseman walks at the spearman's pace.
        assert!((a.x - b.x).abs() <= 200, "{a:?} {b:?}");
        assert!(a.x > 15_000);
        let pace = w.entity(fast).unwrap().orders()[0].pace;
        assert!(pace.is_some());
        // Without a formation each walks at its own speed.
        let alone = spawn(&mut w, "unit.horseman", OWNER, Pos::flat(10_500, 40_500));
        let mut march = order(alone, "move");
        march.target_pos = Some(Pos::flat(50_500, 40_500));
        w.submit(march).unwrap();
        ticks(&mut w, 100);
        assert!(w.entity(alone).unwrap().orders()[0].pace.is_none());
        let mut bad = order(alone, "move");
        bad.subject = Some(id(&w, "unit.spearman"));
        bad.target_pos = Some(Pos::flat(50_500, 40_500));
        assert_eq!(w.submit(bad), Err(SimError::UnknownFormation));
        w
    });
    let formation: Vec<_> = w
        .entities()
        .filter(|e| e.owner == OWNER && e.orders().front().is_some_and(|o| o.pace.is_some()))
        .collect();
    assert_eq!(formation.len(), 2);
}

#[test]
fn failed_orders_are_reported_to_their_player_and_survive_a_snapshot() {
    let w = twice(|| {
        let mut w = world(rules(), false);
        spawn(&mut w, "bld.city", OWNER, Pos::flat(20_500, 20_500));
        let farm = spawn(&mut w, "bld.farm", OWNER, Pos::flat(30_000, 20_000));
        // A farm has one slot: the second worker sent there finds it taken.
        let first = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(33_500, 20_500));
        let second = spawn(&mut w, "unit.citizen", OWNER, Pos::flat(36_500, 20_500));
        for worker in [first, second] {
            let mut harvest = order(worker, "harvest");
            harvest.target_entity = Some(farm);
            w.submit(harvest).unwrap();
        }
        ticks(&mut w, 400);
        w
    });
    let failures: Vec<_> = w.order_failures(OWNER).collect();
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert_eq!(failures[0].kind, "harvest");
    assert_eq!(failures[0].reason, "no_slot");
    assert_eq!(w.order_failures(ENEMY).count(), 0);
    let restored = World::restore(Arc::clone(w.rules()), w.snapshot()).unwrap();
    assert_eq!(restored.snapshot(), w.snapshot());
    assert_eq!(restored.order_failures(OWNER).count(), 1);
}

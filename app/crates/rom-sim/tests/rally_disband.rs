//! Rally points (`set_rally`), disbanding (`disband`) and granted handicaps.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, Rally, SimError, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    // Cities reach across the test map, and a citizen trains in 100 ticks.
    conn.execute_batch(
        "UPDATE et_territory SET economic_radius=90000, border_radius=90000 WHERE is_city=1;
         UPDATE entity_type SET build_ticks=100 WHERE id=(SELECT id FROM def WHERE key='unit.citizen');",
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn world_with(rules: Arc<Rules>) -> World {
    let player = Player {
        id: OWNER,
        team: 1,
        age: rules.id_of("age.ancient").unwrap(),
        faction: rules.id_of("nation.romans").unwrap(),
        resources: BTreeMap::new(),
        techs: BTreeMap::new(),
    };
    let plain = rules.id_of("ter.plain").unwrap();
    let mut world = World::new(
        rules,
        MapGrid::new(64, 64, vec![plain; 4096]).unwrap(),
        7,
        vec![player],
    )
    .unwrap();
    let resources: Vec<_> = world.rules().resources.keys().copied().collect();
    for resource in resources {
        world.grant(OWNER, resource, 100_000).unwrap();
    }
    world
}

fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
}

fn spawn(world: &mut World, key: &str, pos: Pos) -> EntityId {
    world.spawn(id(world, key), OWNER, pos).unwrap()
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
    }
}

fn rally_to(executor: EntityId, pos: Option<Pos>, entity: Option<EntityId>) -> Command {
    Command {
        target_pos: pos,
        target_entity: entity,
        ..order(executor, "set_rally")
    }
}

fn train(world: &mut World, city: EntityId) {
    let mut command = order(city, "train");
    command.subject = Some(id(world, "unit.citizen"));
    command.queue = true;
    world.submit(command).unwrap();
}

fn citizens(world: &World) -> Vec<EntityId> {
    let citizen = id(world, "unit.citizen");
    world
        .entities()
        .filter(|e| e.type_id == citizen)
        .map(|e| e.id)
        .collect()
}

fn ticks(world: &mut World, n: u64) {
    for _ in 0..n {
        world.step();
    }
}

#[test]
fn trained_units_walk_to_the_rally_point_and_the_queue_stays() {
    let mut w = world_with(rules(""));
    let city = spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
    train(&mut w, city);
    train(&mut w, city);
    let point = Pos::flat(40_500, 30_500);
    // Not queued, yet it replaces nothing: both trainings stay.
    w.submit(rally_to(city, Some(point), None)).unwrap();
    w.step();
    assert_eq!(w.rally(city), Some(Rally::Point(point)));
    assert_eq!(w.entity(city).unwrap().orders().len(), 2);
    ticks(&mut w, 99);
    let first = citizens(&w);
    assert_eq!(first.len(), 1);
    w.step();
    let unit = w.entity(first[0]).unwrap();
    assert_eq!(unit.orders()[0].kind, "move");
    assert_eq!(unit.orders()[0].target_pos, Some(point));
    ticks(&mut w, 1200);
    for unit in citizens(&w) {
        let at = w.entity(unit).unwrap().pos;
        assert!(w.distance(at, point).unwrap() <= 3_000, "{at:?}");
    }
}

#[test]
fn a_harvester_rallied_to_a_gather_site_works_there() {
    let mut w = world_with(rules(""));
    let city = spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
    let farm = spawn(&mut w, "bld.farm", Pos::flat(30_000, 10_000));
    w.submit(rally_to(city, None, Some(farm))).unwrap();
    train(&mut w, city);
    ticks(&mut w, 102);
    let unit = citizens(&w)[0];
    let orders = w.entity(unit).unwrap().orders();
    assert_eq!(orders[0].kind, "harvest");
    assert_eq!(orders[0].target_entity, Some(farm));
    ticks(&mut w, 600);
    assert_eq!(w.entity(unit).unwrap().work_site, Some(farm));
}

#[test]
fn rally_points_are_cleared_checked_and_dropped_with_the_producer() {
    let mut w = world_with(rules(""));
    let city = spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
    let unit = spawn(&mut w, "unit.citizen", Pos::flat(20_500, 20_500));
    // Only producers take a rally point.
    assert_eq!(
        w.submit(rally_to(unit, Some(Pos::flat(1_000, 1_000)), None)),
        Err(SimError::MissingComponent("producer".into()))
    );
    assert_eq!(
        w.submit(rally_to(city, None, Some(EntityId(999)))),
        Err(SimError::InvalidRallyTarget)
    );
    w.submit(rally_to(city, None, Some(unit))).unwrap();
    w.step();
    assert_eq!(w.rally(city), Some(Rally::Entity(unit)));
    // The producer itself as target clears the point.
    w.submit(rally_to(city, None, Some(city))).unwrap();
    w.step();
    assert_eq!(w.rally(city), None);
    w.submit(rally_to(city, Some(Pos::flat(5_000, 5_000)), None))
        .unwrap();
    w.step();
    w.despawn(city).unwrap();
    assert_eq!(w.rallies().count(), 0);
    // A type whose producer has no rally point refuses one.
    let mut w = world_with(rules(
        "UPDATE et_producer SET has_rally=0 WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.city');",
    ));
    let city = spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
    assert_eq!(
        w.submit(rally_to(city, Some(Pos::flat(1_000, 1_000)), None)),
        Err(SimError::NoRallyPoint)
    );
}

#[test]
fn disband_removes_the_unit_and_frees_its_population() {
    let mut w = world_with(rules(""));
    let city = spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
    let unit = spawn(&mut w, "unit.citizen", Pos::flat(20_500, 20_500));
    let used = w.population_used(OWNER);
    let food = id(&w, "res.food");
    let before = w.players().next().unwrap().resources[&food];
    w.submit(order(unit, "disband")).unwrap();
    assert!(w.entity(unit).is_some(), "applied at the next step");
    let done = w.stats().orders_done;
    w.step();
    assert!(w.entity(unit).is_none());
    assert_eq!(w.stats().orders_done, done + 1);
    assert!(w.population_used(OWNER) < used);
    assert_eq!(w.players().next().unwrap().resources[&food], before);
    // Buildings do not move and cannot be disbanded.
    assert_eq!(
        w.submit(order(city, "disband")),
        Err(SimError::MissingComponent("movement".into()))
    );
}

#[test]
fn a_queued_disband_waits_for_the_orders_before_it() {
    let mut w = world_with(rules(""));
    let unit = spawn(&mut w, "unit.citizen", Pos::flat(20_500, 20_500));
    let goal = Pos::flat(30_500, 20_500);
    w.submit(Command {
        target_pos: Some(goal),
        ..order(unit, "move")
    })
    .unwrap();
    w.submit(Command {
        queue: true,
        ..order(unit, "disband")
    })
    .unwrap();
    // A later order for the same unit in the same tick is never applied to a ghost.
    ticks(&mut w, 20);
    assert!(w.entity(unit).is_some(), "still walking");
    ticks(&mut w, 600);
    assert!(w.entity(unit).is_none());
    // A disband followed by another order in the same tick: the second is dropped.
    let other = spawn(&mut w, "unit.citizen", Pos::flat(20_500, 20_500));
    w.submit(order(other, "disband")).unwrap();
    w.submit(Command {
        target_pos: Some(goal),
        queue: true,
        ..order(other, "move")
    })
    .unwrap();
    let cancelled = w.stats().orders_cancelled;
    w.step();
    assert!(w.entity(other).is_none());
    assert_eq!(w.stats().orders_cancelled, cancelled + 1);
}

/// A scenario with both orders: trainings rally to a point, then two of them disband.
fn scenario(steps_before: u64, steps_after: u64, restore: bool) -> u64 {
    let mut w = world_with(rules(""));
    let city = spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
    let farm = spawn(&mut w, "bld.farm", Pos::flat(30_000, 10_000));
    for _ in 0..4 {
        train(&mut w, city);
    }
    w.submit(rally_to(city, Some(Pos::flat(40_500, 40_500)), None))
        .unwrap();
    ticks(&mut w, 250);
    w.submit(rally_to(city, None, Some(farm))).unwrap();
    ticks(&mut w, steps_before);
    if restore {
        w = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    }
    let units = citizens(&w);
    w.submit(order(units[0], "disband")).unwrap();
    w.submit(Command {
        queue: true,
        ..order(units[1], "disband")
    })
    .unwrap();
    ticks(&mut w, steps_after);
    assert!(w.entity(units[0]).is_none());
    w.hash()
}

#[test]
fn rally_and_disband_are_deterministic_and_survive_a_snapshot() {
    let first = scenario(300, 400, false);
    assert_eq!(first, scenario(300, 400, false));
    assert_eq!(first, scenario(300, 400, true));
}

#[test]
fn a_granted_handicap_raises_income_like_a_technology() {
    let r = rules("");
    let handicap = r
        .techs
        .values()
        .find(|t| t.kind == rom_rules::TechKind::Handicap)
        .unwrap()
        .id;
    let food = r.id_of("res.food").unwrap();
    let run = |grant: bool| {
        let mut w = world_with(r.clone());
        if grant {
            w.grant_handicap(OWNER, handicap).unwrap();
        }
        spawn(&mut w, "bld.city", Pos::flat(10_500, 10_500));
        let farm = spawn(&mut w, "bld.farm", Pos::flat(30_000, 10_000));
        let workers: Vec<_> = (0..4)
            .map(|i| spawn(&mut w, "unit.citizen", Pos::flat(28_000 + i * 500, 13_000)))
            .collect();
        for worker in workers {
            w.submit(Command {
                target_entity: Some(farm),
                ..order(worker, "harvest")
            })
            .unwrap();
        }
        ticks(&mut w, 2_000);
        (
            w.players().next().unwrap().resources[&food] - 100_000,
            w.active_sources(OWNER).contains(&handicap),
        )
    };
    let (plain, without) = run(false);
    let (bonus, with) = run(true);
    assert!(!without && with);
    assert!(plain > 0);
    // +25 %, give or take the income period's rounding.
    assert!(
        bonus * 100 >= plain * 120 && bonus * 100 <= plain * 130,
        "{plain} {bonus}"
    );
    // Only a handicap, and only before the first tick.
    let mut w = world_with(r.clone());
    let research = r.id_of("tech.age.classical").unwrap();
    assert_eq!(
        w.grant_handicap(OWNER, research),
        Err(SimError::NotAHandicap)
    );
    w.step();
    assert_eq!(
        w.grant_handicap(OWNER, handicap),
        Err(SimError::NotAHandicap)
    );
}

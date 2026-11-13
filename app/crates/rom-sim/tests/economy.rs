use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, OrderStatus, Player, PlayerId, Pos, SimError, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    // Cities reach across the whole 64×64 test map, so placement rules hold anywhere
    // once a town stands.
    conn.execute_batch(
        "UPDATE et_territory SET economic_radius=90000, border_radius=90000 WHERE is_city=1;",
    )
    .unwrap();
    // These mechanics tests use fixed queue durations and shared ramp counters.
    // The generated progression is exercised without these overrides in scenario.rs.
    conn.execute_batch("UPDATE entity_type SET build_ticks=100 WHERE id=(SELECT id FROM def WHERE key='unit.citizen');
        UPDATE entity_type SET ramp_mode='per_group',ramp_group='barracks' WHERE id IN (SELECT id FROM def WHERE key IN ('unit.spearman','unit.medic'));
        UPDATE cost SET ramp=2 WHERE def_id=(SELECT id FROM def WHERE key='unit.spearman') AND resource_id=(SELECT id FROM def WHERE key='res.food');
        DELETE FROM prerequisite WHERE def_id IN (SELECT id FROM def WHERE key IN ('tech.age.classical','tech.age.abundance'));
        DELETE FROM prerequisite WHERE def_id IN (SELECT id FROM def WHERE key IN ('bld.temple','bld.barracks')) AND requires_def_id IN (SELECT id FROM tech WHERE kind='track');").unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn setup(catalogue: Arc<Rules>, techs: &[&str]) -> World {
    let player = Player {
        id: OWNER,
        team: 1,
        age: catalogue.id_of("age.ancient").unwrap(),
        faction: catalogue.id_of("nation.romans").unwrap(),
        resources: BTreeMap::new(),
        techs: techs
            .iter()
            .map(|key| (catalogue.id_of(key).unwrap(), 1))
            .collect(),
    };
    let plain = catalogue.id_of("ter.plain").unwrap();
    let mut world = World::new(
        catalogue,
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

fn world() -> World {
    setup(rules(""), &[])
}
fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
}
fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x, y)
}
fn spawn(world: &mut World, key: &str, pos: Pos) -> EntityId {
    world.spawn(id(world, key), OWNER, pos).unwrap()
}
/// Places a starting town for `owner` through match setup, in a corner away from the tests.
fn town(world: &mut World, owner: PlayerId, pos: Pos) -> EntityId {
    world
        .place_for_setup(id(world, "bld.city"), owner, pos)
        .unwrap()
}
fn player(world: &World) -> &Player {
    world.players().next().unwrap()
}
fn balance(world: &World, key: &str) -> i64 {
    player(world).resources[&id(world, key)]
}
fn count(world: &World, key: &str) -> usize {
    world
        .entities()
        .filter(|e| e.type_id == id(world, key))
        .count()
}
fn ticks(world: &mut World, n: u64) {
    for _ in 0..n {
        world.step();
    }
}
fn command(world: &World, executor: EntityId, kind: &str, subject: &str) -> Command {
    Command {
        player: OWNER,
        executor,
        kind: kind.into(),
        subject: Some(id(world, subject)),
        target_entity: None,
        target_pos: None,
        queue: true,
    }
}
fn train(
    world: &mut World,
    executor: EntityId,
    subject: &str,
) -> Result<rom_sim::OrderId, SimError> {
    world.submit(command(world, executor, "train", subject))
}
fn research(
    world: &mut World,
    executor: EntityId,
    subject: &str,
) -> Result<rom_sim::OrderId, SimError> {
    world.submit(command(world, executor, "research", subject))
}
fn construction(world: &World, executor: EntityId, subject: &str, pos: Pos) -> Command {
    let mut order = command(world, executor, "construct", subject);
    order.target_pos = Some(pos);
    order
}
fn stop(world: &mut World, executor: EntityId) {
    world
        .submit(Command {
            player: OWNER,
            executor,
            kind: "stop".into(),
            subject: None,
            target_entity: None,
            target_pos: None,
            queue: false,
        })
        .unwrap();
}
fn help(world: &mut World, builder: EntityId, site: EntityId) {
    world
        .submit(Command {
            player: OWNER,
            executor: builder,
            kind: "repair".into(),
            subject: None,
            target_entity: Some(site),
            target_pos: None,
            queue: true,
        })
        .unwrap();
}
fn build_duration(world: &World, key: &str) -> u64 {
    world
        .rules()
        .build_ticks(id(world, key), &player(world).active_sources()) as u64
}
fn site(world: &World, key: &str) -> EntityId {
    world
        .entities()
        .find(|e| e.type_id == id(world, key))
        .unwrap()
        .id
}
fn finish(world: &mut World, executor: EntityId) {
    world.step();
    for _ in 0..10_000 {
        if world.entity(executor).unwrap().orders().is_empty() {
            return;
        }
        world.step();
    }
    panic!("executor did not finish: {:?}", world.entity(executor));
}

#[test]
fn train_deducts_cost_and_spawns_after_build_ticks() {
    let mut w = world();
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    let before = balance(&w, "res.food");
    train(&mut w, city, "unit.citizen").unwrap();
    assert_eq!(balance(&w, "res.food"), before - 20);
    assert_eq!(count(&w, "unit.citizen"), 0);
    let duration = build_duration(&w, "unit.citizen");
    ticks(&mut w, duration - 1);
    assert_eq!(count(&w, "unit.citizen"), 0);
    assert_eq!(
        w.entity(city).unwrap().orders()[0].progress_ticks,
        duration - 1
    );
    w.step();
    assert_eq!(count(&w, "unit.citizen"), 1);
    let unit = w
        .entities()
        .find(|e| e.type_id == id(&w, "unit.citizen"))
        .unwrap();
    assert_eq!(unit.owner, OWNER);
    assert_eq!(unit.pos, at(14_001, 10_500));
    assert_eq!(w.stats().orders_done, 1);
}

#[test]
fn train_cost_ramps_per_type() {
    let mut w = world();
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    spawn(&mut w, "unit.citizen", at(0, 0));
    let food = id(&w, "res.food");
    let citizen = id(&w, "unit.citizen");
    assert_eq!(w.price(OWNER, citizen), vec![(food, 21)]);
    train(&mut w, city, "unit.citizen").unwrap();
    assert_eq!(w.price(OWNER, citizen), vec![(food, 22)]);
    w.step();
    assert_eq!(w.price(OWNER, citizen), vec![(food, 22)]);
}

#[test]
fn ramp_group_is_shared() {
    let mut w = world();
    let spear = id(&w, "unit.spearman");
    let base = w.price(OWNER, spear);
    spawn(&mut w, "unit.medic", at(0, 0));
    assert_eq!(
        w.price(OWNER, spear),
        vec![(id(&w, "res.food"), 42), (id(&w, "res.timber"), 21)]
    );
    assert_ne!(base, w.price(OWNER, spear));
}

#[test]
fn insufficient_resources_rejected_and_nothing_deducted() {
    let catalogue = rules("");
    let mut w = setup(catalogue, &[]);
    let citizen = spawn(&mut w, "unit.citizen", at(0, 0));
    // Check atomic rejection with less timber than a farm requires.
    let timber = id(&w, "res.timber");
    let mut resources = player(&w).resources.clone();
    resources.insert(timber, 49);
    let mut p = player(&w).clone();
    p.resources = resources;
    let mut w = World::new(w.rules().clone(), w.map().clone(), 7, vec![p]).unwrap();
    let citizen2 = spawn(&mut w, "unit.citizen", at(0, 0));
    assert_eq!(citizen, citizen2);
    town(&mut w, OWNER, at(56_500, 56_500));
    let hash = w.hash();
    assert_eq!(
        w.submit(construction(&w, citizen2, "bld.farm", at(10_000, 10_000))),
        Err(SimError::InsufficientResources)
    );
    assert_eq!(hash, w.hash());
    // A two-resource price must not partially charge food when timber is short.
    let barracks = spawn(&mut w, "bld.barracks", at(20_000, 20_000));
    for _ in 0..2 {
        train(&mut w, barracks, "unit.spearman").unwrap();
    }
    let hash = w.hash();
    assert_eq!(
        train(&mut w, barracks, "unit.spearman"),
        Err(SimError::InsufficientResources)
    );
    assert_eq!(hash, w.hash());
}

#[test]
fn queue_runs_in_order_and_respects_queue_limit() {
    let mut w = setup(rules("UPDATE et_producer SET queue_limit=2;"), &[]);
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    let a = train(&mut w, city, "unit.citizen").unwrap();
    let b = train(&mut w, city, "unit.citizen").unwrap();
    let hash = w.hash();
    assert_eq!(
        train(&mut w, city, "unit.citizen"),
        Err(SimError::QueueFull)
    );
    assert_eq!(hash, w.hash());
    w.step();
    assert_eq!(w.entity(city).unwrap().orders()[0].id, a);
    assert_eq!(
        w.entity(city).unwrap().orders()[1].status,
        OrderStatus::Queued
    );
    ticks(&mut w, 99);
    assert_eq!(count(&w, "unit.citizen"), 1);
    assert_eq!(w.entity(city).unwrap().orders()[0].id, b);
    assert_eq!(w.entity(city).unwrap().orders()[0].progress_ticks, 0);
    ticks(&mut w, 99);
    assert_eq!(count(&w, "unit.citizen"), 1);
    w.step();
    assert_eq!(count(&w, "unit.citizen"), 2);
}

#[test]
fn producer_parallel_front_orders_progress_once_each_tick() {
    let mut w = setup(rules("UPDATE et_producer SET parallel=2;"), &[]);
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    for _ in 0..3 {
        train(&mut w, city, "unit.citizen").unwrap();
    }
    ticks(&mut w, 99);
    let queue = w.entity(city).unwrap().orders();
    assert_eq!(
        queue.iter().map(|o| o.progress_ticks).collect::<Vec<_>>(),
        [99, 99, 0]
    );
    w.step();
    assert_eq!(count(&w, "unit.citizen"), 2);
    ticks(&mut w, 100);
    assert_eq!(count(&w, "unit.citizen"), 3);
}

#[test]
fn pop_cap_blocks_until_room() {
    let mut w = world();
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    let mut units = Vec::new();
    for _ in 0..w.population_cap(OWNER) {
        units.push(spawn(&mut w, "unit.citizen", at(0, 0)));
    }
    train(&mut w, city, "unit.citizen").unwrap();
    ticks(&mut w, 110);
    assert_eq!(
        w.entity(city).unwrap().orders()[0].status,
        OrderStatus::Blocked("pop_cap".into())
    );
    assert_eq!(count(&w, "unit.citizen"), 50);
    w.despawn(units[0]).unwrap();
    w.step();
    assert_eq!(count(&w, "unit.citizen"), 50);
    assert!(w.entity(city).unwrap().orders().is_empty());
}

#[test]
fn stop_refunds_queued_training() {
    let mut w = setup(
        rules("UPDATE game_constant SET value=50 WHERE key='economy.cancel_refund_pct';"),
        &[],
    );
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    let before = balance(&w, "res.food");
    for _ in 0..3 {
        train(&mut w, city, "unit.citizen").unwrap();
    }
    assert_eq!(balance(&w, "res.food"), before - 63);
    w.step();
    stop(&mut w, city);
    w.step();
    assert_eq!(balance(&w, "res.food"), before - 63 + 10 + 10 + 11);
    assert_eq!(w.stats().orders_cancelled, 3);
    assert!(w.entity(city).unwrap().orders().is_empty());
}

#[test]
fn construct_creates_site_then_completes() {
    let mut w = world();
    town(&mut w, OWNER, at(56_500, 56_500));
    let worker = spawn(&mut w, "unit.citizen", at(0, 10_000));
    let food = balance(&w, "res.food");
    w.submit(construction(&w, worker, "bld.farm", at(10_100, 10_100)))
        .unwrap();
    assert_eq!(count(&w, "bld.farm"), 0);
    w.step();
    let farm = site(&w, "bld.farm");
    assert_eq!(w.entity(farm).unwrap().pos, at(10_000, 10_000));
    assert_eq!(w.entity(farm).unwrap().hp, 40);
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(0));
    assert!(!w.owned_types(OWNER).contains(&id(&w, "bld.farm")));
    // 6500 milli-tiles to reach, at 65 per tick: first work on tick 100.
    ticks(&mut w, 98);
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(0));
    w.step();
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(1));
    ticks(&mut w, 148);
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(149));
    assert_eq!(balance(&w, "res.food"), food);
    w.step();
    assert_eq!(w.entity(farm).unwrap().build_progress, None);
    assert_eq!(w.entity(farm).unwrap().hp, 400);
    assert!(w.owned_types(OWNER).contains(&id(&w, "bld.farm")));
    assert_eq!(balance(&w, "res.food"), food + 20);
    ticks(&mut w, 20);
    assert_eq!(balance(&w, "res.food"), food + 20);
}

fn build_farm(helping: bool) -> u64 {
    let mut w = world();
    town(&mut w, OWNER, at(56_500, 56_500));
    let worker = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    let other = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000)))
        .unwrap();
    w.step();
    let farm = site(&w, "bld.farm");
    if helping {
        help(&mut w, other, farm);
    }
    finish(&mut w, worker);
    w.tick()
}

#[test]
fn two_builders_finish_sooner_than_one() {
    assert_eq!(build_farm(false), 150);
    assert_eq!(build_farm(true), 76);
}

#[test]
fn construct_rejects_overlap() {
    let mut w = world();
    town(&mut w, OWNER, at(56_500, 56_500));
    let worker = spawn(&mut w, "unit.citizen", at(0, 0));
    let other = spawn(&mut w, "unit.citizen", at(0, 0));
    w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000)))
        .unwrap();
    let hash = w.hash();
    assert_eq!(
        w.submit(construction(&w, other, "bld.library", at(13_000, 10_000))),
        Err(SimError::PlacementOverlap)
    );
    assert_eq!(hash, w.hash());
    w.step();
    assert_eq!(
        w.submit(construction(&w, other, "bld.library", at(13_000, 10_000))),
        Err(SimError::PlacementOverlap)
    );
    stop(&mut w, worker);
    w.step();
    assert_eq!(
        w.submit(construction(&w, other, "bld.library", at(13_000, 10_000))),
        Err(SimError::PlacementOverlap)
    );
    // Touching edges do not overlap.
    w.submit(construction(&w, other, "bld.library", at(14_000, 10_000)))
        .unwrap();
}

#[test]
fn construct_rejects_unbuildable_terrain() {
    let catalogue = rules("");
    let p = player(&setup(catalogue.clone(), &[])).clone();
    let plain = catalogue.id_of("ter.plain").unwrap();
    let deep = catalogue.id_of("ter.deep").unwrap();
    let mut terrain = vec![plain; 4096];
    terrain[8 * 64 + 8] = deep;
    let mut w = World::new(
        catalogue,
        MapGrid::new(64, 64, terrain).unwrap(),
        7,
        vec![p],
    )
    .unwrap();
    let worker = spawn(&mut w, "unit.citizen", at(0, 0));
    let hash = w.hash();
    assert_eq!(
        w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000))),
        Err(SimError::UnbuildableTerrain)
    );
    assert_eq!(hash, w.hash());
}

#[test]
fn construct_rejects_outside_map() {
    let mut w = world();
    let worker = spawn(&mut w, "unit.citizen", at(0, 0));
    for pos in [
        at(1000, 10_000),
        at(10_000, 1000),
        at(63_000, 10_000),
        at(10_000, 63_000),
    ] {
        let hash = w.hash();
        assert_eq!(
            w.submit(construction(&w, worker, "bld.farm", pos)),
            Err(SimError::OutsideMap)
        );
        assert_eq!(hash, w.hash());
    }
}

#[test]
fn incomplete_building_cannot_produce() {
    let mut w = world();
    let worker = spawn(&mut w, "unit.citizen", at(0, 0));
    w.submit(construction(&w, worker, "bld.city", at(10_500, 10_500)))
        .unwrap();
    w.step();
    let city = site(&w, "bld.city");
    let hash = w.hash();
    assert_eq!(
        train(&mut w, city, "unit.citizen"),
        Err(SimError::IncompleteExecutor)
    );
    assert_eq!(hash, w.hash());
}

#[test]
fn locked_item_rejected() {
    let mut w = world();
    let temple = spawn(&mut w, "bld.temple", at(10_000, 10_000));
    assert_eq!(train(&mut w, temple, "unit.priest"), Err(SimError::Locked));
}

#[test]
fn research_sets_age() {
    let mut w = world();
    town(&mut w, OWNER, at(56_500, 56_500));
    let worker = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    w.submit(construction(&w, worker, "bld.library", at(10_000, 10_000)))
        .unwrap();
    finish(&mut w, worker);
    let library = site(&w, "bld.library");
    assert_eq!(
        w.submit(construction(&w, worker, "bld.temple", at(20_000, 10_000))),
        Err(SimError::Locked)
    );
    research(&mut w, library, "tech.age.classical").unwrap();
    ticks(&mut w, 599);
    assert_eq!(player(&w).age, id(&w, "age.ancient"));
    w.step();
    assert_eq!(player(&w).age, id(&w, "age.classical"));
    w.submit(construction(&w, worker, "bld.temple", at(20_000, 10_000)))
        .unwrap();
    finish(&mut w, worker);
}

#[test]
fn research_twice_rejected() {
    let mut w = world();
    let library = spawn(&mut w, "bld.library", at(10_000, 10_000));
    let other = spawn(&mut w, "bld.library", at(20_000, 10_000));
    research(&mut w, library, "tech.age.classical").unwrap();
    assert_eq!(
        research(&mut w, other, "tech.age.classical"),
        Err(SimError::ResearchQueued)
    );
    w.step();
    assert_eq!(
        research(&mut w, other, "tech.age.classical"),
        Err(SimError::ResearchQueued)
    );
    ticks(&mut w, 599);
    let hash = w.hash();
    assert_eq!(
        research(&mut w, other, "tech.age.classical"),
        Err(SimError::AlreadyResearched)
    );
    assert_eq!(hash, w.hash());
}

#[test]
fn exclusive_group_rejected() {
    let mut w = setup(
        rules(
            "UPDATE tech SET exclusive_group='constitution' WHERE id IN (SELECT id FROM def WHERE key IN ('tech.age.classical','tech.age.abundance'));",
        ),
        &[],
    );
    let library = spawn(&mut w, "bld.library", at(10_000, 10_000));
    let other = spawn(&mut w, "bld.library", at(20_000, 10_000));
    research(&mut w, library, "tech.age.classical").unwrap();
    assert_eq!(
        research(&mut w, other, "tech.age.abundance"),
        Err(SimError::ExclusiveResearch)
    );
    ticks(&mut w, 600);
    assert_eq!(
        research(&mut w, other, "tech.age.abundance"),
        Err(SimError::ExclusiveResearch)
    );
}

#[test]
fn abundance_age_makes_training_free_but_not_instant() {
    let mut w = setup(rules(""), &["tech.age.abundance"]);
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    spawn(&mut w, "unit.citizen", at(0, 0));
    let citizen = id(&w, "unit.citizen");
    assert!(
        w.price(OWNER, citizen)
            .iter()
            .all(|(_, amount)| *amount == 0)
    );
    let balances = player(&w).resources.clone();
    train(&mut w, city, "unit.citizen").unwrap();
    assert_eq!(balances, player(&w).resources);
    ticks(&mut w, 99);
    assert_eq!(count(&w, "unit.citizen"), 1);
    w.step();
    assert_eq!(count(&w, "unit.citizen"), 2);
}

#[test]
fn per_player_limit_enforced() {
    let mut w = setup(
        rules(
            "UPDATE entity_type SET limit_scope='per_player',limit_count=2 WHERE id=(SELECT id FROM def WHERE key='unit.citizen');",
        ),
        &[],
    );
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    spawn(&mut w, "unit.citizen", at(0, 0));
    train(&mut w, city, "unit.citizen").unwrap();
    let hash = w.hash();
    assert_eq!(
        train(&mut w, city, "unit.citizen"),
        Err(SimError::LimitReached)
    );
    assert_eq!(hash, w.hash());
    ticks(&mut w, 100);
    assert_eq!(
        train(&mut w, city, "unit.citizen"),
        Err(SimError::LimitReached)
    );
}

fn scenario(catalogue: Arc<Rules>) -> World {
    let mut w = setup(catalogue, &[]);
    spawn(&mut w, "bld.city", at(6500, 6500));
    let workers: Vec<_> = (0..5)
        .map(|n| spawn(&mut w, "unit.citizen", at(10_000 + n * 6000, 15_000)))
        .collect();
    for (worker, key, pos) in [
        (workers[0], "bld.farm", at(10_000, 20_000)),
        (workers[1], "bld.farm", at(16_000, 20_000)),
        (workers[2], "bld.library", at(22_000, 20_000)),
        (workers[3], "bld.barracks", at(28_000, 20_000)),
    ] {
        w.submit(construction(&w, worker, key, pos)).unwrap();
    }
    ticks(&mut w, 500);
    let library = site(&w, "bld.library");
    let barracks = site(&w, "bld.barracks");
    research(&mut w, library, "tech.age.classical").unwrap();
    for _ in 0..3 {
        train(&mut w, barracks, "unit.spearman").unwrap();
    }
    ticks(&mut w, 700);
    assert_eq!(count(&w, "unit.spearman"), 3);
    assert_eq!(player(&w).age, id(&w, "age.classical"));
    assert_eq!(w.stats().orders_failed, 0);
    w
}

#[test]
fn economy_scenario_is_deterministic() {
    let catalogue = rules("");
    let first = scenario(catalogue.clone());
    let second = scenario(catalogue);
    assert_eq!(first.hash(), second.hash());
    assert_eq!(first.tick(), 1200);
}

#[test]
fn research_repeatable_counts_and_faction_effects() {
    let mut w = setup(rules("UPDATE tech SET repeatable=1,research_ticks=2 WHERE id=(SELECT id FROM def WHERE key='tech.age.classical');
        INSERT INTO tech_effect(tech_id,effect,target_def_id) SELECT t.id,'change_faction',f.id FROM def t,def f WHERE t.key='tech.age.classical' AND f.key='nation.greeks';"), &[]);
    let library = spawn(&mut w, "bld.library", at(10_000, 10_000));
    for _ in 0..2 {
        research(&mut w, library, "tech.age.classical").unwrap();
        ticks(&mut w, 2);
    }
    assert_eq!(player(&w).techs[&id(&w, "tech.age.classical")], 2);
    assert_eq!(player(&w).faction, id(&w, "nation.greeks"));
}

#[test]
fn availability_and_production_checks_precede_payment() {
    let mut w = setup(
        rules(
            "INSERT INTO availability(def_id,faction_id) SELECT d.id,f.id FROM def d,def f WHERE d.key='unit.citizen' AND f.key='nation.greeks';",
        ),
        &[],
    );
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    let hash = w.hash();
    assert_eq!(
        train(&mut w, city, "unit.citizen"),
        Err(SimError::Unavailable)
    );
    assert_eq!(
        train(&mut w, city, "unit.spearman"),
        Err(SimError::ProductionUnavailable)
    );
    assert_eq!(w.hash(), hash);
}

#[test]
fn per_match_limits_count_other_players_and_construction_sites_once() {
    let catalogue = rules(
        "UPDATE entity_type SET limit_scope='per_match',limit_count=2 WHERE id=(SELECT id FROM def WHERE key='bld.farm');",
    );
    let initial = setup(catalogue.clone(), &[]);
    let p = player(&initial).clone();
    let mut other = p.clone();
    other.id = PlayerId(2);
    let mut w = World::new(catalogue, initial.map().clone(), 7, vec![p, other]).unwrap();
    town(&mut w, OWNER, at(56_500, 56_500));
    town(&mut w, PlayerId(2), at(56_500, 30_500));
    let worker = spawn(&mut w, "unit.citizen", at(0, 0));
    let other_worker = w
        .spawn(id(&w, "unit.citizen"), PlayerId(2), at(0, 0))
        .unwrap();
    w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000)))
        .unwrap();
    w.step();
    let mut order = construction(&w, other_worker, "bld.farm", at(20_000, 10_000));
    order.player = PlayerId(2);
    w.submit(order).unwrap();
    assert_eq!(
        w.submit(construction(&w, worker, "bld.farm", at(30_000, 10_000))),
        Err(SimError::LimitReached)
    );
}

#[test]
fn construction_fractional_work_and_cancelled_sites_can_be_resumed() {
    let mut w = setup(rules("UPDATE et_builder SET build_pct=50;"), &[]);
    town(&mut w, OWNER, at(56_500, 56_500));
    let worker = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    let timber = balance(&w, "res.timber");
    w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000)))
        .unwrap();
    w.step();
    let farm = site(&w, "bld.farm");
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(0));
    assert_eq!(w.entity(worker).unwrap().orders()[0].build_remainder, 50);
    w.step();
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(1));
    stop(&mut w, worker);
    w.step();
    assert_eq!(balance(&w, "res.timber"), timber - 50);
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(1));
    help(&mut w, worker, farm);
    ticks(&mut w, 297);
    assert_eq!(w.entity(farm).unwrap().build_progress, Some(149));
    w.step();
    assert_eq!(w.entity(farm).unwrap().build_progress, None);
    help(&mut w, worker, farm);
    w.step();
    assert_eq!(w.stats().orders_unsupported, 1);
}

#[test]
fn destroyed_sites_fail_builders_and_helpers() {
    let mut w = world();
    town(&mut w, OWNER, at(56_500, 56_500));
    let worker = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    let other = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000)))
        .unwrap();
    w.step();
    let farm = site(&w, "bld.farm");
    help(&mut w, other, farm);
    w.despawn(farm).unwrap();
    w.step();
    assert!(w.entity(worker).unwrap().orders().is_empty());
    assert!(w.entity(other).unwrap().orders().is_empty());
    assert_eq!(w.stats().orders_failed, 2);
}

#[test]
fn research_refunds_recorded_payment_and_releases_pending_lock() {
    let mut w = world();
    let library = spawn(&mut w, "bld.library", at(10_000, 10_000));
    let before = balance(&w, "res.food");
    research(&mut w, library, "tech.age.classical").unwrap();
    stop(&mut w, library);
    w.step();
    assert_eq!(balance(&w, "res.food"), before);
    assert!(player(&w).techs.is_empty());
    research(&mut w, library, "tech.age.classical").unwrap();
}

#[test]
fn durations_are_evaluated_on_activation_and_research_uses_player_stat() {
    let mut w = setup(rules("UPDATE tech SET research_ticks=100 WHERE id=(SELECT id FROM def WHERE key='tech.age.classical');
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'research_time','mul',50 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'build_time','mul',50 FROM def WHERE key='tech.age.classical';"), &[]);
    let city = spawn(&mut w, "bld.city", at(10_500, 10_500));
    let library = spawn(&mut w, "bld.library", at(20_000, 10_000));
    train(&mut w, city, "unit.citizen").unwrap();
    train(&mut w, city, "unit.citizen").unwrap();
    research(&mut w, library, "tech.age.classical").unwrap();
    ticks(&mut w, 49);
    assert_eq!(player(&w).age, id(&w, "age.ancient"));
    w.step();
    assert_eq!(player(&w).age, id(&w, "age.classical"));
    assert_eq!(w.entity(city).unwrap().orders()[0].total_ticks, Some(100));
    ticks(&mut w, 50);
    assert_eq!(count(&w, "unit.citizen"), 1);
    w.step();
    assert_eq!(w.entity(city).unwrap().orders()[0].total_ticks, Some(50));
    ticks(&mut w, 49);
    assert_eq!(count(&w, "unit.citizen"), 2);
}

#[test]
fn ramp_is_applied_before_cost_modifiers_including_resource_specific_rounding() {
    let mut w = setup(rules("INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'cost.all','add',7 FROM def WHERE key='nation.romans';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'cost.food','mul',50 FROM def WHERE key='nation.romans';"), &[]);
    spawn(&mut w, "unit.citizen", at(0, 0));
    assert_eq!(
        w.price(OWNER, id(&w, "unit.citizen")),
        [(id(&w, "res.food"), 14)]
    );
    // (20 + ramp 1 + add 7) * 50 / 100 = 14, not modified-base + ramp.
}

#[test]
fn training_chooses_another_side_when_producer_touches_map_edge() {
    let mut w = world();
    let city = spawn(&mut w, "bld.city", at(60_500, 10_500));
    train(&mut w, city, "unit.citizen").unwrap();
    ticks(&mut w, 100);
    let unit = site(&w, "unit.citizen");
    assert_eq!(w.entity(unit).unwrap().pos, at(56_999, 10_500));
    assert_eq!(w.entities_in_radius(at(56_999, 10_500), 0), [unit]);
}

#[test]
fn completed_entity_prerequisites_exclude_sites() {
    let mut w = setup(
        rules(
            "INSERT INTO prerequisite(def_id,requires_def_id,alt_group) SELECT d.id,r.id,1 FROM def d,def r WHERE d.key='unit.citizen' AND r.key='bld.library';",
        ),
        &[],
    );
    let city = spawn(&mut w, "bld.city", at(6500, 6500));
    let worker = spawn(&mut w, "unit.citizen", at(17_000, 20_000));
    w.submit(construction(&w, worker, "bld.library", at(20_000, 20_000)))
        .unwrap();
    w.step();
    assert_eq!(train(&mut w, city, "unit.citizen"), Err(SimError::Locked));
    ticks(&mut w, 399);
    train(&mut w, city, "unit.citizen").unwrap();
}

#[test]
fn build_reach_is_measured_to_each_footprint_edge() {
    for pos in [
        at(6500, 10_000),
        at(13_500, 10_000),
        at(10_000, 6500),
        at(10_000, 13_500),
    ] {
        let mut w = world();
        town(&mut w, OWNER, at(56_500, 56_500));
        let worker = spawn(&mut w, "unit.citizen", pos);
        w.submit(construction(&w, worker, "bld.farm", at(10_000, 10_000)))
            .unwrap();
        w.step();
        assert_eq!(
            w.entity(site(&w, "bld.farm")).unwrap().build_progress,
            Some(1)
        );
        assert_eq!(w.entity(worker).unwrap().pos, pos);
    }
}

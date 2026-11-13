use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{
    Command, EntityId, Income, MapGrid, OrderStatus, Player, PlayerId, Pos, SimError, World,
};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);

fn rules(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    // Cities reach across the whole 64×64 test map, so placement rules hold anywhere
    // once a town stands.
    conn.execute_batch(
        "UPDATE et_territory SET economic_radius=90000, border_radius=90000 WHERE is_city=1;",
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn setup(catalogue: Arc<Rules>, forest: &[(usize, usize)]) -> World {
    let player = Player {
        id: OWNER,
        team: 1,
        age: catalogue.id_of("age.ancient").unwrap(),
        faction: catalogue.id_of("nation.romans").unwrap(),
        resources: ["res.food", "res.timber"]
            .into_iter()
            .map(|key| (catalogue.id_of(key).unwrap(), 200))
            .collect(),
        techs: BTreeMap::new(),
    };
    let mut other = player.clone();
    other.id = PlayerId(2);
    let mut terrain = vec![catalogue.id_of("ter.plain").unwrap(); 64 * 64];
    for (x, y) in forest {
        terrain[y * 64 + x] = catalogue.id_of("ter.forest").unwrap();
    }
    World::new(
        catalogue,
        MapGrid::new(64, 64, terrain).unwrap(),
        7,
        vec![player, other],
    )
    .unwrap()
}

fn world() -> World {
    setup(rules(""), &[])
}
fn at(x: i32, y: i32) -> Pos {
    Pos { x, y }
}
fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}
fn spawn(w: &mut World, key: &str, pos: Pos) -> EntityId {
    w.spawn(id(w, key), OWNER, pos).unwrap()
}
/// Places the owner's starting town through match setup, in a corner away from the tests.
fn town(w: &mut World) {
    w.place_for_setup(id(w, "bld.city"), OWNER, at(56_500, 56_500))
        .unwrap();
}
fn ticks(w: &mut World, count: u64) {
    for _ in 0..count {
        w.step();
    }
}
fn constant(w: &World, key: &str) -> u64 {
    w.rules().constant(key).unwrap() as u64
}
fn balance(w: &World, key: &str) -> i64 {
    w.players()
        .find(|p| p.id == OWNER)
        .unwrap()
        .resources
        .get(&id(w, key))
        .copied()
        .unwrap_or(0)
}
fn command(worker: EntityId, kind: &str) -> Command {
    Command {
        player: OWNER,
        executor: worker,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
    }
}
fn harvest(worker: EntityId, site: EntityId) -> Command {
    let mut c = command(worker, "harvest");
    c.target_entity = Some(site);
    c
}
fn construct(w: &World, worker: EntityId, key: &str, pos: Pos) -> Command {
    let mut c = command(worker, "construct");
    c.subject = Some(id(w, key));
    c.target_pos = Some(pos);
    c
}
fn research(w: &World, library: EntityId, key: &str) -> Command {
    let mut c = command(library, "research");
    c.subject = Some(id(w, key));
    c
}
fn working(w: &mut World) -> (EntityId, EntityId) {
    let farm = spawn(w, "bld.farm", at(10_000, 10_000));
    let worker = spawn(w, "unit.citizen", at(6500, 10_000));
    w.submit(harvest(worker, farm)).unwrap();
    w.step();
    assert_eq!(w.entity(worker).unwrap().work_site, Some(farm));
    (worker, farm)
}

#[test]
fn farm_has_one_slot_and_second_worker_fails_no_slot() {
    let mut w = world();
    let farm = spawn(&mut w, "bld.farm", at(10_000, 10_000));
    let first = spawn(&mut w, "unit.citizen", at(6500, 10_000));
    let second = spawn(&mut w, "unit.citizen", at(13_500, 10_000));
    w.submit(harvest(first, farm)).unwrap();
    w.submit(harvest(second, farm)).unwrap();
    w.step();
    assert_eq!(w.entity(farm).unwrap().gather_slots, 1);
    assert_eq!(w.entity(first).unwrap().work_site, Some(farm));
    assert_eq!(
        w.entity(first).unwrap().orders()[0].status,
        OrderStatus::Active
    );
    assert_eq!(w.entity(second).unwrap().work_site, None);
    assert!(w.entity(second).unwrap().orders().is_empty());
    assert_eq!(w.stats().orders_no_slot, 1);
    assert_eq!(w.stats().orders_failed, 1);
}

#[test]
fn slots_scale_with_nearby_node_terrain_and_respect_max() {
    let catalogue = rules("UPDATE entity_type SET footprint_w=3,footprint_h=3 WHERE id=(SELECT id FROM def WHERE key='bld.library');
        INSERT INTO et_gather_site(entity_type_id,resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id)
        SELECT s.id,r.id,1,2,6,3000,t.id FROM def s,def r,def t
        WHERE s.key='bld.library' AND r.key='res.food' AND t.key='ter.forest';");
    // At (10500,10500), the tile at (13,10) is exactly on the radius;
    // (13,11) lies outside the circle despite being inside its bounding square.
    for (forest, expected) in [
        (vec![], 1),
        (vec![(13, 10), (13, 11)], 3),
        (vec![(13, 10), (10, 13), (7, 10), (10, 7)], 6),
    ] {
        let mut w = setup(catalogue.clone(), &forest);
        town(&mut w);
        let worker = spawn(&mut w, "unit.citizen", at(7500, 10_500));
        w.submit(construct(&w, worker, "bld.library", at(10_500, 10_500)))
            .unwrap();
        w.step();
        let site = w
            .entities()
            .find(|e| e.type_id == id(&w, "bld.library"))
            .unwrap()
            .id;
        assert_eq!(w.entity(site).unwrap().gather_slots, 0);
        ticks(&mut w, 399);
        assert_eq!(w.entity(site).unwrap().build_progress, None);
        assert_eq!(w.entity(site).unwrap().gather_slots, expected);
        let far = spawn(&mut w, "bld.library", at(50_500, 50_500));
        assert_eq!(w.entity(far).unwrap().gather_slots, 1);
    }
    let mut w = setup(
        rules("UPDATE et_gather_site SET slots_per_node=20;"),
        &[(10, 10)],
    );
    let farm = spawn(&mut w, "bld.farm", at(10_500, 10_500));
    assert_eq!(w.entity(farm).unwrap().gather_slots, 1);
}

#[test]
fn harvest_rejected_for_foreign_incomplete_or_wrong_resource_site() {
    let mut w = setup(
        rules(
            "INSERT INTO et_gather_site(entity_type_id,resource_id)
        SELECT s.id,r.id FROM def s,def r WHERE s.key='bld.library' AND r.key='res.knowledge';",
        ),
        &[],
    );
    let worker = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    let foreign = w
        .spawn(id(&w, "bld.farm"), PlayerId(2), at(20_000, 10_000))
        .unwrap();
    let wrong = spawn(&mut w, "bld.library", at(30_000, 10_000));
    let non_site = spawn(&mut w, "bld.city", at(40_000, 10_000));
    w.submit(construct(&w, worker, "bld.farm", at(10_000, 10_000)))
        .unwrap();
    w.step();
    let incomplete = w
        .entities()
        .find(|e| e.build_progress.is_some())
        .unwrap()
        .id;
    for target in [foreign, wrong, incomplete, non_site, EntityId(999)] {
        let hash = w.hash();
        assert_eq!(
            w.submit(harvest(worker, target)),
            Err(SimError::InvalidHarvestTarget)
        );
        assert_eq!(w.hash(), hash);
    }
}

#[test]
fn worker_adds_exact_income_over_one_period() {
    let mut w = world();
    working(&mut w);
    let before = balance(&w, "res.food");
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period);
    assert_eq!(balance(&w, "res.food") - before, 10);
    assert_eq!(
        w.income(OWNER, id(&w, "res.food")),
        Income {
            raw: 10,
            effective: 10,
            capped: false
        }
    );
}

#[test]
fn income_is_spread_evenly_not_lumped() {
    let mut w = world();
    working(&mut w);
    let before = balance(&w, "res.food");
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period / 2);
    assert_eq!(balance(&w, "res.food") - before, 5);
}

#[test]
fn leaving_the_slot_stops_income() {
    for queue in [false, true] {
        let mut w = world();
        let (worker, _) = working(&mut w);
        let before = balance(&w, "res.food");
        let mut c = command(worker, "move");
        c.target_pos = Some(at(60_000, 60_000));
        c.queue = queue;
        w.submit(c).unwrap();
        w.step();
        assert_eq!(w.entity(worker).unwrap().work_site, None);
        let period = constant(&w, "economy.income_period_ticks");
        ticks(&mut w, period);
        assert_eq!(balance(&w, "res.food"), before);
        assert_eq!(w.income(OWNER, id(&w, "res.food")).raw, 0);
    }
}

#[test]
fn commerce_cap_limits_capped_resources_only() {
    let mut w = setup(
        rules(
            "UPDATE game_constant SET value=4 WHERE key='player.commerce_cap.base';
        INSERT INTO et_gather_site(entity_type_id,resource_id,self_income)
        SELECT s.id,r.id,10 FROM def s,def r WHERE s.key='bld.library' AND r.key='res.knowledge';
        INSERT INTO modifier(source_def_id,stat_key,op,value)
        SELECT id,'commerce_cap','add',2 FROM def WHERE key='nation.romans';",
        ),
        &[],
    );
    working(&mut w);
    spawn(&mut w, "bld.library", at(30_000, 10_000));
    let food = balance(&w, "res.food");
    let knowledge = balance(&w, "res.knowledge");
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period);
    assert_eq!(balance(&w, "res.food") - food, 6);
    assert_eq!(balance(&w, "res.knowledge") - knowledge, 10);
    assert_eq!(
        w.income(OWNER, id(&w, "res.food")),
        Income {
            raw: 10,
            effective: 6,
            capped: true
        }
    );
    assert!(!w.income(OWNER, id(&w, "res.knowledge")).capped);
    assert_eq!(w.income(PlayerId(2), id(&w, "res.food")).raw, 0);
}

#[test]
fn income_modifier_applies() {
    let mut w = setup(rules("INSERT INTO def(kind,key,name) VALUES ('tech','tech.agriculture','Agriculture');
        INSERT INTO tech(id,kind,research_ticks) SELECT id,'upgrade',1 FROM def WHERE key='tech.agriculture';
        INSERT INTO production(producer_type_id,product_def_id,mode) SELECT p.id,t.id,'research' FROM def p,def t WHERE p.key='bld.library' AND t.key='tech.agriculture';
        INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'income.food','pct',50 FROM def WHERE key='tech.agriculture';"), &[]);
    working(&mut w);
    let library = spawn(&mut w, "bld.library", at(30_000, 10_000));
    w.submit(research(&w, library, "tech.agriculture")).unwrap();
    w.step();
    let before = balance(&w, "res.food");
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period);
    assert_eq!(w.income(OWNER, id(&w, "res.food")).effective, 15);
    assert_eq!(balance(&w, "res.food") - before, 15);
}

#[test]
fn self_income_site_pays_without_workers() {
    let mut w = setup(
        rules(
            "INSERT INTO et_gather_site(entity_type_id,resource_id,self_income)
        SELECT s.id,r.id,13 FROM def s,def r WHERE s.key='bld.library' AND r.key='res.food';",
        ),
        &[],
    );
    spawn(&mut w, "bld.library", at(10_000, 10_000));
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period);
    assert_eq!(balance(&w, "res.food"), 213);
    assert_eq!(w.income(OWNER, id(&w, "res.food")).raw, 13);
}

#[test]
fn idle_worker_takes_nearest_free_site_after_delay() {
    let mut w = world();
    let far = spawn(&mut w, "bld.farm", at(40_000, 10_000));
    let near = spawn(&mut w, "bld.farm", at(10_000, 10_000));
    let worker = spawn(&mut w, "unit.citizen", at(6500, 10_000));
    let delay = constant(&w, "citizen.auto_work_delay_ticks");
    ticks(&mut w, delay - 1);
    assert_eq!(w.entity(worker).unwrap().idle_ticks, delay - 1);
    assert!(w.entity(worker).unwrap().orders().is_empty());
    w.step();
    assert_eq!(w.entity(worker).unwrap().work_site, Some(near));
    assert_ne!(near, far);
}

#[test]
fn idle_workers_do_not_all_pick_the_same_slot() {
    let mut w = world();
    let first = spawn(&mut w, "bld.farm", at(20_000, 10_000));
    let second = spawn(&mut w, "bld.farm", at(40_000, 10_000));
    let workers = [
        spawn(&mut w, "unit.citizen", at(0, 10_000)),
        spawn(&mut w, "unit.citizen", at(0, 10_000)),
    ];
    let delay = constant(&w, "citizen.auto_work_delay_ticks");
    ticks(&mut w, delay);
    assert_eq!(
        w.entity(workers[0]).unwrap().orders()[0].target_entity,
        Some(first)
    );
    assert_eq!(
        w.entity(workers[1]).unwrap().orders()[0].target_entity,
        Some(second)
    );
    assert_eq!(w.entity(workers[0]).unwrap().work_site, None);
    ticks(&mut w, 600);
    assert_eq!(w.entity(workers[0]).unwrap().work_site, Some(first));
    assert_eq!(w.entity(workers[1]).unwrap().work_site, Some(second));
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn player_order_resets_idle_counter() {
    let mut w = world();
    let farm = spawn(&mut w, "bld.farm", at(10_000, 10_000));
    let worker = spawn(&mut w, "unit.citizen", at(6500, 10_000));
    let delay = constant(&w, "citizen.auto_work_delay_ticks");
    ticks(&mut w, delay - 1);
    w.submit(command(worker, "stop")).unwrap();
    assert_eq!(w.entity(worker).unwrap().idle_ticks, 0);
    w.step();
    ticks(&mut w, delay - 1);
    assert_eq!(w.entity(worker).unwrap().work_site, None);
    w.step();
    assert_eq!(w.entity(worker).unwrap().work_site, Some(farm));
}

#[test]
fn site_destroyed_frees_worker() {
    let mut w = world();
    let (worker, farm) = working(&mut w);
    let walking = spawn(&mut w, "unit.citizen", at(0, 0));
    let pending = spawn(&mut w, "unit.citizen", at(0, 0));
    w.submit(harvest(walking, farm)).unwrap();
    w.step();
    w.submit(harvest(pending, farm)).unwrap();
    w.despawn(farm).unwrap();
    assert_eq!(w.entity(worker).unwrap().work_site, None);
    assert!(w.entity(worker).unwrap().orders().is_empty());
    assert!(w.entity(walking).unwrap().orders().is_empty());
    assert_eq!(w.stats().orders_target_gone, 3);
    w.step();
    assert!(w.entity(pending).unwrap().orders().is_empty());
    assert_eq!(w.income(OWNER, id(&w, "res.food")).effective, 0);
}

#[test]
fn bootstrap_to_classical_age_using_only_orders() {
    let catalogue = rules("");
    let run = || {
        let mut w = setup(catalogue.clone(), &[]);
        spawn(&mut w, "bld.city", at(6500, 6500));
        let workers: Vec<_> = (0..5)
            .map(|n| spawn(&mut w, "unit.citizen", at(7000 + n * 10_000, 20_000)))
            .collect();
        for (worker, key, pos) in [
            (workers[0], "bld.farm", at(10_000, 20_000)),
            (workers[1], "bld.farm", at(20_000, 20_000)),
            (workers[2], "bld.library", at(30_000, 20_000)),
        ] {
            w.submit(construct(&w, worker, key, pos)).unwrap();
        }
        ticks(&mut w, 150);
        let farms: Vec<_> = w
            .entities()
            .filter(|e| e.type_id == id(&w, "bld.farm"))
            .map(|e| e.id)
            .collect();
        for (worker, farm) in workers.iter().zip(farms) {
            w.submit(harvest(*worker, farm)).unwrap();
        }
        ticks(&mut w, 300);
        assert_eq!(balance(&w, "res.food"), 250);
        let library = w
            .entities()
            .find(|e| e.type_id == id(&w, "bld.library"))
            .unwrap()
            .id;
        w.submit(research(&w, library, "tech.age.classical"))
            .unwrap();
        ticks(&mut w, 600);
        assert_eq!(w.players().next().unwrap().age, id(&w, "age.classical"));
        assert_eq!(w.stats().orders_failed, 0);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn idle_search_breaks_distance_ties_by_id_and_honors_existing_walkers() {
    for claimed in [false, true] {
        let mut w = world();
        let first = spawn(&mut w, "bld.farm", at(20_000, 10_000));
        let second = spawn(&mut w, "bld.farm", at(20_000, 30_000));
        if claimed {
            let walking = spawn(&mut w, "unit.citizen", at(0, 0));
            w.submit(harvest(walking, first)).unwrap();
        }
        let idle = spawn(&mut w, "unit.citizen", at(0, 20_000));
        let delay = constant(&w, "citizen.auto_work_delay_ticks");
        ticks(&mut w, delay);
        assert_eq!(
            w.entity(idle).unwrap().orders()[0].target_entity,
            Some(if claimed { second } else { first })
        );
    }
}

#[test]
fn despawning_worker_releases_slot_and_stops_income() {
    let mut w = world();
    let (worker, farm) = working(&mut w);
    w.despawn(worker).unwrap();
    assert_eq!(w.income(OWNER, id(&w, "res.food")).raw, 0);
    let replacement = spawn(&mut w, "unit.citizen", at(6500, 10_000));
    w.submit(harvest(replacement, farm)).unwrap();
    w.step();
    assert_eq!(w.entity(replacement).unwrap().work_site, Some(farm));
    assert_eq!(w.stats().orders_no_slot, 0);
}

#[test]
fn income_remainders_are_independent_per_player_and_resource() {
    let mut w = setup(
        rules(
            "INSERT INTO et_gather_site(entity_type_id,resource_id,self_income)
        SELECT s.id,r.id,13 FROM def s,def r WHERE s.key='bld.library' AND r.key='res.knowledge';",
        ),
        &[],
    );
    working(&mut w);
    spawn(&mut w, "bld.library", at(30_000, 10_000));
    let other = w
        .spawn(id(&w, "bld.library"), PlayerId(2), at(40_000, 10_000))
        .unwrap();
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period / 2);
    assert_eq!(balance(&w, "res.food"), 205);
    assert_eq!(balance(&w, "res.knowledge"), 6);
    let other_balance = |w: &World| {
        w.players().find(|p| p.id == PlayerId(2)).unwrap().resources[&id(w, "res.knowledge")]
    };
    assert_eq!(other_balance(&w), 6);
    w.despawn(other).unwrap();
    ticks(&mut w, period / 2);
    assert_eq!(balance(&w, "res.food"), 210);
    assert_eq!(balance(&w, "res.knowledge"), 13);
    assert_eq!(other_balance(&w), 6);
}

#[test]
fn missing_income_stat_preserves_raw_and_incomplete_self_income_does_not_pay() {
    let mut w = setup(
        rules(
            "DELETE FROM stat WHERE key='income.food';
        INSERT INTO et_gather_site(entity_type_id,resource_id,self_income,slots_base,slots_max)
        SELECT s.id,r.id,600,0,0 FROM def s,def r WHERE s.key='bld.library' AND r.key='res.food';",
        ),
        &[],
    );
    town(&mut w);
    let worker = spawn(&mut w, "unit.citizen", at(7000, 10_000));
    w.submit(construct(&w, worker, "bld.library", at(10_000, 10_000)))
        .unwrap();
    ticks(&mut w, 399);
    assert_eq!(balance(&w, "res.food"), 200);
    assert_eq!(w.income(OWNER, id(&w, "res.food")).raw, 0);
    w.step();
    assert_eq!(
        w.income(OWNER, id(&w, "res.food")),
        Income {
            raw: 600,
            effective: 70,
            capped: true
        }
    );
    let period = constant(&w, "economy.income_period_ticks");
    ticks(&mut w, period);
    assert_eq!(balance(&w, "res.food"), 270);
}

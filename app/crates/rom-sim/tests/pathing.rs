//! Path search, path following, group moves and local avoidance.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, OrderStatus, Player, PlayerId, Pos, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);

fn rules_with(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn rules() -> Arc<Rules> {
    rules_with("")
}

/// Rows of `.` plain, `#` mountain (impassable on foot) and `f` forest; row 0 is y = 0.
fn world_on(rules: Arc<Rules>, rows: &[&str]) -> World {
    let id = |key: &str| rules.id_of(key).unwrap();
    let terrain = rows
        .iter()
        .flat_map(|row| row.chars())
        .map(|c| match c {
            '#' => id("ter.mountain"),
            'f' => id("ter.forest"),
            _ => id("ter.plain"),
        })
        .collect();
    let player = Player {
        id: OWNER,
        team: 1,
        age: id("age.ancient"),
        faction: id("nation.romans"),
        resources: rules.resources.keys().map(|r| (*r, 10_000)).collect(),
        techs: BTreeMap::new(),
    };
    let map = MapGrid::new(rows[0].len() as u32, rows.len() as u32, terrain).unwrap();
    World::new(rules, map, 7, vec![player]).unwrap()
}

/// A square map of plain tiles with `#` wherever `blocked` says so.
fn rows_where(size: usize, blocked: impl Fn(usize, usize) -> bool) -> Vec<String> {
    (0..size)
        .map(|y| {
            (0..size)
                .map(|x| if blocked(x, y) { '#' } else { '.' })
                .collect()
        })
        .collect()
}

fn as_str(rows: &[String]) -> Vec<&str> {
    rows.iter().map(String::as_str).collect()
}

fn at(x: i32, y: i32) -> Pos {
    Pos { x, y }
}

fn tile(x: i32, y: i32) -> Pos {
    at(x * 1000 + 500, y * 1000 + 500)
}

fn tile_of(pos: Pos) -> (i32, i32) {
    (pos.x.div_euclid(1000), pos.y.div_euclid(1000))
}

fn spawn(w: &mut World, key: &str, pos: Pos) -> EntityId {
    w.spawn(w.rules().id_of(key).unwrap(), OWNER, pos).unwrap()
}

fn command(executor: EntityId, kind: &str) -> Command {
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

fn move_to(w: &mut World, id: EntityId, target: Pos) {
    let mut c = command(id, "move");
    c.target_pos = Some(target);
    w.submit(c).unwrap();
}

fn terrain_at(w: &World, pos: Pos) -> DefId {
    w.map().terrain_at_pos(pos).unwrap()
}

/// Steps until `id` has no orders, recording its position after every tick.
fn walk(w: &mut World, id: EntityId, limit: u64) -> Vec<Pos> {
    let mut trail = vec![w.entity(id).unwrap().pos];
    for _ in 0..limit {
        w.step();
        trail.push(w.entity(id).unwrap().pos);
        if w.entity(id).unwrap().orders().is_empty() {
            return trail;
        }
    }
    panic!("unit {id:?} still walking after {limit} ticks");
}

fn assert_trail_avoids(w: &World, trail: &[Pos], blocked: DefId) {
    for pos in trail {
        assert_ne!(
            terrain_at(w, *pos),
            blocked,
            "stood on blocked tile at {pos:?}"
        );
    }
}

#[test]
fn path_goes_around_a_wall() {
    let rows = rows_where(20, |x, y| x == 10 && y < 16);
    let mut w = world_on(rules(), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(5, 5));
    move_to(&mut w, id, tile(15, 5));
    let trail = walk(&mut w, id, 2000);
    assert_eq!(*trail.last().unwrap(), tile(15, 5));
    assert_trail_avoids(&w, &trail, w.rules().id_of("ter.mountain").unwrap());
    assert!(trail.iter().any(|p| p.y >= 16_000), "went below the wall");
    assert_eq!(w.stats().orders_done, 1);
    assert_eq!(w.stats().orders_failed, 0);
    assert_eq!(w.path_stats().searches, 1);
}

#[test]
fn path_goes_through_the_only_gap() {
    let rows = rows_where(20, |x, y| x == 10 && y != 12);
    let mut w = world_on(rules(), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(3, 3));
    move_to(&mut w, id, tile(16, 3));
    let trail = walk(&mut w, id, 2000);
    assert_eq!(*trail.last().unwrap(), tile(16, 3));
    assert!(trail.iter().any(|p| tile_of(*p) == (10, 12)));
    assert_trail_avoids(&w, &trail, w.rules().id_of("ter.mountain").unwrap());
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn unreachable_goal_paths_to_nearest_reachable_tile() {
    let rows = rows_where(20, |x, y| (10..15).contains(&x) && (8..13).contains(&y));
    let mut w = world_on(rules(), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(2, 10));
    move_to(&mut w, id, tile(12, 10));
    let trail = walk(&mut w, id, 2000);
    let end = tile_of(*trail.last().unwrap());
    // The block's centre is three tiles from its nearest open neighbours.
    assert_eq!((end.0 - 12).pow(2) + (end.1 - 10).pow(2), 9);
    assert_eq!(*trail.last().unwrap(), tile(end.0, end.1));
    assert_eq!(w.stats().orders_done, 1);
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn enclosed_unit_fails_no_path() {
    let rows = rows_where(12, |x, y| {
        (4..=6).contains(&x) && (4..=6).contains(&y) && (x, y) != (5, 5)
    });
    let mut w = world_on(rules(), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(5, 5));
    move_to(&mut w, id, tile(10, 10));
    w.step();
    assert_eq!(w.entity(id).unwrap().pos, tile(5, 5));
    assert!(w.entity(id).unwrap().orders().is_empty());
    assert_eq!(w.stats().orders_no_path, 1);
    assert_eq!(w.stats().orders_failed, 1);
}

#[test]
fn path_prefers_fast_terrain_when_cheaper() {
    // A thick wall with two tunnels: the direct one is forest, the longer one plain.
    let rows: Vec<String> = (0..16)
        .map(|y| {
            (0..31)
                .map(|x| match (x, y) {
                    (8..=22, 10) => 'f',
                    (8..=22, 6) => '.',
                    (8..=22, _) => '#',
                    _ => '.',
                })
                .collect()
        })
        .collect();
    let mut w = world_on(rules(), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(3, 10));
    move_to(&mut w, id, tile(27, 12));
    let trail = walk(&mut w, id, 3000);
    assert_eq!(*trail.last().unwrap(), tile(27, 12));
    let inside = |y: i32| {
        trail.iter().any(|p| {
            let (tx, ty) = tile_of(*p);
            (8..=22).contains(&tx) && ty == y
        })
    };
    assert!(inside(6), "took the plain tunnel");
    assert!(!inside(10), "avoided the forest tunnel");
    // Without the forest the direct tunnel is shorter and wins.
    let plain: Vec<String> = rows.iter().map(|r| r.replace('f', ".")).collect();
    let mut w = world_on(rules(), &as_str(&plain));
    let id = spawn(&mut w, "unit.citizen", tile(3, 10));
    move_to(&mut w, id, tile(27, 12));
    let trail = walk(&mut w, id, 3000);
    assert!(
        trail
            .iter()
            .any(|p| tile_of(*p).1 == 10 && (8..=22).contains(&tile_of(*p).0))
    );
}

fn budgeted_arrival(sql: &str) -> (u64, World) {
    let rows = rows_where(20, |x, y| x == 10 && y < 16);
    let mut w = world_on(rules_with(sql), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(5, 5));
    move_to(&mut w, id, tile(15, 5));
    w.step();
    assert_eq!(w.entity(id).unwrap().pos, tile(5, 5));
    assert_eq!(
        w.entity(id).unwrap().orders()[0].status,
        OrderStatus::Active
    );
    let mut nodes = Vec::new();
    while !w.entity(id).unwrap().orders().is_empty() {
        let before = w.path_stats().nodes;
        w.step();
        nodes.push(w.path_stats().nodes - before);
        assert!(w.tick() < 3000);
    }
    assert_eq!(w.entity(id).unwrap().pos, tile(15, 5));
    let limit = w.rules().constant("path.nodes_per_tick").unwrap_or(20_000) as u64;
    assert!(nodes.iter().all(|n| *n <= limit));
    (w.tick(), w)
}

#[test]
fn search_is_spread_over_ticks_under_a_small_budget() {
    let (normal, _) = budgeted_arrival("");
    let (slow, w) =
        budgeted_arrival("UPDATE game_constant SET value=5 WHERE key='path.nodes_per_tick';");
    assert!(w.path_stats().nodes > 5 * 3, "search needed several ticks");
    assert!(slow > normal + 3, "arrived later: {slow} vs {normal}");
    assert_eq!(w.path_stats().searches, 1);
    // Without the row the default budget applies.
    let (absent, _) =
        budgeted_arrival("DELETE FROM game_constant WHERE key='path.nodes_per_tick';");
    assert_eq!(absent, normal);
}

#[test]
fn flyer_ignores_terrain_and_buildings() {
    let rows = rows_where(20, |x, _| x == 8);
    let mut w = world_on(rules(), &as_str(&rows));
    spawn(&mut w, "bld.farm", at(13_000, 5_000));
    let drone = spawn(&mut w, "unit.repair_drone", at(2_500, 5_500));
    move_to(&mut w, drone, at(17_500, 5_500));
    let trail = walk(&mut w, drone, 1000);
    // 15,000 milli-tiles at 2,600 per second, 20 ticks per second.
    assert_eq!(trail.len() - 1, 116);
    assert_eq!(*trail.last().unwrap(), at(17_500, 5_500));
    assert!(trail.iter().any(|p| tile_of(*p).0 == 8));
    assert!(trail.iter().any(|p| (11..15).contains(&tile_of(*p).0)));
    assert_eq!(w.path_stats().searches, 0);
}

#[test]
fn new_building_on_the_path_triggers_repath() {
    let rows = rows_where(30, |_, _| false);
    let mut w = world_on(rules(), &as_str(&rows));
    let id = spawn(&mut w, "unit.citizen", tile(2, 10));
    move_to(&mut w, id, tile(27, 10));
    for _ in 0..30 {
        w.step();
    }
    assert_eq!(w.path_stats().searches, 0);
    spawn(&mut w, "bld.barracks", at(15_000, 10_000));
    let trail = walk(&mut w, id, 2000);
    assert_eq!(*trail.last().unwrap(), tile(27, 10));
    assert!(w.path_stats().repaths >= 1);
    for pos in &trail {
        let (x, y) = tile_of(*pos);
        assert!(!((13..17).contains(&x) && (8..12).contains(&y)), "{pos:?}");
    }
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn builder_walks_around_obstacles_to_its_site() {
    let rows = rows_where(30, |x, y| x == 10 && y < 25);
    let mut w = world_on(rules(), &as_str(&rows));
    let town = w.rules().id_of("bld.city").unwrap();
    w.place_for_setup(town, OWNER, at(26_500, 20_500)).unwrap();
    let builder = spawn(&mut w, "unit.citizen", tile(5, 5));
    let mut c = command(builder, "construct");
    c.subject = w.rules().id_of("bld.farm");
    c.target_pos = Some(at(20_000, 5_000));
    w.submit(c).unwrap();
    let trail = walk(&mut w, builder, 4000);
    assert_trail_avoids(&w, &trail, w.rules().id_of("ter.mountain").unwrap());
    let farm = w
        .entities()
        .find(|e| e.type_id == w.rules().id_of("bld.farm").unwrap())
        .unwrap();
    assert_eq!(farm.build_progress, None);
    assert_eq!(w.stats().orders_failed, 0);
    assert!(w.path_stats().searches >= 1);
}

#[test]
fn worker_reaches_farm_behind_a_wall() {
    let rows = rows_where(30, |x, y| x == 10 && y > 3);
    let mut w = world_on(rules(), &as_str(&rows));
    let farm = spawn(&mut w, "bld.farm", at(20_000, 20_000));
    let worker = spawn(&mut w, "unit.citizen", tile(5, 20));
    let mut c = command(worker, "harvest");
    c.target_entity = Some(farm);
    w.submit(c).unwrap();
    for _ in 0..2000 {
        w.step();
        if w.entity(worker).unwrap().work_site.is_some() {
            break;
        }
    }
    assert_eq!(w.entity(worker).unwrap().work_site, Some(farm));
    let food = w.rules().id_of("res.food").unwrap();
    assert_eq!(w.income(OWNER, food).raw, 10);
    assert!(w.path_stats().searches >= 1);
    assert_eq!(w.stats().orders_failed, 0);
}

fn group(size: i32, rows: &[&str], target: Pos) -> (World, Vec<EntityId>) {
    let mut w = world_on(rules(), rows);
    let units: Vec<_> = (0..size)
        .map(|n| {
            spawn(
                &mut w,
                "unit.citizen",
                at(4_200 + (n % 4) * 900, 4_200 + (n / 4) * 900),
            )
        })
        .collect();
    for id in &units {
        move_to(&mut w, *id, target);
    }
    (w, units)
}

#[test]
fn group_shares_one_search() {
    let rows = rows_where(30, |x, y| x == 15 && y < 26);
    let (mut w, units) = group(16, &as_str(&rows), tile(24, 5));
    w.step();
    assert_eq!(w.path_stats().searches, 1);
    for _ in 0..3000 {
        if units
            .iter()
            .all(|id| w.entity(*id).unwrap().orders().is_empty())
        {
            break;
        }
        w.step();
    }
    assert_eq!(w.stats().orders_done, 16);
    assert_eq!(w.stats().orders_failed, 0);
    assert_eq!(w.path_stats().searches, 1);
    for id in &units {
        let pos = w.entity(*id).unwrap().pos;
        assert!(pos.distance(tile(24, 5)) < 3_000, "{pos:?}");
    }
}

#[test]
fn group_spreads_at_goal() {
    let rows = rows_where(30, |_, _| false);
    let goal = tile(20, 20);
    let (mut w, units) = group(16, &as_str(&rows), goal);
    for _ in 0..600 {
        w.step();
    }
    assert!(
        units
            .iter()
            .all(|id| w.entity(*id).unwrap().orders().is_empty())
    );
    let spots: Vec<_> = units.iter().map(|id| w.entity(*id).unwrap().pos).collect();
    for (i, a) in spots.iter().enumerate() {
        for b in &spots[i + 1..] {
            assert_ne!(a, b);
        }
    }
    // 16 units fill two square rings; spacing is two radii plus a margin.
    let radius = w
        .rules()
        .entity_types
        .values()
        .filter(|et| et.movement.is_some())
        .map(|et| et.radius)
        .max()
        .unwrap();
    let spacing = 2 * radius + 100;
    let bound = (2 * spacing) as u64 * 3 / 2 + radius as u64;
    assert!(spots.iter().all(|p| p.distance(goal) <= bound), "{spots:?}");
    assert!(
        spots
            .iter()
            .filter(|p| p.distance(goal) > radius as u64)
            .count()
            >= 8
    );
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn overlapping_units_are_pushed_apart() {
    let rows = rows_where(20, |_, _| false);
    let mut w = world_on(rules(), &as_str(&rows));
    let ids: Vec<_> = [at(10_000, 10_000), at(10_000, 10_000), at(10_100, 10_000)]
        .into_iter()
        .map(|p| spawn(&mut w, "unit.citizen", p))
        .collect();
    for _ in 0..100 {
        w.step();
    }
    let spots: Vec<_> = ids.iter().map(|id| w.entity(*id).unwrap().pos).collect();
    let radius = w
        .rules()
        .entity_type(w.entity(ids[0]).unwrap().type_id)
        .unwrap()
        .radius as u64;
    for (i, a) in spots.iter().enumerate() {
        for b in &spots[i + 1..] {
            assert!(a.distance(*b) >= 2 * radius, "{a:?} {b:?}");
        }
    }
    // Once separated they rest.
    w.step();
    assert_eq!(
        ids.iter()
            .map(|id| w.entity(*id).unwrap().pos)
            .collect::<Vec<_>>(),
        spots
    );
    assert!(spots.iter().all(|p| p.distance(at(10_000, 10_000)) < 1_000));
}

#[test]
fn push_never_enters_impassable_tile() {
    // A crowd piled against a wall and the map edge.
    let rows = rows_where(20, |x, y| x == 3 || (x == 2 && y == 2));
    let mut w = world_on(rules(), &as_str(&rows));
    let mountain = w.rules().id_of("ter.mountain").unwrap();
    let ids: Vec<_> = (0..20)
        .map(|n| spawn(&mut w, "unit.citizen", at(2_950 - n, 1_050 + n)))
        .collect();
    for _ in 0..300 {
        w.step();
        for id in &ids {
            let pos = w.entity(*id).unwrap().pos;
            assert_ne!(terrain_at(&w, pos), mountain, "{pos:?}");
        }
    }
    let spread = ids
        .iter()
        .map(|id| w.entity(*id).unwrap().pos.distance(at(2_950, 1_050)))
        .max()
        .unwrap();
    assert!(spread > 500, "crowd spread out along the wall");
}

fn obstacle_run(seed: u64) -> World {
    let mut w = World::obstacle_scenario(rules(), seed, 1000, 20).unwrap();
    for _ in 0..1200 {
        w.step();
    }
    w
}

#[test]
fn same_seed_same_hash_with_obstacles() {
    let first = obstacle_run(7);
    let second = obstacle_run(7);
    assert_eq!(first.hash(), second.hash());
    assert_eq!(first.stats(), second.stats());
    assert_eq!(first.path_stats(), second.path_stats());
    assert!(first.path_stats().searches > 100);
    assert!(first.stats().orders_done > 100);
    let rules = first.rules();
    let class = rules
        .entity_type(first.entities().next().unwrap().type_id)
        .unwrap();
    let rows = &rules.movement_class_terrain[&class.movement.as_ref().unwrap().movement_class_id];
    assert!(first.entities().all(|e| {
        let terrain = terrain_at(&first, e.pos);
        rows.iter()
            .any(|r| r.terrain_id == terrain && r.speed_pct > 0)
    }));
    let blocked = first
        .map()
        .terrain
        .iter()
        .filter(|t| !rows.iter().any(|r| r.terrain_id == **t))
        .count();
    assert!(
        (2_800..3_800).contains(&blocked),
        "about a fifth blocked: {blocked}"
    );
    assert_ne!(first.hash(), obstacle_run(8).hash());
}

//! Formations (task 143): one player's orders of one tick that name a formation and share a goal
//! lay the group out about it, ranks facing the way it goes, close fighters in front of ranged
//! and support members. Each scenario runs twice from the same start and must end alike.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// A 64 × 64 plain.
fn world() -> World {
    world_with_pond(false)
}

/// A 64 × 64 plain, with deep water over x 36..46, y 34..38 when `pond` is set.
fn world_with_pond(pond: bool) -> World {
    let rules = rules();
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
            let (x, y) = (i % 64, i / 64);
            if pond && (36..46).contains(&x) && (34..38).contains(&y) {
                deep
            } else {
                plain
            }
        })
        .collect();
    World::new(
        Arc::clone(&rules),
        MapGrid::new(64, 64, terrain).unwrap(),
        11,
        vec![player(0), player(1), player(2)],
    )
    .unwrap()
}

fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
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

/// Three spearmen, three archers and three priests standing in a loose crowd at the west,
/// sent east to (`goal_x`, 32.5) in `formation` with `kind`, one order each in one tick.
fn march(w: &mut World, formation: &str, kind: &str) -> Vec<(EntityId, &'static str)> {
    let mut units = Vec::new();
    let mixed = [
        "unit.priest",
        "unit.archer",
        "unit.spearman",
        "unit.archer",
        "unit.priest",
        "unit.spearman",
        "unit.spearman",
        "unit.priest",
        "unit.archer",
    ];
    for (n, key) in mixed.into_iter().enumerate() {
        let at = Pos::flat(8_500 + (n as i32 % 3) * 900, 31_600 + (n as i32 / 3) * 900);
        units.push((w.spawn(id(w, key), OWNER, at).unwrap(), key));
    }
    let formation = id(w, formation);
    for (unit, key) in &units {
        let kind = if *key == "unit.priest" { "move" } else { kind };
        w.submit(Command {
            player: OWNER,
            executor: *unit,
            kind: kind.into(),
            subject: Some(formation),
            target_entity: None,
            target_pos: Some(Pos::flat(40_500, 32_500)),
            queue: false,
            text: None,
            facing: None,
        })
        .unwrap();
    }
    units
}

fn xs(w: &World, units: &[(EntityId, &str)], key: &str) -> Vec<i32> {
    units
        .iter()
        .filter(|(_, k)| *k == key)
        .map(|(u, _)| w.entity(*u).unwrap().pos.x)
        .collect()
}

/// Every pair of the group stands at least `apart` milli-tiles apart.
fn spread(w: &World, units: &[(EntityId, &str)], apart: u64) {
    for (a, _) in units {
        for (b, _) in units {
            if a < b {
                let (p, q) = (w.entity(*a).unwrap().pos, w.entity(*b).unwrap().pos);
                assert!(w.distance(p, q).unwrap() >= apart, "{p:?} {q:?}");
            }
        }
    }
}

#[test]
fn a_square_faces_the_way_it_goes_with_fighters_in_front() {
    let w = twice(|| {
        let mut w = world();
        let units = march(&mut w, "form.square", "attack_move");
        ticks(&mut w, 800);
        // Three ranks of three across the line of travel (east): spearmen in front, archers
        // behind them, the unarmed priests at the back.
        let (front, middle, back) = (
            xs(&w, &units, "unit.spearman"),
            xs(&w, &units, "unit.archer"),
            xs(&w, &units, "unit.priest"),
        );
        assert!(
            front.iter().min() > middle.iter().max(),
            "{front:?} {middle:?}"
        );
        assert!(
            middle.iter().min() > back.iter().max(),
            "{middle:?} {back:?}"
        );
        // Each rank straight across, the ranks a spacing apart, the centre on the goal.
        for rank in [&front, &middle, &back] {
            assert!(rank.iter().max().unwrap() - rank.iter().min().unwrap() <= 60);
        }
        assert!((middle[0] - 40_500).abs() <= 60, "{middle:?}");
        spread(&w, &units, 700);
        // The batch keeps to its slowest member's pace; the walk is done.
        assert!(
            units
                .iter()
                .all(|(u, _)| w.entity(*u).unwrap().orders().is_empty())
        );
        w
    });
    assert_eq!(w.tick(), 800);
}

#[test]
fn a_line_is_one_rank_and_a_double_line_two() {
    let w = twice(|| {
        let mut w = world();
        let units = march(&mut w, "form.line", "attack_move");
        ticks(&mut w, 900);
        let all: Vec<i32> = units
            .iter()
            .map(|(u, _)| w.entity(*u).unwrap().pos.x)
            .collect();
        assert!(
            all.iter().max().unwrap() - all.iter().min().unwrap() <= 60,
            "{all:?}"
        );
        let ys: Vec<i32> = units
            .iter()
            .map(|(u, _)| w.entity(*u).unwrap().pos.y)
            .collect();
        assert!(ys.iter().max().unwrap() - ys.iter().min().unwrap() >= 8 * 700);
        w
    });
    assert_eq!(w.tick(), 900);
    let w = twice(|| {
        let mut w = world();
        let units = march(&mut w, "form.double_line", "move");
        ticks(&mut w, 900);
        let mut ranks: Vec<i32> = units
            .iter()
            .map(|(u, _)| w.entity(*u).unwrap().pos.x)
            .collect();
        ranks.sort();
        // Five in front (east), four behind, one spacing apart.
        assert!(
            ranks[3] - ranks[0] <= 60 && ranks[8] - ranks[4] <= 60,
            "{ranks:?}"
        );
        assert!(ranks[4] - ranks[3] >= 700, "{ranks:?}");
        spread(&w, &units, 700);
        w
    });
    assert_eq!(w.tick(), 900);
}

#[test]
fn a_circle_rings_the_goal_and_compact_is_tighter_than_square() {
    let w = twice(|| {
        let mut w = world();
        let units = march(&mut w, "form.circle", "move");
        ticks(&mut w, 900);
        let goal = Pos::flat(40_500, 32_500);
        let radii: Vec<u64> = units
            .iter()
            .map(|(u, _)| w.distance(w.entity(*u).unwrap().pos, goal).unwrap())
            .collect();
        assert!(
            radii.iter().max().unwrap() - radii.iter().min().unwrap() <= 80,
            "{radii:?}"
        );
        spread(&w, &units, 600);
        w
    });
    assert_eq!(w.tick(), 900);
    let width = |formation: &str| {
        let mut w = world();
        let units = march(&mut w, formation, "move");
        ticks(&mut w, 900);
        let ys: Vec<i32> = units
            .iter()
            .map(|(u, _)| w.entity(*u).unwrap().pos.y)
            .collect();
        ys.iter().max().unwrap() - ys.iter().min().unwrap()
    };
    assert!(width("form.compact") < width("form.square"));
    assert!(width("form.square") < width("form.sparse"));
}

#[test]
fn the_formation_is_the_orders_and_survives_a_snapshot() {
    let mut w = world();
    let units = march(&mut w, "form.square", "move");
    w.step();
    // Each order holds its own place, and the save carries it.
    let mut goals: Vec<Pos> = units
        .iter()
        .map(|(u, _)| w.entity(*u).unwrap().orders()[0].target_pos.unwrap())
        .collect();
    goals.sort_by_key(|p| (p.x, p.y));
    goals.dedup();
    assert_eq!(goals.len(), units.len());
    let restored = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(restored.hash(), w.hash());
    // A formation of one walks to the goal itself; a subject that is no formation is refused.
    let alone = w
        .spawn(id(&w, "unit.archer"), OWNER, Pos::flat(5_500, 5_500))
        .unwrap();
    let mut order = Command {
        player: OWNER,
        executor: alone,
        kind: "patrol".into(),
        subject: Some(id(&w, "unit.archer")),
        target_entity: None,
        target_pos: Some(Pos::flat(9_500, 5_500)),
        queue: false,
        text: None,
        facing: None,
    };
    assert_eq!(w.submit(order.clone()), Err(SimError::UnknownFormation));
    order.kind = "move".into();
    order.subject = Some(id(&w, "form.square"));
    w.submit(order).unwrap();
    w.step();
    assert_eq!(
        w.entity(alone).unwrap().orders()[0].target_pos,
        Some(Pos::flat(9_500, 5_500))
    );
}

#[test]
fn a_formation_turned_about_marches_back_in_order() {
    let w = twice(|| {
        let mut w = world();
        let units = march(&mut w, "form.square", "attack_move");
        ticks(&mut w, 800);
        // Back west: the spearmen, now at the back, come through to the new front.
        let square = id(&w, "form.square");
        for (unit, key) in &units {
            let kind = if *key == "unit.priest" {
                "move"
            } else {
                "attack_move"
            };
            w.submit(Command {
                player: OWNER,
                executor: *unit,
                kind: kind.into(),
                subject: Some(square),
                target_entity: None,
                target_pos: Some(Pos::flat(12_500, 32_500)),
                queue: false,
                text: None,
                facing: None,
            })
            .unwrap();
        }
        ticks(&mut w, 800);
        let (front, middle, back) = (
            xs(&w, &units, "unit.spearman"),
            xs(&w, &units, "unit.archer"),
            xs(&w, &units, "unit.priest"),
        );
        assert!(
            front.iter().max() < middle.iter().min(),
            "{front:?} {middle:?}"
        );
        assert!(
            middle.iter().max() < back.iter().min(),
            "{middle:?} {back:?}"
        );
        spread(&w, &units, 700);
        w
    });
    assert_eq!(w.tick(), 1600);
}

#[test]
fn a_place_on_water_moves_to_the_nearest_free_ground() {
    let w = twice(|| {
        let mut w = world_with_pond(true);
        let units = march(&mut w, "form.line", "move");
        ticks(&mut w, 1000);
        // The line runs north and south across the goal; its southern places lie in the pond,
        // and those members stand on the nearest dry spots of the lattice instead.
        for (unit, _) in &units {
            let at = w.entity(*unit).unwrap().pos;
            assert!(!(36_000..46_000).contains(&at.x) || !(34_000..38_000).contains(&at.y));
        }
        spread(&w, &units, 700);
        w
    });
    assert_eq!(w.tick(), 1000);
}

use rom_rules::{Rules, build_rules_db};
use rom_sim::{Command, PlayerId, Pos, World};
use std::sync::Arc;

fn rules() -> Arc<Rules> {
    rules_with("")
}

fn rules_with(extra: &str) -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(extra).unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

fn finish(mut world: World) -> World {
    for _ in 0..2400 {
        world.step();
    }
    world
}

fn survivors(world: &World, swap: bool) -> [usize; 2] {
    let mut counts = [0; 2];
    for e in world.entities() {
        let side = if swap { 2 - e.owner.0 } else { e.owner.0 - 1 };
        counts[side as usize] += 1;
    }
    counts
}

// Rebuild the CLI's two lines with independently sized armies and allocation order.
fn lines(rules: Arc<Rules>, seed: u64, sizes: [u32; 2], reverse: bool) -> World {
    let type_id = rules
        .entity_types
        .values()
        .find(|et| et.movement.is_some() && !et.weapons.is_empty())
        .unwrap()
        .id;
    let mut w = World::battle_scenario(rules, seed, 0).unwrap();
    for side in if reverse { [1, 0] } else { [0, 1] } {
        let (x, target) = if side == 0 {
            (33000, 63000)
        } else {
            (63000, 33000)
        };
        let owner = PlayerId(side as u32 + 1);
        for row in 0..sizes[side] {
            let y = (1000 + u64::from(row) * 94000 / u64::from(sizes[side])) as i32;
            let executor = w.spawn(type_id, owner, Pos { x, y, face: 0 }).unwrap();
            w.submit(Command {
                player: owner,
                executor,
                kind: "attack_move".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(Pos {
                    x: target,
                    y,
                    face: 0,
                }),
                queue: false,
            })
            .unwrap();
        }
    }
    w
}

// Compare physical state, including the target's physical location, before deaths
// make an all-zero final survivor comparison vacuous. Ids intentionally differ.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Fighter {
    side: u32,
    pos: (i32, i32),
    hp: i64,
    target: Option<(i32, i32)>,
}

fn physical_state(w: &World, swap: bool) -> Vec<Fighter> {
    let mut state: Vec<_> = w
        .entities()
        .map(|e| {
            let side = if swap { 3 - e.owner.0 } else { e.owner.0 };
            let target = e
                .orders()
                .front()
                .and_then(|o| o.target_entity)
                .and_then(|id| w.entity(id))
                .map(|e| (e.pos.x, e.pos.y));
            Fighter {
                side,
                pos: (e.pos.x, e.pos.y),
                hp: e.hp,
                target,
            }
        })
        .collect();
    state.sort_unstable();
    state
}

#[test]
fn symmetric_battle_is_close() {
    let rules = rules();
    for seed in 1..=10 {
        let w = finish(World::battle_scenario(rules.clone(), seed, 200).unwrap());
        let alive = survivors(&w, false);
        assert!(alive.into_iter().all(|n| n <= 70), "seed {seed}: {alive:?}");
        for (side, count) in alive.into_iter().enumerate() {
            let owner = PlayerId(side as u32 + 1);
            assert_eq!(w.stats().losses[&owner] + count as u64, 200);
            assert!(w.stats().kills[&owner] >= 130);
        }
    }
}

#[test]
fn outcome_does_not_depend_on_player_order() {
    let rules = rules();
    for seed in 1..=5 {
        let mut a = World::battle_scenario(rules.clone(), seed, 200).unwrap();
        let mut b = World::battle_scenario_swapped(rules.clone(), seed, 200, true).unwrap();
        for tick in 0..2400 {
            a.step();
            b.step();
            if tick % 100 == 0 {
                assert_eq!(
                    physical_state(&a, false),
                    physical_state(&b, true),
                    "seed {seed}, tick {tick}"
                );
            }
        }
        assert_eq!(survivors(&a, false), survivors(&b, true), "seed {seed}");
        for owner in 1..=2 {
            assert_eq!(
                a.stats().kills.get(&PlayerId(owner)),
                b.stats().kills.get(&PlayerId(3 - owner))
            );
        }
    }
}

#[test]
fn outcome_does_not_depend_on_spawn_order() {
    let rules = rules();
    for (seed, sizes) in (1..=5)
        .map(|s| (s, [200, 200]))
        .chain([(3, [199, 199]), (3, [200, 120])])
    {
        let mut a = lines(rules.clone(), seed, sizes, false);
        let mut b = lines(rules.clone(), seed, sizes, true);
        for tick in 0..2400 {
            a.step();
            b.step();
            if tick % 100 == 0 {
                assert_eq!(
                    physical_state(&a, false),
                    physical_state(&b, false),
                    "seed {seed}, sizes {sizes:?}, tick {tick}"
                );
            }
        }
        assert_eq!(
            survivors(&a, false),
            survivors(&b, false),
            "seed {seed}, sizes {sizes:?}"
        );
        assert_eq!(a.stats().kills, b.stats().kills);
    }
}

#[test]
fn stronger_side_still_wins() {
    let rules = rules();
    for sizes in [[200, 120], [120, 200]] {
        let w = finish(lines(rules.clone(), 3, sizes, false));
        let alive = survivors(&w, false);
        let winner = usize::from(sizes[1] > sizes[0]);
        assert!(alive[winner] >= 60, "sizes {sizes:?}: {alive:?}");
        assert_eq!(alive[1 - winner], 0, "sizes {sizes:?}: {alive:?}");
    }
}

#[test]
fn accuracy_rolls_do_not_depend_on_spawn_order() {
    let rules = rules_with("UPDATE et_weapon SET accuracy_pct=50;");
    let mut a = lines(rules.clone(), 3, [40, 40], false);
    let mut b = lines(rules, 3, [40, 40], true);
    for tick in 0..2400 {
        a.step();
        b.step();
        assert_eq!(
            physical_state(&a, false),
            physical_state(&b, false),
            "tick {tick}"
        );
        assert_eq!(a.rng().state(), b.rng().state());
    }
    assert_eq!(a.stats().kills, b.stats().kills);
}

#[test]
fn swapped_scenario_only_exchanges_ownership() {
    let rules = rules();
    let a = World::battle_scenario(rules.clone(), 3, 200).unwrap();
    let b = World::battle_scenario_swapped(rules, 3, 200, true).unwrap();
    assert_eq!(a.map(), b.map());
    assert_eq!(a.rng(), b.rng());
    for (a, b) in a.entities().zip(b.entities()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.pos, b.pos);
        assert_eq!(a.type_id, b.type_id);
        assert_eq!(a.hp, b.hp);
        assert_eq!(a.owner.0, 3 - b.owner.0);
    }
}

#[test]
fn same_seed_same_hash() {
    let rules = rules();
    let mut a = World::battle_scenario(rules.clone(), 3, 200).unwrap();
    let mut b = World::battle_scenario(rules, 3, 200).unwrap();
    for tick in 0..2400 {
        a.step();
        b.step();
        if tick % 100 == 0 {
            assert_eq!(a.hash(), b.hash(), "tick {tick}");
        }
    }
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.stats(), b.stats());
    assert!(a.stats().kills.values().sum::<u64>() > 0);
}

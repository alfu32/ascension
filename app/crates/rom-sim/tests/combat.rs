use rom_rules::{Rules, build_rules_db};
use rom_sim::{PlayerId, World};
use std::sync::Arc;

#[test]
fn battle_is_deterministic() {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let run = || {
        let mut world = World::battle_scenario(rules.clone(), 3, 200).unwrap();
        for _ in 0..2400 {
            world.step();
        }
        let alive = |owner| {
            world
                .entities()
                .filter(|e| e.owner == PlayerId(owner))
                .count()
        };
        assert!(alive(1) == 0 || alive(2) == 0 || (alive(1) < 200 && alive(2) < 200));
        assert!(world.stats().kills.values().sum::<u64>() > 0);
        world
    };
    let a = run();
    let b = run();
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.stats(), b.stats());
}

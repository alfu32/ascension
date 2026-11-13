use rom_rules::{Rules, build_rules_db};
use rom_sim::{PlayerId, World};
use std::sync::Arc;
#[test]
fn wealth_scenario_repeats_on_flat_and_sphere() {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&conn).unwrap());
    for sphere in [false, true] {
        let mut a = World::wealth_scenario(rules.clone(), 1, sphere).unwrap();
        let mut b = World::wealth_scenario(rules.clone(), 1, sphere).unwrap();
        for tick in 0..6000 {
            a.step();
            b.step();
            if tick % 600 == 0 {
                assert_eq!(a.hash(), b.hash());
            }
        }
        assert_eq!(a.hash(), b.hash());
        let kept: usize = a
            .entities()
            .filter(|e| rules.entity_types[&e.type_id].keeper.is_some())
            .map(|e| a.kept_nodes(e.id).count())
            .sum();
        assert_eq!(kept, 2);
        assert_eq!(a.trade_routes(PlayerId(1)).count(), 2);
        assert!(a.stats().trade_trips > 10);
    }
}

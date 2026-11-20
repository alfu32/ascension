//! Computer players: `rom_match::computer`, where the match's leader runs them.

pub use rom_match::computer::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use rom_match::NEUTRAL;

    /// Prices and placement previews in a generated universe (every resource body's nodes
    /// are entities) cost microseconds, not milliseconds.
    #[test]
    fn price_and_placement_are_cheap_in_a_universe() {
        let (_, mut game) = crate::match_setup::universe_game_for_tests();
        // Fill the home planet with neutral units, one every two tiles, as a late
        // universe holds tens of thousands of entities.
        let start = game.starts[0].clone();
        let body = start.city_pos.body;
        let filler = game.builder_type;
        for face in 0..6_u8 {
            for y in (1..512).step_by(2) {
                if game.world.terrain_on_body(body, face, 0, y).is_none() {
                    break;
                }
                for x in (1..512).step_by(2) {
                    if game.world.terrain_on_body(body, face, x, y).is_none() {
                        break;
                    }
                    let at = rom_sim::Pos {
                        x: x * 1000 + 500,
                        y: y * 1000 + 500,
                        face,
                        body,
                    };
                    let _ = game.world.spawn(filler, NEUTRAL, at);
                }
            }
        }
        let world = &game.world;
        let rules = Arc::clone(world.rules());
        let buildings: Vec<_> = rules
            .entity_types
            .values()
            .filter(|t| t.class == rom_rules::EntityClass::Building)
            .map(|t| t.id)
            .take(40)
            .collect();
        let count = world.entities().count();
        let begin = Instant::now();
        let mut calls = 0_u32;
        for (i, def) in buildings.iter().enumerate() {
            for step in 0..5 {
                let at = world.offset(
                    start.city_pos,
                    (i as i64 % 7 - 3) * 3_000 + step * 1_000,
                    (i as i64 / 7 - 3) * 3_000,
                );
                let _ = world.price(start.player, *def);
                let _ = world.check_placement(start.player, *def, at);
                calls += 1;
            }
        }
        let each = begin.elapsed() / calls;
        eprintln!("{count} entities: price + check_placement {each:?} per pair");
        assert!(count > 5_000, "{count}");
        assert!(each < Duration::from_millis(1), "{each:?}");
    }
}

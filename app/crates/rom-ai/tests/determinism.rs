//! Scenario tests: the same match twice, and a brain restored in the middle of one.

mod common;

use rom_ai::Brain;
use rom_sim::{Command, World};
use std::sync::Arc;

const PLAYERS: [(&str, &str); 2] = [("balanced", "tough"), ("rusher", "moderate")];

/// Runs ticks `from..to`, returning every order issued, in order.
fn record(world: &mut World, brains: &mut [Brain], from: u64, to: u64) -> Vec<Command> {
    let mut issued = Vec::new();
    for tick in from..to {
        assert_eq!(world.tick(), tick);
        for brain in brains.iter_mut() {
            for command in brain.think(world, tick) {
                match world.submit(command.clone()) {
                    Ok(_) => brain.accepted(),
                    Err(error) => brain.rejected(&command, &error, tick),
                }
                issued.push(command);
            }
        }
        world.step();
    }
    issued
}

#[test]
fn the_same_match_twice_gives_equal_hashes() {
    let rules = common::rules();
    let run = || {
        let (mut world, mut brains) = common::start(&rules, 11, &PLAYERS);
        let orders = record(&mut world, &mut brains, 0, 2400);
        let saved: Vec<_> = brains.iter().map(Brain::save).collect();
        (world.hash(), orders, saved)
    };
    let (first, second) = (run(), run());
    assert!(!first.1.is_empty(), "the brains issued no orders");
    assert_eq!(first.0, second.0, "state hashes differ");
    assert_eq!(first.1, second.1, "orders differ");
    assert_eq!(first.2, second.2, "saved brains differ");
}

#[test]
fn a_brain_restored_mid_match_issues_the_same_orders() {
    let rules = common::rules();
    let (mut world, mut brains) = common::plain(&rules, 5, &PLAYERS);
    record(&mut world, &mut brains, 0, 1500);
    let (mut twin_world, mut twin_brains) = common::plain(&rules, 5, &PLAYERS);
    record(&mut twin_world, &mut twin_brains, 0, 1500);
    assert_eq!(world.hash(), twin_world.hash());

    // One pair of brains goes through its saved form, the other carries on in memory.
    let mut restored: Vec<Brain> = twin_brains
        .iter()
        .map(|brain| {
            let saved = brain.save();
            assert_eq!(saved.profile_key, brain.profile_key());
            assert_eq!(saved.difficulty_key, brain.difficulty_key());
            Brain::restore(Arc::clone(&rules), brain.player(), &saved).unwrap()
        })
        .collect();
    let kept = record(&mut world, &mut brains, 1500, 3200);
    let again = record(&mut twin_world, &mut restored, 1500, 3200);
    assert!(kept.len() > 10, "too few orders to compare: {}", kept.len());
    assert_eq!(kept, again, "orders differ after a restore");
    assert_eq!(world.hash(), twin_world.hash());
    for (a, b) in brains.iter().zip(&restored) {
        assert_eq!(a.save(), b.save());
    }
}

//! Every profile, alone on a generated world, runs a civilisation: it reaches the third
//! age within one game hour and the simulation hardly ever refuses an order.

mod common;

use rom_sim::PlayerId;

/// One game hour at 20 ticks per second.
const HOUR: u64 = 72_000;
/// Ordinal of the third age.
const THIRD_AGE: i64 = 2;

/// Plays `profile` on `seed` until the age with ordinal `age` or the end of the hour;
/// returns the tick.
fn reach(profile: &str, seed: u64, age: i64) -> u64 {
    let rules = common::rules();
    let (mut world, mut brains) = common::start(&rules, seed, &[(profile, "moderate")]);
    while world.tick() < HOUR && common::age_ordinal(&world, PlayerId(1)) < age {
        common::run(&mut world, &mut brains, 200);
    }
    let stats = brains[0].stats();
    assert!(
        common::age_ordinal(&world, PlayerId(1)) >= age,
        "{profile} on seed {seed} is in age {} after an hour",
        common::age_ordinal(&world, PlayerId(1))
    );
    assert!(
        stats.max_reject_streak <= 3,
        "{profile} on seed {seed}: {} orders rejected in a row",
        stats.max_reject_streak
    );
    assert!(
        stats.rejected * 20 <= stats.orders,
        "{profile} on seed {seed}: {} of {} orders rejected",
        stats.rejected,
        stats.orders
    );
    world.tick()
}

/// The quick form for the ordinary test run: one profile, one seed, the second age, which
/// already takes a library, all four technology tracks and a working economy.
#[test]
fn the_balanced_profile_reaches_the_second_age() {
    let tick = reach("balanced", 1, THIRD_AGE - 1);
    assert!(tick < HOUR / 2, "the second age came only at tick {tick}");
}

/// Every profile of the seed on seeds 1 to 5. Slow without optimisation; run it with
/// `cargo test --release -p rom-ai --test ages -- --ignored`.
#[test]
#[ignore = "twenty game hours: run with --release"]
fn every_profile_reaches_the_third_age_on_five_seeds() {
    let rules = common::rules();
    for profile in rules.ai_profiles.keys() {
        for seed in 1..=5 {
            let tick = reach(profile, seed, THIRD_AGE);
            println!("{profile} seed {seed}: third age at tick {tick}");
        }
    }
}

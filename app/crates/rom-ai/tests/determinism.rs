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

/// Both sides see everything and start with an army, so they are at war within a minute.
fn at_war(seed: u64) -> (Arc<rom_rules::Rules>, World, Vec<Brain>) {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    let rules = Arc::new(rules);
    let (mut world, brains) = common::plain(&rules, seed, &PLAYERS);
    let soldier = rules
        .entity_types
        .values()
        .find(|t| {
            t.class == rom_rules::EntityClass::Unit
                && !t.weapons.is_empty()
                && t.harvester.is_empty()
                && t.builder.is_none()
                && t.container.is_none()
                && t.movement.is_some()
                && t.upgrades_from_id.is_none()
        })
        .unwrap()
        .id;
    // Units that break buildings, so each side can take the other's city.
    let city = world
        .entity(world.cities(rom_sim::PlayerId(1))[0])
        .unwrap()
        .type_id;
    let armour = &rules.entity_types[&city];
    let breaker = rules
        .entity_types
        .values()
        .filter(|t| {
            t.class == rom_rules::EntityClass::Unit
                && !t.weapons.is_empty()
                && t.harvester.is_empty()
                && t.container.is_none()
                && t.movement.is_some()
        })
        .max_by_key(|t| {
            let damage: i64 = t
                .weapons
                .iter()
                .map(|w| {
                    let pct = armour
                        .armor_class_id
                        .map_or(100, |a| rules.damage_pct(w.damage_class_id, a));
                    (w.damage * pct / 100 - armour.armor).max(1) * 100 / w.cooldown_ticks.max(1)
                })
                .sum();
            (damage, std::cmp::Reverse(t.id))
        })
        .unwrap()
        .id;
    // Player one is the stronger: it attacks, the other answers.
    for (player, x, y, count) in [(1, 56_500, 54_500, 28), (2, 72_500, 74_500, 10)] {
        for n in 0..count {
            let at = rom_sim::Pos::flat(x + (n % 5) * 1200, y + (n / 5) * 1200);
            let unit = if n % 2 == 0 { soldier } else { breaker };
            world.spawn(unit, rom_sim::PlayerId(player), at).unwrap();
        }
    }
    (rules, world, brains)
}

#[test]
fn a_brain_restored_mid_war_issues_the_same_orders() {
    let (rules, mut world, mut brains) = at_war(7);
    let (_, mut twin_world, mut twin_brains) = at_war(7);
    // Run until an army is fighting and units have fallen; restore there.
    let fighting = |brains: &[Brain]| {
        brains.iter().any(|b| {
            b.armies().any(|(_, a)| {
                matches!(
                    a.phase,
                    rom_ai::Phase::Advance | rom_ai::Phase::Respond | rom_ai::Phase::Retreat
                ) && !a.members.is_empty()
            })
        })
    };
    let mut at = 0;
    while at < 6000 && !(fighting(&brains) && world.stats().kills.values().sum::<u64>() > 0) {
        record(&mut world, &mut brains, at, at + 200);
        record(&mut twin_world, &mut twin_brains, at, at + 200);
        at += 200;
    }
    assert_eq!(world.hash(), twin_world.hash());
    assert!(
        fighting(&brains),
        "no army is fighting at the restore point"
    );
    assert!(
        world.stats().kills.values().sum::<u64>() > 0,
        "nobody has fallen yet"
    );

    let mut restored: Vec<Brain> = twin_brains
        .iter()
        .map(|brain| Brain::restore(Arc::clone(&rules), brain.player(), &brain.save()).unwrap())
        .collect();
    let kept = record(&mut world, &mut brains, at, at + 2000);
    let again = record(&mut twin_world, &mut restored, at, at + 2000);
    assert!(
        kept.iter().any(|c| c.kind == "attack_move"),
        "no attack orders after the restore point"
    );
    assert_eq!(kept, again, "orders differ after a restore in mid-war");
    assert_eq!(world.hash(), twin_world.hash());
    for (a, b) in brains.iter().zip(&restored) {
        assert_eq!(a.save(), b.save());
    }
}

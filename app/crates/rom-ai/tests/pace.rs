//! Task 166: computer players at a slow economy (the setup's gathering rate, which every
//! player sees) and in the automated ages. A lost scout is not replaced at once when a scout
//! costs many minutes of income; workers no site can employ leave the match beyond a few
//! builders. Each scenario runs twice to the same state.

mod common;

use rom_ai::{Brain, Role};
use rom_rules::{DefId, EntityClass, Rules};
use rom_sim::{EconomyPace, EntityId, PlayerId, World};
use std::collections::BTreeSet;

const ME: PlayerId = PlayerId(1);

fn pace(gather_rate: u32) -> EconomyPace {
    EconomyPace {
        gather_rate,
        research_time: 100,
    }
}

/// An armed unit type of the first age that fights for a living (as the military tests pick
/// their soldier).
fn soldier(rules: &Rules) -> DefId {
    rules
        .entity_types
        .values()
        .find(|t| {
            t.class == EntityClass::Unit
                && !t.weapons.is_empty()
                && t.harvester.is_empty()
                && t.builder.is_none()
                && t.container.is_none()
                && t.movement.is_some()
                && t.upgrades_from_id.is_none()
        })
        .unwrap()
        .id
}

fn scouts(brain: &Brain) -> BTreeSet<u32> {
    brain
        .armies()
        .find(|(role, _)| *role == Role::Scout)
        .map(|(_, army)| army.members.clone())
        .unwrap_or_default()
}

/// One computer player alone at `gather_rate`: a scout is enlisted, lost, and another fresh
/// unit stands ready. Returns the ticks from the loss to the next scout, and the state.
fn scout_again(gather_rate: u32) -> (u64, u64, Vec<u8>) {
    let rules = common::rules();
    let (mut world, mut brains) = common::plain(&rules, 5, &[("balanced", "moderate")]);
    world.set_pace(pace(gather_rate)).unwrap();
    let unit = soldier(&rules);
    let city = world
        .entities()
        .find(|e| e.owner == ME && world.rules().entity_types[&e.type_id].territory.is_some())
        .unwrap()
        .pos;
    let near = |w: &World, dx: i64| w.offset(city, dx, 6000);
    common::run(&mut world, &mut brains, 200);
    let first = world.spawn(unit, ME, near(&world, 0)).unwrap();
    common::run(&mut world, &mut brains, 200);
    assert!(scouts(&brains[0]).contains(&first.0), "the soldier scouts");
    world.despawn(first).unwrap();
    let lost = world.tick();
    // A fresh unit for the next scout, beside the city.
    let ready: EntityId = world.spawn(unit, ME, near(&world, 2000)).unwrap();
    let mut again = None;
    while world.tick() < lost + 30_000 {
        common::run(&mut world, &mut brains, 40);
        if !scouts(&brains[0]).is_empty() {
            again = Some(world.tick() - lost);
            break;
        }
    }
    let again = again.expect("a scout again");
    assert!(world.entity(ready).is_some());
    (again, world.hash(), brains[0].save().data)
}

/// At the default pace a lost scout is replaced by the next fresh unit at once; at a 25 %
/// gathering rate, where it costs four times the minutes of income, it waits three times
/// `SCOUT_WAIT_TICKS` (3600) first.
#[test]
fn a_lost_scout_waits_at_a_slow_economy() {
    let (at_default, _, _) = scout_again(100);
    assert!(at_default <= 200, "replaced at once: {at_default}");
    let slow = scout_again(25);
    assert!(
        (3 * 3600..3 * 3600 + 400).contains(&slow.0),
        "waits for its income: {}",
        slow.0
    );
    assert_eq!(scout_again(25), slow, "run twice, the same state");
}

/// In the last age every gather site runs without workers: a player with twenty idle workers
/// beyond its builders lets them go (a few a thought), keeping one per city and two more for
/// building, so they do not fill the population for good.
#[test]
fn workers_with_no_work_left_leave_in_the_automated_ages() {
    let rules = common::rules();
    let last = rules
        .ages
        .values()
        .max_by_key(|a| (a.ordinal, a.id))
        .unwrap()
        .id;
    let techs: Vec<DefId> = rules.techs.keys().copied().collect();
    let workers = |w: &World| {
        w.entities()
            .filter(|e| e.owner == ME && !w.rules().entity_types[&e.type_id].harvester.is_empty())
            .count()
    };
    let play = || {
        let (mut world, mut brains) =
            common::plain_in(&rules, 3, &[("balanced", "moderate")], last, &techs);
        let builder = world
            .entities()
            .find(|e| e.owner == ME && !world.rules().entity_types[&e.type_id].harvester.is_empty())
            .unwrap()
            .type_id;
        let city = world
            .entities()
            .find(|e| e.owner == ME && world.rules().entity_types[&e.type_id].territory.is_some())
            .unwrap()
            .pos;
        for i in 0..20 {
            let at = world.offset(city, 3000 + 600 * (i % 5), 7000 + 600 * (i / 5));
            world.spawn(builder, ME, at).unwrap();
        }
        assert_eq!(workers(&world), 25);
        common::run(&mut world, &mut brains, 1200);
        let cities = world
            .entities()
            .filter(|e| {
                e.owner == ME
                    && world.rules().entity_types[&e.type_id]
                        .territory
                        .as_ref()
                        .is_some_and(|t| t.is_city)
            })
            .count();
        let left = workers(&world);
        assert!(left <= cities + 2, "{left} workers for {cities} cities");
        assert!(left >= 2, "builders are kept: {left}");
        (left, world.hash(), brains[0].save().data)
    };
    assert_eq!(play(), play());
}

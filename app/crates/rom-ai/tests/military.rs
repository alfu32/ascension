//! The military manager on the small hand-built world: taking a city it knows, waiting
//! for the attack ratio, retreating when beaten (only where the difficulty allows it),
//! answering intruders, scouting and formations.

mod common;

use rom_ai::{Brain, Phase, Role};
use rom_rules::{DefId, EntityClass, Rules};
use rom_sim::{Command, EntityId, PlayerId, Pos, World};
use std::sync::Arc;

const ME: PlayerId = PlayerId(1);
const THEM: PlayerId = PlayerId(2);

/// The rules with `difficulty` seeing through the fog, so a test does not wait for scouts.
fn seeing(difficulty: &str, change: impl Fn(&mut rom_rules::AiDifficulty)) -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    let d = rules.ai_difficulties.get_mut(difficulty).unwrap();
    d.sees_all = true;
    change(d);
    Arc::new(rules)
}

/// An armed unit type of the first age that fights for a living.
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

/// The walking unit type that does most damage to a building of `target`.
fn breaker(rules: &Rules, target: DefId) -> DefId {
    let armour = &rules.entity_types[&target];
    rules
        .entity_types
        .values()
        .filter(|t| {
            t.class == EntityClass::Unit
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
        .id
}

/// Spawns `count` units of a type for a player in a block around `centre`.
fn spawn(world: &mut World, player: PlayerId, type_id: DefId, count: i32, centre: Pos) {
    for n in 0..count {
        let at = world.offset(centre, i64::from(n % 5) * 1200, i64::from(n / 5) * 1200);
        world.spawn(type_id, player, at).unwrap();
    }
}

fn city_of(world: &World, player: PlayerId) -> EntityId {
    world.cities(player)[0]
}

fn army(brain: &Brain, role: Role) -> Option<&rom_ai::Army> {
    brain.armies().find(|(r, _)| *r == role).map(|(_, a)| a)
}

/// Thinks and steps only the first brain, the other player standing still.
fn run_first(world: &mut World, brains: &mut [Brain], ticks: u64) {
    common::run(world, &mut brains[..1], ticks);
}

#[test]
fn the_main_army_takes_a_city_it_knows() {
    let rules = seeing("tougher", |_| {});
    let players = [("balanced", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    let target = city_of(&world, THEM);
    let city_type = world.entity(target).unwrap().type_id;
    spawn(
        &mut world,
        ME,
        breaker(&rules, city_type),
        10,
        Pos::flat(55_500, 52_500),
    );
    spawn(
        &mut world,
        ME,
        soldier(&rules),
        10,
        Pos::flat(55_500, 57_500),
    );
    let mut advanced = false;
    for _ in 0..60 {
        run_first(&mut world, &mut brains, 200);
        advanced |= army(&brains[0], Role::Attack)
            .is_some_and(|a| a.phase == Phase::Advance && a.target == Some(target.0));
        if world.entity(target).is_some_and(|c| c.owner == ME) {
            break;
        }
    }
    assert!(advanced, "the main army never set off for the city");
    assert_eq!(
        world.entity(target).map(|c| c.owner),
        Some(ME),
        "the city was not taken by tick {}",
        world.tick()
    );
}

#[test]
fn the_army_waits_for_the_attack_ratio() {
    let rules = seeing("tougher", |_| {});
    let players = [("balanced", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    let unit = soldier(&rules);
    spawn(&mut world, ME, unit, 10, Pos::flat(55_500, 52_500));
    // The other side stands in strength around its city.
    spawn(&mut world, THEM, unit, 40, Pos::flat(72_500, 70_500));
    run_first(&mut world, &mut brains, 3000);
    let main = army(&brains[0], Role::Attack).expect("a main army");
    assert!(!main.members.is_empty());
    assert_ne!(main.phase, Phase::Advance, "attacked against the odds");
    let near = world
        .entities()
        .filter(|e| e.owner == ME && e.type_id == unit)
        .filter(|e| world.distance(e.pos, Pos::flat(80_500, 80_500)).unwrap() < 20_000)
        .count();
    assert_eq!(near, 0, "soldiers went to the enemy city");
}

/// Runs an advance into a sudden enemy army; returns whether the main army retreated.
fn retreats(retreat: bool) -> bool {
    let rules = seeing("tougher", |d| d.retreats = retreat);
    let players = [("balanced", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    let target = city_of(&world, THEM);
    let city_type = world.entity(target).unwrap().type_id;
    spawn(
        &mut world,
        ME,
        breaker(&rules, city_type),
        8,
        Pos::flat(55_500, 52_500),
    );
    spawn(
        &mut world,
        ME,
        soldier(&rules),
        8,
        Pos::flat(55_500, 57_500),
    );
    for _ in 0..40 {
        run_first(&mut world, &mut brains, 100);
        if army(&brains[0], Role::Attack).is_some_and(|a| a.phase == Phase::Advance) {
            break;
        }
    }
    assert_eq!(
        army(&brains[0], Role::Attack).map(|a| a.phase),
        Some(Phase::Advance),
        "the army never advanced"
    );
    // A large enemy force appears between the army and its target.
    spawn(
        &mut world,
        THEM,
        breaker(&rules, city_type),
        30,
        Pos::flat(64_500, 64_500),
    );
    spawn(
        &mut world,
        THEM,
        soldier(&rules),
        30,
        Pos::flat(64_500, 69_500),
    );
    for _ in 0..150 {
        run_first(&mut world, &mut brains, 20);
        if army(&brains[0], Role::Attack).is_some_and(|a| a.phase == Phase::Retreat) {
            return true;
        }
    }
    false
}

#[test]
fn a_beaten_army_retreats_where_the_difficulty_allows_it() {
    assert!(retreats(true), "the army did not retreat");
    assert!(!retreats(false), "the army retreated although it may not");
}

#[test]
fn intruders_are_met_by_the_home_guard() {
    let rules = common::rules();
    let players = [("turtle", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    let unit = soldier(&rules);
    spawn(&mut world, ME, unit, 8, Pos::flat(55_500, 52_500));
    run_first(&mut world, &mut brains, 200);
    let guard = army(&brains[0], Role::Defend).expect("a home guard");
    assert!(!guard.members.is_empty());
    assert_eq!(guard.phase, Phase::Gather);
    // Enemy soldiers walk into the city's borders.
    spawn(&mut world, THEM, unit, 4, Pos::flat(44_500, 40_500));
    let mut answered = false;
    for _ in 0..20 {
        run_first(&mut world, &mut brains, 20);
        answered |= army(&brains[0], Role::Defend).is_some_and(|a| a.phase == Phase::Respond);
    }
    assert!(answered, "the home guard did not answer");
    let intruders_left = world
        .entities()
        .filter(|e| e.owner == THEM && e.type_id == unit)
        .count();
    run_first(&mut world, &mut brains, 1600);
    let after = world
        .entities()
        .filter(|e| e.owner == THEM && e.type_id == unit)
        .count();
    assert!(
        after < intruders_left || intruders_left == 0,
        "no intruder fell: {intruders_left} → {after}"
    );
}

#[test]
fn scouts_look_at_new_ground() {
    let rules = common::rules();
    let (mut world, mut brains) = common::plain(&rules, 3, &[("rusher", "tougher")]);
    spawn(
        &mut world,
        ME,
        soldier(&rules),
        2,
        Pos::flat(55_500, 52_500),
    );
    common::run(&mut world, &mut brains, 400);
    let early = brains[0].cells_looked_at();
    let scouts = army(&brains[0], Role::Scout).expect("scouts");
    assert!(!scouts.members.is_empty(), "nobody scouts");
    common::run(&mut world, &mut brains, 3000);
    let later = brains[0].cells_looked_at();
    assert!(
        later > early + 10,
        "the scouts looked at {early} cells, then {later}"
    );
}

/// The attack-move orders of the first advance on the enemy city.
fn advance_orders(formations: bool) -> Vec<Command> {
    let rules = seeing("tougher", |d| d.uses_formations = formations);
    let players = [("balanced", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    let city_type = world.entity(city_of(&world, THEM)).unwrap().type_id;
    spawn(
        &mut world,
        ME,
        breaker(&rules, city_type),
        8,
        Pos::flat(55_500, 52_500),
    );
    spawn(
        &mut world,
        ME,
        soldier(&rules),
        8,
        Pos::flat(55_500, 57_500),
    );
    let mut out = Vec::new();
    for tick in 0..4000 {
        for command in brains[0].think(&world, tick) {
            if command.kind == "attack_move" {
                out.push(command.clone());
            }
            let _ = world.submit(command);
        }
        world.step();
        if out.len() >= 12 {
            break;
        }
    }
    out
}

#[test]
fn formations_spread_the_advance() {
    let spread = advance_orders(true);
    let massed = advance_orders(false);
    assert!(spread.len() >= 8 && massed.len() >= 8);
    let goals = |orders: &[Command]| {
        orders
            .iter()
            .filter_map(|c| c.target_pos.map(|p| (p.x, p.y)))
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    };
    assert_eq!(
        goals(&massed),
        1,
        "without formations everyone goes to one point"
    );
    assert!(goals(&spread) >= 4, "the formation did not spread the army");
}

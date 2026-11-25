//! The military manager on the small hand-built world: taking a city it knows, waiting
//! for the attack ratio, retreating when beaten (only where the difficulty allows it),
//! answering intruders, scouting and formations (task 153: chosen by their rows and laid out
//! by the simulation).

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
            // Weapons that strike a building where it stands (not from orbit, task 75).
            let damage: i64 = t
                .weapons
                .iter()
                .filter(|w| {
                    !w.orbit_only
                        && w.targets
                            .iter()
                            .any(|l| l.layer == rom_rules::Layer::Ground)
                })
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

/// A plain building of four tiles a side that does not fight: what a town is ringed with.
fn wall_piece(rules: &Rules) -> DefId {
    rules
        .entity_types
        .values()
        .filter(|t| {
            t.class == EntityClass::Building
                && t.weapons.is_empty()
                && t.territory.is_none()
                && t.footprint_w == 4
                && t.footprint_h == 4
        })
        .min_by_key(|t| t.id)
        .unwrap()
        .id
}

/// A town closed by a ring of its own buildings is broken into: the army strikes the ring
/// (the march itself strikes what stands in its way; a wave left standing with no path
/// strikes the building nearest it on the way, `army::breach`).
#[test]
fn an_army_with_no_way_in_breaks_through_the_ring_of_buildings() {
    let rules = seeing("tougher", |_| {});
    let players = [("balanced", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    // The enemy town at tile 80,80 is closed by a square of buildings 20 tiles a side.
    let piece = wall_piece(&rules);
    let mut ring = Vec::new();
    for i in 0..5 {
        let along = 72_000 + i * 4000;
        for (x, y) in [
            (along, 70_000),
            (along, 90_000),
            (70_000, along),
            (90_000, along),
        ] {
            if let Ok(id) = world.spawn(piece, THEM, Pos::flat(x, y)) {
                ring.push(id);
            }
        }
    }
    for corner in [
        (70_000, 70_000),
        (90_000, 70_000),
        (70_000, 90_000),
        (90_000, 90_000),
    ] {
        if let Ok(id) = world.spawn(piece, THEM, Pos::flat(corner.0, corner.1)) {
            ring.push(id);
        }
    }
    assert!(ring.len() >= 20, "the ring was not built: {}", ring.len());
    // Close fighters only: what reaches over the ring from outside would open it anyway.
    spawn(
        &mut world,
        ME,
        soldier(&rules),
        30,
        Pos::flat(55_500, 52_500),
    );
    let mut breached = false;
    for _ in 0..60 {
        run_first(&mut world, &mut brains, 200);
        breached = ring
            .iter()
            .any(|id| world.entity(*id).is_none_or(|e| e.hp < e.max_hp));
        if breached {
            break;
        }
    }
    assert!(
        breached,
        "no building of the ring was struck by tick {}",
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

#[test]
fn the_easiest_level_keeps_its_grace_and_sends_small_waves() {
    let rules = seeing("easiest", |d| {
        d.grace_ticks = 4000;
        d.first_wave_max = 5;
        d.wave_growth_per_hour = 0;
        d.mistake_pct = 0;
    });
    let players = [("balanced", "easiest"), ("balanced", "easiest")];
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
    // Strong enough to take the city, but the grace period holds it back.
    while world.tick() < 3800 {
        run_first(&mut world, &mut brains, 200);
        assert!(
            brains[0].armies().all(|(_, a)| a.phase != Phase::Advance),
            "set off during the grace period at tick {}",
            world.tick()
        );
    }
    let mut wave = None;
    for _ in 0..40 {
        run_first(&mut world, &mut brains, 200);
        if let Some(main) = army(&brains[0], Role::Attack).filter(|a| a.phase == Phase::Advance) {
            wave = Some(main.members.len());
            break;
        }
    }
    let wave = wave.expect("the main army never set off after the grace period");
    assert!(wave <= 5, "a first wave of {wave} units");
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
    // Tick by tick: a retreat that ends near the rally point lasts a thought or two.
    for _ in 0..3000 {
        run_first(&mut world, &mut brains, 1);
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

/// Task 153: a home guard that finds no way to its capital (walled in by the player's own
/// buildings) is not sent there again thought after thought: each failed order cost the order
/// budget, which starved the armies and the navy in duel seed 14.
#[test]
fn a_walled_in_capital_does_not_eat_the_order_budget() {
    let rules = common::rules();
    let players = [("turtle", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    let piece = wall_piece(&rules);
    let mut ring = 0;
    for i in 0..5 {
        let along = 40_000 + i * 4000;
        for (x, y) in [
            (along, 38_000),
            (along, 58_000),
            (38_000, along),
            (58_000, along),
        ] {
            ring += usize::from(world.spawn(piece, ME, Pos::flat(x, y)).is_ok());
        }
    }
    for (x, y) in [
        (38_000, 38_000),
        (58_000, 38_000),
        (38_000, 58_000),
        (58_000, 58_000),
    ] {
        ring += usize::from(world.spawn(piece, ME, Pos::flat(x, y)).is_ok());
    }
    assert!(ring >= 20, "the ring was not built: {ring}");
    spawn(
        &mut world,
        ME,
        soldier(&rules),
        12,
        Pos::flat(20_500, 70_500),
    );
    let mut guards = 0;
    for tick in 0..3000 {
        for command in brains[0].think(&world, tick) {
            guards += usize::from(command.kind == "guard");
            let _ = world.submit(command);
        }
        world.step();
    }
    let failed = world
        .order_failures(ME)
        .filter(|f| f.kind == "guard" && f.reason == "no_path")
        .count();
    assert!(failed > 0, "the guard found its way after all");
    assert!(guards <= 24, "{guards} guard orders in 3000 ticks");
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

/// A walking soldier that fights hand to hand (`close`) or from a distance (no dead zone), of
/// the movement class of `soldier`.
fn fighter(rules: &Rules, close: bool) -> DefId {
    let class = rules.entity_types[&soldier(rules)]
        .movement
        .as_ref()
        .map(|m| m.movement_class_id);
    rules
        .entity_types
        .values()
        .filter(|t| {
            t.class == EntityClass::Unit
                && !t.weapons.is_empty()
                && t.weapons.iter().all(|w| w.range_min == 0)
                && t.harvester.is_empty()
                && t.builder.is_none()
                && t.container.is_none()
                && t.supply.is_none()
                && t.movement.as_ref().map(|m| m.movement_class_id) == class
                && (t.weapons.iter().map(|w| w.range_max).max() == Some(0)) == close
        })
        .min_by_key(|t| t.id)
        .unwrap()
        .id
}

/// The first advance on the enemy city of eight close fighters and eight ranged ones: the
/// attack-move orders of the first thought that sends them, each member's place once the
/// simulation applied them (type, place, where it stood), and the state some ticks later.
fn first_advance(formations: bool) -> (Vec<Command>, Vec<(DefId, Pos, Pos)>, u64) {
    let rules = seeing("tougher", |d| d.uses_formations = formations);
    let players = [("balanced", "tougher"), ("balanced", "tougher")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    spawn(
        &mut world,
        ME,
        fighter(&rules, true),
        8,
        Pos::flat(55_500, 52_500),
    );
    spawn(
        &mut world,
        ME,
        fighter(&rules, false),
        8,
        Pos::flat(55_500, 57_500),
    );
    let mut sent = Vec::new();
    for tick in 0..4000 {
        let commands = brains[0].think(&world, tick);
        let batch: Vec<Command> = commands
            .iter()
            .filter(|c| c.kind == "attack_move")
            .cloned()
            .collect();
        for command in commands {
            let _ = world.submit(command);
        }
        world.step();
        if batch.len() >= 8 {
            sent = batch;
            break;
        }
    }
    let places = sent
        .iter()
        .filter_map(|c| {
            let e = world.entity(c.executor)?;
            let order = e.orders().iter().find(|o| o.kind == "attack_move")?;
            Some((e.type_id, order.target_pos?, e.pos))
        })
        .collect();
    common::run(&mut world, &mut brains[..1], 400);
    (sent, places, world.hash())
}

/// Task 153: the main army advances in a formation chosen by its row: every member is sent to
/// one point naming a formation that puts ranged members behind, and the simulation lays the
/// group out as a front facing the city with the ranged members behind the close fighters.
/// Without formations everyone goes to the one point. The same advance twice is the same.
#[test]
fn an_advance_marches_in_formation_with_the_ranged_behind() {
    let rules = common::rules();
    let (sent, places, hash) = first_advance(true);
    assert!(sent.len() >= 8, "the first thought sent {}", sent.len());
    let subject = sent[0].subject.expect("a formation");
    let row = &rules.formations[&subject];
    assert!(row.ranged_behind, "{row:?}");
    assert!(sent.iter().all(|c| c.subject == Some(subject)));
    let goal = sent[0].target_pos.expect("a point");
    assert!(
        sent.iter().all(|c| c.target_pos == Some(goal)),
        "the group is sent to one point"
    );
    let spread: std::collections::BTreeSet<(i32, i32)> =
        places.iter().map(|(_, p, _)| (p.x, p.y)).collect();
    assert!(
        spread.len() >= 8,
        "the formation did not spread: {spread:?}"
    );
    // Ahead is from where the members stood toward the goal.
    let n = places.len() as i64;
    let (cx, cy) = places.iter().fold((0, 0), |(x, y), (_, _, at)| {
        (x + i64::from(at.x), y + i64::from(at.y))
    });
    let (cx, cy) = (cx / n, cy / n);
    let (dx, dy) = (i64::from(goal.x) - cx, i64::from(goal.y) - cy);
    let ahead = |close: bool| {
        let chosen: Vec<i64> = places
            .iter()
            .filter(|(t, ..)| (*t == fighter(&rules, true)) == close)
            .map(|(_, p, _)| ((i64::from(p.x) - cx) * dx + (i64::from(p.y) - cy) * dy) / 1000)
            .collect();
        assert!(!chosen.is_empty());
        chosen.iter().sum::<i64>() / chosen.len() as i64
    };
    assert!(
        ahead(true) > ahead(false),
        "the ranged members are not behind in {:?} of {}: close {} ranged {}",
        row.shape,
        sent.len(),
        ahead(true),
        ahead(false)
    );
    assert_eq!(first_advance(true), (sent, places, hash));
    // Without formations: one point, no formation named.
    let (massed, places, _) = first_advance(false);
    assert!(massed.len() >= 8 && massed.iter().all(|c| c.subject.is_none()));
    let spread: std::collections::BTreeSet<(i32, i32)> =
        places.iter().map(|(_, p, _)| (p.x, p.y)).collect();
    assert_eq!(
        spread.len(),
        1,
        "without formations everyone goes to one point"
    );
}

/// Task 152: enemy soldiers inside an undefended city's borders make the player call up some
/// of its workers there as militia (never all of them), who stay out of the armies and stand
/// down again once the borders are quiet; the same raid twice gives the same state.
#[test]
fn a_raided_city_calls_up_militia_and_stands_them_down() {
    let rules = common::rules();
    let unit = soldier(&rules);
    let militia = |w: &World| -> Vec<EntityId> {
        w.entities()
            .filter(|e| e.owner == ME && rules.militia_worker(e.type_id).is_some())
            .map(|e| e.id)
            .collect()
    };
    let workers = |w: &World| {
        w.entities()
            .filter(|e| e.owner == ME && w.calls_up(e.type_id))
            .count()
    };
    let raid = || {
        let players = [("balanced", "tougher"), ("balanced", "tougher")];
        let (mut world, mut brains) = common::plain(&rules, 3, &players);
        run_first(&mut world, &mut brains, 600);
        assert!(militia(&world).is_empty(), "militia before any raid");
        // Enemy soldiers walk into the town, beyond the reach of the city's own weapons.
        spawn(&mut world, THEM, unit, 3, Pos::flat(46_500, 62_500));
        let raiders: Vec<EntityId> = world
            .entities()
            .filter(|e| e.owner == THEM && e.type_id == unit)
            .map(|e| e.id)
            .collect();
        let mut most = 0;
        for _ in 0..30 {
            run_first(&mut world, &mut brains, 20);
            let called = militia(&world);
            most = most.max(called.len());
            // Held at the city: never in an army.
            for (_, army) in brains[0].armies() {
                assert!(called.iter().all(|m| !army.members.contains(&m.0)));
            }
            assert!(
                called.len() <= (called.len() + workers(&world)) / 2,
                "more than half the workers called up"
            );
        }
        assert!(most > 0, "no militia called up");
        // The raiders leave (or fall); the borders stay quiet and the militia stand down.
        for raider in raiders {
            if world.entity(raider).is_some() {
                world.despawn(raider).unwrap();
            }
        }
        run_first(&mut world, &mut brains, 1200);
        assert!(
            militia(&world).is_empty(),
            "militia still under arms: {:?}",
            militia(&world)
        );
        (most, world.hash(), brains[0].save())
    };
    assert_eq!(raid(), raid());
}

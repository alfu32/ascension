//! The managers on a small hand-built world: economy, opening book, construction and
//! research, production, the difficulty limits and what the brain is allowed to see.

mod common;

use rom_ai::Brain;
use rom_rules::{DefId, EntityClass, OpeningAction, Rules};
use rom_sim::{PlayerId, Pos, World};
use std::sync::Arc;

const ME: PlayerId = PlayerId(1);

fn own_count(world: &World, test: impl Fn(&rom_rules::EntityType) -> bool) -> usize {
    world
        .entities()
        .filter(|e| e.owner == ME && test(&world.rules().entity_types[&e.type_id]))
        .count()
}

fn income(world: &World, test: impl Fn(&rom_rules::EntityType) -> bool) -> i64 {
    // The resources of the gather sites that pass the test.
    let rules = world.rules();
    let resources: std::collections::BTreeSet<DefId> = rules
        .entity_types
        .values()
        .filter(|t| test(t))
        .filter_map(|t| t.gather_site.as_ref().map(|s| s.resource_id))
        .collect();
    resources
        .iter()
        .map(|r| world.income(ME, *r).effective)
        .sum()
}

#[test]
fn economy_puts_workers_on_food_and_timber() {
    let rules = common::rules();
    let (mut world, mut brains) = common::plain(&rules, 3, &[("balanced", "tougher")]);
    common::run(&mut world, &mut brains, 2600);
    let summary = brains[0].summary(&world).unwrap();
    assert!(summary.workers >= 10, "{summary:?}");
    assert!(summary.idle_workers <= 1, "{summary:?}");
    // Sites without a node terrain (farms) and sites that need one (woodcutter camps).
    let plain_sites = |t: &rom_rules::EntityType| {
        t.gather_site
            .as_ref()
            .is_some_and(|s| s.needs_node_terrain_id.is_none() && s.slots_max > 0)
    };
    let node_sites = |t: &rom_rules::EntityType| {
        t.gather_site
            .as_ref()
            .is_some_and(|s| s.needs_node_terrain_id.is_some() && s.slots_max > 0)
    };
    assert!(own_count(&world, plain_sites) >= 3);
    assert!(own_count(&world, node_sites) >= 1);
    assert!(income(&world, plain_sites) > 0);
    assert!(income(&world, node_sites) > 0);
    assert_eq!(brains[0].stats().rejected, 0, "orders were rejected");
}

/// Rules whose `profile` opening is replaced by the given steps.
fn with_opening(profile: &str, steps: &[(OpeningAction, DefId, i64)]) -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    let template = rules.ai_opening_steps[profile][0].clone();
    rules.ai_opening_steps.insert(
        profile.into(),
        steps
            .iter()
            .enumerate()
            .map(|(seq, (action, subject, count))| rom_rules::AiOpeningStep {
                seq: seq as i64,
                action: *action,
                subject_def_id: Some(*subject),
                count: *count,
                blocking: true,
                ..template.clone()
            })
            .collect(),
    );
    Arc::new(rules)
}

#[test]
fn opening_book_is_followed_in_order_and_then_left() {
    let base = common::rules();
    let worker = base
        .entity_types
        .values()
        .find(|t| t.builder.is_some() && t.movement.is_some())
        .unwrap()
        .id;
    // The costliest building of the first age the worker can build that is neither a city
    // nor a gather site: something the planner would not start with.
    let first_age = base.ages.values().min_by_key(|a| a.ordinal).unwrap().id;
    let odd = base.production[&worker]
        .iter()
        .map(|p| &base.entity_types[&p.product_def_id])
        .filter(|t| {
            t.class == EntityClass::Building
                && t.gather_site.is_none()
                && t.territory.is_none()
                && t.first_age_id == Some(first_age)
                && base.prerequisites[&t.id].len() == 1
        })
        .map(|t| t.id)
        .next()
        .expect("an unconditional first-age building");
    let rules = with_opening(
        "balanced",
        &[
            (OpeningAction::Train, worker, 8),
            (OpeningAction::Construct, odd, 1),
        ],
    );
    let (mut world, mut brains) = common::plain(&rules, 3, &[("balanced", "tougher")]);
    let mut seen_steps = vec![brains[0].opening_step()];
    let mut workers_when_done = None;
    for _ in 0..1500 {
        common::run(&mut world, &mut brains, 1);
        if seen_steps.last() != Some(&brains[0].opening_step()) {
            seen_steps.push(brains[0].opening_step());
        }
        if workers_when_done.is_none() && brains[0].opening_step() >= 1 {
            // Units and the orders that train them, as the step counts them.
            let queued: usize = world
                .entities()
                .filter(|e| e.owner == ME)
                .map(|e| {
                    e.orders()
                        .iter()
                        .filter(|o| o.subject == Some(worker))
                        .count()
                })
                .sum();
            workers_when_done = Some(own_count(&world, |t| t.id == worker) + queued);
        }
    }
    assert_eq!(seen_steps.first(), Some(&0));
    assert_eq!(seen_steps.last(), Some(&2), "the book was not finished");
    assert!(seen_steps.is_sorted(), "the book went backwards");
    assert!(
        workers_when_done.expect("the first step was passed") >= 8,
        "the first step was left before it held"
    );
    assert!(own_count(&world, |t| t.id == odd) > 0);
    assert!(brains[0].opening_finished());
    assert!(own_count(&world, |t| t.id == worker) >= 8);
}

#[test]
fn an_impossible_opening_step_is_skipped_after_a_while() {
    let base = common::rules();
    // A technology of a later age cannot be researched at the start.
    let late = base
        .techs
        .values()
        .filter(|t| t.level >= 5)
        .map(|t| t.id)
        .next()
        .unwrap();
    let rules = with_opening("balanced", &[(OpeningAction::Research, late, 1)]);
    let (mut world, mut brains) = common::plain(&rules, 3, &[("balanced", "tougher")]);
    common::run(&mut world, &mut brains, 600);
    assert!(!brains[0].opening_finished(), "gave up at once");
    common::run(&mut world, &mut brains, 900);
    assert!(brains[0].opening_finished(), "never gave up");
    // The planner carried on meanwhile.
    assert!(brains[0].summary(&world).unwrap().buildings > 3);
}

#[test]
fn construction_and_research_work_towards_the_next_age() {
    let rules = common::rules();
    let (mut world, mut brains) = common::plain(&rules, 3, &[("builder", "tougher")]);
    common::run(&mut world, &mut brains, 3200);
    // A building that researches age technologies stands, and research has begun.
    let researches_ages = |t: &rom_rules::EntityType| {
        rules.production.get(&t.id).is_some_and(|rows| {
            rows.iter().any(|p| {
                rules
                    .techs
                    .get(&p.product_def_id)
                    .is_some_and(|tech| tech.kind == rom_rules::TechKind::Age)
            })
        })
    };
    assert!(own_count(&world, researches_ages) >= 1);
    let player = world.players().find(|p| p.id == ME).unwrap();
    assert!(!player.techs.is_empty(), "nothing was researched");
    // Everything researched so far lies on the way to the next age.
    let next = rules
        .techs
        .values()
        .filter(|t| t.kind == rom_rules::TechKind::Age)
        .min_by_key(|t| t.level)
        .unwrap();
    let on_path: Vec<DefId> = rules.prerequisites[&next.id]
        .iter()
        .map(|p| p.requires_def_id)
        .collect();
    assert!(player.techs.keys().all(|t| on_path.contains(t)));
}

#[test]
fn production_trains_an_army_and_gathers_it() {
    let rules = common::rules();
    let (mut world, mut brains) = common::plain(&rules, 3, &[("rusher", "tougher")]);
    common::run(&mut world, &mut brains, 5200);
    let summary = brains[0].summary(&world).unwrap();
    assert!(summary.military >= 4, "{summary:?}");
    let rally = brains[0].rally_point().expect("a rally point");
    let resting: Vec<u64> = world
        .entities()
        .filter(|e| {
            let t = &rules.entity_types[&e.type_id];
            e.owner == ME
                && t.class == EntityClass::Unit
                && t.harvester.is_empty()
                && !t.weapons.is_empty()
                && e.orders().is_empty()
        })
        .map(|e| world.distance(e.pos, rally).unwrap())
        .collect();
    assert!(!resting.is_empty());
    assert!(
        resting.iter().filter(|d| **d <= 8000).count() * 2 >= resting.len(),
        "the army is not at its rally point: {resting:?}"
    );
}

#[test]
fn difficulty_limits_thoughts_and_orders() {
    let rules = common::rules();
    let slow = &rules.ai_difficulties["easiest"];
    let (mut world, mut brains) = common::plain(&rules, 3, &[("balanced", "easiest")]);
    let mut issued: Vec<u64> = Vec::new();
    for tick in 0..3600 {
        let commands = brains[0].think(&world, tick);
        for command in commands {
            issued.push(tick);
            match world.submit(command.clone()) {
                Ok(_) => brains[0].accepted(),
                Err(error) => brains[0].rejected(&command, &error, tick),
            }
        }
        world.step();
    }
    // One thought per think interval.
    let stats = brains[0].stats();
    assert_eq!(stats.thinks, 3600 / slow.think_interval_ticks as u64);
    assert!(!issued.is_empty());
    // In any minute, no more orders than the budget and its small reserve allow.
    let per_minute = slow.orders_per_minute as usize;
    for start in (0..3600).step_by(100) {
        let count = issued
            .iter()
            .filter(|t| (start..start + 1200).contains(*t))
            .count();
        assert!(
            count <= per_minute + per_minute / 4 + 1,
            "{count} orders in the minute from tick {start}"
        );
    }
    // A quicker mind does more in the same time.
    let (mut fast_world, mut fast) = common::plain(&rules, 3, &[("balanced", "tougher")]);
    common::run(&mut fast_world, &mut fast, 3600);
    assert!(fast[0].stats().orders > stats.orders);
    assert!(fast[0].stats().thinks > stats.thinks);
}

#[test]
fn mistakes_change_the_choices() {
    let run = |mistake_pct: i64| {
        let mut rules = (*common::rules()).clone();
        rules
            .ai_difficulties
            .get_mut("tougher")
            .unwrap()
            .mistake_pct = mistake_pct;
        let rules = Arc::new(rules);
        let (mut world, mut brains) = common::plain(&rules, 3, &[("balanced", "tougher")]);
        let mut orders = Vec::new();
        for tick in 0..3000 {
            for command in brains[0].think(&world, tick) {
                let _ = world.submit(command.clone());
                orders.push((command.kind, command.subject));
            }
            world.step();
        }
        orders
    };
    let (sure, sloppy) = (run(0), run(100));
    assert_eq!(sure, run(0));
    assert_ne!(sure, sloppy, "a mistake rate of 100 % changed nothing");
}

/// An armed unit type of the first age.
fn soldier(rules: &Rules) -> DefId {
    rules
        .entity_types
        .values()
        .find(|t| {
            t.class == EntityClass::Unit
                && !t.weapons.is_empty()
                && t.harvester.is_empty()
                && t.movement.is_some()
                && t.upgrades_from_id.is_none()
        })
        .unwrap()
        .id
}

#[test]
fn the_fog_hides_enemies_unless_the_difficulty_sees_all() {
    let rules = common::rules();
    let players = [("balanced", "moderate"), ("balanced", "moderate")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    common::run(&mut world, &mut brains, 200);
    assert_eq!(brains[0].known_enemies().count(), 0, "saw through the fog");

    let mut all_seeing = (*rules).clone();
    all_seeing
        .ai_difficulties
        .get_mut("moderate")
        .unwrap()
        .sees_all = true;
    let all_seeing = Arc::new(all_seeing);
    let (mut world, mut brains) = common::plain(&all_seeing, 3, &players);
    common::run(&mut world, &mut brains, 200);
    assert!(brains[0].known_enemies().count() > 0);
    assert!(
        brains[0]
            .known_enemies()
            .all(|(_, e)| e.owner == PlayerId(2))
    );
}

#[test]
fn an_attack_on_a_city_ends_the_opening_after_the_reaction_time() {
    let rules = common::rules();
    let players = [("balanced", "moderate"), ("balanced", "moderate")];
    let (mut world, mut brains) = common::plain(&rules, 3, &players);
    common::run(&mut world, &mut brains, 100);
    assert!(!brains[0].opening_finished());
    // An enemy soldier appears beside player one's city.
    let intruder = world
        .spawn(soldier(&rules), PlayerId(2), Pos::flat(55_500, 48_500))
        .unwrap();
    let reaction = rules.ai_difficulties["moderate"].reaction_ticks as u64;
    let interval = rules.ai_difficulties["moderate"].think_interval_ticks as u64;
    common::run(&mut world, &mut brains[..1], interval);
    assert!(brains[0].known_enemies().any(|(id, _)| id == intruder.0));
    assert!(!brains[0].opening_finished(), "reacted at once");
    common::run(&mut world, &mut brains[..1], reaction + 2 * interval);
    assert!(brains[0].opening_finished(), "did not react");
}

#[test]
fn an_income_bonus_is_an_ordinary_modifier() {
    // The shipped rules carry the bonus in the difficulty's handicap technology (task 64).
    let shipped = common::rules();
    let handicap = rom_ai::handicap_tech(&shipped, "toughest");
    assert!(handicap.is_some());
    assert_eq!(
        handicap,
        shipped.ai_difficulties["toughest"].handicap_tech_id
    );
    assert_eq!(rom_ai::handicap_tech(&shipped, "moderate"), None);

    // With one in the rules, the host grants it and income rises through the stat engine.
    let db = rom_rules::build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "INSERT INTO def(kind,key,name) VALUES ('tech','tech.test_bonus','Test bonus');
         INSERT INTO tech(id,kind,research_ticks) SELECT id,'upgrade',0 FROM def WHERE key='tech.test_bonus';
         INSERT INTO modifier(source_def_id,stat_key,op,value)
          SELECT d.id, s.key, 'pct', (SELECT income_bonus_pct FROM ai_difficulty WHERE key='toughest')
          FROM def d JOIN stat s ON s.key LIKE 'income.%' WHERE d.key='tech.test_bonus';",
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let tech = rom_ai::handicap_tech(&rules, "toughest").expect("the bonus technology");
    assert_eq!(rom_ai::handicap_tech(&rules, "tougher"), None);
    let (mut world, mut brains) = common::plain(&rules, 3, &[("balanced", "toughest")]);
    assert_eq!(brains[0].handicap(), Some(tech));
    assert!(
        world
            .players()
            .find(|p| p.id == ME)
            .unwrap()
            .techs
            .contains_key(&tech)
    );
    common::run(&mut world, &mut brains, 1500);
    let bonus = rules.ai_difficulties["toughest"].income_bonus_pct;
    let boosted: Vec<_> = rules
        .resources
        .keys()
        .map(|r| world.income(ME, *r))
        .filter(|income| income.raw > 0 && !income.capped)
        .collect();
    assert!(!boosted.is_empty());
    for income in boosted {
        assert_eq!(income.effective, income.raw * (100 + bonus) / 100);
    }
}

#[test]
fn unknown_profiles_and_difficulties_are_refused() {
    let rules = common::rules();
    assert_eq!(
        Brain::new(Arc::clone(&rules), ME, "nobody", "moderate", 1).unwrap_err(),
        rom_ai::AiError::UnknownProfile("nobody".into())
    );
    assert_eq!(
        Brain::new(Arc::clone(&rules), ME, "balanced", "hard", 1).unwrap_err(),
        rom_ai::AiError::UnknownDifficulty("hard".into())
    );
    for profile in rules.ai_profiles.keys() {
        for difficulty in rules.ai_difficulties.keys() {
            Brain::new(Arc::clone(&rules), ME, profile, difficulty, 1).unwrap();
        }
    }
}

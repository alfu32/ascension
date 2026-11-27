use std::sync::Arc;

use rom_rules::{DefId, OrderTarget, Rules, build_rules_db};
use rom_sim::{
    Command, EntityId, MapGrid, OrderStatus, Player, PlayerId, Pos, SimError, SimRng, World,
};

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");
const OWNER: PlayerId = PlayerId(1);

fn rules_with(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn rules() -> Arc<Rules> {
    rules_with("")
}

fn player(rules: &Rules, id: PlayerId) -> Player {
    Player {
        id,
        team: id.0,
        resources: Default::default(),
        techs: Default::default(),
        faction: *rules.factions.keys().next().unwrap(),
        age: *rules.ages.keys().next().unwrap(),
    }
}

fn world_on(rules: Arc<Rules>, terrain: Vec<DefId>, width: u32, height: u32) -> World {
    let players = vec![player(&rules, OWNER), player(&rules, PlayerId(2))];
    World::new(
        rules,
        MapGrid::new(width, height, terrain).unwrap(),
        7,
        players,
    )
    .unwrap()
}

fn world_with(rules: Arc<Rules>, terrain: &str) -> World {
    let terrain = rules.id_of(terrain).unwrap();
    world_on(rules, vec![terrain; 32 * 32], 32, 32)
}

fn world() -> World {
    world_with(rules(), "ter.plain")
}

fn citizen(world: &mut World, pos: Pos) -> EntityId {
    world
        .spawn(world.rules().id_of("unit.citizen").unwrap(), OWNER, pos)
        .unwrap()
}

fn move_command(executor: EntityId, target: Pos, queue: bool) -> Command {
    Command {
        player: OWNER,
        executor,
        kind: "move".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(target),
        queue,
        text: None,
        facing: None,
    }
}

fn untargeted(executor: EntityId, kind: &str, queue: bool) -> Command {
    Command {
        player: OWNER,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue,
        text: None,
        facing: None,
    }
}

fn run_scenario(rules: Arc<Rules>, seed: u64) -> World {
    let mut world = World::movement_scenario(rules, seed, 500).unwrap();
    for _ in 0..600 {
        world.step();
    }
    world
}

#[test]
fn same_seed_same_hash() {
    let rules = rules();
    let first = run_scenario(rules.clone(), 7);
    let second = run_scenario(rules, 7);
    assert_eq!(first.hash(), second.hash());
    assert_eq!(first.stats(), second.stats());
    assert_eq!(first.tick(), 600);
    assert_eq!(first.entities().count(), 500);
    assert!(first.stats().orders_done > 0);
    assert_eq!(first.stats().orders_failed, 0);
}

#[test]
fn different_seed_different_hash() {
    let rules = rules();
    assert_ne!(
        run_scenario(rules.clone(), 7).hash(),
        run_scenario(rules, 8).hash()
    );
}

#[test]
fn hash_changes_when_any_entity_moves() {
    let rules = rules();
    for moving_index in 0..3 {
        let mut moving = world_with(rules.clone(), "ter.plain");
        let mut idle = world_with(rules.clone(), "ter.plain");
        for _ in 0..3 {
            citizen(&mut moving, Pos::default());
            citizen(&mut idle, Pos::default());
        }
        let id = EntityId(moving_index + 1);
        moving
            .submit(move_command(
                id,
                Pos {
                    x: 1000,
                    y: 0,
                    face: 0,
                    ..Pos::default()
                },
                false,
            ))
            .unwrap();
        idle.submit(move_command(
            id,
            Pos {
                x: 0,
                y: 1000,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
        moving.step();
        idle.step();
        assert_ne!(moving.entity(id).unwrap().pos, idle.entity(id).unwrap().pos);
        assert_ne!(moving.hash(), idle.hash());
    }
}

fn assert_arrival(mut world: World, distance: i32, ticks: u64) {
    let origin = Pos {
        x: 500,
        y: 500,
        face: 0,
        ..Pos::default()
    };
    let target = Pos {
        x: origin.x + distance,
        y: origin.y,
        face: 0,
        ..Pos::default()
    };
    let id = citizen(&mut world, origin);
    world.submit(move_command(id, target, false)).unwrap();
    for _ in 0..ticks - 1 {
        world.step();
    }
    assert_ne!(world.entity(id).unwrap().pos, target);
    assert_eq!(world.stats().orders_done, 0);
    assert_eq!(
        world.entity(id).unwrap().orders()[0].status,
        OrderStatus::Active
    );
    world.step();
    assert_eq!(world.tick(), ticks);
    assert_eq!(world.entity(id).unwrap().pos, target);
    assert!(world.entity(id).unwrap().orders().is_empty());
    assert_eq!(world.stats().orders_done, 1);
    assert_eq!(world.stats().orders_failed, 0);
}

#[test]
fn unit_reaches_target_in_expected_ticks() {
    assert_arrival(world(), 13_000, 200);
}

#[test]
fn terrain_speed_is_applied() {
    assert_arrival(world_with(rules(), "ter.forest"), 13_000, 286);
}

#[test]
fn no_distance_lost_to_rounding() {
    let rules = rules_with(
        "UPDATE et_movement SET speed=1301 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.citizen');",
    );
    let plain = rules.id_of("ter.plain").unwrap();
    let world = world_on(rules, vec![plain; 20], 20, 1);
    assert_arrival(world, 13_010, 200);
}

#[test]
fn terrain_and_speed_remainders_are_both_carried() {
    let rules = rules_with(
        "UPDATE et_movement SET speed=1 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.citizen');",
    );
    assert_arrival(world_with(rules, "ter.forest"), 7, 200);
}

#[test]
fn slow_diagonal_motion_accumulates_without_stalling_or_drifting() {
    let rules = rules_with(
        "UPDATE et_movement SET speed=20 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.citizen');",
    );
    let mut world = world_with(rules, "ter.plain");
    let id = citizen(
        &mut world,
        Pos {
            x: 100,
            y: 100,
            face: 0,
            ..Pos::default()
        },
    );
    let target = Pos {
        x: 107,
        y: 107,
        face: 0,
        ..Pos::default()
    };
    world.submit(move_command(id, target, false)).unwrap();
    for _ in 0..9 {
        world.step();
        let pos = world.entity(id).unwrap().pos;
        assert_eq!(pos.x, pos.y);
        assert_ne!(pos, target);
    }
    world.step();
    assert_eq!(world.entity(id).unwrap().pos, target);
    assert_eq!(world.entity(id).unwrap().facing, 45);
    assert_eq!(world.stats().orders_done, 1);
}

#[test]
fn terrain_speed_uses_the_tile_stood_on_each_tick() {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    let forest = rules.id_of("ter.forest").unwrap();
    let mut world = world_on(rules, vec![plain, forest], 2, 1);
    let id = citizen(
        &mut world,
        Pos {
            x: 935,
            y: 0,
            face: 0,
            ..Pos::default()
        },
    );
    world
        .submit(move_command(
            id,
            Pos {
                x: 1500,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world.step();
    assert_eq!(world.entity(id).unwrap().pos.x, 1000);
    world.step();
    assert_eq!(world.entity(id).unwrap().pos.x, 1045);
    world.step();
    assert_eq!(world.entity(id).unwrap().pos.x, 1091);
}

#[test]
fn impassable_terrain_fails_the_order() {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    let deep = rules.id_of("ter.deep").unwrap();
    let mut world = world_on(rules, vec![plain, deep], 2, 1);
    let pos = Pos {
        x: 990,
        y: 0,
        face: 0,
        ..Pos::default()
    };
    let id = citizen(&mut world, pos);
    world
        .submit(move_command(
            id,
            Pos {
                x: 1500,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world.step();
    assert_eq!(world.entity(id).unwrap().pos, pos);
    assert!(world.entity(id).unwrap().orders().is_empty());
    assert_eq!(world.stats().orders_failed, 1);
    assert_eq!(world.stats().orders_no_path, 1);
    assert_eq!(world.stats().orders_done, 0);
    assert_eq!(world.entities_in_radius(pos, 0), vec![id]);
}

#[test]
fn leaving_the_map_fails_the_order() {
    for (pos, target) in [
        (
            Pos::default(),
            Pos {
                x: -100,
                y: 0,
                face: 0,
                ..Pos::default()
            },
        ),
        (
            Pos::default(),
            Pos {
                x: 0,
                y: -100,
                face: 0,
                ..Pos::default()
            },
        ),
        (
            Pos {
                x: 31_990,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            Pos {
                x: 32_100,
                y: 0,
                face: 0,
                ..Pos::default()
            },
        ),
        (
            Pos {
                x: 0,
                y: 31_990,
                face: 0,
                ..Pos::default()
            },
            Pos {
                x: 0,
                y: 32_100,
                face: 0,
                ..Pos::default()
            },
        ),
    ] {
        let mut world = world();
        let id = citizen(&mut world, pos);
        world.submit(move_command(id, target, false)).unwrap();
        world.step();
        assert_eq!(world.entity(id).unwrap().pos, pos);
        assert_eq!(world.stats().orders_no_path, 1);
        assert!(world.entity(id).unwrap().orders().is_empty());
    }
}

#[test]
fn arrival_checks_destination_terrain_before_snapping() {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    let deep = rules.id_of("ter.deep").unwrap();
    let mut world = world_on(rules, vec![plain, deep], 2, 1);
    let pos = Pos {
        x: 990,
        y: 0,
        face: 0,
        ..Pos::default()
    };
    let id = citizen(&mut world, pos);
    world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world.step();
    assert_eq!(world.entity(id).unwrap().pos, pos);
    assert_eq!(world.stats().orders_no_path, 1);
}

#[test]
fn commands_apply_next_tick_in_submission_order() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    let first = world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    let second = world
        .submit(move_command(
            id,
            Pos {
                x: 0,
                y: 1000,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    assert!(second > first);
    assert_eq!(world.tick(), 0);
    assert_eq!(world.entity(id).unwrap().pos, Pos::default());
    assert!(world.entity(id).unwrap().orders().is_empty());
    world.step();
    let entity = world.entity(id).unwrap();
    assert_eq!(
        entity.pos,
        Pos {
            x: 0,
            y: 65,
            face: 0,
            ..Pos::default()
        }
    );
    assert_eq!(entity.orders().len(), 1);
    assert_eq!(entity.orders()[0].id, second);
    assert_eq!(entity.orders()[0].issued_tick, 0);
    assert_eq!(entity.orders()[0].progress_ticks, 1);
    assert_eq!(entity.orders()[0].status, OrderStatus::Active);
    assert_eq!(world.stats().orders_cancelled, 1);
}

#[test]
fn queue_flag_appends_and_replace_clears() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    let a = world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    let b = world
        .submit(move_command(
            id,
            Pos {
                x: 2000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            true,
        ))
        .unwrap();
    world.step();
    assert_eq!(
        world
            .entity(id)
            .unwrap()
            .orders()
            .iter()
            .map(|o| o.id)
            .collect::<Vec<_>>(),
        vec![a, b]
    );
    assert_eq!(
        world.entity(id).unwrap().orders()[1].status,
        OrderStatus::Queued
    );
    let replacement = world
        .submit(move_command(
            id,
            Pos {
                x: 65,
                y: 1000,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    assert_eq!(world.entity(id).unwrap().orders().len(), 2);
    world.step();
    let entity = world.entity(id).unwrap();
    assert_eq!(entity.orders().len(), 1);
    assert_eq!(entity.orders()[0].id, replacement);
    assert_eq!(
        entity.pos,
        Pos {
            x: 65,
            y: 65,
            face: 0,
            ..Pos::default()
        }
    );
    assert_eq!(world.stats().orders_cancelled, 2);
}

#[test]
fn finishing_an_order_activates_the_next_without_a_second_movement_step() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    world
        .submit(move_command(
            id,
            Pos {
                x: 65,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    let next = world
        .submit(move_command(
            id,
            Pos {
                x: 130,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            true,
        ))
        .unwrap();
    world.step();
    let entity = world.entity(id).unwrap();
    assert_eq!(entity.pos.x, 65);
    assert_eq!(entity.orders()[0].id, next);
    assert_eq!(entity.orders()[0].status, OrderStatus::Active);
    assert_eq!(entity.orders()[0].progress_ticks, 0);
    world.step();
    assert_eq!(world.entity(id).unwrap().pos.x, 130);
    assert_eq!(world.stats().orders_done, 2);
}

#[test]
fn stop_clears_the_queue() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world
        .submit(move_command(
            id,
            Pos {
                x: 2000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            true,
        ))
        .unwrap();
    world.step();
    let before = world.entity(id).unwrap().pos;
    world.submit(untargeted(id, "stop", false)).unwrap();
    assert_eq!(world.entity(id).unwrap().orders().len(), 2);
    world.step();
    assert_eq!(world.entity(id).unwrap().pos, before);
    assert!(world.entity(id).unwrap().orders().is_empty());
    assert_eq!(world.stats().orders_done, 1);
    assert_eq!(world.stats().orders_cancelled, 2);
}

#[test]
fn queued_stop_clears_orders_behind_it_when_it_becomes_active() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    world
        .submit(move_command(
            id,
            Pos {
                x: 130,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world.submit(untargeted(id, "stop", true)).unwrap();
    world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            true,
        ))
        .unwrap();
    world.step();
    assert_eq!(world.entity(id).unwrap().orders().len(), 3);
    world.step();
    assert_eq!(world.entity(id).unwrap().pos.x, 130);
    assert!(world.entity(id).unwrap().orders().is_empty());
    assert_eq!(world.stats().orders_done, 2);
    assert_eq!(world.stats().orders_cancelled, 1);
}

#[test]
fn submit_rejects_unknown_executor() {
    let mut world = world();
    let hash = world.hash();
    assert_eq!(
        world.submit(move_command(EntityId(999), Pos::default(), false)),
        Err(SimError::UnknownExecutor(EntityId(999)))
    );
    assert_eq!(world.hash(), hash);
}

#[test]
fn submit_rejects_foreign_owner() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    let mut command = move_command(id, Pos::default(), false);
    command.player = PlayerId(2);
    let hash = world.hash();
    assert_eq!(world.submit(command), Err(SimError::ForeignOwner(id)));
    assert_eq!(world.hash(), hash);
}

#[test]
fn submit_rejects_missing_component() {
    let mut world = world();
    let id = world
        .spawn(
            world.rules().id_of("bld.temple").unwrap(),
            OWNER,
            Pos::default(),
        )
        .unwrap();
    let hash = world.hash();
    assert_eq!(
        world.submit(move_command(id, Pos::default(), false)),
        Err(SimError::MissingComponent("movement".into()))
    );
    assert_eq!(world.hash(), hash);
}

#[test]
fn submit_rejects_wrong_target_shape() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    let mut both = move_command(id, Pos::default(), false);
    both.target_entity = Some(id);
    let mut stop = untargeted(id, "stop", false);
    stop.target_pos = Some(Pos::default());
    let mut repair = untargeted(id, "repair", false);
    repair.target_pos = Some(Pos::default());
    let mut construct = untargeted(id, "construct", false);
    construct.target_pos = Some(Pos::default());
    for command in [untargeted(id, "move", false), both, stop, repair, construct] {
        let hash = world.hash();
        assert_eq!(world.submit(command), Err(SimError::WrongTargetShape));
        assert_eq!(world.hash(), hash);
    }
}

#[test]
fn submit_rejects_unknown_kind() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    let hash = world.hash();
    assert_eq!(
        world.submit(untargeted(id, "not_an_order", false)),
        Err(SimError::UnknownKind("not_an_order".into()))
    );
    assert_eq!(world.hash(), hash);
}

#[test]
fn component_validation_follows_the_rules_requirement() {
    let rules = rules_with("UPDATE order_kind SET needs='' WHERE key='move';");
    let mut world = world_with(rules, "ter.plain");
    let id = world
        .spawn(
            world.rules().id_of("bld.temple").unwrap(),
            OWNER,
            Pos::default(),
        )
        .unwrap();
    world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world.step();
    assert_eq!(world.stats().orders_unsupported, 1);
    assert_eq!(world.entity(id).unwrap().pos, Pos::default());
}

#[test]
fn unsupported_kinds_fail_when_activated() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    world
        .submit(move_command(
            id,
            Pos {
                x: 130,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    // `explore` runs since task 65 (tests/tactics.rs); following an entity does not yet.
    let mut follow = untargeted(id, "move", true);
    follow.target_entity = Some(id);
    world.submit(follow).unwrap();
    assert_eq!(world.stats().orders_failed, 0);
    world.step();
    assert_eq!(
        world.entity(id).unwrap().orders()[1].status,
        OrderStatus::Queued
    );
    assert_eq!(world.stats().orders_failed, 0);
    world.step();
    assert!(world.entity(id).unwrap().orders().is_empty());
    assert_eq!(world.stats().orders_done, 1);
    assert_eq!(world.stats().orders_failed, 1);
    assert_eq!(world.stats().orders_unsupported, 1);
}

#[test]
fn every_vocabulary_kind_validates_its_component_and_target() {
    let rules =
        rules_with("UPDATE game_constant SET value=1000 WHERE key='player.caravan_limit.base';");
    let mut world = world_with(rules.clone(), "ter.plain");
    let farm = world
        .spawn(rules.id_of("bld.farm").unwrap(), OWNER, Pos::default())
        .unwrap();
    let enemy = world
        .spawn(
            rules.id_of("unit.citizen").unwrap(),
            PlayerId(2),
            Pos::default(),
        )
        .unwrap();
    let patient = world
        .spawn(rules.id_of("unit.citizen").unwrap(), OWNER, Pos::default())
        .unwrap();
    let mechanical = world
        .spawn(
            rules.id_of("unit.repair_drone").unwrap(),
            OWNER,
            Pos::default(),
        )
        .unwrap();
    let carry_node = world
        .spawn(
            rules.id_of("node.relics").unwrap(),
            OWNER,
            Pos::flat(1500, 1500),
        )
        .unwrap();
    let city = rules.id_of("bld.city").unwrap();
    world.spawn(city, OWNER, Pos::flat(4500, 4500)).unwrap();
    let partner = world.spawn(city, OWNER, Pos::flat(28500, 4500)).unwrap();
    for kind in rules
        .order_kinds
        .values()
        // `descend` has no executor: an ascended player gives it (task 76, `ascension_tests`);
        // nor have `bring_life` and `paint` (task 77, `life_tests`).
        .filter(|kind| {
            !matches!(
                kind.key.as_str(),
                "train" | "research" | "construct" | "descend" | "bring_life" | "paint"
            )
        })
    {
        let mut accepted = false;
        let mut needs_space = false;
        let mut rejected = false;
        for et in rules.entity_types.values() {
            let id = world.spawn(et.id, OWNER, Pos::default()).unwrap();
            let mut command = untargeted(id, &kind.key, false);
            match kind.target {
                OrderTarget::None => (),
                OrderTarget::Point | OrderTarget::EntityOrPoint => {
                    command.target_pos = Some(Pos::default())
                }
                OrderTarget::Entity => command.target_entity = Some(id),
                OrderTarget::Def => command.subject = Some(et.id),
            }
            if kind.key == "harvest" {
                let site = et.harvester.first().and_then(|h| {
                    rules.entity_types.values().find(|site| {
                        site.gather_site
                            .as_ref()
                            .is_some_and(|s| s.resource_id == h.resource_id && s.slots_base > 0)
                    })
                });
                command.target_entity = Some(match site {
                    Some(site) => world.spawn(site.id, OWNER, Pos::default()).unwrap(),
                    None => farm,
                });
            }
            if kind.key == "carry" {
                command.target_entity = Some(carry_node);
            }
            if kind.key == "trade" {
                command.target_entity = Some(partner);
            }
            if kind.key == "attack" {
                command.target_entity = Some(enemy);
            }
            if kind.key == "guard" {
                command.target_entity = Some(patient);
            }
            if kind.key == "stance" {
                command.subject = rules.id_of("stance.defensive");
            }
            if kind.key == rom_sim::RENAME_CITY {
                command.text = Some("Nova Roma".into());
            }
            if matches!(kind.key.as_str(), "buy" | "sell") {
                command.subject = rules.id_of("res.food");
            }
            if kind.key == "heal" {
                command.target_entity = Some(
                    if et.healer.as_ref().is_some_and(|h| {
                        rules.has_tag(rules.id_of("unit.citizen").unwrap(), h.target_tag_id)
                    }) {
                        patient
                    } else {
                        mechanical
                    },
                );
            }
            match world.submit(command) {
                Ok(_) => {
                    accepted = true;
                }
                Err(SimError::MissingComponent(needs)) => {
                    assert_eq!(needs, kind.needs);
                    rejected = true;
                }
                Err(SimError::CannotCarry) => {
                    assert_eq!(kind.key, "carry");
                    let tag = rules.entity_types[&rules.id_of("node.relics").unwrap()]
                        .node
                        .as_ref()
                        .unwrap()
                        .claim_tag_id
                        .unwrap();
                    assert!(!rules.has_tag(et.id, tag));
                }
                // Only buildings can be razed, never a finished city; a guard cannot guard
                // itself.
                Err(SimError::CannotRaze) => {
                    assert_eq!(kind.key, "raze");
                    assert!(
                        et.class != rom_rules::EntityClass::Building
                            || et.territory.as_ref().is_some_and(|t| t.capturable)
                    );
                }
                // A finished city is captured, never deleted (task 142).
                Err(SimError::CannotDelete) => {
                    assert_eq!(kind.key, "delete");
                    assert!(et.territory.as_ref().is_some_and(|t| t.capturable));
                }
                // Only weapons that throw a projectile attack the ground (task 142), and a ship's
                // orbital weapons only from orbit.
                Err(SimError::NoGroundWeapon) => {
                    assert_eq!(kind.key, "attack_ground");
                    assert!(
                        !world.attacks_ground(et.id)
                            || et
                                .weapons
                                .iter()
                                .filter(|w| w.projectile_speed > 0
                                    && w.targets
                                        .iter()
                                        .any(|t| t.layer == rom_rules::Layer::Ground))
                                .all(|w| w.orbit_only),
                        "{}",
                        rules.key(et.id)
                    );
                }
                // A market reaches the trade's own checks (task 107, tests/market.rs): this
                // player has not researched trading.
                Err(SimError::NeedsMarketTrading) => {
                    assert!(matches!(kind.key.as_str(), "buy" | "sell"));
                    assert!(et.market.is_some());
                    accepted = true;
                }
                // Only workers with a call-up are called up and only militia stand down
                // (task 141, `militia_tests`).
                Err(SimError::CannotCallUp) => {
                    assert_eq!(kind.key, "call_up");
                    assert!(et.call_up.is_none());
                }
                // Only a city is renamed (task 144, `city_names_tests`).
                Err(SimError::NotACity) => {
                    assert_eq!(kind.key, rom_sim::RENAME_CITY);
                    assert!(!et.territory.as_ref().is_some_and(|t| t.is_city));
                }
                Err(SimError::CannotStandDown) => {
                    assert_eq!(kind.key, "stand_down");
                    assert!(rules.militia_worker(et.id).is_none());
                }
                Err(SimError::InvalidGuardTarget) => {
                    assert_eq!(kind.key, "guard");
                    assert_eq!(id, patient);
                }
                Err(SimError::CannotEnterSpace) => {
                    assert!(matches!(kind.key.as_str(), "orbit" | "travel" | "launch"));
                    needs_space = true;
                }
                // A space freighter trades from a planet it holds orbit above or stands on.
                Err(SimError::NoHomeCity) => {
                    assert_eq!(kind.key, "trade");
                    assert!(world.is_spacecraft_type(et.id));
                    needs_space = true;
                }
                // A satellite takes only the orders that keep it in its orbit.
                Err(SimError::BoundToOrbit) => {
                    assert!(world.is_orbit_bound(et.id), "{}", kind.key);
                    needs_space = true;
                }
                // Carrying needs a carrier in orbit or landed on a planet of a universe.
                Err(
                    SimError::CannotBoard
                    | SimError::CannotLand
                    | SimError::NotLanded
                    | SimError::CannotUnload
                    | SimError::ShipNotLanded
                    | SimError::CannotClimb,
                ) => {
                    assert!(matches!(
                        kind.key.as_str(),
                        "enter" | "land" | "lift" | "unload"
                    ));
                    needs_space = true;
                }
                // A jump needs a unit tagged `jumper` and its jetpacks (task 169,
                // `leap_tests`); this player has researched nothing.
                Err(SimError::CannotJump) => {
                    assert_eq!(kind.key, "jump");
                    needs_space = true;
                }
                // A drop needs a ship in orbit with troops aboard (task 169,
                // tests/orbital_drop.rs).
                Err(SimError::NotInOrbit | SimError::CannotDrop) => {
                    assert_eq!(kind.key, "drop");
                    needs_space = true;
                }
                other => panic!("unexpected result for {}: {other:?}", kind.key),
            }
        }
        assert!(
            accepted || needs_space,
            "seed has no executor for {}",
            kind.key
        );
        assert_eq!(rejected, !kind.needs.is_empty(), "{}", kind.key);
    }
    world.step();
}

#[test]
fn subjects_and_construction_placement_are_accepted() {
    let mut world = world();
    let city = world.rules().id_of("bld.city").unwrap();
    world
        .place_for_setup(
            city,
            OWNER,
            Pos {
                x: 20_500,
                y: 20_500,
                face: 0,
                ..Pos::default()
            },
        )
        .unwrap();
    let id = citizen(&mut world, Pos::default());
    let mut command = untargeted(id, "construct", false);
    command.subject = world.rules().id_of("bld.farm");
    // The whole 4 x 4 footprint stands inside the city's 16-tile economic radius.
    command.target_pos = Some(Pos {
        x: 12_000,
        y: 12_000,
        face: 0,
        ..Pos::default()
    });
    world
        .grant(OWNER, world.rules().id_of("res.timber").unwrap(), 50)
        .unwrap();
    world.submit(command).unwrap();
    world.step();
    assert_eq!(world.stats().orders_unsupported, 0);
    assert_eq!(world.entities().count(), 3);
}

#[test]
fn entity_ids_are_never_reused() {
    let mut world = world();
    let first = citizen(&mut world, Pos::default());
    world.despawn(first).unwrap();
    let second = citizen(&mut world, Pos::default());
    assert!(second > first);
    assert!(world.entity(first).is_none());
    assert!(world.entity(second).is_some());
    assert_eq!(world.next_entity_id().0, second.0 + 1);
    assert_eq!(
        world.entity(second).unwrap().hp,
        world
            .rules()
            .entity_type(world.entity(second).unwrap().type_id)
            .unwrap()
            .hp
    );
}

#[test]
fn iteration_is_in_ascending_id_order() {
    let mut world = world();
    let ids: Vec<_> = (0..20)
        .map(|_| citizen(&mut world, Pos::default()))
        .collect();
    for index in [1, 3, 10, 18] {
        world.despawn(ids[index]).unwrap();
    }
    citizen(&mut world, Pos::default());
    let live: Vec<_> = world.entities().map(|e| e.id).collect();
    assert_eq!(live.len(), 17);
    assert!(live.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(world.players().map(|p| p.id).is_sorted());
}

fn assert_queries_match(world: &World, rng: &mut SimRng) {
    for _ in 0..50 {
        let center = Pos {
            x: rng.range(0, 40_000) as i32 - 4000,
            y: rng.range(0, 40_000) as i32 - 4000,
            face: 0,
            ..Pos::default()
        };
        let radius = rng.range(0, 20_000);
        let expected: Vec<_> = world
            .entities()
            .filter(|e| e.pos.distance_squared(center) <= i64::from(radius).pow(2))
            .map(|e| e.id)
            .collect();
        assert_eq!(world.entities_in_radius(center, radius), expected);
    }
}

#[test]
fn spatial_query_matches_brute_force() {
    let mut world = world();
    let mut rng = SimRng::new(42);
    for _ in 0..1000 {
        let pos = Pos {
            x: rng.range(0, 32_000) as i32,
            y: rng.range(0, 32_000) as i32,
            face: 0,
            ..Pos::default()
        };
        let id = citizen(&mut world, pos);
        let target = Pos {
            x: rng.range(0, 32_000) as i32,
            y: rng.range(0, 32_000) as i32,
            face: 0,
            ..Pos::default()
        };
        world.submit(move_command(id, target, false)).unwrap();
    }
    assert_queries_match(&world, &mut rng);
    for _ in 0..200 {
        world.step();
    }
    assert_queries_match(&world, &mut rng);
    for id in (1..=1000).step_by(3) {
        world.despawn(EntityId(id)).unwrap();
    }
    assert_queries_match(&world, &mut rng);
}

#[test]
fn spatial_query_includes_exact_boundary_and_extreme_queries() {
    let mut world = world();
    let a = citizen(
        &mut world,
        Pos {
            x: 4000,
            y: 3000,
            face: 0,
            ..Pos::default()
        },
    );
    citizen(
        &mut world,
        Pos {
            x: 4000,
            y: 3001,
            face: 0,
            ..Pos::default()
        },
    );
    assert_eq!(world.entities_in_radius(Pos::default(), 5000), vec![a]);
    assert_eq!(
        world.entities_in_radius(
            Pos {
                x: 4000,
                y: 3000,
                face: 0,
                ..Pos::default()
            },
            0
        ),
        vec![a]
    );
    assert!(
        world
            .entities_in_radius(
                Pos {
                    x: i32::MIN,
                    y: i32::MIN,
                    face: 0,
                    ..Pos::default()
                },
                1
            )
            .is_empty()
    );
    assert_eq!(
        world
            .entities_in_radius(
                Pos {
                    x: i32::MIN,
                    y: i32::MIN,
                    face: 0,
                    ..Pos::default()
                },
                u32::MAX
            )
            .len(),
        2
    );
}

#[test]
fn despawn_removes_from_spatial_grid() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    assert_eq!(world.entities_in_radius(Pos::default(), 0), vec![id]);
    world.despawn(id).unwrap();
    assert!(world.entities_in_radius(Pos::default(), 1000).is_empty());
    assert_eq!(world.stats().orders_cancelled, 1);
    world.step();
    assert!(world.entities().next().is_none());
    assert_eq!(world.despawn(id), Err(SimError::UnknownExecutor(id)));
}

#[test]
fn despawn_cancels_live_and_pending_orders() {
    let mut world = world();
    let id = citizen(&mut world, Pos::default());
    world
        .submit(move_command(
            id,
            Pos {
                x: 1000,
                y: 0,
                face: 0,
                ..Pos::default()
            },
            false,
        ))
        .unwrap();
    world.step();
    world.submit(untargeted(id, "stop", false)).unwrap();
    world.despawn(id).unwrap();
    assert_eq!(world.stats().orders_cancelled, 2);
    world.step();
    assert_eq!(world.stats().orders_done, 0);
}

#[test]
fn map_bounds_and_invalid_spawns_are_checked_without_consuming_ids() {
    assert_eq!(MapGrid::new(0, 1, vec![]), Err(SimError::InvalidMap));
    assert_eq!(MapGrid::new(1, 1, vec![]), Err(SimError::InvalidMap));
    let mut world = world();
    assert_eq!(world.map().terrain_at(-1, 0), None);
    assert_eq!(world.map().terrain_at(32, 0), None);
    assert_eq!(world.map().terrain_at(0, 32), None);
    assert!(world.map().terrain_at(31, 31).is_some());
    let type_id = world.rules().id_of("unit.citizen").unwrap();
    let hash = world.hash();
    assert_eq!(
        world.spawn(
            type_id,
            OWNER,
            Pos {
                x: -1,
                y: 0,
                face: 0,
                ..Pos::default()
            }
        ),
        Err(SimError::OutsideMap)
    );
    assert_eq!(
        world.spawn(DefId(u32::MAX), OWNER, Pos::default()),
        Err(SimError::UnknownType(DefId(u32::MAX)))
    );
    assert_eq!(
        world.spawn(type_id, PlayerId(3), Pos::default()),
        Err(SimError::UnknownPlayer(PlayerId(3)))
    );
    assert_eq!(world.hash(), hash);
}

#[test]
fn repeated_identical_scenarios_match_after_every_tick() {
    let rules = rules();
    let mut first = World::movement_scenario(rules.clone(), 123, 40).unwrap();
    let mut second = World::movement_scenario(rules, 123, 40).unwrap();
    assert_eq!(first.hash(), second.hash());
    for _ in 0..600 {
        first.step();
        second.step();
        assert_eq!(first.hash(), second.hash());
    }
}

/// Task 85: the leader's journal. Only accepted commands are kept, in submit order; replaying
/// them into a second world gives the same world; recording changes nothing, and run twice the
/// result is the same.
#[test]
fn accepted_commands_are_recorded_in_order_and_replay_alike() {
    let play = |record: bool| {
        let mut world = world();
        let mut replica = world_with(Arc::clone(world.rules()), "ter.plain");
        world.record_accepted(record);
        let mut kept = Vec::new();
        let walkers: Vec<EntityId> = (0..3)
            .map(|n| citizen(&mut world, Pos::flat(4_500 + n * 2_000, 4_500)))
            .collect();
        for n in 0..3 {
            citizen(&mut replica, Pos::flat(4_500 + n * 2_000, 4_500));
        }
        for tick in 0..60_i32 {
            if tick % 10 == 0 {
                for (n, walker) in walkers.iter().enumerate() {
                    let goal = Pos::flat(6_000 + tick * 300, 8_000 + n as i32 * 3_000);
                    world.submit(move_command(*walker, goal, false)).unwrap();
                }
                // Refused: not recorded.
                let foreign = Command {
                    player: PlayerId(2),
                    ..move_command(walkers[0], Pos::flat(9_000, 9_000), false)
                };
                assert!(world.submit(foreign).is_err());
            }
            let journal = world.take_accepted();
            for command in &journal {
                replica.submit(command.clone()).unwrap();
            }
            kept.extend(journal);
            world.step();
            replica.step();
            if record {
                assert_eq!(replica.hash(), world.hash(), "tick {tick}");
            }
        }
        (world.hash(), kept)
    };
    let (hash, kept) = play(true);
    assert_eq!(kept.len(), 18);
    assert!(kept.iter().all(|c| c.player == OWNER && c.kind == "move"));
    assert_eq!(kept[0].executor, kept[3].executor);
    let (unrecorded, nothing) = play(false);
    assert!(nothing.is_empty());
    assert_eq!(unrecorded, hash, "recording does not change the world");
    assert_eq!(play(true).0, hash, "run twice, the same");
    // Orders a step gives itself are not recorded: armed enemies side by side attack at once.
    let mut world = world();
    world.record_accepted(true);
    let armed = world
        .rules()
        .entity_types
        .values()
        .find(|et| et.movement.is_some() && !et.weapons.is_empty())
        .map(|et| et.id)
        .unwrap();
    world
        .spawn(armed, OWNER, Pos::flat(10_500, 10_500))
        .unwrap();
    world
        .spawn(armed, PlayerId(2), Pos::flat(11_500, 10_500))
        .unwrap();
    for _ in 0..40 {
        world.step();
    }
    assert!(
        world
            .entities()
            .any(|e| e.orders().iter().any(|o| o.kind == "attack"))
            || world.stats().losses.values().sum::<u64>() > 0,
        "the step gave itself an attack"
    );
    assert!(world.take_accepted().is_empty(), "a step records nothing");
    let walker = citizen(&mut world, Pos::flat(4_500, 4_500));
    world
        .submit(move_command(walker, Pos::flat(9_000, 9_000), false))
        .unwrap();
    world.record_accepted(false);
    assert!(
        world.take_accepted().is_empty(),
        "stopping drops what was kept"
    );
}

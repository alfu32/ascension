//! System-space scenarios use the same movement, income, combat and fog as planets.
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SimError,
    SystemInfo, Topology, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

fn world_at(sql: &str, age_key: &str) -> World {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    let rules = Arc::new(Rules::load(&conn).unwrap());
    let id = |k| rules.id_of(k).unwrap();
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: id("nation.romans"),
            age: id(age_key),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let body = |n, system, kind, radius, angle| BodyInfo {
        id: BodyId(n),
        system,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: angle,
        n: (kind == BodyKind::Habitable).then_some(64),
        home_race: None,
        node_type: (kind == BodyKind::Resource).then_some(id("node.ore_asteroid")),
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: [(1, 0, 0), (2, 100, 0), (3, 200, 0), (4, 0, 200)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
                z: 0,
                constellation: String::new(),
                id,
                x,
                y,
                name: format!("System {id}"),
            })
            .collect(),
        bodies: vec![
            body(1, 1, BodyKind::Star, 0, 0),
            body(2, 1, BodyKind::Habitable, 20, 0),
            body(3, 1, BodyKind::Resource, 40, 90_000),
            body(4, 1, BodyKind::Resource, 40, 270_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
            body(12, 2, BodyKind::Resource, 40, 90_000),
            body(20, 3, BodyKind::Star, 0, 0),
            body(21, 3, BodyKind::Resource, 40, 90_000),
            body(30, 4, BodyKind::Star, 0, 0),
            body(31, 4, BodyKind::Resource, 40, 90_000),
        ],
        routes: vec![
            RouteInfo {
                system_a: 1,
                system_b: 2,
                length: 22,
            },
            RouteInfo {
                system_a: 2,
                system_b: 3,
                length: 22,
            },
            RouteInfo {
                system_a: 1,
                system_b: 3,
                length: 100,
            },
        ],
        planets: BTreeMap::from([(
            BodyId(2),
            PlanetSurface {
                surface: CubeSurface { n: 64 },
                terrain: vec![id("ter.plain"); 6 * 64 * 64],
                nodes: vec![],
            },
        )]),
    };
    World::from_universe(
        rules,
        setup,
        7,
        players,
        BTreeMap::from([(PlayerId(1), BodyId(2)), (PlayerId(2), BodyId(2))]),
    )
    .unwrap()
}
fn world_with(sql: &str, early: bool) -> World {
    world_at(
        sql,
        if early {
            "age.ancient"
        } else {
            "age.expansion"
        },
    )
}
fn world() -> World {
    world_with("", false)
}
/// Route time alone: no jump charge and no charting delay (task 70 tests those in `jumps.rs`).
const NO_JUMP_DELAYS: &str = "UPDATE et_movement SET jump_charge_ticks=0; \
    UPDATE game_constant SET value=0 WHERE key='space.jump_charge_ticks'; \
    UPDATE game_constant SET value=100 WHERE key='space.charting_pct';";
fn quick_world() -> World {
    world_with(NO_JUMP_DELAYS, false)
}
fn command(w: &World, ship: EntityId, kind: &str) -> Command {
    Command {
        player: w.entity(ship).unwrap().owner,
        executor: ship,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
    }
}
fn ship(w: &mut World, player: u32, pos: Pos) -> EntityId {
    w.spawn(
        w.rules().id_of("unit.survey_ship").unwrap(),
        PlayerId(player),
        pos,
    )
    .unwrap()
}
fn run(w: &mut World, ticks: u64) {
    for _ in 0..ticks {
        w.step();
    }
}
fn travel(w: &mut World, ship: EntityId, system: u32) -> Result<rom_sim::OrderId, SimError> {
    w.submit(Command {
        target_pos: Some(Pos {
            body: w.space_of(system).unwrap(),
            ..Pos::default()
        }),
        ..command(w, ship, "travel")
    })
}
fn balance(w: &World, player: u32) -> i64 {
    *w.players()
        .find(|p| p.id == PlayerId(player))
        .unwrap()
        .resources
        .get(&w.rules().id_of("res.metal").unwrap())
        .unwrap_or(&0)
}
fn claimant(w: &World, body: u32) -> Option<PlayerId> {
    w.claims()
        .find_map(|(b, p)| (b == BodyId(body)).then_some(p))
}

#[test]
fn every_system_has_a_space_surface_with_body_markers_at_fixed_positions() {
    let mut w = world();
    assert_eq!(w.surfaces().len(), 5);
    for system in w.systems() {
        let space = w.space_of(system.id).unwrap();
        let surface = w.surface(space).unwrap();
        let Topology::Flat { width, height } = surface.topology() else {
            panic!("space must be flat");
        };
        assert!(width <= 512 && height <= 512);
        assert_eq!(
            w.body_position(space),
            Some(Pos::on(space, 0, width as i32 * 500, height as i32 * 500))
        );
    }
    let markers: Vec<_> = w
        .bodies()
        .iter()
        .filter(|b| b.kind != BodyKind::Star)
        .map(|b| {
            let marker = w.body_marker(b.id).unwrap();
            let e = w.entity(marker).unwrap();
            assert_eq!(e.owner, PlayerId(0));
            assert_eq!(Some(e.pos), w.body_position(b.id));
            assert_eq!(w.space_of(b.system), Some(e.pos.body));
            (marker, e.pos)
        })
        .collect();
    assert_eq!(
        w.body_position(BodyId(3)),
        Some(Pos::on(BodyId(1), 0, 56_000, 96_000))
    );
    run(&mut w, 40);
    for (marker, pos) in markers {
        assert_eq!(w.entity(marker).unwrap().pos, pos);
    }
}

#[test]
fn ground_units_cannot_enter_space() {
    let mut w = world();
    let space = w.body_position(BodyId(3)).unwrap();
    let citizen = w.rules().id_of("unit.citizen").unwrap();
    assert_eq!(
        w.spawn(citizen, PlayerId(1), space),
        Err(SimError::CannotEnterSpace)
    );
    let unit = w
        .spawn(citizen, PlayerId(1), Pos::on(BodyId(2), 0, 10_000, 10_000))
        .unwrap();
    assert_eq!(
        w.submit(Command {
            target_pos: Some(space),
            ..command(&w, unit, "move")
        }),
        Err(SimError::OtherBody)
    );
    assert_eq!(travel(&mut w, unit, 2), Err(SimError::CannotEnterSpace));
    let mut early = world_with("", true);
    assert_eq!(
        early.spawn(
            early.rules().id_of("unit.survey_ship").unwrap(),
            PlayerId(1),
            space
        ),
        Err(SimError::CannotEnterSpace)
    );
}

#[test]
fn orbit_order_holds_within_reach() {
    let mut w = world();
    let goal = w.body_position(BodyId(3)).unwrap();
    let start = w.offset(goal, 0, -15_000);
    let ship = ship(&mut w, 1, start);
    let marker = w.body_marker(BodyId(3));
    w.submit(Command {
        target_entity: marker,
        ..command(&w, ship, "orbit")
    })
    .unwrap();
    run(&mut w, 200);
    let held = w.entity(ship).unwrap().pos;
    assert!(
        w.distance(held, goal).unwrap() <= w.rules().constant("space.orbit_reach").unwrap() as u64
    );
    assert_eq!(claimant(&w, 3), Some(PlayerId(1)));
    run(&mut w, 60);
    assert_eq!(w.entity(ship).unwrap().pos, held);
    assert_eq!(w.entity(ship).unwrap().orders()[0].kind, "orbit");
    let point = w.offset(held, 10_000, 0);
    w.submit(Command {
        target_pos: Some(point),
        ..command(&w, ship, "orbit")
    })
    .unwrap();
    run(&mut w, 100);
    assert!(w.distance(w.entity(ship).unwrap().pos, point).unwrap() <= 6000);
}

#[test]
fn presence_claim_pays_income_and_counts_toward_the_cap() {
    let mut w = world_with(
        "UPDATE et_node SET income=120 WHERE claim_mode='presence';",
        false,
    );
    let pos = w.body_position(BodyId(3)).unwrap();
    ship(&mut w, 1, pos);
    run(&mut w, 20);
    let metal = w.rules().id_of("res.metal").unwrap();
    let income = w.income(PlayerId(1), metal);
    assert_eq!(
        (income.raw, income.effective, income.capped),
        (120, 70, true)
    );
    let before = balance(&w, 1);
    run(&mut w, 600);
    assert_eq!(balance(&w, 1) - before, 70);
}

#[test]
fn contested_body_pays_nobody() {
    let mut w = world();
    let pos = w.body_position(BodyId(3)).unwrap();
    ship(&mut w, 1, pos);
    ship(&mut w, 2, pos);
    run(&mut w, 620);
    assert_eq!(claimant(&w, 3), None);
    assert_eq!((balance(&w, 1), balance(&w, 2)), (0, 0));
}

#[test]
fn body_claim_and_yield_report_what_the_economy_pays() {
    use rom_sim::BodyClaim;
    let mut w = world();
    let body = BodyId(3);
    assert_eq!(w.body_claim(body), BodyClaim::Unclaimed);
    let paid = w.body_yield(body, PlayerId(1)).expect("a presence body");
    assert_eq!(paid.period_ticks, 600);
    let pos = w.body_position(body).unwrap();
    ship(&mut w, 1, pos);
    run(&mut w, 40);
    assert_eq!(w.body_claim(body), BodyClaim::Held(PlayerId(1)));
    // What the body lists is what the player's income gains by holding it.
    let income = w.income(PlayerId(1), paid.resource);
    assert_eq!(income.effective, paid.per_period.min(income.effective));
    assert_eq!(
        income.raw,
        w.body_yield(body, PlayerId(1)).unwrap().per_period
    );
    // Rivals at once: contested, and no one is paid.
    ship(&mut w, 2, pos);
    run(&mut w, 40);
    assert_eq!(w.body_claim(body), BodyClaim::Contested);
    // A habitable planet's marker is no presence body.
    assert!(w.body_yield(BodyId(2), PlayerId(1)).is_none());
}

#[test]
fn claim_persists_when_the_ship_leaves_until_someone_else_claims() {
    let mut w = world();
    let pos = w.body_position(BodyId(3)).unwrap();
    let a = ship(&mut w, 1, pos);
    run(&mut w, 20);
    w.submit(Command {
        target_pos: Some(w.offset(pos, 20_000, 0)),
        ..command(&w, a, "move")
    })
    .unwrap();
    run(&mut w, 250);
    assert_eq!(claimant(&w, 3), Some(PlayerId(1)));
    let before = balance(&w, 1);
    run(&mut w, 600);
    assert_eq!(balance(&w, 1) - before, 40);
    ship(&mut w, 2, pos);
    run(&mut w, 20);
    assert_eq!(claimant(&w, 3), Some(PlayerId(2)));
    let before = balance(&w, 1);
    run(&mut w, 600);
    assert_eq!(balance(&w, 1), before);
}

#[test]
fn travel_to_a_neighbour_takes_route_time() {
    let mut w = quick_world();
    let edge = w.system_edge(1, 2).unwrap();
    let a = ship(&mut w, 1, edge);
    travel(&mut w, a, 2).unwrap();
    w.step();
    let transit = w.transits().next().unwrap().clone();
    assert_eq!((transit.origin, transit.destination), (1, 2));
    assert_eq!(transit.arrives - transit.departed, 8); // ceil(22 * 1 / 3)
    assert_eq!(w.entity(a), None);
    assert_eq!(w.population_used(PlayerId(1)), 2);
    run(&mut w, 7);
    assert_eq!(w.transits().count(), 1);
    w.step();
    assert_eq!(w.transits().count(), 0);
    assert_eq!(w.tick(), transit.arrives);
    assert_eq!(w.entity(a).unwrap().pos, w.system_edge(2, 1).unwrap());
    assert!(w.entity(a).unwrap().orders().is_empty());
    assert_eq!(w.transits_done(), 1);
}

#[test]
fn travel_follows_the_shortest_route_path() {
    let mut w = quick_world();
    let pos = w.system_edge(1, 2).unwrap();
    let a = ship(&mut w, 1, pos);
    travel(&mut w, a, 3).unwrap();
    w.step();
    assert_eq!(w.transits().next().unwrap().destination, 2);
    let mut legs = vec![];
    let mut previous = None;
    for _ in 0..1600 {
        let leg = w.transits().next().map(|t| (t.origin, t.destination));
        if leg != previous
            && let Some(leg) = leg
        {
            legs.push(leg);
        }
        previous = leg;
        w.step();
        if w.entity(a).is_some_and(|e| e.pos.body == BodyId(20)) {
            break;
        }
    }
    assert_eq!(legs, vec![(1, 2), (2, 3)]);
    assert_eq!(w.entity(a).unwrap().pos.body, BodyId(20));
    assert_eq!(w.transits_done(), 2);
}

#[test]
fn travel_to_an_unconnected_system_is_rejected() {
    let mut w = world();
    let pos = w.body_position(BodyId(3)).unwrap();
    let a = ship(&mut w, 1, pos);
    let before = w.hash();
    assert_eq!(travel(&mut w, a, 4), Err(SimError::InvalidTravelTarget));
    assert_eq!(w.hash(), before);
    assert_eq!(travel(&mut w, a, 1), Err(SimError::InvalidTravelTarget));
}

#[test]
fn ships_in_transit_are_untargetable() {
    let mut w = world_with(
        &format!(
            "UPDATE et_weapon SET damage=9,range_max=10000,cooldown_ticks=1 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship'); {NO_JUMP_DELAYS}"
        ),
        false,
    );
    let pos = w.system_edge(1, 2).unwrap();
    let a = ship(&mut w, 1, pos);
    travel(&mut w, a, 2).unwrap();
    w.step();
    let b = ship(&mut w, 2, pos);
    assert_eq!(
        w.submit(Command {
            target_entity: Some(a),
            ..command(&w, b, "attack")
        }),
        Err(SimError::InvalidAttackTarget)
    );
    assert!(!w.entities_in_radius(pos, 10_000).contains(&a));
    assert!(
        !w.visible_entities(PlayerId(1))
            .iter()
            .any(|e| matches!(e,rom_sim::Sighting::Live(e) if e.id==a))
    );
    let hp = w.transits().next().unwrap().entity.hp;
    run(&mut w, 10);
    assert_eq!(w.entity(a).unwrap().hp, hp);
}

#[test]
fn space_vision_is_per_system() {
    let mut w = world();
    for system in [1, 2, 3, 4] {
        assert!(
            w.visibility_tiles(PlayerId(1), w.space_of(system).unwrap())
                .iter()
                .all(|v| *v == 0)
        );
    }
    let pos = w.body_position(BodyId(3)).unwrap();
    ship(&mut w, 1, pos);
    assert!(w.is_visible_on_body(PlayerId(1), pos.body, 0, pos.x / 1000, pos.y / 1000));
    assert!(
        w.visibility_tiles(PlayerId(1), BodyId(10))
            .iter()
            .all(|v| *v == 0)
    );
    assert!(
        w.visibility_tiles(PlayerId(2), BodyId(1))
            .iter()
            .all(|v| *v == 0)
    );
}

#[test]
fn spaceport_trains_and_launches_starships_into_planet_orbit() {
    let mut w = world();
    let p = PlayerId(1);
    let rules = w.rules().clone();
    let id = |k| rules.id_of(k).unwrap();
    for resource in rules.resources.keys() {
        w.grant(p, *resource, 100_000).unwrap();
    }
    w.place_for_setup(id("bld.city"), p, Pos::on(BodyId(2), 0, 20_500, 20_500))
        .unwrap();
    let builder = w
        .spawn(id("unit.citizen"), p, Pos::on(BodyId(2), 0, 30_000, 20_000))
        .unwrap();
    w.submit(Command {
        subject: Some(id("bld.spaceport")),
        target_pos: Some(Pos::on(BodyId(2), 0, 32_000, 20_000)),
        ..command(&w, builder, "construct")
    })
    .unwrap();
    run(&mut w, 1500);
    let port = w
        .entities()
        .find(|e| e.type_id == id("bld.spaceport") && e.build_progress.is_none())
        .expect("completed spaceport")
        .id;
    w.submit(Command {
        subject: Some(id("unit.survey_ship")),
        ..command(&w, port, "train")
    })
    .unwrap();
    run(&mut w, 550);
    let ship = w
        .entities()
        .find(|e| e.type_id == id("unit.survey_ship"))
        .expect("trained ship");
    assert_eq!(ship.pos.body, w.space_of(1).unwrap());
    let orbit = w.orbit(ship.id).expect("trained ship launched into orbit");
    assert_eq!(orbit.body, BodyId(2));
    assert!(
        w.distance(ship.pos, w.body_position(BodyId(2)).unwrap())
            .unwrap()
            <= 2000
    );
}

#[test]
fn route_duration_uses_configured_time_and_ship_speed() {
    let mut w = world_with(
        &format!(
            "UPDATE game_constant SET value=3 WHERE key='space.route_ticks_per_unit'; UPDATE et_movement SET speed=4400 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship'); {NO_JUMP_DELAYS}"
        ),
        false,
    );
    let pos = w.system_edge(1, 2).unwrap();
    let a = ship(&mut w, 1, pos);
    travel(&mut w, a, 2).unwrap();
    w.step();
    let transit = w.transits().next().unwrap();
    assert_eq!(transit.arrives - transit.departed, 15);
}

#[test]
fn zero_claim_radius_uses_orbit_reach_and_contest_clears_existing_income() {
    let mut w = world_with(
        "UPDATE et_node SET claim_radius=0 WHERE claim_mode='presence';",
        false,
    );
    let pos = w.body_position(BodyId(3)).unwrap();
    let near = w.offset(pos, 5000, 0);
    ship(&mut w, 1, near);
    run(&mut w, 20);
    assert_eq!(claimant(&w, 3), Some(PlayerId(1)));
    ship(
        &mut w,
        2,
        Pos {
            x: pos.x - 5000,
            ..pos
        },
    );
    run(&mut w, 20);
    assert_eq!(claimant(&w, 3), None);
    let metal = w.rules().id_of("res.metal").unwrap();
    assert_eq!(w.income(PlayerId(1), metal).raw, 0);
    let before = (balance(&w, 1), balance(&w, 2));
    run(&mut w, 600);
    assert_eq!((balance(&w, 1), balance(&w, 2)), before);
}

fn launch(w: &mut World, type_key: &str, owner: u32, pos: Pos) -> EntityId {
    let port = w
        .spawn(
            w.rules().id_of("bld.spaceport").unwrap(),
            PlayerId(owner),
            pos,
        )
        .unwrap();
    let id = w
        .spawn(
            w.rules().id_of(type_key).unwrap(),
            PlayerId(owner),
            w.offset(pos, 3000, 0),
        )
        .unwrap();
    w.submit(Command {
        target_entity: Some(port),
        ..command(w, id, "launch")
    })
    .unwrap();
    let duration = w.rules().constant("space.launch_ticks").unwrap() as u64;
    run(w, duration - 1);
    assert!(w.orbit(id).is_none());
    w.step();
    assert_eq!(w.orbit(id).unwrap().pos, pos);
    assert!(
        !w.entities_in_radius(pos, 10000).contains(&id),
        "launched ship leaves the ground spatial index"
    );
    assert!(
        w.entities_in_radius(w.body_position(pos.body).unwrap(), 1000)
            .contains(&id),
        "orbiting ship remains targetable in system space"
    );
    id
}

#[test]
fn launch_orbit_and_bombardment_repeat_with_equal_hashes() {
    let scenario = || {
        let mut w = world_at("", "age.starfaring");
        let pos = Pos::on(BodyId(2), 0, 32_000, 32_000);
        let id = launch(&mut w, "unit.corvette", 1, pos);
        let ground = w
            .spawn(
                w.rules().id_of("bld.library").unwrap(),
                PlayerId(2),
                w.offset(pos, 3000, 0),
            )
            .unwrap();
        let before = w.entity(ground).unwrap().hp;
        w.submit(Command {
            target_entity: Some(ground),
            ..command(&w, id, "attack")
        })
        .unwrap();
        run(&mut w, 65);
        assert!(w.entity(ground).unwrap().hp < before);
        // In range from the start, it holds over its launch point (task 68).
        let orbit = w.orbit(id).unwrap();
        assert_eq!(orbit.pos, pos);
        assert_eq!(orbit.body, BodyId(2));
        let shot = orbit.last_shot.expect("orbital weapon fired");
        assert_eq!(shot.target, w.entity(ground).unwrap().pos);
        assert!(w.is_visible_on_body(
            PlayerId(1),
            BodyId(2),
            orbit.pos.face,
            orbit.pos.x / 1000,
            orbit.pos.y / 1000
        ));
        let defender = w
            .spawn(
                w.rules().id_of("bld.tower").unwrap(),
                PlayerId(2),
                w.offset(pos, 5000, 0),
            )
            .unwrap();
        // A tower's weapon does not reach orbit (only weapons with an `orbit` layer row do).
        assert_eq!(
            w.submit(Command {
                target_entity: Some(id),
                ..command(&w, defender, "attack")
            }),
            Err(SimError::InvalidAttackTarget)
        );
        let hp = w.entity(id).unwrap().hp;
        run(&mut w, 40);
        assert_eq!(w.entity(id).unwrap().hp, hp);
        w.hash()
    };
    assert_eq!(scenario(), scenario());
}

#[test]
fn bombardment_requires_orbit_compatible_weapon_and_local_footprint() {
    let mut w = world_at("", "age.starfaring");
    let pos = Pos::on(BodyId(2), 0, 32_000, 32_000);
    let scout = launch(&mut w, "unit.survey_ship", 1, pos);
    let ground = w
        .spawn(
            w.rules().id_of("bld.library").unwrap(),
            PlayerId(2),
            w.offset(pos, 3000, 0),
        )
        .unwrap();
    let before = w.hash();
    assert_eq!(
        w.submit(Command {
            target_entity: Some(ground),
            ..command(&w, scout, "attack")
        }),
        Err(SimError::InvalidAttackTarget)
    );
    assert_eq!(w.hash(), before);
    let ship = launch(&mut w, "unit.corvette", 1, pos);
    let far = Pos::on(BodyId(2), 2, 32_000, 32_000);
    let target = w
        .spawn(w.rules().id_of("bld.library").unwrap(), PlayerId(2), far)
        .unwrap();
    w.spawn(
        w.rules().id_of("bld.library").unwrap(),
        PlayerId(1),
        w.offset(far, 6000, 0),
    )
    .unwrap();
    let hp = w.entity(target).unwrap().hp;
    w.submit(Command {
        target_entity: Some(target),
        ..command(&w, ship, "attack")
    })
    .unwrap();
    run(&mut w, 100);
    assert_eq!(
        w.entity(target).unwrap().hp,
        hp,
        "target beyond the orbital footprint"
    );
    assert!(w.orbit(ship).unwrap().last_shot.is_none());
    let system_pos = w.body_position(BodyId(2)).unwrap();
    w.submit(Command {
        target_pos: Some(w.offset(system_pos, 10000, 0)),
        ..command(&w, ship, "move")
    })
    .unwrap();
    w.step();
    assert!(w.orbit(ship).is_none());
    assert_eq!(
        w.submit(Command {
            target_entity: Some(ground),
            ..command(&w, ship, "attack")
        }),
        Err(SimError::OtherBody)
    );
}

#[test]
fn interplanetary_ships_can_orbit_but_cannot_depart_the_system() {
    let mut w = world_at("", "age.starfaring");
    let pos = w.body_position(BodyId(2)).unwrap();
    let id = ship(&mut w, 1, pos);
    let before = w.hash();
    assert_eq!(travel(&mut w, id, 2), Err(SimError::CannotEnterSpace));
    assert_eq!(w.hash(), before);
    let subpoint = Pos::on(BodyId(2), 1, 32000, 32000);
    w.submit(Command {
        target_pos: Some(subpoint),
        ..command(&w, id, "orbit")
    })
    .unwrap();
    w.step();
    assert_eq!(w.orbit(id).unwrap().pos, subpoint);
    // It holds over that point (task 68); ordered on, it crosses cube face edges in orbit.
    run(&mut w, 200);
    assert_eq!(w.orbit(id).unwrap().pos, subpoint);
    let elsewhere = Pos::on(BodyId(2), 2, 32000, 32000);
    w.submit(Command {
        target_pos: Some(elsewhere),
        ..command(&w, id, "move")
    })
    .unwrap();
    run(&mut w, 2000);
    let orbit = w.orbit(id).unwrap();
    assert_eq!(orbit.pos, elsewhere, "orbit crosses cube face edges");
    assert_eq!(w.entity(id).unwrap().pos.body, BodyId(1));
}

#[test]
fn ships_in_orbit_and_system_space_can_see_and_fight_each_other() {
    let mut w = world_at("", "age.starfaring");
    let pos = Pos::on(BodyId(2), 0, 32_000, 32_000);
    let orbiter = launch(&mut w, "unit.survey_ship", 1, pos);
    let system_pos = w.offset(w.body_position(BodyId(2)).unwrap(), 5000, 0);
    let enemy = ship(&mut w, 2, system_pos);
    let before = (w.entity(orbiter).unwrap().hp, w.entity(enemy).unwrap().hp);
    w.submit(Command {
        target_entity: Some(enemy),
        ..command(&w, orbiter, "attack")
    })
    .unwrap();
    w.submit(Command {
        target_entity: Some(orbiter),
        ..command(&w, enemy, "attack")
    })
    .unwrap();
    w.step();
    assert!(w.entity(orbiter).unwrap().hp < before.0);
    assert!(w.entity(enemy).unwrap().hp < before.1);
}

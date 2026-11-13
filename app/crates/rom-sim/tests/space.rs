//! System-space scenarios use the same movement, income, combat and fog as planets.
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SimError,
    SystemInfo, Topology, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

fn world_with(sql: &str, early: bool) -> World {
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
            age: id(if early {
                "age.ancient"
            } else {
                "age.starfaring"
            }),
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
        systems: [(1, 0, 0), (2, 100, 0), (3, 200, 0), (4, 0, 200)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
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
fn world() -> World {
    world_with("", false)
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
    let mut w = world();
    let edge = w.system_edge(1, 2).unwrap();
    let a = ship(&mut w, 1, edge);
    travel(&mut w, a, 2).unwrap();
    w.step();
    let transit = w.transits().next().unwrap().clone();
    assert_eq!((transit.origin, transit.destination), (1, 2));
    assert_eq!(transit.arrives - transit.departed, 10); // ceil(22 * 1 / 2.2)
    assert_eq!(w.entity(a), None);
    assert_eq!(w.population_used(PlayerId(1)), 2);
    run(&mut w, 9);
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
    let mut w = world();
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
        "INSERT INTO et_weapon(entity_type_id,slot,damage,damage_class_id,range_max,cooldown_ticks) SELECT d.id,0,9,c.id,10000,1 FROM def d, damage_class c WHERE d.key='unit.survey_ship' AND c.key='energy'; INSERT INTO et_weapon_target SELECT w.id,'space',100 FROM et_weapon w JOIN def d ON d.id=w.entity_type_id WHERE d.key='unit.survey_ship';",
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
fn spaceport_spawns_starships_in_system_space() {
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
    assert!(
        w.distance(ship.pos, w.body_position(BodyId(2)).unwrap())
            .unwrap()
            <= 2000
    );
}

#[test]
fn route_duration_uses_configured_time_and_ship_speed() {
    let mut w = world_with(
        "UPDATE game_constant SET value=3 WHERE key='space.route_ticks_per_unit'; UPDATE et_movement SET speed=4400 WHERE entity_type_id=(SELECT id FROM def WHERE key='unit.survey_ship');",
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

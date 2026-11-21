//! Alerts (task 118): a city under attack, foreign ships jumping into a watched system, and the
//! ages of the jump detector and the jump inhibitor. Every scenario runs twice and must end in
//! the same state hash.
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use rom_sim::{
    Alert, AlertKind, BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos,
    RouteInfo, SimError, SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
/// Player two's ally.
const P3: PlayerId = PlayerId(3);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// Two systems joined by a route; planet 2 (system 1) has a surface and is everyone's home.
/// Player one starts with `techs`; player three is player two's ally.
fn world(rules: &Arc<Rules>, age: &str, techs: &[&str]) -> World {
    let id = |k: &str| rules.id_of(k).unwrap();
    let players = (0..=3)
        .map(|p| Player {
            id: PlayerId(p),
            team: p.min(2),
            faction: id("nation.romans"),
            age: id(age),
            resources: [
                "res.food",
                "res.timber",
                "res.metal",
                "res.wealth",
                "res.energy",
            ]
            .into_iter()
            .map(|r| (id(r), 100_000))
            .collect(),
            techs: if p == 1 {
                techs.iter().map(|t| (id(t), 1)).collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let body = |n, system, kind, radius| BodyInfo {
        id: BodyId(n),
        system,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: 0,
        n: (kind == BodyKind::Habitable).then_some(64),
        home_race: None,
        node_type: None,
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: [(1, 0, 0), (2, 100, 0)]
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
            body(1, 1, BodyKind::Star, 0),
            body(2, 1, BodyKind::Habitable, 20),
            body(10, 2, BodyKind::Star, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
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
        rules.clone(),
        setup,
        7,
        players,
        BTreeMap::from([(P1, BodyId(2)), (P2, BodyId(2)), (P3, BodyId(2))]),
    )
    .unwrap()
}

fn id(w: &World, key: &str) -> rom_rules::DefId {
    w.rules().id_of(key).unwrap()
}

fn spawn(w: &mut World, key: &str, player: PlayerId, pos: Pos) -> EntityId {
    w.spawn(id(w, key), player, pos).unwrap()
}

fn place(w: &mut World, key: &str, player: PlayerId, pos: Pos) -> EntityId {
    w.place_for_setup(id(w, key), player, pos).unwrap()
}

fn order(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    target_entity: Option<EntityId>,
    target_pos: Option<Pos>,
) -> Result<rom_sim::OrderId, SimError> {
    w.submit(Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity,
        target_pos,
        queue: false,
    })
}

/// A point of planet 2's face 0, in tiles.
fn ground(x: i32, y: i32) -> Pos {
    Pos::on(BodyId(2), 0, x * 1000 + 500, y * 1000 + 500)
}

/// Steps `ticks` times and returns every alert raised meanwhile, oldest first (an alert carries
/// the tick count at the end of the step that raised it).
fn watch(w: &mut World, ticks: u64) -> Vec<Alert> {
    let mut seen = Vec::new();
    for _ in 0..ticks {
        let before = w.tick();
        w.step();
        seen.extend(
            w.alerts()
                .iter()
                .filter(|a| a.tick > before)
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    seen
}

/// Runs `scenario` twice and checks that both runs end in the same state.
fn twice<T: PartialEq + std::fmt::Debug>(scenario: impl Fn() -> (World, T)) -> T {
    let (first, a) = scenario();
    let (second, b) = scenario();
    assert_eq!(first.hash(), second.hash(), "scenario is not deterministic");
    assert_eq!(a, b, "the alerts differ between runs");
    b
}

/// A company of marines of `player` 4 tiles east of the city that fires at `target`.
fn marines_at(w: &mut World, player: PlayerId, target: EntityId) -> EntityId {
    let marines = spawn(w, "unit.marine", player, ground(36, 32));
    let at = w.entity(target).unwrap().pos;
    order(w, marines, "attack_move", None, Some(at)).unwrap();
    marines
}

fn disband(w: &mut World, unit: EntityId) {
    order(w, unit, "disband", None, None).unwrap();
}

#[test]
fn an_attack_on_a_city_alerts_its_owner_once_per_window() {
    let rules = rules();
    let repeat = rules.constant("alert.attack_repeat_ticks").unwrap() as u64;
    assert_eq!(repeat, 600);
    let alerts = twice(|| {
        let mut w = world(&rules, "age.industrial", &[]);
        let city = place(&mut w, "bld.city", P1, ground(32, 32));
        // A first company hits the city, then is gone.
        let marines = marines_at(&mut w, P2, city);
        let mut alerts = watch(&mut w, 120);
        disband(&mut w, marines);
        alerts.extend(watch(&mut w, 2));
        let first = alerts.iter().find(|a| a.player == P1).unwrap().tick;
        // Another attack inside the window: no new alert.
        let marines = marines_at(&mut w, P2, city);
        alerts.extend(watch(&mut w, 200));
        disband(&mut w, marines);
        alerts.extend(watch(&mut w, 2));
        assert_eq!(
            alerts.iter().filter(|a| a.player == P1).count(),
            1,
            "{alerts:?}"
        );
        // After the window an attack raises another.
        let quiet = (first + repeat).saturating_sub(w.tick());
        alerts.extend(watch(&mut w, quiet));
        marines_at(&mut w, P2, city);
        alerts.extend(watch(&mut w, 120));
        (w, alerts)
    });
    // The city fires back, so player two hears of its marines being hit; its ally hears nothing.
    assert!(alerts.iter().all(|a| a.player != P3), "{alerts:?}");
    let alerts: Vec<_> = alerts.into_iter().filter(|a| a.player == P1).collect();
    assert_eq!(alerts.len(), 2, "{alerts:?}");
    for alert in &alerts {
        let AlertKind::Attack {
            city,
            field,
            attacker,
            ..
        } = alert.kind
        else {
            panic!("not an attack: {alert:?}");
        };
        assert!(city.is_some() && !field);
        assert_eq!(attacker, P2);
    }
    assert!(alerts[1].tick >= alerts[0].tick + 600);
}

#[test]
fn a_unit_hit_in_the_field_is_told_near_its_city() {
    let rules = rules();
    let alerts = twice(|| {
        let mut w = world(&rules, "age.industrial", &[]);
        let city = place(&mut w, "bld.city", P1, ground(10, 10));
        // Outside the city's borders (40 tiles), on the far side of the planet.
        let far = |x, y| Pos::on(BodyId(2), 5, x * 1000 + 500, y * 1000 + 500);
        spawn(&mut w, "unit.field_gun", P1, far(30, 30));
        let gun = spawn(&mut w, "unit.field_gun", P2, far(34, 30));
        order(&mut w, gun, "attack_move", None, Some(far(30, 30))).unwrap();
        let alerts = watch(&mut w, 120);
        (w, (city, alerts))
    });
    let (city, alerts) = alerts;
    let mine: Vec<_> = alerts.iter().filter(|a| a.player == P1).collect();
    assert_eq!(mine.len(), 1, "{alerts:?}");
    let AlertKind::Attack {
        city: near,
        field,
        attacker,
        ..
    } = mine[0].kind
    else {
        panic!("not an attack: {alerts:?}");
    };
    assert_eq!((near, field, attacker), (Some(city), true, P2));
    // Player one's gun fires back: player two hears of it, its ally does not.
    assert!(alerts.iter().all(|a| a.player != P3));
}

/// Sends `ship` from system 2 to the point `to` of system 1's plane.
fn jump_to(w: &mut World, ship: EntityId, to: Pos) {
    order(w, ship, "travel", None, Some(to)).unwrap();
}

/// A point of system 1's plane `dx`, `dy` tiles from planet 2.
fn by_planet(w: &World, dx: i32, dy: i32) -> Pos {
    let at = w.body_position(BodyId(2)).unwrap();
    Pos {
        x: at.x + dx * 1000,
        y: at.y + dy * 1000,
        ..at
    }
}

/// Runs until every ship in `ships` has arrived in system 1, then a few ticks more, and returns
/// the arrival alerts raised meanwhile.
fn arrivals(w: &mut World, ships: &[EntityId]) -> Vec<Alert> {
    let star = w.space_of(1).unwrap();
    let mut alerts = Vec::new();
    for _ in 0..2000 {
        alerts.extend(watch(w, 1));
        let here = |s: &EntityId| match w.entity(*s) {
            Some(e) => e.pos.body == star,
            None => !w.transits().any(|t| t.entity.id == *s),
        };
        if ships.iter().all(here) {
            break;
        }
    }
    alerts.extend(watch(w, 5));
    alerts.retain(|a| matches!(a.kind, AlertKind::Arrival { .. }));
    alerts
}

#[test]
fn a_detector_tells_its_owner_of_foreign_ships_jumping_in() {
    let rules = rules();
    let (alerts, ships) = twice(|| {
        let mut w = world(&rules, "age.abundance", &[]);
        place(&mut w, "bld.jump_detector", P2, ground(30, 30));
        // A second detector of the same player: no second alert.
        place(&mut w, "bld.jump_detector", P2, ground(40, 30));
        let from = w.system_edge(2, 1).unwrap();
        let foes = [
            spawn(&mut w, "unit.corvette", P1, from),
            spawn(&mut w, "unit.corvette", P1, from),
            spawn(&mut w, "unit.corvette", P1, from),
        ];
        let friend = spawn(&mut w, "unit.corvette", P3, from);
        let own = spawn(&mut w, "unit.corvette", P2, from);
        // Far from the planet: no inhibitor stands, and a system detector sees the whole system.
        let to = by_planet(&w, 60, 0);
        for ship in foes.into_iter().chain([friend, own]) {
            jump_to(&mut w, ship, to);
        }
        let alerts = arrivals(&mut w, &[foes[0], foes[1], foes[2], friend, own]);
        (w, (alerts, foes))
    });
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    assert_eq!(alerts[0].player, P2);
    assert_eq!(
        alerts[0].kind,
        AlertKind::Arrival {
            system: 1,
            owner: P1,
            ships: ships.to_vec(),
        }
    );
}

#[test]
fn without_a_detector_an_arrival_raises_no_alert() {
    let rules = rules();
    let alerts = twice(|| {
        let mut w = world(&rules, "age.abundance", &[]);
        let from = w.system_edge(2, 1).unwrap();
        let ship = spawn(&mut w, "unit.corvette", P1, from);
        let to = by_planet(&w, 60, 0);
        jump_to(&mut w, ship, to);
        let alerts = arrivals(&mut w, &[ship]);
        (w, alerts)
    });
    assert!(alerts.is_empty(), "{alerts:?}");
}

/// Player one's survey ship stands in system 1 at `post`; with `techs` it may have the sensor.
/// Player two's corvettes jump in beside planet 2 and 50 tiles inward of it.
fn survey_watch(rules: &Arc<Rules>, techs: &[&str], post: (i32, i32)) -> Vec<Alert> {
    twice(|| {
        let mut w = world(rules, "age.abundance", techs);
        let at = by_planet(&w, post.0, post.1);
        spawn(&mut w, "unit.survey_ship", P1, at);
        let from = w.system_edge(2, 1).unwrap();
        let near = spawn(&mut w, "unit.corvette", P2, from);
        let far = spawn(&mut w, "unit.corvette", P2, from);
        let (a, b) = (by_planet(&w, 0, 0), by_planet(&w, -50, 0));
        jump_to(&mut w, near, a);
        jump_to(&mut w, far, b);
        let alerts = arrivals(&mut w, &[near, far]);
        (w, alerts)
    })
}

#[test]
fn a_survey_ship_with_the_sensor_detects_arrivals_within_its_radius_only() {
    let rules = rules();
    let radius = rules.entity_types[&rules.id_of("unit.survey_ship").unwrap()]
        .detector
        .as_ref()
        .unwrap()
        .radius;
    assert_eq!(radius, 40_000);
    // Without the research the sensor is off.
    assert!(survey_watch(&rules, &[], (5, 0)).is_empty());
    // With it, 5 tiles out from planet 2: the ship arriving beside the planet is seen, the one
    // 50 tiles inward (55 from the survey ship) is not.
    let alerts = survey_watch(&rules, &["tech.survey_sensor_array"], (5, 0));
    assert_eq!(alerts.len(), 1, "{alerts:?}");
    let AlertKind::Arrival {
        system,
        owner,
        ref ships,
    } = alerts[0].kind
    else {
        unreachable!()
    };
    assert_eq!(
        (alerts[0].player, system, owner, ships.len()),
        (P1, 1, P2, 1)
    );
    assert_eq!(alerts[0].pos, {
        let w = world(&rules, "age.abundance", &[]);
        by_planet(&w, 0, 0)
    });
}

/// Whether player one may order a citizen to put up `building` in the age `age`.
fn may_build(rules: &Arc<Rules>, age: &str, building: &str) -> Result<(), SimError> {
    let mut w = world(rules, age, &[]);
    place(&mut w, "bld.city", P1, ground(32, 32));
    let citizen = spawn(&mut w, "unit.citizen", P1, ground(32, 38));
    w.submit(Command {
        player: P1,
        executor: citizen,
        kind: "construct".into(),
        subject: Some(id(&w, building)),
        target_entity: None,
        target_pos: Some(ground(38, 38)),
        queue: false,
    })
    .map(|_| ())
}

#[test]
fn the_detector_comes_in_the_fusion_age_and_the_inhibitor_in_the_expansion_age() {
    let rules = rules();
    let first = |key: &str| {
        let et = &rules.entity_types[&rules.id_of(key).unwrap()];
        rules.def(et.first_age_id.unwrap()).key.clone()
    };
    assert_eq!(first("bld.jump_detector"), "age.fusion");
    assert_eq!(first("bld.jump_inhibitor"), "age.expansion");
    assert_eq!(
        may_build(&rules, "age.automation", "bld.jump_detector"),
        Err(SimError::Locked)
    );
    assert_eq!(may_build(&rules, "age.fusion", "bld.jump_detector"), Ok(()));
    assert_eq!(
        may_build(&rules, "age.starfaring", "bld.jump_inhibitor"),
        Err(SimError::Locked)
    );
    assert_eq!(
        may_build(&rules, "age.expansion", "bld.jump_inhibitor"),
        Ok(())
    );
}

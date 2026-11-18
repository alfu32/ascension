//! Charted routes, charged jumps, jump inhibitors and interstellar trade (task 70). Every
//! scenario runs twice and must end in the same state hash.
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, OrderStatus, PlanetSurface, Player, PlayerId, Pos,
    RouteInfo, SimError, SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// Four systems on a line and a spur: 1 – 2 – 3, a long route 1 – 3 and a route 1 – 4. Planet 2
/// (system 1) and planet 11 (system 2) have surfaces. Player one starts with `techs`.
fn world(rules: &Arc<Rules>, age: &str, techs: &[&str]) -> World {
    let id = |k: &str| rules.id_of(k).unwrap();
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: id("nation.romans"),
            age: id(age),
            resources: BTreeMap::new(),
            techs: if p == 1 {
                techs.iter().map(|t| (id(t), 1)).collect()
            } else {
                BTreeMap::new()
            },
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
    let planet = || PlanetSurface {
        surface: CubeSurface { n: 64 },
        terrain: vec![id("ter.plain"); 6 * 64 * 64],
        nodes: vec![],
    };
    let route = |system_a, system_b, length| RouteInfo {
        system_a,
        system_b,
        length,
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
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
            body(20, 3, BodyKind::Star, 0, 0),
            body(30, 4, BodyKind::Star, 0, 0),
        ],
        routes: vec![
            route(1, 2, 22),
            route(2, 3, 22),
            route(1, 3, 100),
            route(1, 4, 30),
        ],
        planets: BTreeMap::from([(BodyId(2), planet()), (BodyId(11), planet())]),
    };
    World::from_universe(
        rules.clone(),
        setup,
        7,
        players,
        BTreeMap::from([(P1, BodyId(2)), (P2, BodyId(2))]),
    )
    .unwrap()
}

fn spawn(w: &mut World, key: &str, player: PlayerId, pos: Pos) -> EntityId {
    w.spawn(w.rules().id_of(key).unwrap(), player, pos).unwrap()
}

fn star(w: &World, system: u32) -> Pos {
    w.body_position(w.space_of(system).unwrap()).unwrap()
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

fn travel(w: &mut World, ship: EntityId, system: u32) -> Result<rom_sim::OrderId, SimError> {
    let to = star(w, system);
    order(w, ship, "travel", None, Some(to))
}

fn run(w: &mut World, ticks: u64) {
    for _ in 0..ticks {
        w.step();
    }
}

fn run_until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "condition not met within {limit} ticks");
}

fn in_transit(w: &World, ship: EntityId) -> bool {
    w.transits().any(|t| t.entity.id == ship)
}

/// The transit time of a leg in ticks.
fn leg_ticks(w: &World, ship: EntityId) -> u64 {
    let t = w.transits().find(|t| t.entity.id == ship).unwrap();
    t.arrives - t.departed
}

fn charge_of(w: &World, key: &str) -> u64 {
    let rules = w.rules();
    let ship = rules.id_of(key).unwrap();
    rules.entity_types[&ship]
        .movement
        .as_ref()
        .unwrap()
        .jump_charge_ticks
        .unwrap() as u64
}

/// Runs `scenario` twice and checks that both runs end in the same state.
fn twice(scenario: impl Fn() -> World) -> World {
    let first = scenario();
    let second = scenario();
    assert_eq!(first.hash(), second.hash(), "scenario is not deterministic");
    second
}

#[test]
fn a_first_voyage_charts_the_route_slowly_and_the_next_is_quick() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let ship = {
            let at = w.system_edge(1, 2).unwrap();
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        assert!(!w.knows_route(P1, 1, 2) && w.known_routes(P1).is_empty());
        assert!(w.charting(P1, 1, 2));
        travel(&mut w, ship, 2).unwrap();
        run_until(&mut w, 400, |w| in_transit(w, ship));
        // ceil(22 x 1 tick x 1000 / 3000 milli-tiles per second x 500 %) against ceil(22 / 3).
        assert_eq!(leg_ticks(&w, ship), 37);
        run_until(&mut w, 100, |w| w.entity(ship).is_some());
        assert!(w.knows_route(P1, 2, 1));
        assert_eq!(w.known_routes(P1), vec![(1, 2)]);
        assert_eq!(w.arrived_from(ship), Some(1));
        travel(&mut w, ship, 1).unwrap();
        run_until(&mut w, 400, |w| in_transit(w, ship));
        assert_eq!(leg_ticks(&w, ship), 8);
        run_until(&mut w, 100, |w| w.entity(ship).is_some());
        w
    });
    assert_eq!(w.known_routes(P1), vec![(1, 2)]);
    assert!(w.known_routes(P2).is_empty());
}

#[test]
fn a_route_is_charted_only_on_arrival() {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[]);
    let ship = {
        let at = w.system_edge(1, 2).unwrap();
        spawn(&mut w, "unit.survey_ship", P1, at)
    };
    travel(&mut w, ship, 2).unwrap();
    run_until(&mut w, 400, |w| in_transit(w, ship));
    // On the way it has charted nothing: a ship lost now would leave the route unknown.
    assert!(!w.knows_route(P1, 1, 2));
    run_until(&mut w, 100, |w| w.entity(ship).is_some());
    assert!(w.knows_route(P1, 1, 2));
}

#[test]
fn enhanced_sensors_reveal_routes_ahead_and_remove_the_delay() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &["tech.enhanced_sensors"]);
        let ship = {
            let at = w.system_edge(1, 2).unwrap();
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        run(&mut w, 21);
        // Every route leaving the system where it has a ship or a planet, nothing beyond.
        assert_eq!(w.known_routes(P1), vec![(1, 2), (1, 3), (1, 4)]);
        assert!(!w.knows_route(P1, 2, 3));
        travel(&mut w, ship, 2).unwrap();
        run_until(&mut w, 400, |w| in_transit(w, ship));
        assert_eq!(leg_ticks(&w, ship), 8, "no charting delay");
        run_until(&mut w, 100, |w| w.entity(ship).is_some());
        run(&mut w, 21);
        // In view now: the routes leaving system 2.
        assert!(w.knows_route(P1, 2, 3));
        w
    });
    assert_eq!(w.known_routes(P1), vec![(1, 2), (1, 3), (1, 4), (2, 3)]);
    assert!(w.known_routes(P2).is_empty());
}

#[test]
fn the_route_survey_reveals_every_route() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.galactic", &["tech.automated_route_survey"]);
        run(&mut w, 21);
        w
    });
    assert_eq!(w.known_routes(P1), vec![(1, 2), (1, 3), (1, 4), (2, 3)]);
    assert!(w.knows_route(P1, 3, 2));
    assert!(!w.charting(P1, 2, 3));
    assert!(w.known_routes(P2).is_empty());
}

#[test]
fn a_single_ship_charges_at_the_edge_then_departs() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let edge = w.system_edge(1, 2).unwrap();
        let ship = spawn(&mut w, "unit.survey_ship", P1, edge);
        let charge = charge_of(&w, "unit.survey_ship");
        travel(&mut w, ship, 2).unwrap();
        w.step();
        let start = w.tick();
        let first = w.jump_charge(ship).expect("charging");
        assert_eq!((first.required, first.toward), (charge, 2));
        let front = &w.entity(ship).unwrap().orders()[0];
        assert_eq!(front.status, OrderStatus::Blocked("charging".into()));
        run_until(&mut w, 400, |w| in_transit(w, ship));
        // It held still while it charged and left once the charge was full.
        assert_eq!(w.tick() - start, charge);
        let transit = w.transits().find(|t| t.entity.id == ship).unwrap();
        assert_eq!(transit.entity.pos, edge);
        assert!(transit.entity.orders()[0].total_ticks.is_none());
        w
    });
    assert_eq!(w.transits().count(), 1);
}

#[test]
fn a_ship_charging_holds_still_and_holds_fire() {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[]);
    let edge = w.system_edge(1, 2).unwrap();
    let ship = spawn(&mut w, "unit.corvette", P1, edge);
    travel(&mut w, ship, 2).unwrap();
    run(&mut w, 2);
    assert!(w.jump_charge(ship).is_some());
    // An enemy beside it is not fired upon, and the charge goes on while it is hit.
    let enemy = {
        let at = w.offset(edge, -3000, 0);
        spawn(&mut w, "unit.corvette", P2, at)
    };
    order(&mut w, enemy, "attack", Some(ship), None).unwrap();
    run(&mut w, 60);
    let (own, theirs) = (w.entity(ship).unwrap(), w.entity(enemy).unwrap());
    assert!(own.hp < own.max_hp, "the charging ship can be hit");
    assert_eq!(theirs.hp, theirs.max_hp, "the charging ship does not fire");
    assert_eq!(own.pos, edge);
    assert!(w.jump_charge(ship).unwrap().progress >= 60);
}

#[test]
fn a_new_order_cancels_the_charge() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let edge = w.system_edge(1, 2).unwrap();
        let ship = spawn(&mut w, "unit.survey_ship", P1, edge);
        travel(&mut w, ship, 2).unwrap();
        run(&mut w, 60);
        assert!(w.jump_charge(ship).unwrap().progress > 50);
        let away = w.offset(edge, -20_000, 0);
        order(&mut w, ship, "move", None, Some(away)).unwrap();
        run(&mut w, 200);
        assert!(w.jump_charge(ship).is_none());
        assert!(!in_transit(&w, ship));
        // Ordered again, it charges from the start.
        travel(&mut w, ship, 2).unwrap();
        run_until(&mut w, 400, |w| w.jump_charge(ship).is_some());
        assert_eq!(w.jump_charge(ship).unwrap().progress, 0);
        w
    });
    assert_eq!(w.transits().count(), 0);
}

#[test]
fn a_group_departs_together_once_its_slowest_ship_has_charged() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let edge = w.system_edge(1, 2).unwrap();
        let quick = spawn(&mut w, "unit.survey_ship", P1, edge);
        let slow = {
            let at = w.offset(edge, 0, 2000);
            spawn(&mut w, "unit.battleship", P1, at)
        };
        let (fast_charge, slow_charge) = (
            charge_of(&w, "unit.survey_ship"),
            charge_of(&w, "unit.battleship"),
        );
        assert!(fast_charge < slow_charge);
        travel(&mut w, quick, 2).unwrap();
        travel(&mut w, slow, 2).unwrap();
        let start = w.tick();
        run_until(&mut w, 1000, |w| in_transit(w, quick));
        // The quick one waited, charged, for the slow one: both left on the same tick.
        assert!(in_transit(&w, slow));
        assert!(w.tick() - start >= slow_charge);
        let departed: Vec<u64> = w.transits().map(|t| t.departed).collect();
        assert_eq!(departed[0], departed[1]);
        w
    });
    assert_eq!(w.transits().count(), 2);
}

#[test]
fn an_enemy_inhibitor_holds_ships_but_lets_them_retreat() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let inhibitor = w
            .place_for_setup(
                rules.id_of("bld.jump_inhibitor").unwrap(),
                P2,
                Pos::on(BodyId(2), 0, 30_500, 30_500),
            )
            .unwrap();
        // A ship of player one comes into system 1 from system 2.
        let ship = {
            let at = w.system_edge(2, 1).unwrap();
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        travel(&mut w, ship, 1).unwrap();
        run_until(&mut w, 1000, |w| {
            w.entity(ship).is_some_and(|e| e.pos.body == BodyId(1))
        });
        let held = w.inhibition(P1, 1).expect("held");
        assert_eq!(
            (held.inhibitor, held.owner, held.charge_pct),
            (inhibitor, P2, 50)
        );
        assert!(w.inhibition(P2, 1).is_none(), "own ships are free");
        assert!(w.inhibition(P1, 2).is_none(), "only in its own system");
        // Another route is refused with a reason; the way back is open, charging longer.
        let before = w.hash();
        assert_eq!(travel(&mut w, ship, 4), Err(SimError::HeldByInhibitor));
        assert_eq!(w.hash(), before);
        travel(&mut w, ship, 2).unwrap();
        run_until(&mut w, 2000, |w| w.jump_charge(ship).is_some());
        let charge = charge_of(&w, "unit.survey_ship");
        assert_eq!(w.jump_charge(ship).unwrap().required, charge * 3 / 2);
        run_until(&mut w, 2000, |w| in_transit(w, ship));
        // A ship of the inhibitor's owner leaves along any route.
        let own = {
            let at = w.system_edge(1, 4).unwrap();
            spawn(&mut w, "unit.survey_ship", P2, at)
        };
        travel(&mut w, own, 4).unwrap();
        run_until(&mut w, 1000, |w| in_transit(w, own));
        w
    });
    assert!(w.knows_route(P1, 1, 2));
}

#[test]
fn a_queued_route_under_an_inhibitor_fails_when_it_comes_up() {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[]);
    w.place_for_setup(
        rules.id_of("bld.jump_inhibitor").unwrap(),
        P2,
        Pos::on(BodyId(2), 0, 30_500, 30_500),
    )
    .unwrap();
    // Built in the held system, it arrived by no route: every route is closed to it.
    let ship = {
        let at = w.system_edge(1, 2).unwrap();
        spawn(&mut w, "unit.survey_ship", P1, at)
    };
    assert_eq!(travel(&mut w, ship, 2), Err(SimError::HeldByInhibitor));
    let to = star(&w, 2);
    w.submit(Command {
        player: P1,
        executor: ship,
        kind: "travel".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(to),
        queue: true,
    })
    .unwrap();
    run(&mut w, 3);
    assert!(w.entity(ship).unwrap().orders().is_empty());
    assert!(
        w.order_failures(P1)
            .any(|f| f.kind == "travel" && f.reason == "inhibited")
    );
}

#[test]
fn destroying_the_inhibitor_mid_charge_lifts_it_at_once() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let inhibitor = w
            .place_for_setup(
                rules.id_of("bld.jump_inhibitor").unwrap(),
                P2,
                Pos::on(BodyId(2), 0, 30_500, 30_500),
            )
            .unwrap();
        let ship = {
            let at = w.system_edge(2, 1).unwrap();
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        travel(&mut w, ship, 1).unwrap();
        run_until(&mut w, 1000, |w| {
            w.entity(ship).is_some_and(|e| e.pos.body == BodyId(1))
        });
        travel(&mut w, ship, 2).unwrap();
        run_until(&mut w, 2000, |w| {
            w.jump_charge(ship).is_some_and(|c| c.progress >= 30)
        });
        let charge = charge_of(&w, "unit.survey_ship");
        assert_eq!(w.jump_charge(ship).unwrap().required, charge * 3 / 2);
        w.despawn(inhibitor).unwrap();
        w.step();
        // The charge needs its usual time again, and every route is open.
        assert_eq!(w.jump_charge(ship).unwrap().required, charge);
        assert!(w.inhibition(P1, 1).is_none());
        let other = {
            let at = w.system_edge(1, 4).unwrap();
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        travel(&mut w, other, 4).unwrap();
        run_until(&mut w, 2000, |w| in_transit(w, ship));
        w
    });
    assert!(w.transits().count() >= 1);
}

/// A freighter of player one in orbit of planet 2 (system 1), with a city there and one on
/// planet 11 (system 2).
fn trade_world(rules: &Arc<Rules>) -> (World, EntityId, EntityId, EntityId) {
    let mut w = world(rules, "age.abundance", &[]);
    let id = |k: &str| rules.id_of(k).unwrap();
    for resource in rules.resources.keys() {
        w.grant(P1, *resource, 100_000).unwrap();
    }
    let home = w
        .place_for_setup(id("bld.city"), P1, Pos::on(BodyId(2), 0, 20_500, 20_500))
        .unwrap();
    let far = w
        .place_for_setup(id("bld.city"), P1, Pos::on(BodyId(11), 0, 20_500, 20_500))
        .unwrap();
    let at = w.body_position(BodyId(2)).unwrap();
    let freighter = spawn(&mut w, "unit.space_freighter", P1, at);
    let marker = w.body_marker(BodyId(2));
    order(&mut w, freighter, "orbit", marker, None).unwrap();
    run_until(&mut w, 400, |w| w.orbit(freighter).is_some());
    (w, freighter, home, far)
}

fn wealth(w: &World) -> i64 {
    let id = w.rules().id_of("res.wealth").unwrap();
    w.players()
        .find(|p| p.id == P1)
        .unwrap()
        .resources
        .get(&id)
        .copied()
        .unwrap_or(0)
}

#[test]
fn a_freighter_trades_between_star_systems_after_the_research() {
    let rules = rules();
    let w = twice(|| {
        let (mut w, freighter, home, far) = trade_world(&rules);
        // Before the research: refused, and told why.
        let before = w.hash();
        assert_eq!(
            order(&mut w, freighter, "trade", Some(far), None),
            Err(SimError::NeedsInterstellarTrade)
        );
        assert_eq!(w.hash(), before);
        // The research, at a spaceport.
        let port = w
            .place_for_setup(
                rules.id_of("bld.spaceport").unwrap(),
                P1,
                Pos::on(BodyId(2), 0, 34_000, 20_000),
            )
            .unwrap();
        w.submit(Command {
            player: P1,
            executor: port,
            kind: "research".into(),
            subject: Some(rules.id_of("tech.interstellar_trade").unwrap()),
            target_entity: None,
            target_pos: None,
            queue: false,
        })
        .unwrap();
        run_until(&mut w, 3000, |w| {
            w.rule_enabled(P1, rom_sim::RULE_INTERSTELLAR_TRADE)
        });
        order(&mut w, freighter, "trade", Some(far), None).unwrap();
        w.step();
        let route = w.trade_routes(P1).next().unwrap();
        assert_eq!((route.home, route.target), (home, far));
        // The path's length: the two planes and the route counted at its rate.
        let per_unit = rules.constant("trade.route_length_mtiles").unwrap() as u64;
        assert!(route.length > 22 * per_unit);
        // Out: lift, cross the system, charge, jump (charting), cross, land, deliver; then back.
        let start = wealth(&w);
        run_until(&mut w, 40_000, |w| w.stats().trade_trips >= 2);
        assert!(w.knows_route(P1, 1, 2), "the first trip charted the route");
        assert_eq!(wealth(&w) - start, 2 * route.per_trip_income);
        w
    });
    assert_eq!(w.stats().trade_trips, 2);
}

#[test]
fn an_inhibitor_holds_a_freighter_like_any_ship() {
    let rules = rules();
    let (mut w, freighter, _, far) = trade_world(&rules);
    let tech = rules.id_of("tech.interstellar_trade").unwrap();
    let port = w
        .place_for_setup(
            rules.id_of("bld.spaceport").unwrap(),
            P1,
            Pos::on(BodyId(2), 0, 34_000, 20_000),
        )
        .unwrap();
    w.submit(Command {
        player: P1,
        executor: port,
        kind: "research".into(),
        subject: Some(tech),
        target_entity: None,
        target_pos: None,
        queue: false,
    })
    .unwrap();
    run_until(&mut w, 3000, |w| {
        w.rule_enabled(P1, rom_sim::RULE_INTERSTELLAR_TRADE)
    });
    w.place_for_setup(
        rules.id_of("bld.jump_inhibitor").unwrap(),
        P2,
        Pos::on(BodyId(2), 0, 40_500, 40_500),
    )
    .unwrap();
    order(&mut w, freighter, "trade", Some(far), None).unwrap();
    // It lands at its home city, lifts off for the other system and is held.
    run(&mut w, 600);
    assert_eq!(w.transits().count(), 0);
    assert!(
        w.order_failures(P1)
            .any(|f| f.kind == "trade" && f.reason == "inhibited")
    );
}

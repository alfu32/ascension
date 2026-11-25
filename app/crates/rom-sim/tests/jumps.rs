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
/// Player two's ally (task 104).
const P3: PlayerId = PlayerId(3);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// Four systems on a line and a spur: 1 – 2 – 3, a long route 1 – 3 and a route 1 – 4. Planet 2
/// (system 1) and planet 11 (system 2) have surfaces. Player one starts with `techs`; player
/// three is player two's ally.
fn world(rules: &Arc<Rules>, age: &str, techs: &[&str]) -> World {
    let id = |k: &str| rules.id_of(k).unwrap();
    let players = (0..=3)
        .map(|p| Player {
            id: PlayerId(p),
            team: p.min(2),
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
        BTreeMap::from([(P1, BodyId(2)), (P2, BodyId(2)), (P3, BodyId(2))]),
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
        text: None,
        facing: None,
    })
}

fn travel(w: &mut World, ship: EntityId, system: u32) -> Result<rom_sim::OrderId, SimError> {
    let to = star(w, system);
    order(w, ship, "travel", None, Some(to))
}

/// Sends `ship` to the point `to` of another system's plane (task 111).
fn travel_to(w: &mut World, ship: EntityId, to: Pos) -> Result<rom_sim::OrderId, SimError> {
    order(w, ship, "travel", None, Some(to))
}

/// Where planet 2 (system 1, where the inhibitors stand) lies in its system's plane.
fn planet_2(w: &World) -> Pos {
    w.body_position(BodyId(2)).unwrap()
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
        // A ship of player one comes into system 1 from system 2 (a cruiser: it survives the
        // inhibitor's blow on arrival, task 104).
        let ship = {
            let at = w.system_edge(2, 1).unwrap();
            spawn(&mut w, "unit.space_cruiser", P1, at)
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
        let charge = charge_of(&w, "unit.space_cruiser");
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
        text: None,
        facing: None,
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
            spawn(&mut w, "unit.space_cruiser", P1, at)
        };
        travel(&mut w, ship, 1).unwrap();
        run_until(&mut w, 1000, |w| {
            w.entity(ship).is_some_and(|e| e.pos.body == BodyId(1))
        });
        travel(&mut w, ship, 2).unwrap();
        run_until(&mut w, 2000, |w| {
            w.jump_charge(ship).is_some_and(|c| c.progress >= 30)
        });
        let charge = charge_of(&w, "unit.space_cruiser");
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

/// The fixed blow of a jump inhibitor's type (`et_inhibitor.arrival_damage`).
fn arrival_damage(rules: &Rules) -> i64 {
    rules.entity_types[&rules.id_of("bld.jump_inhibitor").unwrap()]
        .inhibitor
        .as_ref()
        .unwrap()
        .arrival_damage
}

#[test]
fn an_inhibitor_strikes_enemy_ships_jumping_into_its_system() {
    let rules = rules();
    let damage = arrival_damage(&rules);
    assert!(damage > 0);
    let hp = |key: &str| rules.entity_types[&rules.id_of(key).unwrap()].hp;
    // The blow destroys light hulls and leaves line ships hurt but alive.
    assert!(hp("unit.corvette") <= damage && hp("unit.battleship") > damage);
    let w = twice(|| {
        let mut w = world(&rules, "age.abundance", &[]);
        let inhibitor = w
            .place_for_setup(
                rules.id_of("bld.jump_inhibitor").unwrap(),
                P2,
                Pos::on(BodyId(2), 0, 30_500, 30_500),
            )
            .unwrap();
        // Already in the held system when the inhibitor stood: never struck.
        let resident = {
            let at = w.system_edge(1, 4).unwrap();
            spawn(&mut w, "unit.corvette", P1, at)
        };
        let from_2 = w.system_edge(2, 1).unwrap();
        // Orbital fighters jump only aboard a carrier: the corvette is the lightest warship.
        let corvette = spawn(&mut w, "unit.corvette", P1, from_2);
        let battleship = spawn(&mut w, "unit.battleship", P1, from_2);
        let allied = spawn(&mut w, "unit.corvette", P3, from_2);
        let own = spawn(&mut w, "unit.corvette", P2, from_2);
        // They jump in beside the inhibitor's planet, within its radius (task 111).
        let beside = planet_2(&w);
        for ship in [corvette, battleship, allied, own] {
            travel_to(&mut w, ship, beside).unwrap();
        }
        let full = w.entity(battleship).unwrap().max_hp;
        run_until(&mut w, 2000, |w| {
            w.entity(battleship)
                .is_some_and(|e| e.pos.body == BodyId(1))
        });
        let tick = w.tick();
        let mut struck_at = Vec::new();
        // The light hull died on arrival, credited to the inhibitor's owner.
        assert!(w.entity(corvette).is_none() && !in_transit(&w, corvette));
        assert_eq!(w.stats().kills.get(&P2), Some(&1));
        assert_eq!(w.stats().losses.get(&P1), Some(&1));
        // The battleship arrived with its hit points less the blow.
        assert_eq!(w.entity(battleship).unwrap().hp, full - damage);
        // Allied and own ships, and the one already there, are untouched.
        for ship in [allied, own, resident] {
            let e = w.entity(ship).unwrap();
            assert_eq!(e.hp, e.max_hp, "{ship:?} was struck");
        }
        // The views are told: one strike each, at its arrival tick (the corvette is faster).
        let strikes: Vec<_> = w
            .arrival_strikes()
            .iter()
            .map(|s| {
                struck_at.push(s.tick);
                (
                    s.ship,
                    s.inhibitor,
                    s.owner,
                    s.damage,
                    s.destroyed,
                    s.system,
                )
            })
            .collect();
        assert_eq!(
            strikes,
            [
                (corvette, inhibitor, P2, damage, true, 1),
                (battleship, inhibitor, P2, damage, false, 1),
            ]
        );
        assert!(struck_at[0] <= tick && struck_at[1] == tick);
        // Struck once: staying in the system brings no further blow (the ships here fight
        // each other, so their hit points are no measure).
        run(&mut w, 200);
        assert_eq!(w.arrival_strikes().len(), 2);
        w
    });
    assert!(w.arrival_strikes().iter().all(|s| s.owner == P2));
}

#[test]
fn several_inhibitors_do_not_stack_and_a_destroyed_one_strikes_no_more() {
    let rules = rules();
    let damage = arrival_damage(&rules);
    let w = twice(|| {
        let mut w = world(&rules, "age.abundance", &[]);
        let kind = rules.id_of("bld.jump_inhibitor").unwrap();
        let first = w
            .place_for_setup(kind, P2, Pos::on(BodyId(2), 0, 30_500, 30_500))
            .unwrap();
        let second = w
            .place_for_setup(kind, P2, Pos::on(BodyId(2), 0, 40_500, 40_500))
            .unwrap();
        let beside = planet_2(&w);
        assert_eq!(
            w.arrival_blow(P1, beside).map(|b| (b.inhibitor, b.damage)),
            Some((first.min(second), damage))
        );
        let elsewhere = w.body_position(BodyId(11)).unwrap();
        assert!(w.arrival_blow(P3, beside).is_none() && w.arrival_blow(P1, elsewhere).is_none());
        let from_2 = w.system_edge(2, 1).unwrap();
        let cruiser = spawn(&mut w, "unit.space_cruiser", P1, from_2);
        let full = w.entity(cruiser).unwrap().max_hp;
        travel_to(&mut w, cruiser, beside).unwrap();
        run_until(&mut w, 2000, |w| {
            w.entity(cruiser).is_some_and(|e| e.pos.body == BodyId(1))
        });
        assert_eq!(w.entity(cruiser).unwrap().hp, full - damage);
        // With both gone, the next arrival is not struck.
        w.despawn(first).unwrap();
        w.despawn(second).unwrap();
        let later = spawn(&mut w, "unit.corvette", P1, from_2);
        travel_to(&mut w, later, beside).unwrap();
        run_until(&mut w, 2000, |w| {
            w.entity(later).is_some_and(|e| e.pos.body == BodyId(1))
        });
        let e = w.entity(later).unwrap();
        assert_eq!(e.hp, e.max_hp);
        w
    });
    assert_eq!(w.arrival_strikes().len(), 1);
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
            text: None,
            facing: None,
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
        text: None,
        facing: None,
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

// Task 111: a jump from anywhere in a system, to the edge of an unexplored system or anywhere in
// an explored one.

/// A point of system `system`'s plane, `dx`, `dy` milli-tiles from its star.
fn near_star(w: &World, system: u32, dx: i32, dy: i32) -> Pos {
    let at = star(w, system);
    Pos {
        x: at.x + dx,
        y: at.y + dy,
        ..at
    }
}

#[test]
fn a_ship_charges_where_it_stands_and_departs_from_there() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        // In the middle of its system, far from the edge toward system 2.
        let start = near_star(&w, 1, -4_000, 3_000);
        let ship = spawn(&mut w, "unit.survey_ship", P1, start);
        let charge = charge_of(&w, "unit.survey_ship");
        travel(&mut w, ship, 2).unwrap();
        w.step();
        let begun = w.tick();
        assert_eq!(w.jump_charge(ship).unwrap().required, charge);
        run_until(&mut w, 400, |w| in_transit(w, ship));
        // No flight to the edge: it charged for its jump_charge and left from where it stood.
        assert_eq!(w.tick() - begun, charge);
        let transit = w.transits().find(|t| t.entity.id == ship).unwrap();
        assert_eq!(transit.entity.pos, start);
        w
    });
    assert_eq!(w.transits().count(), 1);
}

#[test]
fn a_ship_arrives_at_the_point_in_an_explored_system_and_at_the_edge_of_an_unexplored_one() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        assert!(w.system_explored(P1, 1), "the home system");
        assert!(!w.system_explored(P1, 2));
        let wanted = near_star(&w, 2, 4_000, -6_000);
        let scout = {
            let at = near_star(&w, 1, 0, 5_000);
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        travel_to(&mut w, scout, wanted).unwrap();
        run_until(&mut w, 1000, |w| {
            w.entity(scout).is_some_and(|e| e.pos.body == wanted.body)
        });
        // Unexplored: it came out at the edge facing system 1.
        assert_eq!(w.entity(scout).unwrap().pos, w.system_edge(2, 1).unwrap());
        assert!(w.system_explored(P1, 2), "a ship there explores it");
        // Its enemies have not.
        assert!(!w.system_explored(P2, 2));
        // The next ship sent there comes out at the point its order names.
        let second = {
            let at = near_star(&w, 1, 0, -5_000);
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        travel_to(&mut w, second, wanted).unwrap();
        run_until(&mut w, 1000, |w| {
            w.entity(second).is_some_and(|e| e.pos.body == wanted.body)
        });
        assert_eq!(w.entity(second).unwrap().pos, wanted);
        // A point beyond the plane is kept inside it; back home it arrives where it is sent.
        let beyond = Pos {
            x: 900_000,
            ..wanted
        };
        let third = {
            let at = near_star(&w, 1, 3_000, 0);
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        travel_to(&mut w, third, beyond).unwrap();
        let home = near_star(&w, 1, -9_000, 2_000);
        travel_to(&mut w, scout, home).unwrap();
        run_until(&mut w, 2000, |w| {
            w.entity(third).is_some_and(|e| e.pos.body == wanted.body)
                && w.entity(scout).is_some_and(|e| e.pos.body == home.body)
        });
        let rom_sim::Topology::Flat { width, .. } = w.surface(wanted.body).unwrap().topology()
        else {
            panic!("a system's plane is flat");
        };
        assert_eq!(
            w.entity(third).unwrap().pos,
            Pos {
                x: width as i32 * 1000 - 1,
                ..wanted
            }
        );
        assert_eq!(w.entity(scout).unwrap().pos, home);
        w
    });
    assert_eq!(w.transits_done(), 4);
}

#[test]
fn a_voyage_over_several_systems_charges_at_each_intermediate_edge() {
    let rules = rules();
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        // Systems 2 and 3 explored (their planes seen): only the last leg ends at the point.
        w.reveal_terrain(P1, BodyId(10));
        w.reveal_terrain(P1, BodyId(20));
        assert!(w.system_explored(P1, 2) && w.system_explored(P1, 3));
        let ship = {
            let at = near_star(&w, 1, 2_000, 2_000);
            spawn(&mut w, "unit.survey_ship", P1, at)
        };
        let wanted = near_star(&w, 3, -7_000, 4_000);
        travel_to(&mut w, ship, wanted).unwrap();
        assert_eq!(w.travel_path(P1, 1, 3), Some(vec![1, 2, 3]));
        run_until(&mut w, 2000, |w| {
            w.entity(ship).is_some_and(|e| e.pos.body == BodyId(10))
        });
        // Passing through system 2 it comes out at the edge facing system 1 and charges there.
        let edge = w.system_edge(2, 1).unwrap();
        assert_eq!(w.entity(ship).unwrap().pos, edge);
        w.step();
        let charge = w
            .jump_charge(ship)
            .expect("charging at the intermediate edge");
        assert_eq!(charge.toward, 3);
        run_until(&mut w, 400, |w| in_transit(w, ship));
        assert_eq!(
            w.transits()
                .find(|t| t.entity.id == ship)
                .unwrap()
                .entity
                .pos,
            edge
        );
        run_until(&mut w, 2000, |w| {
            w.entity(ship).is_some_and(|e| e.pos.body == BodyId(20))
        });
        assert_eq!(w.entity(ship).unwrap().pos, wanted);
        assert!(w.entity(ship).unwrap().orders().is_empty());
        w
    });
    assert_eq!(w.transits_done(), 2);
}

#[test]
fn an_inhibitor_strikes_an_arrival_inside_its_radius_and_not_one_outside() {
    let rules = rules();
    let kind = rules.id_of("bld.jump_inhibitor").unwrap();
    let rule = rules.entity_types[&kind].inhibitor.clone().unwrap();
    assert!(rule.arrival_radius > 0);
    let w = twice(|| {
        let mut w = world(&rules, "age.abundance", &[]);
        let inhibitor = w
            .place_for_setup(kind, P2, Pos::on(BodyId(2), 0, 30_500, 30_500))
            .unwrap();
        let planet = planet_2(&w);
        let radius = rule.arrival_radius as i32;
        // The blow covers the planet and its near space, to the radius and no further.
        let edge_of_reach = Pos {
            x: planet.x - radius,
            ..planet
        };
        assert_eq!(
            w.arrival_blow(P1, edge_of_reach).map(|b| b.inhibitor),
            Some(inhibitor)
        );
        assert!(
            w.arrival_blow(
                P1,
                Pos {
                    x: planet.x - radius - 1,
                    ..planet
                }
            )
            .is_none()
        );
        // Ships arriving at the edge of a system fall outside it.
        assert!(w.arrival_blow(P1, w.system_edge(1, 2).unwrap()).is_none());
        assert!(w.arrival_blow(P1, w.system_edge(1, 4).unwrap()).is_none());
        let from_2 = w.system_edge(2, 1).unwrap();
        let near = spawn(&mut w, "unit.space_cruiser", P1, from_2);
        let far = spawn(&mut w, "unit.space_cruiser", P1, from_2);
        let full = w.entity(near).unwrap().max_hp;
        let inside = Pos {
            x: planet.x - 5_000,
            ..planet
        };
        // The star, 20 tiles from the planet: within the system, outside the radius.
        let outside = star(&w, 1);
        travel_to(&mut w, near, inside).unwrap();
        travel_to(&mut w, far, outside).unwrap();
        run_until(&mut w, 2000, |w| {
            [near, far]
                .iter()
                .all(|s| w.entity(*s).is_some_and(|e| e.pos.body == BodyId(1)))
        });
        assert_eq!(w.entity(near).unwrap().hp, full - rule.arrival_damage);
        assert_eq!(w.entity(far).unwrap().hp, full);
        let strikes: Vec<_> = w
            .arrival_strikes()
            .iter()
            .map(|s| (s.ship, s.pos))
            .collect();
        assert_eq!(strikes, [(near, inside)]);
        w
    });
    assert_eq!(w.arrival_strikes().len(), 1);
}

/// A transport of player one landed on planet 2.
fn landed_transport(w: &mut World) -> EntityId {
    let at = planet_2(w);
    let ship = spawn(w, "unit.transport_ship", P1, at);
    let marker = w.body_marker(BodyId(2));
    order(w, ship, "orbit", marker, None).unwrap();
    run_until(w, 400, |w| w.orbit(ship).is_some());
    let site = w
        .landing_site(ship, Pos::on(BodyId(2), 0, 20_500, 20_500))
        .unwrap();
    order(w, ship, "land", None, Some(site)).unwrap();
    run_until(w, 4000, |w| w.is_landed(ship));
    ship
}

#[test]
fn a_landed_ship_lifts_off_to_orbit_before_it_charges() {
    let rules = rules();
    let launch = rules.constant("space.launch_ticks").unwrap() as u64;
    let w = twice(|| {
        let mut w = world(&rules, "age.expansion", &[]);
        let ship = landed_transport(&mut w);
        let charge = charge_of(&w, "unit.transport_ship");
        travel(&mut w, ship, 2).unwrap();
        w.step();
        // Rising, not yet charging.
        assert!(w.is_landed(ship) && w.jump_charge(ship).is_none());
        assert!(w.altitude_pct(ship).is_some_and(|a| a > 0 && a < 100));
        let ordered = w.tick() - 1;
        run_until(&mut w, launch + 10, |w| !w.is_landed(ship));
        assert_eq!(w.tick() - ordered, launch);
        // Up in its planet's orbit it charges, from the start, where it rose.
        w.step();
        assert_eq!(w.jump_charge(ship).unwrap().progress, 0);
        assert_eq!(w.entity(ship).unwrap().pos, planet_2(&w));
        run_until(&mut w, charge + 10, |w| in_transit(w, ship));
        assert_eq!(w.tick() - ordered, launch + charge + 1);
        w
    });
    assert_eq!(w.transits().count(), 1);
}

#[test]
fn save_and_load_mid_charge_and_mid_transit_keep_the_hash() {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[]);
    w.reveal_terrain(P1, BodyId(10));
    let ship = {
        let at = near_star(&w, 1, 6_000, -2_000);
        spawn(&mut w, "unit.space_cruiser", P1, at)
    };
    let wanted = near_star(&w, 2, -3_000, 8_000);
    travel_to(&mut w, ship, wanted).unwrap();
    let check = |w: &mut World| {
        let mut copy = World::restore(rules.clone(), w.snapshot()).unwrap();
        assert_eq!(copy.hash(), w.hash());
        for _ in 0..30 {
            w.step();
            copy.step();
        }
        assert_eq!(copy.hash(), w.hash());
    };
    run(&mut w, 40);
    assert!(w.jump_charge(ship).is_some_and(|c| c.progress > 0));
    check(&mut w);
    run_until(&mut w, 1000, |w| in_transit(w, ship));
    check(&mut w);
    run_until(&mut w, 2000, |w| w.entity(ship).is_some());
    assert_eq!(w.entity(ship).unwrap().pos, wanted);
}

//! Task 170b: buildings that lift off travel through their star system from the Starfaring age
//! and jump to other stars from the Age of Galactic War. Their reach is the stat `lift_reach`;
//! they land on another planet only where their owner has a city or nobody has one; they jump
//! under the jump rules of ships and never below half their hit points; a world restored in each
//! stage runs on alike. Every scenario runs twice and compares state hashes.

use crate::{
    BodyId, BodyInfo, Command, EntityId, FlightStage, OrderStatus, PlanetSurface, Player, PlayerId,
    Pos, RouteInfo, SimError, SystemInfo, UniverseSetup, World,
};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const ENGINES: &str = "tech.lift_engines.barracks";
/// The research of the Starfaring age: the star system (`lift_reach` +1).
const SYSTEM: &str = "tech.age.expansion";
/// The research of the Age of Galactic War: other stars (`lift_reach` +1).
const STARS: &str = "tech.age.galactic";
/// The research that opens the marines.
const MARINES: &str = "tech.up.marine";

/// Planets: 2 and 3 in system 1, 11 in system 2 (both in cluster 1), 21 in system 3 (cluster
/// 2). Routes 1 – 2 and 2 – 3. Player one starts on planet 2 with a city, has a city on planet
/// 3 too, and has researched `techs` and the barracks' engines; player two has a city on planet
/// 11.
fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

fn world(rules: &Arc<Rules>, age: &str, techs: &[&str]) -> World {
    let id = |k: &str| rules.id_of(k).unwrap();
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: id("nation.romans"),
            age: id(age),
            resources: rules.resources.keys().map(|r| (*r, 50_000)).collect(),
            techs: if p == 1 {
                techs
                    .iter()
                    .chain(&[ENGINES, MARINES])
                    .map(|t| (id(t), 1))
                    .collect()
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
        n: (kind == BodyKind::Habitable).then_some(48),
        home_race: None,
        node_type: None,
    };
    let planet = || PlanetSurface {
        surface: CubeSurface { n: 48 },
        terrain: vec![id("ter.plain"); 6 * 48 * 48],
        nodes: vec![],
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: [(1, 0, 0, 1), (2, 100, 0, 1), (3, 200, 0, 2)]
            .into_iter()
            .map(|(id, x, y, cluster)| SystemInfo {
                z: 0,
                constellation: String::new(),
                cluster: Some(cluster),
                id,
                x,
                y,
                name: format!("System {id}"),
            })
            .collect(),
        bodies: vec![
            body(1, 1, BodyKind::Star, 0, 0),
            body(2, 1, BodyKind::Habitable, 20, 0),
            body(3, 1, BodyKind::Habitable, 60, 90_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
            body(20, 3, BodyKind::Star, 0, 0),
            body(21, 3, BodyKind::Habitable, 20, 0),
        ],
        routes: [(1, 2), (2, 3)]
            .into_iter()
            .map(|(system_a, system_b)| RouteInfo {
                system_a,
                system_b,
                length: 20,
            })
            .collect(),
        planets: [2, 3, 11, 21]
            .into_iter()
            .map(|b| (BodyId(b), planet()))
            .collect(),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        7,
        players,
        BTreeMap::from([(P1, BodyId(2)), (P2, BodyId(11))]),
    )
    .unwrap();
    spawn(&mut w, "bld.city", P1, on(2, 10, 24));
    spawn(&mut w, "bld.city", P1, on(3, 24, 24));
    spawn(&mut w, "bld.city", P2, on(11, 24, 24));
    w
}

/// The centre of tile (x, y) of the first face of planet `body`.
fn on(body: u32, x: i32, y: i32) -> Pos {
    Pos::on(BodyId(body), 0, x * 1000 + 500, y * 1000 + 500)
}
/// The centre of a footprint of even side `n` whose top-left tile is (x, y), on planet `body`.
fn corner(body: u32, x: i32, y: i32, n: i32) -> Pos {
    Pos::on(BodyId(body), 0, x * 1000 + n * 500, y * 1000 + n * 500)
}
fn spawn(w: &mut World, key: &str, owner: PlayerId, at: Pos) -> EntityId {
    w.place_for_setup(w.rules.id_of(key).unwrap(), owner, at)
        .unwrap()
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
/// Steps until `done` holds, at most `limit` ticks; panics past it.
fn until(w: &mut World, limit: u64, what: &str, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "{what} within {limit} ticks");
}
fn command(w: &World, executor: EntityId, kind: &str, to: Option<Pos>) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: to,
        queue: false,
        text: None,
        facing: None,
    }
}
fn order(w: &mut World, building: EntityId, kind: &str, to: Option<Pos>) -> Result<(), SimError> {
    let c = command(w, building, kind, to);
    w.submit(c).map(|_| ())
}
/// A barracks of player one beside its city on planet 2, in the air at its flight height.
fn lifted_barracks(w: &mut World) -> EntityId {
    let barracks = spawn(w, "bld.barracks", P1, on(2, 16, 24));
    order(w, barracks, "lift_off", None).unwrap();
    let climb = w.building_climb_ticks();
    ticks(w, climb + 2);
    assert_eq!(w.building_height_pct(barracks), Some(100));
    barracks
}
/// Sends `building` to the star of `system`.
fn to_star(w: &mut World, building: EntityId, system: u32) -> Result<(), SimError> {
    let at = star(w, system);
    order(w, building, "fly", Some(at))
}
fn star(w: &World, system: u32) -> Pos {
    w.body_position(w.space_of(system).unwrap()).unwrap()
}
fn stage(w: &World, id: EntityId) -> Option<FlightStage> {
    w.flight_stage(id)
}

/// Its reach is its age's: in the Spacefaring age its planet only; with the Starfaring age's
/// research its star system but not another star; with the Galactic War's research other stars.
fn reach_scenario() -> (u64, Vec<(i64, Result<(), SimError>, Result<(), SimError>)>) {
    let rules = rules();
    let mut out = Vec::new();
    let mut last = 0;
    for techs in [&[][..], &[SYSTEM][..], &[SYSTEM, STARS][..]] {
        let age = if techs.len() == 2 {
            "age.galactic"
        } else {
            "age.expansion"
        };
        let mut w = world(&rules, age, techs);
        let barracks = lifted_barracks(&mut w);
        let reach = w.lift_reach(barracks);
        let system = order(&mut w, barracks, "fly", Some(on(3, 20, 20)));
        let stars = to_star(&mut w, barracks, 2);
        // Over its own planet it flies anywhere it may.
        order(&mut w, barracks, "fly", Some(on(2, 20, 20))).unwrap();
        ticks(&mut w, 40);
        out.push((reach, system, stars));
        last = w.hash();
    }
    (last, out)
}

#[test]
fn a_flight_beyond_its_reach_is_refused() {
    let (hash, out) = reach_scenario();
    assert_eq!(
        out,
        vec![
            (0, Err(SimError::BeyondReach), Err(SimError::BeyondReach)),
            (1, Ok(()), Err(SimError::BeyondReach)),
            (2, Ok(()), Ok(())),
        ]
    );
    assert_eq!(reach_scenario().0, hash);
}

/// A barracks with marines queued flies from planet 2 to planet 3 of its system and lands
/// beside its owner's city there: it climbs to orbit, crosses the plane, comes down over its
/// landing, settles and trains again. Each stage is restored from a snapshot and runs on alike.
fn system_flight() -> (u64, Vec<FlightStage>, Pos, u64) {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[SYSTEM]);
    let barracks = spawn(&mut w, "bld.barracks", P1, on(2, 16, 24));
    let marine = w.rules.id_of("unit.marine").unwrap();
    w.submit(Command {
        subject: Some(marine),
        queue: true,
        ..command(&w, barracks, "train", None)
    })
    .unwrap();
    ticks(&mut w, 5);
    order(&mut w, barracks, "lift_off", None).unwrap();
    ticks(&mut w, 2);
    let landing = corner(3, 18, 24, 2);
    w.submit(Command {
        facing: Some(90),
        ..command(&w, barracks, "touch_down", Some(landing))
    })
    .unwrap();
    let mut seen = Vec::new();
    let mut paused = None;
    for _ in 0..6000 {
        let now = stage(&w, barracks);
        if let Some(now) = now
            && seen.last() != Some(&now)
        {
            seen.push(now);
            // A world restored in this stage runs on alike.
            let mut copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
            assert_eq!(copy.hash(), w.hash(), "restored in {now:?}");
            for _ in 0..3 {
                w.step();
                copy.step();
                assert_eq!(copy.hash(), w.hash(), "runs on alike in {now:?}");
            }
        }
        if now == Some(FlightStage::Space) {
            // In the plane it is no target for ground weapons and keeps its queue paused.
            let progress = w.entity(barracks).unwrap().orders().front().unwrap();
            let held = progress.progress_ticks;
            paused.get_or_insert(held);
            assert_eq!(paused, Some(held), "the queue stays paused in space");
            assert_eq!(w.target_layer(barracks), rom_rules::Layer::Space);
        }
        if now.is_none() {
            break;
        }
        w.step();
    }
    let e = w.entity(barracks).unwrap();
    assert!(!w.airborne(barracks), "it has landed");
    assert_eq!(e.pos.body, BodyId(3));
    assert_eq!(e.facing, 90);
    let pos = e.pos;
    // Its queue trains again on the ground.
    let before = e.orders().front().unwrap().progress_ticks;
    ticks(&mut w, 10);
    let after = w
        .entity(barracks)
        .unwrap()
        .orders()
        .front()
        .map_or(u64::MAX, |o| o.progress_ticks);
    assert!(after > before, "the queue resumes on landing");
    (w.hash(), seen, pos, w.tick())
}

#[test]
fn a_building_flies_to_another_planet_of_its_system_and_lands_there() {
    let (hash, seen, pos, tick) = system_flight();
    assert_eq!(
        seen,
        vec![
            FlightStage::Air,
            FlightStage::Climbing,
            FlightStage::Orbit,
            FlightStage::Space,
            FlightStage::Orbit,
            FlightStage::Descending,
            FlightStage::Air,
        ]
    );
    assert_eq!(pos, corner(3, 18, 24, 2));
    let again = system_flight();
    assert_eq!((again.0, again.3), (hash, tick));
}

/// It may not land on planet 11, where player two has a city, and may on planet 21, where
/// nobody has one (the borders waived there); a flight to the place of planet 3 in the plane
/// ends in its orbit.
fn landing_rules() -> (u64, Result<(), SimError>, Result<(), SimError>, FlightStage) {
    let rules = rules();
    let mut w = world(&rules, "age.galactic", &[SYSTEM, STARS]);
    let barracks = lifted_barracks(&mut w);
    let foreign = order(&mut w, barracks, "touch_down", Some(corner(11, 30, 30, 2)));
    let unclaimed = order(&mut w, barracks, "touch_down", Some(corner(21, 30, 30, 2)));
    let place = w.body_position(BodyId(3)).unwrap();
    order(&mut w, barracks, "fly", Some(place)).unwrap();
    until(&mut w, 4000, "orbit of planet 3", |w| {
        w.orbit(barracks).is_some_and(|o| o.body == BodyId(3))
    });
    ticks(&mut w, 5);
    let held = stage(&w, barracks).unwrap();
    assert_eq!(w.building_flight(barracks).unwrap().goal, None);
    (w.hash(), foreign, unclaimed, held)
}

#[test]
fn a_building_lands_only_where_its_owner_has_a_city_or_nobody_has_one() {
    let (hash, foreign, unclaimed, held) = landing_rules();
    assert_eq!(foreign, Err(SimError::ForeignPlanet));
    assert_eq!(unclaimed, Ok(()));
    assert_eq!(held, FlightStage::Orbit);
    assert_eq!(landing_rules().0, hash);
}

/// In the Starfaring age with both researches (reach 2, map scale interstellar) it may jump to
/// system 2 of its cluster but not to system 3 of another; below half its hit points it may
/// not jump, and a building hit below half while it charges gives up; whole again, it charges,
/// jumps to system 2 (as long as a ship of its speed would), comes out where it is bound (an
/// unexplored system: at its edge) and flies on to the star, where it hovers.
fn jump_scenario() -> (u64, Vec<Result<(), SimError>>, u64) {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[SYSTEM, STARS]);
    let barracks = lifted_barracks(&mut w);
    let mut out = vec![to_star(&mut w, barracks, 3), to_star(&mut w, barracks, 2)];
    until(&mut w, 4000, "charging", |w| {
        stage(w, barracks) == Some(FlightStage::Charging)
    });
    assert!(w.jump_charge(barracks).is_some_and(|c| c.toward == 2));
    // Hit below half its hit points while it charges: it gives up and stays in the plane.
    let e = w.entities[barracks.0 as usize].as_mut().unwrap();
    let whole = e.hp;
    e.hp = e.max_hp / 2 - 1;
    ticks(&mut w, 2);
    assert_eq!(stage(&w, barracks), Some(FlightStage::Space));
    assert_eq!(w.building_flight(barracks).unwrap().target(), None);
    out.push(to_star(&mut w, barracks, 2));
    w.entities[barracks.0 as usize].as_mut().unwrap().hp = whole;
    out.push(to_star(&mut w, barracks, 2));
    until(&mut w, 400, "in transit", |w| {
        stage(w, barracks) == Some(FlightStage::Transit)
    });
    assert!(
        w.entity(barracks).is_none(),
        "between systems it is on no surface"
    );
    let departed = w.tick();
    // A world restored between systems runs on alike.
    let mut copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(copy.hash(), w.hash());
    until(&mut w, 2000, "arrived", |w| w.entity(barracks).is_some());
    for _ in 0..w.tick() - departed {
        copy.step();
    }
    assert_eq!(copy.hash(), w.hash(), "a restored transit arrives alike");
    let leg = w.tick() - departed;
    let e = w.entity(barracks).unwrap();
    assert_eq!(w.system_space(e.pos.body), Some(2));
    assert_eq!(stage(&w, barracks), Some(FlightStage::Space));
    assert_eq!(w.arrived_from(barracks), Some(1));
    until(&mut w, 2000, "at the star", |w| {
        w.building_flight(barracks)
            .is_some_and(|f| f.goal.is_none())
    });
    assert_eq!(w.entity(barracks).unwrap().pos, star(&w, 2));
    (w.hash(), out, leg)
}

#[test]
fn a_jump_follows_the_cluster_tier_and_needs_half_its_hit_points() {
    let (hash, out, leg) = jump_scenario();
    assert_eq!(
        out,
        vec![
            Err(SimError::ClusterClosed),
            Ok(()),
            Err(SimError::TooDamagedToJump),
            Ok(()),
        ]
    );
    // A leg of 20 units at 1.3 tiles a second, uncharted (500 %): 20 × 1 000 × 500 / 1 300 / 100
    // ticks, rounded up.
    assert_eq!(leg, 77);
    let again = jump_scenario();
    assert_eq!((again.0, again.2), (hash, leg));
}

/// Space weapons reach a building in its system's plane: an enemy corvette beside it fires.
fn space_fight() -> (u64, i64) {
    let rules = rules();
    let mut w = world(&rules, "age.expansion", &[SYSTEM]);
    let barracks = lifted_barracks(&mut w);
    to_star(&mut w, barracks, 1).unwrap();
    until(&mut w, 4000, "in the plane", |w| {
        stage(w, barracks) == Some(FlightStage::Space)
    });
    let pos = w.entity(barracks).unwrap().pos;
    let corvette = w
        .spawn(
            w.rules.id_of("unit.corvette").unwrap(),
            P2,
            w.offset(pos, 3000, 0),
        )
        .unwrap();
    let c = Command {
        target_entity: Some(barracks),
        ..command(&w, corvette, "attack", None)
    };
    w.submit(c).unwrap();
    let whole = w.entity(barracks).unwrap().hp;
    ticks(&mut w, 200);
    let hp = w.entity(barracks).map_or(0, |e| e.hp);
    assert!(hp < whole, "the corvette hits it in space");
    assert!(
        w.entity(corvette)
            .unwrap()
            .orders()
            .front()
            .is_none_or(|o| !matches!(o.status, OrderStatus::Failed(_)))
    );
    (w.hash(), hp)
}

#[test]
fn space_weapons_reach_a_building_in_space() {
    let (hash, hp) = space_fight();
    assert_eq!(space_fight(), (hash, hp));
}

//! Task 170: buildings that lift off. Lift-off needs the line's engines, which open in the
//! Spacefaring age; a building lands only where its footprint passes the placement check; its
//! queue pauses in the air and resumes on the ground; units under a landing are moved out;
//! anti-air reaches it in the air and ground weapons do not; a world restored mid-flight runs
//! on alike. Every scenario runs twice and compares state hashes.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const ENGINES: &str = "tech.lift_engines.barracks";
/// The research that opens the marines.
const MARINES: &str = "tech.up.marine";

/// A plain of 80 × 40 tiles with a mountain block at columns 30–33, rows 30–33, in `age`;
/// player one has researched `techs` and owns a city at (10, 20), whose borders reach 40 tiles.
/// Every player starts with 5 000 of each resource.
fn world(age: &str, techs: &[&str]) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of(age).unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: if n == 1 {
                techs.iter().map(|k| (rules.id_of(k).unwrap(), 1)).collect()
            } else {
                Default::default()
            },
        })
        .collect();
    let plain = rules.id_of("ter.plain").unwrap();
    let mountain = rules.id_of("ter.mountain").unwrap();
    let (w, h) = (80, 40);
    let terrain = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            if (30..34).contains(&x) && (30..34).contains(&y) {
                mountain
            } else {
                plain
            }
        })
        .collect();
    let mut world = World::new(
        rules,
        MapGrid::new(w as u32, h as u32, terrain).unwrap(),
        1,
        players,
    )
    .unwrap();
    spawn(&mut world, "bld.city", P1, 10, 20);
    world
}
fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}
fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}
/// The centre of a footprint of even size whose top-left tile is (x, y) and side `n`.
fn corner(x: i32, y: i32, n: i32) -> Pos {
    Pos::flat(x * 1000 + n * 500, y * 1000 + n * 500)
}
fn spawn(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    w.place_for_setup(id(w, key), owner, at(x, y)).unwrap()
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn command(w: &World, executor: EntityId, kind: &str) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    }
}
fn lift(w: &mut World, building: EntityId) -> Result<(), SimError> {
    let c = command(w, building, "lift_off");
    w.submit(c).map(|_| ())
}
fn fly(w: &mut World, building: EntityId, to: Pos) -> Result<(), SimError> {
    let c = Command {
        target_pos: Some(to),
        ..command(w, building, "fly")
    };
    w.submit(c).map(|_| ())
}
fn land(w: &mut World, building: EntityId, to: Pos, facing: u16) -> Result<(), SimError> {
    let c = Command {
        target_pos: Some(to),
        facing: Some(facing),
        ..command(w, building, "touch_down")
    };
    w.submit(c).map(|_| ())
}
fn train(w: &mut World, producer: EntityId, unit: &str) {
    let c = Command {
        subject: Some(id(w, unit)),
        queue: true,
        ..command(w, producer, "train")
    };
    w.submit(c).unwrap();
}
/// Ticks of training done by the producer's front order.
fn progress(w: &World, producer: EntityId) -> u64 {
    w.entity(producer)
        .unwrap()
        .orders()
        .front()
        .map_or(0, |o| o.progress_ticks)
}
fn units_of(w: &World, key: &str) -> Vec<EntityId> {
    let t = id(w, key);
    w.entities()
        .filter(|e| e.type_id == t && e.owner == P1)
        .map(|e| e.id)
        .collect()
}

/// In the Fusion age the engines cannot be researched; in the Spacefaring age a barracks
/// without them cannot lift off; with them it lifts off, rises in the climb's ticks and is in
/// the air.
fn lift_off_rules(age: &str, techs: &[&str]) -> (u64, Vec<Result<(), SimError>>) {
    let mut w = world(age, techs);
    let barracks = spawn(&mut w, "bld.barracks", P1, 20, 20);
    let research = Command {
        subject: Some(id(&w, ENGINES)),
        queue: true,
        ..command(&w, barracks, "research")
    };
    let mut out = vec![w.submit(research).map(|_| ())];
    // Fresh orders go before the research in this world: drop it.
    let stop = command(&w, barracks, "stop");
    w.submit(stop).unwrap();
    ticks(&mut w, 1);
    out.push(lift(&mut w, barracks));
    ticks(&mut w, 1);
    out.push(fly(&mut w, barracks, at(30, 20)));
    let climb = w.building_climb_ticks();
    ticks(&mut w, climb);
    if out[1].is_ok() {
        assert!(w.airborne(barracks));
        assert_eq!(w.building_height_pct(barracks), Some(100));
        // In the air it gives up its footprint: a walker crosses where it stood.
        let footprint_free = w
            .check_placement(P1, id(&w, "bld.barracks"), at(20, 20))
            .is_ok();
        assert!(footprint_free, "its place is free while it flies");
        out.push(lift(&mut w, barracks));
    }
    (w.hash(), out)
}

#[test]
fn lift_off_needs_the_engines_and_their_age() {
    let (_, early) = lift_off_rules("age.fusion", &[]);
    assert_eq!(early[0], Err(SimError::Locked));
    assert_eq!(early[1], Err(SimError::CannotLiftOff));
    assert_eq!(early[2], Err(SimError::NotAirborne));
    let (_, without) = lift_off_rules("age.starfaring", &[]);
    assert_eq!(without[0], Ok(()));
    assert_eq!(without[1], Err(SimError::CannotLiftOff));
    let (first, with) = lift_off_rules("age.starfaring", &[ENGINES]);
    assert_eq!(with[1], Ok(()));
    assert_eq!(with[2], Ok(()));
    assert_eq!(with[3], Err(SimError::Airborne));
    let (second, _) = lift_off_rules("age.starfaring", &[ENGINES]);
    assert_eq!(first, second);
}

/// The engines of a line reach every step of it and no other building: a barracks's research
/// does not lift a library.
#[test]
fn the_engines_belong_to_their_line() {
    let mut w = world("age.starfaring", &[ENGINES]);
    let library = spawn(&mut w, "bld.library", P1, 20, 26);
    let city = w.entities().find(|e| e.owner == P1).unwrap().id;
    assert_eq!(lift(&mut w, library), Err(SimError::CannotLiftOff));
    // Cities, gather sites and walls are never fitted with engines.
    assert_eq!(lift(&mut w, city), Err(SimError::CannotLiftOff));
    let rules = w.rules().clone();
    let liftable = rules
        .tags
        .values()
        .find(|t| t.key == "liftable")
        .unwrap()
        .id;
    for key in [
        "bld.city",
        "bld.farm",
        "bld.mine",
        "bld.stone_wall",
        "bld.shield_emitter",
    ] {
        assert!(!rules.has_tag(id(&w, key), liftable), "{key}");
    }
    for key in [
        "bld.barracks",
        "bld.factory",
        "bld.airfield",
        "bld.spaceport",
        "bld.library",
    ] {
        assert!(rules.has_tag(id(&w, key), liftable), "{key}");
    }
}

/// An airborne barracks is refused a landing on another building, on a mountain, across the
/// borders and off the map; a landing it was given is abandoned when a building is put there
/// meanwhile, and it climbs back; it then lands where it fits, turned as the order says.
fn landing_rules() -> (u64, Vec<Result<(), SimError>>, Pos, u16) {
    let mut w = world("age.starfaring", &[ENGINES]);
    let barracks = spawn(&mut w, "bld.barracks", P1, 20, 20);
    spawn(&mut w, "bld.library", P1, 26, 20);
    lift(&mut w, barracks).unwrap();
    ticks(&mut w, 2);
    let out = vec![
        land(&mut w, barracks, at(26, 20), 0),
        land(&mut w, barracks, corner(30, 30, 4), 0),
        land(&mut w, barracks, at(70, 20), 0),
        land(&mut w, barracks, at(120, 20), 0),
    ];
    // Sent to land in the open, it finds a building standing there when it comes down.
    let spot = corner(20, 10, 4);
    land(&mut w, barracks, spot, 0).unwrap();
    ticks(&mut w, 70);
    w.place_for_setup(id(&w, "bld.barracks"), P1, spot).unwrap();
    ticks(&mut w, 400);
    assert!(w.airborne(barracks), "a building took its place");
    assert_eq!(w.building_height_pct(barracks), Some(100));
    // Another place, turned a quarter.
    land(&mut w, barracks, corner(40, 14, 4), 90).unwrap();
    ticks(&mut w, 600);
    assert!(!w.airborne(barracks));
    let e = w.entity(barracks).unwrap();
    (w.hash(), out, e.pos, e.facing)
}

#[test]
fn a_building_lands_only_where_its_footprint_fits() {
    let (first, out, pos, facing) = landing_rules();
    assert_eq!(out[0], Err(SimError::PlacementOverlap));
    assert_eq!(out[1], Err(SimError::UnbuildableTerrain));
    assert_eq!(out[2], Err(SimError::NotInOwnTerritory));
    assert_eq!(out[3], Err(SimError::OutsideMap));
    assert_eq!((pos, facing), (corner(40, 14, 4), 90));
    let (second, ..) = landing_rules();
    assert_eq!(first, second);
}

/// A barracks training marines lifts off: the training stops where it was; it flies, lands
/// and the training goes on from there; the marines come out beside its new place.
fn queue_pauses() -> (u64, u64, u64, Vec<Pos>) {
    let mut w = world("age.starfaring", &[ENGINES, MARINES]);
    let barracks = spawn(&mut w, "bld.barracks", P1, 20, 20);
    train(&mut w, barracks, "unit.marine");
    train(&mut w, barracks, "unit.marine");
    ticks(&mut w, 100);
    let before = progress(&w, barracks);
    assert!(before > 50);
    lift(&mut w, barracks).unwrap();
    ticks(&mut w, 2);
    fly(&mut w, barracks, at(40, 20)).unwrap();
    ticks(&mut w, 300);
    let paused = progress(&w, barracks);
    assert_eq!(
        w.entity(barracks).unwrap().orders().len(),
        2,
        "the queue stays"
    );
    assert!(units_of(&w, "unit.marine").is_empty());
    land(&mut w, barracks, corner(44, 20, 4), 0).unwrap();
    ticks(&mut w, 600);
    assert!(!w.airborne(barracks));
    ticks(&mut w, 1_200);
    let marines: Vec<Pos> = units_of(&w, "unit.marine")
        .into_iter()
        .map(|m| w.entity(m).unwrap().pos)
        .collect();
    (w.hash(), before, paused, marines)
}

#[test]
fn the_queue_pauses_in_the_air_and_resumes_on_the_ground() {
    let (first, before, paused, marines) = queue_pauses();
    // The step that lifts it off may still have trained one tick.
    assert!(paused <= before + 1, "{before} → {paused}");
    assert_eq!(marines.len(), 2);
    for m in &marines {
        assert!(m.x > 40_000, "beside its new place: {m:?}");
    }
    let (second, ..) = queue_pauses();
    assert_eq!(first, second);
}

/// Units standing where a building lands are moved to the nearest ground they can stand on,
/// never shut in under its footprint.
fn landing_on_units() -> (u64, Vec<Pos>) {
    let mut w = world("age.starfaring", &[ENGINES]);
    let barracks = spawn(&mut w, "bld.barracks", P1, 20, 20);
    let mut crowd = Vec::new();
    for (x, y) in [(40, 20), (41, 21), (42, 22), (43, 23)] {
        crowd.push(spawn(&mut w, "unit.spearman", P1, x, y));
    }
    crowd.push(spawn(&mut w, "unit.citizen", P1, 41, 22));
    lift(&mut w, barracks).unwrap();
    ticks(&mut w, 2);
    land(&mut w, barracks, corner(40, 20, 4), 0).unwrap();
    ticks(&mut w, 600);
    assert!(!w.airborne(barracks));
    let spots: Vec<Pos> = crowd.iter().map(|u| w.entity(*u).unwrap().pos).collect();
    (w.hash(), spots)
}

#[test]
fn units_under_a_landing_are_moved_out() {
    let (first, spots) = landing_on_units();
    for p in &spots {
        let inside = (40_000..44_000).contains(&p.x) && (20_000..24_000).contains(&p.y);
        assert!(!inside, "a unit was shut in: {p:?}");
    }
    let (second, _) = landing_on_units();
    assert_eq!(first, second);
}

/// In the air an enemy laser battery (anti-air) hits the building and a spearman cannot; on
/// the ground the spearman can.
fn anti_air() -> (u64, bool, bool, i64, bool) {
    let mut w = world("age.starfaring", &[ENGINES]);
    let barracks = spawn(&mut w, "bld.barracks", P1, 20, 20);
    lift(&mut w, barracks).unwrap();
    ticks(&mut w, 2);
    let battery = spawn(&mut w, "unit.laser_battery", P2, 26, 20);
    let spear = spawn(&mut w, "unit.spearman", P2, 23, 21);
    let spear_in_air = w.can_target(spear, barracks);
    let battery_in_air = w.can_target(battery, barracks);
    let full = w.entity(barracks).unwrap().hp;
    let attack = Command {
        target_entity: Some(barracks),
        ..command(&w, battery, "attack")
    };
    w.submit(attack).unwrap();
    ticks(&mut w, 200);
    let hit = full - w.entity(barracks).map_or(0, |e| e.hp);
    // Down again, the spearman reaches it.
    let stop = command(&w, battery, "stop");
    w.submit(stop).unwrap();
    land(&mut w, barracks, corner(20, 22, 4), 0).unwrap();
    ticks(&mut w, 200);
    let spear_on_ground = w.can_target(spear, barracks);
    (w.hash(), spear_in_air, battery_in_air, hit, spear_on_ground)
}

#[test]
fn anti_air_reaches_a_building_in_the_air() {
    let (first, spear_in_air, battery_in_air, hit, spear_on_ground) = anti_air();
    assert!(!spear_in_air);
    assert!(battery_in_air);
    assert!(hit > 0);
    assert!(spear_on_ground);
    let (second, ..) = anti_air();
    assert_eq!(first, second);
}

/// A world restored from a snapshot mid-flight has the same hash and runs on alike: the
/// building lands at the same tick and place.
#[test]
fn a_flight_survives_a_snapshot() {
    let mut w = world("age.starfaring", &[ENGINES, MARINES]);
    let barracks = spawn(&mut w, "bld.barracks", P1, 20, 20);
    train(&mut w, barracks, "unit.marine");
    ticks(&mut w, 30);
    lift(&mut w, barracks).unwrap();
    ticks(&mut w, 2);
    land(&mut w, barracks, corner(44, 20, 4), 0).unwrap();
    ticks(&mut w, 120);
    assert!(w.airborne(barracks));
    let mut copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(copy.hash(), w.hash());
    for _ in 0..1_000 {
        w.step();
        copy.step();
        assert_eq!(copy.hash(), w.hash());
    }
    assert!(!copy.airborne(barracks));
    assert_eq!(
        copy.entity(barracks).unwrap().pos,
        w.entity(barracks).unwrap().pos
    );
}

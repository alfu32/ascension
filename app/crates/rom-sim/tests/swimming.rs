//! Swimming (task 168): citizens cross shallow water after the dock's Swimming, foot infantry
//! after the barracks' Combat Swimming; deep water stays closed, vehicles never swim, a path
//! takes a short walk round before a long swim, a swimming citizen neither works nor fights and
//! a swimming soldier fires from the water. Every scenario runs twice and compares state hashes.

use std::sync::Arc;

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, World};

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

fn player(rules: &Rules, n: u32, age: &str, techs: &[&str]) -> Player {
    Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of(age).unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
        techs: techs.iter().map(|k| (rules.id_of(k).unwrap(), 1)).collect(),
    }
}

/// A 64 x 64 flat map from a terrain picker over tile coordinates.
fn world(age: &str, techs: &[&str], terrain: impl Fn(i32, i32) -> &'static str) -> World {
    let rules = rules();
    let tiles = (0..64 * 64)
        .map(|i| rules.id_of(terrain(i % 64, i / 64)).unwrap())
        .collect();
    let map = MapGrid::new(64, 64, tiles).unwrap();
    let players = vec![player(&rules, 1, age, techs), player(&rules, 2, age, &[])];
    World::new(rules, map, 7, players).unwrap()
}

/// West land up to column 16, a shallow strait to 22, an island to 34, deep water to 40, then
/// east land.
fn strait(x: i32, _y: i32) -> &'static str {
    match x {
        ..16 => "ter.plain",
        16..22 => "ter.coast",
        22..34 => "ter.plain",
        34..40 => "ter.deep",
        _ => "ter.plain",
    }
}

fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x, y)
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn spawn(w: &mut World, owner: PlayerId, key: &str, pos: Pos) -> EntityId {
    let kind = id(w, key);
    w.spawn(kind, owner, pos).unwrap()
}

fn command(executor: EntityId, kind: &str) -> Command {
    Command {
        player: P1,
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

fn move_to(w: &mut World, unit: EntityId, to: Pos) {
    w.submit(Command {
        target_pos: Some(to),
        ..command(unit, "move")
    })
    .unwrap();
}

/// Researches `key` at `building` and steps until it is done.
fn research(w: &mut World, building: EntityId, key: &str) {
    let tech = id(w, key);
    w.submit(Command {
        subject: Some(tech),
        ..command(building, "research")
    })
    .unwrap();
    for _ in 0..5000 {
        w.step();
        if researched(w, key) {
            return;
        }
    }
    panic!("{key} was never researched");
}

fn researched(w: &World, key: &str) -> bool {
    let tech = id(w, key);
    w.players()
        .find(|p| p.id == P1)
        .unwrap()
        .techs
        .get(&tech)
        .is_some_and(|n| *n > 0)
}

fn terrain_key(w: &World, pos: Pos) -> String {
    let t = w.terrain_at(pos).unwrap();
    w.rules().key(t).to_string()
}

/// What a walk to `to` did: where the unit ended, whether it ever stood in shallow water or in
/// deep water, and the longest step it took while swimming, in milli-tiles.
struct Trip {
    end: Pos,
    swam: bool,
    deep: bool,
    swim_step: i64,
    land_step: i64,
}

fn walk(w: &mut World, unit: EntityId, to: Pos, ticks: u32) -> Trip {
    move_to(w, unit, to);
    let mut trip = Trip {
        end: w.entity(unit).unwrap().pos,
        swam: false,
        deep: false,
        swim_step: 0,
        land_step: 0,
    };
    for _ in 0..ticks {
        let before = w.entity(unit).unwrap().pos;
        let wet = w.swimming(unit);
        w.step();
        let now = w.entity(unit).unwrap().pos;
        let step = i64::from((now.x - before.x).abs().max((now.y - before.y).abs()));
        // Whole steps only: a step that starts or ends at a waypoint is shorter.
        if wet && w.swimming(unit) {
            trip.swim_step = trip.swim_step.max(step);
        } else if !wet && !w.swimming(unit) {
            trip.land_step = trip.land_step.max(step);
        }
        trip.swam |= w.swimming(unit);
        trip.deep |= terrain_key(w, now) == "ter.deep";
        trip.end = now;
        if w.entity(unit).unwrap().orders().is_empty() {
            break;
        }
    }
    trip
}

#[test]
fn a_citizen_swims_the_strait_only_after_swimming_is_researched() {
    let run = || {
        let mut w = world("age.industrial", &[], strait);
        let dock = spawn(&mut w, P1, "bld.dock", at(8_000, 50_000));
        let citizen = spawn(&mut w, P1, "unit.citizen", at(10_500, 10_500));
        let kind = id(&w, "unit.citizen");
        let (home, island) = (at(10_500, 10_500), at(28_500, 10_500));
        assert_eq!(w.reachable_for(P1, kind, home, island), Some(false));
        assert!(!w.can_swim(P1, kind));
        let before = walk(&mut w, citizen, island, 1200);
        assert!(!before.swam, "a citizen entered the water before Swimming");
        assert!(before.end.x < 16_000, "it stays ashore: {:?}", before.end);

        research(&mut w, dock, "tech.swimming");
        assert!(w.can_swim(P1, kind));
        assert_eq!(w.reachable_for(P1, kind, home, island), Some(true));
        let after = walk(&mut w, citizen, island, 3000);
        assert!(after.swam, "it never swam");
        assert!(
            (after.end.x - island.x).abs() <= 1000 && (after.end.y - island.y).abs() <= 1000,
            "it reached the island: {:?}",
            after.end
        );
        // 1 300 milli-tiles a second on land is 65 a tick; in the water 40 % of it.
        assert_eq!(after.land_step, 65);
        assert_eq!(after.swim_step, 26);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn deep_water_stays_closed_to_swimmers() {
    let run = || {
        let mut w = world(
            "age.fusion",
            &["tech.swimming", "tech.combat_swimming"],
            strait,
        );
        let citizen = spawn(&mut w, P1, "unit.citizen", at(28_500, 20_500));
        let soldier = spawn(&mut w, P1, "unit.rifleman", at(28_500, 24_500));
        let east = at(50_500, 20_500);
        for (unit, key) in [(citizen, "unit.citizen"), (soldier, "unit.rifleman")] {
            let kind = id(&w, key);
            assert_eq!(
                w.reachable_for(P1, kind, at(28_500, 20_500), east),
                Some(false)
            );
            let trip = walk(&mut w, unit, east, 1500);
            assert!(!trip.deep, "{key} entered deep water");
            assert!(
                trip.end.x < 34_000,
                "{key} stays on the island: {:?}",
                trip.end
            );
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn infantry_swims_only_after_its_own_research() {
    let run = || {
        let mut w = world("age.fusion", &["tech.swimming"], strait);
        let barracks = spawn(&mut w, P1, "bld.barracks", at(8_000, 50_000));
        let soldier = spawn(&mut w, P1, "unit.rifleman", at(10_500, 10_500));
        let kind = id(&w, "unit.rifleman");
        let island = at(28_500, 10_500);
        // The citizens' research does not teach soldiers.
        assert!(!w.can_swim(P1, kind));
        let before = walk(&mut w, soldier, island, 1200);
        assert!(!before.swam && before.end.x < 16_000, "{:?}", before.end);

        research(&mut w, barracks, "tech.combat_swimming");
        assert!(w.can_swim(P1, kind));
        let after = walk(&mut w, soldier, island, 3000);
        assert!(after.swam);
        assert!((after.end.x - island.x).abs() <= 1000, "{:?}", after.end);
        // 1 250 milli-tiles a second is 62.5 a tick; in the water half of it.
        assert!((31..=32).contains(&after.swim_step), "{}", after.swim_step);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn vehicles_and_riders_never_swim() {
    let run = || {
        let mut w = world(
            "age.fusion",
            &["tech.swimming", "tech.combat_swimming"],
            strait,
        );
        let island = at(28_500, 10_500);
        // Wheeled soldiers count as infantry but are machines; horsemen are not infantry.
        for (i, key) in ["unit.combat_robot", "unit.knight", "unit.armoured_car"]
            .into_iter()
            .enumerate()
        {
            let from = at(10_500, 10_500 + 4_000 * i as i32);
            let unit = spawn(&mut w, P1, key, from);
            let kind = id(&w, key);
            assert!(!w.can_swim(P1, kind), "{key} swims");
            assert_eq!(w.reachable_for(P1, kind, from, island), Some(false));
            let trip = walk(&mut w, unit, island, 1200);
            assert!(!trip.swam && trip.end.x < 16_000, "{key}: {:?}", trip.end);
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

/// A strait from column 16 to 22 that ends at row `end`: land south of it joins both shores.
fn bay(end: i32) -> impl Fn(i32, i32) -> &'static str {
    move |x, y| {
        if (16..22).contains(&x) && y < end {
            "ter.coast"
        } else {
            "ter.plain"
        }
    }
}

#[test]
fn a_path_takes_a_short_walk_round_before_a_long_swim() {
    let run = |end: i32| {
        let mut w = world("age.industrial", &["tech.swimming"], bay(end));
        let citizen = spawn(&mut w, P1, "unit.citizen", at(10_500, 8_500));
        let goal = at(28_500, 8_500);
        let trip = walk(&mut w, citizen, goal, 4000);
        assert!(
            (trip.end.x - goal.x).abs() <= 1000 && (trip.end.y - goal.y).abs() <= 1000,
            "{:?}",
            trip.end
        );
        (trip.swam, w.hash())
    };
    // The bay ends four rows past the start: walking round is shorter than swimming 6 tiles
    // at 40 %.
    let short = run(12);
    assert!(!short.0, "it swam where a short walk round was quicker");
    assert_eq!(short, run(12));
    // The bay runs almost to the map's edge: the swim wins.
    let long = run(60);
    assert!(long.0, "it walked round a long bay");
    assert_eq!(long, run(60));
}

#[test]
fn a_swimming_citizen_neither_builds_nor_fights_until_ashore() {
    let run = || {
        let mut w = world("age.industrial", &["tech.swimming"], strait);
        // Both start in the water within reach of their targets, whose footprints begin at the
        // shore (column 22): the farm's 4 x 4 at 22..26, the city site's 7 x 7 at 22..29.
        let farm = spawn(&mut w, P2, "bld.farm", at(24_000, 30_000));
        let fighter = spawn(&mut w, P1, "unit.citizen", at(21_800, 30_000));
        let builder = spawn(&mut w, P1, "unit.citizen", at(21_500, 10_500));
        // A city may be founded outside every border: the first of its owner.
        let site = at(25_500, 10_500);
        w.submit(Command {
            target_entity: Some(farm),
            ..command(fighter, "attack")
        })
        .unwrap();
        let city = id(&w, "bld.city");
        w.submit(Command {
            subject: Some(city),
            target_pos: Some(site),
            ..command(builder, "construct")
        })
        .unwrap();
        let hp = w.entity(farm).unwrap().hp;
        let (mut fighter_ashore, mut builder_ashore) = (None, None);
        let (mut first_hit, mut first_work) = (None, None);
        let mut site_id = None;
        for tick in 0..2000_u32 {
            w.step();
            if fighter_ashore.is_none() && !w.swimming(fighter) {
                fighter_ashore = Some(tick);
            }
            if builder_ashore.is_none() && !w.swimming(builder) {
                builder_ashore = Some(tick);
            }
            if first_hit.is_none() && w.entity(farm).is_none_or(|f| f.hp < hp) {
                first_hit = Some(tick);
            }
            site_id = site_id.or_else(|| {
                w.entities()
                    .find(|e| e.owner == P1 && e.type_id == city)
                    .map(|e| e.id)
            });
            if first_work.is_none()
                && site_id
                    .and_then(|s| w.entity(s))
                    .is_some_and(|s| s.build_progress.is_some_and(|p| p > 0))
            {
                first_work = Some(tick);
            }
        }
        let (fighter_ashore, builder_ashore) = (fighter_ashore.unwrap(), builder_ashore.unwrap());
        assert!(
            fighter_ashore > 0 && builder_ashore > 0,
            "they start in the water"
        );
        // A unit that steps ashore in a tick may strike or work in that same tick.
        assert!(first_hit.expect("it fought ashore") >= fighter_ashore);
        assert!(first_work.expect("it built ashore") >= builder_ashore);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_swimming_soldier_fires_from_the_water() {
    let run = || {
        let mut w = world(
            "age.fusion",
            &["tech.swimming", "tech.combat_swimming"],
            strait,
        );
        let farm = spawn(&mut w, P2, "bld.farm", at(25_500, 30_500));
        let soldier = spawn(&mut w, P1, "unit.rifleman", at(20_500, 30_500));
        let hp = w.entity(farm).unwrap().hp;
        w.submit(Command {
            target_entity: Some(farm),
            ..command(soldier, "attack")
        })
        .unwrap();
        for _ in 0..200 {
            w.step();
            assert!(w.swimming(soldier), "it stays in the water to fire");
        }
        assert!(w.entity(farm).is_none_or(|f| f.hp < hp), "it never fired");
        w.hash()
    };
    assert_eq!(run(), run());
}

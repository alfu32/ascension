//! Task 141: citizens called up as militia and stood down again.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// A 96 × 96 plain; player 1 holds the land around (25, 25), player 2 around (75, 75); between
/// them lies no one's land. Buildings do not shoot.
fn world_in(age: &str, techs: &[&str]) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');
         UPDATE et_territory SET border_radius=16000, economic_radius=16000 WHERE is_city=1;",
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
            techs: techs.iter().map(|t| (rules.id_of(t).unwrap(), 1)).collect(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules,
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        1,
        players,
    )
    .unwrap();
    spawn(&mut w, "bld.city", P1, 25, 25);
    spawn(&mut w, "bld.city", P2, 75, 75);
    w
}
fn world() -> World {
    world_in("age.ancient", &[])
}
fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}
fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}
fn spawn(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    w.place_for_setup(id(w, key), owner, at(x, y)).unwrap()
}
fn key(w: &World, unit: EntityId) -> &str {
    w.rules.key(w.entity(unit).unwrap().type_id)
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn order(w: &mut World, executor: EntityId, kind: &str) -> Result<(), SimError> {
    let player = w.entity(executor).unwrap().owner;
    w.submit(Command {
        player,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    })
    .map(|_| ())
}
fn walk(w: &mut World, executor: EntityId, x: i32, y: i32) {
    let player = w.entity(executor).unwrap().owner;
    w.submit(Command {
        player,
        executor,
        kind: "move".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(at(x, y)),
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
}

#[test]
fn a_citizen_called_up_keeps_its_place_and_share_of_health_and_joins_the_army() {
    let mut w = world();
    let citizen = spawn(&mut w, "unit.citizen", P1, 30, 25);
    w.entities[citizen.0 as usize].as_mut().unwrap().hp = 20;
    walk(&mut w, citizen, 40, 25);
    ticks(&mut w, 1);
    let army = w.army_size(P1);
    let before = w.entity(citizen).unwrap().pos;
    order(&mut w, citizen, "call_up").unwrap();
    ticks(&mut w, 1);
    let e = w.entity(citizen).unwrap();
    assert_eq!(key(&w, citizen), "unit.militia_ancient");
    // Half its health before, half after: 20 of 40, then 30 of 60.
    assert_eq!((e.hp, e.max_hp), (30, 60));
    assert!(e.orders().is_empty(), "orders are cleared");
    assert_eq!(e.pos, before, "it stays where it was called up");
    assert_eq!(w.army_size(P1), army + 1);
    // It gathers and builds nothing, and no idle work finds it: not the farm, not its mending.
    let farm = spawn(&mut w, "bld.farm", P1, 32, 28);
    w.entities[farm.0 as usize].as_mut().unwrap().hp = 100;
    ticks(&mut w, 400);
    let e = w.entity(citizen).unwrap();
    assert!(e.orders().is_empty() && e.work_site.is_none());
}

#[test]
fn standing_down_makes_an_idle_citizen_again_with_the_same_share() {
    let mut w = world();
    let citizen = spawn(&mut w, "unit.citizen", P1, 30, 25);
    order(&mut w, citizen, "call_up").unwrap();
    ticks(&mut w, 1);
    w.entities[citizen.0 as usize].as_mut().unwrap().hp = 45;
    order(&mut w, citizen, "stand_down").unwrap();
    ticks(&mut w, 1);
    let e = w.entity(citizen).unwrap();
    assert_eq!(key(&w, citizen), "unit.citizen");
    assert_eq!((e.hp, e.max_hp), (30, 40));
    // As any idle citizen it goes by itself to mend own buildings nearby (task 140).
    let site = spawn(&mut w, "bld.farm", P1, 33, 28);
    w.entities[site.0 as usize].as_mut().unwrap().hp = 100;
    ticks(&mut w, 200);
    let e = w.entity(citizen).unwrap();
    assert!(
        e.orders().iter().any(|o| o.target_entity == Some(site)),
        "the citizen went back to work"
    );
}

#[test]
fn a_working_citizen_called_up_leaves_its_site() {
    let mut w = world();
    let farm = spawn(&mut w, "bld.farm", P1, 30, 30);
    let citizen = spawn(&mut w, "unit.citizen", P1, 30, 27);
    w.submit(Command {
        player: P1,
        executor: citizen,
        kind: "harvest".into(),
        subject: None,
        target_entity: Some(farm),
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
    ticks(&mut w, 200);
    assert_eq!(w.occupants(farm), vec![citizen]);
    order(&mut w, citizen, "call_up").unwrap();
    ticks(&mut w, 1);
    assert!(w.occupants(farm).is_empty());
    assert!(w.entity(citizen).unwrap().work_site.is_none());
}

#[test]
fn militia_are_called_up_and_stood_down_only_inside_own_territory() {
    let mut w = world();
    let outside = spawn(&mut w, "unit.citizen", P1, 50, 50);
    let foreign = spawn(&mut w, "unit.citizen", P1, 75, 72);
    let home = spawn(&mut w, "unit.citizen", P1, 25, 30);
    for unit in [outside, foreign] {
        assert_eq!(
            order(&mut w, unit, "call_up"),
            Err(SimError::MilitiaOutsideTerritory)
        );
    }
    order(&mut w, home, "call_up").unwrap();
    ticks(&mut w, 1);
    // Called up at home, it marches out; out there it cannot stand down.
    walk(&mut w, home, 50, 50);
    ticks(&mut w, 1_000);
    assert_eq!(
        order(&mut w, home, "stand_down"),
        Err(SimError::MilitiaOutsideTerritory)
    );
    // Only workers are called up and only militia stand down.
    let soldier = spawn(&mut w, "unit.spearman", P1, 26, 30);
    assert_eq!(
        order(&mut w, soldier, "call_up"),
        Err(SimError::CannotCallUp)
    );
    assert_eq!(
        order(&mut w, soldier, "stand_down"),
        Err(SimError::CannotStandDown)
    );
    assert_eq!(
        order(&mut w, outside, "stand_down"),
        Err(SimError::CannotStandDown)
    );
}

#[test]
fn the_militia_takes_the_weapon_of_its_age() {
    let mut w = world();
    let early = spawn(&mut w, "unit.citizen", P1, 30, 25);
    order(&mut w, early, "call_up").unwrap();
    ticks(&mut w, 1);
    // The advance to the next age changes the militia standing into that age's.
    let advance = id(&w, "tech.age.classical");
    w.complete_research(P1, advance);
    assert_eq!(key(&w, early), "unit.militia_classical");
    let late = spawn(&mut w, "unit.citizen", P1, 31, 25);
    order(&mut w, late, "call_up").unwrap();
    ticks(&mut w, 1);
    assert_eq!(key(&w, late), "unit.militia_classical");

    // In the automation age with worker drones: a drone is called up, and every militia stands
    // down into the newest worker, the drone.
    let mut w = world_in("age.automation", &["tech.up.worker_drone"]);
    let drone = spawn(&mut w, "unit.worker_drone", P1, 30, 25);
    let citizen = spawn(&mut w, "unit.citizen", P1, 31, 25);
    for unit in [drone, citizen] {
        order(&mut w, unit, "call_up").unwrap();
    }
    ticks(&mut w, 1);
    assert_eq!(key(&w, drone), "unit.militia_automation");
    assert_eq!(key(&w, citizen), "unit.militia_automation");
    for unit in [drone, citizen] {
        order(&mut w, unit, "stand_down").unwrap();
    }
    ticks(&mut w, 1);
    assert_eq!(key(&w, drone), "unit.worker_drone");
    assert_eq!(key(&w, citizen), "unit.worker_drone");
}

/// Five of `a` against five of `b` on open ground: how many of each are left.
fn fight(age: &str, a: &str, b: &str) -> (usize, usize) {
    let mut w = world_in(age, &[]);
    let side = |w: &mut World, key: &str, owner: PlayerId, x: i32| -> Vec<EntityId> {
        (0..5).map(|n| spawn(w, key, owner, x, 45 + n)).collect()
    };
    let first = side(&mut w, a, P1, 47);
    let second = side(&mut w, b, P2, 50);
    for unit in first.iter().chain(&second) {
        let target = if first.contains(unit) { 52 } else { 45 };
        let player = w.entity(*unit).unwrap().owner;
        w.submit(Command {
            player,
            executor: *unit,
            kind: "attack_move".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(at(target, 47)),
            queue: false,
            text: None,
            facing: None,
        })
        .unwrap();
    }
    ticks(&mut w, 2_400);
    let alive = |ids: &[EntityId]| ids.iter().filter(|id| w.entity(**id).is_some()).count();
    (alive(&first), alive(&second))
}

#[test]
fn militia_beat_citizens_and_lose_to_the_infantry_of_their_age() {
    for (age, militia, infantry) in [
        ("age.ancient", "unit.militia_ancient", "unit.spearman"),
        ("age.industrial", "unit.militia_industrial", "unit.rifleman"),
    ] {
        let (left, citizens) = fight(age, militia, "unit.citizen");
        assert!(
            left > 0 && citizens == 0,
            "{militia} vs citizens: {left}–{citizens}"
        );
        let (left, soldiers) = fight(age, militia, infantry);
        assert!(
            left == 0 && soldiers > 0,
            "{militia} vs {infantry}: {left}–{soldiers}"
        );
    }
}

#[test]
fn militia_are_saved_as_ordinary_state() {
    let mut w = world();
    let citizen = spawn(&mut w, "unit.citizen", P1, 30, 25);
    order(&mut w, citizen, "call_up").unwrap();
    ticks(&mut w, 1);
    let copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(copy.hash(), w.hash());
    assert_eq!(key(&copy, citizen), "unit.militia_ancient");
}

/// Citizens called up as an enemy comes, fighting, some stood down again, and an age advance.
fn scenario() -> u64 {
    let mut w = world();
    let citizens: Vec<EntityId> = (0..6)
        .map(|n| spawn(&mut w, "unit.citizen", P1, 22 + n, 30))
        .collect();
    let raiders: Vec<EntityId> = (0..3)
        .map(|n| spawn(&mut w, "unit.spearman", P2, 40 + n, 34))
        .collect();
    for raider in raiders {
        w.submit(Command {
            player: P2,
            executor: raider,
            kind: "attack_move".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(at(25, 30)),
            queue: false,
            text: None,
            facing: None,
        })
        .unwrap();
    }
    ticks(&mut w, 100);
    for citizen in &citizens {
        order(&mut w, *citizen, "call_up").unwrap();
    }
    ticks(&mut w, 900);
    let advance = id(&w, "tech.age.classical");
    w.complete_research(P1, advance);
    for citizen in citizens.iter().step_by(2) {
        if w.entity(*citizen).is_some() {
            let _ = order(&mut w, *citizen, "stand_down");
        }
    }
    ticks(&mut w, 600);
    w.hash()
}

#[test]
fn militia_scenario_runs_alike_twice() {
    assert_eq!(scenario(), scenario());
}

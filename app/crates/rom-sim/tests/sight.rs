//! Sight and defences that fire (task 68): sight technologies widen the fog of war, an armed
//! building fires while it trains or researches, and a range upgrade makes a tower see and hit
//! further. Each scenario runs twice from the same start and must end in the same state.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const ENEMY: PlayerId = PlayerId(2);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// A 64 × 64 plain; player one in `age` with `techs`.
fn world(rules: &Arc<Rules>, age: &str, techs: &[&str]) -> World {
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        age: rules
            .id_of(if id == 1 { age } else { "age.ancient" })
            .unwrap(),
        faction: rules.id_of("nation.romans").unwrap(),
        resources: BTreeMap::new(),
        techs: if id == 1 {
            techs.iter().map(|t| (rules.id_of(t).unwrap(), 1)).collect()
        } else {
            BTreeMap::new()
        },
    };
    let plain = rules.id_of("ter.plain").unwrap();
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(64, 64, vec![plain; 64 * 64]).unwrap(),
        17,
        vec![player(0), player(1), player(2)],
    )
    .unwrap();
    for resource in rules.resources.keys().copied().collect::<Vec<_>>() {
        world.grant(OWNER, resource, 100_000).unwrap();
    }
    world
}

fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        ..Pos::default()
    }
}

fn spawn(world: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    world.spawn(id(world, key), owner, pos).unwrap()
}

fn command(player: PlayerId, executor: EntityId, kind: &str) -> Command {
    Command {
        player,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
    }
}

fn visible_tiles(world: &World, player: PlayerId) -> usize {
    world
        .visibility_tiles(player, rom_sim::BodyId(0))
        .iter()
        .filter(|t| **t == 2)
        .count()
}

/// A city that trains citizens shoots an enemy walking into its range all the same.
fn city_training_fires() -> (u64, i64, bool) {
    let rules = rules();
    let mut w = world(&rules, "age.ancient", &[]);
    let city = spawn(&mut w, "bld.city", OWNER, at(16_500, 32_500));
    let intruder = spawn(&mut w, "unit.spearman", ENEMY, at(50_500, 32_500));
    let citizen = id(&w, "unit.citizen");
    for _ in 0..3 {
        w.submit(Command {
            subject: Some(citizen),
            queue: true,
            ..command(OWNER, city, "train")
        })
        .unwrap();
    }
    w.submit(Command {
        target_pos: Some(at(16_500, 32_500)),
        ..command(ENEMY, intruder, "move")
    })
    .unwrap();
    let mut training_while_hit = false;
    let full = w.entity(intruder).unwrap().hp;
    for _ in 0..400 {
        w.step();
        let Some(e) = w.entity(intruder) else {
            break;
        };
        if e.hp < full
            && w.entity(city)
                .unwrap()
                .orders()
                .front()
                .is_some_and(|o| o.kind == "train")
        {
            training_while_hit = true;
        }
    }
    let hp = w.entity(intruder).map_or(0, |e| e.hp);
    (w.hash(), full - hp, training_while_hit)
}

#[test]
fn a_city_training_fires_at_an_intruder() {
    let first = city_training_fires();
    assert!(first.1 > 0, "the intruder was never hit");
    assert!(first.2, "the city did not fire while training");
    assert_eq!(first, city_training_fires());
}

/// An armed building keeps firing while it researches, and its aim is in the saved state.
#[test]
fn a_researching_tower_fires_and_its_aim_survives_a_snapshot() {
    let rules = rules();
    let mut w = world(&rules, "age.classical", &[]);
    let tower = spawn(&mut w, "bld.tower", OWNER, at(20_000, 20_000));
    let tech = id(&w, "tech.militia_training");
    w.submit(Command {
        subject: Some(tech),
        ..command(OWNER, tower, "research")
    })
    .unwrap();
    let foe = spawn(&mut w, "unit.spearman", ENEMY, at(27_000, 20_000));
    for _ in 0..12 {
        w.step();
    }
    assert_eq!(w.entity(tower).unwrap().aim, Some(foe));
    assert_eq!(
        w.entity(tower).unwrap().orders().front().unwrap().kind,
        "research"
    );
    let restored = World::restore(Arc::clone(&rules), w.snapshot()).unwrap();
    assert_eq!(restored.hash(), w.hash());
    let mut a = w;
    let mut b = restored;
    for _ in 0..200 {
        a.step();
        b.step();
    }
    assert_eq!(a.hash(), b.hash());
    assert!(a.entity(foe).is_none_or(|e| e.hp < e.max_hp));
}

/// Mathematics adds a tile to a tower's range and a tile and a half to its sight.
fn tower_reach(upgraded: bool) -> (u64, i64, i64, bool, bool) {
    let rules = rules();
    let techs: &[&str] = if upgraded { &["tech.science.2"] } else { &[] };
    let mut w = world(&rules, "age.classical", techs);
    let tower = spawn(&mut w, "bld.tower", OWNER, at(20_000, 32_000));
    // Unarmed, so they stay put. 10.55 tiles from the tower's centre to the edge of the near
    // one: beyond the base range of 10 tiles; the far one 16 tiles out, beyond its sight of 15.
    let near = spawn(&mut w, "unit.merchant", ENEMY, at(30_800, 32_000));
    let far = spawn(&mut w, "unit.merchant", ENEMY, at(20_000, 48_000));
    let seen = w
        .visible_entities(OWNER)
        .iter()
        .any(|s| matches!(s, rom_sim::Sighting::Live(e) if e.id == far));
    for _ in 0..100 {
        w.step();
    }
    let e = w.entity(tower).unwrap();
    let hit = w.entity(near).is_none_or(|n| n.hp < n.max_hp);
    (
        w.hash(),
        e.los,
        w.entity(near).map_or(0, |n| n.hp),
        seen,
        hit,
    )
}

#[test]
fn a_tower_with_upgraded_range_sees_and_hits_further() {
    let base = tower_reach(false);
    let upgraded = tower_reach(true);
    assert_eq!(base.1, 15_000);
    assert_eq!(upgraded.1, 16_500);
    assert!(
        !base.3 && upgraded.3,
        "sight: {} then {}",
        base.3,
        upgraded.3
    );
    assert!(!base.4 && upgraded.4, "hit: {} then {}", base.4, upgraded.4);
    assert_eq!(base, tower_reach(false));
    assert_eq!(upgraded, tower_reach(true));
}

/// Researching Optics widens what every unit and building reveals.
fn optics_research() -> (u64, usize, usize, i64) {
    let rules = rules();
    let mut w = world(
        &rules,
        "age.gunpowder",
        &["tech.science.1", "tech.science.2", "tech.science.3"],
    );
    let library = spawn(&mut w, "bld.library", OWNER, at(10_000, 10_000));
    let scout = spawn(&mut w, "unit.scout", OWNER, at(40_500, 40_500));
    let before = visible_tiles(&w, OWNER);
    w.submit(Command {
        subject: Some(id(&w, "tech.science.4")),
        ..command(OWNER, library, "research")
    })
    .unwrap();
    for _ in 0..2000 {
        w.step();
        if w.entity(library).unwrap().orders().is_empty() {
            break;
        }
    }
    (
        w.hash(),
        before,
        visible_tiles(&w, OWNER),
        w.entity(scout).unwrap().los,
    )
}

#[test]
fn sight_technologies_widen_the_fog_of_war() {
    let first = optics_research();
    assert!(
        first.2 > first.1,
        "{} tiles before, {} after",
        first.1,
        first.2
    );
    // A scout of 11 tiles sees 15 % further.
    assert_eq!(first.3, 12_650);
    assert_eq!(first, optics_research());
}

//! Task 172: buildings in forests (foot, air and research only) and units never stuck after
//! their type changes. Every scenario runs twice and compares state hashes.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// A 64 x 64 map from a terrain picker over tile columns; player 1's city stands at (10, 32)
/// with borders and an economic radius of 24 tiles; player 2's far away at (10, 4) does not
/// matter. Buildings do not shoot.
fn world(age: &str, techs: &[&str], terrain: impl Fn(i32) -> &'static str) -> World {
    world_with("", age, techs, terrain)
}

/// [`world`] with `sql` run on the rules first.
fn world_with(
    sql: &str,
    age: &str,
    techs: &[&str],
    terrain: impl Fn(i32) -> &'static str,
) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');
         UPDATE et_territory SET border_radius=24000, economic_radius=24000 WHERE is_city=1;",
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of(age).unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: techs.iter().map(|t| (rules.id_of(t).unwrap(), 1)).collect(),
        })
        .collect();
    let tiles = (0..64 * 64)
        .map(|i| rules.id_of(terrain(i % 64)).unwrap())
        .collect();
    let mut w = World::new(rules, MapGrid::new(64, 64, tiles).unwrap(), 3, players).unwrap();
    setup(&mut w, "bld.city", P1, 10, 32);
    setup(&mut w, "bld.city", P2, 10, 4);
    w
}

/// Plain up to column 24, forest from 24 to 44, plain beyond.
fn forest_belt(x: i32) -> &'static str {
    if (24..44).contains(&x) {
        "ter.forest"
    } else {
        "ter.plain"
    }
}

/// Plain up to column 24, forest from there to the east edge.
fn deep_forest(x: i32) -> &'static str {
    if x >= 24 { "ter.forest" } else { "ter.plain" }
}

fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}
fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}
fn setup(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    w.place_for_setup(id(w, key), owner, at(x, y)).unwrap()
}
fn key(w: &World, unit: EntityId) -> &str {
    w.rules.key(w.entity(unit).unwrap().type_id)
}
fn terrain(w: &World, pos: Pos) -> &str {
    w.rules.key(w.terrain_at(pos).unwrap())
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
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
fn walk(w: &mut World, unit: EntityId, x: i32, y: i32) {
    w.submit(Command {
        target_pos: Some(at(x, y)),
        ..command(unit, "move")
    })
    .unwrap();
}
/// Walks `unit` towards (x, y) for at most `limit` ticks; true once it stands within a tile.
fn arrives(w: &mut World, unit: EntityId, x: i32, y: i32, limit: u64) -> bool {
    walk(w, unit, x, y);
    for _ in 0..limit {
        w.step();
        let pos = w.entity(unit).unwrap().pos;
        if (pos.x - at(x, y).x).abs() <= 1000 && (pos.y - at(x, y).y).abs() <= 1000 {
            return true;
        }
    }
    false
}

#[test]
fn foot_and_research_buildings_stand_in_the_forest_vehicle_works_do_not() {
    let run = || {
        let mut w = world("age.ancient", &["tech.military.1"], forest_belt);
        let forest = id(&w, "ter.forest");
        let check = |w: &World, key: &str, x: i32| w.check_placement(P1, id(w, key), at(x, 32));
        // Wholly in the forest, and half in it.
        for x in [28, 24] {
            assert!(check(&w, "bld.barracks", x).is_ok(), "barracks at {x}");
            assert!(check(&w, "bld.library", x).is_ok(), "library at {x}");
        }
        for key in ["bld.stable", "bld.vehicle_plant"] {
            assert_eq!(
                check(&w, key, 28),
                Err(SimError::UnfitForCover(forest)),
                "{key}"
            );
        }
        // A city, a resource building: not in a forest, as before.
        assert_eq!(
            check(&w, "bld.woodcutter_camp", 28),
            Err(SimError::UnbuildableTerrain)
        );
        assert!(
            check(&w, "bld.stable", 18).is_ok(),
            "the stable on open ground"
        );
        // A citizen puts the barracks up in the forest.
        let citizen = w.spawn(id(&w, "unit.citizen"), P1, at(22, 30)).unwrap();
        let barracks = id(&w, "bld.barracks");
        w.submit(Command {
            subject: Some(barracks),
            target_pos: Some(at(28, 32)),
            ..command(citizen, "construct")
        })
        .unwrap();
        ticks(&mut w, 3_000);
        let built = w
            .entities()
            .find(|e| e.type_id == barracks)
            .expect("the barracks stands");
        assert!(built.build_progress.is_none(), "the barracks is finished");
        assert_eq!(terrain(&w, built.pos), "ter.forest");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_unit_trained_in_a_forest_barracks_walks_out() {
    let run = || {
        let mut w = world("age.ancient", &[], forest_belt);
        let barracks = setup(&mut w, "bld.barracks", P1, 32, 32);
        let spearman = id(&w, "unit.spearman");
        w.submit(Command {
            subject: Some(spearman),
            ..command(barracks, "train")
        })
        .unwrap();
        let mut unit = None;
        for _ in 0..2_000 {
            w.step();
            unit = w.entities().find(|e| e.type_id == spearman).map(|e| e.id);
            if unit.is_some() {
                break;
            }
        }
        let unit = unit.expect("the spearman is trained");
        // It comes out beside the barracks, among the trees, and walks out to open ground.
        assert_eq!(terrain(&w, w.entity(unit).unwrap().pos), "ter.forest");
        assert!(arrives(&mut w, unit, 14, 36, 3_000), "it walks out");
        assert_eq!(terrain(&w, w.entity(unit).unwrap().pos), "ter.plain");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_wheeled_unit_trained_in_the_forest_appears_on_open_ground() {
    let run = || {
        let mut w = world("age.classical", &["tech.commerce.2"], forest_belt);
        // Put there by the setup: a market's own rules keep it out of the forest.
        let market = setup(&mut w, "bld.market", P1, 27, 32);
        let caravan = id(&w, "unit.caravan");
        w.submit(Command {
            subject: Some(caravan),
            ..command(market, "train")
        })
        .unwrap();
        let mut unit = None;
        for _ in 0..2_000 {
            w.step();
            unit = w.entities().find(|e| e.type_id == caravan).map(|e| e.id);
            if unit.is_some() {
                break;
            }
        }
        let unit = unit.expect("the caravan is trained");
        assert_eq!(terrain(&w, w.entity(unit).unwrap().pos), "ter.plain");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn cavalry_in_a_forest_upgraded_into_a_vehicle_is_moved_to_open_ground() {
    let run = || {
        let mut w = world("age.industrial", &[], forest_belt);
        let dragoon = w.spawn(id(&w, "unit.dragoon"), P1, at(30, 32)).unwrap();
        let riders = w.spawn(id(&w, "unit.dragoon"), P1, at(16, 32)).unwrap();
        let tech = id(&w, "tech.up.armoured_car");
        w.complete_research(P1, tech);
        assert_eq!(key(&w, dragoon), "unit.armoured_car");
        // The nearest tile it can stand on: column 23, the forest's west edge.
        assert_eq!(w.entity(dragoon).unwrap().pos, at(23, 32));
        // One on open ground stays where it was.
        assert_eq!(key(&w, riders), "unit.armoured_car");
        assert_eq!(w.entity(riders).unwrap().pos, at(16, 32));
        assert_eq!(w.waiting_retypes().count(), 0);
        // And it can be ordered away.
        assert!(arrives(&mut w, dragoon, 12, 40, 2_000), "it drives off");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn with_no_open_ground_near_the_change_waits_until_the_unit_comes_near() {
    let run = || {
        let mut w = world("age.industrial", &[], deep_forest);
        let dragoon = w.spawn(id(&w, "unit.dragoon"), P1, at(45, 32)).unwrap();
        let car = id(&w, "unit.armoured_car");
        w.complete_research(P1, id(&w, "tech.up.armoured_car"));
        // Twenty-one tiles from open ground: it stays a dragoon where it stands, waiting.
        assert_eq!(key(&w, dragoon), "unit.dragoon");
        assert_eq!(w.entity(dragoon).unwrap().pos, at(45, 32));
        assert_eq!(
            w.waiting_retypes().collect::<Vec<_>>(),
            vec![(dragoon, car)]
        );
        ticks(&mut w, 20);
        assert_eq!(key(&w, dragoon), "unit.dragoon", "still waiting");
        // A save in between keeps the waiting change.
        let restored = World::restore(w.rules.clone(), w.snapshot()).unwrap();
        assert_eq!(restored.hash(), w.hash());
        assert_eq!(restored.waiting_retypes().count(), 1);
        // Ridden towards the open, it becomes the car within reach and drives out.
        walk(&mut w, dragoon, 10, 32);
        for _ in 0..3_000 {
            w.step();
            if key(&w, dragoon) == "unit.armoured_car" {
                break;
            }
        }
        assert_eq!(key(&w, dragoon), "unit.armoured_car");
        assert_eq!(terrain(&w, w.entity(dragoon).unwrap().pos), "ter.plain");
        assert_eq!(w.waiting_retypes().count(), 0);
        assert!(arrives(&mut w, dragoon, 12, 40, 2_000), "it drives off");
        w.hash()
    };
    assert_eq!(run(), run());
}

/// Plain up to column 20, shallow water from 20 to 30, plain beyond.
fn shore(x: i32) -> &'static str {
    if (20..30).contains(&x) {
        "ter.coast"
    } else {
        "ter.plain"
    }
}

#[test]
fn a_swimming_citizen_called_up_is_moved_ashore() {
    let run = || {
        let mut w = world("age.industrial", &["tech.swimming"], shore);
        let citizen = w.spawn(id(&w, "unit.citizen"), P1, at(18, 32)).unwrap();
        assert!(arrives(&mut w, citizen, 23, 32, 2_000), "it swims out");
        assert_eq!(terrain(&w, w.entity(citizen).unwrap().pos), "ter.coast");
        w.submit(command(citizen, "call_up")).unwrap();
        ticks(&mut w, 1);
        assert!(w.is_militia_type(w.entity(citizen).unwrap().type_id));
        // Militia do not swim: it stands on the nearest dry tile, on the west shore.
        let pos = w.entity(citizen).unwrap().pos;
        assert_eq!((pos.x, terrain(&w, pos)), (19_500, "ter.plain"));
        assert_eq!(w.waiting_retypes().count(), 0);
        assert!(arrives(&mut w, citizen, 12, 36, 2_000), "it marches off");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn trees_under_a_building_give_no_timber_while_built_over() {
    let run = || {
        let mut w = world_with(
            "UPDATE et_gather_site SET slots_max=1000;",
            "age.ancient",
            &[],
            forest_belt,
        );
        let camp = id(&w, "bld.woodcutter_camp");
        let site = w.snap_placement(camp, at(19, 32)).unwrap();
        let before = w.gather_slots(camp, site);
        let barracks = setup(&mut w, "bld.barracks", P1, 25, 32);
        let after = w.gather_slots(camp, site);
        assert!(after < before, "{after} of {before} slots");
        // The trees are still there: the ground under the barracks stays forest.
        assert_eq!(terrain(&w, w.entity(barracks).unwrap().pos), "ter.forest");
        w.despawn(barracks).unwrap();
        assert_eq!(w.gather_slots(camp, site), before);
        w.hash()
    };
    assert_eq!(run(), run());
}

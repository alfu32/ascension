//! Task 92: warships of different classes are built side by side, the stable becomes the vehicle
//! plant and the siege workshop the factory by a research that keeps their queues and rally
//! points, and a building upgraded that way leaves the build menu unless people work it.
//! Every scenario runs twice and compares state hashes.

use std::sync::Arc;

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, Rally, SimError, World};

const P1: PlayerId = PlayerId(1);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// A 64 x 64 flat map, shallow water west of column 16, plain elsewhere; one player in `age`
/// with `techs` researched and plenty of everything.
fn world(age: &str, techs: &[&str]) -> World {
    let rules = rules();
    let (plain, coast) = (
        rules.id_of("ter.plain").unwrap(),
        rules.id_of("ter.coast").unwrap(),
    );
    let terrain = (0..64 * 64)
        .map(|i| if i % 64 < 16 { coast } else { plain })
        .collect();
    let map = MapGrid::new(64, 64, terrain).unwrap();
    let player = Player {
        id: P1,
        team: 1,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of(age).unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
        techs: techs.iter().map(|t| (rules.id_of(t).unwrap(), 1)).collect(),
    };
    World::new(rules, map, 11, vec![player]).unwrap()
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn order(executor: EntityId, kind: &str, subject: Option<DefId>, pos: Option<Pos>) -> Command {
    Command {
        player: P1,
        executor,
        kind: kind.into(),
        subject,
        target_entity: None,
        target_pos: pos,
        queue: true,
    }
}

fn place(w: &mut World, key: &str, x: i32, y: i32) -> EntityId {
    let kind = id(w, key);
    w.place_for_setup(kind, P1, Pos::flat(x, y)).unwrap()
}

fn count(w: &World, key: &str) -> usize {
    let kind = id(w, key);
    w.entities()
        .filter(|e| e.type_id == kind && e.owner == P1 && e.build_progress.is_none())
        .count()
}

/// The owner (2026-11-18): frigates, line ships and the other classes are built side by side.
/// Once the modern age has the ironclad's and the dreadnought's research and the destroyer,
/// frigates are still trained at the dock; the dreadnought's research retires the ironclad.
#[test]
fn later_classes_do_not_retire_frigates() {
    let run = || {
        let mut w = world(
            "age.modern",
            &[
                "tech.up.war_galley",
                "tech.up.frigate",
                "tech.up.ship_of_the_line",
                "tech.up.ironclad",
                "tech.up.dreadnought",
            ],
        );
        let dock = place(&mut w, "bld.dock", 18_000, 32_000);
        for ship in ["unit.frigate", "unit.destroyer", "unit.dreadnought"] {
            let kind = id(&w, ship);
            w.submit(order(dock, "train", Some(kind), None))
                .unwrap_or_else(|e| panic!("{ship}: {e}"));
        }
        let ironclad = id(&w, "unit.ironclad");
        assert_eq!(
            w.submit(order(dock, "train", Some(ironclad), None)),
            Err(SimError::Obsolete)
        );
        for _ in 0..2400 {
            w.step();
        }
        for ship in ["unit.frigate", "unit.destroyer", "unit.dreadnought"] {
            assert_eq!(count(&w, ship), 1, "{ship}");
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

/// Stable -> Vehicle Plant and Siege Workshop -> Factory: the research at the old building
/// turns it into the new one in place; what was queued behind the research is trained there
/// and the rally point stays.
#[test]
fn stable_and_siege_workshop_upgrade_in_place_with_their_queues() {
    let run = || {
        let mut w = world("age.industrial", &[]);
        let stable = place(&mut w, "bld.stable", 30_000, 20_000);
        let workshop = place(&mut w, "bld.siege_workshop", 30_000, 44_000);
        let point = Pos::flat(50_000, 32_000);
        for (building, upgrade, unit) in [
            (stable, "tech.up.vehicle_plant", "unit.horseman"),
            (workshop, "tech.up.factory", "unit.catapult"),
        ] {
            w.submit(Command {
                queue: false,
                ..order(building, "set_rally", None, Some(point))
            })
            .unwrap();
            let research = id(&w, upgrade);
            w.submit(order(building, "research", Some(research), None))
                .unwrap();
            let kind = id(&w, unit);
            for _ in 0..2 {
                w.submit(order(building, "train", Some(kind), None))
                    .unwrap();
            }
        }
        let (plant, factory) = (id(&w, "bld.vehicle_plant"), id(&w, "bld.factory"));
        let mut upgraded_at = None;
        for tick in 0..3000 {
            w.step();
            let types = [stable, workshop].map(|b| w.entity(b).unwrap().type_id);
            if upgraded_at.is_none() && types == [plant, factory] {
                upgraded_at = Some(tick);
                // The trainings queued behind the research are still there.
                for b in [stable, workshop] {
                    let trains = w
                        .entity(b)
                        .unwrap()
                        .orders()
                        .iter()
                        .filter(|o| o.kind == "train")
                        .count();
                    assert_eq!(trains, 2, "queue kept");
                    assert_eq!(w.rally(b), Some(Rally::Point(point)));
                }
            }
        }
        assert!(upgraded_at.is_some(), "both buildings were upgraded");
        assert_eq!(count(&w, "unit.horseman"), 2);
        assert_eq!(count(&w, "unit.catapult"), 2);
        assert_eq!(count(&w, "bld.stable"), 0);
        assert_eq!(count(&w, "bld.siege_workshop"), 0);
        w.hash()
    };
    assert_eq!(run(), run());
}

/// Once its upgrade is researched a stable can no longer be built; a farm, which people work,
/// still can after Automated Farm.
#[test]
fn superseded_buildings_leave_the_build_menu_but_manned_sites_stay() {
    let run = || {
        let mut w = world(
            "age.automation",
            &[
                "tech.military.1",
                "tech.up.vehicle_plant",
                "tech.up.auto_farm",
                "tech.up.bastion",
            ],
        );
        let city = place(&mut w, "bld.city", 40_000, 32_000);
        let _ = city;
        let citizen = id(&w, "unit.citizen");
        let builder = w.spawn(citizen, P1, Pos::flat(34_000, 32_000)).unwrap();
        for (key, x, y, expected) in [
            ("bld.stable", 46_000, 24_000, Err(SimError::Superseded)),
            ("bld.tower", 46_000, 40_000, Err(SimError::Superseded)),
            ("bld.farm", 34_000, 24_000, Ok(())),
            ("bld.vehicle_plant", 34_000, 42_000, Ok(())),
        ] {
            let kind = id(&w, key);
            let result = w
                .submit(order(
                    builder,
                    "construct",
                    Some(kind),
                    Some(Pos::flat(x, y)),
                ))
                .map(|_| ());
            assert_eq!(result, expected, "{key}");
            let active = w.active_sources(P1);
            assert_eq!(
                w.rules().is_superseded_building(kind, &active),
                expected.is_err(),
                "{key}"
            );
        }
        for _ in 0..200 {
            w.step();
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

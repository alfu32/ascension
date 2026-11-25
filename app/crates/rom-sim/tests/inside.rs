//! Workers inside a gather building: listed, hidden from enemies, released by an order, and out
//! again when given another order. Every scenario runs twice and compares state hashes.

use std::sync::Arc;

use rom_rules::{Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};

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

fn command(player: PlayerId, executor: EntityId, kind: &str) -> Command {
    Command {
        player,
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

/// A woodcutter's camp with trees around it, and `count` citizens working in it.
fn camp(count: usize) -> (World, EntityId, Vec<EntityId>) {
    let rules = rules();
    let (plain, forest) = (
        rules.id_of("ter.plain").unwrap(),
        rules.id_of("ter.forest").unwrap(),
    );
    let terrain = (0..64 * 64)
        .map(|i| {
            let (x, y) = (i % 64, i / 64);
            if (27..31).contains(&x) && (20..30).contains(&y) {
                forest
            } else {
                plain
            }
        })
        .collect();
    let player = |n: u32| Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.classical").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
        techs: Default::default(),
    };
    let map = MapGrid::new(64, 64, terrain).unwrap();
    let mut w = World::new(rules.clone(), map, 11, vec![player(1), player(2)]).unwrap();
    let camp = w
        .place_for_setup(
            rules.id_of("bld.woodcutter_camp").unwrap(),
            P1,
            Pos::flat(24_000, 24_000),
        )
        .unwrap();
    let citizen = rules.id_of("unit.citizen").unwrap();
    let workers: Vec<_> = (0..count)
        .map(|k| {
            w.spawn(citizen, P1, Pos::flat(24_000 + k as i32 * 500, 28_000))
                .unwrap()
        })
        .collect();
    for &worker in &workers {
        w.submit(Command {
            target_entity: Some(camp),
            ..command(P1, worker, "harvest")
        })
        .unwrap();
    }
    for _ in 0..400 {
        w.step();
    }
    (w, camp, workers)
}

#[test]
fn workers_in_a_gather_building_are_inside_and_listed() {
    let run = || {
        let (w, camp, workers) = camp(4);
        assert_eq!(w.occupants(camp), workers);
        assert!(workers.iter().all(|id| w.entity(*id).unwrap().is_inside()));
        assert!(w.entity(camp).unwrap().gather_slots >= 4);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn enemies_cannot_see_or_target_workers_inside() {
    let run = || {
        let (mut w, camp, workers) = camp(2);
        let spear = w.rules().id_of("unit.spearman").unwrap();
        let foe = w.spawn(spear, P2, Pos::flat(24_000, 30_000)).unwrap();
        let sees = |w: &World, player, id| {
            w.visible_entities(player)
                .iter()
                .any(|s| matches!(s, rom_sim::Sighting::Live(e) if e.id == id))
        };
        assert!(!sees(&w, P2, workers[0]), "an enemy sees a worker inside");
        assert!(sees(&w, P1, workers[0]));
        let attack = Command {
            target_entity: Some(workers[0]),
            ..command(P2, foe, "attack")
        };
        assert_eq!(w.submit(attack), Err(SimError::TargetHidden));
        // Once out, they are targets again.
        w.submit(command(P1, camp, "release")).unwrap();
        w.step();
        assert!(!w.entity(workers[0]).unwrap().is_inside());
        assert!(
            sees(&w, P2, workers[0]),
            "released workers are targets again"
        );
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn release_sends_everyone_out_and_an_order_sends_one_worker_out() {
    let run = || {
        let (mut w, camp, workers) = camp(4);
        // An order given to one worker takes just that worker out.
        w.submit(Command {
            target_pos: Some(Pos::flat(10_000, 10_000)),
            ..command(P1, workers[1], "move")
        })
        .unwrap();
        w.step();
        assert_eq!(w.occupants(camp), vec![workers[0], workers[2], workers[3]]);
        assert!(!w.entity(workers[1]).unwrap().is_inside());
        // Releasing the building takes the rest out; they are idle and have no work order.
        w.submit(command(P1, camp, "release")).unwrap();
        w.step();
        assert!(w.occupants(camp).is_empty());
        for id in [workers[0], workers[2], workers[3]] {
            let e = w.entity(id).unwrap();
            assert!(!e.is_inside() && e.orders().is_empty(), "{id:?}");
        }
        // Only gather buildings can release; only the owner may.
        assert!(w.submit(command(P1, workers[0], "release")).is_err());
        assert!(w.submit(command(P2, camp, "release")).is_err());
        w.hash()
    };
    assert_eq!(run(), run());
}

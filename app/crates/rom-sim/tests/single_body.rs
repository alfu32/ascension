//! A world with a single surface behaves exactly as before several bodies existed: the
//! flat and sphere scenarios reach the same entities, positions, hit points, orders and
//! kills as the code before the change (values recorded from it; only the state hash
//! format changed).

use std::sync::Arc;

use rom_geo::CubeSurface;
use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::World;

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// Counters plus an FNV-1a checksum of every entity's id, owner, position, hit points,
/// facing and queue length.
fn outcome(mut w: World, ticks: u64) -> String {
    for _ in 0..ticks {
        w.step();
    }
    let mut h = 0xcbf29ce484222325_u64;
    let mut feed = |v: i64| {
        for b in v.to_le_bytes() {
            h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
        }
    };
    for e in w.entities() {
        feed(i64::from(e.id.0));
        feed(i64::from(e.owner.0));
        feed(i64::from(e.pos.x));
        feed(i64::from(e.pos.y));
        feed(i64::from(e.pos.face));
        feed(e.hp);
        feed(i64::from(e.facing));
        feed(e.orders().len() as i64);
    }
    let s = w.stats();
    let p = w.path_stats();
    format!(
        "entities={} done={} failed={} no_path={} kills={:?} losses={:?} searches={} nodes={} repaths={} positions={:016x}",
        w.entities().count(),
        s.orders_done,
        s.orders_failed,
        s.orders_no_path,
        s.kills,
        s.losses,
        p.searches,
        p.nodes,
        p.repaths,
        h
    )
}

/// Plain ground with every ninth tile (by a fixed mix of its index) a mountain.
fn terrain(rules: &Rules, n: u32) -> Vec<DefId> {
    let plain = rules.id_of("ter.plain").unwrap();
    let mountain = rules.id_of("ter.mountain").unwrap();
    (0..6 * n * n)
        .map(|i| {
            if (u64::from(i).wrapping_mul(2654435761) >> 7) % 9 == 0 {
                mountain
            } else {
                plain
            }
        })
        .collect()
}

#[test]
fn single_body_world_behaves_as_before() {
    let r = rules();
    let sphere_battle = |ticks| {
        let w = World::sphere_battle_scenario(
            r.clone(),
            CubeSurface { n: 128 },
            terrain(&r, 128),
            3,
            40,
            false,
        )
        .unwrap();
        outcome(w, ticks)
    };
    let cases = [
        (
            outcome(World::movement_scenario(r.clone(), 7, 300).unwrap(), 400),
            "entities=300 done=36 failed=0 no_path=0 kills={} losses={} searches=0 nodes=0 repaths=0 positions=7f3cb28a05d98240",
        ),
        (
            outcome(
                World::obstacle_scenario(r.clone(), 7, 300, 20).unwrap(),
                400,
            ),
            "entities=300 done=27 failed=0 no_path=0 kills={} losses={} searches=301 nodes=223881 repaths=4 positions=62db2420cfc72b02",
        ),
        (
            outcome(World::battle_scenario(r.clone(), 3, 60).unwrap(), 700),
            "entities=120 done=0 failed=0 no_path=0 kills={} losses={} searches=0 nodes=0 repaths=0 positions=f80e6b8f4377e835",
        ),
        (
            outcome(World::battle_scenario(r.clone(), 3, 60).unwrap(), 1200),
            "entities=0 done=0 failed=0 no_path=0 kills={PlayerId(1): 60, PlayerId(2): 60} losses={PlayerId(1): 60, PlayerId(2): 60} searches=0 nodes=0 repaths=0 positions=cbf29ce484222325",
        ),
        (
            outcome(
                World::sphere_movement_scenario(
                    r.clone(),
                    CubeSurface { n: 64 },
                    terrain(&r, 64),
                    7,
                    300,
                )
                .unwrap(),
                400,
            ),
            "entities=300 done=142 failed=0 no_path=0 kills={} losses={} searches=287 nodes=62235 repaths=1 positions=761d1a021517fd32",
        ),
        (
            sphere_battle(700),
            "entities=80 done=0 failed=0 no_path=0 kills={} losses={} searches=0 nodes=0 repaths=0 positions=ea50172e6bf9732d",
        ),
        (
            sphere_battle(1200),
            "entities=0 done=0 failed=0 no_path=0 kills={PlayerId(1): 40, PlayerId(2): 40} losses={PlayerId(1): 40, PlayerId(2): 40} searches=0 nodes=0 repaths=0 positions=cbf29ce484222325",
        ),
    ];
    for (got, expected) in cases {
        assert_eq!(got, expected);
    }
}

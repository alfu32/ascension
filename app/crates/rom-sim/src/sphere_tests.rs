//! Incremental spatial structures on a cube sphere against brute-force recomputation.

use crate::{Command, EntityId, Player, PlayerId, Pos, SimRng, World};
use rom_geo::CubeSurface;
use rom_rules::{Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules(sql: &str) -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// A plain planet for players 1 and 3 (one team) and 2.
fn sphere(sql: &str, n: u32) -> World {
    let rules = rules(sql);
    let players = (1..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: BTreeMap::new(),
        })
        .collect();
    let surface = CubeSurface { n };
    let plain = rules.id_of("ter.plain").unwrap();
    World::from_sphere(rules, surface, vec![plain; surface.len()], &[], 7, players).unwrap()
}

fn random_pos(rng: &mut SimRng, n: u32) -> Pos {
    Pos {
        face: rng.range(0, 6) as u8,
        x: rng.range(0, n * 1000) as i32,
        y: rng.range(0, n * 1000) as i32,
        ..Pos::default()
    }
}

/// A random position within a few tiles of one of the corners of face 0.
fn near_corner(rng: &mut SimRng, w: &World, n: u32) -> Pos {
    let l = n as i64 * 1000;
    let corner = [(0, 0), (l - 1, 0), (0, l - 1), (l - 1, l - 1)][rng.range(0, 4) as usize];
    let base = Pos {
        face: 0,
        x: corner.0 as i32,
        y: corner.1 as i32,
        ..Pos::default()
    };
    let dx = i64::from(rng.range(0, 8001)) - 4000;
    let dy = i64::from(rng.range(0, 8001)) - 4000;
    w.offset(base, dx, dy)
}

#[test]
fn spatial_query_across_an_edge_matches_brute_force() {
    let n = 16;
    let mut w = sphere("", n);
    let citizen = w.rules.id_of("unit.citizen").unwrap();
    let mut rng = SimRng::new(4);
    for i in 0..400 {
        let pos = if i % 2 == 0 {
            near_corner(&mut rng, &w, n)
        } else {
            random_pos(&mut rng, n)
        };
        w.spawn(citizen, P1, pos).unwrap();
    }
    let mut across = 0;
    for i in 0..400 {
        let centre = if i % 2 == 0 {
            near_corner(&mut rng, &w, n)
        } else {
            random_pos(&mut rng, n)
        };
        let radius = rng.range(0, 10_000);
        let found = w.entities_in_radius(centre, radius);
        let brute: Vec<EntityId> = w
            .entities()
            .filter(|e| w.surfaces[0].geo.within(e.pos, centre, i64::from(radius)))
            .map(|e| e.id)
            .collect();
        assert_eq!(found, brute, "{centre:?} {radius}");
        across += found
            .iter()
            .filter(|id| w.entity(**id).unwrap().pos.face != centre.face)
            .count();
    }
    assert!(across > 100, "{across} matches across edges");
}

fn check_territory(w: &World) {
    assert_eq!(
        w.surfaces[0].territory.winner,
        w.full_territory(),
        "tick {}",
        w.tick
    );
    let mut owned: BTreeMap<PlayerId, u64> = BTreeMap::new();
    for winner in &w.surfaces[0].territory.winner {
        if *winner != 0 {
            *owned
                .entry(w.territory.emitters[&EntityId(*winner)].owner)
                .or_default() += 1;
        }
    }
    assert_eq!(w.territory.owned, owned);
}

#[test]
fn territory_disc_across_an_edge_matches_brute_force() {
    let n = 32;
    let l = n as i32 * 1000;
    let mut w = sphere("", n);
    let city = w.rules.id_of("bld.city").unwrap();
    let temple = w.rules.id_of("bld.temple").unwrap();
    // Cities hugging edges and corners on several faces, so their discs reach
    // neighbouring faces, and temples that contest the same tiles.
    let mut emitters = Vec::new();
    for (owner, key, pos) in [
        (
            P1,
            city,
            Pos {
                face: 0,
                x: l - 3500,
                y: 3500,
                ..Pos::default()
            },
        ),
        (
            P2,
            city,
            Pos {
                face: 1,
                x: 3500,
                y: 16_500,
                ..Pos::default()
            },
        ),
        (
            P1,
            temple,
            Pos {
                face: 4,
                x: l - 2000,
                y: l - 2000,
                ..Pos::default()
            },
        ),
        (
            P2,
            city,
            Pos {
                face: 5,
                x: 16_500,
                y: l - 3500,
                ..Pos::default()
            },
        ),
        (
            P2,
            temple,
            Pos {
                face: 3,
                x: 2000,
                y: 2000,
                ..Pos::default()
            },
        ),
        (
            P1,
            city,
            Pos {
                face: 2,
                x: 3500,
                y: l - 3500,
                ..Pos::default()
            },
        ),
    ] {
        emitters.push(w.place_for_setup(key, owner, pos).unwrap());
        check_territory(&w);
    }
    // Every face but none of the far side of a disc is owned.
    let faces: std::collections::BTreeSet<u8> = (0..w.surfaces[0].territory.winner.len())
        .filter(|i| w.surfaces[0].territory.winner[*i] != 0)
        .map(|i| w.surfaces[0].geo.tile_at(i).face)
        .collect();
    assert_eq!(faces.len(), 6);
    // The first city's own disc, tile by tile, against the unfolded distance.
    let first = w.entity(emitters[0]).unwrap().pos;
    let radius = w.territory.emitters[&emitters[0]].radius;
    let mut disc: Vec<usize> = w.surfaces[0].geo.disc(first, radius).flatten().collect();
    disc.sort_unstable();
    let brute: Vec<usize> = (0..w.surfaces[0].geo.tiles())
        .filter(|i| {
            w.surfaces[0]
                .geo
                .within(first, w.surfaces[0].geo.index_centre(*i), radius)
        })
        .collect();
    assert_eq!(disc, brute);
    assert!(
        brute
            .iter()
            .any(|i| w.surfaces[0].geo.tile_at(*i).face != 0)
    );
    // Removing emitters hands their tiles to the next best across edges.
    for id in emitters.into_iter().step_by(2) {
        w.despawn(id).unwrap();
        check_territory(&w);
    }
}

fn check_vision(w: &World) {
    let full = w.full_counts();
    for (player, vision) in &w.surfaces[0].vision.players {
        let expected = full
            .get(player)
            .cloned()
            .unwrap_or_else(|| vec![0; w.surfaces[0].map.terrain.len()]);
        assert_eq!(vision.count, expected, "player {player:?} tick {}", w.tick);
        for index in 0..w.surfaces[0].map.terrain.len() {
            if w.visible_index(*player, 0, index) {
                assert!(vision.explored[index / 64] & (1 << (index % 64)) != 0);
            }
        }
    }
}

#[test]
fn vision_across_an_edge_matches_brute_force() {
    let n = 24;
    // Longer sight than a citizen's two tiles, so circles spill over edges and corners.
    let mut w = sphere(
        "UPDATE entity_type SET los = 7000 \
         WHERE id = (SELECT id FROM def WHERE key = 'unit.citizen');",
        n,
    );
    let citizen = w.rules.id_of("unit.citizen").unwrap();
    let mut rng = SimRng::new(21);
    let mut units = Vec::new();
    for i in 0..30 {
        let owner = if i % 3 == 0 { P2 } else { P1 };
        let pos = near_corner(&mut rng, &w, n);
        units.push(w.spawn(citizen, owner, pos).unwrap());
    }
    check_vision(&w);
    // Walk each unit across an edge or round a corner and recheck as they go.
    for (i, unit) in units.iter().enumerate() {
        let from = w.entity(*unit).unwrap().pos;
        let to = w.offset(from, if i % 2 == 0 { 6000 } else { -6000 }, 5000);
        w.submit(Command {
            player: w.entity(*unit).unwrap().owner,
            executor: *unit,
            kind: "move".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(to),
            queue: false,
        })
        .unwrap();
    }
    for tick in 0..300 {
        w.step();
        if tick % 10 == 0 {
            check_vision(&w);
        }
    }
    check_vision(&w);
    // Face 0's corner tiles and tiles on other faces were all seen.
    let last = n as i32 - 1;
    for (x, y) in [(0, 0), (last, 0), (0, last), (last, last)] {
        assert!(w.is_explored_on(P1, 0, x, y), "({x}, {y})");
    }
    let explored_faces = (0..6)
        .filter(|f| {
            (0..n as i32)
                .any(|x| w.is_explored_on(P1, *f, x, 0) || w.is_explored_on(P1, *f, x, last))
        })
        .count();
    assert!(explored_faces >= 5, "{explored_faces} faces explored");
}

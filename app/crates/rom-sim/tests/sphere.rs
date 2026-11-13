//! Scenarios on a cube-sphere planet: walking, path search, facing, combat and battles
//! across face edges, building footprints, and determinism.

use std::sync::Arc;

use rom_geo::{CubeSurface, SurfacePos};
use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, Topology, World};

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules_with(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn rules() -> Arc<Rules> {
    rules_with("")
}

fn player(rules: &Rules, n: u32) -> Player {
    Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.ancient").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
        techs: Default::default(),
    }
}

/// A planet of `n` tiles per face edge whose terrain is named per tile, for players 1 and 2.
fn sphere_on(rules: Arc<Rules>, n: u32, terrain: impl Fn(Pos) -> &'static str) -> World {
    let surface = CubeSurface { n };
    let tiles = (0..surface.len())
        .map(|i| {
            let t = surface.tile(i);
            let centre = SurfacePos::from(t);
            rules
                .id_of(terrain(World::pos_of(centre).unwrap()))
                .unwrap()
        })
        .collect();
    let players = vec![player(&rules, 1), player(&rules, 2)];
    World::from_sphere(rules, surface, tiles, &[], 7, players).unwrap()
}

fn plain_sphere(n: u32) -> World {
    sphere_on(rules(), n, |_| "ter.plain")
}

fn flat_plain(side: u32) -> World {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    let players = vec![player(&rules, 1), player(&rules, 2)];
    let map = MapGrid::new(side, side, vec![plain; (side * side) as usize]).unwrap();
    World::new(rules, map, 7, players).unwrap()
}

fn at(face: u8, x: i32, y: i32) -> Pos {
    Pos { x, y, face }
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn order(w: &World, unit: EntityId, kind: &str, pos: Pos, queue: bool) -> Command {
    Command {
        player: w.entity(unit).unwrap().owner,
        executor: unit,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: Some(pos),
        queue,
    }
}

/// Steps until the unit has no orders; returns every position it stood on, one per tick.
fn walk(w: &mut World, unit: EntityId, limit: u32) -> Vec<Pos> {
    let mut trail = vec![w.entity(unit).unwrap().pos];
    for _ in 0..limit {
        w.step();
        let e = w.entity(unit).unwrap();
        assert!(
            w.terrain_at(e.pos).is_some(),
            "off the surface: {:?}",
            e.pos
        );
        trail.push(e.pos);
        if e.orders().is_empty() {
            return trail;
        }
    }
    panic!("unit did not arrive: {:?}", trail.last());
}

fn arrival_ticks(w: &mut World, start: Pos, target: Pos) -> (u64, Pos) {
    let unit = w.spawn(id(w, "unit.citizen"), P1, start).unwrap();
    w.submit(order(w, unit, "move", target, false)).unwrap();
    let begin = w.tick();
    walk(w, unit, 5000);
    assert_eq!(w.stats().orders_failed, 0);
    (w.tick() - begin, w.entity(unit).unwrap().pos)
}

#[test]
fn walk_across_a_face_edge_arrives_exactly() {
    // Face 0 → face 1 (no rotation), and face 1 → face 4 (a quarter turn).
    for (start, dx, dy) in [
        (at(0, 26_500, 16_500), 11_000, 0),
        (at(1, 16_500, 5_500), 0, -11_000),
        (at(0, 20_300, 30_700), 9_400, 4_100),
    ] {
        let mut sphere = plain_sphere(32);
        let target = sphere.offset(start, dx, dy);
        assert_ne!(target.face, start.face);
        let (ticks, end) = arrival_ticks(&mut sphere, start, target);
        assert_eq!(end, target);
        let mut flat = flat_plain(64);
        let from = Pos::flat(20_500, 30_500);
        let to = Pos::flat(20_500 + dx as i32, 30_500 + dy as i32);
        let (flat_ticks, flat_end) = arrival_ticks(&mut flat, from, to);
        assert_eq!(flat_end, to);
        assert_eq!(ticks, flat_ticks, "{start:?} {dx} {dy}");
    }
}

#[test]
fn walk_around_a_cube_corner() {
    let n = 32;
    let l = n as i32 * 1000;
    let mut w = plain_sphere(n);
    // Face 0's top right corner is shared with faces 1 and 4.
    let a = at(0, l - 1500, 1500);
    let b = w.offset(a, 3000, 0);
    let c = w.offset(a, 0, -3000);
    assert_eq!((b.face, c.face), (1, 4));
    let unit = w.spawn(id(&w, "unit.citizen"), P1, a).unwrap();
    for (n, point) in [b, c, a].into_iter().enumerate() {
        w.submit(order(&w, unit, "move", point, n > 0)).unwrap();
    }
    let trail = walk(&mut w, unit, 2000);
    for point in [b, c, a] {
        assert!(trail.contains(&point), "{point:?} never reached");
    }
    assert_eq!(*trail.last().unwrap(), a);
    assert!(trail.iter().any(|p| p.face == 1) && trail.iter().any(|p| p.face == 4));
    assert_eq!(w.stats().orders_failed, 0);
    // Three short legs of at most about four tiles each: no detour around the planet.
    assert!(trail.len() < 400, "{} ticks", trail.len());
}

#[test]
fn path_search_crosses_edges_and_avoids_walls_on_the_far_face() {
    // A mountain wall on face 1, four tiles past the edge with face 0, open only
    // near the top and bottom of the face.
    let wall = |p: Pos| p.face == 1 && p.x / 1000 == 4 && (3..29).contains(&(p.y / 1000));
    let mut w = sphere_on(rules(), 32, |p| {
        if wall(p) { "ter.mountain" } else { "ter.plain" }
    });
    let start = at(0, 28_500, 16_500);
    let goal = at(1, 9_500, 16_500);
    let unit = w.spawn(id(&w, "unit.citizen"), P1, start).unwrap();
    w.submit(order(&w, unit, "move", goal, false)).unwrap();
    let trail = walk(&mut w, unit, 5000);
    assert_eq!(*trail.last().unwrap(), goal);
    assert!(w.path_stats().searches > 0);
    let mountain = id(&w, "ter.mountain");
    for p in &trail {
        assert_ne!(w.terrain_at(*p), Some(mountain), "{p:?} is in the wall");
    }
    assert!(trail.iter().any(|p| p.face == 0) && trail.iter().any(|p| p.face == 1));
    // The detour goes round an end of the wall, possibly over a neighbouring face.
    assert!(trail.iter().any(|p| (p.face != 0 && p.face != 1)
        || (p.face == 1 && (p.y < 3000 || p.y >= 29_000))));
    assert_eq!(w.stats().orders_failed, 0);
}

#[test]
fn facing_rotates_with_the_position_across_an_edge() {
    let mut w = plain_sphere(32);
    let start = at(1, 16_500, 3_500);
    let target = w.offset(start, 0, -7000);
    assert_eq!(target.face, 4);
    let unit = w.spawn(id(&w, "unit.citizen"), P1, start).unwrap();
    w.submit(order(&w, unit, "move", target, false)).unwrap();
    let mut seen = Vec::new();
    for _ in 0..1000 {
        w.step();
        let e = w.entity(unit).unwrap();
        seen.push((e.pos, e.facing));
        if e.orders().is_empty() {
            break;
        }
    }
    assert_eq!(seen.last().unwrap().0, target);
    // Moving "up" on face 1 continues along face 4's negative x axis.
    for pair in seen.windows(2) {
        let ((p, _), (q, facing)) = (pair[0], pair[1]);
        if p.face == q.face && p != q {
            let expected = match (q.x.cmp(&p.x), q.y.cmp(&p.y)) {
                (std::cmp::Ordering::Greater, _) => 0,
                (_, std::cmp::Ordering::Greater) => 90,
                (std::cmp::Ordering::Less, _) => 180,
                _ => 270,
            };
            assert_eq!(facing, expected, "{p:?} → {q:?}");
        }
    }
    assert!(seen.iter().any(|(p, f)| p.face == 1 && *f == 270));
    assert!(seen.iter().any(|(p, f)| p.face == 4 && *f == 180));
}

#[test]
fn combat_range_and_acquisition_work_across_an_edge() {
    let n = 32;
    let l = n as i32 * 1000;
    // Two idle citizens on either side of an edge find each other.
    let mut w = plain_sphere(n);
    let citizen = id(&w, "unit.citizen");
    // Within the citizens' two-tile sight of each other.
    let a = w.spawn(citizen, P1, at(0, l - 800, 16_500)).unwrap();
    let b = w.spawn(citizen, P2, at(1, 800, 16_500)).unwrap();
    w.step();
    for (unit, enemy) in [(a, b), (b, a)] {
        let front = w.entity(unit).unwrap().orders().front().cloned();
        assert_eq!(front.and_then(|o| o.target_entity), Some(enemy));
    }
    for _ in 0..2000 {
        w.step();
        if w.entity(a).is_none() || w.entity(b).is_none() {
            break;
        }
    }
    assert!(w.stats().kills.values().sum::<u64>() >= 1);

    // A city's ranged weapon reaches nine tiles across the edge but not eleven and a half.
    let mut w = plain_sphere(n);
    let city = w
        .place_for_setup(id(&w, "bld.city"), P1, at(0, l - 3500, 16_500))
        .unwrap();
    let centre = w.entity(city).unwrap().pos;
    assert_eq!(centre, at(0, l - 3500, 16_500));
    let hold = w
        .rules()
        .stances
        .values()
        .find(|s| s.behavior == "hold_fire")
        .unwrap()
        .id;
    let near = w.spawn(citizen, P2, at(1, 5500, 16_500)).unwrap();
    let far = w.spawn(citizen, P2, at(1, 8000, 14_500)).unwrap();
    for unit in [near, far] {
        w.set_stance(P2, unit, hold).unwrap();
    }
    assert_eq!(w.distance(centre, at(1, 5500, 16_500)), Some(9000));
    for _ in 0..200 {
        w.step();
    }
    let hp = |w: &World, unit| w.entity(unit).map_or(0, |e| e.hp);
    assert!(hp(&w, near) < 40, "the near citizen was not hit");
    assert_eq!(hp(&w, far), 40, "the far citizen was hit");
}

fn survivors(w: &World, swap: bool) -> [usize; 2] {
    let mut counts = [0; 2];
    for e in w.entities() {
        let side = if swap { 2 - e.owner.0 } else { e.owner.0 - 1 };
        counts[side as usize] += 1;
    }
    counts
}

#[test]
fn battle_across_an_edge_is_symmetric() {
    let rules = rules();
    let surface = CubeSurface { n: 128 };
    let plain = rules.id_of("ter.plain").unwrap();
    let terrain = vec![plain; surface.len()];
    let run = |swap| {
        let mut w =
            World::sphere_battle_scenario(rules.clone(), surface, terrain.clone(), 3, 60, swap)
                .unwrap();
        // Fighters on one face strike enemies standing on the other.
        let mut crossed = false;
        for _ in 0..2400 {
            w.step();
            crossed |= w.entities().any(|e| {
                e.orders()
                    .front()
                    .and_then(|o| o.target_entity)
                    .and_then(|t| w.entity(t))
                    .is_some_and(|t| t.pos.face != e.pos.face)
            });
        }
        assert!(crossed, "no fight across the edge");
        w
    };
    let (a, b) = (run(false), run(true));
    let (alive_a, alive_b) = (survivors(&a, false), survivors(&b, true));
    assert_eq!(alive_a, alive_b);
    let kills = a.stats().kills.values().sum::<u64>();
    assert!(kills >= 60, "{kills} kills");
    assert_eq!(a.stats().kills[&P1], b.stats().kills[&P2]);
    assert_eq!(a.stats().kills[&P2], b.stats().kills[&P1]);
    // Neither line wins outright: the mirrored lines fight to a close result.
    assert!(alive_a[0].abs_diff(alive_a[1]) <= 20, "{alive_a:?}");
}

#[test]
fn footprint_crossing_an_edge_is_rejected() {
    let n = 32;
    let l = n as i32 * 1000;
    let mut w = plain_sphere(n);
    let farm = id(&w, "bld.farm");
    assert_eq!(
        w.place_for_setup(farm, P1, at(0, l - 1500, 16_500)),
        Err(SimError::CrossesFaceEdge)
    );
    assert_eq!(
        w.check_placement(P1, farm, at(2, 16_500, 500)),
        Err(SimError::CrossesFaceEdge)
    );
    assert!(w.place_for_setup(farm, P1, at(0, l - 2500, 16_500)).is_ok());
    // A footprint touching the face edge from inside is fine.
    let placed = w.place_for_setup(farm, P1, at(3, 2000, 16_000)).unwrap();
    assert_eq!(w.entity(placed).unwrap().pos, at(3, 2000, 16_000));
}

#[test]
fn four_by_four_footprints_rule_out_three_columns_and_rows_per_face() {
    // A click on a tile snaps a 4×4 footprint to cover it and the tiles 1 left/up and
    // 2 right/down of it; clicks in the first column and the last two cross an edge.
    let n = 192;
    let w = plain_sphere(n);
    let farm = id(&w, "bld.farm");
    let et = &w.rules().entity_types[&farm];
    assert_eq!((et.footprint_w, et.footprint_h), (4, 4));
    let mut crossing = 0;
    for face in 0..6 {
        for y in 0..n as i32 {
            for x in 0..n as i32 {
                let click = at(face, x * 1000 + 500, y * 1000 + 500);
                if w.check_placement(P1, farm, click) == Err(SimError::CrossesFaceEdge) {
                    crossing += 1;
                }
            }
        }
    }
    let per_face = n * n - (n - 3) * (n - 3);
    assert_eq!(per_face, 1143);
    assert_eq!(crossing, 6 * per_face);
}

/// A planet with seas and mountain ranges from a fixed pattern.
fn rough_terrain(p: Pos) -> &'static str {
    let (bx, by) = (p.x / 6000, p.y / 6000);
    let mix = (u32::from(p.face) * 7 + bx as u32 * 13 + by as u32 * 29) % 9;
    match mix {
        0 | 4 => "ter.deep",
        7 if (p.x / 1000) % 6 == 2 => "ter.mountain",
        _ => "ter.plain",
    }
}

#[test]
fn sphere_world_same_seed_same_hash() {
    let rules = rules();
    let n = 48;
    let surface = CubeSurface { n };
    let terrain: Vec<DefId> = (0..surface.len())
        .map(|i| {
            let p = World::pos_of(SurfacePos::from(surface.tile(i))).unwrap();
            rules.id_of(rough_terrain(p)).unwrap()
        })
        .collect();
    let build = || {
        World::sphere_movement_scenario(rules.clone(), surface, terrain.clone(), 11, 1000).unwrap()
    };
    let (mut a, mut b) = (build(), build());
    assert_eq!(a.topology(), Topology::Cube(surface));
    assert_eq!(a.hash(), b.hash());
    let mut faces = std::collections::BTreeSet::new();
    for tick in 0..1200 {
        a.step();
        b.step();
        if tick % 100 == 0 {
            assert_eq!(a.hash(), b.hash(), "tick {tick}");
        }
        faces.extend(a.entities().map(|e| e.pos.face));
    }
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.stats(), b.stats());
    assert!(a.stats().orders_done > 500, "{:?}", a.stats());
    assert!(a.path_stats().searches > 0);
    assert_eq!(faces.len(), 6);
    // Another seed differs.
    let c =
        World::sphere_movement_scenario(rules.clone(), surface, terrain.clone(), 12, 1000).unwrap();
    assert_ne!(c.hash(), build().hash());
}

#[test]
fn sphere_nodes_belong_to_the_neutral_player() {
    let rules = rules();
    let surface = CubeSurface { n: 16 };
    let plain = rules.id_of("ter.plain").unwrap();
    let relics = rules.id_of("node.relics").unwrap();
    let mut neutral = player(&rules, 0);
    neutral.team = 0;
    let nodes = [(
        relics,
        SurfacePos {
            face: 5,
            x: 7500,
            y: 8500,
        },
    )];
    let w = World::from_sphere(
        rules.clone(),
        surface,
        vec![plain; surface.len()],
        &nodes,
        1,
        vec![neutral, player(&rules, 1)],
    )
    .unwrap();
    let node = w.entities().next().unwrap();
    assert_eq!(
        (node.owner, node.type_id, node.pos),
        (PlayerId(0), relics, at(5, 7500, 8500))
    );
    assert_eq!(
        World::from_sphere(
            rules.clone(),
            surface,
            vec![plain; surface.len()],
            &nodes,
            1,
            vec![player(&rules, 1)],
        )
        .err(),
        Some(SimError::UnknownPlayer(PlayerId(0)))
    );
}

use super::*;
use crate::SimRng;

fn flat(width: u32, height: u32) -> Geometry {
    Geometry::new(Topology::Flat { width, height }).unwrap()
}

fn cube(n: u32) -> Geometry {
    Geometry::new(Topology::Cube(CubeSurface { n })).unwrap()
}

fn surface_pos(p: Pos) -> SurfacePos {
    SurfacePos {
        face: p.face,
        x: i64::from(p.x),
        y: i64::from(p.y),
    }
}

fn random_pos(rng: &mut SimRng, n: u32) -> Pos {
    Pos {
        face: rng.range(0, 6) as u8,
        x: rng.range(0, n * 1000) as i32,
        y: rng.range(0, n * 1000) as i32,
    }
}

#[test]
fn disc_runs_match_a_direct_scan() {
    let geo = flat(20, 15);
    for (centre, radius) in [
        (Pos::flat(5500, 5500), 2000),
        (Pos::flat(5000, 7300), 3700),
        (Pos::flat(0, 0), 4500),
        (Pos::flat(19_900, 14_900), 0),
        (Pos::flat(10_500, 7500), 0),
        (Pos::flat(10_500, 7500), -5),
    ] {
        let runs: Vec<usize> = geo.disc(centre, radius).flatten().collect();
        let scan: Vec<usize> = (0..300)
            .filter(|i| geo.within(geo.index_centre(*i), centre, radius))
            .collect();
        assert_eq!(runs, scan, "{centre:?} {radius}");
    }
}

#[test]
fn cube_vectors_match_rom_geo_unfold() {
    let n = 24;
    let geo = cube(n);
    let surface = CubeSurface { n };
    let mut rng = SimRng::new(17);
    let mut across = 0;
    for _ in 0..200_000 {
        let a = random_pos(&mut rng, n);
        let b = random_pos(&mut rng, n);
        let expected = surface.unfold(surface_pos(a), surface_pos(b));
        assert_eq!(geo.vector(a, b), expected, "{a:?} {b:?}");
        if a.face != b.face && expected.is_some() {
            across += 1;
        }
    }
    assert!(across > 1000);
}

#[test]
fn single_edge_offsets_match_rom_geo_step() {
    let n = 24;
    let geo = cube(n);
    let surface = CubeSurface { n };
    let mut rng = SimRng::new(5);
    let l = i64::from(n) * 1000;
    let mut crossed = 0;
    for _ in 0..200_000 {
        let a = random_pos(&mut rng, n);
        let dx = i64::from(rng.range(0, 24_001)) - 12_000;
        let dy = i64::from(rng.range(0, 24_001)) - 12_000;
        let (x, y) = (i64::from(a.x) + dx, i64::from(a.y) + dy);
        if (x < 0 || x >= l) && (y < 0 || y >= l) {
            continue;
        }
        let expected = surface.step(surface_pos(a), dx, dy);
        let (got, turns) = geo.offset(a, dx, dy);
        assert_eq!(surface_pos(got), expected, "{a:?} {dx} {dy}");
        if got.face != a.face {
            crossed += 1;
            assert_eq!(turns, geo.turns(a.face, got.face));
        }
    }
    assert!(crossed > 1000);
}

#[test]
fn offsets_round_a_corner_onto_the_third_face() {
    let geo = cube(16);
    // Face 0's top right corner meets faces 1 and 4.
    let start = Pos {
        face: 0,
        x: 15_500,
        y: 600,
    };
    let (end, _) = geo.offset(start, 1000, -1000);
    assert_ne!(end.face, 0);
    assert!(geo.contains(end));
    let (end, _) = geo.offset(start, 3000, -3000);
    assert!(geo.contains(end));
}

#[test]
fn neighbours_match_rom_geo() {
    let n = 8;
    let geo = cube(n);
    let surface = CubeSurface { n };
    let order = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    let mut missing = 0;
    for index in 0..surface.len() {
        let got = geo.neighbours(index, &order);
        for (slot, (dx, dy)) in got.into_iter().zip(order) {
            let expected = surface
                .neighbor(surface.tile(index), dx, dy)
                .map(|t| surface.index(t) as u32)
                .unwrap_or(u32::MAX);
            assert_eq!(slot, expected, "{index} {dx} {dy}");
            missing += usize::from(slot == u32::MAX);
        }
    }
    assert_eq!(missing, 24);
}

#[test]
fn cube_discs_match_brute_force() {
    let n = 20;
    let geo = cube(n);
    let mut rng = SimRng::new(9);
    for _ in 0..200 {
        let centre = random_pos(&mut rng, n);
        let radius = i64::from(rng.range(0, 12_000));
        let mut runs: Vec<usize> = geo.disc(centre, radius).flatten().collect();
        runs.sort_unstable();
        let scan: Vec<usize> = (0..geo.tiles())
            .filter(|i| geo.within(centre, geo.index_centre(*i), radius))
            .collect();
        assert_eq!(runs, scan, "{centre:?} {radius}");
    }
}

#[test]
fn space_points_never_exceed_walked_distance() {
    let geo = cube(6);
    let order = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    for index in 0..geo.tiles() {
        let a = geo.space_point(index);
        for (slot, (dx, dy)) in geo.neighbours(index, &order).into_iter().zip(order) {
            if slot == u32::MAX {
                continue;
            }
            let b = geo.space_point(slot as usize);
            let squared: i64 = (0..3).map(|i| (a[i] - b[i]).pow(2)).sum();
            // Doubled milli-tiles: a straight step is at most 2000, a diagonal 2000·√2.
            let limit = if dx != 0 && dy != 0 {
                8_000_000
            } else {
                4_000_000
            };
            assert!(squared <= limit, "{index} {dx} {dy} {squared}");
        }
    }
}

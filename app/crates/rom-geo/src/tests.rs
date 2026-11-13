use super::*;

fn directions() -> impl Iterator<Item = (i32, i32)> {
    (-1..=1).flat_map(|dy| (-1..=1).filter_map(move |dx| (dx != 0 || dy != 0).then_some((dx, dy))))
}

#[test]
fn every_tile_has_eight_neighbours_except_corner_diagonals() {
    for n in [2, 8, 64] {
        let s = CubeSurface { n };
        let mut missing = 0;
        for i in 0..s.len() {
            let t = s.tile(i);
            assert_eq!(s.index(t), i);
            let neighbours: Vec<_> = directions()
                .filter_map(|(dx, dy)| s.neighbor(t, dx, dy))
                .collect();
            let corner = (t.x == 0 || t.x == n - 1) && (t.y == 0 || t.y == n - 1);
            assert_eq!(neighbours.len(), if corner { 7 } else { 8 });
            let mut unique = neighbours.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), neighbours.len());
            missing += 8 - neighbours.len();
        }
        assert_eq!(missing, 8 * 3);
    }
    // At n=1 a single tile touches four corners and loses four diagonals.
    let s = CubeSurface { n: 1 };
    assert_eq!(
        (0..6)
            .map(|i| directions()
                .filter(|&(x, y)| s.neighbor(s.tile(i), x, y).is_none())
                .count())
            .sum::<usize>(),
        24
    );
}

#[test]
fn neighbor_is_symmetric() {
    let s = CubeSurface { n: 8 };
    for i in 0..s.len() {
        let t = s.tile(i);
        for b in directions().filter_map(|(dx, dy)| s.neighbor(t, dx, dy)) {
            assert!(
                directions().any(|(dx, dy)| s.neighbor(b, dx, dy) == Some(t)),
                "{t:?} -> {b:?}"
            );
        }
    }
}

#[test]
fn crossing_an_edge_and_back_returns_to_start() {
    let s = CubeSurface { n: 8 };
    for i in 0..s.len() {
        let t = s.tile(i);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let b = s.neighbor(t, dx, dy).unwrap();
            if t.face != b.face {
                let p = SurfacePos::from(b);
                let (back_x, back_y) = s.unfold(p, t.into()).unwrap();
                assert_eq!(s.step(p, back_x, back_y), t.into());
            }
        }
    }
}

fn distance2(a: [i32; 3], b: [i32; 3]) -> i64 {
    a.into_iter()
        .zip(b)
        .map(|(x, y)| (i64::from(x) - i64::from(y)).pow(2))
        .sum()
}

#[test]
fn sphere_points_of_neighbours_are_close() {
    let s = CubeSurface { n: 64 };
    let centre = Tile {
        face: 0,
        x: 32,
        y: 32,
    };
    let spacing2 = distance2(
        s.sphere_point(centre),
        s.sphere_point(s.neighbor(centre, 1, 0).unwrap()),
    );
    for i in 0..s.len() {
        let t = s.tile(i);
        let a = s.sphere_point(t);
        for b in directions().filter_map(|(dx, dy)| s.neighbor(t, dx, dy)) {
            let d = distance2(a, s.sphere_point(b));
            // At these small angles, chord bounds are slightly stricter than
            // the requested 0.5x..2x angular bounds (allow 1/1000 for rounding).
            assert!(
                d * 4 >= spacing2 && d * 1000 <= spacing2 * 3999,
                "{t:?} -> {b:?}: {d}/{spacing2}"
            );
        }
    }
}

#[test]
fn sphere_points_are_unit_length() {
    for n in [1, 8, 64, 192] {
        let s = CubeSurface { n };
        for i in 0..s.len() {
            let p = s.sphere_point(s.tile(i)).map(i64::from);
            let norm = (dot(p, p) as u64).isqrt() as i64;
            assert!((norm - i64::from(SPHERE_SCALE)).abs() <= 3);
        }
    }
}

#[test]
fn inverse_mapping_round_trips() {
    for n in [1, 8, 64, 192] {
        let s = CubeSurface { n };
        for i in 0..s.len() {
            let t = s.tile(i);
            assert_eq!(s.tile_at_point(s.sphere_point(t)), t);
        }
    }
    let s = CubeSurface { n: 64 };
    // Off-centre directions return a centre within half a tile per face axis.
    for face in 0..6 {
        for x in (1..64000).step_by(997) {
            let p = SurfacePos {
                face,
                x,
                y: (x * 13) % 63999,
            };
            let t = SurfacePos::from(s.tile_at_point(s.sphere_point(p)));
            assert_eq!(p.face, t.face);
            assert!((p.x - t.x).abs() <= 501 && (p.y - t.y).abs() <= 501);
        }
    }
}

#[test]
fn unfold_matches_step() {
    let s = CubeSurface { n: 8 };
    for i in 0..s.len() {
        let from = SurfacePos::from(s.tile(i));
        for j in 0..s.len() {
            let to = SurfacePos::from(s.tile(j));
            if let Some((x, y)) = s.unfold(from, to) {
                assert_eq!(s.step(from, x, y), to, "{from:?} -> {to:?}");
            }
        }
    }
    assert!(
        s.unfold(
            s.tile(0).into(),
            Tile {
                face: 2,
                x: 0,
                y: 0
            }
            .into()
        )
        .is_none()
    );
}

// Independent Q56 Taylor oracle, with 12 terms and a more precise pi/4.
fn reference_tan(u: i64) -> i64 {
    let q = 1_i128 << 56;
    let x = i128::from(u) * 56_593_902_016_227_522 / i128::from(Q);
    let xx = x * x / q;
    let (mut sin, mut term) = (x, x);
    for k in 1..=12 {
        term = -term * xx / (q * (2 * k) * (2 * k + 1));
        sin += term;
    }
    let (mut cos, mut term) = (q, q);
    for k in 1..=12 {
        term = -term * xx / (q * (2 * k - 1) * (2 * k));
        cos += term;
    }
    (sin * i128::from(Q) / cos) as i64
}

#[test]
fn tan_table_error_is_within_bound() {
    let mut worst = 0;
    for i in 0..=TABLE_SIZE * 16 {
        let u = i as i64 * Q / (TABLE_SIZE * 16) as i64;
        let e = (tan(u) - reference_tan(u)).abs();
        worst = worst.max(e);
        assert!(e < Q / (1 << 22), "u={u}, error={e}");
        assert_eq!(tan(-u), -tan(u));
    }
    eprintln!("tan maximum sampled Q30 error: {worst} / {Q}");
}

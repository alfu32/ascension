//! Integer cube-sphere geometry, independent of rules, simulation and rendering.

use std::sync::OnceLock;

/// Unit sphere radius in the returned Cartesian coordinates.
pub const SPHERE_SCALE: i32 = 1 << 20;
const Q: i64 = 1 << 30;
const TABLE_SIZE: usize = 4096;
// (normal, local right, local down), with world +Y pointing down.
const AXES: [[[i64; 3]; 3]; 6] = [
    [[0, 0, 1], [1, 0, 0], [0, 1, 0]],
    [[1, 0, 0], [0, 0, -1], [0, 1, 0]],
    [[0, 0, -1], [-1, 0, 0], [0, 1, 0]],
    [[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
    [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
    [[0, 1, 0], [1, 0, 0], [0, 0, -1]],
];

/// Six square grids. Face (normal; local +x, +y): 0 (+Z; +X,+Y),
/// 1 (+X; -Z,+Y), 2 (-Z; -X,+Y), 3 (-X; +Z,+Y),
/// 4 (-Y; +X,+Z), 5 (+Y; +X,-Z). All local y axes point down in this net:
/// ```text
///       [4]
///    [3][0][1][2]
///       [5]
/// ```
/// `n` must be 1..=4096; positions are half-open in 0..n*1000 on each axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeSurface {
    /// Tiles along each face edge.
    pub n: u32,
}

/// A tile on one of the six faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tile {
    /// Face index, 0..6.
    pub face: u8,
    /// Local column.
    pub x: u32,
    /// Local row.
    pub y: u32,
}

/// A surface position in milli-tiles, measured from the face's top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePos {
    /// Face index, 0..6.
    pub face: u8,
    /// Local horizontal coordinate.
    pub x: i64,
    /// Local vertical coordinate.
    pub y: i64,
}

impl From<Tile> for SurfacePos {
    fn from(t: Tile) -> Self {
        Self {
            face: t.face,
            x: i64::from(t.x) * 1000 + 500,
            y: i64::from(t.y) * 1000 + 500,
        }
    }
}

impl SurfacePos {
    /// The tile containing this position.
    pub fn tile(self) -> Tile {
        Tile {
            face: self.face,
            x: (self.x / 1000) as u32,
            y: (self.y / 1000) as u32,
        }
    }
}

fn dot(a: [i64; 3], b: [i64; 3]) -> i64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

impl CubeSurface {
    /// Number of tiles, in face / row / column order.
    pub fn len(self) -> usize {
        assert!((1..=4096).contains(&self.n));
        6 * self.n as usize * self.n as usize
    }

    /// A valid cube surface always has tiles.
    pub fn is_empty(self) -> bool {
        self.n == 0
    }

    /// Dense index of a valid tile; panics for out-of-bounds coordinates.
    pub fn index(self, t: Tile) -> usize {
        assert!(t.face < 6 && t.x < self.n && t.y < self.n);
        (usize::from(t.face) * self.n as usize + t.y as usize) * self.n as usize + t.x as usize
    }

    /// Tile at a dense index; panics when out of bounds.
    pub fn tile(self, index: usize) -> Tile {
        assert!(index < self.len());
        let n = self.n as usize;
        Tile {
            face: (index / (n * n)) as u8,
            x: (index % n) as u32,
            y: (index / n % n) as u32,
        }
    }

    /// Neighbour in the local direction, including rotated face crossings.
    /// At each corner three tiles each lose one diagonal: 24 missing directions.
    /// The zero direction returns the input tile.
    pub fn neighbor(self, t: Tile, dx: i32, dy: i32) -> Option<Tile> {
        assert!((-1..=1).contains(&dx) && (-1..=1).contains(&dy));
        self.index(t);
        let x = i64::from(t.x) + i64::from(dx);
        let y = i64::from(t.y) + i64::from(dy);
        let outside = |v| v < 0 || v >= i64::from(self.n);
        if outside(x) && outside(y) {
            return None;
        }
        Some(
            self.step(t.into(), i64::from(dx) * 1000, i64::from(dy) * 1000)
                .tile(),
        )
    }

    /// Move by milli-tiles across at most one edge, rotating the remaining vector.
    /// Panics if the endpoint would cross two edges (the caller must split the move).
    pub fn step(self, p: SurfacePos, dx: i64, dy: i64) -> SurfacePos {
        self.index(p.tile());
        let l = i64::from(self.n) * 1000;
        let (x, y) = (p.x + dx, p.y + dy);
        let ox = x < 0 || x >= l;
        let oy = y < 0 || y >= l;
        assert!(!(ox && oy), "split moves that cross two edges");
        if !ox && !oy {
            return SurfacePos { face: p.face, x, y };
        }
        let [normal, right, down] = AXES[usize::from(p.face)];
        let (axis, sign, parallel, along, across) = if ox {
            (
                right,
                if x < 0 { -1 } else { 1 },
                down,
                2 * y - l,
                2 * x - l,
            )
        } else {
            (
                down,
                if y < 0 { -1 } else { 1 },
                right,
                2 * x - l,
                2 * y - l,
            )
        };
        let next_normal = axis.map(|v| v * sign);
        let face = AXES.iter().position(|a| a[0] == next_normal).unwrap();
        let world = std::array::from_fn(|i| {
            next_normal[i] * l + parallel[i] * along + normal[i] * (2 * l - sign * across)
        });
        let result = SurfacePos {
            face: face as u8,
            x: (dot(world, AXES[face][1]) + l) / 2,
            y: (dot(world, AXES[face][2]) + l) / 2,
        };
        assert!(
            result.x >= 0 && result.x <= l && result.y >= 0 && result.y <= l,
            "split moves longer than a face"
        );
        // An exact edge belongs to the face just entered. Reflecting a half-open
        // interval can produce its closed end; use its nearest milli-tile.
        SurfacePos {
            x: result.x.min(l - 1),
            y: result.y.min(l - 1),
            ..result
        }
    }

    /// Local vector to a same/adjacent-face position within half a face width.
    /// Opposite faces and longer vectors have no local unfolding.
    pub fn unfold(self, from: SurfacePos, to: SurfacePos) -> Option<(i64, i64)> {
        self.index(from.tile());
        self.index(to.tile());
        let l = i64::from(self.n) * 1000;
        let (x, y) = if from.face == to.face {
            (to.x, to.y)
        } else {
            let [normal, right, down] = AXES[usize::from(from.face)];
            let [tn, tr, td] = AXES[usize::from(to.face)];
            if dot(normal, tn) != 0 {
                return None;
            }
            let world = std::array::from_fn(|i| {
                tn[i] * l + tr[i] * (2 * to.x - l) + td[i] * (2 * to.y - l)
            });
            let sx = dot(tn, right);
            if sx != 0 {
                (
                    (sx * (2 * l - dot(world, normal)) + l) / 2,
                    (dot(world, down) + l) / 2,
                )
            } else {
                (
                    (dot(world, right) + l) / 2,
                    (dot(tn, down) * (2 * l - dot(world, normal)) + l) / 2,
                )
            }
        };
        let (dx, dy) = (x - from.x, y - from.y);
        (dx * dx + dy * dy <= (l / 2).pow(2)).then_some((dx, dy))
    }

    /// Equal-angle cube mapping to a unit sphere scaled by 2^20.
    /// The interpolated Q30 tan table has absolute error < 2^-22 (Taylor
    /// remainder + fixed-point rounding + interpolation). Normalisation and
    /// output rounding give Cartesian error < 4 / 2^20 per axis, hence vector
    /// error < 8 / 2^20 of the radius, below 1/4096.
    pub fn sphere_point(self, pos: impl Into<SurfacePos>) -> [i32; 3] {
        let pos = pos.into();
        let l = i64::from(self.n) * 1000;
        let u = tan((2 * pos.x - l) * Q / l);
        let v = tan((2 * pos.y - l) * Q / l);
        let [normal, right, down] = AXES[usize::from(pos.face)];
        let raw: [i64; 3] = std::array::from_fn(|i| normal[i] * Q + right[i] * u + down[i] * v);
        let norm = (dot(raw, raw) as u64).isqrt() as i64;
        raw.map(|v| (v * i64::from(SPHERE_SCALE) / norm) as i32)
    }

    /// Inverse equal-angle mapping of a nonzero Cartesian direction to a tile.
    /// Face ties choose the lowest face id. Components need not be normalised.
    pub fn tile_at_point(self, point: [i32; 3]) -> Tile {
        assert!(point != [0; 3]);
        let p = point.map(i64::from);
        let face = (0..6)
            .max_by_key(|&f| (dot(p, AXES[f][0]), std::cmp::Reverse(f)))
            .unwrap();
        let normal = dot(p, AXES[face][0]);
        let coordinate = |axis| {
            let ratio = dot(p, axis) * Q / normal;
            let u = inverse_tan(ratio);
            ((u + Q) * i64::from(self.n) / (2 * Q)).clamp(0, i64::from(self.n) - 1) as u32
        };
        Tile {
            face: face as u8,
            x: coordinate(AXES[face][1]),
            y: coordinate(AXES[face][2]),
        }
    }
}

// Degree 11 sine / degree 12 cosine on [0, pi/4], evaluated in Q30.
// Alternating-series remainders are < 7e-12 and < 4e-13 respectively.
fn tan_entry(u: i64) -> i64 {
    const PI_OVER_4: i64 = 843_314_857;
    let x = u * PI_OVER_4 / Q;
    let xx = x * x / Q;
    let (mut sin, mut term) = (x, x);
    for k in 1..=5 {
        term = -term * xx / (Q * (2 * k) * (2 * k + 1));
        sin += term;
    }
    let (mut cos, mut term) = (Q, Q);
    for k in 1..=6 {
        term = -term * xx / (Q * (2 * k - 1) * (2 * k));
        cos += term;
    }
    sin * Q / cos
}

fn table() -> &'static [i64; TABLE_SIZE + 1] {
    static TABLE: OnceLock<[i64; TABLE_SIZE + 1]> = OnceLock::new();
    TABLE.get_or_init(|| std::array::from_fn(|i| tan_entry(i as i64 * Q / TABLE_SIZE as i64)))
}

fn tan(u: i64) -> i64 {
    let a = u.abs().min(Q);
    let scaled = a * TABLE_SIZE as i64;
    let i = (scaled / Q) as usize;
    let values = table();
    let value = if i == TABLE_SIZE {
        values[i]
    } else {
        values[i] + (values[i + 1] - values[i]) * (scaled % Q) / Q
    };
    value * u.signum()
}

fn inverse_tan(ratio: i64) -> i64 {
    let a = ratio.abs().min(Q);
    let t = table();
    let hi = t.partition_point(|&v| v < a).clamp(1, TABLE_SIZE);
    let fractional = (a - t[hi - 1]) * (Q / TABLE_SIZE as i64) / (t[hi] - t[hi - 1]);
    (((hi - 1) as i64 * Q / TABLE_SIZE as i64) + fractional) * ratio.signum()
}

#[cfg(test)]
mod tests;

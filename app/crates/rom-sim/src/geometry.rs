//! Surface geometry: the one place that knows whether the map is a flat row-major grid
//! or a cube sphere of six square faces. Every measurement, step and iteration of space
//! in the simulation goes through here.
//!
//! On the cube, an adjacent face is unfolded into a face's plane across their shared
//! edge (as `rom_geo::CubeSurface::unfold` does); vectors and distances exist only
//! within half a face width, and callers treat a missing vector as out of range.

use crate::{Pos, integer_sqrt};
use rom_geo::{CubeSurface, SurfacePos};
use std::ops::RangeInclusive;

/// The shape of the playing surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topology {
    /// A rectangle of tiles in row-major order; every position is on face 0.
    Flat {
        /// Width in tiles.
        width: u32,
        /// Height in tiles.
        height: u32,
    },
    /// Six square faces joined at their edges; tiles in the surface's dense index order.
    Cube(CubeSurface),
}

/// A tile coordinate on one face; x and y may lie off the face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Tile {
    pub face: u8,
    pub x: i32,
    pub y: i32,
}

impl Tile {
    pub const fn new(face: u8, x: i32, y: i32) -> Self {
        Self { face, x, y }
    }
}

/// How an adjacent face's local coordinates appear in a face's plane:
/// `rotate(p, turns) + shift`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frame {
    turns: u8,
    shift: (i64, i64),
}

/// A centre expressed in one face's own coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct View {
    pub face: u8,
    pub x: i64,
    pub y: i64,
}

/// Directions of the four face edges: right, left, down, up.
const EDGES: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Cube face axes (normal, local right, local down), as documented by `rom_geo`.
const AXES: [[[i64; 3]; 3]; 6] = [
    [[0, 0, 1], [1, 0, 0], [0, 1, 0]],
    [[1, 0, 0], [0, 0, -1], [0, 1, 0]],
    [[0, 0, -1], [-1, 0, 0], [0, 1, 0]],
    [[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
    [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
    [[0, 1, 0], [1, 0, 0], [0, 0, -1]],
];

/// Rotates a local vector by quarter turns; one turn adds 90 degrees of facing.
pub(crate) fn rotate(v: (i64, i64), turns: u8) -> (i64, i64) {
    match turns % 4 {
        0 => v,
        1 => (-v.1, v.0),
        2 => (-v.0, -v.1),
        _ => (v.1, -v.0),
    }
}

/// Facing after its frame rotates by `turns` quarter turns.
pub(crate) fn turn_facing(facing: u16, turns: u8) -> u16 {
    ((u32::from(facing) + 90 * u32::from(turns % 4)) % 360) as u16
}

/// Topology plus the tables derived from it once.
#[derive(Debug, Clone)]
pub(crate) struct Geometry {
    pub topology: Topology,
    /// Tiles per face row and column (flat: the map's width and height).
    width: i32,
    height: i32,
    /// Face edge length in milli-tiles (cube only).
    side: i64,
    /// `frames[f][g]`: face g's coordinates in face f's plane, for adjacent faces.
    frames: [[Option<Frame>; 6]; 6],
    /// `adjacent[f][edge]`: the face across each edge, in `EDGES` order.
    adjacent: [[u8; 4]; 6],
}

impl Geometry {
    /// Derives the edge frames from `rom_geo`'s unfolding; the cube needs n >= 4.
    pub fn new(topology: Topology) -> Option<Self> {
        let mut geometry = Self {
            topology,
            width: 0,
            height: 0,
            side: 0,
            frames: [[None; 6]; 6],
            adjacent: [[0; 4]; 6],
        };
        match topology {
            Topology::Flat { width, height } => {
                geometry.width = i32::try_from(width).ok()?;
                geometry.height = i32::try_from(height).ok()?;
            }
            Topology::Cube(surface) => {
                if !(4..=4096).contains(&surface.n) {
                    return None;
                }
                let n = surface.n as i32;
                geometry.width = n;
                geometry.height = n;
                geometry.side = i64::from(n) * 1000;
                for face in 0..6_u8 {
                    for (edge, (dx, dy)) in EDGES.into_iter().enumerate() {
                        let (b, frame) = edge_frame(surface, geometry.side, face, edge, dx, dy);
                        geometry.frames[usize::from(face)][usize::from(b)] = Some(frame);
                        geometry.adjacent[usize::from(face)][edge] = b;
                    }
                }
            }
        }
        Some(geometry)
    }

    pub fn is_flat(&self) -> bool {
        matches!(self.topology, Topology::Flat { .. })
    }

    /// Number of tiles on the whole surface.
    pub fn tiles(&self) -> usize {
        self.faces() * self.width as usize * self.height as usize
    }

    fn faces(&self) -> usize {
        if self.is_flat() { 1 } else { 6 }
    }

    /// Tiles per face row and column.
    pub fn face_size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Half a face width in milli-tiles: the cube's reach limit (flat: unlimited).
    pub fn reach_limit(&self) -> Option<i64> {
        (!self.is_flat()).then_some(self.side / 2)
    }

    /// Clamps a search or effect radius to the topology's reach.
    pub fn clamp_radius(&self, radius: i64) -> i64 {
        self.reach_limit().map_or(radius, |limit| radius.min(limit))
    }

    /// The tile containing a position.
    pub fn tile_of(&self, pos: Pos) -> Tile {
        Tile::new(pos.face, pos.x.div_euclid(1000), pos.y.div_euclid(1000))
    }

    /// The centre of a tile in milli-tiles.
    pub fn tile_centre(&self, tile: Tile) -> Pos {
        Pos {
            x: tile.x * 1000 + 500,
            y: tile.y * 1000 + 500,
            face: tile.face,
        }
    }

    /// The dense storage index of a tile, or None off the surface.
    pub fn index_of(&self, tile: Tile) -> Option<usize> {
        (usize::from(tile.face) < self.faces()
            && tile.x >= 0
            && tile.y >= 0
            && tile.x < self.width
            && tile.y < self.height)
            .then(|| self.index_unchecked(tile))
    }

    fn index_unchecked(&self, tile: Tile) -> usize {
        (usize::from(tile.face) * self.height as usize + tile.y as usize) * self.width as usize
            + tile.x as usize
    }

    /// The storage index of the tile under a position.
    pub fn pos_index(&self, pos: Pos) -> Option<usize> {
        self.index_of(self.tile_of(pos))
    }

    /// The tile stored at an index.
    pub fn tile_at(&self, index: usize) -> Tile {
        let (w, h) = (self.width as usize, self.height as usize);
        Tile::new(
            (index / (w * h)) as u8,
            (index % w) as i32,
            (index / w % h) as i32,
        )
    }

    /// The centre of the tile stored at an index.
    pub fn index_centre(&self, index: usize) -> Pos {
        self.tile_centre(self.tile_at(index))
    }

    /// Whether a position lies on the surface.
    pub fn contains(&self, pos: Pos) -> bool {
        self.pos_index(pos).is_some()
    }

    /// Clamps a tile onto the surface (flat: onto the map; cube: onto its face).
    pub fn clamp_tile(&self, tile: Tile) -> Tile {
        Tile::new(
            tile.face.min(self.faces() as u8 - 1),
            tile.x.clamp(0, self.width - 1),
            tile.y.clamp(0, self.height - 1),
        )
    }

    /// Face g's local coordinates seen in face f's plane.
    fn onto_plane(&self, f: u8, g: u8, p: (i64, i64)) -> Option<(i64, i64)> {
        if f == g {
            return Some(p);
        }
        let frame = self.frames[usize::from(f)][usize::from(g)]?;
        let r = rotate(p, frame.turns);
        Some((r.0 + frame.shift.0, r.1 + frame.shift.1))
    }

    /// A point of face f's plane in adjacent face g's own coordinates.
    fn out_of_plane(&self, f: u8, g: u8, p: (i64, i64)) -> (i64, i64) {
        let frame = self.frames[usize::from(f)][usize::from(g)].expect("adjacent faces");
        rotate(
            (p.0 - frame.shift.0, p.1 - frame.shift.1),
            (4 - frame.turns % 4) % 4,
        )
    }

    /// Quarter turns that carry a vector in face f's frame into adjacent face g's frame.
    pub fn turns(&self, f: u8, g: u8) -> u8 {
        if f == g {
            return 0;
        }
        self.frames[usize::from(f)][usize::from(g)].map_or(0, |frame| (4 - frame.turns % 4) % 4)
    }

    /// The local vector from `a` to `b` in `a`'s frame with no reach limit; None when
    /// the faces are neither equal nor adjacent.
    pub fn delta(&self, a: Pos, b: Pos) -> Option<(i64, i64)> {
        let p = self.onto_plane(a.face, b.face, (i64::from(b.x), i64::from(b.y)))?;
        Some((p.0 - i64::from(a.x), p.1 - i64::from(a.y)))
    }

    /// The local vector from `a` to `b` in `a`'s frame. On the cube it is the unfolded
    /// vector and None beyond half a face width.
    pub fn vector(&self, a: Pos, b: Pos) -> Option<(i64, i64)> {
        if self.is_flat() {
            return Some((
                i64::from(b.x) - i64::from(a.x),
                i64::from(b.y) - i64::from(a.y),
            ));
        }
        let v = self.delta(a, b)?;
        let limit = self.side / 2;
        (v.0 * v.0 + v.1 * v.1 <= limit * limit).then_some(v)
    }

    /// Squared distance in milli-tiles squared; None when out of reach.
    pub fn distance_squared(&self, a: Pos, b: Pos) -> Option<u128> {
        if self.is_flat() {
            return Some(a.distance_squared_wide(b));
        }
        let (dx, dy) = self.vector(a, b)?;
        Some((i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)) as u128)
    }

    /// Squared distance, or `u128::MAX` when out of reach; for nearest-first ordering.
    pub fn rank_distance(&self, a: Pos, b: Pos) -> u128 {
        self.distance_squared(a, b).unwrap_or(u128::MAX)
    }

    /// Distance rounded down to whole milli-tiles; None when out of reach.
    pub fn distance(&self, a: Pos, b: Pos) -> Option<u64> {
        self.distance_squared(a, b).map(integer_sqrt)
    }

    /// Ceiling of the distance, as straight-line movement measures it.
    pub fn ceil_distance(&self, a: Pos, b: Pos) -> Option<u64> {
        let squared = self.distance_squared(a, b)?;
        let root = integer_sqrt(squared);
        Some(root + u64::from(u128::from(root) * u128::from(root) < squared))
    }

    /// Whether two positions are at most `radius` apart; a negative radius reaches nothing.
    pub fn within(&self, a: Pos, b: Pos, radius: i64) -> bool {
        radius >= 0
            && self
                .distance_squared(a, b)
                .is_some_and(|d| d <= (radius as u128).pow(2))
    }

    /// Whether two positions are strictly less than `radius` apart.
    pub fn closer_than(&self, a: Pos, b: Pos, radius: i64) -> bool {
        radius > 0
            && self
                .distance_squared(a, b)
                .is_some_and(|d| d < (radius as u128).pow(2))
    }

    /// A conservative `within` for pre-filters: on the cube, radii beyond the reach
    /// limit cannot be measured and always pass.
    pub fn may_be_within(&self, a: Pos, b: Pos, radius: i64) -> bool {
        self.reach_limit().is_some_and(|limit| radius > limit) || self.within(a, b, radius)
    }

    /// Steps a position by a local vector, crossing face edges with rotation; returns the
    /// new position and the quarter turns its frame rotated by. Flat maps do not clip.
    /// On the cube the vector must be at most a face width long.
    pub fn offset(&self, pos: Pos, dx: i64, dy: i64) -> (Pos, u8) {
        if self.is_flat() {
            let clamp = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            return (
                Pos {
                    x: clamp(i64::from(pos.x) + dx),
                    y: clamp(i64::from(pos.y) + dy),
                    face: pos.face,
                },
                0,
            );
        }
        let l = self.side;
        let (mut face, mut x, mut y) = (pos.face, i64::from(pos.x), i64::from(pos.y));
        let (mut dx, mut dy) = (dx, dy);
        let mut turns = 0_u8;
        let outside = |v: i64| v < 0 || v >= l;
        // Each pass crosses one edge; passing a corner needs two.
        for _ in 0..4 {
            let (tx, ty) = (x + dx, y + dy);
            let (ox, oy) = (outside(tx), outside(ty));
            if !ox && !oy {
                return (
                    Pos {
                        x: tx as i32,
                        y: ty as i32,
                        face,
                    },
                    turns,
                );
            }
            // Split at the first edge reached so that each leg leaves on one axis only.
            let (lx, ly) = if ox && oy {
                let need_x = if dx > 0 { l - x } else { -1 - x };
                let need_y = if dy > 0 { l - y } else { -1 - y };
                // need_x / dx against need_y / dy; x first on ties.
                if i128::from(need_x.abs()) * i128::from(dy.abs())
                    <= i128::from(need_y.abs()) * i128::from(dx.abs())
                {
                    (need_x, (dy * need_x / dx).clamp(-y, l - 1 - y))
                } else {
                    ((dx * need_y / dy).clamp(-x, l - 1 - x), need_y)
                }
            } else {
                (dx, dy)
            };
            let (px, py) = (x + lx, y + ly);
            let edge = if outside(px) {
                if px >= l { 0 } else { 1 }
            } else if py >= l {
                2
            } else {
                3
            };
            let next = self.adjacent[usize::from(face)][edge];
            let (nx, ny) = self.out_of_plane(face, next, (px, py));
            let turn = self.turns(face, next);
            (dx, dy) = rotate((dx - lx, dy - ly), turn);
            turns = (turns + turn) % 4;
            face = next;
            // An exact edge belongs to the face just entered; keep the half-open range.
            x = nx.clamp(0, l - 1);
            y = ny.clamp(0, l - 1);
            if dx == 0 && dy == 0 {
                break;
            }
        }
        (
            Pos {
                x: x as i32,
                y: y as i32,
                face,
            },
            turns,
        )
    }

    /// The tile at local tile coordinates of face f's plane, mapped onto the surface:
    /// across one edge on the cube; None off a flat map and in a cube corner's gap.
    pub fn plane_index(&self, face: u8, x: i32, y: i32) -> Option<usize> {
        let (ox, oy) = (x < 0 || x >= self.width, y < 0 || y >= self.height);
        if !ox && !oy {
            return self.index_of(Tile::new(face, x, y));
        }
        if self.is_flat() || (ox && oy) {
            return None;
        }
        let edge = if ox {
            if x >= self.width { 0 } else { 1 }
        } else if y >= self.height {
            2
        } else {
            3
        };
        let next = self.adjacent[usize::from(face)][edge];
        let centre = (i64::from(x) * 1000 + 500, i64::from(y) * 1000 + 500);
        let (px, py) = self.out_of_plane(face, next, centre);
        self.index_of(Tile::new(
            next,
            px.div_euclid(1000) as i32,
            py.div_euclid(1000) as i32,
        ))
    }

    /// The 8-way neighbours of a tile index in the given order, or `u32::MAX` where none
    /// exists (off a flat map; a cube corner's missing diagonal).
    pub fn neighbours(&self, index: usize, order: &[(i32, i32); 8]) -> [u32; 8] {
        let tile = self.tile_at(index);
        let mut out = [u32::MAX; 8];
        let interior =
            tile.x > 0 && tile.y > 0 && tile.x < self.width - 1 && tile.y < self.height - 1;
        for (slot, (dx, dy)) in out.iter_mut().zip(order) {
            let (x, y) = (tile.x + dx, tile.y + dy);
            if interior {
                *slot = self.index_unchecked(Tile::new(tile.face, x, y)) as u32;
            } else if let Some(index) = self.plane_index(tile.face, x, y) {
                *slot = index as u32;
            }
        }
        out
    }

    /// A tile centre in space, in doubled milli-tiles: on the cube's surface, or on the
    /// plane z = 0 for a flat map. Straight-line distances here never exceed walked ones.
    pub fn space_point(&self, index: usize) -> [i64; 3] {
        let tile = self.tile_at(index);
        let (x, y) = (
            2 * (i64::from(tile.x) * 1000 + 500),
            2 * (i64::from(tile.y) * 1000 + 500),
        );
        if self.is_flat() {
            return [x, y, 0];
        }
        let l = self.side;
        let [normal, right, down] = AXES[usize::from(tile.face)];
        std::array::from_fn(|i| normal[i] * l + right[i] * (x - l) + down[i] * (y - l))
    }

    /// A centre seen from each face a box of half-size `reach` around it may cover
    /// (flat: one view). On the cube, `reach` should not exceed the reach limit.
    pub fn views(&self, centre: Pos, reach: i64) -> [Option<View>; 5] {
        let own = View {
            face: centre.face,
            x: i64::from(centre.x),
            y: i64::from(centre.y),
        };
        let mut out = [Some(own), None, None, None, None];
        if self.is_flat() {
            return out;
        }
        let l = self.side;
        let reach = reach.max(0);
        for (slot, next) in out[1..]
            .iter_mut()
            .zip(self.adjacent[usize::from(centre.face)])
        {
            let (x, y) = self
                .onto_plane(next, centre.face, (own.x, own.y))
                .expect("adjacent faces");
            if x + reach >= 0 && x - reach < l && y + reach >= 0 && y - reach < l {
                *slot = Some(View { face: next, x, y });
            }
        }
        out
    }

    /// Runs of tile indices whose centres lie within `radius` of `centre`, clipped to the
    /// surface. On the cube this is the unfolded distance test, tile by tile, with the
    /// radius clamped to the reach limit.
    pub fn disc(
        &self,
        centre: Pos,
        radius: i64,
    ) -> impl Iterator<Item = RangeInclusive<usize>> + use<> {
        let radius = self.clamp_radius(radius.max(-1));
        let (width, height) = (i64::from(self.width), i64::from(self.height));
        self.views(centre, radius)
            .into_iter()
            .flatten()
            .flat_map(move |view| {
                let base = usize::from(view.face) * (width * height) as usize;
                disc_runs(width, height, view.x, view.y, radius)
                    .map(move |run| base + run.start()..=base + run.end())
            })
    }

    /// Tiles covered by footprint bounds `[left, top, right, bottom]` on one face.
    pub fn footprint_tiles(
        &self,
        face: u8,
        bounds: [i64; 4],
    ) -> impl Iterator<Item = Tile> + use<> {
        (bounds[1].div_euclid(1000)..bounds[3].div_euclid(1000)).flat_map(move |y| {
            (bounds[0].div_euclid(1000)..bounds[2].div_euclid(1000))
                .map(move |x| Tile::new(face, x as i32, y as i32))
        })
    }

    /// Whether footprint bounds lie wholly on their face (flat: on the map).
    pub fn footprint_fits(&self, bounds: [i64; 4]) -> bool {
        bounds[0] >= 0
            && bounds[1] >= 0
            && bounds[2] <= i64::from(self.width) * 1000
            && bounds[3] <= i64::from(self.height) * 1000
    }

    /// Indices of the footprint and its neighbouring tiles, including diagonals, out to
    /// `reach` milli-tiles rounded up to whole tiles and at least one tile; tiles across
    /// a cube edge are included, tiles off a flat map are not.
    pub fn footprint_surroundings(
        &self,
        face: u8,
        bounds: [i64; 4],
        reach: i64,
    ) -> impl Iterator<Item = usize> + '_ {
        let ring = (reach.max(1) + 999).div_euclid(1000) * 1000;
        self.footprint_tiles(
            face,
            [
                bounds[0] - ring,
                bounds[1] - ring,
                bounds[2] + ring,
                bounds[3] + ring,
            ],
        )
        .filter_map(move |t| self.plane_index(face, t.x, t.y))
    }

    /// The point of footprint bounds on `face` closest to `pos`, and its squared distance
    /// from `pos`; None when `pos` is out of reach of the footprint's face.
    pub fn closest_in_bounds(&self, face: u8, bounds: [i64; 4], pos: Pos) -> Option<(Pos, u128)> {
        let (px, py) = self.onto_plane(face, pos.face, (i64::from(pos.x), i64::from(pos.y)))?;
        let (cx, cy) = (
            px.clamp(bounds[0], bounds[2]),
            py.clamp(bounds[1], bounds[3]),
        );
        let (dx, dy) = (i128::from(px - cx), i128::from(py - cy));
        let squared = (dx * dx + dy * dy) as u128;
        if let Some(limit) = self.reach_limit()
            && squared > (limit as u128).pow(2)
        {
            return None;
        }
        // On the cube a far edge of the face belongs to the next face; stay on this one.
        let (low, high) = match self.reach_limit() {
            Some(_) => (0, self.side - 1),
            None => (i64::from(i32::MIN), i64::from(i32::MAX)),
        };
        Some((
            Pos {
                x: cx.clamp(low, high) as i32,
                y: cy.clamp(low, high) as i32,
                face,
            },
            squared,
        ))
    }
}

/// The face across `edge` of `face` and the frame of its coordinates in `face`'s plane,
/// derived from two unfolded points.
fn edge_frame(
    surface: CubeSurface,
    side: i64,
    face: u8,
    edge: usize,
    dx: i32,
    dy: i32,
) -> (u8, Frame) {
    let mid = surface.n / 2;
    let (x, y) = match edge {
        0 => (surface.n - 1, mid),
        1 => (0, mid),
        2 => (mid, surface.n - 1),
        _ => (mid, 0),
    };
    let a = rom_geo::Tile { face, x, y };
    let b = surface
        .neighbor(a, dx, dy)
        .expect("edge midpoints have neighbours");
    let (pa, pb) = (SurfacePos::from(a), SurfacePos::from(b));
    let along = if pb.x + 1000 < side { 1000 } else { -1000 };
    let pb2 = SurfacePos {
        x: pb.x + along,
        ..pb
    };
    let v1 = surface.unfold(pa, pb).expect("adjacent tiles unfold");
    let v2 = surface.unfold(pa, pb2).expect("nearby tiles unfold");
    let image = ((v2.0 - v1.0) / along, (v2.1 - v1.1) / along);
    let turns = (0..4_u8)
        .find(|t| rotate((1, 0), *t) == image)
        .expect("edges join by quarter turns");
    let rotated = rotate((pb.x, pb.y), turns);
    (
        b.face,
        Frame {
            turns,
            shift: (pa.x + v1.0 - rotated.0, pa.y + v1.1 - rotated.1),
        },
    )
}

/// Runs of tile indices on one `width` × `height` grid whose centres lie within `radius`
/// of `(cx, cy)`, clipped to the grid.
fn disc_runs(
    width: i64,
    height: i64,
    cx: i64,
    cy: i64,
    radius: i64,
) -> impl Iterator<Item = RangeInclusive<usize>> {
    let squared = i128::from(radius.max(0)).pow(2);
    // Rows whose centre y lies within the radius: centre(y) = 1000·y + 500.
    let first = (cy - radius - 500 + 999).div_euclid(1000).max(0);
    let last = (cy + radius - 500).div_euclid(1000).min(height - 1);
    (first..=last).filter_map(move |y| {
        if radius < 0 {
            return None;
        }
        let dy = i128::from(y * 1000 + 500 - cy);
        let rest = squared - dy * dy;
        if rest < 0 {
            return None;
        }
        let half = integer_sqrt(rest as u128) as i64;
        let x0 = (cx - half - 500 + 999).div_euclid(1000).max(0);
        let x1 = (cx + half - 500).div_euclid(1000).min(width - 1);
        (x0 <= x1).then(|| {
            let row = (y * width) as usize;
            row + x0 as usize..=row + x1 as usize
        })
    })
}

/// Whether a position lies inside footprint bounds on a face, edges included.
pub(crate) fn footprint_contains(face: u8, bounds: [i64; 4], pos: Pos) -> bool {
    pos.face == face
        && (bounds[0]..=bounds[2]).contains(&i64::from(pos.x))
        && (bounds[1]..=bounds[3]).contains(&i64::from(pos.y))
}

#[cfg(test)]
mod tests;

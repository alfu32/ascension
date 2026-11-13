//! Sphere planets for drawing: the scene description, the globe geometry, chunk meshes and
//! the ray cast behind picking. Nothing here touches raylib.
//!
//! The planet is centred on the origin with y up. The cube-sphere points of `rom-geo` are
//! mirrored in y (`to_render`) so face 0 seen from outside has x to the right and y down, as
//! in the face net; vertices sit at tile corners, displaced radially by the tile heights.

use std::collections::BTreeMap;
use std::f32::consts::TAU;

use rom_geo::{CubeSurface, SPHERE_SCALE, SurfacePos, Tile};

pub use crate::vision::ChunkSpec;

use crate::entities::{Footing, GroundFrame};
use crate::terrain::{ChunkMesh, FLOOR_DEEP, RELIEF, Shape, TileKind, byte, cross, mix, normalize};

/// Largest chunk side in tiles on a globe: small enough that chunks behind the horizon or
/// outside the view are skipped in bulk (at most the 64 tiles that keep indices in 16 bits).
pub const GLOBE_CHUNK_TILES: u32 = 32;
const _: () = assert!(GLOBE_CHUNK_TILES <= 64);

/// A sphere planet as the renderer needs it: per-tile grids in cube-surface index order
/// (face, row, column), feature colours and marker positions.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetScene {
    /// Tiles along each face edge.
    pub n: u32,
    /// Kind per tile.
    pub kinds: Vec<TileKind>,
    /// Height 0..=255 per tile.
    pub heights: Vec<u8>,
    /// Colour (RGB) per feature id.
    pub feature_colors: BTreeMap<u16, [u8; 3]>,
    /// Player starts.
    pub starts: Vec<SurfacePos>,
    /// Resource nodes.
    pub nodes: Vec<SurfacePos>,
    /// How the light that represents the sun moves over this planet.
    pub sun: crate::sun::SunSettings,
}

impl PlanetScene {
    /// True when both grids hold exactly `6 n²` tiles and `n` is in `1..=4096`.
    pub fn is_valid(&self) -> bool {
        if !(1..=4096).contains(&self.n) {
            return false;
        }
        let tiles = 6 * self.n as usize * self.n as usize;
        self.kinds.len() == tiles && self.heights.len() == tiles
    }
}

/// Planet radius in tiles for a face of `n` tiles: the circumference is `4 n`.
pub fn planet_radius(n: u32) -> f32 {
    4.0 * n as f32 / TAU
}

/// Maps between the cube-sphere frame of `rom-geo` and the render frame (a mirror in y).
pub fn to_render(p: [f32; 3]) -> [f32; 3] {
    [p[0], -p[1], p[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(v: [f32; 3]) -> f32 {
    dot(v, v).sqrt()
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn along(dir: [f32; 3], length: f32) -> [f32; 3] {
    dir.map(|c| c * length)
}

/// Which tiles a chunk mesh covers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cover {
    Land,
    Water,
}

/// Where a ray met the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceHit {
    /// World position of the hit.
    pub point: [f32; 3],
    /// The surface position under it, in milli-tiles.
    pub pos: SurfacePos,
}

/// A bounding sphere around a chunk mesh, for culling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChunkBounds {
    /// Centre in planet space.
    pub centre: [f32; 3],
    /// Radius around the centre.
    pub radius: f32,
    /// Distance from the planet centre of the highest vertex.
    pub top: f32,
}

impl ChunkBounds {
    /// The sphere around the vertices of `mesh`.
    pub fn of(mesh: &ChunkMesh) -> Self {
        let mut low = [f32::MAX; 3];
        let mut high = [f32::MIN; 3];
        for p in &mesh.positions {
            for k in 0..3 {
                low[k] = low[k].min(p[k]);
                high[k] = high[k].max(p[k]);
            }
        }
        let centre = [0, 1, 2].map(|k| (low[k] + high[k]) / 2.0);
        let radius = mesh
            .positions
            .iter()
            .map(|&p| length(sub(p, centre)))
            .fold(0.0, f32::max);
        let top = mesh
            .positions
            .iter()
            .map(|&p| length(p))
            .fold(0.0, f32::max);
        Self {
            centre,
            radius,
            top,
        }
    }

    /// False when the sphere lies wholly beyond the horizon of a camera at `eye` over a planet
    /// of `radius` (with the relief of the ground taken into account), or outside the view
    /// pyramid of `view`; true when it might be seen. The view test is deliberately loose.
    pub fn may_be_seen(&self, planet_radius: f32, view: &CullView) -> bool {
        let eye = view.eye;
        let eye_radius = length(eye);
        // The sea-level sphere hides what lies below it; a high chunk shows over the horizon.
        let low = planet_radius.max(1e-3);
        if eye_radius > low {
            let horizon = (low / eye_radius).acos() + (low / self.top.max(low)).acos() + 0.02;
            let centre_radius = length(self.centre).max(1e-6);
            let spread = (self.radius / centre_radius).min(1.0).asin();
            let between = (dot(eye, self.centre) / (eye_radius * centre_radius)).clamp(-1.0, 1.0);
            if between.acos() - spread > horizon {
                return false;
            }
        }
        let d = sub(self.centre, eye);
        let z = dot(d, view.forward);
        if z < -self.radius {
            return false;
        }
        let beyond = |lateral: f32, tangent: f32| {
            lateral > tangent * z + self.radius * (1.0 + tangent * tangent).sqrt()
        };
        !(beyond(dot(d, view.right), view.tan_x)
            || beyond(-dot(d, view.right), view.tan_x)
            || beyond(dot(d, view.up), view.tan_y)
            || beyond(-dot(d, view.up), view.tan_y))
    }
}

/// The camera pyramid used for culling, with generous half-angle tangents.
#[derive(Clone, Copy, Debug)]
pub struct CullView {
    /// Eye position.
    pub eye: [f32; 3],
    /// Unit forward vector.
    pub forward: [f32; 3],
    /// Unit right vector.
    pub right: [f32; 3],
    /// Unit up vector.
    pub up: [f32; 3],
    /// Tangent of the half-angle across the view.
    pub tan_x: f32,
    /// Tangent of the half-angle up and down.
    pub tan_y: f32,
}

impl CullView {
    /// A pyramid for `orbit` on a canvas with the given aspect ratio and vertical field of view
    /// (degrees); `slack` widens it, for example to allow for a shifted projection centre.
    pub fn of_orbit(
        orbit: &crate::orbit::OrbitCamera,
        aspect: f32,
        fov_deg: f32,
        slack: f32,
    ) -> Self {
        let forward = orbit.forward();
        let right = normalize(cross(forward, orbit.view_up()));
        let up = cross(right, forward);
        let tan_y = (fov_deg.to_radians() / 2.0).tan() * slack;
        Self {
            eye: orbit.position(),
            forward,
            right,
            up,
            tan_x: tan_y * aspect,
            tan_y,
        }
    }
}

/// Geometry of a planet: vertex heights, colours, normals and the tile lookups.
#[derive(Clone, Debug)]
pub struct Globe {
    surface: CubeSurface,
    radius: f32,
    kinds: Vec<TileKind>,
    shape: Shape,
    /// Unit direction per face vertex, face / row / column order, `(n + 1)²` per face.
    dirs: Vec<[f32; 3]>,
    /// Height above the sea-level sphere per face vertex.
    heights: Vec<f32>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 3]>,
    tile_normals: Vec<[f32; 3]>,
}

impl Globe {
    /// Builds the geometry of `planet`; `planet` must be valid.
    pub fn new(planet: &PlanetScene) -> Self {
        assert!(planet.is_valid(), "inconsistent planet scene");
        let surface = CubeSurface { n: planet.n };
        let n = planet.n;
        let side = n as usize + 1;
        let shape = Shape::from_tiles(&planet.kinds, &planet.heights, &planet.feature_colors);
        let radius = planet_radius(n);
        let vertices = 6 * side * side;
        let mut globe = Self {
            surface,
            radius,
            kinds: planet.kinds.clone(),
            shape,
            dirs: Vec::with_capacity(vertices),
            heights: Vec::with_capacity(vertices),
            normals: Vec::new(),
            colors: Vec::with_capacity(vertices),
            tile_normals: Vec::new(),
        };
        for face in 0..6_u8 {
            for vy in 0..=n {
                for vx in 0..=n {
                    let point = surface.sphere_point(SurfacePos {
                        face,
                        x: i64::from(vx) * 1000,
                        y: i64::from(vy) * 1000,
                    });
                    let unit = to_render(point.map(|c| c as f32 / SPHERE_SCALE as f32));
                    globe.dirs.push(normalize(unit));
                    let (tiles, count) = globe.adjoining(face, vx, vy);
                    let mut height = 0.0;
                    let mut color = [0.0; 3];
                    for &t in &tiles[..count] {
                        height += globe.shape.elevation[t];
                        let c = globe.shape.color[t];
                        color = add(color, c);
                    }
                    let inverse = 1.0 / count.max(1) as f32;
                    globe.heights.push(height * inverse);
                    globe.colors.push(along(color, inverse));
                }
            }
        }
        globe.tile_normals = (0..surface.len())
            .map(|i| globe.compute_tile_normal(surface.tile(i)))
            .collect();
        globe.normals = (0..vertices)
            .map(|i| {
                let (face, vy, vx) = (i / (side * side), i / side % side, i % side);
                let (tiles, count) = globe.adjoining(face as u8, vx as u32, vy as u32);
                let sum = tiles[..count]
                    .iter()
                    .fold([0.0; 3], |sum, &t| add(sum, globe.tile_normals[t]));
                normalize(sum)
            })
            .collect();
        globe
    }

    /// Tiles along each face edge.
    pub fn n(&self) -> u32 {
        self.surface.n
    }

    /// The cube surface the globe is laid on.
    pub fn surface(&self) -> CubeSurface {
        self.surface
    }

    /// Planet radius, the sea level, in tiles.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Dense indices (ascending, so every face sums them in the same order) of the one to four
    /// tiles around the corner (`vx`, `vy`) of `face`, looking across face edges.
    fn adjoining(&self, face: u8, vx: u32, vy: u32) -> ([usize; 4], usize) {
        let n = i64::from(self.surface.n);
        let mut out = [0; 4];
        let mut count = 0;
        for (ox, oy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
            let (x, y) = (i64::from(vx) + ox, i64::from(vy) + oy);
            let (cx, cy) = (x.clamp(0, n - 1), y.clamp(0, n - 1));
            let base = Tile {
                face,
                x: cx as u32,
                y: cy as u32,
            };
            let tile = if (cx, cy) == (x, y) {
                Some(base)
            } else {
                self.surface
                    .neighbor(base, (x - cx) as i32, (y - cy) as i32)
            };
            if let Some(tile) = tile {
                let index = self.surface.index(tile);
                if !out[..count].contains(&index) {
                    out[count] = index;
                    count += 1;
                }
            }
        }
        out[..count].sort_unstable();
        (out, count)
    }

    fn vertex_index(&self, face: u8, vx: u32, vy: u32) -> usize {
        let side = self.surface.n as usize + 1;
        (usize::from(face) * side + vy as usize) * side + vx as usize
    }

    /// Height of the corner (`vx`, `vy`) of `face` above sea level, in tiles.
    pub fn vertex_height(&self, face: u8, vx: u32, vy: u32) -> f32 {
        self.heights[self.vertex_index(face, vx, vy)]
    }

    /// Position of the displaced corner (`vx`, `vy`) of `face`.
    pub fn vertex(&self, face: u8, vx: u32, vy: u32) -> [f32; 3] {
        let i = self.vertex_index(face, vx, vy);
        along(self.dirs[i], self.radius + self.heights[i])
    }

    fn compute_tile_normal(&self, tile: Tile) -> [f32; 3] {
        let at = |dx, dy| self.vertex(tile.face, tile.x + dx, tile.y + dy);
        let (c00, c10, c01, c11) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
        // The two triangles of the tile, in the order of the index list.
        let first = cross(sub(c01, c00), sub(c10, c00));
        let second = cross(sub(c01, c10), sub(c11, c10));
        add(first, second)
    }

    /// Outward normal of the displaced tile `tile` (the sum of its two triangle normals, unit length).
    pub fn tile_normal(&self, tile: Tile) -> [f32; 3] {
        normalize(self.tile_normals[self.surface.index(tile)])
    }

    /// The tile under the direction `dir` from the planet centre.
    pub fn tile_of(&self, dir: [f32; 3]) -> Tile {
        let unit = normalize(dir);
        let scale = SPHERE_SCALE as f32;
        let geo = to_render(unit).map(|c| (c * scale).round() as i32);
        self.surface.tile_at_point(geo)
    }

    /// Height of the ground (or of the sea surface over water) under `dir`, in tiles.
    pub fn ground_height(&self, dir: [f32; 3]) -> f32 {
        let tile = self.tile_of(dir);
        self.shape.elevation[self.surface.index(tile)].max(0.0)
    }

    /// The point on the surface above the centre of `tile`.
    pub fn tile_point(&self, tile: Tile) -> [f32; 3] {
        let dir = self.dir_of(SurfacePos::from(tile));
        along(dir, self.radius + self.ground_height(dir))
    }

    /// Unit direction of a surface position.
    pub fn dir_of(&self, pos: SurfacePos) -> [f32; 3] {
        let unit = self
            .surface
            .sphere_point(pos)
            .map(|c| c as f32 / SPHERE_SCALE as f32);
        normalize(to_render(unit))
    }

    /// The surface position under the direction `dir`, to the milli-tile: the tile of
    /// `tile_at_point` refined by a local linear solve inside that tile.
    pub fn pos_of(&self, dir: [f32; 3]) -> SurfacePos {
        let tile = self.tile_of(dir);
        let centre = SurfacePos::from(tile);
        let at = |dx: i64, dy: i64| {
            self.dir_of(SurfacePos {
                face: tile.face,
                x: centre.x + dx,
                y: centre.y + dy,
            })
        };
        let origin = at(0, 0);
        let ex = sub(at(500, 0), at(-500, 0));
        let ey = sub(at(0, 500), at(0, -500));
        let offset = sub(normalize(dir), origin);
        let (xx, xy, yy) = (dot(ex, ex), dot(ex, ey), dot(ey, ey));
        let (bx, by) = (dot(offset, ex), dot(offset, ey));
        let det = xx * yy - xy * xy;
        let (a, b) = if det.abs() > 1e-12 {
            ((bx * yy - by * xy) / det, (by * xx - bx * xy) / det)
        } else {
            (0.0, 0.0)
        };
        let clamp = |centre: i64, shift: f32| {
            let low = centre - 500;
            (centre + (shift * 1000.0).round() as i64).clamp(low, low + 999)
        };
        SurfacePos {
            face: tile.face,
            x: clamp(centre.x, a),
            y: clamp(centre.y, b),
        }
    }

    /// The surface position of face-local tile coordinates (`x`, `y`) in tiles, clamped onto the face.
    fn clamp_pos(&self, face: u8, x: f32, y: f32) -> SurfacePos {
        let limit = i64::from(self.surface.n) * 1000 - 1;
        SurfacePos {
            face,
            x: ((x * 1000.0).round() as i64).clamp(0, limit),
            y: ((y * 1000.0).round() as i64).clamp(0, limit),
        }
    }

    /// Height above sea level at face-local tile coordinates, never below zero: the corner
    /// heights interpolated over the tile's two triangles, split as in the meshes.
    pub fn surface_height(&self, face: u8, x: f32, y: f32) -> f32 {
        let n = self.surface.n;
        let top = n as f32 - 1e-3;
        let (x, y) = (x.clamp(0.0, top), y.clamp(0.0, top));
        let (tx, ty) = (x.floor(), y.floor());
        let (u, v) = (x - tx, y - ty);
        let (vx, vy) = (tx as u32, ty as u32);
        let h00 = self.vertex_height(face, vx, vy);
        let h10 = self.vertex_height(face, vx + 1, vy);
        let h01 = self.vertex_height(face, vx, vy + 1);
        let h11 = self.vertex_height(face, vx + 1, vy + 1);
        let height = if u + v <= 1.0 {
            h00 + u * (h10 - h00) + v * (h01 - h00)
        } else {
            h11 + (1.0 - u) * (h01 - h11) + (1.0 - v) * (h10 - h11)
        };
        height.max(0.0)
    }

    /// The point `lift` tiles above the ground at face-local tile coordinates.
    pub fn world_point(&self, face: u8, x: f32, y: f32, lift: f32) -> [f32; 3] {
        let dir = self.dir_of(self.clamp_pos(face, x, y));
        along(dir, self.radius + self.surface_height(face, x, y) + lift)
    }

    /// The local frame on the ground at face-local tile coordinates: `up` is the radial
    /// direction, `ex` and `ey` follow the face's x and y axes (so `ex × up = ey`).
    pub fn ground_frame(&self, face: u8, x: f32, y: f32) -> GroundFrame {
        let pos = self.clamp_pos(face, x, y);
        let up = self.dir_of(pos);
        let limit = i64::from(self.surface.n) * 1000 - 1;
        // The east axis follows a one-tile step along the face's x (backwards at the far edge).
        let ahead = pos.x + 1000 <= limit;
        let other = self.dir_of(SurfacePos {
            x: if ahead { pos.x + 1000 } else { pos.x - 1000 },
            ..pos
        });
        let tangent = |v: [f32; 3]| normalize(sub(v, along(up, dot(v, up))));
        let ex = tangent(if ahead {
            sub(other, up)
        } else {
            sub(up, other)
        });
        let ey = cross(ex, up);
        GroundFrame {
            origin: along(up, self.radius + self.surface_height(face, x, y)),
            ex,
            up,
            ey,
        }
    }

    /// Moves a surface position by a local vector in milli-tiles across at most one face edge;
    /// at a corner the part that would cross a second edge is dropped.
    pub fn step_on(&self, from: SurfacePos, dx: i64, dy: i64) -> SurfacePos {
        let limit = i64::from(self.surface.n) * 1000;
        let outside = |v: i64| !(0..limit).contains(&v);
        let dx = if outside(from.x + dx) && outside(from.y + dy) {
            (from.x + dx).clamp(0, limit - 1) - from.x
        } else {
            dx
        };
        self.surface.step(from, dx, dy)
    }

    /// Where the ray from `eye` along the unit vector `ray` first meets the displaced surface:
    /// the ray is intersected with the sphere around the highest ground, marched until it is
    /// below the ground, then refined by bisection. `None` when it misses the planet.
    pub fn raycast(&self, eye: [f32; 3], ray: [f32; 3]) -> Option<SurfaceHit> {
        let top = self.radius + RELIEF + 0.01;
        let b = dot(eye, ray);
        let c = dot(eye, eye) - top * top;
        let discriminant = b * b - c;
        if discriminant < 0.0 {
            return None;
        }
        let root = discriminant.sqrt();
        let (enter, leave) = (-b - root, -b + root);
        if leave < 0.0 {
            return None;
        }
        let at = |t: f32| add(eye, along(ray, t));
        let above = |t: f32| {
            let p = at(t);
            let length = dot(p, p).sqrt().max(1e-6);
            length - self.radius - self.ground_height(p.map(|c| c / length))
        };
        let mut t = enter.max(0.0);
        let (mut near, mut far) = if above(t) <= 0.0 {
            (t, t)
        } else {
            loop {
                let next = t + (t * 0.01).max(0.25);
                if next > leave {
                    return None;
                }
                if above(next) <= 0.0 {
                    break (t, next);
                }
                t = next;
            }
        };
        for _ in 0..30 {
            let mid = (near + far) / 2.0;
            if above(mid) > 0.0 {
                near = mid;
            } else {
                far = mid;
            }
        }
        let point = at((near + far) / 2.0);
        let dir = normalize(point);
        let point = along(dir, self.radius + self.ground_height(dir));
        Some(SurfaceHit {
            point,
            pos: self.pos_of(dir),
        })
    }

    /// Land meshes, at most 64 × 64 tiles each, on every face.
    pub fn land_chunks(&self) -> Vec<ChunkMesh> {
        self.land_chunks_with_specs()
            .into_iter()
            .map(|(mesh, _)| mesh)
            .collect()
    }

    /// Sea meshes at sea level over the water tiles, tinted and made opaque by depth.
    pub fn sea_chunks(&self) -> Vec<ChunkMesh> {
        self.sea_chunks_with_specs()
            .into_iter()
            .map(|(mesh, _)| mesh)
            .collect()
    }

    /// [`Self::land_chunks`] with the tile window each mesh covers.
    pub fn land_chunks_with_specs(&self) -> Vec<(ChunkMesh, ChunkSpec)> {
        self.chunks(Cover::Land)
    }

    /// [`Self::sea_chunks`] with the tile window each mesh covers.
    pub fn sea_chunks_with_specs(&self) -> Vec<(ChunkMesh, ChunkSpec)> {
        self.chunks(Cover::Water)
    }

    /// The tile indices touching the corner (`vx`, `vy`) of `face`, looking across face edges.
    pub fn corner_tiles(&self, face: u8, vx: u32, vy: u32) -> ([usize; 4], usize) {
        self.adjoining(face, vx, vy)
    }

    fn chunks(&self, cover: Cover) -> Vec<(ChunkMesh, ChunkSpec)> {
        let n = self.surface.n;
        let mut chunks = Vec::new();
        for face in 0..6_u8 {
            for y0 in (0..n).step_by(GLOBE_CHUNK_TILES as usize) {
                for x0 in (0..n).step_by(GLOBE_CHUNK_TILES as usize) {
                    let (w, h) = (GLOBE_CHUNK_TILES.min(n - x0), GLOBE_CHUNK_TILES.min(n - y0));
                    if let Some(mesh) = self.chunk(face, x0, y0, w, h, cover) {
                        chunks.push((mesh, ChunkSpec { face, x0, y0, w, h }));
                    }
                }
            }
        }
        chunks
    }

    fn chunk(&self, face: u8, x0: u32, y0: u32, w: u32, h: u32, cover: Cover) -> Option<ChunkMesh> {
        let stride = w as usize + 1;
        let mut indices = Vec::new();
        for j in 0..h {
            for i in 0..w {
                let tile = Tile {
                    face,
                    x: x0 + i,
                    y: y0 + j,
                };
                let covered = match (cover, self.kinds[self.surface.index(tile)]) {
                    (Cover::Land, kind) => kind != TileKind::Space,
                    (Cover::Water, kind) => {
                        matches!(kind, TileKind::ShallowSea | TileKind::DeepSea)
                    }
                };
                if !covered {
                    continue;
                }
                let v00 = (j as usize * stride + i as usize) as u16;
                let v10 = v00 + 1;
                let v01 = v00 + stride as u16;
                let v11 = v01 + 1;
                indices.extend_from_slice(&[v00, v01, v10, v10, v01, v11]);
            }
        }
        if indices.is_empty() {
            return None;
        }
        let vertices = stride * (h as usize + 1);
        let mut mesh = ChunkMesh {
            positions: Vec::with_capacity(vertices),
            normals: Vec::new(),
            colors: Vec::with_capacity(vertices),
            indices,
        };
        for j in 0..=h {
            for i in 0..=w {
                let index = self.vertex_index(face, x0 + i, y0 + j);
                let dir = self.dirs[index];
                match cover {
                    Cover::Land => {
                        mesh.positions
                            .push(along(dir, self.radius + self.heights[index]));
                        mesh.normals.push(self.normals[index]);
                        let c = self.colors[index];
                        mesh.colors.push([byte(c[0]), byte(c[1]), byte(c[2]), 255]);
                    }
                    Cover::Water => {
                        let depth = (-self.heights[index] / FLOOR_DEEP).clamp(0.0, 1.0);
                        let c = mix([0.22, 0.62, 0.74], [0.03, 0.14, 0.38], depth);
                        mesh.positions.push(along(dir, self.radius));
                        mesh.colors.push([
                            byte(c[0]),
                            byte(c[1]),
                            byte(c[2]),
                            byte(0.35 + 0.5 * depth),
                        ]);
                    }
                }
            }
        }
        Some(mesh)
    }

    /// Instances (column-major 4×4 with the tint in cells 3, 7 and 11) of the start pillars and
    /// node boxes, standing along the local up. `scale` grows them so they stay visible from far
    /// away.
    pub fn marker_instances(&self, planet: &PlanetScene, scale: f32, out: &mut Vec<[f32; 16]>) {
        out.clear();
        let mut push = |pos: SurfacePos, size: [f32; 3], color: [f32; 3]| {
            let dir = self.dir_of(pos);
            let ground = self.radius + self.ground_height(dir);
            let (north, east) = crate::orbit::tangent_frame(dir);
            let size = size.map(|s| s * scale);
            let centre = along(dir, ground + size[1] / 2.0);
            let (x, y, z) = (
                along(east, size[0]),
                along(dir, size[1]),
                along(north, size[2]),
            );
            out.push([
                x[0], x[1], x[2], color[0], y[0], y[1], y[2], color[1], z[0], z[1], z[2], color[2],
                centre[0], centre[1], centre[2], 1.0,
            ]);
        };
        for (i, &start) in planet.starts.iter().enumerate() {
            let color = crate::entities::owner_color((i % 8) as u8 + 1);
            push(start, [0.5, 8.0, 0.5], color);
        }
        for &node in &planet.nodes {
            push(node, [0.8, 0.8, 0.8], crate::entities::NODE_COLOR);
        }
    }
}

impl Footing for Globe {
    fn height_at(&self, face: u8, x: f32, y: f32) -> f32 {
        self.surface_height(face, x, y)
    }

    fn frame(&self, face: u8, x: f32, y: f32) -> GroundFrame {
        self.ground_frame(face, x, y)
    }

    fn point(&self, face: u8, x: f32, y: f32, delta: (f32, f32), lift: f32) -> [f32; 3] {
        let from = self.clamp_pos(face, x, y);
        let to = self.step_on(
            from,
            (delta.0 * 1000.0).round() as i64,
            (delta.1 * 1000.0).round() as i64,
        );
        self.world_point(to.face, to.x as f32 / 1000.0, to.y as f32 / 1000.0, lift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbit::OrbitCamera;
    use crate::pick::View;
    use std::collections::HashMap;

    /// A planet with a patchwork of kinds and heights.
    fn planet(n: u32) -> PlanetScene {
        let tiles = 6 * (n * n) as usize;
        let kinds = (0..tiles)
            .map(|i| match i % 7 {
                1 => TileKind::DeepSea,
                2 => TileKind::ShallowSea,
                3 => TileKind::Feature(9),
                _ => TileKind::Land,
            })
            .collect();
        PlanetScene {
            n,
            kinds,
            heights: (0..tiles).map(|i| (i * 37 % 251) as u8).collect(),
            feature_colors: BTreeMap::from([(9, [10, 200, 30])]),
            starts: vec![SurfacePos::from(Tile {
                face: 0,
                x: 3,
                y: 4,
            })],
            nodes: vec![SurfacePos::from(Tile {
                face: 2,
                x: 5,
                y: 1,
            })],
            sun: Default::default(),
        }
    }

    /// A planet of level ground: every tile at sea level.
    fn level_planet(n: u32) -> PlanetScene {
        let tiles = 6 * (n * n) as usize;
        PlanetScene {
            n,
            kinds: vec![TileKind::Land; tiles],
            heights: vec![100; tiles],
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
            sun: Default::default(),
        }
    }

    #[test]
    fn globe_vertices_lie_on_radius_plus_height() {
        let planet = planet(70);
        let globe = Globe::new(&planet);
        let r = planet_radius(70);
        let mut checked = 0;
        for chunk in globe.land_chunks() {
            for (p, nrm) in chunk.positions.iter().zip(&chunk.normals).step_by(53) {
                let radial = length(*p);
                assert!(
                    (-FLOOR_DEEP - 1e-3..=RELIEF + 1e-3).contains(&(radial - r)),
                    "vertex at {radial}"
                );
                assert!((length(*nrm) - 1.0).abs() < 1e-4);
                // The surface is bulging outward: the normal agrees with the radial direction.
                assert!(dot(*nrm, *p) > 0.0);
                checked += 1;
            }
        }
        assert!(checked > 100);
        // Exact check against the vertex heights, on each face.
        for face in 0..6 {
            for (vx, vy) in [(0, 0), (13, 7), (70, 70), (35, 0), (69, 31)] {
                let p = globe.vertex(face, vx, vy);
                let expected = r + globe.vertex_height(face, vx, vy);
                assert!((length(p) - expected).abs() < 1e-3);
            }
        }
        // Level ground sits exactly on the sea-level sphere, and the sea chunks too.
        let level = Globe::new(&level_planet(40));
        let r = planet_radius(40);
        for chunk in level.land_chunks() {
            for p in &chunk.positions {
                assert!((length(*p) - r).abs() < 1e-3);
            }
        }
        let sea = Globe::new(&planet);
        for chunk in sea.sea_chunks() {
            for p in &chunk.positions {
                assert!((length(*p) - planet_radius(70)).abs() < 1e-3);
            }
        }
    }

    /// The vertex at the far end of a face's row, as the corner of face `face`.
    fn corner(globe: &Globe, face: u8, vx: u32, vy: u32) -> [f32; 3] {
        globe.vertex(face, vx, vy)
    }

    #[test]
    fn neighbouring_chunks_share_edge_vertices() {
        let n = 16;
        let globe = Globe::new(&planet(n));
        // Across an edge: every vertex of face 0's right edge (x = n) equals a vertex of the
        // neighbouring face, found by position, and has the same height and normal.
        let mut by_position: HashMap<[u32; 3], Vec<(u8, u32, u32)>> = HashMap::new();
        for face in 0..6_u8 {
            for vy in 0..=n {
                for vx in 0..=n {
                    let p = globe.vertex(face, vx, vy);
                    by_position
                        .entry(p.map(f32::to_bits))
                        .or_default()
                        .push((face, vx, vy));
                }
            }
        }
        let mut shared = 0;
        let mut corners = 0;
        for vertices in by_position.values() {
            if vertices.len() > 1 {
                shared += 1;
            }
            if vertices.len() == 3 {
                corners += 1;
            }
            assert!(vertices.len() <= 3);
        }
        // Edge vertices: 12 edges with n + 1 vertices, shared by two faces; 8 cube corners by three.
        assert_eq!(corners, 8);
        assert_eq!(shared, 12 * (n as usize - 1) + 8);
        assert_eq!(
            by_position.len(),
            6 * (n as usize + 1).pow(2) - 12 * (n as usize - 1) - 16
        );
        // The corner (n, n) of face 0 is a cube corner; heights and normals agree on all three faces.
        let reference = corner(&globe, 0, n, n);
        let on_corner: Vec<_> = by_position[&reference.map(f32::to_bits)].clone();
        assert_eq!(on_corner.len(), 3);
        for (face, vx, vy) in &on_corner {
            let i = globe.vertex_index(*face, *vx, *vy);
            let j = globe.vertex_index(0, n, n);
            assert_eq!(globe.heights[i].to_bits(), globe.heights[j].to_bits());
            assert_eq!(
                globe.normals[i].map(f32::to_bits),
                globe.normals[j].map(f32::to_bits)
            );
            assert_eq!(
                globe.colors[i].map(f32::to_bits),
                globe.colors[j].map(f32::to_bits)
            );
        }
        // The chunk meshes contain those exact positions: edge vertices of two chunks coincide.
        let chunks = globe.land_chunks();
        let mut seen: HashMap<[u32; 3], usize> = HashMap::new();
        for chunk in &chunks {
            for p in &chunk.positions {
                *seen.entry(p.map(f32::to_bits)).or_default() += 1;
            }
        }
        assert!(seen.contains_key(&reference.map(f32::to_bits)));
        assert!(seen[&reference.map(f32::to_bits)] >= 3);
    }

    #[test]
    fn neighbouring_chunks_share_chunk_boundaries_on_a_large_face() {
        // 100 tiles per face: chunk boundaries every 32 tiles inside a face.
        let globe = Globe::new(&planet(100));
        let chunks = globe.land_chunks();
        assert_eq!(chunks.len(), 6 * 16);
        let mut seen: HashMap<[u32; 3], usize> = HashMap::new();
        for chunk in &chunks {
            for p in &chunk.positions {
                *seen.entry(p.map(f32::to_bits)).or_default() += 1;
            }
        }
        let boundary = globe.vertex(1, 64, 20);
        assert_eq!(seen[&boundary.map(f32::to_bits)], 2);
        let interior = globe.vertex(1, 30, 20);
        assert_eq!(seen[&interior.map(f32::to_bits)], 1);
    }

    #[test]
    fn chunk_index_counts_fit_16_bits() {
        let globe = Globe::new(&planet(130));
        let land = globe.land_chunks();
        assert_eq!(land.len(), 6 * 5 * 5);
        for chunk in land.iter().chain(&globe.sea_chunks()) {
            assert!(chunk.positions.len() <= usize::from(u16::MAX) + 1);
            assert!(chunk.indices.len().is_multiple_of(3));
            assert!(
                chunk
                    .indices
                    .iter()
                    .all(|&i| usize::from(i) < chunk.positions.len())
            );
            assert_eq!(chunk.colors.len(), chunk.positions.len());
        }
        for chunk in &land {
            assert_eq!(chunk.normals.len(), chunk.positions.len());
            // Counter-clockwise seen from outside: each triangle's normal points away from the centre.
            for t in chunk.indices.as_chunks::<3>().0.iter().step_by(7) {
                let [a, b, c] = t.map(|i| chunk.positions[usize::from(i)]);
                let normal = cross(sub(b, a), sub(c, a));
                let centre = add(add(a, b), c);
                assert!(dot(normal, centre) > 0.0, "triangle faces inward");
            }
        }
    }

    #[test]
    fn tile_lookup_is_consistent_with_the_geometry() {
        let globe = Globe::new(&planet(48));
        for (face, x, y) in [(0, 0, 0), (1, 47, 47), (2, 10, 30), (4, 24, 24), (5, 0, 47)] {
            let tile = Tile { face, x, y };
            let dir = globe.dir_of(SurfacePos::from(tile));
            assert_eq!(globe.tile_of(dir), tile);
            let pos = globe.pos_of(dir);
            assert_eq!(pos.tile(), tile);
            assert!((pos.x - SurfacePos::from(tile).x).abs() <= 2);
            assert!((pos.y - SurfacePos::from(tile).y).abs() <= 2);
        }
        // On level ground the tile normal is the radial direction.
        let level = Globe::new(&level_planet(48));
        let tile = Tile {
            face: 2,
            x: 7,
            y: 40,
        };
        let dir = level.dir_of(SurfacePos::from(tile));
        assert!(dot(level.tile_normal(tile), dir) > 0.999);
        // A point a quarter tile to the right of a tile centre resolves to that offset.
        let tile = Tile {
            face: 3,
            x: 20,
            y: 20,
        };
        let centre = SurfacePos::from(tile);
        let shifted = SurfacePos {
            x: centre.x + 250,
            y: centre.y - 300,
            ..centre
        };
        let pos = globe.pos_of(globe.dir_of(shifted));
        assert!((pos.x - shifted.x).abs() <= 3 && (pos.y - shifted.y).abs() <= 3);
    }

    fn view_of(orbit: &OrbitCamera) -> View {
        View::from_orbit(orbit, (1280.0, 720.0), 0.0)
    }

    #[test]
    fn ray_from_camera_hits_the_surface_at_the_view_direction() {
        let globe = Globe::new(&level_planet(64));
        let r = globe.radius();
        for (lon, lat, distance, heading) in [
            (0.0, 0.0, 400.0, 0.0),
            (35.0, 20.0, 60.0, 40.0),
            (200.0, -50.0, 12.0, 130.0),
            (90.0, 80.0, 25.0, 10.0),
        ] {
            let orbit = OrbitCamera::at(lon, lat, distance, heading, r, 3.0 * r);
            let view = view_of(&orbit);
            let ray = view.ray(640.0, 360.0);
            let hit = globe.raycast(view.eye(), ray).expect("the centre ray hits");
            assert!(length(sub(hit.point, orbit.target())) < 0.1, "{hit:?}");
            assert!((length(hit.point) - r).abs() < 1e-3);
            assert_eq!(hit.pos, globe.pos_of(orbit.dir));
        }
        // A ray above the planet misses it.
        let orbit = OrbitCamera::at(0.0, 0.0, 3.0 * r, 0.0, r, 3.0 * r);
        let view = view_of(&orbit);
        assert!(globe.raycast(view.eye(), view.ray(2.0, 2.0)).is_none());
    }

    #[test]
    fn pick_returns_the_tile_under_the_cursor() {
        let globe = Globe::new(&planet(96));
        let r = globe.radius();
        // From high above (looking straight down) and from a mid view, round-trip tile centres
        // of every face, including the edges and corners of the cube.
        for (distance, centre_y) in [(3.0 * r, 0.0), (45.0, 0.0), (45.0, 0.3)] {
            for face in 0..6_u8 {
                for (x, y) in [(0, 0), (95, 95), (0, 95), (48, 48), (95, 40), (20, 0)] {
                    let tile = Tile { face, x, y };
                    let target = globe.dir_of(SurfacePos::from(tile));
                    let (lon, lat) = crate::orbit::lon_lat_of(target);
                    let orbit = OrbitCamera::at(
                        lon.to_degrees(),
                        lat.to_degrees(),
                        distance,
                        25.0,
                        r,
                        3.0 * r,
                    );
                    let view = View::from_orbit(&orbit, (1280.0, 720.0), centre_y);
                    let (px, py, _) = view.project(globe.tile_point(tile)).expect("in front");
                    let hit = globe
                        .raycast(view.eye(), view.ray(px, py))
                        .unwrap_or_else(|| panic!("no hit for {tile:?}"));
                    assert_eq!(hit.pos.tile(), tile, "distance {distance}");
                    assert_eq!(hit.pos.face, face);
                }
            }
        }
    }

    #[test]
    fn hidden_chunks_are_culled() {
        let globe = Globe::new(&planet(192));
        let r = globe.radius();
        let chunks = globe.land_chunks();
        let bounds: Vec<ChunkBounds> = chunks.iter().map(ChunkBounds::of).collect();
        let seen = |orbit: &OrbitCamera| {
            let view = CullView::of_orbit(orbit, 16.0 / 9.0, 45.0, 1.5);
            bounds.iter().filter(|b| b.may_be_seen(r, &view)).count()
        };
        // From far above, a bit under half of the planet is in view.
        let globe_view = OrbitCamera::at(0.0, 0.0, 3.0 * r, 0.0, r, 3.0 * r);
        let count = seen(&globe_view);
        assert!(
            count < bounds.len() * 6 / 10 && count > bounds.len() / 5,
            "{count}"
        );
        // A chunk on the far side is culled, the chunk under the camera is not.
        let under = globe.vertex(0, 96, 96);
        let far_side = globe.vertex(2, 96, 96);
        let view = CullView::of_orbit(&globe_view, 16.0 / 9.0, 45.0, 1.5);
        let sphere = |centre: [f32; 3], radius| ChunkBounds {
            centre,
            radius,
            top: length(centre) + radius,
        };
        assert!(sphere(under, 10.0).may_be_seen(r, &view));
        assert!(!sphere(far_side, 10.0).may_be_seen(r, &view));
        // At street level only the surroundings count, and everything checked by picking a
        // visible tile stays visible.
        let street = OrbitCamera::at(0.0, 0.0, 8.0, 0.0, r, 3.0 * r);
        assert!(seen(&street) < bounds.len() / 4);
        let view = CullView::of_orbit(&street, 16.0 / 9.0, 45.0, 1.5);
        let ahead = globe.vertex(0, 96, 88);
        assert!(sphere(ahead, 2.0).may_be_seen(r, &view));
        let behind = globe.vertex(0, 96, 112);
        assert!(!sphere(behind, 2.0).may_be_seen(r, &view));
    }

    #[test]
    fn markers_stand_along_the_local_up() {
        let planet = planet(32);
        let globe = Globe::new(&planet);
        let mut out = Vec::new();
        globe.marker_instances(&planet, 1.0, &mut out);
        assert_eq!(out.len(), 2);
        let pillar = out[0];
        let up = normalize([pillar[4], pillar[5], pillar[6]]);
        let centre = [pillar[12], pillar[13], pillar[14]];
        assert!(dot(up, normalize(centre)) > 0.9999);
        assert!(length([pillar[4], pillar[5], pillar[6]]) - 8.0 < 1e-4);
    }
}

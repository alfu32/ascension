//! Feature objects baked before the first tick: forest canopies, forest edges with trunks, rock
//! massifs and crop rows. Each generator builds a few small variant meshes from its `mesh` row's
//! parameters and the map seed; the feature tiles then get one instance each (a transform on
//! the tile's ground frame), grouped by object chunk so the renderer culls and draws them in
//! bulk. Nothing here touches raylib; the same seed always bakes the same result.

use std::collections::{BTreeMap, VecDeque};
use std::f32::consts::TAU;

use crate::entities::{EntityView, Footing, GroundFrame, Surface};
use crate::globe::Globe;
use crate::looks::{Generator, ObjectLook, Program, SceneLooks};
use crate::terrain::{ChunkMesh, TileKind};
use crate::vision::ChunkSpec;

/// Side of an object chunk in tiles: at most one instance per tile and object kind, so a chunk
/// holds at most `OBJECT_CHUNK_TILES²` instances.
pub const OBJECT_CHUNK_TILES: u32 = 16;
/// Largest number of instances a chunk can hold.
pub const MAX_CHUNK_INSTANCES: usize = (OBJECT_CHUNK_TILES * OBJECT_CHUNK_TILES) as usize;
/// Clump depth (tiles from the clump's edge) at which a massif reaches its full height.
const MASSIF_DEPTH: u32 = 3;
/// Share of a massif's inner tiles that carry a peak.
const MASSIF_PEAK_SHARE: f32 = 0.4;

/// One placed object: which variant mesh, on which tile, and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FeatureInstance {
    /// Index into [`BakedFeatures::meshes`].
    pub mesh: u16,
    /// Dense tile index (cube-surface order on a sphere, row-major on a flat map).
    pub tile: u32,
    /// Column-major transform; cells 3, 7 and 11 carry the tint (brightness variation).
    pub transform: [f32; 16],
}

/// The instances on one window of tiles, with a sphere around them for culling.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureChunk {
    /// Tile window the instances stand on.
    pub spec: ChunkSpec,
    /// Centre of the bounding sphere.
    pub centre: [f32; 3],
    /// Radius of the bounding sphere.
    pub radius: f32,
    /// The instances, in tile order.
    pub instances: Vec<FeatureInstance>,
}

/// What a bake produced: the variant meshes and the placed instances.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BakedFeatures {
    /// Variant meshes in the object's local frame (x east, y up, z south), tiles as units.
    /// Vertex alpha is the [`Program`] id of the part.
    pub meshes: Vec<ChunkMesh>,
    /// The same variants with fewer triangles, for instances far from the eye; same indices.
    pub far_meshes: Vec<ChunkMesh>,
    /// Snow per mesh: the height above sea level from which it shows and its colour.
    pub snow: Vec<Option<(f32, [f32; 3])>>,
    /// Instances grouped by object chunk, in face / row / column order.
    pub chunks: Vec<FeatureChunk>,
    /// Variant meshes of buildings drawn as objects, by entity type.
    pub buildings: BTreeMap<u16, Vec<u16>>,
}

impl BakedFeatures {
    /// Total number of placed instances.
    pub fn instance_count(&self) -> usize {
        self.chunks.iter().map(|c| c.instances.len()).sum()
    }

    /// Total number of vertices in the variant meshes.
    pub fn vertex_count(&self) -> usize {
        self.meshes.iter().map(|m| m.positions.len()).sum()
    }
}

/// A grid of tiles with kinds and neighbours: a sphere planet or a flat map.
pub(crate) trait TileGrid {
    /// Number of tiles.
    fn tile_count(&self) -> usize;
    /// Kind of tile `i`.
    fn kind(&self, i: usize) -> TileKind;
    /// Face and face-local column and row of tile `i`.
    fn coords(&self, i: usize) -> (u8, u32, u32);
    /// The tile `dx`, `dy` steps from tile `i`, across face edges on a sphere.
    fn neighbour(&self, i: usize, dx: i32, dy: i32) -> Option<usize>;
    /// Tiles along each side of a face: (columns, rows).
    fn face_size(&self) -> (u32, u32);
}

impl TileGrid for Globe {
    fn tile_count(&self) -> usize {
        self.surface().len()
    }

    fn kind(&self, i: usize) -> TileKind {
        self.kinds()[i]
    }

    fn coords(&self, i: usize) -> (u8, u32, u32) {
        let tile = self.surface().tile(i);
        (tile.face, tile.x, tile.y)
    }

    fn neighbour(&self, i: usize, dx: i32, dy: i32) -> Option<usize> {
        let surface = self.surface();
        surface
            .neighbor(surface.tile(i), dx, dy)
            .map(|t| surface.index(t))
    }

    fn face_size(&self) -> (u32, u32) {
        (self.n(), self.n())
    }
}

impl TileGrid for Surface {
    fn tile_count(&self) -> usize {
        self.scene().kinds.len()
    }

    fn kind(&self, i: usize) -> TileKind {
        self.scene().kinds[i]
    }

    fn coords(&self, i: usize) -> (u8, u32, u32) {
        let width = self.scene().width as usize;
        (0, (i % width) as u32, (i / width) as u32)
    }

    fn neighbour(&self, i: usize, dx: i32, dy: i32) -> Option<usize> {
        let scene = self.scene();
        let (_, x, y) = self.coords(i);
        let (x, y) = (i64::from(x) + i64::from(dx), i64::from(y) + i64::from(dy));
        let inside =
            (0..i64::from(scene.width)).contains(&x) && (0..i64::from(scene.height)).contains(&y);
        inside.then(|| (y * i64::from(scene.width) + x) as usize)
    }

    fn face_size(&self) -> (u32, u32) {
        (self.scene().width, self.scene().height)
    }
}

/// The eight neighbour steps.
const AROUND: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Whether tile `i` lies at the edge of its terrain: one of its eight neighbours is missing or
/// of another kind.
pub(crate) fn is_edge(grid: &dyn TileGrid, i: usize) -> bool {
    let kind = grid.kind(i);
    AROUND.iter().any(|&(dx, dy)| {
        grid.neighbour(i, dx, dy)
            .is_none_or(|j| grid.kind(j) != kind)
    })
}

/// Steps from each tile of a clump to the clump's edge (0 on edge tiles), up to `cap`; tiles of
/// kinds without objects get 0.
fn clump_depths(grid: &dyn TileGrid, wanted: &dyn Fn(TileKind) -> bool, cap: u32) -> Vec<u32> {
    let count = grid.tile_count();
    let mut depth = vec![u32::MAX; count];
    let mut queue = VecDeque::new();
    for (i, d) in depth.iter_mut().enumerate() {
        let kind = grid.kind(i);
        if !wanted(kind) {
            *d = 0;
        } else if is_edge(grid, i) {
            *d = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let next = depth[i] + 1;
        if next > cap {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(j) = grid.neighbour(i, dx, dy)
                && depth[j] == u32::MAX
            {
                depth[j] = next;
                queue.push_back(j);
            }
        }
    }
    depth.iter().map(|&d| d.min(cap)).collect()
}

/// A small deterministic generator (SplitMix64).
#[derive(Clone, Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64, salt: u64) -> Self {
        Self(mix64(
            seed ^ mix64(salt.wrapping_add(0x5851_f42d_4c95_7f2d)),
        ))
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        mix64(self.0)
    }

    /// Uniform in 0..1.
    pub(crate) fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1_u64 << 24) as f32
    }

    /// Uniform in `low..high`.
    pub(crate) fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    /// Uniform in `0..bound` (0 when `bound` is 0).
    fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            0
        } else {
            (self.next() % u64::from(bound)) as u32
        }
    }
}

fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Smooth value noise in 0..1 on a lattice hashed with `seed`.
fn value_noise(seed: u64, p: [f32; 3]) -> f32 {
    let cell = p.map(f32::floor);
    let f = [0, 1, 2].map(|k| {
        let t = p[k] - cell[k];
        t * t * (3.0 - 2.0 * t)
    });
    let corner = |dx: i64, dy: i64, dz: i64| {
        let key = (cell[0] as i64 + dx).wrapping_mul(73_856_093)
            ^ (cell[1] as i64 + dy).wrapping_mul(19_349_663)
            ^ (cell[2] as i64 + dz).wrapping_mul(83_492_791);
        (mix64(seed ^ key as u64) >> 40) as f32 / (1_u64 << 24) as f32
    };
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), f[0]);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), f[0]);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), f[0]);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), f[0]);
    lerp(lerp(x00, x10, f[1]), lerp(x01, x11, f[1]), f[2])
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|c| c * s)
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length < 1e-9 {
        [0.0, 1.0, 0.0]
    } else {
        scale(v, 1.0 / length)
    }
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|k| a[k] + (b[k] - a[k]) * t)
}

fn rgba(c: [f32; 3], program: Program) -> [u8; 4] {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    [byte(c[0]), byte(c[1]), byte(c[2]), program.id() as u8]
}

/// Collects triangles and computes smooth normals from them.
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    colors: Vec<[u8; 4]>,
    indices: Vec<u16>,
    /// Vertices whose normal is given rather than averaged.
    fixed: Vec<Option<[f32; 3]>>,
}

impl MeshBuilder {
    fn vertex(&mut self, p: [f32; 3], colour: [u8; 4], normal: Option<[f32; 3]>) -> u16 {
        self.positions.push(p);
        self.colors.push(colour);
        self.fixed.push(normal);
        (self.positions.len() - 1) as u16
    }

    /// Adds a triangle wound so its face normal points along `outward`.
    fn triangle(&mut self, a: u16, b: u16, c: u16, outward: [f32; 3]) {
        let [pa, pb, pc] = [a, b, c].map(|i| self.positions[usize::from(i)]);
        if dot(cross(sub(pb, pa), sub(pc, pa)), outward) >= 0.0 {
            self.indices.extend_from_slice(&[a, b, c]);
        } else {
            self.indices.extend_from_slice(&[a, c, b]);
        }
    }

    fn finish(self) -> ChunkMesh {
        let mut sums = vec![[0.0_f32; 3]; self.positions.len()];
        for t in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = [t[0], t[1], t[2]].map(usize::from);
            let n = cross(
                sub(self.positions[b], self.positions[a]),
                sub(self.positions[c], self.positions[a]),
            );
            for v in [a, b, c] {
                sums[v] = add(sums[v], n);
            }
        }
        let normals = sums
            .iter()
            .zip(&self.fixed)
            .map(|(sum, fixed)| fixed.unwrap_or_else(|| normalize(*sum)))
            .collect();
        ChunkMesh {
            positions: self.positions,
            normals,
            colors: self.colors,
            indices: self.indices,
        }
    }
}

/// Unit icosahedron (12 directions, 20 triangles), subdivided once when `fine` (42, 80).
fn icosphere(fine: bool) -> (Vec<[f32; 3]>, Vec<[usize; 3]>) {
    let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let mut dirs: Vec<[f32; 3]> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .into_iter()
    .map(normalize)
    .collect();
    let faces = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    if !fine {
        return (dirs, faces.to_vec());
    }
    let mut midpoints: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut midpoint = |a: usize, b: usize, dirs: &mut Vec<[f32; 3]>| {
        let key = (a.min(b), a.max(b));
        *midpoints.entry(key).or_insert_with(|| {
            dirs.push(normalize(add(dirs[a], dirs[b])));
            dirs.len() - 1
        })
    };
    let mut out = Vec::with_capacity(80);
    for [a, b, c] in faces {
        let ab = midpoint(a, b, &mut dirs);
        let bc = midpoint(b, c, &mut dirs);
        let ca = midpoint(c, a, &mut dirs);
        out.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
    }
    (dirs, out)
}

/// Adds a lumpy foliage clump: an icosphere scaled by `radii` and displaced by noise.
fn add_clump(
    mesh: &mut MeshBuilder,
    rng: &mut Rng,
    centre: [f32; 3],
    radii: [f32; 3],
    colour: [f32; 3],
    program: Program,
    fine: bool,
) {
    let (dirs, faces) = icosphere(fine);
    let noise_seed = rng.next();
    let first = mesh.positions.len() as u16;
    for d in &dirs {
        let lump = 0.78 + 0.42 * value_noise(noise_seed, scale(*d, 2.3));
        let p = [0, 1, 2].map(|k| centre[k] + d[k] * radii[k] * lump);
        // Lighter on top, darker underneath: light through the leaves.
        let shade = 0.72 + 0.34 * (d[1] * 0.5 + 0.5) + 0.1 * (lump - 1.0);
        mesh.vertex(p, rgba(scale(colour, shade), program), None);
    }
    for [a, b, c] in faces {
        let outward = add(add(dirs[a], dirs[b]), dirs[c]);
        let outward = [
            outward[0] * radii[0],
            outward[1] * radii[1],
            outward[2] * radii[2],
        ];
        mesh.triangle(
            first + a as u16,
            first + b as u16,
            first + c as u16,
            outward,
        );
    }
}

/// Adds a tapered six-sided trunk from below the ground to `top`, flat-shaded.
fn add_trunk(mesh: &mut MeshBuilder, base: [f32; 3], top: f32, radius: f32, colour: [f32; 3]) {
    const SIDES: usize = 6;
    let ring = |k: usize, r: f32, y: f32| {
        let a = k as f32 / SIDES as f32 * TAU;
        [base[0] + r * a.cos(), y, base[2] + r * a.sin()]
    };
    for k in 0..SIDES {
        let (a0, a1) = (ring(k, radius, base[1]), ring(k + 1, radius, base[1]));
        let (b0, b1) = (ring(k, radius * 0.6, top), ring(k + 1, radius * 0.6, top));
        let mid = (k as f32 + 0.5) / SIDES as f32 * TAU;
        let outward = [mid.cos(), 0.15, mid.sin()];
        let normal = normalize(outward);
        let shade = 0.85 + 0.15 * ((k % 2) as f32);
        let c = rgba(scale(colour, shade), Program::Bark);
        let v = [a0, a1, b1, b0].map(|p| mesh.vertex(p, c, Some(normal)));
        mesh.triangle(v[0], v[1], v[2], outward);
        mesh.triangle(v[0], v[2], v[3], outward);
    }
}

/// A closed canopy for a tile inside a forest: `count` overlapping clumps reaching beyond the
/// tile so neighbouring canopies merge.
fn forest_centre(look: &ObjectLook, rng: &mut Rng, fine: bool) -> ChunkMesh {
    let mut mesh = MeshBuilder::default();
    let count = look.count.max(1);
    let colours = [look.material.colour, look.material.colour_alt];
    // A low fill keeps the ground from showing between the clumps.
    add_clump(
        &mut mesh,
        rng,
        [0.0, look.height * 0.45, 0.0],
        [0.62, look.height * 0.42, 0.62],
        mix3(colours[0], colours[1], 0.15),
        look.material.program,
        fine,
    );
    for i in 0..count {
        let angle = (i as f32 + rng.range(-0.3, 0.3)) / count as f32 * TAU;
        let spread = rng.range(0.15, 0.32);
        let r = look.radius * rng.range(0.85, 1.15);
        let centre = [
            angle.cos() * spread,
            look.height - r * 0.75 + rng.range(-0.12, 0.08),
            angle.sin() * spread,
        ];
        let tone = mix3(colours[0], colours[1], rng.range(0.0, 0.55));
        add_clump(
            &mut mesh,
            rng,
            centre,
            [r, r * 0.82, r],
            tone,
            look.material.program,
            fine,
        );
    }
    mesh.finish()
}

/// A tile at the edge of a forest: `count` trees whose trunks show under their crowns.
fn forest_edge(look: &ObjectLook, rng: &mut Rng, fine: bool) -> ChunkMesh {
    let mut mesh = MeshBuilder::default();
    let (trunk_height, bark) = look
        .trunk
        .unwrap_or((look.height * 0.4, [0.36, 0.26, 0.19]));
    let count = look.count.max(1);
    let colours = [look.material.colour, look.material.colour_alt];
    for i in 0..count {
        let angle = (i as f32 + rng.range(-0.25, 0.25)) / count as f32 * TAU + rng.range(0.0, 1.0);
        let spread = rng.range(0.12, 0.3);
        let (x, z) = (angle.cos() * spread, angle.sin() * spread);
        let size = rng.range(0.8, 1.15);
        let trunk = trunk_height * size;
        let r = look.radius * size;
        add_trunk(&mut mesh, [x, -0.25, z], trunk + r * 0.5, 0.06 * size, bark);
        let crown = (look.height * size - trunk).max(r * 1.2);
        let tone = mix3(colours[0], colours[1], rng.range(0.0, 0.6));
        add_clump(
            &mut mesh,
            rng,
            [x, trunk + crown * 0.5, z],
            [r, crown * 0.55, r],
            tone,
            look.material.program,
            fine,
        );
    }
    mesh.finish()
}

/// A ridged rock peak of unit radius and height (the instance scales it); its rim sinks below
/// the ground so peaks on slopes leave no gaps.
fn rock_peak(look: &ObjectLook, rng: &mut Rng, fine: bool) -> ChunkMesh {
    let (rings_count, segments) = if fine { (8, 18) } else { (4, 9) };
    let mut mesh = MeshBuilder::default();
    let seed = rng.next();
    let ridges = look.count.max(2) as f32;
    let phase = rng.range(0.0, TAU);
    let apex_shift = [rng.range(-0.12, 0.12), rng.range(-0.12, 0.12)];
    let colours = [look.material.colour, look.material.colour_alt];
    let program = look.material.program;
    let height_at = |r: f32, a: f32| -> f32 {
        let p = [r * a.cos() * 2.0, 0.0, r * a.sin() * 2.0];
        let wobble = value_noise(seed, add(p, [0.0, 3.1, 0.0])) - 0.5;
        let ridge = 1.0 - (((ridges * a + phase) / 2.0 + wobble * 1.2).sin()).abs();
        let profile = (1.0 - r.powf(1.1)).max(0.0).powf(1.6);
        let rough = value_noise(seed, scale(p, 2.7)) - 0.5;
        // Ridges run down the flanks; the summit stays one peak.
        let flank = ((r - 0.1) / 0.4).clamp(0.0, 1.0);
        profile * (0.85 + 0.55 * (ridge * ridge - 0.45) * flank) + rough * 0.1 * flank
    };
    let colour_at = |y: f32, a: f32, r: f32| {
        let strata = value_noise(seed, [r * a.cos() * 4.0, y * 6.0, r * a.sin() * 4.0]);
        mix3(
            colours[0],
            colours[1],
            (strata * 0.8 + (1.0 - y) * 0.2).clamp(0.0, 1.0),
        )
    };
    let apex = mesh.vertex(
        [apex_shift[0], height_at(0.0, 0.0), apex_shift[1]],
        rgba(colours[0], program),
        None,
    );
    let mut rings = Vec::with_capacity(rings_count);
    for ring in 1..=rings_count {
        let r = ring as f32 / rings_count as f32;
        let mut ids = Vec::with_capacity(segments);
        for s in 0..segments {
            let a = s as f32 / segments as f32 * TAU;
            let jitter = 1.0 + (value_noise(seed, [a.cos() * 1.7, 9.0, a.sin() * 1.7]) - 0.5) * 0.3;
            let rr = r * jitter;
            let y = if ring == rings_count {
                -0.18
            } else {
                height_at(r, a)
            };
            let pull = 1.0 - r;
            let p = [
                rr * a.cos() + apex_shift[0] * pull,
                y,
                rr * a.sin() + apex_shift[1] * pull,
            ];
            ids.push(mesh.vertex(p, rgba(colour_at(y, a, r), program), None));
        }
        rings.push(ids);
    }
    let up = |p: [f32; 3]| [p[0], 0.6, p[2]];
    for s in 0..segments {
        let t = (s + 1) % segments;
        let (a, b) = (rings[0][s], rings[0][t]);
        let outward = up(mesh.positions[usize::from(a)]);
        mesh.triangle(apex, a, b, outward);
    }
    for ring in 1..rings_count {
        for s in 0..segments {
            let t = (s + 1) % segments;
            let (a, b) = (rings[ring - 1][s], rings[ring - 1][t]);
            let (c, d) = (rings[ring][s], rings[ring][t]);
            let outward = up(mesh.positions[usize::from(c)]);
            mesh.triangle(a, c, d, outward);
            mesh.triangle(a, d, b, outward);
        }
    }
    mesh.finish()
}

/// One tile of one crop row along local x: wide blades leaning to both sides of the row.
fn crop_row(look: &ObjectLook, rng: &mut Rng) -> ChunkMesh {
    const LEVELS: usize = 4;
    let mut mesh = MeshBuilder::default();
    let blades = look.blades.max(1);
    let colours = [look.material.colour, look.material.colour_alt];
    for i in 0..blades * 2 {
        let x = -0.5 + (i as f32 + rng.range(0.1, 0.9)) / (blades * 2) as f32;
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        let height = look.height * rng.range(0.75, 1.1);
        let width = look.blade_width * rng.range(0.8, 1.2);
        let turn = rng.range(-0.6, 0.6);
        // The blade's flat side faces along the row, turned a little; it leans across the row.
        let flat = [turn.cos(), 0.0, turn.sin() * 0.4];
        let lean = [0.0, 0.0, side * rng.range(0.08, 0.22)];
        let ripe = rng.range(0.0, 0.4);
        let mut ids = Vec::with_capacity(LEVELS * 2);
        for level in 0..LEVELS {
            let t = level as f32 / (LEVELS - 1) as f32;
            let half = width * 0.5 * (1.0 - t * 0.85);
            let bend = t * t;
            let centre = [
                x + lean[0] * bend,
                height * t * (1.0 - 0.15 * bend),
                lean[2] * bend * height,
            ];
            let tone = mix3(colours[0], colours[1], (t * 0.7 + ripe).min(1.0));
            let c = rgba(scale(tone, 0.75 + 0.25 * t), Program::Crop);
            ids.push(mesh.vertex(sub(centre, scale(flat, half)), c, None));
            ids.push(mesh.vertex(add(centre, scale(flat, half)), c, None));
        }
        let facing = normalize([-flat[2], 0.3, flat[0]]);
        for level in 0..LEVELS - 1 {
            let k = level * 2;
            mesh.triangle(ids[k], ids[k + 1], ids[k + 3], facing);
            mesh.triangle(ids[k], ids[k + 3], ids[k + 2], facing);
        }
    }
    let mut baked = mesh.finish();
    // Thin blades are lit like the ground they grow from, seen from either side.
    for n in &mut baked.normals {
        *n = normalize([n[0] * 0.35, 1.0, n[2] * 0.35]);
    }
    baked
}

/// Bakes the `variants` meshes of `look`, seeded by `seed` and `salt`.
/// Bakes the `variants` meshes of `look`, seeded by `seed` and `salt`, each as a near and a
/// far (coarser) mesh of the same shape.
fn bake_variants(look: &ObjectLook, seed: u64, salt: u64) -> Vec<(ChunkMesh, ChunkMesh)> {
    (0..look.variants.clamp(1, 16))
        .map(|v| {
            let build = |fine: bool| {
                let mut rng = Rng::new(seed, salt * 64 + u64::from(v));
                match look.generator {
                    Generator::ForestCentre => forest_centre(look, &mut rng, fine),
                    Generator::ForestEdge => forest_edge(look, &mut rng, fine),
                    Generator::RockMassif => rock_peak(look, &mut rng, fine),
                    Generator::CropRows => crop_row(look, &mut rng),
                }
            };
            (build(true), build(false))
        })
        .collect()
}

/// Column-major transform with axes `x`, `y`, `z` (already scaled), origin `o` and a grey tint.
fn transform(x: [f32; 3], y: [f32; 3], z: [f32; 3], o: [f32; 3], tint: f32) -> [f32; 16] {
    [
        x[0], x[1], x[2], tint, y[0], y[1], y[2], tint, z[0], z[1], z[2], tint, o[0], o[1], o[2],
        1.0,
    ]
}

/// The axes of `frame` turned by `angle` about its up vector.
fn turned(frame: &GroundFrame, angle: f32) -> ([f32; 3], [f32; 3]) {
    let (s, c) = angle.sin_cos();
    (
        add(scale(frame.ex, c), scale(frame.ey, s)),
        add(scale(frame.ey, c), scale(frame.ex, -s)),
    )
}

/// Which object of a tile look a tile gets.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Centre,
    Edge,
}

/// Bakes the feature objects of a sphere planet.
pub fn bake_planet(globe: &Globe, looks: &SceneLooks) -> BakedFeatures {
    bake(globe, globe, looks)
}

/// Bakes the feature objects of a flat map.
pub fn bake_flat(surface: &Surface, looks: &SceneLooks) -> BakedFeatures {
    bake(surface, surface, looks)
}

fn bake(grid: &dyn TileGrid, ground: &dyn Footing, looks: &SceneLooks) -> BakedFeatures {
    let mut baked = BakedFeatures::default();
    // Variant meshes of every object look, in the order of the look table.
    let mut variants: BTreeMap<(usize, Part), (Vec<u16>, ObjectLook)> = BTreeMap::new();
    let add_meshes = |baked: &mut BakedFeatures, look: &ObjectLook, salt: u64| {
        let first = baked.meshes.len();
        for (near, far) in bake_variants(look, looks.seed, salt) {
            baked.meshes.push(near);
            baked.far_meshes.push(far);
            baked
                .snow
                .push(look.snow.map(|s| (s.line, s.material.colour)));
        }
        (first..baked.meshes.len())
            .map(|i| i as u16)
            .collect::<Vec<_>>()
    };
    for (index, (_, tile_look)) in looks.tiles.iter().enumerate() {
        for (part, object) in [
            (Part::Centre, tile_look.centre),
            (Part::Edge, tile_look.edge),
        ] {
            if let Some(object) = object {
                let salt = index as u64 * 2 + u64::from(part == Part::Edge);
                let ids = add_meshes(&mut baked, &object, salt);
                variants.insert((index, part), (ids, object));
            }
        }
    }
    for (&kind, look) in &looks.buildings {
        let ids = add_meshes(&mut baked, look, 1_000_000 + u64::from(kind));
        baked.buildings.insert(kind, ids);
    }
    let look_of = |kind: TileKind| looks.tiles.iter().position(|(k, _)| *k == kind);
    let has_objects = |kind: TileKind| {
        look_of(kind).is_some_and(|i| {
            variants.contains_key(&(i, Part::Centre)) || variants.contains_key(&(i, Part::Edge))
        })
    };
    let massif = |kind: TileKind| {
        look_of(kind).is_some_and(|i| {
            [Part::Centre, Part::Edge].iter().any(|part| {
                variants
                    .get(&(i, *part))
                    .is_some_and(|(_, l)| l.generator == Generator::RockMassif)
            })
        })
    };
    let depths = clump_depths(grid, &massif, MASSIF_DEPTH);
    let (columns, rows) = grid.face_size();
    let side = OBJECT_CHUNK_TILES;
    let mut chunks: BTreeMap<(u8, u32, u32), Vec<FeatureInstance>> = BTreeMap::new();
    for (i, &depth) in depths.iter().enumerate() {
        let kind = grid.kind(i);
        if !has_objects(kind) {
            continue;
        }
        let Some(look_index) = look_of(kind) else {
            continue;
        };
        let edge = is_edge(grid, i);
        let part = if edge { Part::Edge } else { Part::Centre };
        let Some((ids, object)) = variants
            .get(&(look_index, part))
            .or_else(|| variants.get(&(look_index, Part::Centre)))
            .or_else(|| variants.get(&(look_index, Part::Edge)))
        else {
            continue;
        };
        let (face, x, y) = grid.coords(i);
        let mut rng = Rng::new(looks.seed, 0x7f4a_0000_0000 + i as u64);
        let mesh = ids[rng.below(ids.len() as u32) as usize];
        let angle = rng.range(0.0, TAU);
        let jitter = if edge { 0.12 } else { 0.08 };
        let (jx, jy) = (rng.range(-jitter, jitter), rng.range(-jitter, jitter));
        let frame = ground.frame(face, x as f32 + 0.5 + jx, y as f32 + 0.5 + jy);
        let (ax, az) = turned(&frame, angle);
        let tint = rng.range(0.86, 1.08);
        let (horizontal, vertical) = match object.generator {
            Generator::RockMassif => {
                // Inside the clump only some tiles carry a peak, wide enough to merge with its
                // neighbours; the edge gets low foothills. Peak heights follow ridge lines of
                // low-frequency noise across the range.
                let depth = depth as f32 / MASSIF_DEPTH as f32;
                if depth > 0.0 && rng.unit() > MASSIF_PEAK_SHARE {
                    continue;
                }
                let ridge = 1.0
                    - (value_noise(looks.seed, scale(frame.origin, 1.0 / 11.0)) * 2.0 - 1.0).abs();
                let swell = value_noise(looks.seed ^ 0x55, scale(frame.origin, 1.0 / 23.0));
                let mass = (0.25 + 0.75 * depth) * (0.35 + 0.65 * ridge.max(swell));
                (
                    object.radius * rng.range(0.8, 1.2) * (0.55 + 0.45 * depth),
                    object.height * mass * rng.range(0.85, 1.15),
                )
            }
            _ => {
                let s = rng.range(0.9, 1.1);
                (s, s * rng.range(0.92, 1.08))
            }
        };
        let instance = FeatureInstance {
            mesh,
            tile: i as u32,
            transform: transform(
                scale(ax, horizontal),
                scale(frame.up, vertical),
                scale(az, horizontal),
                frame.origin,
                tint,
            ),
        };
        chunks
            .entry((face, y / side, x / side))
            .or_default()
            .push(instance);
    }
    let reach = |m: &ChunkMesh| {
        m.positions
            .iter()
            .map(|p| dot(*p, *p).sqrt())
            .fold(0.0_f32, f32::max)
    };
    let extents: Vec<f32> = baked.meshes.iter().map(reach).collect();
    for ((face, row, column), instances) in chunks {
        let (x0, y0) = (column * side, row * side);
        let spec = ChunkSpec {
            face,
            x0,
            y0,
            w: side.min(columns - x0),
            h: side.min(rows - y0),
        };
        let origin = |t: &[f32; 16]| [t[12], t[13], t[14]];
        let count = instances.len() as f32;
        let centre = scale(
            instances
                .iter()
                .fold([0.0; 3], |s, inst| add(s, origin(&inst.transform))),
            1.0 / count,
        );
        let radius = instances
            .iter()
            .map(|inst| {
                let t = &inst.transform;
                let axis =
                    |k: usize| dot([t[k], t[k + 1], t[k + 2]], [t[k], t[k + 1], t[k + 2]]).sqrt();
                let size = axis(0).max(axis(4)).max(axis(8));
                let d = sub(origin(t), centre);
                dot(d, d).sqrt() + size * extents[usize::from(inst.mesh)]
            })
            .fold(0.0_f32, f32::max);
        baked.chunks.push(FeatureChunk {
            spec,
            centre,
            radius,
            instances,
        });
    }
    baked
}

/// Instances (mesh, transform) of the crop rows of a building drawn as `variants` of a crop
/// look: rows run along the footprint's longer axis turned towards the building's facing, one
/// instance per tile of each row, and grow with construction.
pub fn crop_instances(
    view: &EntityView,
    ground: &dyn Footing,
    look: &ObjectLook,
    variants: &[u16],
    out: &mut Vec<(u16, [f32; 16])>,
) {
    if variants.is_empty() {
        return;
    }
    let (w, h) = (
        u32::from(view.footprint.0.max(1)),
        u32::from(view.footprint.1.max(1)),
    );
    // Rows follow the facing: east-west when the building faces along x, else north-south.
    let facing = view.facing_deg.to_radians();
    let along_x = facing.cos().abs() >= facing.sin().abs();
    let (length, across) = if along_x { (w, h) } else { (h, w) };
    let rows = across * look.count.max(1);
    let spacing = across as f32 / rows as f32;
    let growth = view
        .under_construction
        .map_or(1.0, |p| p.clamp(0.0, 1.0))
        .max(0.05);
    let mut rng = Rng::new(u64::from(view.id), 0xc0b5);
    for row in 0..rows {
        let offset = -(across as f32) / 2.0 + spacing * (row as f32 + 0.5);
        for step in 0..length {
            let along = -(length as f32) / 2.0 + step as f32 + 0.5;
            let (dx, dy) = if along_x {
                (along, offset)
            } else {
                (offset, along)
            };
            let frame = ground.frame(view.face, view.x, view.y);
            let point = ground.point(view.face, view.x, view.y, (dx, dy), 0.0);
            let local = GroundFrame {
                origin: point,
                ..frame
            };
            let (x_axis, z_axis) = if along_x {
                (local.ex, local.ey)
            } else {
                (local.ey, scale(local.ex, -1.0))
            };
            let mesh = variants[rng.below(variants.len() as u32) as usize];
            out.push((
                mesh,
                transform(
                    x_axis,
                    scale(local.up, growth),
                    scale(z_axis, spacing * 1.2),
                    local.origin,
                    rng.range(0.9, 1.06),
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globe::PlanetScene;
    use crate::looks::{MaterialLook, SnowLook, TileLook};
    use crate::terrain::TerrainScene;
    use std::sync::Arc;

    const FOREST: TileKind = TileKind::Feature(7);
    const MOUNTAIN: TileKind = TileKind::Feature(8);

    fn material(program: Program) -> MaterialLook {
        MaterialLook {
            program,
            colour: [0.2, 0.5, 0.2],
            colour_alt: [0.5, 0.6, 0.2],
            scale: 1.0,
        }
    }

    fn looks(seed: u64) -> SceneLooks {
        let foliage = material(Program::Foliage);
        let mut centre = ObjectLook::new(Generator::ForestCentre, foliage);
        centre.height = 1.3;
        let mut edge = ObjectLook::new(Generator::ForestEdge, foliage);
        edge.trunk = Some((0.6, [0.4, 0.3, 0.2]));
        let mut rock = ObjectLook::new(Generator::RockMassif, material(Program::Stone));
        rock.snow = Some(SnowLook {
            line: 4.0,
            material: material(Program::Snow),
        });
        let mut crops = ObjectLook::new(Generator::CropRows, material(Program::Crop));
        crops.variants = 2;
        SceneLooks {
            seed,
            tiles: vec![
                (
                    FOREST,
                    TileLook {
                        surface: Some(material(Program::ForestFloor)),
                        centre: Some(centre),
                        edge: Some(edge),
                        ..TileLook::default()
                    },
                ),
                (
                    MOUNTAIN,
                    TileLook {
                        surface: Some(material(Program::Stone)),
                        centre: Some(rock),
                        ..TileLook::default()
                    },
                ),
            ],
            buildings: BTreeMap::from([(42, crops)]),
        }
    }

    /// A flat map with a 6 × 5 forest block, a mountain strip and plain land around.
    fn flat_scene(width: u32, height: u32) -> Surface {
        let mut kinds = vec![TileKind::Land; (width * height) as usize];
        for y in 2..7 {
            for x in 3..9 {
                kinds[(y * width + x) as usize] = FOREST;
            }
        }
        for x in 12..20 {
            for y in 3..6 {
                kinds[(y * width + x) as usize] = MOUNTAIN;
            }
        }
        Surface::new(Arc::new(TerrainScene {
            width,
            height,
            heights: (0..width * height).map(|i| (i * 13 % 200) as u8).collect(),
            kinds,
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
        }))
    }

    #[test]
    fn forest_edge_and_centre_tiles_are_split_correctly() {
        let surface = flat_scene(24, 12);
        let width = 24;
        let index = |x: usize, y: usize| y * width + x;
        // Every tile on the block's border is an edge tile; the inner 4 × 3 are centre tiles.
        for y in 2..7 {
            for x in 3..9 {
                let border = x == 3 || x == 8 || y == 2 || y == 6;
                assert_eq!(is_edge(&surface, index(x, y)), border, "tile {x},{y}");
            }
        }
        // A diagonal neighbour of another kind is enough to make an edge.
        let baked = bake_flat(&surface, &looks(3));
        let meshes_of = |part: usize| -> Vec<u16> {
            let per_look = 4;
            ((part * per_look) as u16..((part + 1) * per_look) as u16).collect()
        };
        let (centre_meshes, edge_meshes) = (meshes_of(0), meshes_of(1));
        let mut seen = (0, 0);
        for chunk in &baked.chunks {
            for instance in &chunk.instances {
                let tile = instance.tile as usize;
                let (x, y) = (tile % width, tile / width);
                if surface.scene().kinds[tile] != FOREST {
                    continue;
                }
                if is_edge(&surface, tile) {
                    assert!(edge_meshes.contains(&instance.mesh), "edge tile {x},{y}");
                    seen.1 += 1;
                } else {
                    assert!(
                        centre_meshes.contains(&instance.mesh),
                        "centre tile {x},{y}"
                    );
                    seen.0 += 1;
                }
            }
        }
        assert_eq!(seen, (12, 18));
        // The edge variants have trunks (bark parts), the centre variants are all foliage.
        let has_bark = |m: u16| {
            baked.meshes[usize::from(m)]
                .colors
                .iter()
                .any(|c| c[3] == Program::Bark.id() as u8)
        };
        assert!(edge_meshes.iter().all(|&m| has_bark(m)));
        assert!(centre_meshes.iter().all(|&m| !has_bark(m)));
        // Trunks start below the ground and stand under their crown.
        for &m in &edge_meshes {
            let mesh = &baked.meshes[usize::from(m)];
            let low = mesh.positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
            assert!(low < 0.0);
        }
    }

    #[test]
    fn feature_instances_are_bounded_per_chunk() {
        let planet = PlanetScene {
            n: 40,
            kinds: (0..6 * 40 * 40)
                .map(|i| match i % 11 {
                    0..=5 => FOREST,
                    6 | 7 => MOUNTAIN,
                    _ => TileKind::Land,
                })
                .collect(),
            heights: (0..6 * 40 * 40).map(|i| (i * 7 % 255) as u8).collect(),
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
            sun: Default::default(),
        };
        let globe = Globe::new(&planet);
        let baked = bake_planet(&globe, &looks(1));
        let feature_tiles = planet
            .kinds
            .iter()
            .filter(|k| **k == FOREST || **k == MOUNTAIN)
            .count();
        // One instance at most per tile; inner mountain tiles may carry none.
        assert!(baked.instance_count() <= feature_tiles);
        assert!(baked.instance_count() > feature_tiles / 2);
        let mut windows = std::collections::BTreeSet::new();
        for chunk in &baked.chunks {
            assert!(chunk.instances.len() <= MAX_CHUNK_INSTANCES);
            assert!(chunk.spec.w <= OBJECT_CHUNK_TILES && chunk.spec.h <= OBJECT_CHUNK_TILES);
            assert!(windows.insert((chunk.spec.face, chunk.spec.x0, chunk.spec.y0)));
            for instance in &chunk.instances {
                let tile = globe.surface().tile(instance.tile as usize);
                assert_eq!(tile.face, chunk.spec.face);
                assert!((chunk.spec.x0..chunk.spec.x0 + chunk.spec.w).contains(&tile.x));
                assert!((chunk.spec.y0..chunk.spec.y0 + chunk.spec.h).contains(&tile.y));
                // The bounding sphere holds every instance origin.
                let t = &instance.transform;
                let d = sub([t[12], t[13], t[14]], chunk.centre);
                assert!(dot(d, d).sqrt() <= chunk.radius);
            }
        }
        // Every variant mesh fits 16-bit indices and the bake made a few variants per look.
        assert!(
            baked
                .meshes
                .iter()
                .all(|m| m.positions.len() < usize::from(u16::MAX))
        );
        assert_eq!(baked.meshes.len(), 4 + 4 + 4 + 2);
        assert_eq!(baked.buildings[&42], vec![12, 13]);
    }

    #[test]
    fn bake_is_deterministic_for_a_seed() {
        let surface = flat_scene(24, 12);
        let first = bake_flat(&surface, &looks(9));
        assert_eq!(first, bake_flat(&surface, &looks(9)));
        assert!(first.instance_count() > 0);
        let other = bake_flat(&surface, &looks(10));
        assert_ne!(first.meshes, other.meshes);
        assert_ne!(first.chunks, other.chunks);
        // Every rim tile of the massif has a foothill; the inner peaks stand taller.
        let height_of = |tile: usize| {
            first
                .chunks
                .iter()
                .flat_map(|c| &c.instances)
                .find(|i| i.tile as usize == tile)
                .map(|i| {
                    let t = &i.transform;
                    dot([t[4], t[5], t[6]], [t[4], t[5], t[6]]).sqrt()
                })
        };
        let rim: Vec<f32> = (12..20)
            .map(|x| height_of(3 * 24 + x).expect("foothill"))
            .collect();
        let core: Vec<f32> = (13..19).filter_map(|x| height_of(4 * 24 + x)).collect();
        let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
        assert!(
            core.is_empty() || mean(&core) > mean(&rim),
            "core {core:?} rim {rim:?}"
        );
    }

    fn farm(facing: f32, footprint: (u8, u8)) -> EntityView {
        EntityView {
            id: 77,
            kind: 42,
            owner: 1,
            face: 0,
            x: 10.0,
            y: 6.0,
            facing_deg: facing,
            hp_frac: 1.0,
            building: true,
            footprint,
            radius: 2.0,
            under_construction: None,
        }
    }

    #[test]
    fn crop_rows_follow_the_footprint_and_facing() {
        let surface = flat_scene(24, 12);
        let look = looks(1).buildings[&42];
        for (facing, footprint) in [
            (0.0, (4, 3)),
            (90.0, (4, 3)),
            (180.0, (2, 4)),
            (270.0, (4, 4)),
        ] {
            let view = farm(facing, footprint);
            let mut out = Vec::new();
            crop_instances(&view, &surface, &look, &[5, 6], &mut out);
            let (w, h) = (f32::from(footprint.0), f32::from(footprint.1));
            let along_x = facing == 0.0 || facing == 180.0;
            let (length, across) = if along_x { (w, h) } else { (h, w) };
            assert_eq!(out.len(), (length * across) as usize * look.count as usize);
            for (mesh, t) in &out {
                assert!([5, 6].contains(mesh));
                // Each row tile stands inside the footprint …
                let (x, y) = (t[12], t[14]);
                assert!((x - view.x).abs() <= w / 2.0 && (y - view.y).abs() <= h / 2.0);
                // … on the ground …
                assert!((t[13] - surface.height(x, y)).abs() < 1e-4);
                // … with its row axis along the facing and its blades upright.
                let row_axis = [t[0], t[1], t[2]];
                let expected = if along_x {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 0.0, 1.0]
                };
                assert!((dot(row_axis, expected) - 1.0).abs() < 1e-5, "{row_axis:?}");
                assert!((t[5] - 1.0).abs() < 1e-5);
                // Right-handed, so the baked winding survives.
                let z = [t[8], t[9], t[10]];
                assert!(dot(cross(row_axis, [t[4], t[5], t[6]]), z) > 0.0);
            }
            // Rows are evenly spaced across the footprint.
            let mut offsets: Vec<i32> = out
                .iter()
                .map(|(_, t)| {
                    let across_value = if along_x {
                        t[14] - view.y
                    } else {
                        t[12] - view.x
                    };
                    (across_value * 1000.0).round() as i32
                })
                .collect();
            offsets.sort_unstable();
            offsets.dedup();
            assert_eq!(offsets.len(), across as usize * look.count as usize);
        }
        // A site under construction grows its rows from the ground.
        let mut site = farm(0.0, (4, 4));
        site.under_construction = Some(0.5);
        let mut out = Vec::new();
        crop_instances(&site, &surface, &look, &[5], &mut out);
        assert!(out.iter().all(|(_, t)| (t[5] - 0.5).abs() < 1e-5));
    }
}

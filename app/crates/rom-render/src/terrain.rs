//! Terrain description and the pure functions that turn it into mesh data.
//!
//! Nothing here touches raylib, so every function can be tested without a window.
//! World space: tile (x, y) occupies x..x+1 and z = y..y+1; y is up, sea level is 0.

use std::collections::BTreeMap;

use crate::camera::Camera;

/// Largest chunk side in tiles; 65 × 65 vertices stay far below the 16-bit index limit.
pub const CHUNK_TILES: u32 = 64;
/// Height of the highest land above sea level, in tiles.
pub const RELIEF: f32 = 5.0;
/// Depth of the sea floor next to the coast, in tiles.
const FLOOR_NEAR: f32 = 0.15;
/// Depth of the deepest sea floor, in tiles.
const FLOOR_DEEP: f32 = 3.0;
/// Radius of the sphere the stars sit on.
pub const STAR_RADIUS: f32 = 100.0;
/// Number of stars in the backdrop.
pub const STAR_COUNT: usize = 3_000;

/// What a tile is made of, as far as drawing cares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    /// Plain ground.
    Land,
    /// Water with a floor close to the surface.
    ShallowSea,
    /// Deep water.
    DeepSea,
    /// Open space; never drawn.
    Space,
    /// A special terrain; the id selects a colour in [`TerrainScene::feature_colors`].
    Feature(u16),
}

/// A map as the renderer needs it: tile grids, feature colours and marker positions.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainScene {
    /// Width in tiles.
    pub width: u32,
    /// Height in tiles.
    pub height: u32,
    /// Height 0..=255 per tile, row-major.
    pub heights: Vec<u8>,
    /// Kind per tile, row-major.
    pub kinds: Vec<TileKind>,
    /// Colour (RGB) per feature id.
    pub feature_colors: BTreeMap<u16, [u8; 3]>,
    /// Player starts in tile units.
    pub starts: Vec<(f32, f32)>,
    /// Resource nodes in tile units.
    pub nodes: Vec<(f32, f32)>,
}

impl TerrainScene {
    /// True when both grids hold exactly `width * height` tiles and the map is not empty.
    pub fn is_valid(&self) -> bool {
        let tiles = u64::from(self.width) * u64::from(self.height);
        tiles > 0 && self.heights.len() as u64 == tiles && self.kinds.len() as u64 == tiles
    }

    fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.width as usize + x as usize
    }
}

/// Per-tile elevation and colour derived from a scene.
#[derive(Clone, Debug)]
pub struct Shape {
    /// Elevation of each tile in tiles of height; negative below sea level.
    pub elevation: Vec<f32>,
    /// Colour of each tile, RGB in 0..=1.
    pub color: Vec<[f32; 3]>,
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn land_color(t: f32) -> [f32; 3] {
    const GREEN: [f32; 3] = [0.24, 0.46, 0.20];
    const BROWN: [f32; 3] = [0.47, 0.36, 0.22];
    const PALE: [f32; 3] = [0.88, 0.87, 0.82];
    if t < 0.5 {
        mix(GREEN, BROWN, t * 2.0)
    } else {
        mix(BROWN, PALE, (t - 0.5) * 2.0)
    }
}

impl Shape {
    /// Computes elevation and colour for every tile.
    pub fn new(scene: &TerrainScene) -> Self {
        let (mut land_min, mut land_max) = (u8::MAX, 0_u8);
        let (mut water_min, mut water_max) = (u8::MAX, 0_u8);
        let (mut has_land, mut has_water) = (false, false);
        for (kind, &h) in scene.kinds.iter().zip(&scene.heights) {
            match kind {
                TileKind::Land | TileKind::Feature(_) => {
                    has_land = true;
                    land_min = land_min.min(h);
                    land_max = land_max.max(h);
                }
                TileKind::ShallowSea | TileKind::DeepSea => {
                    has_water = true;
                    water_min = water_min.min(h);
                    water_max = water_max.max(h);
                }
                TileKind::Space => {}
            }
        }
        // Sea level sits at the highest water tile, or at the lowest land when there is no water.
        let base = match (has_water, has_land) {
            (true, _) => f32::from(water_max),
            (false, true) => f32::from(land_min),
            (false, false) => 0.0,
        };
        let top = f32::from(land_max);
        let land_span = (top - base).max(1.0);
        let water_span = f32::from(water_max.saturating_sub(water_min)).max(1.0);

        let tiles = scene.kinds.len();
        let mut elevation = vec![0.0; tiles];
        let mut color = vec![[0.0; 3]; tiles];
        for (i, (kind, &h)) in scene.kinds.iter().zip(&scene.heights).enumerate() {
            let h = f32::from(h);
            match kind {
                TileKind::Land | TileKind::Feature(_) => {
                    let t = ((h - base) / land_span).clamp(0.0, 1.0);
                    elevation[i] = t * RELIEF;
                    let ground = land_color(t);
                    color[i] = match kind {
                        TileKind::Feature(id) => {
                            let tint = scene
                                .feature_colors
                                .get(id)
                                .copied()
                                .unwrap_or([200, 0, 200]);
                            mix(ground, tint.map(|c| f32::from(c) / 255.0), 0.65)
                        }
                        _ => ground,
                    };
                }
                TileKind::ShallowSea | TileKind::DeepSea => {
                    let depth = ((f32::from(water_max) - h) / water_span).clamp(0.0, 1.0);
                    elevation[i] = -(FLOOR_NEAR + depth * (FLOOR_DEEP - FLOOR_NEAR));
                    color[i] = if *kind == TileKind::DeepSea {
                        [0.07, 0.09, 0.13]
                    } else {
                        [0.42, 0.40, 0.30]
                    };
                }
                TileKind::Space => {}
            }
        }
        Self { elevation, color }
    }
}

/// Indices of the non-space tiles around vertex (`vx`, `vy`).
fn adjoining(scene: &TerrainScene, vx: i64, vy: i64) -> impl Iterator<Item = usize> + '_ {
    [(-1, -1), (0, -1), (-1, 0), (0, 0)]
        .into_iter()
        .filter_map(move |(dx, dy)| {
            let (x, y) = (vx + dx, vy + dy);
            if x < 0 || y < 0 || x >= i64::from(scene.width) || y >= i64::from(scene.height) {
                return None;
            }
            let i = scene.index(x as u32, y as u32);
            (scene.kinds[i] != TileKind::Space).then_some(i)
        })
}

/// Height of the vertex at the corner (`vx`, `vy`): the average of the adjoining
/// non-space tiles, or `None` when all of them are space.
pub fn vertex_height(scene: &TerrainScene, shape: &Shape, vx: i64, vy: i64) -> Option<f32> {
    let (sum, count) = adjoining(scene, vx, vy).fold((0.0, 0_u32), |(sum, count), i| {
        (sum + shape.elevation[i], count + 1)
    });
    (count > 0).then(|| sum / count as f32)
}

fn vertex_color(scene: &TerrainScene, shape: &Shape, vx: i64, vy: i64) -> [f32; 3] {
    let (sum, count) = adjoining(scene, vx, vy).fold(([0.0; 3], 0_u32), |(sum, count), i| {
        let c = shape.color[i];
        ([sum[0] + c[0], sum[1] + c[1], sum[2] + c[2]], count + 1)
    });
    let n = count.max(1) as f32;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

fn vertex_normal(scene: &TerrainScene, shape: &Shape, vx: i64, vy: i64, here: f32) -> [f32; 3] {
    let at = |x: i64, y: i64| {
        let x = x.clamp(0, i64::from(scene.width));
        let y = y.clamp(0, i64::from(scene.height));
        vertex_height(scene, shape, x, y).unwrap_or(here)
    };
    let dx = (at(vx - 1, vy) - at(vx + 1, vy)) / 2.0;
    let dz = (at(vx, vy - 1) - at(vx, vy + 1)) / 2.0;
    let length = (dx * dx + 1.0 + dz * dz).sqrt();
    [dx / length, 1.0 / length, dz / length]
}

/// Vertex and index data of one chunk.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChunkMesh {
    /// Vertex positions in world space.
    pub positions: Vec<[f32; 3]>,
    /// Unit normals, one per vertex (empty for the sea).
    pub normals: Vec<[f32; 3]>,
    /// RGBA per vertex.
    pub colors: Vec<[u8; 4]>,
    /// Triangle list, counter-clockwise seen from above.
    pub indices: Vec<u16>,
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Which tiles a chunk mesh covers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cover {
    Terrain,
    Water,
}

fn chunk_origins(scene: &TerrainScene) -> impl Iterator<Item = (u32, u32, u32, u32)> {
    let (width, height) = (scene.width, scene.height);
    (0..height.div_ceil(CHUNK_TILES)).flat_map(move |cy| {
        (0..width.div_ceil(CHUNK_TILES)).map(move |cx| {
            let (x0, y0) = (cx * CHUNK_TILES, cy * CHUNK_TILES);
            (
                x0,
                y0,
                CHUNK_TILES.min(width - x0),
                CHUNK_TILES.min(height - y0),
            )
        })
    })
}

fn covered(kind: TileKind, cover: Cover) -> bool {
    match cover {
        Cover::Terrain => kind != TileKind::Space,
        Cover::Water => matches!(kind, TileKind::ShallowSea | TileKind::DeepSea),
    }
}

fn build_chunks(scene: &TerrainScene, shape: &Shape, cover: Cover) -> Vec<ChunkMesh> {
    let mut chunks = Vec::new();
    for (x0, y0, w, h) in chunk_origins(scene) {
        let mut indices = Vec::new();
        let stride = w as usize + 1;
        for j in 0..h {
            for i in 0..w {
                if !covered(scene.kinds[scene.index(x0 + i, y0 + j)], cover) {
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
            continue;
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
                let (vx, vy) = (i64::from(x0 + i), i64::from(y0 + j));
                let height = vertex_height(scene, shape, vx, vy);
                match cover {
                    Cover::Terrain => {
                        let here = height.unwrap_or(0.0);
                        mesh.positions.push([vx as f32, here, vy as f32]);
                        mesh.normals.push(vertex_normal(scene, shape, vx, vy, here));
                        let c = vertex_color(scene, shape, vx, vy);
                        mesh.colors.push([byte(c[0]), byte(c[1]), byte(c[2]), 255]);
                    }
                    Cover::Water => {
                        let depth = (-height.unwrap_or(-FLOOR_DEEP) / FLOOR_DEEP).clamp(0.0, 1.0);
                        let c = mix([0.22, 0.62, 0.74], [0.03, 0.14, 0.38], depth);
                        mesh.positions.push([vx as f32, 0.0, vy as f32]);
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
        chunks.push(mesh);
    }
    chunks
}

/// Terrain meshes, at most 64 × 64 tiles each; chunks without non-space tiles are left out.
pub fn build_terrain_chunks(scene: &TerrainScene, shape: &Shape) -> Vec<ChunkMesh> {
    build_chunks(scene, shape, Cover::Terrain)
}

/// Flat sea-level meshes over the water tiles, tinted and made opaque by depth.
pub fn build_sea_chunks(scene: &TerrainScene, shape: &Shape) -> Vec<ChunkMesh> {
    build_chunks(scene, shape, Cover::Water)
}

/// Ground height under a point given in tile units; zero outside the map.
pub fn ground_height(scene: &TerrainScene, shape: &Shape, x: f32, y: f32) -> f32 {
    if x < 0.0 || y < 0.0 || x >= scene.width as f32 || y >= scene.height as f32 {
        return 0.0;
    }
    shape.elevation[scene.index(x as u32, y as u32)].max(0.0)
}

/// Camera on the first player start (or the map centre), at a close strategy angle.
pub fn home_camera(scene: &TerrainScene) -> Camera {
    let (x, z) = scene
        .starts
        .first()
        .copied()
        .unwrap_or((scene.width as f32 / 2.0, scene.height as f32 / 2.0));
    Camera {
        target: [x, 0.0, z],
        distance: 48.0,
        pitch: 38_f32.to_radians(),
        bounds: map_bounds(scene),
        ..Camera::default()
    }
}

/// Camera that frames the whole map.
pub fn overview_camera(scene: &TerrainScene) -> Camera {
    let (w, h) = (scene.width as f32, scene.height as f32);
    Camera {
        target: [w / 2.0, 0.0, h / 2.0],
        pitch: 60_f32.to_radians(),
        distance: w.max(h) * 1.4,
        bounds: map_bounds(scene),
        ..Camera::default()
    }
    .limited()
}

fn map_bounds(scene: &TerrainScene) -> [f32; 4] {
    [0.0, 0.0, scene.width as f32, scene.height as f32]
}

/// One star: a direction on the unit sphere, an apparent size and a brightness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Unit vector from the viewer to the star.
    pub direction: [f32; 3],
    /// Half the edge of the star's triangle at [`STAR_RADIUS`].
    pub size: f32,
    /// Brightness 0..=255.
    pub brightness: u8,
}

fn splitmix(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    state = (state ^ (state >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    state ^ (state >> 31)
}

/// A fixed star field: the same stars on every call.
pub fn star_field(count: usize) -> Vec<Star> {
    (0..count as u64)
        .map(|i| {
            let bits = splitmix(i ^ 0x5eed_5eed_5eed_5eed);
            let unit = |shift: u32| ((bits >> shift) & 0xf_ffff) as f32 / 1_048_575.0;
            let z = unit(0) * 2.0 - 1.0;
            let angle = unit(20) * std::f32::consts::TAU;
            let ring = (1.0 - z * z).sqrt();
            let rank = (bits >> 40) & 0xff;
            Star {
                direction: [ring * angle.cos(), z, ring * angle.sin()],
                size: 0.1 + 0.2 * (rank * rank) as f32 / 65_025.0,
                brightness: (110 + rank * 145 / 255) as u8,
            }
        })
        .collect()
}

/// Triangle list (positions and colours) for the stars, facing a viewer at the origin.
pub fn star_triangles(stars: &[Star]) -> (Vec<[f32; 3]>, Vec<[u8; 4]>) {
    let mut positions = Vec::with_capacity(stars.len() * 3);
    let mut colors = Vec::with_capacity(stars.len() * 3);
    for star in stars {
        let d = star.direction;
        let helper = if d[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let t1 = normalize(cross(helper, d));
        let t2 = cross(d, t1);
        let corner = |a: f32, b: f32| {
            let s = star.size;
            [0, 1, 2].map(|k| STAR_RADIUS * d[k] + s * (a * t1[k] + b * t2[k]))
        };
        // Seen from the origin the (t1, t2) plane runs clockwise, so the corners go in reverse.
        positions.extend([corner(-1.0, -0.6), corner(0.0, 1.2), corner(1.0, -0.6)]);
        let b = star.brightness;
        let tint = [b, b, b.saturating_add(20), 255];
        colors.extend([tint; 3]);
    }
    (positions, colors)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / length, v[1] / length, v[2] / length]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A scene with a patchwork of kinds; `space_every` makes every n-th tile space.
    fn scene(width: u32, height: u32, space_every: usize) -> TerrainScene {
        let tiles = (width * height) as usize;
        let kinds = (0..tiles)
            .map(|i| match i {
                _ if space_every > 0 && i % space_every == 0 => TileKind::Space,
                _ if i % 7 == 1 => TileKind::DeepSea,
                _ if i % 7 == 2 => TileKind::ShallowSea,
                _ if i % 7 == 3 => TileKind::Feature(9),
                _ => TileKind::Land,
            })
            .collect();
        TerrainScene {
            width,
            height,
            heights: (0..tiles).map(|i| (i * 37 % 251) as u8).collect(),
            kinds,
            feature_colors: BTreeMap::from([(9, [10, 200, 30])]),
            starts: vec![(3.5, 4.5)],
            nodes: vec![(5.5, 5.5)],
        }
    }

    fn triangles(mesh: &ChunkMesh) -> impl Iterator<Item = [usize; 3]> + '_ {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(usize::from))
    }

    /// The tile a triangle belongs to: the lowest corner in x and z.
    fn tile_of(mesh: &ChunkMesh, triangle: [usize; 3]) -> (u32, u32) {
        let min = |axis: usize| {
            triangle
                .iter()
                .map(|&i| mesh.positions[i][axis])
                .fold(f32::MAX, f32::min)
        };
        (min(0) as u32, min(2) as u32)
    }

    #[test]
    fn terrain_chunks_cover_every_non_space_tile_once() {
        let scene = scene(150, 70, 11);
        let shape = Shape::new(&scene);
        let mut seen: HashMap<(u32, u32), u32> = HashMap::new();
        for chunk in build_terrain_chunks(&scene, &shape) {
            for triangle in triangles(&chunk) {
                *seen.entry(tile_of(&chunk, triangle)).or_default() += 1;
            }
        }
        for y in 0..scene.height {
            for x in 0..scene.width {
                let expected = u32::from(scene.kinds[scene.index(x, y)] != TileKind::Space) * 2;
                assert_eq!(
                    seen.get(&(x, y)).copied().unwrap_or(0),
                    expected,
                    "tile {x},{y}"
                );
            }
        }
    }

    #[test]
    fn chunk_index_counts_fit_16_bits() {
        let scene = scene(200, 130, 0);
        let shape = Shape::new(&scene);
        let chunks = build_terrain_chunks(&scene, &shape);
        assert_eq!(chunks.len(), 4 * 3);
        for chunk in chunks.iter().chain(&build_sea_chunks(&scene, &shape)) {
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
    }

    #[test]
    fn vertex_height_is_average_of_adjoining_tiles() {
        let scene = scene(6, 6, 0);
        let shape = Shape::new(&scene);
        let average = |tiles: &[(u32, u32)]| {
            tiles
                .iter()
                .map(|&(x, y)| shape.elevation[scene.index(x, y)])
                .sum::<f32>()
                / tiles.len() as f32
        };
        let inner = vertex_height(&scene, &shape, 3, 2).expect("height");
        assert!((inner - average(&[(2, 1), (3, 1), (2, 2), (3, 2)])).abs() < 1e-6);
        let corner = vertex_height(&scene, &shape, 0, 0).expect("height");
        assert!((corner - average(&[(0, 0)])).abs() < 1e-6);
        let edge = vertex_height(&scene, &shape, 6, 3).expect("height");
        assert!((edge - average(&[(5, 2), (5, 3)])).abs() < 1e-6);
    }

    #[test]
    fn space_tiles_produce_no_triangles() {
        let mut scene = scene(20, 20, 0);
        scene.kinds.fill(TileKind::Space);
        let shape = Shape::new(&scene);
        assert!(build_terrain_chunks(&scene, &shape).is_empty());
        assert!(build_sea_chunks(&scene, &shape).is_empty());
        assert_eq!(vertex_height(&scene, &shape, 4, 4), None);
        scene.kinds[21] = TileKind::Land;
        let chunks = build_terrain_chunks(&scene, &Shape::new(&scene));
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].indices.len(), 6);
    }

    #[test]
    fn star_field_is_deterministic() {
        let first = star_field(STAR_COUNT);
        assert_eq!(first, star_field(STAR_COUNT));
        assert_eq!(first.len(), STAR_COUNT);
        for star in &first {
            let [x, y, z] = star.direction;
            assert!((x * x + y * y + z * z - 1.0).abs() < 1e-4);
        }
        assert_ne!(first[0].direction, first[1].direction);
    }

    #[test]
    fn star_triangles_face_the_viewer() {
        let stars = star_field(50);
        let (positions, colors) = star_triangles(&stars);
        assert_eq!(positions.len(), 150);
        assert_eq!(colors.len(), 150);
        for (star, tri) in stars.iter().zip(positions.as_chunks::<3>().0) {
            let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
            let normal = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
            let facing: f32 = (0..3).map(|k| normal[k] * star.direction[k]).sum();
            assert!(facing < 0.0, "triangle must face the origin");
        }
    }

    #[test]
    fn land_rises_above_sea_level_and_sea_floor_lies_below() {
        let scene = scene(30, 30, 0);
        let shape = Shape::new(&scene);
        for (kind, elevation) in scene.kinds.iter().zip(&shape.elevation) {
            match kind {
                TileKind::ShallowSea | TileKind::DeepSea => assert!(*elevation < 0.0),
                _ => assert!((0.0..=RELIEF).contains(elevation)),
            }
        }
    }
}

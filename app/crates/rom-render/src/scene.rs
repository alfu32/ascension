//! The world scene: terrain, sea, star field and markers, plus the optional stress-test boxes.
//!
//! Without a terrain it falls back to the original proof scene (a grid ground).
//! The boxes are a display test, not game content; their motion is computed here
//! from elapsed time only.

use std::time::Instant;

use raylib::prelude::{
    Camera3D, Color, Matrix, Mesh, RaylibDraw, RaylibDraw3D, RaylibHandle, RaylibMaterial,
    RaylibMesh, RaylibMode3DExt, RaylibShader, RaylibTextureMode, RaylibTextureModeExt,
    RaylibThread, RenderTexture2D, Shader, Vector2, Vector3, WeakMaterial,
};

use std::sync::Arc;

use crate::camera::Camera;
use crate::entities::{EntityView, Footing, Surface, build_instances_with_casters_on};
use crate::features::{BakedFeatures, crop_instances};
use crate::globe::{ChunkBounds, CullView, Globe, PlanetScene};
use crate::graticule::{GraticuleLine, graticule_fade, graticule_on};
use crate::hud::HudSnapshot;
use crate::looks::{SceneLooks, lod_weight};
use crate::materials::{SurfaceSlots, chunk_attributes, surface_slots};
use crate::orbit::OrbitCamera;
use crate::overlay::{OverlayGeometry, build_overlays, build_overlays_on};
use crate::sun;
use crate::terrain::{
    ChunkMesh, RELIEF, STAR_COUNT, TerrainScene, build_sea_chunks_with_specs,
    build_terrain_chunks_with_specs, ground_height, star_field, star_triangles,
};
use crate::vision::{ChunkSpec, shade_chunk};

mod materials;
mod shadow;
mod sky;
mod space;
pub(crate) use space::SpaceLabel;

/// Width of the ground in tiles.
const GROUND_TILES: f32 = 256.0;
/// Direction the sunlight travels (normalised in the shaders).
const LIGHT_DIRECTION: [f32; 3] = [-0.45, -1.0, -0.3];
/// Direction the star's light travels where ships are drawn in a system: from above the plane.
const SHIP_LIGHT: [f32; 3] = [-0.35, -1.0, -0.45];
/// Largest factor the start pillars and node boxes grow by when the camera is far away.
const MARKER_GROWTH: f32 = 2.5;
/// Camera distance up to which markers keep their size.
const MARKER_NEAR: f32 = 60.0;
const SKY: Color = Color::new(146, 178, 204, 255);
const SPACE: Color = Color::new(1, 2, 5, 255);
/// Height of a player-start pillar, in tiles.
const PILLAR_HEIGHT: f32 = 8.0;
const PLAYER_COLORS: [Color; 8] = [
    Color::new(220, 60, 60, 255),
    Color::new(60, 110, 230, 255),
    Color::new(240, 200, 50, 255),
    Color::new(70, 200, 90, 255),
    Color::new(190, 90, 220, 255),
    Color::new(240, 140, 50, 255),
    Color::new(60, 210, 220, 255),
    Color::new(235, 235, 235, 255),
];

// Stars are pushed to the far plane (z = w) so terrain always draws over them.
const STAR_VS: &str = r#"#version 330
in vec3 vertexPosition;
in vec4 vertexColor;
uniform mat4 mvp;
out vec4 fragColor;
void main() {
    fragColor = vertexColor;
    gl_Position = (mvp * vec4(vertexPosition, 1.0)).xyww;
}
"#;

const BOX_VS: &str = r#"#version 330
in vec3 vertexPosition;
in vec3 vertexNormal;
in mat4 instanceTransform;
uniform mat4 mvp;
out vec3 fragNormal;
flat out vec3 fragTint;
void main() {
    fragNormal = mat3(instanceTransform) * vertexNormal;
    float shade = fract(float(gl_InstanceID) * 0.6180339887);
    fragTint = mix(vec3(0.78, 0.36, 0.24), vec3(0.24, 0.44, 0.78), shade);
    gl_Position = mvp * instanceTransform * vec4(vertexPosition, 1.0);
}
"#;

const BOX_FS: &str = r#"#version 330
in vec3 fragNormal;
flat in vec3 fragTint;
uniform vec3 lightDirection;
out vec4 finalColor;
void main() {
    float diffuse = max(dot(normalize(fragNormal), -normalize(lightDirection)), 0.0);
    finalColor = vec4(fragTint * (0.3 + 0.7 * diffuse), 1.0);
}
"#;

// Entities are unit cubes; each instance carries its colour in the cells `m3`, `m7` and `m11` of
// its transform, which an affine matrix leaves at zero.
const ENTITY_VS: &str = r#"#version 330
in vec3 vertexPosition;
in vec3 vertexNormal;
in mat4 instanceTransform;
uniform mat4 mvp;
out vec3 fragNormal;
out vec3 fragWorld;
flat out vec3 fragTint;
void main() {
    mat4 transform = instanceTransform;
    fragTint = vec3(transform[0][3], transform[1][3], transform[2][3]);
    transform[0][3] = 0.0;
    transform[1][3] = 0.0;
    transform[2][3] = 0.0;
    fragNormal = mat3(transform) * vertexNormal;
    fragWorld = (transform * vec4(vertexPosition, 1.0)).xyz;
    gl_Position = mvp * transform * vec4(vertexPosition, 1.0);
}
"#;

/// Neutral fill keeps entity colours stable through the day; daylight shadows remain visible.
fn entity_fs() -> String {
    format!(
        r#"#version 330
in vec3 fragNormal;
in vec3 fragWorld;
flat in vec3 fragTint;
out vec4 finalColor;
{}
void main() {{
    vec3 normal = normalize(fragNormal);
    finalColor = vec4(fragTint * entityLight(fragWorld, normal), 1.0);
}}
"#,
        sun::glsl_lighting()
    )
}

// The shadow pass: depth from the light, packed into RGB (24 bits) of an ordinary render target.
const DEPTH_VS: &str = r#"#version 330
in vec3 vertexPosition;
uniform mat4 mvp;
void main() {
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

const DEPTH_INSTANCED_VS: &str = r#"#version 330
in vec3 vertexPosition;
in mat4 instanceTransform;
uniform mat4 mvp;
void main() {
    mat4 transform = instanceTransform;
    transform[0][3] = 0.0;
    transform[1][3] = 0.0;
    transform[2][3] = 0.0;
    gl_Position = mvp * transform * vec4(vertexPosition, 1.0);
}
"#;

const DEPTH_FS: &str = r#"#version 330
out vec4 finalColor;
void main() {
    float depth = floor(min(gl_FragCoord.z, 0.99999994) * 16777215.0);
    finalColor = vec4(floor(depth / 65536.0), mod(floor(depth / 256.0), 256.0), mod(depth, 256.0), 255.0) / 255.0;
}
"#;

const GROUND_VS: &str = r#"#version 330
in vec3 vertexPosition;
uniform mat4 mvp;
uniform mat4 matModel;
out vec2 groundPosition;
void main() {
    groundPosition = (matModel * vec4(vertexPosition, 1.0)).xz;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

const GROUND_FS: &str = r#"#version 330
in vec2 groundPosition;
uniform vec3 lightDirection;
out vec4 finalColor;
float gridLine(vec2 position) {
    vec2 distanceToLine = abs(fract(position - 0.5) - 0.5) / fwidth(position);
    return 1.0 - min(min(distanceToLine.x, distanceToLine.y), 1.0);
}
void main() {
    float diffuse = max(dot(vec3(0.0, 1.0, 0.0), -normalize(lightDirection)), 0.0);
    vec3 color = vec3(0.40, 0.50, 0.32) * (0.3 + 0.7 * diffuse);
    color = mix(color, vec3(0.27, 0.34, 0.22), gridLine(groundPosition) * 0.55);
    color = mix(color, vec3(0.18, 0.22, 0.15), gridLine(groundPosition / 16.0) * 0.8);
    finalColor = vec4(color, 1.0);
}
"#;

/// What baking and uploading the feature objects of a planet or map took.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BakeReport {
    /// Milliseconds to bake the variant meshes and place the instances.
    pub(crate) bake_ms: f32,
    /// Milliseconds to upload the variant meshes.
    pub(crate) upload_ms: f32,
    /// Variant meshes.
    pub(crate) meshes: usize,
    /// Vertices in all variant meshes.
    pub(crate) vertices: usize,
    /// Placed instances.
    pub(crate) instances: usize,
    /// Object chunks.
    pub(crate) chunks: usize,
    /// Land material slots in use.
    pub(crate) slots: usize,
}

/// Parameters of one box's circular path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Mover {
    center: [f32; 2],
    radius: f32,
    speed: f32,
    phase: f32,
    size: [f32; 3],
}

/// Deterministic box layout so every run draws the same scene.
pub(crate) fn movers(count: usize) -> Vec<Mover> {
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 40) as f32 / (1_u64 << 24) as f32
    };
    let half = GROUND_TILES / 2.0 - 6.0;
    (0..count)
        .map(|_| Mover {
            center: [(next() * 2.0 - 1.0) * half, (next() * 2.0 - 1.0) * half],
            radius: 1.0 + next() * 3.0,
            speed: (0.3 + next() * 0.9) * if next() < 0.5 { -1.0 } else { 1.0 },
            phase: next() * std::f32::consts::TAU,
            size: [0.6 + next() * 0.6, 0.6 + next() * 1.4, 0.6 + next() * 0.6],
        })
        .collect()
}

/// World transform of a unit cube following `mover` at `seconds`.
pub(crate) fn mover_transform(mover: &Mover, seconds: f32) -> Matrix {
    let angle = mover.phase + mover.speed * seconds;
    let (sin, cos) = angle.sin_cos();
    let [sx, sy, sz] = mover.size;
    Matrix {
        m0: cos * sx,
        m1: 0.0,
        m2: -sin * sx,
        m3: 0.0,
        m4: 0.0,
        m5: sy,
        m6: 0.0,
        m7: 0.0,
        m8: sin * sz,
        m9: 0.0,
        m10: cos * sz,
        m11: 0.0,
        m12: mover.center[0] + mover.radius * cos,
        m13: sy / 2.0,
        m14: mover.center[1] + mover.radius * sin,
        m15: 1.0,
    }
}

/// GPU resources of the proof scene. Create, draw and drop on the render thread only.
pub(crate) struct Scene {
    pub(crate) projection_centre_y: f32,
    box_mesh: Mesh,
    ground_mesh: Mesh,
    star_mesh: Option<Mesh>,
    terrain: Option<TerrainGpu>,
    terrain_material: WeakMaterial,
    star_material: WeakMaterial,
    // raylib-rs hands out the default material only as a non-owning handle and frees it
    // only through an `unsafe` call, so the two small map tables are left to process exit.
    // Their GPU objects (the shaders below) are freed normally.
    box_material: WeakMaterial,
    ground_material: WeakMaterial,
    /// Kept alive for the materials above, which point at them.
    entity_material: WeakMaterial,
    _shaders: [Shader; 5],
    /// The sea, on a sphere and on a flat map.
    sea_globe_shader: Shader,
    sea_globe_material: WeakMaterial,
    /// Baked feature objects and crop rows.
    object_shader: Shader,
    object_material: WeakMaterial,
    /// The uploaded feature objects of the current planet or map.
    features: Option<materials::FeatureGpu>,
    /// How the current planet or map looks up close.
    looks: Arc<SceneLooks>,
    /// Share of the material look drawn this frame (1 up close, 0 far out).
    detail: f32,
    /// Start of the clock that drives wind and waves.
    clock: Instant,
    /// Scratch list for shadow casters.
    caster_scratch: Vec<Matrix>,
    /// The noise texture the material programs sample.
    noise: Option<raylib::prelude::Texture2D>,
    /// Scratch list for crop instances.
    crop_scratch: Vec<(u16, [f32; 16])>,
    /// Direction the light that represents the sun travels, in planet space.
    sun: [f32; 3],
    /// The sky dome and the atmosphere halo.
    sky: sky::SkyGpu,
    /// The shadow map and its passes, made on first use.
    shadow: Option<shadow::ShadowGpu>,
    /// Latitude and longitude lines of the planet, laid on its surface.
    graticule: Vec<GraticuleLine>,
    /// Whether the graticule is drawn.
    pub(crate) show_graticule: bool,
    globe: Option<GlobeGpu>,
    space: Option<space::SpaceGpu>,
    /// What is in space, and which ships are selected, for the next draw.
    space_state: space::SpaceState,
    entity_transforms: Vec<Matrix>,
    /// Entity bodies only; camera-facing health bars never cast shadows.
    entity_casters: Vec<Matrix>,
    caster_ranges: Vec<std::ops::Range<usize>>,
    instances: Vec<[f32; 16]>,
    draw_markers: bool,
    movers: Vec<Mover>,
    transforms: Vec<Matrix>,
    overlay: OverlayGeometry,
    /// Tile shades the meshes were last coloured with.
    last_shades: Vec<[f32; 3]>,
    /// Chunk meshes (land, then sea) waiting to be recoloured.
    shade_queue: Vec<usize>,
}

/// A chunk mesh whose vertex colours follow the player's fog of war and borders.
struct ShadedMesh {
    mesh: Mesh,
    spec: ChunkSpec,
    /// Unshaded vertex colours.
    base: Vec<[u8; 4]>,
    /// Colours currently on the GPU.
    current: Vec<[u8; 4]>,
}

impl ShadedMesh {
    fn new(mesh: Mesh, spec: ChunkSpec, chunk: &ChunkMesh) -> Self {
        Self {
            mesh,
            spec,
            base: chunk.colors.clone(),
            current: chunk.colors.clone(),
        }
    }

    /// Shades the colours by `shades` and uploads them when they changed.
    fn apply(
        &mut self,
        shades: &[[f32; 3]],
        adjoining: crate::vision::Adjoining,
        scratch: &mut Vec<[u8; 4]>,
    ) {
        shade_chunk(&self.base, self.spec, shades, adjoining, scratch);
        if *scratch != self.current {
            // SAFETY: the mesh is uploaded, buffer 3 holds one RGBA byte quadruple per vertex and
            // `scratch` has exactly that many.
            unsafe {
                self.mesh.update_buffer::<u8>(3, scratch.as_flattened(), 0);
            }
            std::mem::swap(&mut self.current, scratch);
        }
    }
}

/// Meshes and markers of the published terrain.
struct TerrainGpu {
    land: Vec<ShadedMesh>,
    /// Bounding spheres of the land meshes, for the shadow pass.
    land_bounds: Vec<ChunkBounds>,
    sea: Vec<ShadedMesh>,
    /// Player pillars: centre, size and colour.
    pillars: Vec<(Vector3, Vector3, Color)>,
    /// Surface sampler for entities.
    surface: Surface,
    /// Map centre in tiles, where the stress-test boxes are centred.
    centre: [f32; 2],
    /// Map size in tiles.
    size: (u32, u32),
}

/// Meshes and markers of a published sphere planet.
struct GlobeGpu {
    land: Vec<(ShadedMesh, ChunkBounds)>,
    sea: Vec<(ShadedMesh, ChunkBounds)>,
    planet: Arc<PlanetScene>,
    globe: Arc<Globe>,
    markers: Vec<Matrix>,
}

/// A column-major array as a raylib matrix.
fn matrix_of(m: &[f32; 16]) -> Matrix {
    Matrix {
        m0: m[0],
        m1: m[1],
        m2: m[2],
        m3: m[3],
        m4: m[4],
        m5: m[5],
        m6: m[6],
        m7: m[7],
        m8: m[8],
        m9: m[9],
        m10: m[10],
        m11: m[11],
        m12: m[12],
        m13: m[13],
        m14: m[14],
        m15: m[15],
    }
}

/// Uploads one chunk of mesh data.
fn upload(thread: &RaylibThread, chunk: &ChunkMesh) -> Option<Mesh> {
    let positions: Vec<Vector3> = chunk
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let normals: Vec<Vector3> = chunk
        .normals
        .iter()
        .map(|n| Vector3::new(n[0], n[1], n[2]))
        .collect();
    let colors: Vec<Color> = chunk
        .colors
        .iter()
        .map(|c| Color::new(c[0], c[1], c[2], c[3]))
        .collect();
    let texcoords = vec![Vector2::new(0.0, 0.0); positions.len()];
    let mut builder = Mesh::gen_mesh(&positions, &texcoords);
    if !normals.is_empty() {
        builder.normals(&normals);
    }
    builder.colors(&colors);
    if !chunk.indices.is_empty() {
        builder.indices(&chunk.indices);
    }
    match builder.build(thread) {
        Ok(mesh) => Some(mesh),
        Err(error) => {
            eprintln!("rom-render: could not upload a mesh: {error:?}");
            None
        }
    }
}

impl Scene {
    pub(crate) fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut box_shader = rl.load_shader_from_memory(thread, Some(BOX_VS), Some(BOX_FS));
        let mut ground_shader =
            rl.load_shader_from_memory(thread, Some(GROUND_VS), Some(GROUND_FS));
        let terrain_shader = rl.load_shader_from_memory(
            thread,
            Some(materials::TERRAIN_VS),
            Some(&materials::terrain_fs()),
        );
        let entity_shader = rl.load_shader_from_memory(thread, Some(ENTITY_VS), Some(&entity_fs()));
        let mut star_shader = rl.load_shader_from_memory(thread, Some(STAR_VS), Some(sky::STAR_FS));
        let location = star_shader.get_shader_location("fade");
        star_shader.set_shader_value(location, 1.0_f32);
        let sea_globe_shader = rl.load_shader_from_memory(
            thread,
            Some(materials::WATER_VS),
            Some(&materials::water_fs()),
        );
        let mut sea_globe_material = rl.load_material_default(thread);
        sea_globe_material.set_shader(&sea_globe_shader);
        for shader in [&mut box_shader, &mut ground_shader] {
            let location = shader.get_shader_location("lightDirection");
            shader.set_shader_value(location, LIGHT_DIRECTION);
        }
        let mut box_material = rl.load_material_default(thread);
        box_material.set_shader(&box_shader);
        let mut ground_material = rl.load_material_default(thread);
        ground_material.set_shader(&ground_shader);
        let mut terrain_material = rl.load_material_default(thread);
        terrain_material.set_shader(&terrain_shader);
        let mut entity_material = rl.load_material_default(thread);
        entity_material.set_shader(&entity_shader);
        let mut object_shader = materials::load_object_shader(rl, thread);
        let noise = materials::load_noise(thread);
        let mut terrain_shader = terrain_shader;
        let mut sea_globe_shader = sea_globe_shader;
        for shader in [
            &mut terrain_shader,
            &mut sea_globe_shader,
            &mut object_shader,
        ] {
            materials::set_noise_slot(shader);
        }
        let mut object_material = rl.load_material_default(thread);
        object_material.set_shader(&object_shader);
        let mut star_material = rl.load_material_default(thread);
        star_material.set_shader(&star_shader);
        let (star_positions, star_colors) = star_triangles(&star_field(STAR_COUNT));
        let star_mesh = upload(
            thread,
            &ChunkMesh {
                positions: star_positions,
                normals: Vec::new(),
                colors: star_colors,
                indices: Vec::new(),
            },
        );
        Self {
            projection_centre_y: 0.0,
            box_mesh: Mesh::gen_mesh_cube(thread, 1.0, 1.0, 1.0),
            ground_mesh: Mesh::gen_mesh_plane(thread, GROUND_TILES, GROUND_TILES, 1, 1),
            star_mesh,
            terrain: None,
            terrain_material,
            star_material,
            box_material,
            ground_material,
            entity_material,
            _shaders: [
                box_shader,
                ground_shader,
                terrain_shader,
                star_shader,
                entity_shader,
            ],
            sea_globe_shader,
            sea_globe_material,
            object_shader,
            object_material,
            features: None,
            looks: Arc::new(SceneLooks::default()),
            detail: 0.0,
            clock: Instant::now(),
            caster_scratch: Vec::new(),
            noise,
            crop_scratch: Vec::new(),
            sun: LIGHT_DIRECTION,
            sky: sky::SkyGpu::load(rl, thread),
            shadow: None,
            graticule: Vec::new(),
            show_graticule: true,
            globe: None,
            space: None,
            space_state: space::SpaceState::default(),
            entity_transforms: Vec::new(),
            entity_casters: Vec::new(),
            caster_ranges: Vec::new(),
            instances: Vec::new(),
            draw_markers: false,
            transforms: Vec::new(),
            movers: Vec::new(),
            overlay: OverlayGeometry::default(),
            last_shades: Vec::new(),
            shade_queue: Vec::new(),
        }
    }

    /// The surface of the published terrain, for height sampling.
    pub(crate) fn surface(&self) -> Option<&Surface> {
        self.terrain.as_ref().map(|t| &t.surface)
    }

    /// Builds the selection, marker and placement overlays for the next draw.
    pub(crate) fn prepare_overlays(
        &mut self,
        views: &[EntityView],
        hud: &HudSnapshot,
        camera_yaw: f32,
        orbit: Option<&OrbitCamera>,
    ) {
        if let (Some(gpu), Some(orbit)) = (&self.globe, orbit) {
            build_overlays_on(views, &*gpu.globe, orbit.right(), hud, &mut self.overlay);
            return;
        }
        match &self.terrain {
            Some(terrain) => {
                let surface = &terrain.surface;
                build_overlays(
                    views,
                    &|x, y| surface.height(x, y),
                    hud,
                    camera_yaw,
                    &mut self.overlay,
                );
            }
            None => {
                self.overlay.lines.clear();
                self.overlay.tris.clear();
            }
        }
    }

    /// Takes new tile `shades` of a vision map (fog of war and borders) and queues the chunks
    /// near tiles whose shade changed for recolouring; [`Self::run_shade_queue`] does the work a
    /// few chunks per frame so no frame pays for a whole update.
    pub(crate) fn apply_shades(&mut self, shades: &[[f32; 3]]) {
        let changed: Vec<bool> = if self.last_shades.len() == shades.len() {
            shades
                .iter()
                .zip(&self.last_shades)
                .map(|(a, b)| a != b)
                .collect()
        } else {
            vec![true; shades.len()]
        };
        let mut queue = std::mem::take(&mut self.shade_queue);
        if let Some(gpu) = &self.globe {
            let n = gpu.planet.n;
            if shades.len() != 6 * (n as usize) * (n as usize) {
                self.shade_queue = queue;
                return;
            }
            let globe = Arc::clone(&gpu.globe);
            let adjoining = |face: u8, vx: u32, vy: u32| globe.corner_tiles(face, vx, vy);
            let meshes = gpu.land.iter().chain(gpu.sea.iter());
            for (index, (mesh, _)) in meshes.enumerate() {
                if !queue.contains(&index)
                    && chunk_touches_change(mesh.spec, n, &changed, Some(&adjoining))
                {
                    queue.push(index);
                }
            }
        } else if let Some(terrain) = &self.terrain {
            let (width, height) = terrain.size;
            if shades.len() != width as usize * height as usize {
                self.shade_queue = queue;
                return;
            }
            let meshes = terrain.land.iter().chain(terrain.sea.iter());
            for (index, mesh) in meshes.enumerate() {
                if !queue.contains(&index)
                    && flat_chunk_touches_change(mesh.spec, width, height, &changed)
                {
                    queue.push(index);
                }
            }
        } else {
            self.shade_queue = queue;
            return;
        }
        self.shade_queue = queue;
        if let Some(features) = &mut self.features {
            features.apply_shades(shades);
        }
        self.last_shades.clear();
        self.last_shades.extend_from_slice(shades);
    }

    /// Sets the per-frame uniforms of the material shaders for an eye at `eye` and a camera
    /// `distance` from its target.
    fn set_material_frame(&mut self, eye: [f32; 3], distance: f32) {
        self.detail = lod_weight(distance);
        let time = self.clock.elapsed().as_secs_f32() % 3_600.0;
        let [_, _, terrain, _, _] = &mut self._shaders;
        for shader in [terrain, &mut self.sea_globe_shader, &mut self.object_shader] {
            materials::set_frame(shader, eye, self.detail, time);
        }

        let location = self.object_shader.get_shader_location("planetRadius");
        let radius = self.globe.as_ref().map_or(0.0, |g| g.globe.radius());
        self.object_shader.set_shader_value(location, radius);
    }

    /// Recolours up to `budget` queued chunks with the newest shades.
    pub(crate) fn run_shade_queue(&mut self, budget: usize) {
        if self.shade_queue.is_empty() {
            return;
        }
        let take = budget.min(self.shade_queue.len());
        let batch: Vec<usize> = self.shade_queue.drain(..take).collect();
        let shades = std::mem::take(&mut self.last_shades);
        let mut scratch = Vec::new();
        if let Some(gpu) = &mut self.globe {
            let n = gpu.planet.n as usize;
            let globe = Arc::clone(&gpu.globe);
            let side = n as u32;
            let adjoining = |face: u8, vx: u32, vy: u32| {
                if (1..side).contains(&vx) && (1..side).contains(&vy) {
                    // Inside a face the four tiles around the corner are plain neighbours.
                    let first = (usize::from(face) * n + (vy as usize - 1)) * n + (vx as usize - 1);
                    ([first, first + 1, first + n, first + n + 1], 4)
                } else {
                    globe.corner_tiles(face, vx, vy)
                }
            };
            let lands = gpu.land.len();
            for index in batch {
                let mesh = if index < lands {
                    gpu.land.get_mut(index)
                } else {
                    gpu.sea.get_mut(index - lands)
                };
                if let Some((mesh, _)) = mesh {
                    mesh.apply(&shades, &adjoining, &mut scratch);
                }
            }
        } else if let Some(terrain) = &mut self.terrain {
            let (width, height) = (i64::from(terrain.size.0), i64::from(terrain.size.1));
            let adjoining = |_: u8, vx: u32, vy: u32| {
                let mut out = [0; 4];
                let mut count = 0;
                for (dx, dy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
                    let (x, y) = (i64::from(vx) + dx, i64::from(vy) + dy);
                    if (0..width).contains(&x) && (0..height).contains(&y) {
                        out[count] = (y * width + x) as usize;
                        count += 1;
                    }
                }
                (out, count)
            };
            let lands = terrain.land.len();
            for index in batch {
                let mesh = if index < lands {
                    terrain.land.get_mut(index)
                } else {
                    terrain.sea.get_mut(index - lands)
                };
                if let Some(mesh) = mesh {
                    mesh.apply(&shades, &adjoining, &mut scratch);
                }
            }
        }
        self.last_shades = shades;
    }

    /// Builds and uploads the meshes of `scene`, replacing any earlier terrain.
    ///
    /// Returns the number of meshes, the mesh build time and the upload time in milliseconds.
    pub(crate) fn set_terrain(
        &mut self,
        thread: &RaylibThread,
        scene: &Arc<TerrainScene>,
        looks: &Arc<SceneLooks>,
    ) -> (usize, f32, f32, BakeReport) {
        self.terrain = None;
        self.globe = None;
        self.space = None;
        self.features = None;
        self.last_shades.clear();
        self.shade_queue.clear();
        self.graticule.clear();
        self.sun = LIGHT_DIRECTION;
        if let Some(shadow) = &mut self.shadow {
            shadow.invalidate();
        }
        if !scene.is_valid() {
            eprintln!("rom-render: ignoring a terrain scene with inconsistent tile data");
            return (0, 0.0, 0.0, BakeReport::default());
        }
        let started = Instant::now();
        let surface = Surface::new(Arc::clone(scene));
        let shape = surface.shape();
        let land = build_terrain_chunks_with_specs(scene, shape);
        let sea = build_sea_chunks_with_specs(scene, shape);
        let slots = surface_slots(&surface, looks);
        let (width, height) = (i64::from(scene.width), i64::from(scene.height));
        let adjoining = |_: u8, vx: u32, vy: u32| {
            let mut out = [0; 4];
            let mut count = 0;
            for (dx, dy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
                let (x, y) = (i64::from(vx) + dx, i64::from(vy) + dy);
                if (0..width).contains(&x) && (0..height).contains(&y) {
                    out[count] = (y * width + x) as usize;
                    count += 1;
                }
            }
            (out, count)
        };
        let built = Instant::now();
        let shaded = |chunks: &[(ChunkMesh, ChunkSpec)]| -> Vec<ShadedMesh> {
            chunks
                .iter()
                .filter_map(|(c, spec)| Some(ShadedMesh::new(upload(thread, c)?, *spec, c)))
                .collect()
        };
        let (land_meshes, land_bounds): (Vec<ShadedMesh>, Vec<ChunkBounds>) = land
            .iter()
            .filter_map(|(c, spec)| {
                let attributes = chunk_attributes(*spec, &slots.tile_slot, &adjoining, &c.colors);
                Some((
                    ShadedMesh::new(materials::upload_land(thread, c, &attributes)?, *spec, c),
                    ChunkBounds::of(c),
                ))
            })
            .unzip();
        let sea_meshes = shaded(&sea);
        let uploaded = Instant::now();
        let count = land_meshes.len() + sea_meshes.len();
        let baked = crate::features::bake_flat(&surface, looks);
        let report = self.adopt_looks(thread, looks, &slots, &baked, uploaded);

        let pillars = scene
            .starts
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let ground = ground_height(scene, shape, x, y);
                (
                    Vector3::new(x, ground + PILLAR_HEIGHT / 2.0, y),
                    Vector3::new(0.5, PILLAR_HEIGHT, 0.5),
                    PLAYER_COLORS[i % PLAYER_COLORS.len()],
                )
            })
            .collect();
        self.terrain = Some(TerrainGpu {
            land: land_meshes,
            land_bounds,
            sea: sea_meshes,
            pillars,
            surface,
            centre: [scene.width as f32 / 2.0, scene.height as f32 / 2.0],
            size: (scene.width, scene.height),
        });
        (
            count,
            (built - started).as_secs_f32() * 1_000.0,
            (uploaded - built).as_secs_f32() * 1_000.0,
            report,
        )
    }

    /// Takes the looks of a new planet or map: material slots and water for the shaders, and
    /// the baked feature objects (finished at `baked_from`) uploaded.
    fn adopt_looks(
        &mut self,
        thread: &RaylibThread,
        looks: &Arc<SceneLooks>,
        slots: &SurfaceSlots,
        baked: &BakedFeatures,
        baked_from: Instant,
    ) -> BakeReport {
        let bake_ms = baked_from.elapsed().as_secs_f32() * 1_000.0;
        self.looks = Arc::clone(looks);
        let [_, _, terrain, _, _] = &mut self._shaders;
        materials::set_slots(terrain, slots);
        materials::set_water(&mut self.sea_globe_shader, looks);
        let started = Instant::now();
        self.features = materials::FeatureGpu::upload(thread, baked);
        BakeReport {
            bake_ms,
            upload_ms: started.elapsed().as_secs_f32() * 1_000.0,
            meshes: baked.meshes.len(),
            vertices: baked.vertex_count(),
            instances: baked.instance_count(),
            chunks: baked.chunks.len(),
            slots: slots.slots.len(),
        }
    }

    /// Sets the light in the terrain, entity and sea shaders: the direction toward it, whether
    /// the scene is a sphere planet (with a night side) and the shadow map, if one is in use.
    fn set_lighting(
        &mut self,
        toward: [f32; 3],
        planet: bool,
        shadow: Option<shadow::ShadowUniforms>,
    ) {
        let [_, _, terrain, _, entity] = &mut self._shaders;
        let shaders = [
            terrain,
            entity,
            &mut self.sea_globe_shader,
            &mut self.object_shader,
        ];
        for shader in shaders {
            let at = |shader: &Shader, name: &str| shader.get_shader_location(name);
            let location = at(shader, "toSun");
            shader.set_shader_value(location, toward);
            let location = at(shader, "planet");
            shader.set_shader_value(location, i32::from(planet));
            let location = at(shader, "shadows");
            shader.set_shader_value(location, i32::from(shadow.is_some()));
            let location = at(shader, "shadowMap");
            shader.set_shader_value(location, shadow::SHADOW_SLOT);
            if let Some(uniforms) = &shadow {
                let location = at(shader, "lightVP");
                shader.set_shader_value_matrix(location, uniforms.light_vp);
                let location = at(shader, "depthBias");
                shader.set_shader_value(location, uniforms.depth_bias);
                let location = at(shader, "normalBias");
                shader.set_shader_value(location, uniforms.normal_bias);
            }
        }
    }

    /// Sets the direction the light that represents the sun travels, in planet space.
    pub(crate) fn set_sun(&mut self, light: [f32; 3]) {
        self.sun = light;
    }

    /// Uploads the chunk meshes of `globe`, replacing any earlier terrain or planet.
    ///
    /// Returns the number of meshes and the upload time in milliseconds.
    pub(crate) fn set_planet(
        &mut self,
        thread: &RaylibThread,
        planet: &Arc<PlanetScene>,
        globe: &Arc<Globe>,
        sun: [f32; 3],
        looks: &Arc<SceneLooks>,
    ) -> (usize, f32, f32, BakeReport) {
        self.terrain = None;
        self.globe = None;
        self.space = None;
        self.features = None;
        self.last_shades.clear();
        self.shade_queue.clear();
        self.sun = sun;
        if let Some(shadow) = &mut self.shadow {
            shadow.invalidate();
        }
        let started = Instant::now();
        self.graticule = graticule_on(globe, GRATICULE_LIFT);
        let land = globe.land_chunks_with_specs();
        let sea = globe.sea_chunks_with_specs();
        let slots = surface_slots(&**globe, looks);
        let adjoining = |face: u8, vx: u32, vy: u32| globe.corner_tiles(face, vx, vy);
        let built = Instant::now();
        let upload_all =
            |chunks: &[(ChunkMesh, ChunkSpec)], land: bool| -> Vec<(ShadedMesh, ChunkBounds)> {
                chunks
                    .iter()
                    .filter_map(|(c, spec)| {
                        let mesh = if land {
                            let attributes =
                                chunk_attributes(*spec, &slots.tile_slot, &adjoining, &c.colors);
                            materials::upload_land(thread, c, &attributes)?
                        } else {
                            upload(thread, c)?
                        };
                        Some((ShadedMesh::new(mesh, *spec, c), ChunkBounds::of(c)))
                    })
                    .collect()
            };
        let land = upload_all(&land, true);
        let sea = upload_all(&sea, false);
        let uploaded = Instant::now();
        let count = land.len() + sea.len();
        let baked = crate::features::bake_planet(globe, looks);
        let report = self.adopt_looks(thread, looks, &slots, &baked, uploaded);
        self.globe = Some(GlobeGpu {
            land,
            sea,
            planet: Arc::clone(planet),
            globe: Arc::clone(globe),
            markers: Vec::new(),
        });
        (
            count,
            (built - started).as_secs_f32() * 1_000.0,
            (uploaded - built).as_secs_f32() * 1_000.0,
            report,
        )
    }

    /// Draws the sphere planet seen by `orbit` into `target`.
    pub(crate) fn draw_globe(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        orbit: &OrbitCamera,
    ) {
        let Some(gpu) = &mut self.globe else {
            return;
        };
        let growth = (orbit.distance / MARKER_NEAR).clamp(1.0, MARKER_GROWTH);
        let mut instances = std::mem::take(&mut self.instances);
        gpu.globe
            .marker_instances(&gpu.planet, growth, &mut instances);
        gpu.markers.clear();
        gpu.markers.extend(instances.iter().map(matrix_of));
        self.instances = instances;
        let size = (target.texture.width as f32, target.texture.height as f32);
        self.set_material_frame(orbit.position(), orbit.distance);
        let shadow = self.globe_shadow_pass(rl, thread, orbit, size);
        let toward = sun::unit_or(self.sun.map(|c| -c));
        self.set_lighting(toward, true, shadow);
        let shadow_texture = shadow.and(self.shadow.as_ref()).map(|s| s.texture_id());
        let aspect = target.texture.width as f32 / target.texture.height.max(1) as f32;
        let cull = CullView::of_orbit(orbit, aspect, 45.0, 1.5);
        if let Some(features) = &mut self.features {
            let radius = orbit.radius;
            features.prepare(orbit.position(), &|centre, size| {
                let top = (centre[0] * centre[0] + centre[1] * centre[1] + centre[2] * centre[2])
                    .sqrt()
                    + size;
                ChunkBounds {
                    centre,
                    radius: size,
                    top,
                }
                .may_be_seen(radius, &cull)
            });
        }
        // Near the ground the sky of the point looked at; from higher up, black space.
        let altitude = orbit.altitude();
        let sun_height = (0..3).map(|i| orbit.dir[i] * toward[i]).sum::<f32>();
        let backdrop = crate::sky::backdrop(sun_height, altitude, orbit.radius);
        let dip = crate::sky::horizon_dip(altitude, orbit.radius);
        self.sky.set_sky(&backdrop, orbit.dir, toward, dip);
        self.set_star_fade(backdrop.stars);
        let halo = crate::sky::space_share(altitude, orbit.radius);
        self.sky.set_halo(
            [0.0; 3],
            orbit.radius,
            sky::HALO_THICKNESS,
            orbit.position(),
            toward,
            halo,
        );
        let Some(gpu) = &self.globe else {
            return;
        };

        let eye = orbit.position();
        let target_point = orbit.target();
        let up = orbit.view_up();
        let vector = |v: [f32; 3]| Vector3::new(v[0], v[1], v[2]);
        let view = Camera3D::perspective(vector(eye), vector(target_point), vector(up), 45.0);
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(SPACE);
        sky::draw_backdrop(
            &mut drawing,
            ([0, 1, 2].map(|i| target_point[i] - eye[i]), up),
            self.projection_centre_y,
            &backdrop,
            &self.sky,
            self.star_mesh.as_ref(),
            &self.star_material,
        );
        let radius = orbit.radius;
        let (near, far) = orbit.clip_planes(RELIEF);
        set_clip_planes(f64::from(near), f64::from(far));
        {
            let mut world = drawing.begin_mode3D(view);
            shift_projection(self.projection_centre_y);
            shadow::bind_shadow_map(shadow_texture);
            materials::bind_noise(self.noise.as_ref().map(|t| t.id));
            for (shaded, bounds) in &gpu.land {
                if bounds.may_be_seen(radius, &cull) {
                    world.draw_mesh(
                        &shaded.mesh,
                        self.terrain_material.clone(),
                        Matrix::identity(),
                    );
                }
            }
            draw_features(
                &mut world,
                self.detail,
                self.features.as_ref(),
                &mut self.object_shader,
                &self.object_material,
            );
            if !gpu.markers.is_empty() {
                world.draw_mesh_instanced(
                    &self.box_mesh,
                    self.entity_material.clone(),
                    &gpu.markers,
                );
            }
            if !self.entity_transforms.is_empty() {
                world.draw_mesh_instanced(
                    &self.box_mesh,
                    self.entity_material.clone(),
                    &self.entity_transforms,
                );
            }
            for (shaded, bounds) in &gpu.sea {
                if bounds.may_be_seen(radius, &cull) {
                    world.draw_mesh(
                        &shaded.mesh,
                        self.sea_globe_material.clone(),
                        Matrix::identity(),
                    );
                }
            }
            if halo > 0.01 {
                self.sky
                    .draw_halo(&mut world, [0.0; 3], radius, sky::HALO_THICKNESS);
            }
            if self.show_graticule {
                draw_graticule(
                    &mut world,
                    &self.graticule,
                    eye,
                    graticule_fade(orbit.distance),
                );
            }
            draw_overlay(&mut world, &self.overlay);
            shadow::bind_shadow_map(None);
            materials::bind_noise(None);
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
    }

    /// Turns `views` into instance transforms for the next draw; without a terrain nothing is drawn.
    pub(crate) fn prepare_entities(
        &mut self,
        views: &[EntityView],
        camera_yaw: f32,
        start_markers: bool,
        orbit: Option<&OrbitCamera>,
    ) {
        self.draw_markers = start_markers;
        self.entity_transforms.clear();
        self.entity_casters.clear();
        let mut instances = std::mem::take(&mut self.instances);
        let (ground, right): (&dyn Footing, [f32; 3]) =
            if let (Some(gpu), Some(orbit)) = (&self.globe, orbit) {
                (&*gpu.globe, orbit.right())
            } else if let Some(terrain) = &self.terrain {
                (&terrain.surface, [camera_yaw.cos(), 0.0, -camera_yaw.sin()])
            } else {
                self.instances = instances;
                return;
            };
        let looks = &self.looks;
        let objects = self.features.as_ref().map(|f| &f.buildings);
        let drawn_as_objects = |kind: u16| {
            looks.building(kind).is_some() && objects.is_some_and(|b| b.contains_key(&kind))
        };
        build_instances_with_casters_on(
            views,
            ground,
            right,
            &mut instances,
            &mut self.caster_ranges,
            &drawn_as_objects,
        );
        if let Some(features) = &mut self.features {
            for list in &mut features.crops {
                list.clear();
            }
            let mut placed = std::mem::take(&mut self.crop_scratch);
            for view in views.iter().filter(|v| v.building) {
                let (Some(look), Some(variants)) = (
                    looks.building(view.kind),
                    features.buildings.get(&view.kind),
                ) else {
                    continue;
                };
                placed.clear();
                crop_instances(view, ground, look, variants, &mut placed);
                for (mesh, transform) in &placed {
                    if let Some(list) = features.crops.get_mut(usize::from(*mesh)) {
                        list.push(matrix_of(transform));
                    }
                }
            }
            self.crop_scratch = placed;
        }
        self.entity_transforms
            .extend(instances.iter().map(matrix_of));
        for range in &self.caster_ranges {
            self.entity_casters
                .extend_from_slice(&self.entity_transforms[range.clone()]);
        }
        self.instances = instances;
    }

    /// Draws the scene for `camera` at `seconds` into `target`.
    pub(crate) fn draw(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        camera: &Camera,
        seconds: f32,
        box_count: usize,
    ) {
        if self.movers.len() != box_count {
            self.movers = movers(box_count);
        }
        let offset = self.terrain.as_ref().map_or([0.0, 0.0], |t| t.centre);
        self.transforms.clear();
        self.transforms.extend(self.movers.iter().map(|mover| {
            let mut transform = mover_transform(mover, seconds);
            transform.m12 += offset[0];
            transform.m14 += offset[1];
            transform
        }));
        let size = (target.texture.width as f32, target.texture.height as f32);
        let eye = camera.position();
        self.set_material_frame(eye, camera.distance);
        if let Some(features) = &mut self.features {
            features.prepare(eye, &|_, _| true);
        }
        let shadow = if self.terrain.is_some() {
            self.flat_shadow_pass(rl, thread, camera, size)
        } else {
            None
        };
        self.set_lighting(sun::unit_or(LIGHT_DIRECTION.map(|c| -c)), false, shadow);
        let shadow_texture = shadow.and(self.shadow.as_ref()).map(|s| s.texture_id());
        let view = Camera3D::perspective(
            Vector3::new(eye[0], eye[1], eye[2]),
            Vector3::new(camera.target[0], camera.target[1], camera.target[2]),
            Vector3::new(0.0, 1.0, 0.0),
            45.0,
        );
        // The same sky as on a planet, under the flat map's fixed light.
        let toward = sun::unit_or(LIGHT_DIRECTION.map(|c| -c));
        let backdrop = crate::sky::local_sky(toward[1]);
        self.sky.set_sky(&backdrop, [0.0, 1.0, 0.0], toward, 0.0);
        self.set_star_fade(backdrop.stars);
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(if self.terrain.is_some() { SPACE } else { SKY });
        if self.terrain.is_some() {
            // A camera at the origin with the same orientation: the sky and the stars turn with
            // the view but never move when the camera pans.
            sky::draw_backdrop(
                &mut drawing,
                (
                    [0, 1, 2].map(|i| camera.target[i] - eye[i]),
                    [0.0, 1.0, 0.0],
                ),
                self.projection_centre_y,
                &backdrop,
                &self.sky,
                self.star_mesh.as_ref(),
                &self.star_material,
            );
        }
        let mut world = drawing.begin_mode3D(view);
        shift_projection(self.projection_centre_y);
        shadow::bind_shadow_map(shadow_texture);
        materials::bind_noise(self.noise.as_ref().map(|t| t.id));
        match &self.terrain {
            Some(terrain) => {
                for shaded in &terrain.land {
                    world.draw_mesh(
                        &shaded.mesh,
                        self.terrain_material.clone(),
                        Matrix::identity(),
                    );
                }
                draw_features(
                    &mut world,
                    self.detail,
                    self.features.as_ref(),
                    &mut self.object_shader,
                    &self.object_material,
                );
                if self.draw_markers {
                    for (centre, size, color) in &terrain.pillars {
                        world.draw_cube_v(*centre, *size, *color);
                    }
                }
                if !self.entity_transforms.is_empty() {
                    world.draw_mesh_instanced(
                        &self.box_mesh,
                        self.entity_material.clone(),
                        &self.entity_transforms,
                    );
                }
                if !self.transforms.is_empty() {
                    world.draw_mesh_instanced(
                        &self.box_mesh,
                        self.box_material.clone(),
                        &self.transforms,
                    );
                }
                for shaded in &terrain.sea {
                    world.draw_mesh(
                        &shaded.mesh,
                        self.sea_globe_material.clone(),
                        Matrix::identity(),
                    );
                }
                draw_overlay(&mut world, &self.overlay);
                shadow::bind_shadow_map(None);
                materials::bind_noise(None);
            }
            None => {
                world.draw_mesh(
                    &self.ground_mesh,
                    self.ground_material.clone(),
                    Matrix::identity(),
                );
                if !self.transforms.is_empty() {
                    world.draw_mesh_instanced(
                        &self.box_mesh,
                        self.box_material.clone(),
                        &self.transforms,
                    );
                }
            }
        }
    }
}

/// Whether a tile near the chunk's vertices changed shade: the tiles of its window and, on a
/// sphere, the tiles across a face edge that touch its edge vertices.
fn chunk_touches_change(
    spec: ChunkSpec,
    n: u32,
    changed: &[bool],
    adjoining: Option<crate::vision::Adjoining>,
) -> bool {
    let face_base = usize::from(spec.face) * (n as usize) * (n as usize);
    let (x_lo, x_hi) = (spec.x0.saturating_sub(1), (spec.x0 + spec.w).min(n - 1));
    let (y_lo, y_hi) = (spec.y0.saturating_sub(1), (spec.y0 + spec.h).min(n - 1));
    for y in y_lo..=y_hi {
        let row = face_base + y as usize * n as usize;
        if changed[row + x_lo as usize..=row + x_hi as usize]
            .iter()
            .any(|c| *c)
        {
            return true;
        }
    }
    let Some(adjoining) = adjoining else {
        return false;
    };
    let on_edge = |v: u32| v == 0 || v == n;
    for j in 0..=spec.h {
        for i in 0..=spec.w {
            let (vx, vy) = (spec.x0 + i, spec.y0 + j);
            if on_edge(vx) || on_edge(vy) {
                let (tiles, count) = adjoining(spec.face, vx, vy);
                if tiles[..count].iter().any(|&t| changed[t]) {
                    return true;
                }
            }
        }
    }
    false
}

/// The flat-map form of [`chunk_touches_change`].
fn flat_chunk_touches_change(spec: ChunkSpec, width: u32, height: u32, changed: &[bool]) -> bool {
    let (x_lo, x_hi) = (spec.x0.saturating_sub(1), (spec.x0 + spec.w).min(width - 1));
    let (y_lo, y_hi) = (
        spec.y0.saturating_sub(1),
        (spec.y0 + spec.h).min(height - 1),
    );
    (y_lo..=y_hi).any(|y| {
        let row = y as usize * width as usize;
        changed[row + x_lo as usize..=row + x_hi as usize]
            .iter()
            .any(|c| *c)
    })
}

/// Draws the feature objects and crops prepared for this frame, when `detail` shows them.
fn draw_features(
    world: &mut impl RaylibDraw3D,
    detail: f32,
    features: Option<&materials::FeatureGpu>,
    shader: &mut Shader,
    material: &WeakMaterial,
) {
    if detail <= 0.01 {
        return;
    }
    if let Some(features) = features {
        features.draw(world, shader, material);
    }
}

/// Draws the overlay triangles and lines of one frame.
fn draw_overlay(world: &mut impl RaylibDraw3D, overlay: &OverlayGeometry) {
    for tri in &overlay.tris {
        let colour = Color::new(tri.colour[0], tri.colour[1], tri.colour[2], tri.colour[3]);
        let [a, b, c] = tri.v.map(|p| Vector3::new(p[0], p[1], p[2]));
        world.draw_triangle3D(a, b, c, colour);
        world.draw_triangle3D(a, c, b, colour);
    }
    for line in &overlay.lines {
        let colour = Color::new(
            line.colour[0],
            line.colour[1],
            line.colour[2],
            line.colour[3],
        );
        world.draw_line3D(
            Vector3::new(line.a[0], line.a[1], line.a[2]),
            Vector3::new(line.b[0], line.b[1], line.b[2]),
            colour,
        );
    }
}

/// Height of the graticule above the ground, in tiles.
const GRATICULE_LIFT: f32 = 0.25;

/// Draws the latitude and longitude lines on the side of the planet facing `eye`, their colours
/// scaled by `fade`.
fn draw_graticule(
    world: &mut impl RaylibDraw3D,
    lines: &[GraticuleLine],
    eye: [f32; 3],
    fade: f32,
) {
    if fade <= 0.0 {
        return;
    }
    // A point on the planet faces the eye when the eye lies above its horizon plane.
    let faces = |p: [f32; 3]| {
        let along = p[0] * eye[0] + p[1] * eye[1] + p[2] * eye[2];
        along > 0.97 * (p[0] * p[0] + p[1] * p[1] + p[2] * p[2])
    };
    for line in lines {
        let [r, g, b, a] = line.kind.colour();
        let colour = Color::new(r, g, b, (f32::from(a) * fade).round() as u8);
        for pair in line.points.windows(2) {
            if faces(pair[0]) || faces(pair[1]) {
                world.draw_line3D(
                    Vector3::new(pair[0][0], pair[0][1], pair[0][2]),
                    Vector3::new(pair[1][0], pair[1][1], pair[1][2]),
                    colour,
                );
            }
        }
    }
}

/// raylib's default clip planes, restored after a sphere frame.
const DEFAULT_NEAR: f64 = 0.05;
const DEFAULT_FAR: f64 = 4000.0;

/// Sets the near and far clip planes used by the next 3D mode.
fn set_clip_planes(near: f64, far: f64) {
    // SAFETY: the render thread has an active GL context; this only stores two numbers.
    unsafe { raylib::ffi::rlSetClipPlanes(near, far) }
}

/// Moves the perspective vanishing point to the centre of the unobscured world band.
fn shift_projection(centre_y: f32) {
    // SAFETY: the render thread has an active GL context and has just entered 3D mode.
    unsafe {
        let mut projection = raylib::ffi::rlGetMatrixProjection();
        projection.m9 = -centre_y;
        raylib::ffi::rlSetMatrixProjection(projection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_sun_lights_the_home_start() {
        let normals = [
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.3, -0.5, -0.8],
            [-0.7, 0.2, 0.69],
        ];
        for normal in normals {
            let length = normal.iter().map(|c| c * c).sum::<f32>().sqrt();
            let n = normal.map(|c| c / length);
            let toward = sun::fixed_toward(n);
            let facing: f32 = (0..3).map(|i| toward[i] * n[i]).sum();
            assert!(facing > 0.5, "{n:?} gets {facing}");
            let norm: f32 = toward.iter().map(|c| c * c).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn movers_are_deterministic_and_on_the_map() {
        let first = movers(5_000);
        assert_eq!(first, movers(5_000));
        let half = GROUND_TILES / 2.0;
        for mover in &first {
            for seconds in [0.0, 7.5] {
                let transform = mover_transform(mover, seconds);
                assert!(transform.m12.abs() < half && transform.m14.abs() < half);
            }
        }
    }
}

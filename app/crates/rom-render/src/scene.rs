//! The world scene: terrain, sea, star field and markers, plus the optional stress-test boxes.
//!
//! Without a terrain it falls back to the original proof scene (a grid ground).
//! The boxes are a display test, not game content; their motion is computed here
//! from elapsed time only.

use std::time::Instant;

use raylib::prelude::{
    Camera3D, Color, Matrix, Mesh, RaylibDraw, RaylibDraw3D, RaylibHandle, RaylibMaterial,
    RaylibMesh, RaylibMode3DExt, RaylibShader, RaylibTextureModeExt, RaylibThread, RenderTexture2D,
    Shader, Vector2, Vector3, WeakMaterial,
};

use std::sync::Arc;

use crate::camera::Camera;
use crate::entities::{EntityView, Surface, build_instances};
use crate::terrain::{
    ChunkMesh, STAR_COUNT, TerrainScene, build_sea_chunks, build_terrain_chunks, ground_height,
    star_field, star_triangles,
};

/// Width of the ground in tiles.
const GROUND_TILES: f32 = 256.0;
/// Direction the sunlight travels (normalised in the shaders).
const LIGHT_DIRECTION: [f32; 3] = [-0.45, -1.0, -0.3];
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

const TERRAIN_VS: &str = r#"#version 330
in vec3 vertexPosition;
in vec3 vertexNormal;
in vec4 vertexColor;
uniform mat4 mvp;
out vec3 fragNormal;
out vec4 fragColor;
void main() {
    fragNormal = vertexNormal;
    fragColor = vertexColor;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

const TERRAIN_FS: &str = r#"#version 330
in vec3 fragNormal;
in vec4 fragColor;
uniform vec3 lightDirection;
out vec4 finalColor;
void main() {
    float diffuse = max(dot(normalize(fragNormal), -normalize(lightDirection)), 0.0);
    finalColor = vec4(fragColor.rgb * (0.32 + 0.78 * diffuse), 1.0);
}
"#;

const COLOR_VS: &str = r#"#version 330
in vec3 vertexPosition;
in vec4 vertexColor;
uniform mat4 mvp;
out vec4 fragColor;
void main() {
    fragColor = vertexColor;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

const COLOR_FS: &str = r#"#version 330
in vec4 fragColor;
out vec4 finalColor;
void main() {
    finalColor = fragColor;
}
"#;

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
flat out vec3 fragTint;
void main() {
    mat4 transform = instanceTransform;
    fragTint = vec3(transform[0][3], transform[1][3], transform[2][3]);
    transform[0][3] = 0.0;
    transform[1][3] = 0.0;
    transform[2][3] = 0.0;
    fragNormal = mat3(transform) * vertexNormal;
    gl_Position = mvp * transform * vec4(vertexPosition, 1.0);
}
"#;

const ENTITY_FS: &str = r#"#version 330
in vec3 fragNormal;
flat in vec3 fragTint;
uniform vec3 lightDirection;
out vec4 finalColor;
void main() {
    float diffuse = max(dot(normalize(fragNormal), -normalize(lightDirection)), 0.0);
    finalColor = vec4(fragTint * (0.38 + 0.72 * diffuse), 1.0);
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
    sea_material: WeakMaterial,
    star_material: WeakMaterial,
    // raylib-rs hands out the default material only as a non-owning handle and frees it
    // only through an `unsafe` call, so the two small map tables are left to process exit.
    // Their GPU objects (the shaders below) are freed normally.
    box_material: WeakMaterial,
    ground_material: WeakMaterial,
    /// Kept alive for the materials above, which point at them.
    entity_material: WeakMaterial,
    _shaders: [Shader; 6],
    entity_transforms: Vec<Matrix>,
    instances: Vec<[f32; 16]>,
    draw_markers: bool,
    movers: Vec<Mover>,
    transforms: Vec<Matrix>,
}

/// Meshes and markers of the published terrain.
struct TerrainGpu {
    land: Vec<Mesh>,
    sea: Vec<Mesh>,
    /// Player pillars: centre, size and colour.
    pillars: Vec<(Vector3, Vector3, Color)>,
    /// Surface sampler for entities.
    surface: Surface,
    /// Map centre in tiles, where the stress-test boxes are centred.
    centre: [f32; 2],
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
        let mut terrain_shader =
            rl.load_shader_from_memory(thread, Some(TERRAIN_VS), Some(TERRAIN_FS));
        let mut entity_shader =
            rl.load_shader_from_memory(thread, Some(ENTITY_VS), Some(ENTITY_FS));
        let sea_shader = rl.load_shader_from_memory(thread, Some(COLOR_VS), Some(COLOR_FS));
        let star_shader = rl.load_shader_from_memory(thread, Some(STAR_VS), Some(COLOR_FS));
        for shader in [
            &mut box_shader,
            &mut ground_shader,
            &mut terrain_shader,
            &mut entity_shader,
        ] {
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
        let mut sea_material = rl.load_material_default(thread);
        sea_material.set_shader(&sea_shader);
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
            sea_material,
            star_material,
            box_material,
            ground_material,
            entity_material,
            _shaders: [
                box_shader,
                ground_shader,
                terrain_shader,
                sea_shader,
                star_shader,
                entity_shader,
            ],
            entity_transforms: Vec::new(),
            instances: Vec::new(),
            draw_markers: false,
            transforms: Vec::new(),
            movers: Vec::new(),
        }
    }

    /// Builds and uploads the meshes of `scene`, replacing any earlier terrain.
    ///
    /// Returns the number of meshes, the mesh build time and the upload time in milliseconds.
    pub(crate) fn set_terrain(
        &mut self,
        thread: &RaylibThread,
        scene: &Arc<TerrainScene>,
    ) -> (usize, f32, f32) {
        self.terrain = None;
        if !scene.is_valid() {
            eprintln!("rom-render: ignoring a terrain scene with inconsistent tile data");
            return (0, 0.0, 0.0);
        }
        let started = Instant::now();
        let surface = Surface::new(Arc::clone(scene));
        let shape = surface.shape();
        let land = build_terrain_chunks(scene, shape);
        let sea = build_sea_chunks(scene, shape);
        let built = Instant::now();
        let land_meshes: Vec<Mesh> = land.iter().filter_map(|c| upload(thread, c)).collect();
        let sea_meshes: Vec<Mesh> = sea.iter().filter_map(|c| upload(thread, c)).collect();
        let uploaded = Instant::now();
        let count = land_meshes.len() + sea_meshes.len();

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
            sea: sea_meshes,
            pillars,
            surface,
            centre: [scene.width as f32 / 2.0, scene.height as f32 / 2.0],
        });
        (
            count,
            (built - started).as_secs_f32() * 1_000.0,
            (uploaded - built).as_secs_f32() * 1_000.0,
        )
    }

    /// Turns `views` into instance transforms for the next draw; without a terrain nothing is drawn.
    pub(crate) fn prepare_entities(
        &mut self,
        views: &[EntityView],
        camera_yaw: f32,
        start_markers: bool,
    ) {
        self.draw_markers = start_markers;
        self.entity_transforms.clear();
        let Some(terrain) = &self.terrain else {
            return;
        };
        let mut instances = std::mem::take(&mut self.instances);
        build_instances(views, &terrain.surface, camera_yaw, &mut instances);
        self.entity_transforms
            .extend(instances.iter().map(matrix_of));
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
        let eye = camera.position();
        let view = Camera3D::perspective(
            Vector3::new(eye[0], eye[1], eye[2]),
            Vector3::new(camera.target[0], camera.target[1], camera.target[2]),
            Vector3::new(0.0, 1.0, 0.0),
            45.0,
        );
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(if self.terrain.is_some() { SPACE } else { SKY });
        if let (Some(_), Some(stars)) = (&self.terrain, &self.star_mesh) {
            // A camera at the origin with the same orientation: the stars turn with the view
            // but never move when the camera pans.
            let forward = Vector3::new(
                camera.target[0] - eye[0],
                camera.target[1] - eye[1],
                camera.target[2] - eye[2],
            );
            let sky = Camera3D::perspective(
                Vector3::new(0.0, 0.0, 0.0),
                forward,
                Vector3::new(0.0, 1.0, 0.0),
                45.0,
            );
            let mut backdrop = drawing.begin_mode3D(sky);
            shift_projection(self.projection_centre_y);
            backdrop.draw_mesh(stars, self.star_material.clone(), Matrix::identity());
        }
        let mut world = drawing.begin_mode3D(view);
        shift_projection(self.projection_centre_y);
        match &self.terrain {
            Some(terrain) => {
                for mesh in &terrain.land {
                    world.draw_mesh(mesh, self.terrain_material.clone(), Matrix::identity());
                }
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
                for mesh in &terrain.sea {
                    world.draw_mesh(mesh, self.sea_material.clone(), Matrix::identity());
                }
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

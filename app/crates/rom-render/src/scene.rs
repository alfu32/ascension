//! Proof scene: a 256×256-tile grid ground and 5,000 instanced boxes on small circles.
//!
//! This is a display test, not game content. Box motion is computed here from
//! elapsed time only so the frame has something to draw.

use raylib::prelude::{
    Camera3D, Color, Matrix, Mesh, RaylibDraw, RaylibDraw3D, RaylibHandle, RaylibMaterial,
    RaylibMesh, RaylibMode3DExt, RaylibShader, RaylibTextureModeExt, RaylibThread, RenderTexture2D,
    Shader, Vector3, WeakMaterial,
};

use crate::camera::Camera;

/// Number of moving boxes in the proof scene.
pub(crate) const BOX_COUNT: usize = 5_000;
/// Width of the ground in tiles.
const GROUND_TILES: f32 = 256.0;
/// Direction the sunlight travels (normalised in the shaders).
const LIGHT_DIRECTION: [f32; 3] = [-0.45, -1.0, -0.3];
const SKY: Color = Color::new(146, 178, 204, 255);

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
    box_mesh: Mesh,
    ground_mesh: Mesh,
    // raylib-rs hands out the default material only as a non-owning handle and frees it
    // only through an `unsafe` call, so the two small map tables are left to process exit.
    // Their GPU objects (the shaders below) are freed normally.
    box_material: WeakMaterial,
    ground_material: WeakMaterial,
    /// Kept alive for the materials above, which point at them.
    _shaders: [Shader; 2],
    movers: Vec<Mover>,
    transforms: Vec<Matrix>,
}

impl Scene {
    pub(crate) fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let mut box_shader = rl.load_shader_from_memory(thread, Some(BOX_VS), Some(BOX_FS));
        let mut ground_shader =
            rl.load_shader_from_memory(thread, Some(GROUND_VS), Some(GROUND_FS));
        for shader in [&mut box_shader, &mut ground_shader] {
            let location = shader.get_shader_location("lightDirection");
            shader.set_shader_value(location, LIGHT_DIRECTION);
        }
        let mut box_material = rl.load_material_default(thread);
        box_material.set_shader(&box_shader);
        let mut ground_material = rl.load_material_default(thread);
        ground_material.set_shader(&ground_shader);
        let movers = movers(BOX_COUNT);
        Self {
            box_mesh: Mesh::gen_mesh_cube(thread, 1.0, 1.0, 1.0),
            ground_mesh: Mesh::gen_mesh_plane(thread, GROUND_TILES, GROUND_TILES, 1, 1),
            box_material,
            ground_material,
            _shaders: [box_shader, ground_shader],
            transforms: Vec::with_capacity(movers.len()),
            movers,
        }
    }

    /// Draws the scene for `camera` at `seconds` into `target`.
    pub(crate) fn draw(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        target: &mut RenderTexture2D,
        camera: &Camera,
        seconds: f32,
    ) {
        self.transforms.clear();
        self.transforms.extend(
            self.movers
                .iter()
                .map(|mover| mover_transform(mover, seconds)),
        );
        let eye = camera.position();
        let view = Camera3D::perspective(
            Vector3::new(eye[0], eye[1], eye[2]),
            Vector3::new(camera.target[0], camera.target[1], camera.target[2]),
            Vector3::new(0.0, 1.0, 0.0),
            45.0,
        );
        let mut drawing = rl.begin_texture_mode(thread, target);
        drawing.clear_background(SKY);
        let mut world = drawing.begin_mode3D(view);
        world.draw_mesh(
            &self.ground_mesh,
            self.ground_material.clone(),
            Matrix::identity(),
        );
        world.draw_mesh_instanced(&self.box_mesh, self.box_material.clone(), &self.transforms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movers_are_deterministic_and_on_the_map() {
        let first = movers(BOX_COUNT);
        assert_eq!(first, movers(BOX_COUNT));
        let half = GROUND_TILES / 2.0;
        for mover in &first {
            for seconds in [0.0, 7.5] {
                let transform = mover_transform(mover, seconds);
                assert!(transform.m12.abs() < half && transform.m14.abs() < half);
            }
        }
    }
}

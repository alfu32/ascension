//! The sky dome behind the terrain, the fading star field and the atmosphere halo around
//! habitable planets. The colours come from [`crate::sky`]; this draws them.

use super::*;
pub(super) use crate::sky::HALO_THICKNESS;
use crate::sky::Sky;

/// Halo thickness in the system view, in radii: thicker than on the globe so it shows on the
/// small planets there.
pub(super) const SYSTEM_HALO_THICKNESS: f32 = 0.16;

// The dome sits at the far plane like the stars; its vertex position is the view direction.
const SKY_VS: &str = r#"#version 330
in vec3 vertexPosition;
uniform mat4 mvp;
out vec3 fragDir;
void main() {
    fragDir = vertexPosition;
    gl_Position = (mvp * vec4(vertexPosition, 1.0)).xyww;
}
"#;

pub(super) const STAR_FS: &str = r#"#version 330
in vec4 fragColor;
uniform float fade;
out vec4 finalColor;
void main() {
    finalColor = vec4(fragColor.rgb, fragColor.a * fade);
}
"#;

const HALO_VS: &str = r#"#version 330
in vec3 vertexPosition;
uniform mat4 mvp;
uniform mat4 matModel;
out vec3 fragWorld;
void main() {
    fragWorld = (matModel * vec4(vertexPosition, 1.0)).xyz;
    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
"#;

/// The halo: the far half of a shell a little larger than the planet, coloured where the view ray
/// passes beside the planet, by how far out it passes and how much daylight is there.
fn halo_fs() -> String {
    format!(
        r#"#version 330
in vec3 fragWorld;
uniform vec3 centre;
uniform float radius;
uniform float shell;
uniform vec3 eye;
uniform vec3 toSun;
uniform float strength;
out vec4 finalColor;
{}
void main() {{
    vec3 ray = normalize(fragWorld - eye);
    if (dot(fragWorld - centre, ray) < 0.0) discard;
    vec3 closest = eye + ray * dot(centre - eye, ray) - centre;
    float d = length(closest);
    if (d < radius) discard;
    float t = (d - radius) / (shell - radius);
    float sunDot = dot(closest / d, toSun);
    vec3 colour = mix(ATMOSPHERE, WARM, haloWarmth(sunDot));
    finalColor = vec4(colour, halo(sunDot, t) * strength);
}}
"#,
        crate::sky::glsl_halo()
    )
}

/// GPU objects of the sky dome and the halo.
pub(super) struct SkyGpu {
    dome: Mesh,
    shell: Mesh,
    sky_shader: Shader,
    sky_material: WeakMaterial,
    halo_shader: Shader,
    halo_material: WeakMaterial,
}

impl SkyGpu {
    pub(super) fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let sky_shader =
            rl.load_shader_from_memory(thread, Some(SKY_VS), Some(crate::sky::DOME_FS));
        let mut sky_material = rl.load_material_default(thread);
        sky_material.set_shader(&sky_shader);
        let halo_shader = rl.load_shader_from_memory(thread, Some(HALO_VS), Some(&halo_fs()));
        let mut halo_material = rl.load_material_default(thread);
        halo_material.set_shader(&halo_shader);
        Self {
            dome: space::upload_sphere(thread, 16, 32),
            shell: space::upload_sphere(thread, 24, 48),
            sky_shader,
            sky_material,
            halo_shader,
            halo_material,
        }
    }

    /// Sets the dome's colours for `sky`, with `up` the local vertical, `toward` the light and
    /// `dip` the sine of the angle the horizon lies below the horizontal.
    pub(super) fn set_sky(&mut self, sky: &Sky, up: [f32; 3], toward: [f32; 3], dip: f32) {
        let shader = &mut self.sky_shader;
        let location = shader.get_shader_location("dip");
        shader.set_shader_value(location, dip);
        for (name, value) in [
            ("up", up),
            ("toSun", toward),
            ("zenith", sky.zenith),
            ("horizon", sky.horizon),
            ("glow", sky.glow),
        ] {
            let location = shader.get_shader_location(name);
            shader.set_shader_value(location, value);
        }
        let location = shader.get_shader_location("glowStrength");
        shader.set_shader_value(location, sky.glow_strength);
    }

    /// Sets the halo of a planet of `radius` at `centre`, `thickness` radii deep, seen from
    /// `eye`, lit from `toward` and drawn at `strength` (0 to 1).
    pub(super) fn set_halo(
        &mut self,
        centre: [f32; 3],
        radius: f32,
        thickness: f32,
        eye: [f32; 3],
        toward: [f32; 3],
        strength: f32,
    ) {
        let shader = &mut self.halo_shader;
        for (name, value) in [("centre", centre), ("eye", eye), ("toSun", toward)] {
            let location = shader.get_shader_location(name);
            shader.set_shader_value(location, value);
        }
        for (name, value) in [
            ("radius", radius),
            ("shell", radius * (1.0 + thickness)),
            ("strength", strength),
        ] {
            let location = shader.get_shader_location(name);
            shader.set_shader_value(location, value);
        }
    }

    /// Draws the dome (inside a 3D mode of a camera at the origin).
    pub(super) fn draw_dome(&self, world: &mut impl RaylibDraw3D) {
        // SAFETY: render thread with an active GL context; the dome is seen from inside.
        unsafe { raylib::ffi::rlDisableBackfaceCulling() };
        world.draw_mesh(
            &self.dome,
            self.sky_material.clone(),
            Matrix::scale(10.0, 10.0, 10.0),
        );
        // SAFETY: as above; restores raylib's default culling.
        unsafe { raylib::ffi::rlEnableBackfaceCulling() };
    }

    /// Draws the halo set by [`Self::set_halo`] around a planet of `radius` at `centre`: blended
    /// over what is drawn, without writing depth.
    pub(super) fn draw_halo(
        &self,
        world: &mut impl RaylibDraw3D,
        centre: [f32; 3],
        radius: f32,
        thickness: f32,
    ) {
        let shell = radius * (1.0 + thickness);
        let transform = Matrix {
            m0: shell,
            m5: shell,
            m10: shell,
            m12: centre[0],
            m13: centre[1],
            m14: centre[2],
            ..Matrix::identity()
        };
        // SAFETY: render thread with an active GL context; the shader picks the far half of the
        // shell itself, and the halo must not hide what is drawn after it.
        unsafe {
            raylib::ffi::rlDrawRenderBatchActive();
            raylib::ffi::rlDisableBackfaceCulling();
            raylib::ffi::rlDisableDepthMask();
        }
        world.draw_mesh(&self.shell, self.halo_material.clone(), transform);
        // SAFETY: as above; restores the defaults.
        unsafe {
            raylib::ffi::rlEnableDepthMask();
            raylib::ffi::rlEnableBackfaceCulling();
        }
    }
}

impl Scene {
    /// Sets the opacity of the star field (1 in space, less under a lit sky).
    pub(super) fn set_star_fade(&mut self, fade: f32) {
        let shader = &mut self._shaders[4];
        let location = shader.get_shader_location("fade");
        shader.set_shader_value(location, fade.clamp(0.0, 1.0));
    }
}

/// Draws the backdrop seen from a camera at the origin looking along `forward` with `up`: the
/// sky dome unless it is plain space, then the stars unless the sky hides them.
pub(super) fn draw_backdrop(
    drawing: &mut RaylibTextureMode<'_, '_, RaylibHandle>,
    (forward, up): ([f32; 3], [f32; 3]),
    centre_y: f32,
    sky: &Sky,
    sky_gpu: &SkyGpu,
    stars: Option<&Mesh>,
    star_material: &WeakMaterial,
) {
    let vector = |v: [f32; 3]| Vector3::new(v[0], v[1], v[2]);
    let camera = Camera3D::perspective(Vector3::zero(), vector(forward), vector(up), 45.0);
    let mut backdrop = drawing.begin_mode3D(camera);
    shift_projection(centre_y);
    let space = crate::sky::SPACE_COLOUR;
    if sky.zenith != space || sky.horizon != space {
        sky_gpu.draw_dome(&mut backdrop);
    }
    if let (Some(stars), true) = (stars, sky.stars > 0.01) {
        backdrop.draw_mesh(stars, star_material.clone(), Matrix::identity());
    }
}

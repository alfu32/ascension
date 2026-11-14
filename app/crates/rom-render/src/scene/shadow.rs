//! The shadow pass: depth seen from the light that represents the sun, packed into RGB of an
//! ordinary 2048² render target, over the part of the surface in view. The lit shaders read it
//! back with 3 × 3 percentage-closer filtering (see [`crate::sun::glsl_lighting`]).
//!
//! The map is drawn again only when the light turns by more than a small angle, the view moves
//! the fitted box, or the casters move; otherwise the last map and its matrix are reused.

use super::*;
use crate::pick::View;
use crate::shadow::{LightFit, SHADOW_MAP_SIZE, flat_footprint, globe_footprint};

/// Texture unit the shadow map is bound to while the world is drawn.
pub(super) const SHADOW_SLOT: i32 = 1;
/// Orbit distance, in planet radii, beyond which the globe uses the terminator only.
const GLOBE_SHADOW_RADII: f32 = 0.8;
/// Margin around the receiver footprint; caster bounds extend the light's depth range.
const CASTER_HEIGHT: f32 = RELIEF + 6.0;
/// Turn of the light, in radians, below which the last map is kept (0.15°).
const LIGHT_STEP: f32 = 0.002_618;
/// Constant depth bias in shadow-map texels, scaled up on slopes away from the light.
const DEPTH_BIAS_TEXELS: f32 = 1.5;
/// Offset along the normal before the lookup, in shadow-map texels.
const NORMAL_BIAS_TEXELS: f32 = 1.5;

/// What the lit shaders need to read the shadow map.
#[derive(Clone, Copy, Debug)]
pub(super) struct ShadowUniforms {
    /// View-projection matrix of the light camera.
    pub(super) light_vp: Matrix,
    /// Depth bias in normalised depth.
    pub(super) depth_bias: f32,
    /// Normal offset in world units.
    pub(super) normal_bias: f32,
}

/// What the current map was drawn for.
#[derive(Clone, Debug, PartialEq)]
struct Drawn {
    fit: LightFit,
    casters: [Vec<[f32; 12]>; 4],
    /// Whether the baked feature objects were drawn (they are static, so a flag suffices).
    features: bool,
}

/// The shadow target, the depth materials and the last map drawn.
pub(super) struct ShadowGpu {
    map: RenderTexture2D,
    depth: WeakMaterial,
    depth_instanced: WeakMaterial,
    _shaders: [Shader; 2],
    last: Option<(Drawn, ShadowUniforms)>,
}

impl ShadowGpu {
    fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Option<Self> {
        let map = match rl.load_render_texture(thread, SHADOW_MAP_SIZE, SHADOW_MAP_SIZE) {
            Ok(map) => map,
            Err(error) => {
                eprintln!("rom-render: shadow map: {error}");
                return None;
            }
        };
        let plain = rl.load_shader_from_memory(thread, Some(DEPTH_VS), Some(DEPTH_FS));
        let instanced =
            rl.load_shader_from_memory(thread, Some(DEPTH_INSTANCED_VS), Some(DEPTH_FS));
        let mut depth = rl.load_material_default(thread);
        depth.set_shader(&plain);
        let mut depth_instanced = rl.load_material_default(thread);
        depth_instanced.set_shader(&instanced);
        Some(Self {
            map,
            depth,
            depth_instanced,
            _shaders: [plain, instanced],
            last: None,
        })
    }

    /// Forgets the last map, so the next frame draws it again (new terrain).
    pub(super) fn invalidate(&mut self) {
        self.last = None;
    }

    /// GL id of the shadow map's colour texture.
    pub(super) fn texture_id(&self) -> u32 {
        self.map.texture.id
    }

    /// The light direction to fit with: the last one while the light has turned less than
    /// [`LIGHT_STEP`], so the fit and the map stay put.
    fn steady(&self, toward: [f32; 3]) -> [f32; 3] {
        match &self.last {
            Some((drawn, _)) if dot3(drawn.fit.toward, toward) > LIGHT_STEP.cos() => {
                drawn.fit.toward
            }
            _ => toward,
        }
    }

    /// The reusable uniforms when the last map was drawn for `wanted`.
    fn reuse(&self, wanted: &Drawn) -> Option<ShadowUniforms> {
        self.last
            .as_ref()
            .filter(|(drawn, _)| drawn == wanted)
            .map(|&(_, uniforms)| uniforms)
    }
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Exact geometry of the instances; tint lives in unused matrix cells and cannot cast a shadow.
fn caster_geometry(transforms: &[Matrix]) -> Vec<[f32; 12]> {
    transforms
        .iter()
        .map(|m| {
            [
                m.m0, m.m1, m.m2, m.m4, m.m5, m.m6, m.m8, m.m9, m.m10, m.m12, m.m13, m.m14,
            ]
        })
        .collect()
}

/// Geometry of this frame's crop instances, which move with the buildings they stand on.
fn crop_casters(features: Option<&super::materials::FeatureGpu>) -> Vec<[f32; 12]> {
    features.map_or_else(Vec::new, |f| {
        f.crops
            .iter()
            .flat_map(|list| caster_geometry(list))
            .collect()
    })
}

/// Bounding sphere of a transformed unit cube; tint cells do not affect the geometry.
fn caster_bounds(m: &Matrix) -> ([f32; 3], f32) {
    let squared = m.m0 * m.m0
        + m.m1 * m.m1
        + m.m2 * m.m2
        + m.m4 * m.m4
        + m.m5 * m.m5
        + m.m6 * m.m6
        + m.m8 * m.m8
        + m.m9 * m.m9
        + m.m10 * m.m10;
    ([m.m12, m.m13, m.m14], 0.5 * squared.sqrt())
}

/// Binds the shadow map to its texture unit for the draws that follow, or unbinds it.
pub(super) fn bind_shadow_map(texture: Option<u32>) {
    // SAFETY: the render thread has an active GL context; this only changes the texture bound to
    // a unit that raylib's default materials leave unused.
    unsafe {
        raylib::ffi::rlActiveTextureSlot(SHADOW_SLOT);
        match texture {
            Some(id) => raylib::ffi::rlEnableTexture(id),
            None => raylib::ffi::rlDisableTexture(),
        }
        raylib::ffi::rlActiveTextureSlot(0);
    }
}

/// The orthographic camera of `fit`.
fn light_camera(fit: &LightFit) -> Camera3D {
    let vector = |v: [f32; 3]| Vector3::new(v[0], v[1], v[2]);
    Camera3D::orthographic(
        vector(fit.eye),
        vector(fit.target),
        vector(fit.up),
        2.0 * fit.half,
    )
}

/// Starts drawing casters inside a 3D mode of the light camera: both faces cast. Returns the
/// light's view-projection matrix.
fn begin_casters() -> Matrix {
    // SAFETY: called on the render thread inside an active 3D mode; reads the current matrices
    // and turns face culling off for the caster draws.
    unsafe {
        raylib::ffi::rlDisableBackfaceCulling();
        raylib::ffi::MatrixMultiply(
            raylib::ffi::rlGetMatrixModelview(),
            raylib::ffi::rlGetMatrixProjection(),
        )
    }
}

fn end_casters() {
    // SAFETY: render thread with an active GL context; restores raylib's default culling.
    unsafe { raylib::ffi::rlEnableBackfaceCulling() }
}

/// The lookup uniforms for a map drawn with `fit` and matrix `light_vp`.
fn uniforms(fit: &LightFit, light_vp: Matrix) -> ShadowUniforms {
    let texel = fit.texel();
    ShadowUniforms {
        light_vp,
        depth_bias: DEPTH_BIAS_TEXELS * texel / (fit.far - fit.near).max(1e-3),
        normal_bias: NORMAL_BIAS_TEXELS * texel,
    }
}

impl Scene {
    fn ensure_shadow(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        if self.shadow.is_none() {
            self.shadow = ShadowGpu::load(rl, thread);
        }
    }

    /// Draws the shadow map of the globe for `orbit` on a canvas of `size` pixels when the view
    /// is close enough and in daylight; returns what the lit shaders need, or `None` without
    /// shadows (whole-globe zoom, all night, no map).
    pub(super) fn globe_shadow_pass(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        orbit: &OrbitCamera,
        size: (f32, f32),
    ) -> Option<ShadowUniforms> {
        if orbit.distance > GLOBE_SHADOW_RADII * orbit.radius {
            return None;
        }
        let toward = sun::unit_or(self.sun.map(|c| -c));
        let view = View::from_orbit(orbit, size, self.projection_centre_y);
        let eye = orbit.position();
        let footprint = globe_footprint(orbit, &view);
        let lit = |p: &[f32; 3]| dot3(sun::unit_or(*p), toward) > -sun::TERMINATOR;
        if !footprint.iter().any(lit) {
            return None;
        }
        self.ensure_shadow(rl, thread);
        let gpu = self.globe.as_ref()?;
        let shadow = self.shadow.as_mut()?;
        let features = self.features.as_ref().filter(|_| self.detail > 0.01);
        let crops = crop_casters(features);
        let fit = LightFit::around(&footprint, shadow.steady(toward), CASTER_HEIGHT)?.with_casters(
            gpu.land
                .iter()
                .map(|(_, b)| (b.centre, b.radius))
                .chain(
                    gpu.markers
                        .iter()
                        .chain(&self.entity_casters)
                        .map(caster_bounds),
                )
                .chain(features.into_iter().flat_map(|f| f.bounds()))
                .chain(self.models.iter().flat_map(|m| m.caster_bounds())),
        );
        let wanted = Drawn {
            fit,
            casters: [
                caster_geometry(&self.entity_casters),
                caster_geometry(&gpu.markers),
                crops,
                self.models
                    .as_ref()
                    .map_or_else(Vec::new, |m| m.caster_geometry()),
            ],
            features: features.is_some(),
        };
        if let Some(uniforms) = shadow.reuse(&wanted) {
            return Some(uniforms);
        }
        set_clip_planes(f64::from(fit.near), f64::from(fit.far));
        let light_vp;
        {
            let mut texture = rl.begin_texture_mode(thread, &mut shadow.map);
            texture.clear_background(Color::WHITE);
            let mut pass = texture.begin_mode3D(light_camera(&fit));
            light_vp = begin_casters();
            for (shaded, bounds) in &gpu.land {
                if fit.may_touch(bounds.centre, bounds.radius) {
                    pass.draw_mesh(&shaded.mesh, shadow.depth.clone(), Matrix::identity());
                }
            }
            for transforms in [&gpu.markers, &self.entity_casters] {
                if !transforms.is_empty() {
                    pass.draw_mesh_instanced(
                        &self.box_mesh,
                        shadow.depth_instanced.clone(),
                        transforms,
                    );
                }
            }
            if let Some(features) = features {
                features.draw_casters(
                    &mut pass,
                    &shadow.depth_instanced,
                    eye,
                    &|centre, radius| fit.may_touch(centre, radius),
                    &mut self.caster_scratch,
                );
            }
            if let Some(models) = &self.models {
                models.draw_casters(
                    &mut pass,
                    &shadow.depth_instanced,
                    eye,
                    &|centre, radius| fit.may_touch(centre, radius),
                    &mut self.caster_scratch,
                );
            }
            end_casters();
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
        let uniforms = uniforms(&fit, light_vp);
        shadow.last = Some((wanted, uniforms));
        Some(uniforms)
    }

    /// Draws the shadow map of the flat map for `camera` on a canvas of `size` pixels; the light
    /// is fixed there.
    pub(super) fn flat_shadow_pass(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        camera: &Camera,
        size: (f32, f32),
    ) -> Option<ShadowUniforms> {
        let toward = sun::unit_or(self.flat_light.map(|c| -c));
        let map_size = self.terrain.as_ref()?.size;
        let view = View::new(camera, size, self.projection_centre_y);
        let eye = camera.position();
        let footprint = flat_footprint(camera, &view, map_size);
        self.ensure_shadow(rl, thread);
        let terrain = self.terrain.as_ref()?;
        let shadow = self.shadow.as_mut()?;
        let features = self.features.as_ref().filter(|_| self.detail > 0.01);
        let crops = crop_casters(features);
        let fit = LightFit::around(&footprint, toward, CASTER_HEIGHT)?.with_casters(
            terrain
                .land_bounds
                .iter()
                .map(|b| (b.centre, b.radius))
                .chain(
                    self.entity_casters
                        .iter()
                        .chain(&self.transforms)
                        .map(caster_bounds),
                )
                .chain(features.into_iter().flat_map(|f| f.bounds()))
                .chain(self.models.iter().flat_map(|m| m.caster_bounds())),
        );
        let wanted = Drawn {
            fit,
            casters: [
                caster_geometry(&self.entity_casters),
                caster_geometry(&self.transforms),
                crops,
                self.models
                    .as_ref()
                    .map_or_else(Vec::new, |m| m.caster_geometry()),
            ],
            features: features.is_some(),
        };
        if let Some(uniforms) = shadow.reuse(&wanted) {
            return Some(uniforms);
        }
        set_clip_planes(f64::from(fit.near), f64::from(fit.far));
        let light_vp;
        {
            let mut texture = rl.begin_texture_mode(thread, &mut shadow.map);
            texture.clear_background(Color::WHITE);
            let mut pass = texture.begin_mode3D(light_camera(&fit));
            light_vp = begin_casters();
            for (shaded, bounds) in terrain.land.iter().zip(&terrain.land_bounds) {
                if fit.may_touch(bounds.centre, bounds.radius) {
                    pass.draw_mesh(&shaded.mesh, shadow.depth.clone(), Matrix::identity());
                }
            }
            for transforms in [&self.entity_casters, &self.transforms] {
                if !transforms.is_empty() {
                    pass.draw_mesh_instanced(
                        &self.box_mesh,
                        shadow.depth_instanced.clone(),
                        transforms,
                    );
                }
            }
            if let Some(features) = features {
                features.draw_casters(
                    &mut pass,
                    &shadow.depth_instanced,
                    eye,
                    &|centre, radius| fit.may_touch(centre, radius),
                    &mut self.caster_scratch,
                );
            }
            if let Some(models) = &self.models {
                models.draw_casters(
                    &mut pass,
                    &shadow.depth_instanced,
                    eye,
                    &|centre, radius| fit.may_touch(centre, radius),
                    &mut self.caster_scratch,
                );
            }
            end_casters();
        }
        set_clip_planes(DEFAULT_NEAR, DEFAULT_FAR);
        let uniforms = uniforms(&fit, light_vp);
        shadow.last = Some((wanted, uniforms));
        Some(uniforms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caster_cache_detects_opposite_moves_and_rotation() {
        let original = [Matrix::identity(); 2];
        let mut moved = original;
        moved[0].m12 += 1.0;
        moved[1].m12 -= 1.0;
        assert_ne!(caster_geometry(&original), caster_geometry(&moved));
        let mut turned = original;
        turned[0].m8 = 0.1;
        assert_ne!(caster_geometry(&original), caster_geometry(&turned));
        let mut tinted = original;
        tinted[0].m3 = 0.7;
        tinted[0].m7 = 0.4;
        tinted[0].m11 = 0.2;
        assert_eq!(caster_geometry(&original), caster_geometry(&tinted));
    }
}

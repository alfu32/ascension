//! The GPU side of composed models: the baked near and far meshes of every model, the model
//! shader, and each frame's instances (owner colour in the spare matrix cells, activity and
//! phase in the last), drawn instanced per model with the near or far mesh by distance. Sites
//! under construction are drawn one by one, cut off above their progress. Moving parts read
//! their joint's transform from the pose texture, keyframes of every rigged model baked at
//! upload; shadows keep the models at rest.

use super::*;
use crate::entities::{EntityView, Footing, heading_of};
use crate::models::{ModelLook, ModelMesh, bake_model};
use crate::rig::{Activity, POSE_SAMPLES, Rig};

const MODEL_VS: &str = include_str!("../../../../assets/shaders/model.vs");
const MODEL_FS: &str = include_str!("../../../../assets/shaders/model.fs");

/// Distance from the eye, in tiles, beyond which models use their far meshes.
const MODEL_NEAR_DISTANCE: f32 = 70.0;
/// Distance from the eye, in tiles, beyond which models cast no shadow.
const MODEL_SHADOW_DISTANCE: f32 = 90.0;
/// `reveal` of a finished model: nothing is cut off.
const REVEAL_ALL: f32 = 1.0e9;
/// Texture unit the pose texture is bound to while models are drawn.
const POSE_SLOT: i32 = 3;
/// Width of the pose texture in texels; rigs are packed in rows of this width.
const POSE_WIDTH: usize = 1024;

/// The model shader.
pub(super) fn load_model_shader(rl: &mut RaylibHandle, thread: &RaylibThread) -> Shader {
    let mut shader =
        rl.load_shader_from_memory(thread, Some(MODEL_VS), Some(&materials::expand(MODEL_FS)));
    materials::set_noise_slot(&mut shader);
    let location = shader.get_shader_location("reveal");
    shader.set_shader_value(location, REVEAL_ALL);
    let location = shader.get_shader_location("poseTexture");
    shader.set_shader_value(location, POSE_SLOT);
    let location = shader.get_shader_location("poseSamples");
    shader.set_shader_value(location, POSE_SAMPLES as i32);
    shader
}

/// Where each rigged model's keyframes sit in the pose texture (`None` for a still model),
/// and the texture's size: blocks `3 × joints` wide and `4 × POSE_SAMPLES` high, in rows.
pub(super) fn pack_poses(rigs: &[&Rig]) -> (Vec<Option<[usize; 2]>>, [usize; 2]) {
    let height = 4 * POSE_SAMPLES;
    let (mut x, mut y) = (0, 0);
    let mut origins = Vec::with_capacity(rigs.len());
    for rig in rigs {
        if !rig.is_animated() {
            origins.push(None);
            continue;
        }
        let width = 3 * rig.joints.len();
        if x + width > POSE_WIDTH {
            x = 0;
            y += height;
        }
        origins.push(Some([x, y]));
        x += width;
    }
    let size = if origins.iter().any(Option::is_some) {
        [POSE_WIDTH, y + height]
    } else {
        [0, 0]
    };
    (origins, size)
}

/// Uploads the keyframes of `rigs` at `origins` into a float texture of `size`.
fn upload_poses(
    thread: &RaylibThread,
    rigs: &[&Rig],
    origins: &[Option<[usize; 2]>],
    size: [usize; 2],
) -> Option<raylib::prelude::Texture2D> {
    use raylib::prelude::RaylibTexture2D;
    if size[0] == 0 {
        return None;
    }
    let mut texels = vec![[0.0_f32; 4]; size[0] * size[1]];
    for (rig, origin) in rigs.iter().zip(origins) {
        let Some([x0, y0]) = *origin else {
            continue;
        };
        let width = 3 * rig.joints.len();
        for (i, texel) in rig.keyframes().into_iter().enumerate() {
            let (row, column) = (i / width, i % width);
            texels[(y0 + row) * size[0] + x0 + column] = texel;
        }
    }
    let image = raylib::ffi::Image {
        data: texels.as_mut_ptr().cast(),
        width: size[0] as i32,
        height: size[1] as i32,
        mipmaps: 1,
        format: raylib::consts::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R32G32B32A32 as i32,
    };
    // SAFETY: render thread with an active GL context; `image` points at `texels`, which holds
    // exactly width × height RGBA float texels and outlives the call, and raylib only reads it.
    // The texture it returns is owned by the wrapper, which unloads it on drop.
    let texture =
        unsafe { raylib::prelude::Texture2D::from_raw(raylib::ffi::LoadTextureFromImage(image)) };
    if texture.id == 0 {
        eprintln!("rom-render: could not upload the model poses");
        return None;
    }
    texture.set_texture_filter(thread, raylib::consts::TextureFilter::TEXTURE_FILTER_POINT);
    Some(texture)
}

/// Binds the pose texture to its unit for the draws that follow, or unbinds it.
fn bind_poses(texture: Option<u32>) {
    // SAFETY: the render thread has an active GL context; this only changes the texture bound to
    // a unit that raylib's default materials leave unused.
    unsafe {
        raylib::ffi::rlActiveTextureSlot(POSE_SLOT);
        match texture {
            Some(id) => raylib::ffi::rlEnableTexture(id),
            None => raylib::ffi::rlDisableTexture(),
        }
        raylib::ffi::rlActiveTextureSlot(0);
    }
}

/// The phase (0..1) of an entity's motion: spread by its id so a group does not move as one.
pub(super) fn phase_of(id: u32) -> f32 {
    let mixed = id.wrapping_mul(0x9e37_79b9) >> 16;
    (mixed & 0xffff) as f32 / 65_536.0
}

/// Uploads one model mesh: colour and program in the colours, second colour and contact
/// shade in the tangents, pattern size and joint in the texture coordinates.
fn upload_model(thread: &RaylibThread, mesh: &ModelMesh) -> Option<Mesh> {
    if mesh.indices.is_empty() {
        return None;
    }
    let positions: Vec<Vector3> = mesh
        .positions
        .iter()
        .map(|p| Vector3::new(p[0], p[1], p[2]))
        .collect();
    let normals: Vec<Vector3> = mesh
        .normals
        .iter()
        .map(|n| Vector3::new(n[0], n[1], n[2]))
        .collect();
    let colors: Vec<Color> = mesh
        .colors
        .iter()
        .map(|c| Color::new(c[0], c[1], c[2], c[3]))
        .collect();
    let tangents: Vec<raylib::prelude::Vector4> = mesh
        .alts
        .iter()
        .map(|a| {
            let [r, g, b, s] = a.map(|v| f32::from(v) / 255.0);
            raylib::prelude::Vector4::new(r, g, b, s)
        })
        .collect();
    let texcoords: Vec<Vector2> = mesh
        .scales
        .iter()
        .enumerate()
        .map(|(i, s)| Vector2::new(*s, f32::from(mesh.joints.get(i).copied().unwrap_or(0))))
        .collect();
    let mut builder = Mesh::gen_mesh(&positions, &texcoords);
    builder.normals(&normals);
    builder.colors(&colors);
    builder.tangents(&tangents);
    builder.indices(&mesh.indices);
    match builder.build(thread) {
        Ok(mesh) => Some(mesh),
        Err(error) => {
            eprintln!("rom-render: could not upload a model: {error:?}");
            None
        }
    }
}

/// One baked model on the GPU.
struct ModelMeshes {
    near: Mesh,
    far: Mesh,
    /// Largest distance of a vertex from the model's origin, in model units.
    reach: f32,
    /// Height of the model's top, in model units.
    top: f32,
    /// Size factor of the model.
    scale: f32,
    /// Texel of its keyframes in the pose texture, if it moves.
    poses: Option<[usize; 2]>,
    /// Cycle lengths of its activities, seconds.
    periods: [f32; 4],
}

/// One placed model this frame.
#[derive(Clone, Copy)]
struct Placed {
    model: usize,
    transform: Matrix,
    /// Construction progress of a site.
    progress: Option<f32>,
}

/// The uploaded models and this frame's instances.
pub(super) struct ModelGpu {
    models: Vec<Option<ModelMeshes>>,
    placed: Vec<Placed>,
    near_lists: Vec<Vec<Matrix>>,
    far_lists: Vec<Vec<Matrix>>,
    /// Shadow casters per model (finished models only), with the far meshes.
    casters: Vec<Vec<Matrix>>,
    /// Draw every model with its near mesh (the review gallery).
    pub(super) near_only: bool,
    /// Keyframes of the moving models.
    poses: Option<raylib::prelude::Texture2D>,
    /// Seconds of motion shown this frame.
    pub(super) time: f32,
    /// Every entity at the same point of its cycle (the review gallery).
    pub(super) lockstep: bool,
}

impl ModelGpu {
    /// Bakes and uploads `models`; `None` when there are none.
    pub(super) fn upload(thread: &RaylibThread, models: &[ModelLook]) -> Option<Self> {
        if models.is_empty() {
            return None;
        }
        let bakes: Vec<_> = models.iter().map(|look| bake_model(&look.spec)).collect();
        let rigs: Vec<&Rig> = bakes.iter().map(|b| &b.rig).collect();
        let (origins, size) = pack_poses(&rigs);
        let poses = upload_poses(thread, &rigs, &origins, size);
        let meshes: Vec<Option<ModelMeshes>> = models
            .iter()
            .zip(&bakes)
            .zip(&origins)
            .map(|((look, baked), origin)| {
                for warning in &baked.warnings {
                    eprintln!("rom-render: model {}: {warning}", look.id);
                }
                let reach = baked
                    .near
                    .positions
                    .iter()
                    .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
                    .fold(0.0_f32, f32::max);
                Some(ModelMeshes {
                    near: upload_model(thread, &baked.near)?,
                    far: upload_model(thread, &baked.far)?,
                    reach,
                    top: baked.bounds()[1][1],
                    scale: look.scale.max(0.01),
                    poses: origin.filter(|_| poses.is_some()),
                    periods: Activity::ALL.map(|a| baked.rig.period(a)),
                })
            })
            .collect();
        let count = meshes.len();
        Some(Self {
            models: meshes,
            placed: Vec::new(),
            near_lists: vec![Vec::new(); count],
            far_lists: vec![Vec::new(); count],
            casters: vec![Vec::new(); count],
            near_only: false,
            poses,
            time: 0.0,
            lockstep: false,
        })
    }

    /// Whether model `index` was uploaded.
    pub(super) fn has(&self, index: u16) -> bool {
        self.models
            .get(usize::from(index))
            .is_some_and(Option::is_some)
    }

    /// Places the entities of `views` that have a model on `ground`.
    pub(super) fn place(&mut self, views: &[EntityView], ground: &dyn Footing) {
        self.placed.clear();
        for list in &mut self.casters {
            list.clear();
        }
        for view in views {
            let Some(model) = view.model else {
                continue;
            };
            let index = usize::from(model.index);
            let Some(Some(meshes)) = self.models.get(index) else {
                continue;
            };
            let frame = ground.frame(view.face, view.x, view.y);
            // Buildings keep the ground frame (their footprint is not turned); units face their
            // heading with the model's front (+z).
            let (x_axis, z_axis) = if view.building {
                (frame.ex, frame.ey)
            } else {
                let h = heading_of(&frame, view.facing_deg);
                let u = frame.up;
                let x = [
                    u[1] * h[2] - u[2] * h[1],
                    u[2] * h[0] - u[0] * h[2],
                    u[0] * h[1] - u[1] * h[0],
                ];
                (x, h)
            };
            let s = meshes.scale;
            let owner = crate::entities::owner_color(view.owner);
            let o = frame.origin;
            let mut transform = Matrix {
                m0: x_axis[0] * s,
                m1: x_axis[1] * s,
                m2: x_axis[2] * s,
                m3: owner[0],
                m4: frame.up[0] * s,
                m5: frame.up[1] * s,
                m6: frame.up[2] * s,
                m7: owner[1],
                m8: z_axis[0] * s,
                m9: z_axis[1] * s,
                m10: z_axis[2] * s,
                m11: owner[2],
                m12: o[0],
                m13: o[1],
                m14: o[2],
                m15: 1.0,
            };
            let progress = view.under_construction.map(|p| p.clamp(0.0, 1.0));
            if progress.is_none() {
                self.casters[index].push(transform);
            }
            // The last cell carries the activity and the phase; the shader puts the 1 back.
            let phase = if self.lockstep {
                0.0
            } else {
                phase_of(view.id)
            };
            transform.m15 = view.activity.index() as f32 + phase.min(0.999);
            self.placed.push(Placed {
                model: index,
                transform,
                progress,
            });
        }
    }

    /// Sorts this frame's finished models into near and far lists for an eye at `eye`.
    pub(super) fn prepare(&mut self, eye: [f32; 3]) {
        for list in self.near_lists.iter_mut().chain(&mut self.far_lists) {
            list.clear();
        }
        for placed in self.placed.iter().filter(|p| p.progress.is_none()) {
            let t = &placed.transform;
            let d2 = (t.m12 - eye[0]).powi(2) + (t.m13 - eye[1]).powi(2) + (t.m14 - eye[2]).powi(2);
            let near = self.near_only || d2 < MODEL_NEAR_DISTANCE * MODEL_NEAR_DISTANCE;
            let lists = if near {
                &mut self.near_lists
            } else {
                &mut self.far_lists
            };
            lists[placed.model].push(placed.transform);
        }
    }

    /// Draws this frame's models with `material` (the model shader's).
    pub(super) fn draw(
        &self,
        world: &mut impl RaylibDraw3D,
        shader: &mut Shader,
        material: &WeakMaterial,
    ) {
        let reveal = shader.get_shader_location("reveal");
        shader.set_shader_value(reveal, REVEAL_ALL);
        let origin = shader.get_shader_location("poseOrigin");
        let periods = shader.get_shader_location("posePeriods");
        let time = shader.get_shader_location("poseTime");
        shader.set_shader_value(time, self.time);
        bind_poses(self.poses.as_ref().map(|t| t.id));
        let set_poses = |shader: &mut Shader, model: &ModelMeshes| {
            let at = model.poses.map_or([-1, 0], |[x, y]| [x as i32, y as i32]);
            shader.set_shader_value(origin, at);
            shader.set_shader_value(periods, model.periods);
        };
        for (index, model) in self.models.iter().enumerate() {
            let Some(model) = model else {
                continue;
            };
            if self.near_lists[index].is_empty() && self.far_lists[index].is_empty() {
                continue;
            }
            set_poses(shader, model);
            if !self.near_lists[index].is_empty() {
                world.draw_mesh_instanced(&model.near, material.clone(), &self.near_lists[index]);
            }
            if !self.far_lists[index].is_empty() {
                world.draw_mesh_instanced(&model.far, material.clone(), &self.far_lists[index]);
            }
        }
        for placed in &self.placed {
            let (Some(progress), Some(Some(model))) =
                (placed.progress, self.models.get(placed.model))
            else {
                continue;
            };
            shader.set_shader_value(reveal, model.top * progress);
            set_poses(shader, model);
            world.draw_mesh_instanced(&model.near, material.clone(), &[placed.transform]);
        }
        shader.set_shader_value(reveal, REVEAL_ALL);
        bind_poses(None);
    }

    /// Bounding spheres of this frame's shadow casters.
    pub(super) fn caster_bounds(&self) -> impl Iterator<Item = ([f32; 3], f32)> + '_ {
        self.casters
            .iter()
            .enumerate()
            .flat_map(move |(index, list)| {
                let (reach, scale) = self.models[index]
                    .as_ref()
                    .map_or((0.0, 1.0), |m| (m.reach, m.scale));
                list.iter()
                    .map(move |t| ([t.m12, t.m13, t.m14], reach * scale))
            })
    }

    /// Geometry of this frame's casters (the owner colour does not cast).
    pub(super) fn caster_geometry(&self) -> Vec<[f32; 12]> {
        self.casters
            .iter()
            .flatten()
            .map(|m| {
                [
                    m.m0, m.m1, m.m2, m.m4, m.m5, m.m6, m.m8, m.m9, m.m10, m.m12, m.m13, m.m14,
                ]
            })
            .collect()
    }

    /// Draws the casters within reach of `eye` that the light box `touches` into the shadow
    /// map with `depth`, using the far meshes.
    pub(super) fn draw_casters(
        &self,
        pass: &mut impl RaylibDraw3D,
        depth: &WeakMaterial,
        eye: [f32; 3],
        touches: &dyn Fn([f32; 3], f32) -> bool,
        scratch: &mut Vec<Matrix>,
    ) {
        for (index, list) in self.casters.iter().enumerate() {
            let Some(model) = &self.models[index] else {
                continue;
            };
            scratch.clear();
            scratch.extend(list.iter().copied().filter(|t| {
                let centre = [t.m12, t.m13, t.m14];
                let d2 = (0..3).map(|k| (centre[k] - eye[k]).powi(2)).sum::<f32>();
                d2 < MODEL_SHADOW_DISTANCE * MODEL_SHADOW_DISTANCE
                    && touches(centre, model.reach * model.scale)
            }));
            if !scratch.is_empty() {
                pass.draw_mesh_instanced(&model.far, depth.clone(), scratch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::{Joint, Motion};

    fn rig(joints: usize) -> Rig {
        let mut rig = Rig::default();
        for _ in 1..joints {
            rig.add(Joint {
                parent: 0,
                pivot: [0.0; 3],
                frame: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                scale: 1.0,
                motion: Motion::Wheel,
            });
        }
        rig
    }

    #[test]
    fn poses_pack_in_rows_and_still_models_take_none() {
        let (still, wide) = (rig(1), rig(30));
        let rigs: Vec<&Rig> = std::iter::once(&still)
            .chain(std::iter::repeat_n(&wide, 13))
            .collect();
        let (origins, size) = pack_poses(&rigs);
        assert_eq!(origins[0], None);
        // Eleven blocks of 90 texels fit a row of 1024; the twelfth starts the next row.
        assert_eq!(origins[1], Some([0, 0]));
        assert_eq!(origins[11], Some([900, 0]));
        assert_eq!(origins[12], Some([0, 4 * POSE_SAMPLES]));
        assert_eq!(size, [POSE_WIDTH, 2 * 4 * POSE_SAMPLES]);
        assert_eq!(pack_poses(&[&still]).1, [0, 0]);
    }

    #[test]
    fn phases_spread_over_the_cycle() {
        let phases: Vec<f32> = (1..200).map(phase_of).collect();
        assert!(phases.iter().all(|p| (0.0..1.0).contains(p)));
        let early = phases.iter().filter(|p| **p < 0.5).count();
        assert!((60..140).contains(&early), "{early}");
        assert!(phases.windows(2).all(|w| w[0] != w[1]));
    }
}

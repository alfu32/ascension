//! The GPU side of composed models: the baked near and far meshes of every model, the model
//! shader, and each frame's instances (owner colour in the spare matrix cells, activity and
//! phase in the last), drawn instanced per model with the near or far mesh by distance. Sites
//! under construction are drawn one by one, cut off above their progress. Moving parts read
//! their joint's transform from the pose texture, keyframes of every rigged model baked at
//! upload; shadows keep the models at rest.

use super::*;
use crate::entities::{EntityView, Footing, entity_frame, heading_of};
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

/// How a model whose longest extent across or deep is `extent` model units, `scale` tiles a
/// model unit in the planet view, is drawn `length` frame units long `distance` frame units
/// from the eye: frame units per model unit, and the distance in tiles at which the planet view
/// would show it as large (which picks its near or far mesh the way the planet view does).
fn fit_model(extent: f32, scale: f32, length: f32, distance: f32) -> (f32, f32) {
    let k = length / extent.max(1e-6);
    (k, distance * scale / k.max(1e-12))
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

/// The scale a placed model's transform draws it at (a ship above a planet larger than its
/// own size).
fn placed_scale(t: &Matrix) -> f32 {
    (t.m0 * t.m0 + t.m1 * t.m1 + t.m2 * t.m2).sqrt()
}

/// How many times its own size a model whose longest extent across or deep is `own` tiles is
/// drawn when its view gives it the extent `size` (`ModelRef::size`); 1 when either is empty.
/// How much a building facing `facing_deg` is stretched along its x (task 147): a model that
/// spans its tiles along x spans the diagonal of their square when turned by 45 degrees, so a
/// diagonal wall's segments meet.
pub(crate) fn diagonal_stretch(facing_deg: f32) -> f32 {
    let octant = (facing_deg.rem_euclid(360.0) / 45.0).round() as i32;
    if octant % 2 == 1 {
        std::f32::consts::SQRT_2
    } else {
        1.0
    }
}

pub(crate) fn drawn_factor(own: f32, size: [f32; 3]) -> f32 {
    let drawn = size[0].max(size[2]);
    if own > 0.0 && drawn > 0.0 && (drawn - own).abs() > own * 1e-4 {
        drawn / own
    } else {
        1.0
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
    /// Its longest extent across or deep (as [`crate::entities::unit_width`] reckons a
    /// model's size), in model units.
    extent: f32,
    /// Size factor of the model.
    scale: f32,
    /// Texel of its keyframes in the pose texture, if it moves.
    poses: Option<[usize; 2]>,
    /// Cycle lengths of its activities, seconds.
    periods: [f32; 4],
    /// Its three most used colours, not counting the owner's (task 80: its debris).
    palette: [[u8; 3]; 3],
    /// Extent of its near mesh as drawn idle at the start of the cycle (a hidden muzzle flash
    /// left out), along the palette picture's camera right and up: what the picture frames.
    icon_extent: [[f32; 2]; 2],
}

/// Share of a model's vertices left out at each end of its palette picture's height, so a long
/// upright part (a spear, a mast) does not shrink the body to a speck; the width keeps every
/// vertex, as the pictures stand side by side in one strip.
const ICON_TRIM: f32 = 0.05;

/// The extent of `mesh`, posed by `rig` idle at the start of its cycle (at rest when it is
/// still), along `axes` (across, up): `[low, high]` per axis, leaving out [`ICON_TRIM`] of the
/// vertices at each end of the second.
fn idle_extent(mesh: &ModelMesh, rig: &Rig, axes: [[f32; 3]; 2]) -> [[f32; 2]; 2] {
    let pose = rig.is_animated().then(|| rig.pose(Activity::Idle, 0.0));
    let mut along: [Vec<f32>; 2] = [Vec::new(), Vec::new()];
    for (i, &p) in mesh.positions.iter().enumerate() {
        let joint = usize::from(mesh.joints.get(i).copied().unwrap_or(0));
        let p = match pose.as_ref().and_then(|pose| pose.get(joint)) {
            Some(transform) if joint > 0 => crate::rig::apply(transform, p),
            _ => p,
        };
        for (values, axis) in along.iter_mut().zip(axes) {
            values.push(p[0] * axis[0] + p[1] * axis[1] + p[2] * axis[2]);
        }
    }
    let mut extent = [[0.0; 2]; 2];
    for (k, (values, range)) in along.iter_mut().zip(&mut extent).enumerate() {
        if values.is_empty() {
            continue;
        }
        values.sort_by(f32::total_cmp);
        let trim = if k == 1 { ICON_TRIM } else { 0.0 };
        let cut = ((values.len() as f32 * trim) as usize).min((values.len() - 1) / 2);
        *range = [values[cut], values[values.len() - 1 - cut]];
    }
    extent
}

/// The three colours covering the most triangles of `mesh`, leaving out owner-coloured faces;
/// fewer distinct colours repeat the first ones.
fn palette_of(mesh: &ModelMesh) -> Option<[[u8; 3]; 3]> {
    let mut counts: std::collections::BTreeMap<[u8; 3], usize> = Default::default();
    for triangle in mesh.indices.chunks(3) {
        let c = mesh.colors[usize::from(triangle[0])];
        if c[3] & rom_kit::models::TEAM_BIT == 0 {
            *counts.entry([c[0], c[1], c[2]]).or_default() += 1;
        }
    }
    let mut ranked: Vec<([u8; 3], usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let first = ranked.first()?.0;
    let pick = |i: usize| ranked.get(i).map_or(first, |c| c.0);
    Some([first, pick(1), pick(2)])
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
    /// The index of the lift engine model drawn under a building in the air (task 170).
    engine: Option<usize>,
    /// The index of the first drop-pod model (task 174), the others following it in the order
    /// of `rom_kit::pods::drop_pods`.
    pods: Option<usize>,
}

impl ModelGpu {
    /// Bakes and uploads `models`; `None` when there are none.
    pub(super) fn upload(thread: &RaylibThread, models: &[ModelLook]) -> Option<Self> {
        if models.is_empty() {
            return None;
        }
        // The lift engines come after every model of the rules (task 170), then the drop pods
        // (task 174).
        let ruled = models.len();
        let models: Vec<ModelLook> = models
            .iter()
            .cloned()
            .chain([rom_kit::engines::lift_engine()])
            .chain(rom_kit::pods::drop_pods())
            .collect();
        let (engine, pods) = (Some(ruled), Some(ruled + 1));
        let models = models.as_slice();
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
                let [lo, hi] = baked.bounds();
                let extent = 2.0
                    * [0, 2]
                        .map(|k| lo[k].abs().max(hi[k].abs()))
                        .iter()
                        .fold(0.0_f32, |a, b| a.max(*b));
                Some(ModelMeshes {
                    near: upload_model(thread, &baked.near)?,
                    far: upload_model(thread, &baked.far)?,
                    reach,
                    top: hi[1],
                    extent,
                    scale: look.scale.max(0.01),
                    poses: origin.filter(|_| poses.is_some()),
                    periods: Activity::ALL.map(|a| baked.rig.period(a)),
                    palette: palette_of(&baked.far).unwrap_or(crate::stains::PLAIN_DEBRIS),
                    icon_extent: idle_extent(&baked.near, &baked.rig, {
                        let [_, right, up] = super::icons::icon_axes();
                        [right, up]
                    }),
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
            engine,
            pods,
        })
    }

    /// The colours of model `index`, if it was uploaded.
    pub(super) fn palette(&self, index: u16) -> Option<[[u8; 3]; 3]> {
        self.models
            .get(usize::from(index))
            .and_then(Option::as_ref)
            .map(|m| m.palette)
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
            // A unit falling from orbit is drawn as its drop pod (task 174).
            if view.pod > 0.0 {
                self.place_falling_pod(view, ground);
                continue;
            }
            let Some(model) = view.model else {
                continue;
            };
            let index = usize::from(model.index);
            let Some(Some(meshes)) = self.models.get(index) else {
                continue;
            };
            let frame = entity_frame(view, ground);
            // Buildings turn with their facing (task 147: a wall runs along its line, its model's
            // x along the facing); units face their heading with the model's front (+z).
            let (x_axis, z_axis) = if view.building {
                let along = diagonal_stretch(view.facing_deg);
                let x = heading_of(&frame, view.facing_deg);
                (
                    x.map(|v| v * along),
                    heading_of(&frame, view.facing_deg + 90.0),
                )
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
            // Drawn at the size the view gives it: a ship above a planet is drawn larger than
            // true (`scale_ship_view`), and its pick box, health bar and hull use that size too.
            let s = meshes.scale * drawn_factor(meshes.extent * meshes.scale, model.size);
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
            if view.engines && view.building {
                self.place_engines(view, &frame, owner, phase);
            }
        }
    }

    /// Hangs the lift engines under a building in the air (task 170).
    fn place_engines(
        &mut self,
        view: &EntityView,
        frame: &crate::entities::GroundFrame,
        owner: [f32; 3],
        phase: f32,
    ) {
        let Some(index) = self
            .engine
            .filter(|i| matches!(self.models.get(*i), Some(Some(_))))
        else {
            return;
        };
        let (spots, size) = crate::entities::lift_engine_spots(view, frame);
        let x = heading_of(frame, view.facing_deg);
        let z = heading_of(frame, view.facing_deg + 90.0);
        let up = frame.up;
        for o in spots {
            let transform = Matrix {
                m0: x[0] * size,
                m1: x[1] * size,
                m2: x[2] * size,
                m3: owner[0],
                m4: up[0] * size,
                m5: up[1] * size,
                m6: up[2] * size,
                m7: owner[1],
                m8: z[0] * size,
                m9: z[1] * size,
                m10: z[2] * size,
                m11: owner[2],
                m12: o[0],
                m13: o[1],
                m14: o[2],
                m15: 1.0,
            };
            self.casters[index].push(transform);
            self.placed.push(Placed {
                model: index,
                transform: Matrix {
                    m15: phase.min(0.999),
                    ..transform
                },
                progress: None,
            });
        }
    }

    /// The drop pod a unit falls from orbit in (task 174): white-hot shield down, turning,
    /// drawn larger high up.
    fn place_falling_pod(&mut self, view: &EntityView, ground: &dyn Footing) {
        let frame = entity_frame(view, ground);
        let pose = crate::pod_pose::falling(view.pod, view.id);
        let team = crate::entities::owner_color(view.owner);
        let phase = phase_of(view.id);
        self.place_pod(
            0,
            &frame,
            frame.origin,
            pose.yaw_deg,
            pose.size,
            team,
            phase,
        );
    }

    /// The drop pods standing where they struck (task 174), from the impacts of `blasts`, the
    /// game at `ticks_per_second`: hot for a moment, their petals folding out, then sinking.
    pub(super) fn place_landed_pods(
        &mut self,
        blasts: &[crate::HudBlast],
        ticks_per_second: u32,
        ground: &dyn Footing,
    ) {
        let tps = ticks_per_second.max(1) as f32;
        for blast in blasts.iter().filter(|b| b.pod) {
            let Some(pose) = crate::pod_pose::landed(blast.age_ticks as f32 / tps) else {
                continue;
            };
            let Some(meshes) = self
                .pods
                .and_then(|p| self.models.get(p + pose.model))
                .and_then(Option::as_ref)
            else {
                continue;
            };
            let depth = (rom_kit::pods::DROP_POD_SINK + pose.sink * meshes.top) * meshes.scale;
            let (x, y) = (blast.x as f32 / 1000.0, blast.y as f32 / 1000.0);
            let frame = ground.frame(blast.face, x, y);
            let origin = std::array::from_fn(|k| frame.origin[k] - frame.up[k] * depth);
            let team = crate::entities::owner_color(blast.owner);
            let phase = (blast.seed >> 8) as f32 / (1u32 << 24) as f32;
            self.place_pod(pose.model, &frame, origin, 0.0, 1.0, team, phase);
        }
    }

    /// Places pod model `which` (of `rom_kit::pods::drop_pods`) at `origin` on `frame`, turned
    /// `yaw_deg`, `size` times its own size, in the owner colour `team`.
    #[allow(clippy::too_many_arguments)]
    fn place_pod(
        &mut self,
        which: usize,
        frame: &crate::entities::GroundFrame,
        origin: [f32; 3],
        yaw_deg: f32,
        size: f32,
        team: [f32; 3],
        phase: f32,
    ) {
        let Some(index) = self.pods.map(|p| p + which) else {
            return;
        };
        let Some(Some(meshes)) = self.models.get(index) else {
            return;
        };
        let s = meshes.scale * size;
        let (h, u) = (heading_of(frame, yaw_deg), frame.up);
        let x = [
            u[1] * h[2] - u[2] * h[1],
            u[2] * h[0] - u[0] * h[2],
            u[0] * h[1] - u[1] * h[0],
        ];
        let transform = Matrix {
            m0: x[0] * s,
            m1: x[1] * s,
            m2: x[2] * s,
            m3: team[0],
            m4: u[0] * s,
            m5: u[1] * s,
            m6: u[2] * s,
            m7: team[1],
            m8: h[0] * s,
            m9: h[1] * s,
            m10: h[2] * s,
            m11: team[2],
            m12: origin[0],
            m13: origin[1],
            m14: origin[2],
            m15: 1.0,
        };
        self.casters[index].push(transform);
        let phase = if self.lockstep { 0.0 } else { phase };
        self.placed.push(Placed {
            model: index,
            transform: Matrix {
                m15: Activity::Idle.index() as f32 + phase.min(0.999),
                ..transform
            },
            progress: None,
        });
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

    /// The instance of model `index` at `origin`, its front (+z) along `forward`, `length` long
    /// (its longest extent across or deep), in the owner colour `team`, at `phase` of its idle
    /// cycle; and whether it takes its near mesh, chosen as the planet view would for a model
    /// that looks this large from `eye`. `None` when the model was not uploaded.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn instance_at(
        &self,
        index: u16,
        origin: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
        length: f32,
        team: [f32; 3],
        phase: f32,
        eye: [f32; 3],
    ) -> Option<(Matrix, bool)> {
        let model = self.models.get(usize::from(index))?.as_ref()?;
        if model.extent <= 0.0 || length <= 0.0 {
            return None;
        }
        let d = (0..3)
            .map(|i| (origin[i] - eye[i]).powi(2))
            .sum::<f32>()
            .sqrt();
        let (k, seen_from) = fit_model(model.extent, model.scale, length, d);
        let x = [
            up[1] * forward[2] - up[2] * forward[1],
            up[2] * forward[0] - up[0] * forward[2],
            up[0] * forward[1] - up[1] * forward[0],
        ];
        let near = self.near_only || seen_from < MODEL_NEAR_DISTANCE;
        let phase = if self.lockstep { 0.0 } else { phase };
        let transform = Matrix {
            m0: x[0] * k,
            m1: x[1] * k,
            m2: x[2] * k,
            m3: team[0],
            m4: up[0] * k,
            m5: up[1] * k,
            m6: up[2] * k,
            m7: team[1],
            m8: forward[0] * k,
            m9: forward[1] * k,
            m10: forward[2] * k,
            m11: team[2],
            m12: origin[0],
            m13: origin[1],
            m14: origin[2],
            m15: Activity::Idle.index() as f32 + phase.min(0.999),
        };
        Some((transform, near))
    }

    /// The palette picture's instance of model `index` (task 91): upright and unturned,
    /// scaled and placed so its idle extent along the camera axes `[right, up]` (those of
    /// [`super::icons::icon_axes`]) fills
    /// `fill` of a unit square centred `offset` along `right`; in the owner colour `team`, idle
    /// at the start of its cycle. `None` when the model was not uploaded or is empty.
    pub(super) fn icon_transform(
        &self,
        index: u16,
        [right, up]: [[f32; 3]; 2],
        offset: f32,
        fill: f32,
        team: [f32; 3],
    ) -> Option<Matrix> {
        let model = self.models.get(usize::from(index))?.as_ref()?;
        let [r, u] = model.icon_extent;
        let span = (r[1] - r[0]).max(u[1] - u[0]);
        if span <= 1e-6 {
            return None;
        }
        let k = fill / span;
        let (cr, cu) = ((r[0] + r[1]) / 2.0, (u[0] + u[1]) / 2.0);
        let origin: [f32; 3] =
            std::array::from_fn(|i| right[i] * (offset - k * cr) - up[i] * k * cu);
        Some(Matrix {
            m0: k,
            m1: 0.0,
            m2: 0.0,
            m3: team[0],
            m4: 0.0,
            m5: k,
            m6: 0.0,
            m7: team[1],
            m8: 0.0,
            m9: 0.0,
            m10: k,
            m11: team[2],
            m12: origin[0],
            m13: origin[1],
            m14: origin[2],
            m15: Activity::Idle.index() as f32,
        })
    }

    /// Draws palette pictures (instances from [`ModelGpu::icon_transform`]) with their near
    /// meshes, every model at the start of its cycle.
    pub(super) fn draw_icons(
        &self,
        world: &mut impl RaylibDraw3D,
        shader: &mut Shader,
        material: &WeakMaterial,
        instances: &mut [(u16, bool, Matrix)],
    ) {
        self.draw_at(world, shader, material, instances, &mut Vec::new(), 0.0);
    }

    /// Draws `instances` (model index, near mesh, transform from [`ModelGpu::instance_at`])
    /// with `material` (the model shader's), batched per model and mesh; `scratch` is reused.
    pub(super) fn draw_instances(
        &self,
        world: &mut impl RaylibDraw3D,
        shader: &mut Shader,
        material: &WeakMaterial,
        instances: &mut [(u16, bool, Matrix)],
        scratch: &mut Vec<Matrix>,
    ) {
        self.draw_at(world, shader, material, instances, scratch, self.time);
    }

    /// [`ModelGpu::draw_instances`] at `seconds` of motion.
    fn draw_at(
        &self,
        world: &mut impl RaylibDraw3D,
        shader: &mut Shader,
        material: &WeakMaterial,
        instances: &mut [(u16, bool, Matrix)],
        scratch: &mut Vec<Matrix>,
        seconds: f32,
    ) {
        if instances.is_empty() {
            return;
        }
        instances.sort_by_key(|&(index, near, _)| (index, near));
        let reveal = shader.get_shader_location("reveal");
        shader.set_shader_value(reveal, REVEAL_ALL);
        let time = shader.get_shader_location("poseTime");
        shader.set_shader_value(time, seconds);
        bind_poses(self.poses.as_ref().map(|t| t.id));
        for group in instances.chunk_by(|a, b| (a.0, a.1) == (b.0, b.1)) {
            let (index, near, _) = group[0];
            let Some(Some(model)) = self.models.get(usize::from(index)) else {
                continue;
            };
            self.set_poses(shader, model);
            scratch.clear();
            scratch.extend(group.iter().map(|g| g.2));
            let mesh = if near { &model.near } else { &model.far };
            world.draw_instances(mesh, material.clone(), scratch);
        }
        bind_poses(None);
    }

    /// Points the model shader at the keyframes and cycle lengths of `model`.
    fn set_poses(&self, shader: &mut Shader, model: &ModelMeshes) {
        let origin = shader.get_shader_location("poseOrigin");
        let periods = shader.get_shader_location("posePeriods");
        let at = model.poses.map_or([-1, 0], |[x, y]| [x as i32, y as i32]);
        shader.set_shader_value(origin, at);
        shader.set_shader_value(periods, model.periods);
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
        let time = shader.get_shader_location("poseTime");
        shader.set_shader_value(time, self.time);
        bind_poses(self.poses.as_ref().map(|t| t.id));
        for (index, model) in self.models.iter().enumerate() {
            let Some(model) = model else {
                continue;
            };
            if self.near_lists[index].is_empty() && self.far_lists[index].is_empty() {
                continue;
            }
            self.set_poses(shader, model);
            if !self.near_lists[index].is_empty() {
                world.draw_instances(&model.near, material.clone(), &self.near_lists[index]);
            }
            if !self.far_lists[index].is_empty() {
                world.draw_instances(&model.far, material.clone(), &self.far_lists[index]);
            }
        }
        for placed in &self.placed {
            let (Some(progress), Some(Some(model))) =
                (placed.progress, self.models.get(placed.model))
            else {
                continue;
            };
            shader.set_shader_value(reveal, model.top * progress);
            self.set_poses(shader, model);
            world.draw_instances(&model.near, material.clone(), &[placed.transform]);
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
                let reach = self.models[index].as_ref().map_or(0.0, |m| m.reach);
                list.iter()
                    .map(move |t| ([t.m12, t.m13, t.m14], reach * placed_scale(t)))
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
                    && touches(centre, model.reach * placed_scale(t))
            }));
            if !scratch.is_empty() {
                pass.draw_instances(&model.far, depth.clone(), scratch);
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
    fn a_model_fitted_to_the_system_plane_looks_as_the_planet_view_would_show_it() {
        // A model 4 model units long at 0.75 tiles a unit: 3 tiles long on the planet.
        let (extent, scale) = (4.0, 0.75);
        // Where the planet frame meets the system frame, 2.5 system units a tile: the model is
        // 7.5 units long, and an eye 100 units away is 40 tiles away.
        let per_tile = 2.5;
        let (k, seen) = fit_model(extent, scale, extent * scale * per_tile, 40.0 * per_tile);
        assert!((k * extent - 7.5).abs() < 1e-4, "{k}");
        assert!((seen - 40.0).abs() < 1e-3, "{seen}");
        assert!(seen < MODEL_NEAR_DISTANCE);
        // On the plane it is drawn far larger than true: seen from 20 000 units, 300 long, it
        // looks as it would from about 200 tiles, so it takes its far mesh.
        let (k, seen) = fit_model(extent, scale, 300.0, 20_000.0);
        assert!((k * extent - 300.0).abs() < 1e-2);
        assert!(
            (seen - 200.0).abs() < 1e-1 && seen > MODEL_NEAR_DISTANCE,
            "{seen}"
        );
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

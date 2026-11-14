//! The GPU side of composed models: the baked near and far meshes of every model, the model
//! shader, and each frame's instances (owner colour in the spare matrix cells), drawn
//! instanced per model with the near or far mesh by distance. Sites under construction are
//! drawn one by one, cut off above their progress.

use super::*;
use crate::entities::{EntityView, Footing, heading_of};
use crate::models::{ModelLook, ModelMesh, bake_model};

const MODEL_VS: &str = include_str!("../../../../assets/shaders/model.vs");
const MODEL_FS: &str = include_str!("../../../../assets/shaders/model.fs");

/// Distance from the eye, in tiles, beyond which models use their far meshes.
const MODEL_NEAR_DISTANCE: f32 = 70.0;
/// Distance from the eye, in tiles, beyond which models cast no shadow.
const MODEL_SHADOW_DISTANCE: f32 = 90.0;
/// `reveal` of a finished model: nothing is cut off.
const REVEAL_ALL: f32 = 1.0e9;

/// The model shader.
pub(super) fn load_model_shader(rl: &mut RaylibHandle, thread: &RaylibThread) -> Shader {
    let mut shader =
        rl.load_shader_from_memory(thread, Some(MODEL_VS), Some(&materials::expand(MODEL_FS)));
    materials::set_noise_slot(&mut shader);
    let location = shader.get_shader_location("reveal");
    shader.set_shader_value(location, REVEAL_ALL);
    shader
}

/// Uploads one model mesh: colour and program in the colours, second colour and contact
/// shade in the tangents, pattern size in the first texture coordinates.
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
    let texcoords: Vec<Vector2> = mesh.scales.iter().map(|s| Vector2::new(*s, 0.0)).collect();
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
}

impl ModelGpu {
    /// Bakes and uploads `models`; `None` when there are none.
    pub(super) fn upload(thread: &RaylibThread, models: &[ModelLook]) -> Option<Self> {
        if models.is_empty() {
            return None;
        }
        let meshes: Vec<Option<ModelMeshes>> = models
            .iter()
            .map(|look| {
                let baked = bake_model(&look.spec);
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
            let transform = Matrix {
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
        for (index, model) in self.models.iter().enumerate() {
            let Some(model) = model else {
                continue;
            };
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
            world.draw_mesh_instanced(&model.near, material.clone(), &[placed.transform]);
        }
        shader.set_shader_value(reveal, REVEAL_ALL);
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

//! The GPU side of the stains (task 80): two meshes made before play with room for every stain
//! in view, the soot drawn over the ground without writing depth and the debris as solid
//! pieces, both with the stain shader. Their buffers are rewritten only when the stains in view
//! change; fading runs in the shader from the game clock.

use std::collections::BTreeMap;

use super::*;
use crate::HudStain;
use crate::stains::{DEBRIS_CAPACITY, SOOT_CAPACITY, StainVertex, debris, soot};

const STAIN_VS: &str = include_str!("../../../../assets/shaders/stain.vs");
const STAIN_FS: &str = include_str!("../../../../assets/shaders/stain.fs");

/// One stain's vertices, built once (again only when its model becomes known).
struct Built {
    soot: Vec<StainVertex>,
    debris: Vec<StainVertex>,
    /// The model whose colours its debris wears.
    model: Option<u16>,
    /// The radius it was built with, milli-tiles.
    radius: i32,
}

impl Built {
    /// Whether `stain` brings what some snapshots, made without the models, lack: the model's
    /// colours or a unit's mark widened to its model. What is once known is kept.
    fn stale(&self, stain: &HudStain) -> bool {
        stain.model.is_some_and(|m| self.model != Some(m)) || stain.radius > self.radius
    }
}

/// A mesh of fixed capacity whose leading vertices are rewritten as the stains change.
struct StainMesh {
    mesh: Mesh,
    capacity: usize,
    /// Vertices drawn.
    used: usize,
}

impl StainMesh {
    fn new(thread: &RaylibThread, capacity: usize) -> Option<Self> {
        let positions = vec![Vector3::zero(); capacity];
        let texcoords = vec![Vector2::zero(); capacity];
        let normals = vec![Vector3::new(0.0, 1.0, 0.0); capacity];
        let colors = vec![Color::new(0, 0, 0, 0); capacity];
        let mut builder = Mesh::gen_mesh(&positions, &texcoords);
        builder.normals(&normals);
        builder.colors(&colors);
        builder.texcoords2(&texcoords);
        match builder.build(thread) {
            Ok(mesh) => {
                let mut mesh = Self {
                    mesh,
                    capacity,
                    used: capacity,
                };
                mesh.set_used(0);
                Some(mesh)
            }
            Err(error) => {
                eprintln!("rom-render: could not make the stain mesh: {error:?}");
                None
            }
        }
    }

    /// Draws only the first `used` vertices.
    fn set_used(&mut self, used: usize) {
        self.used = used.min(self.capacity);
        // SAFETY: render thread. The draw call reads `vertexCount` vertices; it stays within
        // the buffers uploaded with `capacity` vertices, and the CPU-side arrays (also
        // `capacity` long) are only read through it, so a lower count keeps every slice valid.
        unsafe {
            use raylib::core::AsRawMut;
            let raw = self.mesh.as_raw_mut();
            raw.vertexCount = self.used as i32;
            raw.triangleCount = (self.used / 3) as i32;
        }
    }

    /// Uploads `vertices` (cut at the capacity) and draws them from now on.
    fn upload(&mut self, vertices: &[StainVertex]) {
        let n = vertices.len().min(self.capacity) / 3 * 3;
        let v = &vertices[..n];
        fn bytes<const N: usize>(
            v: &[StainVertex],
            f: impl Fn(&StainVertex) -> [f32; N],
        ) -> Vec<u8> {
            let mut out = Vec::with_capacity(v.len() * N * 4);
            for vertex in v {
                for x in f(vertex) {
                    out.extend_from_slice(&x.to_ne_bytes());
                }
            }
            out
        }
        let positions = bytes(v, |v| v.pos);
        let normals = bytes(v, |v| v.normal);
        let stamps = bytes(v, |v| v.stamp);
        let locals = bytes(v, |v| v.local);
        let colours: Vec<u8> = v.iter().flat_map(|v| v.colour).collect();
        if n > 0 {
            // SAFETY: render thread; each array holds one attribute per vertex for the first
            // `n` ≤ capacity vertices of buffers made with `capacity` vertices (0 positions,
            // 1 texcoords, 2 normals, 3 colours, 5 texcoords2).
            unsafe {
                self.mesh.update_buffer::<f32>(0, &positions, 0);
                self.mesh.update_buffer::<f32>(1, &stamps, 0);
                self.mesh.update_buffer::<f32>(2, &normals, 0);
                self.mesh.update_buffer::<u8>(3, &colours, 0);
                self.mesh.update_buffer::<f32>(5, &locals, 0);
            }
        }
        self.set_used(n);
    }
}

/// The stain meshes and the stains they hold.
pub(super) struct StainGpu {
    pub(super) shader: Shader,
    material: WeakMaterial,
    soot: StainMesh,
    debris: StainMesh,
    /// Built vertices by stain id, for the stains of `planet`.
    built: BTreeMap<u32, Built>,
    /// The stains in the meshes now (id and tick), and the planet they are on.
    shown: Vec<(u32, u64)>,
    planet: usize,
    /// Game seconds now and a stain's life, for the fading.
    now: f32,
    life: f32,
}

impl StainGpu {
    /// Loads the shader and makes the two meshes; `None` if a mesh cannot be made.
    pub(super) fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Option<Self> {
        let mut shader = rl.load_shader_from_memory(
            thread,
            Some(&materials::expand(STAIN_VS)),
            Some(&materials::expand(STAIN_FS)),
        );
        materials::set_noise_slot(&mut shader);
        let mut material = rl.load_material_default(thread);
        material.set_shader(&shader);
        Some(Self {
            shader,
            material,
            soot: StainMesh::new(thread, SOOT_CAPACITY)?,
            debris: StainMesh::new(thread, DEBRIS_CAPACITY)?,
            built: BTreeMap::new(),
            shown: Vec::new(),
            planet: 0,
            now: 0.0,
            life: 1.0,
        })
    }

    /// Takes the stains of `hud` on the planet `globe` (whose land is `kinds`; `planet`
    /// identifies it); `palette` gives the colours of a model. The meshes are rewritten only
    /// when the stains in view changed.
    pub(super) fn prepare(
        &mut self,
        globe: &Globe,
        kinds: &[TileKind],
        planet: usize,
        hud: &HudSnapshot,
        palette: &dyn Fn(u16) -> Option<[[u8; 3]; 3]>,
    ) {
        let tps = hud.ticks_per_second.max(1) as f32;
        self.now = hud.game_ticks as f32 / tps;
        self.life = hud.stain_life_ticks as f32 / tps;
        if planet != self.planet {
            self.built.clear();
            self.shown.clear();
            self.planet = planet;
        }
        let stale = |s: &HudStain| self.built.get(&s.id).is_some_and(|b| b.stale(s));
        if hud
            .stains
            .iter()
            .map(|s| (s.id, s.tick))
            .eq(self.shown.iter().copied())
            && !hud.stains.iter().any(stale)
        {
            return;
        }
        // Newest first, so a full mesh keeps the freshest stains.
        let mut order: Vec<&HudStain> = hud.stains.iter().collect();
        order.sort_by(|a, b| b.tick.cmp(&a.tick).then(a.id.cmp(&b.id)));
        self.built
            .retain(|id, _| hud.stains.iter().any(|s| s.id == *id));
        let (mut soot_out, mut debris_out) = (Vec::new(), Vec::new());
        for stain in order {
            // Gone already: the simulation drops it at the end of its life.
            if crate::stains::fade((self.now - stain.tick as f32 / tps) / self.life.max(0.001))
                <= 0.0
            {
                continue;
            }
            if self.built.get(&stain.id).is_some_and(|b| b.stale(stain)) {
                self.built.remove(&stain.id);
            }
            let built = self.built.entry(stain.id).or_insert_with(|| {
                let born = stain.tick as f32 / tps;
                let mut built = Built {
                    soot: Vec::new(),
                    debris: Vec::new(),
                    model: stain.model,
                    radius: stain.radius,
                };
                soot(stain, born, globe, kinds, &mut built.soot);
                if stain.building {
                    let colours = stain
                        .model
                        .and_then(palette)
                        .unwrap_or(crate::stains::PLAIN_DEBRIS);
                    let team = crate::entities::owner_color(stain.owner).map(|c| (c * 255.0) as u8);
                    debris(stain, born, globe, kinds, &colours, team, &mut built.debris);
                }
                built
            });
            if soot_out.len() + built.soot.len() <= SOOT_CAPACITY {
                soot_out.extend_from_slice(&built.soot);
            }
            if debris_out.len() + built.debris.len() <= DEBRIS_CAPACITY {
                debris_out.extend_from_slice(&built.debris);
            }
        }
        self.soot.upload(&soot_out);
        self.debris.upload(&debris_out);
        self.shown = hud.stains.iter().map(|s| (s.id, s.tick)).collect();
    }

    /// Forgets the stains when no planet is shown.
    pub(super) fn clear(&mut self) {
        if !self.shown.is_empty() || self.soot.used > 0 || self.debris.used > 0 {
            self.shown.clear();
            self.built.clear();
            self.soot.set_used(0);
            self.debris.set_used(0);
        }
    }

    /// Draws the soot over the ground and the debris on it; call right after the land.
    pub(super) fn draw(&mut self, world: &mut impl RaylibDraw3D) {
        if self.soot.used == 0 && self.debris.used == 0 {
            return;
        }
        let at = |shader: &Shader, name: &str| shader.get_shader_location(name);
        let location = at(&self.shader, "now");
        self.shader.set_shader_value(location, self.now);
        let location = at(&self.shader, "life");
        self.shader.set_shader_value(location, self.life.max(0.001));
        // SAFETY: render thread with an active GL context; only fixed-function state changes,
        // restored below.
        unsafe { raylib::ffi::rlDisableBackfaceCulling() };
        if self.soot.used > 0 {
            // SAFETY: as above.
            unsafe { raylib::ffi::rlDisableDepthMask() };
            world.draw_mesh(&self.soot.mesh, self.material.clone(), Matrix::identity());
            // SAFETY: as above.
            unsafe { raylib::ffi::rlEnableDepthMask() };
        }
        if self.debris.used > 0 {
            world.draw_mesh(&self.debris.mesh, self.material.clone(), Matrix::identity());
        }
        // SAFETY: as above.
        unsafe { raylib::ffi::rlEnableBackfaceCulling() };
    }
}

//! The GPU side of the grass blades (task 83): the grass shader, one dynamic buffer of domain
//! entries uploaded whenever [`GrassState`] rebuilds its field, and four instanced draws of one
//! blade each (Meshcraft's blade path, its eighteen vertices drawn from eight shared corners;
//! see [`crate::grass`]). Blades receive the shadow map but are not drawn into it.

use super::*;
use crate::grass::{self, DOMAIN_ATTRIBUTES, DOMAIN_FLOATS, Domain, GrassState, SLICES};
use crate::hud_model::HudStain;

const GRASS_VS: &str = include_str!("../../../../assets/shaders/grass.vs");
const GRASS_FS: &str = include_str!("../../../../assets/shaders/grass.fs");

/// Corners of one blade, two per row, row by row (`grass.vs` derives them from the index).
/// Meshcraft's blade is three segments of two triangles over them.
const BLADE_INDICES: [u16; 18] = [
    0, 1, 3, 0, 3, 2, // first segment
    2, 3, 5, 2, 5, 4, // second
    4, 5, 7, 4, 7, 6, // tip
];
/// Share of the blade's own flat normal in its lighting; the rest is the ground normal.
const BLADE_LIGHT: f32 = 0.25;
/// Brightness gain of a full blade, so that a meadow's mean tone matches the plain ground's
/// under Meshcraft's darker blade base and seeded variation.
const TONE_GAIN: f32 = 1.4;
/// Domain entries the buffer first holds; it doubles when a view needs more.
const FIRST_CAPACITY: usize = 4096;

/// Uniform locations used every frame.
struct Locations {
    reach: i32,
    view_projection: i32,
    per: i32,
    density: i32,
    pixel_angle: i32,
}

/// The grass shader, its domain buffer and the field it holds.
pub(super) struct GrassGpu {
    pub(super) shader: Shader,
    locations: Locations,
    vao: u32,
    vbo: u32,
    ebo: u32,
    /// Domain entries the buffer can hold.
    capacity: usize,
    /// The entries of the field, band after band, as uploaded.
    upload: Vec<Domain>,
    state: GrassState,
}

impl Drop for GrassGpu {
    fn drop(&mut self) {
        // SAFETY: the scene is dropped on the render thread with its GL context; the vertex
        // array and buffer were made by `load` and are not used after this.
        unsafe {
            raylib::ffi::rlUnloadVertexArray(self.vao);
            raylib::ffi::rlUnloadVertexBuffer(self.vbo);
            raylib::ffi::rlUnloadVertexBuffer(self.ebo);
        }
    }
}

impl GrassGpu {
    /// Loads the grass shader and makes the domain buffer; `None` when the GPU refuses either.
    pub(super) fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Option<Self> {
        let mut shader = rl.load_shader_from_memory(
            thread,
            Some(&materials::expand_vertex(GRASS_VS)),
            Some(&materials::expand(GRASS_FS)),
        );
        // SAFETY: render thread with the GL context; this only reads raylib's default shader id.
        let fallback = unsafe { raylib::ffi::rlGetShaderIdDefault() };
        if shader.id == fallback {
            eprintln!("rom-render: the grass shader did not compile");
            return None;
        }
        materials::set_noise_slot(&mut shader);
        let at = |shader: &Shader, name: &str| shader.get_shader_location(name);
        for (name, value) in [
            ("u_height", grass::HEIGHT),
            ("u_width", grass::WIDTH),
            ("u_curvature", grass::CURVATURE),
            ("u_wind", grass::WIND),
            ("u_variation", grass::VARIATION),
            ("bladeLight", BLADE_LIGHT),
            ("toneGain", TONE_GAIN),
        ] {
            let location = at(&shader, name);
            shader.set_shader_value(location, value);
        }
        let location = at(&shader, "u_seed");
        shader.set_shader_value(location, grass::SEED as i32);
        let locations = Locations {
            reach: at(&shader, "reach"),
            view_projection: at(&shader, "viewProjection"),
            per: at(&shader, "bladesPerTriangle"),
            density: at(&shader, "density"),
            pixel_angle: at(&shader, "pixelAngle"),
        };
        let (vao, vbo, ebo) = make_buffer(FIRST_CAPACITY)?;
        let mut gpu = Self {
            shader,
            locations,
            vao,
            vbo,
            ebo,
            capacity: FIRST_CAPACITY,
            upload: Vec::new(),
            state: GrassState::default(),
        };
        gpu.set_reach_uniform(grass::Detail::default().reach());
        Some(gpu)
    }

    /// Takes the reach of a rendering detail level (task 108); the next frame rebuilds with it.
    pub(super) fn set_reach(&mut self, reach: grass::Reach) {
        if self.state.set_reach(reach) {
            self.set_reach_uniform(reach);
        }
    }

    fn set_reach_uniform(&mut self, reach: grass::Reach) {
        self.shader.set_shader_value(
            self.locations.reach,
            [reach.lod, reach.full, reach.far, reach.coarse],
        );
    }

    /// Takes the land slots of a new planet (or of newly concealed terrain).
    pub(super) fn set_slots(&mut self, slots: &SurfaceSlots) {
        if self.state.set_slots(slots) {
            materials::set_slots(&mut self.shader, slots);
        }
    }

    /// The map shades changed: rebuild with them.
    pub(super) fn shades_changed(&mut self) {
        self.state.shades_changed();
    }

    /// Forgets the planet: no blades until the next one.
    pub(super) fn clear(&mut self) {
        self.state.clear();
    }

    /// Takes the buildings among `views`; their footprints grow no blades.
    pub(super) fn set_buildings(&mut self, globe: &Globe, views: &[EntityView]) {
        self.state.set_buildings(globe, views);
    }

    /// Takes the stains on the ground; they grow no blades.
    pub(super) fn set_stains(&mut self, globe: &Globe, stains: &[HudStain]) {
        self.state.set_stains(globe, stains);
    }

    /// Rebuilds and uploads the domains when the eye moved or turned, or the ground changed;
    /// with `shown` false (the setting off) nothing is drawn.
    pub(super) fn prepare(
        &mut self,
        globe: &Globe,
        shades: &[[f32; 3]],
        cull: &CullView,
        shown: bool,
    ) {
        if !self.state.prepare(globe, shades, cull, shown) {
            return;
        }
        self.upload.clear();
        for band in &self.state.field().bands {
            self.upload.extend_from_slice(band);
        }
        if self.upload.is_empty() {
            return;
        }
        if self.upload.len() > self.capacity {
            let capacity = self.upload.len().next_power_of_two();
            let Some((vao, vbo, ebo)) = make_buffer(capacity) else {
                self.upload.clear();
                return;
            };
            // SAFETY: render thread with the GL context; the old objects are replaced and never
            // used again.
            unsafe {
                raylib::ffi::rlUnloadVertexArray(self.vao);
                raylib::ffi::rlUnloadVertexBuffer(self.vbo);
                raylib::ffi::rlUnloadVertexBuffer(self.ebo);
            }
            (self.vao, self.vbo, self.ebo, self.capacity) = (vao, vbo, ebo, capacity);
        }
        let bytes = std::mem::size_of_val(self.upload.as_slice());
        // SAFETY: render thread; the buffer holds `capacity` ≥ `upload.len()` entries and
        // `upload` is a plain `repr(C)` array of floats of `bytes` bytes, only read.
        unsafe {
            raylib::ffi::rlUpdateVertexBuffer(
                self.vbo,
                self.upload.as_ptr().cast(),
                bytes as i32,
                0,
            );
        }
    }

    /// Draws the blades inside the current 3D mode; `pixel_angle` is the width of a pixel per
    /// tile of distance from the eye.
    pub(super) fn draw(&mut self, pixel_angle: f32) {
        let field = self.state.field();
        // What was uploaded is what is drawn (an upload that failed leaves nothing).
        if field.is_empty() || self.upload.is_empty() {
            return;
        }
        // SAFETY: render thread inside a 3D mode; flushing raylib's batch first keeps its
        // queued geometry ahead of the raw draws below.
        let view_projection = unsafe {
            raylib::ffi::rlDrawRenderBatchActive();
            raylib::ffi::MatrixMultiply(
                raylib::ffi::rlGetMatrixModelview(),
                raylib::ffi::rlGetMatrixProjection(),
            )
        };
        let locations = &self.locations;
        self.shader
            .set_shader_value_matrix(locations.view_projection, view_projection);
        self.shader
            .set_shader_value(locations.density, field.density);
        self.shader
            .set_shader_value(locations.pixel_angle, pixel_angle);
        let stride = (DOMAIN_FLOATS * 4) as i32;
        // SAFETY: render thread with the GL context; the vertex array and buffer belong to this
        // scene and hold `upload.len()` entries, the attribute offsets stay inside them, and the
        // culling, shader and bindings changed here are restored before returning.
        unsafe {
            raylib::ffi::rlDisableBackfaceCulling();
            raylib::ffi::rlEnableShader(self.shader.id);
            raylib::ffi::rlEnableVertexArray(self.vao);
            raylib::ffi::rlEnableVertexBuffer(self.vbo);
        }
        let mut first = 0;
        for (band, per) in field.bands.iter().zip(SLICES) {
            if band.is_empty() {
                continue;
            }
            self.shader.set_shader_value(locations.per, per as i32);
            // SAFETY: as above.
            unsafe {
                raylib::ffi::rlEnableShader(self.shader.id);
                for (location, size, offset) in DOMAIN_ATTRIBUTES {
                    raylib::ffi::rlSetVertexAttribute(
                        location,
                        size,
                        raylib::ffi::RL_FLOAT as i32,
                        false,
                        stride,
                        (first * DOMAIN_FLOATS + offset) as i32 * 4,
                    );
                    raylib::ffi::rlSetVertexAttributeDivisor(location, per as i32);
                }
                raylib::ffi::rlDrawVertexArrayElementsInstanced(
                    0,
                    BLADE_INDICES.len() as i32,
                    std::ptr::null(),
                    (band.len() * per as usize) as i32,
                );
            }
            first += band.len();
        }
        // SAFETY: as above.
        unsafe {
            raylib::ffi::rlDisableVertexArray();
            raylib::ffi::rlDisableVertexBuffer();
            raylib::ffi::rlDisableShader();
            raylib::ffi::rlEnableBackfaceCulling();
        }
    }
}

/// Makes a vertex array over a dynamic buffer of `capacity` domain entries and the blade's
/// index list: the array, the entry buffer and the index buffer.
fn make_buffer(capacity: usize) -> Option<(u32, u32, u32)> {
    let bytes = capacity * DOMAIN_FLOATS * 4;
    // SAFETY: render thread with the GL context; a null pointer asks for an empty buffer of
    // `bytes` bytes; the attributes point inside it, and the array is unbound before returning.
    unsafe {
        let vao = raylib::ffi::rlLoadVertexArray();
        if vao == 0 {
            return None;
        }
        raylib::ffi::rlEnableVertexArray(vao);
        let vbo = raylib::ffi::rlLoadVertexBuffer(std::ptr::null(), bytes as i32, true);
        if vbo == 0 {
            raylib::ffi::rlDisableVertexArray();
            raylib::ffi::rlUnloadVertexArray(vao);
            return None;
        }
        raylib::ffi::rlEnableVertexBuffer(vbo);
        for (location, size, offset) in DOMAIN_ATTRIBUTES {
            raylib::ffi::rlSetVertexAttribute(
                location,
                size,
                raylib::ffi::RL_FLOAT as i32,
                false,
                (DOMAIN_FLOATS * 4) as i32,
                offset as i32 * 4,
            );
            raylib::ffi::rlEnableVertexAttribute(location);
        }
        // The index buffer bound while the array is bound belongs to the array.
        let ebo = raylib::ffi::rlLoadVertexBufferElement(
            BLADE_INDICES.as_ptr().cast(),
            std::mem::size_of_val(&BLADE_INDICES) as i32,
            false,
        );
        raylib::ffi::rlDisableVertexArray();
        raylib::ffi::rlDisableVertexBuffer();
        raylib::ffi::rlDisableVertexBufferElement();
        if ebo == 0 {
            raylib::ffi::rlUnloadVertexArray(vao);
            raylib::ffi::rlUnloadVertexBuffer(vbo);
            return None;
        }
        Some((vao, vbo, ebo))
    }
}

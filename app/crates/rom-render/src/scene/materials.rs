//! The GPU side of the material library: the land, water and object shaders built from the
//! files in `assets/shaders`, their per-planet and per-frame uniforms, and the baked feature
//! objects (uploaded once, then drawn instanced from the chunks in view).

use std::collections::BTreeMap;

use super::*;
use crate::features::{BakedFeatures, FeatureChunk};
use crate::looks::{DETAIL_FRAGMENT_FAR, DETAIL_FRAGMENT_NEAR, MaterialLook, Program, SceneLooks};
use crate::materials::{MAX_SLOTS, SurfaceSlots};
use crate::terrain::TileKind;

const NOISE: &str = include_str!("../../../../assets/shaders/noise.glsl");
const MATERIALS: &str = include_str!("../../../../assets/shaders/materials.glsl");
pub(super) const TERRAIN_VS: &str = include_str!("../../../../assets/shaders/terrain.vs");
const TERRAIN_FS: &str = include_str!("../../../../assets/shaders/terrain.fs");
pub(super) const WATER_VS: &str = include_str!("../../../../assets/shaders/water.vs");
const WATER_FS: &str = include_str!("../../../../assets/shaders/water.fs");
const OBJECT_VS: &str = include_str!("../../../../assets/shaders/object.vs");
const OBJECT_FS: &str = include_str!("../../../../assets/shaders/object.fs");

/// Farthest distance from the eye, in tiles, at which feature objects are drawn.
const OBJECT_DRAW_DISTANCE: f32 = 130.0;
/// Distance from the eye, in tiles, beyond which objects cast no shadow.
const OBJECT_SHADOW_DISTANCE: f32 = 70.0;
/// Distance from the eye, in tiles, beyond which objects use their coarse meshes.
const OBJECT_NEAR_DISTANCE: f32 = 36.0;

/// Texture unit the noise texture is bound to while the world is drawn.
pub(super) const NOISE_SLOT: i32 = 2;
/// Side of the noise texture in texels; the shaders assume it.
const NOISE_SIZE: usize = 256;

/// The noise texture's texels (RGBA): a random red value per texel, and in green the red value
/// of the texel 37 to the right and 17 down (wrapping), as `noise.glsl` expects.
pub(super) fn noise_pixels() -> Vec<u8> {
    let red = |x: usize, y: usize| {
        let mut rng = crate::features::Rng::new(0x006e_6f69_7365, (y * NOISE_SIZE + x) as u64);
        (rng.unit() * 256.0).min(255.0) as u8
    };
    let mut out = Vec::with_capacity(NOISE_SIZE * NOISE_SIZE * 4);
    for y in 0..NOISE_SIZE {
        for x in 0..NOISE_SIZE {
            let shifted = red((x + 37) % NOISE_SIZE, (y + 17) % NOISE_SIZE);
            out.extend_from_slice(&[red(x, y), shifted, 0, 255]);
        }
    }
    out
}

/// Uploads the noise texture, filtered and repeating.
pub(super) fn load_noise(thread: &RaylibThread) -> Option<raylib::prelude::Texture2D> {
    use raylib::prelude::RaylibTexture2D;
    let mut pixels = noise_pixels();
    let image = raylib::ffi::Image {
        data: pixels.as_mut_ptr().cast(),
        width: NOISE_SIZE as i32,
        height: NOISE_SIZE as i32,
        mipmaps: 1,
        format: raylib::consts::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
    };
    // SAFETY: render thread with an active GL context; `image` points at `pixels`, which holds
    // exactly width × height RGBA texels and outlives the call, and raylib only reads it. The
    // texture it returns is owned by the wrapper, which unloads it on drop.
    let texture =
        unsafe { raylib::prelude::Texture2D::from_raw(raylib::ffi::LoadTextureFromImage(image)) };
    if texture.id == 0 {
        eprintln!("rom-render: could not upload the noise texture");
        return None;
    }
    texture.set_texture_filter(
        thread,
        raylib::consts::TextureFilter::TEXTURE_FILTER_BILINEAR,
    );
    texture.set_texture_wrap(thread, raylib::consts::TextureWrap::TEXTURE_WRAP_REPEAT);
    Some(texture)
}

/// Binds the noise texture to its unit for the draws that follow, or unbinds it.
pub(super) fn bind_noise(texture: Option<u32>) {
    // SAFETY: the render thread has an active GL context; this only changes the texture bound to
    // a unit that raylib's default materials leave unused.
    unsafe {
        raylib::ffi::rlActiveTextureSlot(NOISE_SLOT);
        match texture {
            Some(id) => raylib::ffi::rlEnableTexture(id),
            None => raylib::ffi::rlDisableTexture(),
        }
        raylib::ffi::rlActiveTextureSlot(0);
    }
}

/// Points a material shader's noise sampler at [`NOISE_SLOT`].
pub(super) fn set_noise_slot(shader: &mut Shader) {
    let location = shader.get_shader_location("noiseTexture");
    shader.set_shader_value(location, NOISE_SLOT);
}

/// Fills the insertion points of a shader template.
fn expand(template: &str) -> String {
    template
        .replace("/*LIGHTING*/", &sun::glsl_lighting())
        .replace("/*NOISE*/", NOISE)
        .replace("/*MATERIALS*/", MATERIALS)
}

/// The land shader's fragment source.
pub(super) fn terrain_fs() -> String {
    expand(TERRAIN_FS)
}

/// The sea shader's fragment source.
pub(super) fn water_fs() -> String {
    expand(WATER_FS)
}

/// The shader of baked objects and crops.
pub(super) fn load_object_shader(rl: &mut RaylibHandle, thread: &RaylibThread) -> Shader {
    rl.load_shader_from_memory(thread, Some(OBJECT_VS), Some(&expand(OBJECT_FS)))
}

/// Uniform values of the land material slots, in slot order.
struct SlotValues {
    programs: [i32; MAX_SLOTS],
    colours: [[f32; 3]; MAX_SLOTS],
    alts: [[f32; 3]; MAX_SLOTS],
    scales: [f32; MAX_SLOTS],
}

/// Material colours as the shaders take them, with a neutral look for an empty slot.
fn slot_values(slots: &[MaterialLook]) -> SlotValues {
    let mut values = SlotValues {
        programs: [0; MAX_SLOTS],
        colours: [[0.5; 3]; MAX_SLOTS],
        alts: [[0.5; 3]; MAX_SLOTS],
        scales: [1.0; MAX_SLOTS],
    };
    for (i, look) in slots.iter().take(MAX_SLOTS).enumerate() {
        values.programs[i] = look.program.id();
        values.colours[i] = look.colour;
        values.alts[i] = look.colour_alt;
        values.scales[i] = look.scale.max(0.01);
    }
    values
}

/// Gives the land shader its material slots.
pub(super) fn set_slots(shader: &mut Shader, slots: &SurfaceSlots) {
    let values = slot_values(&slots.slots);
    let location = shader.get_shader_location("slotProgram");
    shader.set_shader_value_v(location, &values.programs);
    let location = shader.get_shader_location("slotColour");
    shader.set_shader_value_v(location, &values.colours);
    let location = shader.get_shader_location("slotAlt");
    shader.set_shader_value_v(location, &values.alts);
    let location = shader.get_shader_location("slotScale");
    shader.set_shader_value_v(location, &values.scales);
}

/// Gives the sea shader the shallow and deep water materials of `looks` (map-like blues when
/// none is given).
pub(super) fn set_water(shader: &mut Shader, looks: &SceneLooks) {
    let water = |kind: TileKind| {
        looks
            .tile(kind)
            .and_then(|l| l.surface)
            .filter(|m| m.program == Program::Water)
    };
    let shallow = water(TileKind::ShallowSea);
    let deep = water(TileKind::DeepSea).or(shallow);
    let pair = |m: Option<MaterialLook>, fallback: [f32; 3]| {
        m.map_or((fallback, fallback), |m| (m.colour, m.colour_alt))
    };
    let (shallow_colour, shallow_alt) = pair(shallow, [0.22, 0.62, 0.74]);
    let (deep_colour, deep_alt) = pair(deep, [0.03, 0.14, 0.38]);
    for (name, value) in [
        ("shallowColour", shallow_colour),
        ("shallowAlt", shallow_alt),
        ("deepColour", deep_colour),
        ("deepAlt", deep_alt),
    ] {
        let location = shader.get_shader_location(name);
        shader.set_shader_value(location, value);
    }
}

/// Per-frame uniforms of the material shaders: the eye, the level of detail and the clock.
pub(super) fn set_frame(shader: &mut Shader, eye: [f32; 3], detail: f32, time: f32) {
    let location = shader.get_shader_location("viewPosition");
    shader.set_shader_value(location, eye);
    let location = shader.get_shader_location("detail");
    shader.set_shader_value(location, detail);
    let location = shader.get_shader_location("detailRange");
    shader.set_shader_value(location, [DETAIL_FRAGMENT_NEAR, DETAIL_FRAGMENT_FAR]);
    let location = shader.get_shader_location("time");
    shader.set_shader_value(location, time);
}

/// One placed object on the GPU side.
struct Placed {
    mesh: u16,
    transform: Matrix,
    tint: f32,
    tile: u32,
}

/// The instances of one object chunk.
struct ChunkGpu {
    centre: [f32; 3],
    radius: f32,
    placed: Vec<Placed>,
}

/// Uploaded variant meshes, the placed instances and this frame's draw lists.
pub(super) struct FeatureGpu {
    meshes: Vec<Mesh>,
    /// Coarse versions of `meshes`, for distant instances and the shadow map.
    far_meshes: Vec<Mesh>,
    snow: Vec<Option<(f32, [f32; 3])>>,
    chunks: Vec<ChunkGpu>,
    pub(super) buildings: BTreeMap<u16, Vec<u16>>,
    /// Instances to draw this frame, per mesh: features in view, then crops.
    lists: Vec<Vec<Matrix>>,
    /// Distant instances to draw this frame with the coarse meshes.
    far_lists: Vec<Vec<Matrix>>,
    /// Crop instances of this frame, per mesh (they also cast shadows).
    pub(super) crops: Vec<Vec<Matrix>>,
}

impl FeatureGpu {
    /// Uploads the meshes of `baked`; `None` when nothing was baked.
    pub(super) fn upload(thread: &RaylibThread, baked: &BakedFeatures) -> Option<Self> {
        if baked.meshes.is_empty() {
            return None;
        }
        let meshes: Vec<Mesh> = baked
            .meshes
            .iter()
            .filter_map(|m| upload(thread, m))
            .collect();
        let far_meshes: Vec<Mesh> = baked
            .far_meshes
            .iter()
            .filter_map(|m| upload(thread, m))
            .collect();
        if meshes.len() != baked.meshes.len() || far_meshes.len() != meshes.len() {
            return None;
        }
        let chunks = baked.chunks.iter().map(chunk_gpu).collect();
        Some(Self {
            lists: vec![Vec::new(); meshes.len()],
            far_lists: vec![Vec::new(); meshes.len()],
            crops: vec![Vec::new(); meshes.len()],
            meshes,
            far_meshes,
            snow: baked.snow.clone(),
            chunks,
            buildings: baked.buildings.clone(),
        })
    }

    /// Tints every instance by the shade of its tile (fog of war and borders).
    pub(super) fn apply_shades(&mut self, shades: &[[f32; 3]]) {
        for chunk in &mut self.chunks {
            for placed in &mut chunk.placed {
                let shade = shades
                    .get(placed.tile as usize)
                    .copied()
                    .unwrap_or([1.0; 3]);
                placed.transform.m3 = placed.tint * shade[0];
                placed.transform.m7 = placed.tint * shade[1];
                placed.transform.m11 = placed.tint * shade[2];
            }
        }
    }

    /// Fills the draw lists with the chunks `visible` accepts within reach of `eye`, then the
    /// crops of this frame.
    pub(super) fn prepare(&mut self, eye: [f32; 3], visible: &dyn Fn([f32; 3], f32) -> bool) {
        for list in self.lists.iter_mut().chain(&mut self.far_lists) {
            list.clear();
        }
        for chunk in &self.chunks {
            let d = [0, 1, 2].map(|k| chunk.centre[k] - eye[k]);
            let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if distance - chunk.radius > OBJECT_DRAW_DISTANCE
                || !visible(chunk.centre, chunk.radius)
            {
                continue;
            }
            let near = distance - chunk.radius < OBJECT_NEAR_DISTANCE;
            for placed in &chunk.placed {
                let e = placed.transform;
                let lists = if near
                    && (e.m12 - eye[0]).powi(2)
                        + (e.m13 - eye[1]).powi(2)
                        + (e.m14 - eye[2]).powi(2)
                        < OBJECT_NEAR_DISTANCE * OBJECT_NEAR_DISTANCE
                {
                    &mut self.lists
                } else {
                    &mut self.far_lists
                };
                lists[usize::from(placed.mesh)].push(e);
            }
        }
        for (list, crops) in self.lists.iter_mut().zip(&self.crops) {
            list.extend_from_slice(crops);
        }
    }

    /// Bounds of every chunk, for the shadow fit.
    pub(super) fn bounds(&self) -> impl Iterator<Item = ([f32; 3], f32)> + '_ {
        self.chunks.iter().map(|c| (c.centre, c.radius))
    }

    /// Draws this frame's lists with `material`, setting each mesh's snow on `shader`.
    pub(super) fn draw(
        &self,
        world: &mut impl RaylibDraw3D,
        shader: &mut Shader,
        material: &WeakMaterial,
    ) {
        let snow_on = shader.get_shader_location("hasSnow");
        let line = shader.get_shader_location("snowLine");
        let colour = shader.get_shader_location("snowColour");
        let sets = [
            (&self.meshes, &self.lists),
            (&self.far_meshes, &self.far_lists),
        ];
        for (index, (meshes, list)) in sets.iter().flat_map(|(meshes, lists)| {
            lists
                .iter()
                .enumerate()
                .map(move |(i, l)| (i, (*meshes, l)))
        }) {
            if list.is_empty() {
                continue;
            }
            match self.snow[index] {
                Some((height, tone)) => {
                    shader.set_shader_value(snow_on, 1_i32);
                    shader.set_shader_value(line, height);
                    shader.set_shader_value(colour, tone);
                }
                None => shader.set_shader_value(snow_on, 0_i32),
            }
            let crop = !self.crops[index].is_empty();
            if crop {
                // Blades are single quads, seen from both sides.
                // SAFETY: render thread with an active GL context; restored below.
                unsafe { raylib::ffi::rlDisableBackfaceCulling() }
            }
            world.draw_mesh_instanced(&meshes[index], material.clone(), list);
            if crop {
                // SAFETY: as above; raylib's default is culling on.
                unsafe { raylib::ffi::rlEnableBackfaceCulling() }
            }
        }
    }

    /// Draws the casters within reach of `eye` that the light box `touches` into the shadow
    /// map with `depth`, using the coarse meshes.
    pub(super) fn draw_casters(
        &self,
        pass: &mut impl RaylibDraw3D,
        depth: &WeakMaterial,
        eye: [f32; 3],
        touches: &dyn Fn([f32; 3], f32) -> bool,
        scratch: &mut Vec<Matrix>,
    ) {
        for (index, mesh) in self.far_meshes.iter().enumerate() {
            scratch.clear();
            for chunk in &self.chunks {
                let d = [0, 1, 2].map(|k| chunk.centre[k] - eye[k]);
                let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                if distance - chunk.radius < OBJECT_SHADOW_DISTANCE
                    && touches(chunk.centre, chunk.radius)
                {
                    scratch.extend(
                        chunk
                            .placed
                            .iter()
                            .filter(|p| usize::from(p.mesh) == index)
                            .map(|p| p.transform),
                    );
                }
            }
            scratch.extend_from_slice(&self.crops[index]);
            if !scratch.is_empty() {
                pass.draw_mesh_instanced(mesh, depth.clone(), scratch);
            }
        }
    }
}

fn chunk_gpu(chunk: &FeatureChunk) -> ChunkGpu {
    ChunkGpu {
        centre: chunk.centre,
        radius: chunk.radius,
        placed: chunk
            .instances
            .iter()
            .map(|i| Placed {
                mesh: i.mesh,
                transform: matrix_of(&i.transform),
                tint: i.transform[3],
                tile: i.tile,
            })
            .collect(),
    }
}

/// Uploads a land chunk with its material attributes.
pub(super) fn upload_land(
    thread: &RaylibThread,
    chunk: &ChunkMesh,
    attributes: &crate::materials::MaterialAttributes,
) -> Option<Mesh> {
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
    let texcoords: Vec<Vector2> = attributes
        .texcoords
        .iter()
        .map(|t| Vector2::new(t[0], t[1]))
        .collect();
    let texcoords2: Vec<Vector2> = attributes
        .texcoords2
        .iter()
        .map(|t| Vector2::new(t[0], t[1]))
        .collect();
    let tangents: Vec<raylib::prelude::Vector4> = attributes
        .tangents
        .iter()
        .map(|t| raylib::prelude::Vector4::new(t[0], t[1], t[2], t[3]))
        .collect();
    if texcoords.len() != positions.len() {
        return upload(thread, chunk);
    }
    let mut builder = Mesh::gen_mesh(&positions, &texcoords);
    builder.normals(&normals);
    builder.colors(&colors);
    builder.texcoords2(&texcoords2);
    builder.tangents(&tangents);
    if !chunk.indices.is_empty() {
        builder.indices(&chunk.indices);
    }
    match builder.build(thread) {
        Ok(mesh) => Some(mesh),
        Err(error) => {
            eprintln!("rom-render: could not upload a land mesh: {error:?}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_texture_carries_the_next_slice_in_green() {
        let pixels = noise_pixels();
        assert_eq!(pixels.len(), NOISE_SIZE * NOISE_SIZE * 4);
        let at = |x: usize, y: usize| &pixels[(y * NOISE_SIZE + x) * 4..][..4];
        for (x, y) in [(0, 0), (10, 200), (250, 250), (219, 239)] {
            let shifted = at((x + 37) % NOISE_SIZE, (y + 17) % NOISE_SIZE)[0];
            assert_eq!(at(x, y)[1], shifted);
        }
        // Values spread over the whole range.
        let reds: Vec<u8> = pixels.as_chunks::<4>().0.iter().map(|p| p[0]).collect();
        assert!(reds.iter().any(|&r| r < 16) && reds.iter().any(|&r| r > 240));
        assert_eq!(pixels, noise_pixels());
    }
}

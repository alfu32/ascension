//! Instanced mesh drawing with instance buffers that live as long as the render thread (task 121).
//!
//! raylib's `DrawMeshInstanced` makes a vertex buffer for the transforms on every call, points
//! the mesh's vertex array at it and deletes it once the vertex array is unbound, so the deleted
//! buffer stays attached to that array until its next draw. Dozens of such buffers a frame, sixty
//! frames a second, leave the driver to collect them; [`DrawInstances::draw_instances`] does the same
//! drawing with a few buffers made once and rewritten, so no GL object is made or deleted per
//! frame. The buffers are used round robin within a frame, in [`SETS`] sets taken in turn by
//! successive frames, so a buffer the GPU may still read is not written for two frames.

use std::cell::RefCell;
use std::ffi::{CStr, c_void};

use raylib::ffi;
use raylib::prelude::{Matrix, RaylibDraw3D, WeakMaterial};

/// Sets of buffers, one per frame in turn.
const SETS: usize = 3;
/// Smallest buffer, in instances.
const MIN_INSTANCES: usize = 64;
/// raylib's `MAX_MATERIAL_MAPS` (`rmodels.c`).
const MAX_MATERIAL_MAPS: i32 = 12;

const GL_ARRAY_BUFFER: u32 = 0x8892;
const GL_STREAM_DRAW: u32 = 0x88E0;

/// Declares [`Gl`] with one field per entry point and the function that loads them all.
macro_rules! instancing_table {
    ($($field:ident: $name:literal => fn($($arg:ty),*);)*) => {
        /// The GL entry points raylib's `rlgl` does not wrap: storage without data and
        /// partial writes.
        struct Gl {
            $($field: unsafe extern "system" fn($($arg),*),)*
        }

        impl Gl {
            /// Looks every entry point up; `None` if one is missing.
            fn load() -> Option<Self> {
                Some(Self {
                    $($field: {
                        let address = proc_address($name)?;
                        // SAFETY: the loader returned this GL function, whose C signature is the declared one.
                        unsafe {
                            std::mem::transmute::<*mut c_void, unsafe extern "system" fn($($arg),*)>(
                                address,
                            )
                        }
                    },)*
                })
            }
        }
    };
}

instancing_table! {
    gen_buffers: c"glGenBuffers" => fn(i32, *mut u32);
    delete_buffers: c"glDeleteBuffers" => fn(i32, *const u32);
    bind_buffer: c"glBindBuffer" => fn(u32, u32);
    buffer_data: c"glBufferData" => fn(u32, isize, *const c_void, u32);
    buffer_sub_data: c"glBufferSubData" => fn(u32, isize, isize, *const c_void);
}

/// Looks up one GL function through raylib's loader.
fn proc_address(name: &'static CStr) -> Option<*mut c_void> {
    // SAFETY: raylib's GL context is current on this (the render) thread and `name` is NUL-terminated.
    let address = unsafe { ffi::rlGetProcAddress(name.as_ptr()) };
    (!address.is_null()).then_some(address)
}

/// One instance buffer: its GL name and how many instances its storage holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Slot {
    name: u32,
    capacity: usize,
}

/// Which buffer each draw of a frame takes: pure bookkeeping, no GL.
#[derive(Debug, Default)]
struct Rotation {
    sets: [Vec<Slot>; SETS],
    set: usize,
    next: usize,
    /// Instanced draws of the last finished frame.
    last_draws: usize,
}

impl Rotation {
    /// Starts a frame: the next set of buffers, from its first.
    fn begin_frame(&mut self) {
        self.last_draws = self.next;
        self.set = (self.set + 1) % SETS;
        self.next = 0;
    }

    /// The buffer for the next draw of `instances`, and the capacity it must grow to first, if
    /// any. `generate` names a new buffer.
    fn take(&mut self, instances: usize, generate: impl FnOnce() -> u32) -> (u32, Option<usize>) {
        let set = &mut self.sets[self.set];
        if self.next == set.len() {
            set.push(Slot {
                name: generate(),
                capacity: 0,
            });
        }
        let slot = &mut set[self.next];
        self.next += 1;
        let grow = (instances > slot.capacity).then(|| {
            slot.capacity = instances.next_power_of_two().max(MIN_INSTANCES);
            slot.capacity
        });
        (slot.name, grow)
    }

    /// Every buffer name, emptying the sets.
    fn drain(&mut self) -> Vec<u32> {
        self.next = 0;
        self.sets
            .iter_mut()
            .flat_map(std::mem::take)
            .map(|slot| slot.name)
            .filter(|&name| name != 0)
            .collect()
    }

    /// Buffers held and their bytes, for the counts.
    #[cfg(test)]
    fn held(&self) -> (usize, usize) {
        let slots = self.sets.iter().flatten();
        let bytes = slots.clone().map(|s| s.capacity * INSTANCE_BYTES).sum();
        (slots.count(), bytes)
    }
}

/// Bytes of one instance: a 4×4 float matrix.
const INSTANCE_BYTES: usize = 64;

/// The render thread's instance buffers.
struct Pool {
    gl: Option<Gl>,
    loaded: bool,
    rotation: Rotation,
    /// Column-major transforms of the draw being prepared.
    scratch: Vec<[f32; 16]>,
}

thread_local! {
    static POOL: RefCell<Pool> = RefCell::new(Pool {
        gl: None,
        loaded: false,
        rotation: Rotation::default(),
        scratch: Vec::new(),
    });
}

/// Starts a frame on the render thread: its instanced draws take the next set of buffers.
pub(crate) fn begin_frame() {
    POOL.with_borrow_mut(|pool| pool.rotation.begin_frame());
}

/// Instanced draws of the last finished frame: each one a buffer raylib's own call would have
/// made and deleted.
pub(crate) fn draws_last_frame() -> u32 {
    POOL.with_borrow(|pool| u32::try_from(pool.rotation.last_draws).unwrap_or(u32::MAX))
}

/// Deletes every instance buffer; the render thread calls it before its GL context closes.
pub(crate) fn release() {
    POOL.with_borrow_mut(|pool| {
        let names = pool.rotation.drain();
        if let (Some(gl), false) = (pool.gl.as_ref(), names.is_empty()) {
            // SAFETY: render thread with the context current; the names are this pool's.
            unsafe { (gl.delete_buffers)(names.len() as i32, names.as_ptr()) };
        }
    });
}

/// Draws instances of meshes like raylib's `draw_mesh_instanced`, through the pool's buffers.
pub(crate) trait DrawInstances {
    /// Draws `mesh` with `material` once per transform.
    fn draw_instances(
        &mut self,
        mesh: impl AsRef<ffi::Mesh>,
        material: WeakMaterial,
        transforms: &[Matrix],
    );
}

impl<D: RaylibDraw3D> DrawInstances for D {
    fn draw_instances(
        &mut self,
        mesh: impl AsRef<ffi::Mesh>,
        material: WeakMaterial,
        transforms: &[Matrix],
    ) {
        if transforms.is_empty() {
            return;
        }
        let drawn = POOL.with_borrow_mut(|pool| {
            if !pool.loaded {
                pool.loaded = true;
                pool.gl = Gl::load();
            }
            draw(pool, mesh.as_ref(), &material, transforms)
        });
        if !drawn {
            // No GL entry points or no vertex array: raylib's own way.
            self.draw_mesh_instanced(mesh, material, transforms);
        }
    }
}

/// raylib's `DrawMeshInstanced` (raylib 6, OpenGL 3.3 path, no stereo), with the instance
/// buffer taken from `pool`. `false` when it cannot draw that way.
fn draw(pool: &mut Pool, mesh: &ffi::Mesh, material: &WeakMaterial, transforms: &[Matrix]) -> bool {
    let Some(gl) = pool.gl.as_ref() else {
        return false;
    };
    if mesh.vaoId == 0 {
        return false;
    }
    use ffi::MaterialMapIndex as Map;
    use ffi::ShaderLocationIndex as Loc;
    let material: &ffi::Material = material.as_ref();
    let shader = material.shader;
    // SAFETY: a loaded shader's `locs` holds every shader location index; a loaded material's
    // `maps` holds MAX_MATERIAL_MAPS entries.
    let loc = |index: Loc| unsafe { *shader.locs.add(index as usize) };
    let map = |index: i32| unsafe { *material.maps.add(index as usize) };
    pool.scratch.clear();
    pool.scratch.extend(transforms.iter().map(|m| {
        [
            m.m0, m.m1, m.m2, m.m3, m.m4, m.m5, m.m6, m.m7, m.m8, m.m9, m.m10, m.m11, m.m12, m.m13,
            m.m14, m.m15,
        ]
    }));
    let bytes = (pool.scratch.len() * INSTANCE_BYTES) as isize;
    let (name, grow) = pool.rotation.take(transforms.len(), || {
        let mut name = 0;
        // SAFETY: render thread with the context current; room for one name.
        unsafe { (gl.gen_buffers)(1, &mut name) };
        name
    });
    if name == 0 {
        return false;
    }
    // SAFETY: render thread with raylib's context current, inside a 3D mode (the trait's
    // receiver); the calls mirror raylib's own DrawMeshInstanced and leave the same state
    // behind (shader, vertex array, buffers and texture slots unbound).
    unsafe {
        ffi::rlEnableShader(shader.id);
        let colour = |index: Map| {
            let c = map(index as i32).color;
            [c.r, c.g, c.b, c.a].map(|v| f32::from(v) / 255.0)
        };
        let vec4 = ffi::ShaderUniformDataType::SHADER_UNIFORM_VEC4 as i32;
        if loc(Loc::SHADER_LOC_COLOR_DIFFUSE) != -1 {
            let values = colour(Map::MATERIAL_MAP_ALBEDO);
            ffi::rlSetUniform(
                loc(Loc::SHADER_LOC_COLOR_DIFFUSE),
                values.as_ptr().cast(),
                vec4,
                1,
            );
        }
        if loc(Loc::SHADER_LOC_COLOR_SPECULAR) != -1 {
            let values = colour(Map::MATERIAL_MAP_METALNESS);
            ffi::rlSetUniform(
                loc(Loc::SHADER_LOC_COLOR_SPECULAR),
                values.as_ptr().cast(),
                vec4,
                1,
            );
        }
        let view = ffi::rlGetMatrixModelview();
        let projection = ffi::rlGetMatrixProjection();
        if loc(Loc::SHADER_LOC_MATRIX_VIEW) != -1 {
            ffi::rlSetUniformMatrix(loc(Loc::SHADER_LOC_MATRIX_VIEW), view);
        }
        if loc(Loc::SHADER_LOC_MATRIX_PROJECTION) != -1 {
            ffi::rlSetUniformMatrix(loc(Loc::SHADER_LOC_MATRIX_PROJECTION), projection);
        }

        ffi::rlEnableVertexArray(mesh.vaoId);
        (gl.bind_buffer)(GL_ARRAY_BUFFER, name);
        if let Some(capacity) = grow {
            let size = (capacity * INSTANCE_BYTES) as isize;
            (gl.buffer_data)(GL_ARRAY_BUFFER, size, std::ptr::null(), GL_STREAM_DRAW);
        }
        (gl.buffer_sub_data)(GL_ARRAY_BUFFER, 0, bytes, pool.scratch.as_ptr().cast());
        let instance = loc(Loc::SHADER_LOC_VERTEX_INSTANCETRANSFORM);
        if instance != -1 {
            for column in 0..4 {
                let index = (instance + column) as u32;
                ffi::rlEnableVertexAttribute(index);
                ffi::rlSetVertexAttribute(
                    index,
                    4,
                    ffi::RL_FLOAT as i32,
                    false,
                    INSTANCE_BYTES as i32,
                    column * 16,
                );
                ffi::rlSetVertexAttributeDivisor(index, 1);
            }
        }
        ffi::rlDisableVertexBuffer();
        ffi::rlDisableVertexArray();

        let model_view = ffi::MatrixMultiply(ffi::rlGetMatrixTransform(), view);
        if loc(Loc::SHADER_LOC_MATRIX_NORMAL) != -1 {
            // The model matrix is the identity: its inverse transpose is too.
            ffi::rlSetUniformMatrix(loc(Loc::SHADER_LOC_MATRIX_NORMAL), Matrix::identity());
        }
        let int = ffi::ShaderUniformDataType::SHADER_UNIFORM_INT as i32;
        for slot in 0..MAX_MATERIAL_MAPS {
            let texture = map(slot).texture.id;
            if texture > 0 {
                ffi::rlActiveTextureSlot(slot);
                if is_cube_map(slot) {
                    ffi::rlEnableTextureCubemap(texture);
                } else {
                    ffi::rlEnableTexture(texture);
                }
                let location = *shader
                    .locs
                    .add(Loc::SHADER_LOC_MAP_ALBEDO as usize + slot as usize);
                ffi::rlSetUniform(location, (&raw const slot).cast(), int, 1);
            }
        }
        ffi::rlEnableVertexArray(mesh.vaoId);
        let mvp = ffi::MatrixMultiply(model_view, projection);
        ffi::rlSetUniformMatrix(loc(Loc::SHADER_LOC_MATRIX_MVP), mvp);
        let count = transforms.len() as i32;
        if mesh.indices.is_null() {
            ffi::rlDrawVertexArrayInstanced(0, mesh.vertexCount, count);
        } else {
            ffi::rlDrawVertexArrayElementsInstanced(
                0,
                mesh.triangleCount * 3,
                std::ptr::null(),
                count,
            );
        }
        for slot in 0..MAX_MATERIAL_MAPS {
            if map(slot).texture.id > 0 {
                ffi::rlActiveTextureSlot(slot);
                if is_cube_map(slot) {
                    ffi::rlDisableTextureCubemap();
                } else {
                    ffi::rlDisableTexture();
                }
            }
        }
        ffi::rlDisableVertexArray();
        ffi::rlDisableVertexBuffer();
        ffi::rlDisableVertexBufferElement();
        ffi::rlDisableShader();
    }
    true
}

/// Whether material map `slot` holds a cube map (irradiance, prefilter or cube map).
fn is_cube_map(slot: i32) -> bool {
    use ffi::MaterialMapIndex as Map;
    [
        Map::MATERIAL_MAP_IRRADIANCE,
        Map::MATERIAL_MAP_PREFILTER,
        Map::MATERIAL_MAP_CUBEMAP,
    ]
    .iter()
    .any(|&m| m as i32 == slot)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of draws of these instance counts.
    fn frame(rotation: &mut Rotation, counts: &[usize], names: &mut u32) {
        rotation.begin_frame();
        for &count in counts {
            rotation.take(count, || {
                *names += 1;
                *names
            });
        }
    }

    #[test]
    fn steady_frames_reuse_their_buffers_and_never_make_new_ones() {
        let mut rotation = Rotation::default();
        let mut names = 0;
        let counts = [18_000, 40, 3, 900, 1];
        for _ in 0..SETS {
            frame(&mut rotation, &counts, &mut names);
        }
        let made = names;
        let held = rotation.held();
        assert_eq!(made as usize, counts.len() * SETS);
        for _ in 0..1000 {
            frame(&mut rotation, &counts, &mut names);
        }
        assert_eq!(names, made, "no buffer is made after the first frames");
        assert_eq!(rotation.held(), held, "nor does any grow");
    }

    #[test]
    fn a_buffer_grows_by_powers_of_two_and_only_when_too_small() {
        let mut rotation = Rotation::default();
        rotation.begin_frame();
        let (_, grow) = rotation.take(100, || 7);
        assert_eq!(grow, Some(128));
        for _ in 0..SETS {
            rotation.begin_frame();
        }
        assert_eq!(rotation.take(90, || 8), (7, None));
        for _ in 0..SETS {
            rotation.begin_frame();
        }
        assert_eq!(rotation.take(129, || 9), (7, Some(256)));
        assert_eq!(rotation.drain(), vec![7]);
        assert_eq!(rotation.held(), (0, 0));
    }

    #[test]
    fn successive_frames_take_different_buffers_for_the_same_draw() {
        let mut rotation = Rotation::default();
        let mut names = 0;
        let mut first = Vec::new();
        for _ in 0..SETS {
            rotation.begin_frame();
            first.push(
                rotation
                    .take(10, || {
                        names += 1;
                        names
                    })
                    .0,
            );
        }
        first.sort_unstable();
        first.dedup();
        assert_eq!(first.len(), SETS);
    }
}

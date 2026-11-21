//! A count of the live GL objects of the render thread's context, by kind, with their sizes
//! (task 121). It asks GL itself, so it sees every object whoever made it: raylib's wrappers,
//! raw `rlgl` calls and the read-back buffers alike.

use std::ffi::{CStr, c_void};

/// Live GL objects of the context and the memory they hold, as last counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GpuCensus {
    /// Counts made so far, this one included: 0 while counting is off or where GL lacks the
    /// entry points.
    pub counts: u32,
    /// Textures, render targets' colour and depth textures included.
    pub textures: u32,
    /// Bytes of level 0 of every 2D texture and of every renderbuffer (cube maps and mip chains
    /// not counted).
    pub texture_bytes: u64,
    /// Framebuffers: one per render texture.
    pub framebuffers: u32,
    /// Renderbuffers (depth buffers of render textures).
    pub renderbuffers: u32,
    /// Buffers: vertex, index, instance and pixel pack buffers.
    pub buffers: u32,
    /// Bytes of every buffer's storage.
    pub buffer_bytes: u64,
    /// Vertex arrays: one per uploaded mesh.
    pub vertex_arrays: u32,
    /// Linked shader programs.
    pub programs: u32,
    /// Free video memory as the driver reports it (`GL_NVX_gpu_memory_info`), in KiB; 0 where
    /// the driver does not report it.
    pub free_vram_kib: u64,
    /// Instanced draws in the frame before the count, each drawn from a reused buffer.
    pub instanced_draws: u32,
    /// How long the count took on the render thread, in microseconds.
    pub count_us: u32,
}

impl GpuCensus {
    /// Bytes of textures and buffers together, in whole MiB.
    pub fn mib(&self) -> u64 {
        (self.texture_bytes + self.buffer_bytes) >> 20
    }

    /// One `key=value` line for logs and the bench.
    pub fn line(&self) -> String {
        format!(
            "gpu_textures={} gpu_texture_mb={} gpu_framebuffers={} gpu_renderbuffers={} gpu_buffers={} gpu_buffer_mb={} gpu_vertex_arrays={} gpu_programs={} gpu_free_vram_mb={} gpu_instanced_draws={} gpu_count_us={}",
            self.textures,
            self.texture_bytes >> 20,
            self.framebuffers,
            self.renderbuffers,
            self.buffers,
            self.buffer_bytes >> 20,
            self.vertex_arrays,
            self.programs,
            self.free_vram_kib >> 10,
            self.instanced_draws,
            self.count_us,
        )
    }
}

/// After the highest name in use, names probed without a hit before the count stops.
const MARGIN: u32 = 1024;

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_BINDING_2D: u32 = 0x8069;
const GL_TEXTURE_WIDTH: u32 = 0x1000;
const GL_TEXTURE_HEIGHT: u32 = 0x1001;
const GL_TEXTURE_INTERNAL_FORMAT: u32 = 0x1003;
const GL_COPY_READ_BUFFER: u32 = 0x8F36;
const GL_COPY_READ_BUFFER_BINDING: u32 = 0x8F36;
const GL_BUFFER_SIZE: u32 = 0x8764;
const GL_RENDERBUFFER: u32 = 0x8D41;
const GL_RENDERBUFFER_BINDING: u32 = 0x8CA7;
const GL_RENDERBUFFER_WIDTH: u32 = 0x8D42;
const GL_RENDERBUFFER_HEIGHT: u32 = 0x8D43;
const GL_RENDERBUFFER_INTERNAL_FORMAT: u32 = 0x8D44;
const GL_GPU_MEMORY_INFO_CURRENT_AVAILABLE_VIDMEM_NVX: u32 = 0x9049;

/// Declares [`Gl`] with one field per entry point and the function that loads them all.
macro_rules! census_table {
    ($($field:ident: $name:literal => fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        /// The GL entry points the count needs.
        struct Gl {
            $($field: unsafe extern "system" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Gl {
            /// Looks every entry point up; `None` if one is missing.
            fn load() -> Option<Self> {
                Some(Self {
                    $($field: {
                        let address = proc_address($name)?;
                        // SAFETY: the loader returned this GL function, whose C signature is the declared one.
                        unsafe {
                            std::mem::transmute::<
                                *mut c_void,
                                unsafe extern "system" fn($($arg),*) $(-> $ret)?,
                            >(address)
                        }
                    },)*
                })
            }
        }
    };
}

census_table! {
    is_texture: c"glIsTexture" => fn(u32) -> u8;
    is_buffer: c"glIsBuffer" => fn(u32) -> u8;
    is_framebuffer: c"glIsFramebuffer" => fn(u32) -> u8;
    is_renderbuffer: c"glIsRenderbuffer" => fn(u32) -> u8;
    is_vertex_array: c"glIsVertexArray" => fn(u32) -> u8;
    is_program: c"glIsProgram" => fn(u32) -> u8;
    gen_textures: c"glGenTextures" => fn(i32, *mut u32);
    delete_textures: c"glDeleteTextures" => fn(i32, *const u32);
    gen_buffers: c"glGenBuffers" => fn(i32, *mut u32);
    delete_buffers: c"glDeleteBuffers" => fn(i32, *const u32);
    gen_framebuffers: c"glGenFramebuffers" => fn(i32, *mut u32);
    delete_framebuffers: c"glDeleteFramebuffers" => fn(i32, *const u32);
    gen_renderbuffers: c"glGenRenderbuffers" => fn(i32, *mut u32);
    delete_renderbuffers: c"glDeleteRenderbuffers" => fn(i32, *const u32);
    gen_vertex_arrays: c"glGenVertexArrays" => fn(i32, *mut u32);
    delete_vertex_arrays: c"glDeleteVertexArrays" => fn(i32, *const u32);
    create_program: c"glCreateProgram" => fn() -> u32;
    delete_program: c"glDeleteProgram" => fn(u32);
    bind_texture: c"glBindTexture" => fn(u32, u32);
    bind_buffer: c"glBindBuffer" => fn(u32, u32);
    bind_renderbuffer: c"glBindRenderbuffer" => fn(u32, u32);
    get_renderbuffer_parameter: c"glGetRenderbufferParameteriv" => fn(u32, u32, *mut i32);
    get_integer: c"glGetIntegerv" => fn(u32, *mut i32);
    get_tex_level: c"glGetTexLevelParameteriv" => fn(u32, i32, u32, *mut i32);
    get_buffer_parameter: c"glGetBufferParameteriv" => fn(u32, u32, *mut i32);
    get_error: c"glGetError" => fn() -> u32;
}

/// Looks up one GL function through raylib's loader.
fn proc_address(name: &'static CStr) -> Option<*mut c_void> {
    // SAFETY: raylib's GL context is current on this (the render) thread and `name` is NUL-terminated.
    let address = unsafe { raylib::ffi::rlGetProcAddress(name.as_ptr()) };
    (!address.is_null()).then_some(address)
}

/// Counts the live objects of the render thread's GL context. Lives on the render thread.
pub(crate) struct Census {
    gl: Option<Gl>,
    /// The highest name of each kind found alive so far, where the next count starts looking.
    highest: [u32; 6],
}

impl Census {
    /// Loads the entry points (GL 3.3); without them every count is empty.
    pub(crate) fn new() -> Self {
        Self {
            gl: Gl::load(),
            highest: [0; 6],
        }
    }

    /// Counts now; call between frames (it binds and unbinds textures and a buffer).
    pub(crate) fn count(&mut self) -> GpuCensus {
        let Some(gl) = self.gl.as_ref() else {
            return GpuCensus::default();
        };
        let started = std::time::Instant::now();
        let mut census = GpuCensus {
            counts: 1,
            ..GpuCensus::default()
        };
        // SAFETY: render thread with raylib's context current; every call below takes names or
        // plain out-pointers to locals, and every binding changed is put back.
        unsafe {
            let next = |generate: unsafe extern "system" fn(i32, *mut u32),
                        delete: unsafe extern "system" fn(i32, *const u32)| {
                let mut name = 0_u32;
                generate(1, &mut name);
                delete(1, &name);
                name
            };
            let probes = [
                next(gl.gen_textures, gl.delete_textures),
                next(gl.gen_buffers, gl.delete_buffers),
                next(gl.gen_framebuffers, gl.delete_framebuffers),
                next(gl.gen_renderbuffers, gl.delete_renderbuffers),
                next(gl.gen_vertex_arrays, gl.delete_vertex_arrays),
                {
                    let program = (gl.create_program)();
                    (gl.delete_program)(program);
                    program
                },
            ];
            while (gl.get_error)() != 0 {}
            let mut bound_texture = 0_i32;
            (gl.get_integer)(GL_TEXTURE_BINDING_2D, &mut bound_texture);
            let mut bound_buffer = 0_i32;
            (gl.get_integer)(GL_COPY_READ_BUFFER_BINDING, &mut bound_buffer);
            let mut bound_renderbuffer = 0_i32;
            (gl.get_integer)(GL_RENDERBUFFER_BINDING, &mut bound_renderbuffer);
            let tests: [unsafe extern "system" fn(u32) -> u8; 6] = [
                gl.is_texture,
                gl.is_buffer,
                gl.is_framebuffer,
                gl.is_renderbuffer,
                gl.is_vertex_array,
                gl.is_program,
            ];
            for (kind, alive) in tests.into_iter().enumerate() {
                let start = self.highest[kind].max(probes[kind]);
                let mut highest = 0;
                let mut name = 1_u32;
                while name <= highest.max(start) + MARGIN {
                    if alive(name) != 0 {
                        highest = name;
                        match kind {
                            0 => {
                                census.textures += 1;
                                census.texture_bytes += texture_bytes(gl, name);
                            }
                            1 => {
                                census.buffers += 1;
                                census.buffer_bytes += buffer_bytes(gl, name);
                            }
                            2 => census.framebuffers += 1,
                            3 => {
                                census.renderbuffers += 1;
                                census.texture_bytes += renderbuffer_bytes(gl, name);
                            }
                            4 => census.vertex_arrays += 1,
                            _ => census.programs += 1,
                        }
                    }
                    name += 1;
                }
                self.highest[kind] = highest;
            }
            (gl.bind_texture)(GL_TEXTURE_2D, bound_texture as u32);
            (gl.bind_buffer)(GL_COPY_READ_BUFFER, bound_buffer as u32);
            (gl.bind_renderbuffer)(GL_RENDERBUFFER, bound_renderbuffer as u32);
            let mut free = 0_i32;
            (gl.get_integer)(GL_GPU_MEMORY_INFO_CURRENT_AVAILABLE_VIDMEM_NVX, &mut free);
            census.free_vram_kib = u64::try_from(free).unwrap_or(0);
            census.instanced_draws = crate::instancing::draws_last_frame();
            // Drivers without the NVX query (and textures that are not 2D) leave an error.
            while (gl.get_error)() != 0 {}
        }
        census.count_us = u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX);
        census
    }
}

/// Bytes of level 0 of 2D texture `name`, 0 for another target.
///
/// # Safety
/// Render thread with the context current; the caller restores the 2D texture binding.
unsafe fn texture_bytes(gl: &Gl, name: u32) -> u64 {
    let (mut width, mut height, mut format) = (0_i32, 0_i32, 0_i32);
    // SAFETY: as the function's contract says; out-pointers are locals.
    unsafe {
        (gl.bind_texture)(GL_TEXTURE_2D, name);
        if (gl.get_error)() != 0 {
            return 0;
        }
        (gl.get_tex_level)(GL_TEXTURE_2D, 0, GL_TEXTURE_WIDTH, &mut width);
        (gl.get_tex_level)(GL_TEXTURE_2D, 0, GL_TEXTURE_HEIGHT, &mut height);
        (gl.get_tex_level)(GL_TEXTURE_2D, 0, GL_TEXTURE_INTERNAL_FORMAT, &mut format);
    }
    let pixels = u64::try_from(width).unwrap_or(0) * u64::try_from(height).unwrap_or(0);
    pixels * bytes_per_pixel(format as u32)
}

/// Bytes of renderbuffer `name`.
///
/// # Safety
/// Render thread with the context current; the caller restores the renderbuffer binding.
unsafe fn renderbuffer_bytes(gl: &Gl, name: u32) -> u64 {
    let (mut width, mut height, mut format) = (0_i32, 0_i32, 0_i32);
    // SAFETY: as the function's contract says; out-pointers are locals.
    unsafe {
        (gl.bind_renderbuffer)(GL_RENDERBUFFER, name);
        let query = gl.get_renderbuffer_parameter;
        query(GL_RENDERBUFFER, GL_RENDERBUFFER_WIDTH, &mut width);
        query(GL_RENDERBUFFER, GL_RENDERBUFFER_HEIGHT, &mut height);
        query(
            GL_RENDERBUFFER,
            GL_RENDERBUFFER_INTERNAL_FORMAT,
            &mut format,
        );
    }
    let pixels = u64::try_from(width).unwrap_or(0) * u64::try_from(height).unwrap_or(0);
    pixels * bytes_per_pixel(format as u32)
}

/// Bytes of buffer `name`'s storage.
///
/// # Safety
/// Render thread with the context current; the caller restores the copy-read binding.
unsafe fn buffer_bytes(gl: &Gl, name: u32) -> u64 {
    let mut size = 0_i32;
    // SAFETY: as the function's contract says; the out-pointer is a local.
    unsafe {
        (gl.bind_buffer)(GL_COPY_READ_BUFFER, name);
        (gl.get_buffer_parameter)(GL_COPY_READ_BUFFER, GL_BUFFER_SIZE, &mut size);
    }
    u64::try_from(size).unwrap_or(0)
}

/// Bytes per pixel of the sized internal formats raylib uses; four for the rest.
fn bytes_per_pixel(format: u32) -> u64 {
    match format {
        // R8, the GL 3.3 luminance stand-in.
        0x8229 => 1,
        // RG8, RGB565 and the 16-bit depth format.
        0x822B | 0x8D62 | 0x81A5 => 2,
        // RGB8.
        0x8051 => 3,
        // RGBA16F, RGB16F.
        0x881A | 0x881B => 8,
        // RGBA32F.
        0x8814 => 16,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_names_every_count_and_sizes_in_mib() {
        let census = GpuCensus {
            counts: 1,
            textures: 3,
            texture_bytes: 5 << 20,
            framebuffers: 1,
            renderbuffers: 1,
            buffers: 4,
            buffer_bytes: 3 << 20,
            vertex_arrays: 2,
            programs: 7,
            free_vram_kib: 2048,
            instanced_draws: 5,
            count_us: 900,
        };
        assert_eq!(census.mib(), 8);
        assert_eq!(
            census.line(),
            "gpu_textures=3 gpu_texture_mb=5 gpu_framebuffers=1 gpu_renderbuffers=1 gpu_buffers=4 gpu_buffer_mb=3 gpu_vertex_arrays=2 gpu_programs=7 gpu_free_vram_mb=2 gpu_instanced_draws=5 gpu_count_us=900"
        );
    }

    #[test]
    fn pixel_sizes_follow_the_internal_format() {
        assert_eq!(bytes_per_pixel(0x8229), 1);
        assert_eq!(bytes_per_pixel(0x8058), 4);
        assert_eq!(bytes_per_pixel(0x8814), 16);
    }
}

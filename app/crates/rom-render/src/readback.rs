//! Asynchronous read-back of the render target through pixel pack buffers and fences.
//!
//! After a frame is drawn its pixels are copied on the GPU into one buffer of a small ring and a
//! fence is placed behind the copy. A later frame maps a buffer whose fence has signalled, so the
//! render thread does not wait for the GPU to finish drawing. [`Ring`] is the bookkeeping, free of
//! GL; [`AsyncReadback`] drives it with the GL entry points raylib's loader provides.

use std::ffi::{CStr, c_void};
use std::ptr;
use std::slice;
use std::time::{Duration, Instant};

use raylib::prelude::{RaylibRenderTexture2D, RenderTexture2D};

use crate::mailbox::FramePixels;
use crate::pixels::bgra_rows_flipped;

/// Pack buffers in the ring.
pub(crate) const SLOTS: usize = 3;
/// Time without a published frame after which the oldest read is waited for.
pub(crate) const STALL: Duration = Duration::from_millis(100);
/// Longest blocking wait on one fence, in nanoseconds.
const WAIT_NS: u64 = 1_000_000_000;

/// Which frame each pack buffer holds and which buffer to read next. Pure bookkeeping, no GL.
#[derive(Debug)]
pub(crate) struct Ring {
    size: (u32, u32),
    /// Frame number of the read in flight in each slot.
    slots: [Option<u64>; SLOTS],
    /// Reads issued so far; the next read gets this frame number.
    issued: u64,
    /// The first frame drawn after the newest snapshot request, while it is unanswered.
    snapshot_from: Option<u64>,
    last_publish: Instant,
}

/// What to do with the reads in flight this frame.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Collect {
    /// Slot to map and publish.
    pub map: Option<usize>,
    /// Whether `map`'s fence has not signalled and must be waited for.
    pub wait: bool,
}

impl Ring {
    /// An empty ring with no canvas size yet.
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            size: (0, 0),
            slots: [None; SLOTS],
            issued: 0,
            snapshot_from: None,
            last_publish: now,
        }
    }

    /// Size of the frames the buffers hold.
    pub(crate) fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Switches to `size` and returns the slots whose reads are dropped (none if unchanged).
    pub(crate) fn resize(&mut self, size: (u32, u32), now: Instant) -> Vec<usize> {
        if size == self.size {
            return Vec::new();
        }
        self.size = size;
        self.last_publish = now;
        (0..SLOTS)
            .filter(|&slot| self.slots[slot].take().is_some())
            .collect()
    }

    /// Slot the next read goes into (frame number modulo the ring), or `None` while it is busy.
    pub(crate) fn issue_slot(&self) -> Option<usize> {
        let slot = (self.issued % SLOTS as u64) as usize;
        self.slots[slot].is_none().then_some(slot)
    }

    /// Records a read issued into `slot`; `snapshot` says a snapshot was asked for before the frame was drawn.
    pub(crate) fn issued(&mut self, slot: usize, snapshot: bool) {
        self.slots[slot] = Some(self.issued);
        if snapshot {
            self.snapshot_from = Some(self.issued);
        }
        self.issued += 1;
    }

    /// Plans the collection; `signalled` polls one slot's fence without waiting.
    ///
    /// Fences signal in issue order, so only the oldest read is polled. It is mapped once its
    /// fence has signalled; newer reads wait for later frames, so every read frame is published
    /// in order and the latency settles at one or two frames. With the oldest not signalled and
    /// no frame published for [`STALL`], it is mapped after a wait.
    pub(crate) fn collect(&self, now: Instant, signalled: impl FnOnce(usize) -> bool) -> Collect {
        let Some(oldest) = self.oldest() else {
            return Collect::default();
        };
        if signalled(oldest) {
            Collect {
                map: Some(oldest),
                wait: false,
            }
        } else if now.duration_since(self.last_publish) >= STALL {
            Collect {
                map: Some(oldest),
                wait: true,
            }
        } else {
            Collect::default()
        }
    }

    /// Frees `slot` after its frame was published; true when that frame answers the snapshot request.
    pub(crate) fn published(&mut self, slot: usize, now: Instant) -> bool {
        self.last_publish = now;
        match (self.slots[slot].take(), self.snapshot_from) {
            (Some(frame), Some(from)) if frame >= from => {
                self.snapshot_from = None;
                true
            }
            _ => false,
        }
    }

    /// Frees `slot` without publishing its frame.
    pub(crate) fn release(&mut self, slot: usize) {
        self.slots[slot] = None;
    }

    /// The busy slot holding the oldest read.
    fn oldest(&self) -> Option<usize> {
        (0..SLOTS)
            .filter_map(|slot| self.slots[slot].map(|frame| (frame, slot)))
            .min()
            .map(|(_, slot)| slot)
    }
}

/// A frame read back from the GPU.
#[derive(Debug)]
pub(crate) struct ReadyFrame {
    /// Top-down BGRA pixels, alpha opaque.
    pub pixels: FramePixels,
    /// Whether this frame answers a snapshot request.
    pub snapshot: bool,
}

type GlSync = *const c_void;

const GL_PIXEL_PACK_BUFFER: u32 = 0x88EB;
const GL_STREAM_READ: u32 = 0x88E1;
const GL_MAP_READ_BIT: u32 = 0x0001;
const GL_BGRA: u32 = 0x80E1;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_COLOR_BUFFER_BIT: u32 = 0x4000;
const GL_SYNC_GPU_COMMANDS_COMPLETE: u32 = 0x9117;
const GL_SYNC_FLUSH_COMMANDS_BIT: u32 = 0x0001;
const GL_ALREADY_SIGNALED: u32 = 0x911A;
const GL_TIMEOUT_EXPIRED: u32 = 0x911B;
const GL_CONDITION_SATISFIED: u32 = 0x911C;

/// Looks up one GL function through raylib's loader; `Err` names a missing one.
fn proc_address(name: &'static CStr) -> Result<*mut c_void, String> {
    // SAFETY: raylib's GL context is current on this (the render) thread and `name` is NUL-terminated.
    let address = unsafe { raylib::ffi::rlGetProcAddress(name.as_ptr()) };
    if address.is_null() {
        Err(name.to_string_lossy().into_owned())
    } else {
        Ok(address)
    }
}

/// Declares [`Gl`] with one field per entry point and the function that loads them all.
macro_rules! gl_table {
    ($($field:ident: $name:literal => fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        /// The GL entry points the read-back needs, which raylib's `rlgl` does not wrap.
        struct Gl {
            $($field: unsafe extern "system" fn($($arg),*) $(-> $ret)?,)*
        }

        impl Gl {
            /// Looks every entry point up; `Err` names the first one missing.
            fn entries() -> Result<Self, String> {
                Ok(Self {
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

gl_table! {
    gen_buffers: c"glGenBuffers" => fn(i32, *mut u32);
    delete_buffers: c"glDeleteBuffers" => fn(i32, *const u32);
    bind_buffer: c"glBindBuffer" => fn(u32, u32);
    buffer_data: c"glBufferData" => fn(u32, isize, *const c_void, u32);
    read_pixels: c"glReadPixels" => fn(i32, i32, i32, i32, u32, u32, *mut c_void);
    map_buffer_range: c"glMapBufferRange" => fn(u32, isize, isize, u32) -> *mut c_void;
    unmap_buffer: c"glUnmapBuffer" => fn(u32) -> u8;
    fence_sync: c"glFenceSync" => fn(u32, u32) -> GlSync;
    client_wait_sync: c"glClientWaitSync" => fn(GlSync, u32, u64) -> u32;
    delete_sync: c"glDeleteSync" => fn(GlSync);
    bind_framebuffer: c"glBindFramebuffer" => fn(u32, u32);
    color_mask: c"glColorMask" => fn(u8, u8, u8, u8);
    clear_color: c"glClearColor" => fn(f32, f32, f32, f32);
    clear: c"glClear" => fn(u32);
    flush: c"glFlush" => fn();
}

impl Gl {
    /// Loads every entry point on the render thread; `Err` names what is missing.
    fn load() -> Result<Self, String> {
        // SAFETY: rlGetVersion only reads raylib's record of the context created by `raylib::init`.
        let version = unsafe { raylib::ffi::rlGetVersion() };
        let core = [
            raylib::ffi::rlGlVersion::RL_OPENGL_33 as i32,
            raylib::ffi::rlGlVersion::RL_OPENGL_43 as i32,
        ];
        if !core.contains(&version) {
            return Err(format!(
                "OpenGL 3.3 or later (raylib reports version {version})"
            ));
        }
        Self::entries()
    }
}

/// Reads frames back through a ring of pixel pack buffers. Lives and dies on the render thread.
pub(crate) struct AsyncReadback {
    gl: Gl,
    buffers: [u32; SLOTS],
    fences: [GlSync; SLOTS],
    ring: Ring,
}

impl AsyncReadback {
    /// Loads the GL entry points and creates the buffers; `Err` names what the context lacks.
    pub(crate) fn new(now: Instant) -> Result<Self, String> {
        let gl = Gl::load()?;
        let mut buffers = [0_u32; SLOTS];
        // SAFETY: valid entry point on the render thread; `buffers` has room for SLOTS names.
        unsafe { (gl.gen_buffers)(SLOTS as i32, buffers.as_mut_ptr()) };
        if buffers.contains(&0) {
            // SAFETY: deleting names this call generated (zeros are ignored by GL).
            unsafe { (gl.delete_buffers)(SLOTS as i32, buffers.as_ptr()) };
            return Err("pixel pack buffers".into());
        }
        Ok(Self {
            gl,
            buffers,
            fences: [ptr::null(); SLOTS],
            ring: Ring::new(now),
        })
    }

    /// Starts reading the frame just drawn into `target` and returns an earlier frame that is ready.
    ///
    /// `snapshot` is asked only when this frame's read is issued. `Err` means the buffers cannot
    /// be mapped; the caller then falls back to synchronous read-back.
    pub(crate) fn frame(
        &mut self,
        target: &RenderTexture2D,
        size: (u32, u32),
        now: Instant,
        snapshot: impl FnOnce() -> bool,
    ) -> Result<Option<ReadyFrame>, String> {
        if size != self.ring.size() {
            for slot in self.ring.resize(size, now) {
                self.drop_fence(slot);
            }
            self.allocate(size);
        }
        let mut ready = None;
        if self.ring.issue_slot().is_none() {
            ready = self.collect(now)?;
        }
        if let Some(slot) = self.ring.issue_slot() {
            self.issue(slot, target, size);
            self.ring.issued(slot, snapshot());
        }
        if ready.is_none() {
            ready = self.collect(Instant::now())?;
        }
        Ok(ready)
    }

    /// Gives every buffer storage for one frame of `size`.
    fn allocate(&mut self, size: (u32, u32)) {
        let bytes = frame_bytes(size);
        for &buffer in &self.buffers {
            // SAFETY: binding a buffer this struct generated, sizing it, and unbinding again on the render thread.
            unsafe {
                (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, buffer);
                (self.gl.buffer_data)(GL_PIXEL_PACK_BUFFER, bytes, ptr::null(), GL_STREAM_READ);
                (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
            }
        }
    }

    /// Makes the frame opaque, starts the copy into `slot`'s buffer and fences it.
    fn issue(&mut self, slot: usize, target: &RenderTexture2D, size: (u32, u32)) {
        let gl = &self.gl;
        let (width, height) = (size.0 as i32, size.1 as i32);
        // SAFETY: GL calls on the render thread against raylib's framebuffer and this ring's buffer;
        // the read writes into the bound pack buffer (offset 0), not client memory, and every
        // binding is put back to 0 as raylib leaves it after drawing to a texture.
        unsafe {
            (gl.bind_framebuffer)(GL_FRAMEBUFFER, target.id());
            // Translucent water lowers alpha; the view is opaque, so set alpha on the GPU.
            (gl.color_mask)(0, 0, 0, 1);
            (gl.clear_color)(0.0, 0.0, 0.0, 1.0);
            (gl.clear)(GL_COLOR_BUFFER_BIT);
            (gl.color_mask)(1, 1, 1, 1);
            (gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, self.buffers[slot]);
            (gl.read_pixels)(
                0,
                0,
                width,
                height,
                GL_BGRA,
                GL_UNSIGNED_BYTE,
                ptr::null_mut(),
            );
            (gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
            (gl.bind_framebuffer)(GL_FRAMEBUFFER, 0);
            self.fences[slot] = (gl.fence_sync)(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
            (gl.flush)();
        }
    }

    /// Maps the frame the ring picks, if any, and copies it out top-down.
    fn collect(&mut self, now: Instant) -> Result<Option<ReadyFrame>, String> {
        let fences = self.fences;
        let gl = &self.gl;
        let plan = self.ring.collect(now, |slot| {
            // SAFETY: polling (timeout 0) a fence this struct created and has not deleted.
            let status = unsafe { (gl.client_wait_sync)(fences[slot], 0, 0) };
            status != GL_TIMEOUT_EXPIRED
        });
        let Some(slot) = plan.map else {
            return Ok(None);
        };
        if plan.wait {
            // SAFETY: waiting on a live fence of this struct, flushing so it can signal.
            let status = unsafe {
                (self.gl.client_wait_sync)(self.fences[slot], GL_SYNC_FLUSH_COMMANDS_BIT, WAIT_NS)
            };
            if status != GL_ALREADY_SIGNALED && status != GL_CONDITION_SATISFIED {
                self.drop_fence(slot);
                self.ring.release(slot);
                return Ok(None);
            }
        }
        self.drop_fence(slot);
        let (width, height) = self.ring.size();
        let bytes = frame_bytes((width, height));
        // SAFETY: mapping this ring's buffer, whose storage holds `bytes` bytes, for reading only.
        let mapped = unsafe {
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, self.buffers[slot]);
            (self.gl.map_buffer_range)(GL_PIXEL_PACK_BUFFER, 0, bytes, GL_MAP_READ_BIT)
        };
        if mapped.is_null() {
            // SAFETY: unbinding the pack buffer bound just above.
            unsafe { (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0) };
            self.ring.release(slot);
            return Err("could not map a pixel pack buffer".into());
        }
        // SAFETY: the mapping covers `bytes` readable bytes and stays valid until the unmap below.
        let source = unsafe { slice::from_raw_parts(mapped.cast::<u8>(), bytes as usize) };
        let row = width as usize * 4;
        let pixels = bgra_rows_flipped(source, width as usize, height as usize, row);
        // SAFETY: unmapping and unbinding the buffer mapped above; `source` is not used after this.
        unsafe {
            (self.gl.unmap_buffer)(GL_PIXEL_PACK_BUFFER);
            (self.gl.bind_buffer)(GL_PIXEL_PACK_BUFFER, 0);
        }
        let snapshot = self.ring.published(slot, now);
        let pixels = pixels
            .and_then(|bgra| FramePixels::from_raw(width, height, bgra))
            .ok_or("pixel pack buffer smaller than the frame")?;
        Ok(Some(ReadyFrame { pixels, snapshot }))
    }

    /// Deletes `slot`'s fence, if it has one.
    fn drop_fence(&mut self, slot: usize) {
        let fence = std::mem::replace(&mut self.fences[slot], ptr::null());
        if !fence.is_null() {
            // SAFETY: deleting a fence this struct created, exactly once.
            unsafe { (self.gl.delete_sync)(fence) };
        }
    }
}

impl Drop for AsyncReadback {
    fn drop(&mut self) {
        for slot in 0..SLOTS {
            self.drop_fence(slot);
        }
        // SAFETY: the render thread drops this before closing the GL context; the names are this struct's.
        unsafe { (self.gl.delete_buffers)(SLOTS as i32, self.buffers.as_ptr()) };
    }
}

/// Bytes in one BGRA frame of `size`.
fn frame_bytes(size: (u32, u32)) -> isize {
    (size.0 as isize) * (size.1 as isize) * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring_with(frames: usize, now: Instant) -> Ring {
        let mut ring = Ring::new(now);
        ring.resize((4, 2), now);
        for _ in 0..frames {
            let slot = ring.issue_slot().expect("free slot");
            ring.issued(slot, false);
        }
        ring
    }

    #[test]
    fn ring_maps_oldest_ready_frame() {
        let now = Instant::now();
        let mut ring = ring_with(3, now);
        assert_eq!(ring.issue_slot(), None, "three reads fill the ring");
        // Frames 0 and 1 have signalled: frame 0 is mapped now, frame 1 waits for the next frame.
        let mut polled = Vec::new();
        let plan = ring.collect(now, |slot| {
            polled.push(slot);
            slot != 2
        });
        assert_eq!(polled, vec![0]);
        assert_eq!(
            plan,
            Collect {
                map: Some(0),
                wait: false
            }
        );
        ring.published(0, now);
        assert_eq!(ring.issue_slot(), Some(0), "frame 3 reuses slot 0");
        ring.issued(0, false);
        // Frame 1 in slot 1 is now the oldest, ahead of frame 3 in slot 0.
        assert_eq!(ring.collect(now, |slot| slot != 2).map, Some(1));
        ring.published(1, now);
        assert_eq!(ring.collect(now, |slot| slot != 2), Collect::default());
    }

    #[test]
    fn resize_drops_in_flight_frames() {
        let now = Instant::now();
        let mut ring = ring_with(2, now);
        assert!(ring.resize((4, 2), now).is_empty(), "same size keeps reads");
        assert_eq!(ring.resize((8, 4), now), vec![0, 1]);
        assert_eq!(ring.size(), (8, 4));
        let later = now + STALL * 2;
        assert_eq!(ring.collect(later, |_| true), Collect::default());
        assert_eq!(ring.issue_slot(), Some(2));
    }

    #[test]
    fn stalled_fence_forces_a_wait_after_100_ms() {
        let start = Instant::now();
        let mut ring = ring_with(2, start);
        let soon = start + Duration::from_millis(99);
        assert_eq!(ring.collect(soon, |_| false), Collect::default());
        let late = start + Duration::from_millis(100);
        assert_eq!(
            ring.collect(late, |_| false),
            Collect {
                map: Some(0),
                wait: true
            }
        );
        // A publish restarts the clock.
        ring.published(0, late);
        assert_eq!(
            ring.collect(late + Duration::from_millis(50), |_| false),
            Collect::default()
        );
    }

    #[test]
    fn snapshot_comes_from_a_frame_issued_after_the_request() {
        let now = Instant::now();
        let mut ring = ring_with(1, now);
        let slot = ring.issue_slot().expect("free slot");
        ring.issued(slot, true);
        assert!(
            !ring.published(0, now),
            "frame 0 was drawn before the request"
        );
        // A resize drops frame 1; the request stays open for the next frame.
        ring.resize((2, 2), now);
        let slot = ring.issue_slot().expect("free slot");
        ring.issued(slot, false);
        assert!(ring.published(slot, now));
        let slot = ring.issue_slot().expect("free slot");
        ring.issued(slot, false);
        assert!(!ring.published(slot, now), "answered once");
    }
}

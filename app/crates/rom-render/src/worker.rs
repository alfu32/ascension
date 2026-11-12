//! The render thread and the bridge other threads use to talk to it.

use std::io;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use raylib::consts::PixelFormat;
use raylib::prelude::{RaylibRenderTexture2D, RaylibTexture2D, RenderTexture2D, TraceLogLevel};

use crate::camera::{Camera, CameraController, InputEvent};
use crate::entities::{EntitySnapshot, EntityView, SnapshotPair};
use crate::mailbox::{Frame, FrameMailbox, FramePixels};
use crate::pixels::rgba_to_bgra_flipped;
use crate::scene::Scene;
use crate::stats::{RenderStats, StatsWindow};
use crate::terrain::{TerrainScene, home_camera, overview_camera};

/// Smallest internal render scale, in percent of the canvas size.
pub const MIN_RENDER_SCALE: u32 = 50;
/// Largest internal render scale, in percent of the canvas size.
pub const MAX_RENDER_SCALE: u32 = 100;

const FRAME_INTERVAL: Duration = Duration::from_nanos(16_666_667);
const IDLE_WAIT: Duration = Duration::from_millis(16);

/// Size of the render target for a canvas of `layout` pixels at `percent` scale.
pub fn scaled_size(layout: (u32, u32), percent: u32) -> (u32, u32) {
    let percent = u64::from(percent.clamp(MIN_RENDER_SCALE, MAX_RENDER_SCALE));
    let scale = |extent: u32| {
        let scaled = (u64::from(extent) * percent + 50) / 100;
        u32::try_from(scaled).unwrap_or(u32::MAX).max(1)
    };
    (scale(layout.0), scale(layout.1))
}

/// State shared between the render thread and its bridges.
#[derive(Debug)]
struct Shared {
    layout: Mutex<(u32, u32)>,
    render_scale: AtomicU32,
    camera: Mutex<Option<Camera>>,
    input: Mutex<Vec<InputEvent>>,
    mailbox: FrameMailbox,
    terrain: Mutex<Option<Arc<TerrainScene>>>,
    terrain_ready: AtomicBool,
    box_count: AtomicU32,
    snapshots: Mutex<SnapshotPair>,
    start_markers: AtomicBool,
    snapshot_wanted: AtomicBool,
    snapshot: Mutex<Option<Arc<FramePixels>>>,
    stats: Mutex<RenderStats>,
    stop: AtomicBool,
    finished: (Mutex<bool>, Condvar),
}

/// Cloneable handle for publishing view state to, and taking frames from, the render thread.
#[derive(Clone, Debug)]
pub struct RenderBridge {
    shared: Arc<Shared>,
}

impl RenderBridge {
    /// Sets the canvas size in physical pixels.
    pub fn publish_layout(&self, width: u32, height: u32) {
        *lock(&self.shared.layout) = (width, height);
    }

    /// The canvas size last published, in physical pixels.
    pub fn layout(&self) -> (u32, u32) {
        *lock(&self.shared.layout)
    }

    /// Hands the render thread a terrain to mesh and draw, replacing any earlier one.
    ///
    /// The camera moves to the scene's first player start and is bounded by the map.
    pub fn publish_terrain(&self, scene: Arc<TerrainScene>) {
        self.shared.terrain_ready.store(false, Ordering::Release);
        *lock(&self.shared.terrain) = Some(scene);
    }

    /// True once the render thread has built the meshes of the last published terrain.
    pub fn terrain_ready(&self) -> bool {
        self.shared.terrain_ready.load(Ordering::Acquire)
    }

    /// Sets how many stress-test boxes are drawn (0 for none).
    pub fn publish_box_count(&self, count: u32) {
        self.shared.box_count.store(count, Ordering::Relaxed);
    }

    /// Hands over the newest entity snapshot; the render thread keeps the last two and
    /// interpolates between them by the wall-clock time since the newer one arrived.
    pub fn publish_snapshot(&self, snapshot: Arc<EntitySnapshot>) {
        lock(&self.shared.snapshots).push(snapshot, Instant::now());
    }

    /// Shows or hides the player-start pillars (hidden by default).
    pub fn publish_start_markers(&self, visible: bool) {
        self.shared.start_markers.store(visible, Ordering::Relaxed);
    }

    /// Asks for a copy of the next finished frame; fetch it with [`Self::take_snapshot`].
    pub fn request_snapshot(&self) {
        *lock(&self.shared.snapshot) = None;
        self.shared.snapshot_wanted.store(true, Ordering::Release);
    }

    /// The frame requested by [`Self::request_snapshot`] (BGRA, top-down), once it exists.
    pub fn take_snapshot(&self) -> Option<Arc<FramePixels>> {
        lock(&self.shared.snapshot).take()
    }

    /// Replaces the render thread's camera. Its bounds stay those of the current terrain.
    pub fn publish_camera(&self, camera: Camera) {
        *lock(&self.shared.camera) = Some(camera);
    }

    /// Sets the internal render scale in percent, clamped to 50–100.
    pub fn publish_render_scale(&self, percent: u32) {
        self.shared.render_scale.store(
            percent.clamp(MIN_RENDER_SCALE, MAX_RENDER_SCALE),
            Ordering::Relaxed,
        );
    }

    /// The current internal render scale in percent.
    pub fn render_scale(&self) -> u32 {
        self.shared.render_scale.load(Ordering::Relaxed)
    }

    /// Queues an input event for the next frame.
    pub fn push_input(&self, event: InputEvent) {
        lock(&self.shared.input).push(event);
    }

    /// Takes the newest finished frame, if one arrived since the last call.
    ///
    /// The frame is rendered at the internal scale; the UI stretches it to the canvas.
    pub fn latest_frame(&self) -> Option<Frame> {
        self.shared.mailbox.take()
    }

    /// Timings of the last second.
    pub fn stats(&self) -> RenderStats {
        *lock(&self.shared.stats)
    }
}

/// Stops the render thread and waits until it has released every GPU object.
#[derive(Clone, Debug)]
pub struct ShutdownHandle {
    shared: Arc<Shared>,
}

impl ShutdownHandle {
    /// Asks the render thread to stop and blocks until it has finished.
    pub fn stop_and_wait(&self) {
        self.shared.stop.store(true, Ordering::Release);
        let (finished, changed) = &self.shared.finished;
        let finished = lock(finished);
        let _finished = changed
            .wait_while(finished, |finished| !*finished)
            .unwrap_or_else(PoisonError::into_inner);
    }
}

/// Owns the render thread; dropping it stops and joins the thread.
#[derive(Debug)]
pub struct RenderWorker {
    shutdown: ShutdownHandle,
    thread: Option<JoinHandle<()>>,
}

impl RenderWorker {
    /// Spawns the render thread, which opens a hidden raylib window and renders continuously.
    pub fn start() -> io::Result<(Self, RenderBridge)> {
        let shared = Arc::new(Shared {
            layout: Mutex::new((0, 0)),
            render_scale: AtomicU32::new(MAX_RENDER_SCALE),
            camera: Mutex::new(None),
            input: Mutex::new(Vec::new()),
            mailbox: FrameMailbox::default(),
            terrain: Mutex::new(None),
            terrain_ready: AtomicBool::new(false),
            box_count: AtomicU32::new(0),
            snapshots: Mutex::new(SnapshotPair::default()),
            start_markers: AtomicBool::new(false),
            snapshot_wanted: AtomicBool::new(false),
            snapshot: Mutex::new(None),
            stats: Mutex::new(RenderStats::default()),
            stop: AtomicBool::new(false),
            finished: (Mutex::new(false), Condvar::new()),
        });
        let worker_shared = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("rom-render".into())
            .spawn(move || render_loop(&worker_shared))?;
        let worker = Self {
            shutdown: ShutdownHandle {
                shared: Arc::clone(&shared),
            },
            thread: Some(thread),
        };
        Ok((worker, RenderBridge { shared }))
    }

    /// A handle that can stop the render thread from elsewhere (for example a window-close callback).
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        self.shutdown.clone()
    }

    /// Stops the render thread and joins it. Safe to call more than once.
    pub fn stop(&mut self) {
        self.shutdown.stop_and_wait();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("rom-render: render thread panicked");
        }
    }
}

impl Drop for RenderWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Marks the render thread finished when dropped, including during a panic.
struct FinishedGuard<'a>(&'a Shared);

impl Drop for FinishedGuard<'_> {
    fn drop(&mut self) {
        let (finished, changed) = &self.0.finished;
        *lock(finished) = true;
        changed.notify_all();
    }
}

fn render_loop(shared: &Shared) {
    let _finished = FinishedGuard(shared);
    let (mut rl, thread) = raylib::init()
        .log_level(TraceLogLevel::LOG_WARNING)
        .size(640, 480)
        .title("Rise of Man renderer")
        .hidden()
        .build();
    let mut scene = Scene::load(&mut rl, &thread);
    let mut target: Option<(RenderTexture2D, (u32, u32))> = None;
    let mut camera = Camera::default();
    let mut controller = CameraController::default();
    let mut stats = StatsWindow::default();
    let mut blended: Vec<EntityView> = Vec::new();
    let mut sequence = 0_u64;
    let started = Instant::now();
    let mut last_frame = started;
    let mut deadline = started;

    while !shared.stop.load(Ordering::Acquire) {
        let frame_start = Instant::now();
        let seconds = frame_start
            .duration_since(last_frame)
            .as_secs_f32()
            .min(0.1);
        last_frame = frame_start;

        let published_terrain = lock(&shared.terrain).take();
        if let Some(terrain) = published_terrain {
            let (meshes, build_ms, upload_ms) = scene.set_terrain(&thread, &terrain);
            let (home, overview) = (home_camera(&terrain), overview_camera(&terrain));
            controller.set_scene_cameras(home, overview);
            camera = home;
            eprintln!(
                "terrain-mesh tiles={}x{} meshes={meshes} build_ms={build_ms:.1} upload_ms={upload_ms:.1}",
                terrain.width, terrain.height
            );
            shared.terrain_ready.store(true, Ordering::Release);
        }
        if let Some(published) = lock(&shared.camera).take() {
            camera = Camera {
                bounds: camera.bounds,
                ..published
            }
            .limited();
        }
        let events = std::mem::take(&mut *lock(&shared.input));
        for event in events {
            controller.apply(&mut camera, event);
        }
        let layout = *lock(&shared.layout);
        controller.set_viewport(layout.0, layout.1);
        controller.advance(&mut camera, seconds);

        if layout.0 == 0 || layout.1 == 0 {
            thread::sleep(IDLE_WAIT);
            continue;
        }
        let wanted = scaled_size(layout, shared.render_scale.load(Ordering::Relaxed));
        if target.as_ref().is_none_or(|(_, size)| *size != wanted) {
            // Drop the old target first so two full-size targets never coexist.
            target = None;
            match rl.load_render_texture(&thread, wanted.0, wanted.1) {
                Ok(texture) => target = Some((texture, wanted)),
                Err(error) => {
                    eprintln!(
                        "rom-render: create {}x{} render target: {error}",
                        wanted.0, wanted.1
                    );
                    thread::sleep(IDLE_WAIT);
                    continue;
                }
            }
        }
        let Some((texture, size)) = target.as_mut() else {
            continue;
        };
        let elapsed = frame_start.duration_since(started).as_secs_f32();
        let boxes = shared.box_count.load(Ordering::Relaxed) as usize;
        let pair = lock(&shared.snapshots).clone();
        pair.blend_at(frame_start, &mut blended);
        scene.prepare_entities(
            &blended,
            camera.yaw,
            shared.start_markers.load(Ordering::Relaxed),
        );
        scene.draw(&mut rl, &thread, texture, &camera, elapsed, boxes);

        let readback_start = Instant::now();
        let Some(pixels) = read_back(texture) else {
            eprintln!("rom-render: could not read back the render target");
            thread::sleep(IDLE_WAIT);
            continue;
        };
        let finished_at = Instant::now();
        sequence += 1;
        let pixels = Arc::new(pixels);
        if shared.snapshot_wanted.swap(false, Ordering::AcqRel) {
            *lock(&shared.snapshot) = Some(Arc::clone(&pixels));
        }
        shared.mailbox.publish(Frame {
            pixels,
            width: size.0,
            height: size.1,
            sequence,
        });
        stats.record(
            finished_at,
            millis(finished_at - frame_start),
            millis(finished_at - readback_start),
            *size,
        );
        *lock(&shared.stats) = stats.snapshot();

        deadline += FRAME_INTERVAL;
        let now = Instant::now();
        if deadline > now {
            thread::sleep(deadline - now);
        } else {
            deadline = now;
        }
    }
    // Free GPU objects while the GL context still exists; `rl` closes it when dropped last.
    drop(target);
    drop(scene);
}

/// Reads the render target back and converts it to top-down BGRA.
fn read_back(target: &RenderTexture2D) -> Option<FramePixels> {
    let image = target.texture().load_image().ok()?;
    if image.format() != PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 {
        return None;
    }
    let width = u32::try_from(image.width()).ok()?;
    let height = u32::try_from(image.height()).ok()?;
    let bytes = usize::try_from(width.checked_mul(height)?.checked_mul(4)?).ok()?;
    // SAFETY: LoadImageFromTexture returned a tightly packed RGBA8 buffer of `bytes` bytes owned by `image`, which outlives this slice.
    let rgba = unsafe { slice::from_raw_parts(image.data().cast::<u8>(), bytes) };
    let bgra = rgba_to_bgra_flipped(rgba, width as usize, height as usize)?;
    FramePixels::from_raw(width, height, bgra)
}

fn millis(duration: Duration) -> f32 {
    duration.as_secs_f32() * 1_000.0
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_size_rounds_and_clamps() {
        assert_eq!(scaled_size((1920, 1080), 100), (1920, 1080));
        assert_eq!(scaled_size((1920, 1080), 50), (960, 540));
        assert_eq!(scaled_size((1281, 801), 50), (641, 401));
        assert_eq!(scaled_size((1920, 1080), 10), (960, 540));
        assert_eq!(scaled_size((1, 1), 50), (1, 1));
    }
}

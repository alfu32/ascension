//! The render thread and the bridge other threads use to talk to it.

use std::io;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use raylib::consts::PixelFormat;
use raylib::prelude::{
    RaylibRenderTexture2D, RaylibTexture2D, RaylibTextureModeExt, RenderTexture2D, TraceLogLevel,
};

use crate::camera::{Camera, CameraController, InputEvent};
use crate::entities::{EntitySnapshot, EntityView, SnapshotPair};
use crate::globe::{Globe, PlanetScene};
use crate::hud::{HudOverlay, HudSnapshot};
use crate::hud_gpu::{HudGpu, MinimapGpu};
use crate::hud_layout::HudLayout;
use crate::hud_layout::Rect;
use crate::hud_minimap::GLOBE_MINIMAP_SIZE;
use crate::mailbox::{Frame, FrameMailbox, FramePixels};
use crate::orbit::{HOME_DISTANCE, OrbitCamera, OrbitController, lon_lat_of};
use crate::pick::{PickContext, PickController, PickEvent, canvas_view, canvas_view_orbit};
use crate::pixels::rgba_to_bgra_flipped;
use crate::scene::Scene;
use crate::stats::{RenderStats, StatsWindow};
use crate::terrain::{TerrainScene, home_camera, overview_camera};
use crate::vision::VisionMap;
use rom_geo::SurfacePos;

/// Smallest internal render scale, in percent of the canvas size.
pub const MIN_RENDER_SCALE: u32 = 50;
/// Largest internal render scale, in percent of the canvas size.
pub const MAX_RENDER_SCALE: u32 = 100;

/// Most chunk meshes recoloured per frame after a vision update.
const SHADE_CHUNKS_PER_FRAME: usize = 24;
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

/// How the orbit camera of a sphere planet starts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OrbitSettings {
    /// Largest camera distance in tiles; `None` means three planet radii.
    pub max_distance: Option<f32>,
    /// First view as longitude, latitude, distance and heading (degrees, tiles); `None` looks at
    /// the first player start.
    pub view: Option<[f32; 4]>,
    /// Where the camera starts looking when `view` is not given; `None` looks at the first
    /// start of the planet scene.
    pub home: Option<SurfacePos>,
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
    planet: Mutex<Option<(Arc<PlanetScene>, OrbitSettings)>>,
    terrain_ready: AtomicBool,
    box_count: AtomicU32,
    snapshots: Mutex<SnapshotPair>,
    hud: Mutex<Arc<HudSnapshot>>,
    hud_visible: AtomicBool,
    hud_scale: AtomicU32,
    pick_tx: Mutex<Sender<PickEvent>>,
    pick_rx: Mutex<Receiver<PickEvent>>,
    focus: Mutex<Option<[f32; 2]>>,
    focus_surface: Mutex<Option<SurfacePos>>,
    vision: Mutex<Option<Arc<VisionMap>>>,
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

    /// Hands the render thread a sphere planet to mesh and draw with an orbit camera, replacing
    /// any earlier terrain or planet.
    pub fn publish_planet(&self, scene: Arc<PlanetScene>, settings: OrbitSettings) {
        self.shared.terrain_ready.store(false, Ordering::Release);
        *lock(&self.shared.planet) = Some((scene, settings));
    }

    /// True once the render thread has built the meshes of the last published terrain or planet.
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

    /// Publishes the newest immutable interface snapshot.
    pub fn publish_hud(&self, snapshot: Arc<HudSnapshot>) {
        *lock(&self.shared.hud) = snapshot;
    }

    /// Configures HUD visibility and optional scale in percent of design pixels.
    pub fn publish_hud_options(&self, visible: bool, percent: Option<u32>) {
        self.shared.hud_visible.store(visible, Ordering::Relaxed);
        self.shared
            .hud_scale
            .store(percent.unwrap_or(0), Ordering::Relaxed);
    }

    /// Whether HUD drawing is enabled.
    pub fn hud_visible(&self) -> bool {
        self.shared.hud_visible.load(Ordering::Relaxed)
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

    /// Queues an input event for the next frame; the render thread decides what the HUD
    /// consumes, what moves the camera and what becomes a pick event.
    pub fn push_input(&self, event: InputEvent) {
        lock(&self.shared.input).push(event);
    }

    /// Takes the pick events the render thread produced since the last call, oldest first.
    pub fn take_picks(&self) -> Vec<PickEvent> {
        lock(&self.shared.pick_rx).try_iter().collect()
    }

    /// Asks the render thread to centre the camera on tile (`x`, `y`).
    pub fn focus_camera(&self, x: f32, y: f32) {
        *lock(&self.shared.focus) = Some([x, y]);
    }

    /// Asks the render thread to centre the orbit camera on a surface position of a sphere planet.
    pub fn focus_surface(&self, at: SurfacePos) {
        *lock(&self.shared.focus_surface) = Some(at);
    }

    /// Hands the render thread the newest visibility and territory map; it darkens unexplored
    /// and out-of-view tiles and tints the borders. Replaces any map not yet taken.
    pub fn publish_vision(&self, vision: Arc<VisionMap>) {
        *lock(&self.shared.vision) = Some(vision);
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
        let (pick_tx, pick_rx) = channel();
        let shared = Arc::new(Shared {
            layout: Mutex::new((0, 0)),
            render_scale: AtomicU32::new(MAX_RENDER_SCALE),
            camera: Mutex::new(None),
            input: Mutex::new(Vec::new()),
            mailbox: FrameMailbox::default(),
            terrain: Mutex::new(None),
            planet: Mutex::new(None),
            terrain_ready: AtomicBool::new(false),
            box_count: AtomicU32::new(0),
            snapshots: Mutex::new(SnapshotPair::default()),
            hud: Mutex::new(Arc::new(HudSnapshot::default())),
            hud_visible: AtomicBool::new(true),
            hud_scale: AtomicU32::new(0),
            pick_tx: Mutex::new(pick_tx),
            pick_rx: Mutex::new(pick_rx),
            focus: Mutex::new(None),
            focus_surface: Mutex::new(None),
            vision: Mutex::new(None),
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
        .title("Ascension: Rise of Man renderer")
        .hidden()
        .build();
    let mut scene = Scene::load(&mut rl, &thread);
    let mut hud: Option<HudGpu> = None;
    let mut minimap: Option<MinimapGpu> = None;
    let mut target: Option<(RenderTexture2D, (u32, u32))> = None;
    let mut camera = Camera::default();
    let mut controller = CameraController::default();
    let mut orbit: Option<OrbitCamera> = None;
    let mut orbit_controller = OrbitController::default();
    let mut globe: Option<Arc<Globe>> = None;
    let mut shades: Vec<[f32; 3]> = Vec::new();
    let mut stats = StatsWindow::default();
    let mut blended: Vec<EntityView> = Vec::new();
    let mut picker = PickController::default();
    let mut over_hud = false;
    let mut map_size = (0_u32, 0_u32);
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

        let published_planet = lock(&shared.planet).take();
        if let Some((planet, settings)) = published_planet {
            orbit = None;
            globe = None;
            minimap = None;
            if planet.is_valid() {
                let started_at = Instant::now();
                let built_globe = Arc::new(Globe::new(&planet));
                let geometry_ms = millis(started_at.elapsed());
                let (meshes, build_ms, upload_ms) =
                    scene.set_planet(&thread, &planet, &built_globe);
                minimap = match MinimapGpu::load_planet(&mut rl, &thread, &planet, &built_globe) {
                    Ok(map) => Some(map),
                    Err(error) => {
                        eprintln!("rom-render: minimap: {error}");
                        None
                    }
                };
                let radius = built_globe.radius();
                let max = settings
                    .max_distance
                    .unwrap_or_else(|| OrbitCamera::default_max_distance(radius));
                let (lon, lat) = settings
                    .home
                    .or_else(|| planet.starts.first().copied())
                    .map_or((0.0, 0.0), |s| {
                        // A home position carries milli-tiles of its face.
                        lon_lat_of(built_globe.dir_of(s))
                    });
                let home = OrbitCamera::at(
                    lon.to_degrees(),
                    lat.to_degrees(),
                    HOME_DISTANCE,
                    0.0,
                    radius,
                    max,
                );
                orbit = Some(settings.view.map_or(home, |[lon, lat, distance, heading]| {
                    OrbitCamera::at(lon, lat, distance, heading, radius, max)
                }));
                orbit_controller.set_home(home);
                map_size = GLOBE_MINIMAP_SIZE;
                eprintln!(
                    "planet-mesh n={} meshes={meshes} build_ms={:.1} upload_ms={upload_ms:.1}",
                    planet.n,
                    build_ms + geometry_ms
                );
                globe = Some(built_globe);
            } else {
                eprintln!("rom-render: ignoring a planet scene with inconsistent tile data");
            }
            shared.terrain_ready.store(true, Ordering::Release);
        }
        let published_terrain = lock(&shared.terrain).take();
        if let Some(terrain) = published_terrain {
            orbit = None;
            globe = None;
            let (meshes, build_ms, upload_ms) = scene.set_terrain(&thread, &terrain);
            minimap = if terrain.is_valid() {
                match MinimapGpu::load(&mut rl, &thread, &terrain) {
                    Ok(map) => Some(map),
                    Err(error) => {
                        eprintln!("rom-render: minimap: {error}");
                        None
                    }
                }
            } else {
                None
            };
            let (home, overview) = (home_camera(&terrain), overview_camera(&terrain));
            controller.set_scene_cameras(home, overview);
            camera = home;
            map_size = (terrain.width, terrain.height);
            eprintln!(
                "terrain-mesh tiles={}x{} meshes={meshes} build_ms={build_ms:.1} upload_ms={upload_ms:.1}",
                terrain.width, terrain.height
            );
            shared.terrain_ready.store(true, Ordering::Release);
        }
        if let Some(vision) = lock(&shared.vision).take()
            && vision.is_valid()
        {
            vision.tile_shades(&mut shades);
            scene.apply_shades(&shades);
            if let Some(map) = &mut minimap {
                map.apply_shades(&shades);
            }
        }
        scene.run_shade_queue(SHADE_CHUNKS_PER_FRAME);
        if let Some(published) = lock(&shared.camera).take()
            && orbit.is_none()
        {
            camera = Camera {
                bounds: camera.bounds,
                ..published
            }
            .limited();
        }
        if let Some([x, z]) = lock(&shared.focus).take()
            && orbit.is_none()
        {
            camera.target = [x, 0.0, z];
            camera.pan(0.0, 0.0);
        }
        if let Some(at) = lock(&shared.focus_surface).take()
            && let (Some(orbit), Some(globe)) = (orbit.as_mut(), globe.as_deref())
        {
            orbit.look_at(globe.dir_of(at));
        }
        let layout = *lock(&shared.layout);
        let hud_visible = shared.hud_visible.load(Ordering::Relaxed);
        let percent = shared.hud_scale.load(Ordering::Relaxed);
        let hud_snapshot = Arc::clone(&lock(&shared.hud));
        let canvas = HudLayout::new(layout.0, layout.1, (percent > 0).then_some(percent));
        let pair = lock(&shared.snapshots).clone();
        pair.blend_at_on(frame_start, globe.as_deref(), &mut blended);
        let events = std::mem::take(&mut *lock(&shared.input));
        {
            let surface = scene.surface();
            let ground = |x: f32, y: f32| surface.map_or(0.0, |s| s.height(x, y));
            let now_ms = frame_start.duration_since(started).as_millis() as u64;
            for event in events {
                let ctx = PickContext {
                    layout: canvas,
                    view: current_view(&camera, orbit.as_ref(), &canvas, hud_visible),
                    entities: &blended,
                    ground: &ground,
                    globe: globe.as_deref(),
                    hud: &hud_snapshot,
                    hud_visible,
                    map_size,
                    now_ms,
                };
                for pick in picker.handle_with_orbit(event, &ctx, &mut camera, orbit.as_mut()) {
                    let _ = lock(&shared.pick_tx).send(pick);
                }
                let position = match event {
                    InputEvent::PointerMoved { x, y }
                    | InputEvent::ButtonDown { x, y, .. }
                    | InputEvent::ButtonUp { x, y, .. } => Some((x, y)),
                    _ => None,
                };
                if let Some((x, y)) = position {
                    over_hud = hud_visible && canvas.contains_hud(x, y);
                }
                match event {
                    InputEvent::ButtonDown { .. } | InputEvent::Wheel { .. } if over_hud => {}
                    InputEvent::PointerMoved { .. } if over_hud => {
                        steer(
                            InputEvent::PointerLeft,
                            &mut orbit,
                            &mut orbit_controller,
                            &mut camera,
                            &mut controller,
                        );
                    }
                    InputEvent::ButtonUp { .. } if over_hud => {
                        steer(
                            event,
                            &mut orbit,
                            &mut orbit_controller,
                            &mut camera,
                            &mut controller,
                        );
                        steer(
                            InputEvent::PointerLeft,
                            &mut orbit,
                            &mut orbit_controller,
                            &mut camera,
                            &mut controller,
                        );
                    }
                    _ => steer(
                        event,
                        &mut orbit,
                        &mut orbit_controller,
                        &mut camera,
                        &mut controller,
                    ),
                }
            }
        }
        controller.set_viewport(layout.0, layout.1);
        orbit_controller.set_viewport(layout.0, layout.1);
        match orbit.as_mut() {
            Some(orbit) => orbit_controller.advance(orbit, seconds),
            None => controller.advance(&mut camera, seconds),
        }

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
        scene.prepare_entities(
            &blended,
            camera.yaw,
            shared.start_markers.load(Ordering::Relaxed),
            orbit.as_ref(),
        );
        scene.prepare_overlays(&blended, &hud_snapshot, camera.yaw, orbit.as_ref());
        // Scale explicit HUD pixels with the offscreen render target, just like automatic scale.
        let mut hud_layout = HudLayout::new(size.0, size.1, None);
        if percent > 0 {
            hud_layout.scale = percent as f32 / 100.0 * size.0 as f32 / layout.0 as f32;
        }
        scene.projection_centre_y = if hud_visible {
            crate::hud_minimap::ndc_centre_y(hud_layout.world_area(), size.1 as f32)
        } else {
            0.0
        };
        match &orbit {
            Some(orbit) => scene.draw_globe(&mut rl, &thread, texture, orbit),
            None => scene.draw(&mut rl, &thread, texture, &camera, elapsed, boxes),
        }
        if hud_visible {
            if hud
                .as_ref()
                .is_none_or(|h| h.needs_rebake(hud_layout.scale))
            {
                match HudGpu::load(&mut rl, &thread, hud_layout.scale) {
                    Ok(baked) => hud = Some(baked),
                    Err(error) => eprintln!("rom-render: HUD: {error}"),
                }
            }
            if let Some(hud) = &mut hud {
                let ratio = size.0 as f32 / layout.0.max(1) as f32;
                let hud_overlay = {
                    let surface = scene.surface();
                    let ground = |x: f32, y: f32| surface.map_or(0.0, |s| s.height(x, y));
                    let ctx = PickContext {
                        layout: canvas,
                        view: current_view(&camera, orbit.as_ref(), &canvas, hud_visible),
                        entities: &blended,
                        ground: &ground,
                        globe: globe.as_deref(),
                        hud: &hud_snapshot,
                        hud_visible,
                        map_size,
                        now_ms: 0,
                    };
                    HudOverlay {
                        hover: picker.hover(&ctx),
                        drag_box: picker
                            .drag_rect()
                            .map(|r| Rect::new(r.x * ratio, r.y * ratio, r.w * ratio, r.h * ratio)),
                    }
                };
                let mut drawing = rl.begin_texture_mode(&thread, texture);
                if let Some(map) = &minimap {
                    map.draw(&mut drawing, &hud_layout, &blended, &camera, orbit.as_ref());
                }
                hud.draw(&mut drawing, &hud_layout, &hud_snapshot, &hud_overlay);
            }
        }

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
    drop(hud);
    drop(minimap);
    drop(target);
    drop(scene);
}

/// The canvas view of whichever camera is active.
fn current_view(
    camera: &Camera,
    orbit: Option<&OrbitCamera>,
    layout: &HudLayout,
    hud_visible: bool,
) -> crate::pick::View {
    match orbit {
        Some(orbit) => canvas_view_orbit(orbit, layout, hud_visible),
        None => canvas_view(camera, layout, hud_visible),
    }
}

/// Sends an input event to the camera controller that is active.
fn steer(
    event: InputEvent,
    orbit: &mut Option<OrbitCamera>,
    orbit_controller: &mut OrbitController,
    camera: &mut Camera,
    controller: &mut CameraController,
) {
    match orbit {
        Some(orbit) => orbit_controller.apply(orbit, event),
        None => controller.apply(camera, event),
    }
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

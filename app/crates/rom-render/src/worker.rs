//! The render thread and the bridge other threads use to talk to it.

use std::io;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use raylib::consts::PixelFormat;
use raylib::prelude::{
    RaylibRenderTexture2D, RaylibTexture2D, RaylibTextureModeExt, RenderTexture2D, TraceLogLevel,
};

use crate::camera::{Camera, CameraController, InputEvent, Key, PointerButton};
use crate::entities::{EntitySnapshot, EntityView, SnapshotPair};
use crate::globe::{Globe, PlanetScene};
use crate::hud::{HudOverlay, HudSnapshot};
use crate::hud_gpu::{HudGpu, MinimapGpu, MinimapOverlays, draw_level_pips, draw_space_minimap};
use crate::hud_layout::HudLayout;
use crate::hud_layout::Rect;
use crate::hud_minimap::{GLOBE_MINIMAP_SIZE, ndc_centre_y};
use crate::mailbox::{Frame, FrameMailbox, FramePixels};
use crate::orbit::{
    HOME_DISTANCE, OrbitCamera, OrbitController, dir_from_lon_lat, lon_lat_of, pan_index,
};
use crate::pick::{PickContext, PickController, PickEvent, canvas_view, canvas_view_orbit};
use crate::pixels::rgba_to_bgra_flipped;
use crate::scene::Scene;
use crate::space::{SpaceContext, SpacePicker, SpaceSnapshot, ship_frame_pos};
use crate::stats::{RenderStats, StatsWindow};
use crate::sun::SunClock;
use crate::terrain::{TerrainScene, home_camera, overview_camera};
use crate::universe::UniverseScene;
use crate::vision::VisionMap;
use crate::zoom::{Home, Level, ZoomCamera, ZoomScale};
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
    /// The universe body this planet is. With a universe already published, a planet with a body
    /// replaces the globe of the zoom camera in place (the camera keeps its place); without one,
    /// the planet is the first one.
    pub body: Option<u32>,
}

/// What the camera looks at, for the simulation thread to publish what it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewState {
    /// The level shown.
    pub level: Level,
    /// The star system the view is anchored to.
    pub system: Option<u32>,
    /// The planet whose globe is loaded.
    pub surface: Option<u32>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            level: Level::Globe,
            system: None,
            surface: None,
        }
    }
}

/// How the zoom above the planet camera starts, for a published universe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UniverseSettings {
    /// Id of the home system and of the home planet, the one drawn as a globe.
    pub home: Option<(u32, u32)>,
    /// A level to start in, over the home planet (`--view`).
    pub view: Option<Level>,
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
    universe: Mutex<Option<(Arc<UniverseScene>, UniverseSettings)>>,
    zoom_cap: AtomicU8,
    /// Ground half-width in milli-tiles the camera may show; `u32::MAX` until published.
    view_range: AtomicU32,
    /// Zoom out to the limit once, as soon as the view range is known.
    zoom_out: AtomicBool,
    zoom_all: AtomicBool,
    space: Mutex<Arc<SpaceSnapshot>>,
    view_state: Mutex<ViewState>,
    surface_request: Mutex<Option<u32>>,
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
    graticule: AtomicBool,
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

    /// Hands the render thread the universe to zoom out into: the star map and the systems.
    /// Publish it before the home planet's [`PlanetScene`]; the zoom starts once both are there.
    pub fn publish_universe(&self, scene: Arc<UniverseScene>, settings: UniverseSettings) {
        *lock(&self.shared.universe) = Some((scene, settings));
    }

    /// A bridge with no render thread behind it, for tests of the other threads: everything
    /// published stays in it, nothing is drawn and no window opens.
    pub fn detached() -> Self {
        Self {
            shared: Arc::new(Shared::new()),
        }
    }

    /// Hands over what is in space: ships, claims and transits (the newest replaces the last).
    pub fn publish_space(&self, space: Arc<SpaceSnapshot>) {
        *lock(&self.shared.space) = space;
    }

    /// What the camera looks at now.
    pub fn view_state(&self) -> ViewState {
        *lock(&self.shared.view_state)
    }

    /// The planet whose globe the camera asked for since the last call, once per request.
    pub fn take_surface_request(&self) -> Option<u32> {
        lock(&self.shared.surface_request).take()
    }

    /// Asks for the globe of a planet as the camera would (tests and scripts).
    pub fn request_surface(&self, body: u32) {
        *lock(&self.shared.surface_request) = Some(body);
    }

    /// Sets the farthest level the zoom may reach (the player's age decides it).
    pub fn publish_zoom_cap(&self, level: Level) {
        self.shared.zoom_cap.store(level as u8, Ordering::Relaxed);
    }

    /// Sets the largest ground half-width, in milli-tiles, the camera may show (the player's age
    /// decides it); 0 allows the whole globe.
    pub fn publish_view_range(&self, milli_tiles: u32) {
        self.shared.view_range.store(milli_tiles, Ordering::Relaxed);
    }

    /// Asks the camera to zoom out to its limit once, when the view range is known.
    pub fn publish_zoom_out(&self) {
        self.shared.zoom_out.store(true, Ordering::Relaxed);
    }

    /// Lifts the zoom cap for testing (`--zoom-all`).
    pub fn publish_zoom_all(&self, all: bool) {
        self.shared.zoom_all.store(all, Ordering::Relaxed);
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

    /// Shows or hides the latitude and longitude lines of a sphere planet (shown by default; the
    /// `G` key toggles them when the command palette leaves it free; Shift+G always toggles).
    pub fn publish_graticule(&self, visible: bool) {
        self.shared.graticule.store(visible, Ordering::Relaxed);
    }

    /// Whether the latitude and longitude lines are shown.
    pub fn graticule(&self) -> bool {
        self.shared.graticule.load(Ordering::Relaxed)
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
        let shared = Arc::new(Shared::new());
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

impl Shared {
    fn new() -> Self {
        let (pick_tx, pick_rx) = channel();
        Self {
            layout: Mutex::new((0, 0)),
            render_scale: AtomicU32::new(MAX_RENDER_SCALE),
            camera: Mutex::new(None),
            input: Mutex::new(Vec::new()),
            mailbox: FrameMailbox::default(),
            terrain: Mutex::new(None),
            planet: Mutex::new(None),
            universe: Mutex::new(None),
            zoom_cap: AtomicU8::new(Level::Globe as u8),
            view_range: AtomicU32::new(u32::MAX),
            zoom_out: AtomicBool::new(false),
            zoom_all: AtomicBool::new(false),
            space: Mutex::new(Arc::new(SpaceSnapshot::default())),
            view_state: Mutex::new(ViewState::default()),
            surface_request: Mutex::new(None),
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
            graticule: AtomicBool::new(true),
            snapshot_wanted: AtomicBool::new(false),
            snapshot: Mutex::new(None),
            stats: Mutex::new(RenderStats::default()),
            stop: AtomicBool::new(false),
            finished: (Mutex::new(false), Condvar::new()),
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
    let mut planet_scene: Option<Arc<PlanetScene>> = None;
    let mut home_dir = [0.0, 0.0, 1.0];
    let mut sun_clock: Option<SunClock> = None;
    let mut pending_universe: Option<(Arc<UniverseScene>, UniverseSettings)> = None;
    let mut zoom: Option<ZoomCamera> = None;
    let mut surface_requested: Option<u32> = None;
    let mut space_picker = SpacePicker::default();
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
            // A planet with a body, published while the zoom camera exists, is another globe for
            // the same camera.
            let swap = settings.body.is_some() && zoom.is_some();
            orbit = None;
            globe = None;
            if !swap {
                zoom = None;
            }
            planet_scene = None;
            minimap = None;
            if planet.is_valid() {
                let started_at = Instant::now();
                let built_globe = Arc::new(Globe::new(&planet));
                let geometry_ms = millis(started_at.elapsed());
                let home_start = settings.home.or_else(|| planet.starts.first().copied());
                let clock = SunClock::new(planet.sun, home_start.map(|s| built_globe.dir_of(s)));
                let tick = lock(&shared.snapshots).latest_tick().unwrap_or(0);
                let (meshes, build_ms, upload_ms) =
                    scene.set_planet(&thread, &planet, &built_globe, clock.light(tick));
                sun_clock = Some(clock);
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
                // A fixed light stands over the first view, as the home start was before.
                sun_clock = sun_clock.map(|clock| clock.fixed_over(orbit.as_ref().map(|o| o.dir)));
                orbit_controller.set_home(home);
                home_dir = dir_from_lon_lat(lon, lat);
                planet_scene = Some(Arc::clone(&planet));
                map_size = GLOBE_MINIMAP_SIZE;
                eprintln!(
                    "planet-mesh n={} meshes={meshes} build_ms={:.1} upload_ms={upload_ms:.1}",
                    planet.n,
                    build_ms + geometry_ms
                );
                globe = Some(built_globe);
                if let (true, Some(zoom_camera), Some(orbit_camera), Some(body)) =
                    (swap, zoom.as_mut(), orbit.as_mut(), settings.body)
                {
                    zoom_camera.surface_ready(body, radius, max, home_dir, orbit_camera);
                    scene.set_universe(
                        &mut rl,
                        &thread,
                        &zoom_camera.scene_arc(),
                        Some(body),
                        planet_scene.as_deref(),
                    );
                }
                surface_requested = None;
            } else {
                eprintln!("rom-render: ignoring a planet scene with inconsistent tile data");
            }
            shared.terrain_ready.store(true, Ordering::Release);
        }
        if let Some(published) = lock(&shared.universe).take() {
            pending_universe = Some(published);
        }
        if let (Some(_), Some(globe), Some(orbit)) = (&pending_universe, &globe, &mut orbit)
            && let Some((universe, settings)) = pending_universe.take()
        {
            let scale = ZoomScale::new(globe.radius(), orbit.max_distance);
            scene.set_universe(
                &mut rl,
                &thread,
                &universe,
                settings.home.map(|(_, body)| body),
                planet_scene.as_deref(),
            );
            let home = settings.home.map(|(system, body)| Home {
                system,
                body,
                dir: home_dir,
            });
            let mut camera = ZoomCamera::new(universe, scale, home, Level::Globe);
            camera.set_cap(current_cap(shared));
            if let Some(level) = settings.view {
                camera.set_view(level, orbit);
            }
            zoom = Some(camera);
        }
        let published_terrain = lock(&shared.terrain).take();
        if let Some(terrain) = published_terrain {
            orbit = None;
            globe = None;
            zoom = None;
            sun_clock = None;
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
        if let Some(zoom) = zoom.as_mut() {
            zoom.set_cap(current_cap(shared));
        }
        let view_range = shared.view_range.load(Ordering::Relaxed);
        if view_range != u32::MAX {
            let all = shared.zoom_all.load(Ordering::Relaxed);
            let range = if all { 0 } else { view_range };
            let zoom_out = shared.zoom_out.swap(false, Ordering::Relaxed);
            if let Some(orbit) = orbit.as_mut() {
                orbit.set_view_range(range);
                if let Some(zoom) = zoom.as_mut() {
                    zoom.set_globe_limit((range > 0).then(|| orbit.distance_cap()));
                    if zoom_out {
                        zoom.set_l(f32::MAX);
                    }
                } else if zoom_out {
                    orbit.frame_globe();
                }
            } else {
                let half_width = (range > 0).then(|| range as f32 / 1000.0);
                camera.limit_half_width(half_width);
                if zoom_out {
                    camera.distance = f32::MAX;
                    camera.limit_half_width(half_width);
                    camera = camera.limited();
                }
            }
        }
        let layout = *lock(&shared.layout);
        let hud_visible = shared.hud_visible.load(Ordering::Relaxed);
        let percent = shared.hud_scale.load(Ordering::Relaxed);
        let hud_snapshot = Arc::clone(&lock(&shared.hud));
        let canvas = HudLayout::new(layout.0, layout.1, (percent > 0).then_some(percent));
        let space_snapshot = Arc::clone(&lock(&shared.space));
        let pair = lock(&shared.snapshots).clone();
        pair.blend_at_on(frame_start, globe.as_deref(), &mut blended);
        if let Some(clock) = &sun_clock {
            // Simulation time drives the light, so it follows game speed and pause.
            let tick = pair.latest_tick().unwrap_or(0);
            scene.set_sun(clock.light(tick));
            // The minimap shows the night side of the planet while the planet is in view.
            if let Some(map) = &mut minimap
                && zoom.as_ref().is_none_or(|z| z.level() <= Level::Globe)
            {
                map.set_light(clock.toward(tick));
            }
            if let Some(zoom) = zoom.as_mut() {
                zoom.follow_sun(clock.toward(tick));
            }
        }
        let events = std::mem::take(&mut *lock(&shared.input));
        {
            let surface = scene.surface();
            let ground = |x: f32, y: f32| surface.map_or(0.0, |s| s.height(x, y));
            let now_ms = frame_start.duration_since(started).as_millis() as u64;
            for event in events {
                if graticule_key(event, &hud_snapshot) {
                    let shown = shared.graticule.load(Ordering::Relaxed);
                    shared.graticule.store(!shown, Ordering::Relaxed);
                    continue;
                }
                if let (Some(zoom), Some(orbit)) = (zoom.as_mut(), orbit.as_mut()) {
                    let mut space_events = Vec::new();
                    let consumed = zoom_event(
                        &mut ZoomInput {
                            zoom,
                            orbit,
                            picker: &mut space_picker,
                            space: &space_snapshot,
                            out: &mut space_events,
                        },
                        event,
                        over_hud,
                        now_ms,
                        &canvas,
                        hud_visible,
                    );
                    for pick in space_events {
                        let _ = lock(&shared.pick_tx).send(pick);
                    }
                    if consumed {
                        continue;
                    }
                }
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
                let mut at_edge = false;
                if let Some((x, y)) = position {
                    over_hud = hud_visible && canvas.contains_hud(x, y);
                    at_edge = canvas.at_edge(x, y);
                }
                match event {
                    // The outermost pixels pan the camera even where the HUD covers the edge.
                    InputEvent::PointerMoved { .. } if over_hud && at_edge => steer(
                        event,
                        &mut orbit,
                        &mut orbit_controller,
                        &mut camera,
                        &mut controller,
                    ),
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
        if !picker.is_panning() {
            match (orbit.as_mut(), zoom.as_mut()) {
                (Some(orbit), Some(zoom)) => {
                    // A camera reset by the planet controller moves the log-distance with it.
                    zoom.adopt_orbit(orbit);
                    zoom.advance(seconds);
                    zoom.sync_orbit(orbit);
                    if zoom.pans_planet() {
                        orbit_controller.advance(orbit, seconds);
                    }
                }
                (Some(orbit), None) => orbit_controller.advance(orbit, seconds),
                (None, _) => controller.advance(&mut camera, seconds),
            }
        }

        if let Some(zoom) = &zoom {
            *lock(&shared.view_state) = ViewState {
                level: zoom.level(),
                system: zoom.system_id(),
                surface: zoom
                    .surface_body()
                    .and_then(|i| zoom.scene().bodies.get(i))
                    .map(|b| b.id),
            };
            match zoom.want_surface() {
                Some(body) if surface_requested != Some(body) => {
                    surface_requested = Some(body);
                    *lock(&shared.surface_request) = Some(body);
                }
                None => surface_requested = None,
                _ => {}
            }
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
        scene.set_space_state(&space_snapshot, &hud_snapshot.selected);
        let graticule = shared.graticule.load(Ordering::Relaxed);
        scene.show_graticule = graticule;
        match (&orbit, &zoom) {
            (Some(orbit), Some(zoom)) if scene.has_space() => {
                scene.draw_zoomed(&mut rl, &thread, texture, zoom, orbit);
            }
            (Some(orbit), _) => scene.draw_globe(&mut rl, &thread, texture, orbit),
            (None, _) => scene.draw(&mut rl, &thread, texture, &camera, elapsed, boxes),
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
                            .or_else(|| space_picker.drag_rect())
                            .map(|r| Rect::new(r.x * ratio, r.y * ratio, r.w * ratio, r.h * ratio)),
                    }
                };
                let mut drawing = rl.begin_texture_mode(&thread, texture);
                let space_level = zoom
                    .as_ref()
                    .map(ZoomCamera::level)
                    .filter(|level| *level >= Level::System);
                match (&zoom, &orbit, space_level) {
                    (Some(zoom), Some(orbit), Some(_)) => {
                        draw_space_minimap(&mut drawing, &hud_layout, zoom, orbit);
                    }
                    _ => {
                        if let Some(map) = &minimap {
                            map.draw(
                                &mut drawing,
                                &hud_layout,
                                &blended,
                                &camera,
                                orbit.as_ref(),
                                MinimapOverlays {
                                    cities: &hud_snapshot.city_radii,
                                    graticule,
                                },
                            );
                        }
                    }
                }
                if let Some(zoom) = &zoom {
                    let alpha = zoom.weights().map;
                    hud.draw_labels(&mut drawing, &hud_layout, scene.space_labels(), alpha);
                }
                hud.draw(&mut drawing, &hud_layout, &hud_snapshot, &hud_overlay);
                if let Some(zoom) = &zoom {
                    draw_level_pips(&mut drawing, &hud_layout, zoom.level(), zoom.cap());
                }
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

/// Whether `event` is the graticule key: `G` without Ctrl or Alt while the command palette has
/// no button on that key, or Shift+G even when a palette command uses G.
fn graticule_key(event: InputEvent, hud: &HudSnapshot) -> bool {
    let InputEvent::Hotkey { ch, mods } = event else {
        return false;
    };
    if !ch.eq_ignore_ascii_case(&'g') || mods.ctrl || mods.alt {
        return false;
    }
    let slot = crate::hud_model::slot_of_key(ch);
    mods.shift
        || !hud
            .palette
            .as_ref()
            .is_some_and(|p| p.buttons.iter().any(|b| Some(b.slot) == slot))
}

/// The farthest level the zoom may reach now.
fn current_cap(shared: &Shared) -> Level {
    if shared.zoom_all.load(Ordering::Relaxed) {
        return Level::StarMap;
    }
    Level::ALL
        .get(usize::from(shared.zoom_cap.load(Ordering::Relaxed)))
        .copied()
        .unwrap_or(Level::Globe)
}

/// What `zoom_event` works on: the zoom camera, the planet camera, the space picker, the ships
/// it picks among and where the pick events go.
struct ZoomInput<'a> {
    zoom: &'a mut ZoomCamera,
    orbit: &'a mut OrbitCamera,
    picker: &'a mut SpacePicker,
    space: &'a SpaceSnapshot,
    out: &'a mut Vec<PickEvent>,
}

/// What the zoom camera does with an input event; true when it consumed the event. The wheel
/// zooms through every level, `Home` flies to the home globe, and above the planet range the
/// pan keys move across the frame, a double click flies to the body or system under it, and the
/// other clicks pick ships, bodies and systems for the simulation thread.
fn zoom_event(
    input: &mut ZoomInput,
    event: InputEvent,
    over_hud: bool,
    now_ms: u64,
    canvas: &HudLayout,
    hud_visible: bool,
) -> bool {
    let ZoomInput {
        zoom,
        orbit,
        picker,
        space,
        out,
    } = input;
    let in_space = zoom.level() >= Level::System;
    let on_world = |x: f32, y: f32| in_space && !(hud_visible && canvas.contains_hud(x, y));
    match event {
        InputEvent::Wheel { steps } if !over_hud => {
            zoom.wheel(steps);
            zoom.sync_orbit(orbit);
            true
        }
        InputEvent::KeyDown(Key::FrameMap) => {
            zoom.fly_home(orbit);
            true
        }
        InputEvent::KeyDown(key) => match pan_index(key) {
            Some(index) if !zoom.pans_planet() => {
                zoom.hold(index, true);
                true
            }
            _ => false,
        },
        InputEvent::KeyUp(key) => {
            if let Some(index) = pan_index(key) {
                zoom.hold(index, false);
            }
            false
        }
        InputEvent::ReleaseAll => {
            (0..4).for_each(|index| zoom.hold(index, false));
            picker.cancel();
            false
        }
        InputEvent::PointerMoved { .. } if in_space => {
            let ships = ship_points(zoom, space);
            let ctx = space_context(zoom, orbit, canvas, hud_visible, &ships);
            out.extend(picker.handle(event, &ctx.context(), now_ms));
            false
        }
        InputEvent::ButtonDown {
            button: PointerButton::Left,
            x,
            y,
            ..
        } if on_world(x, y) => {
            let size = (canvas.width, canvas.height);
            let centre_y = centre_of(canvas, hud_visible);
            let view = match zoom.level() {
                Level::System => zoom.system_view(orbit).view(size, centre_y),
                _ => zoom.map_view().view(size, centre_y),
            };
            if zoom.press(now_ms, (x, y), &view, orbit) {
                picker.cancel();
            } else {
                let ships = ship_points(zoom, space);
                let ctx = space_context(zoom, orbit, canvas, hud_visible, &ships);
                out.extend(picker.handle(event, &ctx.context(), now_ms));
            }
            true
        }
        InputEvent::ButtonDown {
            button: PointerButton::Right,
            x,
            y,
            ..
        } if on_world(x, y) => {
            let ships = ship_points(zoom, space);
            let ctx = space_context(zoom, orbit, canvas, hud_visible, &ships);
            out.extend(picker.handle(event, &ctx.context(), now_ms));
            true
        }
        InputEvent::ButtonUp {
            button: PointerButton::Left,
            x,
            y,
            ..
        } => {
            if !on_world(x, y) {
                return false;
            }
            let ships = ship_points(zoom, space);
            let ctx = space_context(zoom, orbit, canvas, hud_visible, &ships);
            out.extend(picker.handle(event, &ctx.context(), now_ms));
            true
        }
        _ => false,
    }
}

fn centre_of(canvas: &HudLayout, hud_visible: bool) -> f32 {
    if hud_visible {
        ndc_centre_y(canvas.world_area(), canvas.height)
    } else {
        0.0
    }
}

/// The ships of the anchored system with their places in the system frame.
fn ship_points(zoom: &ZoomCamera, space: &SpaceSnapshot) -> Vec<(u32, [f32; 3])> {
    let scene = zoom.scene();
    if space.system != zoom.system_id() {
        return Vec::new();
    }
    let outermost = zoom.system_id().map_or(1.0, |s| scene.system_outermost(s));
    space
        .ships
        .iter()
        .map(|ship| {
            (
                ship.id,
                ship_frame_pos(scene, zoom.placed(), outermost, ship),
            )
        })
        .collect()
}

/// The pieces of a [`SpaceContext`] that own their data.
struct OwnedSpaceContext<'a> {
    view: crate::pick::View,
    star_map: bool,
    ships: &'a [(u32, [f32; 3])],
    markers: Vec<(crate::zoom::Target, (f32, f32, f32))>,
    scene: &'a crate::universe::UniverseScene,
    outermost: f32,
}

impl OwnedSpaceContext<'_> {
    fn context(&self) -> SpaceContext<'_> {
        SpaceContext {
            view: self.view,
            star_map: self.star_map,
            ships: self.ships,
            markers: &self.markers,
            scene: self.scene,
            outermost: self.outermost,
        }
    }
}

fn space_context<'a>(
    zoom: &'a ZoomCamera,
    orbit: &OrbitCamera,
    canvas: &HudLayout,
    hud_visible: bool,
    ships: &'a [(u32, [f32; 3])],
) -> OwnedSpaceContext<'a> {
    let size = (canvas.width, canvas.height);
    let centre_y = centre_of(canvas, hud_visible);
    let star_map = zoom.level() >= Level::StarMap;
    let view = if star_map {
        zoom.map_view().view(size, centre_y)
    } else {
        zoom.system_view(orbit).view(size, centre_y)
    };
    OwnedSpaceContext {
        markers: zoom.markers(&view),
        view,
        star_map,
        ships,
        scene: zoom.scene(),
        outermost: zoom
            .system_id()
            .map_or(1.0, |s| zoom.scene().system_outermost(s)),
    }
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
    fn graticule_toggle_preserves_palette_commands() {
        use crate::camera::Modifiers;
        use crate::hud_model::{HudButton, HudPalette};
        let key = |mods| InputEvent::Hotkey { ch: 'g', mods };
        let mut hud = HudSnapshot::default();
        assert!(graticule_key(key(Modifiers::default()), &hud));
        hud.palette = Some(HudPalette {
            buttons: vec![HudButton {
                slot: 9,
                icon: String::new(),
                name: String::new(),
                price: String::new(),
                state: crate::hud_model::ButtonState::Ready,
                armed: false,
            }],
            ..HudPalette::default()
        });
        assert!(!graticule_key(key(Modifiers::default()), &hud));
        assert!(graticule_key(
            key(Modifiers {
                shift: true,
                ..Modifiers::default()
            }),
            &hud
        ));
        assert!(!graticule_key(
            key(Modifiers {
                ctrl: true,
                ..Modifiers::default()
            }),
            &hud
        ));
        assert!(!graticule_key(
            key(Modifiers {
                alt: true,
                ..Modifiers::default()
            }),
            &hud
        ));
        let bridge = RenderBridge::detached();
        assert!(bridge.graticule());
        bridge.publish_graticule(false);
        assert!(!bridge.graticule());
    }

    #[test]
    fn scaled_size_rounds_and_clamps() {
        assert_eq!(scaled_size((1920, 1080), 100), (1920, 1080));
        assert_eq!(scaled_size((1920, 1080), 50), (960, 540));
        assert_eq!(scaled_size((1281, 801), 50), (641, 401));
        assert_eq!(scaled_size((1920, 1080), 10), (960, 540));
        assert_eq!(scaled_size((1, 1), 50), (1, 1));
    }
}

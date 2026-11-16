//! Continuous zoom across four presentation levels: surface, globe, star system and star map.
//!
//! The camera keeps one log-distance value `l` and is one perspective camera at every distance
//! ([`ZoomCamera::one_view`]): near the planet the planet camera, farther out the space camera
//! that continues it exactly. The three frames (the planet frame, the system frame and the
//! star-map frame) only nest the coordinates: the same camera is expressed in each of them and
//! every frame is drawn into the same picture, so nothing is swapped or cross-faded; what
//! changes with the distance is only what is drawn ([`Fades`]). The levels name stretches of
//! `l` for the HUD, picking and the age's caps. Pure maths: no window, no GPU.
//!
//! All three frames are 2.5D grounds under one orbit-style camera: a focus point, a heading, an
//! elevation that rises with the distance (and that the player may tilt) and the distance
//! itself. Beyond the planet camera's range the space camera takes the planet camera's exact
//! pose (the planet frame carried into the system frame), then eases to the plane's own pose as
//! it moves out; the star map shares the system's heading and elevation. Every change of place
//! is a flight of short legs, interrupted by input.

use std::sync::Arc;

use std::collections::VecDeque;

pub use crate::layers::Fades;
use crate::layers::{FrameView, OneView, PlanetPlace, SystemPlace};
use crate::orbit::{MIN_DISTANCE, OrbitCamera};
use crate::pick::View;
use crate::sun::PlanetFrame;
use crate::universe::{
    BodyViewKind, HOME_PLANET_RADIUS, MAP_RADIUS, PlacedBody, SYSTEM_RADIUS, UniverseScene,
};

/// Half the width of a level's fade, in log-distance units.
pub const FADE_HALF: f32 = 0.3;
/// Orbit distance in tiles below which the view counts as street level.
pub const SURFACE_DISTANCE: f32 = 24.0;
/// System camera distance at which the whole system fits the view, in system units.
pub const SYSTEM_MAX_DISTANCE: f32 = 48_000.0;
/// Star-map camera distance at the hand-over from the system frame, in map units.
pub const MAP_MIN_DISTANCE: f32 = 5000.0;
/// Star-map camera distance at which the whole map fits the view, in map units.
pub const MAP_MAX_DISTANCE: f32 = 230_000.0;
/// Elevation of the space camera above the plane at the planet's hand-over, in degrees.
pub const NEAR_ELEVATION: f32 = 38.0;
/// Elevation of the space camera at the system-to-map hand-over, in degrees.
pub const MID_ELEVATION: f32 = 52.0;
/// Elevation of the space camera over the whole star map, in degrees.
pub const FAR_ELEVATION: f32 = 70.0;
/// Log-distance over which the view eases from the planet camera's pose to the plane's pose.
pub const HANDOVER_SPAN: f32 = 1.0;
/// Log-distance over which a pan away from an anchored body fades in above the hand-over.
const PAN_FADE_SPAN: f32 = 0.6;
/// Log-distance change per wheel step near the ground (a factor of 1 / 0.88).
const WHEEL_LOG: f32 = 0.127_833_37;
/// Log-distance change per wheel step in space (a factor of 1.5), where the distances are
/// larger; the step grows smoothly from the ground's as the planet camera nears its limit.
const SPACE_WHEEL_LOG: f32 = 0.405_465_1;
/// Log-distance below the planet camera's limit at which the step starts to grow.
const SPACE_WHEEL_RAMP: f32 = 1.0;
/// Radians of elevation per pixel of Shift + middle-button drag up or down in space.
const TILT_RATE: f32 = 0.004;
/// Lowest and highest elevation of the space camera above the plane, in degrees.
const MIN_ELEVATION: f32 = 10.0;
const MAX_ELEVATION: f32 = 89.0;
/// Per second rate at which flights and glides settle.
const FLY_RATE: f32 = 3.5;
/// Fastest change of the log-distance in a flight, per second (a factor of about 20).
const MAX_LOG_RATE: f32 = 3.0;
/// Pan speed in space as a fraction of the camera distance per second.
const SPACE_PAN_RATE: f32 = 0.5;
/// Pointer distance from the canvas edge, in pixels, that starts edge panning in space.
const EDGE_PIXELS: f32 = 8.0;
/// Radians of heading per pixel of Shift + middle-button drag, as on the planet.
const ROTATE_RATE: f32 = 0.005;
/// Seconds between two presses that make a double click.
const DOUBLE_CLICK_MS: u64 = 450;
/// Pixels between two presses that make a double click.
const DOUBLE_CLICK_PIXELS: f32 = 8.0;
/// Smallest pick radius of a body or system on screen, in pixels.
const PICK_PIXELS: f32 = 14.0;
/// Camera distance of a focus in a system, in system units.
pub const SYSTEM_FOCUS_DISTANCE: f32 = 4500.0;
/// Camera distance of a focus on the star map, in map units.
pub const MAP_FOCUS_DISTANCE: f32 = 30_000.0;
/// Planet-camera distance of a focus on the surface or in orbit, in tiles.
pub const PLANET_FOCUS_DISTANCE: f32 = 70.0;

/// What the camera shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Street level: the planet frame, close to the ground.
    Surface,
    /// The whole planet.
    Globe,
    /// A star with its planets and resource bodies.
    System,
    /// The systems and the routes between them.
    StarMap,
}

impl Level {
    /// All levels from the closest to the farthest.
    pub const ALL: [Level; 4] = [Level::Surface, Level::Globe, Level::System, Level::StarMap];

    /// Position in [`Level::ALL`].
    pub fn index(self) -> usize {
        self as usize
    }

    /// Parses the name used on the command line: `surface`, `globe`, `system` or `starmap`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "surface" => Some(Self::Surface),
            "globe" => Some(Self::Globe),
            "system" => Some(Self::System),
            "starmap" | "star-map" | "map" => Some(Self::StarMap),
            _ => None,
        }
    }
}

use crate::layers::smoothstep;

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|c| c * s)
}

/// The log-distance scale of one universe: where the frames hand over.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomScale {
    /// Radius of the home planet in tiles.
    pub planet_radius: f32,
    /// Largest orbit distance of the planet camera in tiles.
    pub globe_max: f32,
    /// `l` of the globe-to-system hand-over.
    pub l_globe_system: f32,
    /// `l` of the system-to-map hand-over.
    pub l_system_map: f32,
    /// `l` at which the whole star map fits.
    pub l_end: f32,
    /// System camera distance at the globe-to-system hand-over.
    pub system_min: f32,
}

impl ZoomScale {
    /// The scale for a home planet of `planet_radius` tiles whose planet camera moves out to
    /// `globe_max` tiles. At equal `l` the planet looks equally large in the planet frame and in
    /// the system frame, where it is drawn with [`HOME_PLANET_RADIUS`].
    pub fn new(planet_radius: f32, globe_max: f32) -> Self {
        let l_globe_system = globe_max.max(MIN_DISTANCE).ln() + FADE_HALF;
        let system_min = HOME_PLANET_RADIUS / planet_radius.max(1.0) * l_globe_system.exp();
        let l_system_map = l_globe_system + (SYSTEM_MAX_DISTANCE / system_min).ln();
        let l_end = l_system_map + (MAP_MAX_DISTANCE / MAP_MIN_DISTANCE).ln();
        Self {
            planet_radius,
            globe_max,
            l_globe_system,
            l_system_map,
            l_end,
            system_min,
        }
    }

    /// Smallest `l`: the closest planet camera.
    pub fn l_min(&self) -> f32 {
        MIN_DISTANCE.ln()
    }

    /// Largest `l` a level allows: the zoom is capped so that the level is shown alone.
    pub fn l_cap(&self, level: Level) -> f32 {
        match level {
            Level::Surface | Level::Globe => self.globe_max.max(MIN_DISTANCE).ln(),
            Level::System => self.l_system_map - FADE_HALF,
            Level::StarMap => self.l_end,
        }
    }

    /// The level shown at `l`; monotonic in `l`.
    pub fn level_of(&self, l: f32) -> Level {
        if l < SURFACE_DISTANCE.ln() {
            Level::Surface
        } else if l < self.l_globe_system {
            Level::Globe
        } else if l < self.l_system_map {
            Level::System
        } else {
            Level::StarMap
        }
    }

    /// How much of the star map's own detail (routes, names, ship marks) shows at `l`: none up
    /// to the whole system, all of it three wheel steps past the star map's threshold.
    pub fn map_detail(&self, l: f32) -> f32 {
        smoothstep((l - (self.l_system_map - FADE_HALF)) / (4.0 * FADE_HALF))
    }

    /// Camera distance in the planet frame, in tiles.
    pub fn globe_distance(&self, l: f32) -> f32 {
        l.exp()
    }

    /// Camera distance in the system frame, in system units.
    pub fn system_distance(&self, l: f32) -> f32 {
        self.system_min * (l - self.l_globe_system).exp()
    }

    /// Camera distance in the star-map frame, in map units.
    pub fn map_distance(&self, l: f32) -> f32 {
        MAP_MIN_DISTANCE * (l - self.l_system_map).exp()
    }

    /// System units per tile of the home planet.
    pub fn system_per_tile(&self) -> f32 {
        HOME_PLANET_RADIUS / self.planet_radius.max(1.0)
    }

    /// Map units per system unit at the hand-over between the two frames.
    pub fn map_per_system(&self) -> f32 {
        MAP_MIN_DISTANCE / SYSTEM_MAX_DISTANCE
    }
}

/// Where the planet frame's point `p` lies in the system frame, for a planet centred at `planet`.
pub fn globe_to_system(scale: &ZoomScale, planet: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    add3(planet, scale3(p, scale.system_per_tile()))
}

/// Where the system frame's point `p` lies in the star-map frame, for a system centred at `system`.
pub fn system_to_map(scale: &ZoomScale, system: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    add3(system, scale3(p, scale.map_per_system()))
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length3(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

fn unit3(v: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot3(v, v).sqrt();
    (length > 1e-6).then(|| scale3(v, 1.0 / length))
}

fn rotate_about(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    let k = cross3(axis, v);
    let along = dot3(axis, v) * (1.0 - cos);
    [0, 1, 2].map(|i| v[i] * cos + k[i] * sin + axis[i] * along)
}

const UP: [f32; 3] = [0.0, 1.0, 0.0];

/// Unit direction from the focus to the eye for a heading `yaw` (radians; 0 puts the eye on
/// `+z`, looking toward `-z`) and an elevation above the plane (radians).
pub fn eye_direction(yaw: f32, elevation: f32) -> [f32; 3] {
    let (sin_e, cos_e) = elevation.sin_cos();
    let (sin_y, cos_y) = yaw.sin_cos();
    [cos_e * sin_y, sin_e, cos_e * cos_y]
}

/// The up vector of a camera looking along `-eye` without roll: the plane's normal as seen on
/// screen. Looking straight down it is the direction the heading `yaw` faces.
fn level_up(eye: [f32; 3], yaw: f32) -> [f32; 3] {
    unit3(sub3(UP, scale3(eye, dot3(UP, eye)))).unwrap_or_else(|| {
        let (sin, cos) = yaw.sin_cos();
        [-sin, 0.0, -cos]
    })
}

/// The heading of an eye direction: the yaw of its part along the plane.
fn yaw_of(eye: [f32; 3]) -> Option<f32> {
    (eye[0].hypot(eye[2]) > 0.05).then(|| eye[0].atan2(eye[2]))
}

/// Signed angle about `axis` that turns `from` into `to` (both perpendicular to `axis`).
fn angle_about(axis: [f32; 3], from: [f32; 3], to: [f32; 3]) -> f32 {
    dot3(axis, cross3(from, to)).atan2(dot3(from, to))
}

/// Where a camera looks from: the unit direction from the focus to the eye and the view's up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Unit direction from the focus to the eye.
    pub eye: [f32; 3],
    /// Unit up vector, perpendicular to `eye`.
    pub up: [f32; 3],
}

/// A camera placed by eye, target and up vector, with its clip planes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpaceView {
    /// Eye position.
    pub eye: [f32; 3],
    /// Point looked at.
    pub target: [f32; 3],
    /// Up vector.
    pub up: [f32; 3],
    /// Near clip distance.
    pub near: f32,
    /// Far clip distance.
    pub far: f32,
}

impl SpaceView {
    /// The pick view of this camera on a canvas of `size` pixels.
    pub fn view(&self, size: (f32, f32), centre_y: f32) -> View {
        View::looking(self.eye, self.target, self.up, size, centre_y)
    }
}

fn space_view(target: [f32; 3], distance: f32, pose: Pose, extent: f32) -> SpaceView {
    SpaceView {
        eye: add3(target, scale3(pose.eye, distance)),
        target,
        up: pose.up,
        near: (distance * 0.05).max(1.0),
        far: distance + extent,
    }
}

/// Where the ray through pixel `(x, y)` of `view` meets the plane `y = 0`.
fn plane_hit(view: &View, x: f32, y: f32) -> Option<[f32; 3]> {
    plane_hit_at(view, x, y, 0.0)
}

/// Where the ray through pixel `(x, y)` of `view` meets the horizontal plane `y = height`.
fn plane_hit_at(view: &View, x: f32, y: f32, height: f32) -> Option<[f32; 3]> {
    let dir = view.ray(x, y);
    let eye = view.eye();
    if dir[1].abs() < 1e-5 {
        return None;
    }
    let t = (height - eye[1]) / dir[1];
    (t > 0.0).then(|| [eye[0] + dir[0] * t, height, eye[2] + dir[2] * t])
}

/// Where the ray through pixel `(x, y)` of `view` first meets a sphere of `radius` about the
/// origin.
fn sphere_hit(view: &View, x: f32, y: f32, radius: f32) -> Option<[f32; 3]> {
    let dir = view.ray(x, y);
    let eye = view.eye();
    let b = dot3(eye, dir);
    let c = dot3(eye, eye) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t > 0.0).then(|| add3(eye, scale3(dir, t)))
}

/// What a double click can fly to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The body at this index of [`UniverseScene::bodies`].
    Body(usize),
    /// The system at this index of [`UniverseScene::systems`].
    System(usize),
}

/// A place the camera can be asked to fly to: where a ship is, at the level it is seen at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FocusPlace {
    /// A direction from the centre of the loaded planet, in the planet frame (a ship in orbit or
    /// a unit on the surface).
    Globe([f32; 3]),
    /// A point of the plane of system `system` (an id), in system units.
    System {
        /// Id of the system.
        system: u32,
        /// Point in that system's frame.
        at: [f32; 3],
    },
    /// A point of the star map, in map units (a ship between systems).
    Map([f32; 3]),
    /// The habitable planet at this index of [`UniverseScene::bodies`], whose globe is not
    /// loaded: a ship in its orbit is seen on it, so the camera flies down to it.
    Planet(usize),
}

/// Why the camera cannot fly to a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusRefused {
    /// The place lies beyond the level the player's age allows.
    BeyondReach,
    /// The place names a planet or system the camera does not know.
    Unknown,
}

/// Index of the marker nearest to `cursor` whose pick radius contains it; markers are
/// `(x, y, radius)` in pixels, and a nearer marker wins when several contain the cursor.
pub fn nearest_marker(markers: &[(f32, f32, f32)], cursor: (f32, f32)) -> Option<usize> {
    markers
        .iter()
        .enumerate()
        .map(|(i, &(x, y, r))| (i, (x - cursor.0).hypot(y - cursor.1), r.max(PICK_PIXELS)))
        .filter(|&(_, d, r)| d <= r)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _, _)| i)
}

/// Tracks presses to find double clicks.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClickTracker {
    last: Option<(u64, f32, f32)>,
}

impl ClickTracker {
    /// Records a press at `now_ms` and returns whether it completes a double click.
    pub fn press(&mut self, now_ms: u64, x: f32, y: f32) -> bool {
        let double = self.last.is_some_and(|(t, lx, ly)| {
            now_ms.saturating_sub(t) <= DOUBLE_CLICK_MS
                && (x - lx).hypot(y - ly) <= DOUBLE_CLICK_PIXELS
        });
        self.last = if double { None } else { Some((now_ms, x, y)) };
        double
    }
}

/// The home planet: the only one with a generated surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Home {
    /// Id of the home system.
    pub system: u32,
    /// Id of the home planet.
    pub body: u32,
    /// Unit direction from the planet centre to the first start.
    pub dir: [f32; 3],
}

/// One step of a camera flight.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Leg {
    /// Fly the log-distance to `to` (only outward when `out_only`) and wait until it is there.
    Zoom { to: f32, out_only: bool },
    /// Anchor on a system (an index) with the focus at a point of its plane, gliding there.
    System { index: usize, at: [f32; 3] },
    /// Anchor on a body (an index), gliding there.
    Body(usize),
    /// Glide the focus to a point of the anchored system's plane.
    Point([f32; 3]),
    /// Let go of the anchored body, keeping the picture.
    Free,
    /// Turn the planet camera toward a direction, alongside the following legs.
    Look([f32; 3]),
    /// Ask for the globe of a planet (an id).
    Surface(u32),
}

/// How a pointer drag over space moves the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    /// Middle button: the plane follows the pointer.
    Pan { x: f32, y: f32 },
    /// Shift + middle button: the heading turns, as on the planet, and up or down the camera
    /// tilts about the focus.
    Rotate { x: f32, y: f32 },
}

/// The continuous-zoom camera above the planet camera.
#[derive(Clone, Debug)]
pub struct ZoomCamera {
    scene: Arc<UniverseScene>,
    scale: ZoomScale,
    map_positions: Vec<[f32; 3]>,
    /// The system the view is anchored to (an index), with its bodies placed.
    system: usize,
    placed: Vec<PlacedBody>,
    /// The body the view is anchored to (an index into the scene's bodies); none for the star.
    body: Option<usize>,
    home_body: Option<usize>,
    /// The planet whose globe is loaded: the only one that can be entered.
    surface_body: Option<usize>,
    /// A planet whose globe the camera waits for.
    want_surface: Option<u32>,
    home_dir: [f32; 3],
    l: f32,
    cap: Level,
    /// Largest planet-camera distance the age's view range allows, when it limits the globe.
    globe_limit: Option<f32>,
    goal: Option<f32>,
    /// Focus offset from the anchor in the anchored system's frame (system units).
    pan: [f32; 3],
    /// A gliding offset of the focus that settles to zero (system units).
    offset: [f32; 3],
    /// Heading of the space camera: the yaw of the direction from the focus to the eye.
    yaw: f32,
    /// The player's tilt of the space camera, added to the elevation of the level (radians).
    tilt: f32,
    /// A turn about the plane's normal still to be applied to the planet camera.
    spin: f32,
    held: [bool; 4],
    pointer: Option<(f32, f32)>,
    viewport: (f32, f32),
    drag: Option<Drag>,
    clicks: ClickTracker,
    legs: VecDeque<Leg>,
    leg_started: bool,
    look: Option<[f32; 3]>,
    seconds: f32,
    /// A pose the planet camera takes over once the newly loaded planet's frame is known.
    align_pending: Option<Pose>,
    /// Turn of the loaded planet about its axis from the planet frame into the system frame.
    planet_frame: PlanetFrame,
    /// The systems whose bodies the player knows (ids, ascending); `None` knows them all.
    known: Option<Vec<u32>>,
}

impl ZoomCamera {
    /// A camera over the home planet at the largest planet-camera distance. `None` home
    /// anchors to the first system.
    pub fn new(
        scene: Arc<UniverseScene>,
        scale: ZoomScale,
        home: Option<Home>,
        cap: Level,
    ) -> Self {
        let home_body = home.and_then(|h| scene.body_index(h.body));
        let system = home.and_then(|h| scene.system_index(h.system)).unwrap_or(0);
        // The planet frame and the system frame show the home planet equally large: its drawn
        // radius in the system frame decides the scale between them.
        let scale = home_body.map_or(scale, |index| {
            let drawn = crate::universe::body_radius(&scene.bodies[index]);
            ZoomScale::new(
                scale.planet_radius * HOME_PLANET_RADIUS / drawn.max(1.0),
                scale.globe_max,
            )
        });
        let map_positions = scene.place_systems();
        let placed = scene
            .systems
            .get(system)
            .map(|s| scene.place_system(s.id))
            .unwrap_or_default();
        let mut camera = Self {
            scene,
            scale,
            map_positions,
            system,
            placed,
            body: home_body,
            home_body,
            surface_body: home_body,
            want_surface: None,
            home_dir: home.map_or([0.0, 0.0, 1.0], |h| h.dir),
            l: scale.l_cap(Level::Globe),
            cap,
            globe_limit: None,
            goal: None,
            pan: [0.0; 3],
            offset: [0.0; 3],
            yaw: 0.0,
            tilt: 0.0,
            spin: 0.0,
            held: [false; 4],
            pointer: None,
            viewport: (0.0, 0.0),
            drag: None,
            clicks: ClickTracker::default(),
            legs: VecDeque::new(),
            leg_started: false,
            look: None,
            seconds: 0.0,
            align_pending: None,
            planet_frame: PlanetFrame::default(),
            known: None,
        };
        camera.l = camera.l.min(camera.l_cap());
        camera
    }

    /// The scale of the log-distance.
    pub fn scale(&self) -> &ZoomScale {
        &self.scale
    }

    /// The universe being shown.
    pub fn scene(&self) -> &UniverseScene {
        &self.scene
    }

    /// The universe being shown, shared.
    pub fn scene_arc(&self) -> Arc<UniverseScene> {
        Arc::clone(&self.scene)
    }

    /// Star-map positions of the systems, indexed like the scene's systems.
    pub fn map_positions(&self) -> &[[f32; 3]] {
        &self.map_positions
    }

    /// The log-distance of the camera.
    pub fn l(&self) -> f32 {
        self.l
    }

    /// Sets the log-distance, within the limits; a flight under way stops.
    pub fn set_l(&mut self, l: f32) {
        self.interrupt();
        self.l = l.clamp(self.l_min(), self.l_cap());
    }

    /// The level shown now.
    pub fn level(&self) -> Level {
        self.scale.level_of(self.l)
    }

    /// The farthest level the zoom may reach.
    pub fn cap(&self) -> Level {
        self.cap
    }

    /// Sets the farthest level; a camera beyond it is brought back to it.
    pub fn set_cap(&mut self, cap: Level) {
        self.cap = cap;
        self.l = self.l.min(self.l_cap());
        self.goal = self.goal.map(|g| g.min(self.l_cap()));
    }

    /// Limits the planet camera to eye distance `distance` (tiles), from the age's view range;
    /// `None` leaves the globe cap. Only the globe level is limited: space levels keep their caps.
    pub fn set_globe_limit(&mut self, distance: Option<f32>) {
        self.globe_limit = distance;
        self.l = self.l.min(self.l_cap());
        self.goal = self.goal.map(|g| g.min(self.l_cap()));
    }

    /// Largest `l` the cap allows.
    pub fn l_cap(&self) -> f32 {
        let cap = self.scale.l_cap(self.cap);
        match (self.cap, self.globe_limit) {
            (Level::Surface | Level::Globe, Some(d)) => cap.min(d.max(MIN_DISTANCE).ln()),
            _ => cap,
        }
    }

    /// Smallest `l`: a planet without a generated surface cannot be entered.
    pub fn l_min(&self) -> f32 {
        if self.anchored_at_surface() {
            self.scale.l_min()
        } else {
            self.space_floor()
        }
    }

    /// The closest the camera comes to a body that cannot be entered.
    fn space_floor(&self) -> f32 {
        self.scale.l_globe_system + FADE_HALF
    }

    /// How much of each layer's detail shows now.
    pub fn fades(&self) -> Fades {
        let s = &self.scale;
        let space = if self.anchored_at_surface() {
            let from = s.globe_max.max(MIN_DISTANCE).ln();
            let to = s.l_globe_system + FADE_HALF + HANDOVER_SPAN;
            smoothstep((self.l - from) / (to - from).max(1e-3))
        } else {
            1.0
        };
        Fades {
            space,
            map: s.map_detail(self.l),
        }
    }

    /// The one camera now, in every frame it draws in: the planet camera while it decides the
    /// picture, else the space camera, which continues it exactly.
    pub fn one_view(&self, orbit: &OrbitCamera) -> OneView {
        let place = self
            .surface_body
            .and_then(|b| self.placed.iter().find(|p| p.body == b))
            .map(|p| PlanetPlace {
                centre: p.pos,
                per_tile: self.scale.system_per_tile(),
                frame: self.planet_frame,
            });
        let planet_camera = (self.anchored_at_surface() && self.handover_weight() <= 0.0)
            .then(|| self.draw_orbit(orbit));
        let (system, planet, planet_camera) = match (planet_camera, place) {
            (Some(drawn), Some(place)) => {
                let planet = FrameView {
                    eye: drawn.position(),
                    target: drawn.target(),
                    up: drawn.view_up(),
                };
                (place.to_system(&planet), Some(planet), Some(drawn))
            }
            _ => {
                let view = self.system_view(orbit);
                let system = FrameView {
                    eye: view.eye,
                    target: view.target,
                    up: view.up,
                };
                (system, place.map(|p| p.to_planet(&system)), None)
            }
        };
        let system_place = SystemPlace {
            here: self.here(),
            per_unit: self.k(),
        };
        OneView {
            map: system_place.to_map(&system),
            system,
            planet,
            planet_camera,
            place,
            system_place,
            fades: self.fades(),
        }
    }

    /// The system the view is anchored to, as an index into the scene's systems.
    pub fn system(&self) -> usize {
        self.system
    }

    /// The bodies of the anchored system, placed in the system frame.
    pub fn placed(&self) -> &[PlacedBody] {
        &self.placed
    }

    /// The planet the view is anchored to.
    pub fn anchor_body(&self) -> Option<usize> {
        self.body
    }

    /// Whether the anchored planet is the one whose globe is loaded, which can be entered.
    pub fn anchored_at_surface(&self) -> bool {
        self.body.is_some() && self.body == self.surface_body
    }

    /// The planet whose globe is loaded, as an index into the scene's bodies.
    pub fn surface_body(&self) -> Option<usize> {
        self.surface_body
    }

    /// The id of a planet whose globe the camera asked for and has not got yet.
    pub fn want_surface(&self) -> Option<u32> {
        self.want_surface
    }

    /// Sets the systems whose bodies the player knows; the bodies of others cannot be picked.
    pub fn set_known(&mut self, known: Option<&[u32]>) {
        if self.known.as_deref() != known {
            self.known = known.map(<[u32]>::to_vec);
        }
    }

    /// Whether the player knows the bodies of the anchored system.
    pub fn knows_system(&self) -> bool {
        match (&self.known, self.system_id()) {
            (Some(known), Some(id)) => known.binary_search(&id).is_ok(),
            _ => true,
        }
    }

    /// The id of the system the view is anchored to.
    pub fn system_id(&self) -> Option<u32> {
        self.scene.systems.get(self.system).map(|s| s.id)
    }

    /// The heading of the space camera in radians, clockwise from the plane's `-z` seen from
    /// above, like the planet camera's heading from north.
    pub fn heading(&self) -> f32 {
        (-self.yaw).rem_euclid(std::f32::consts::TAU)
    }

    /// Map units per system unit at the hand-over.
    fn k(&self) -> f32 {
        self.scale.map_per_system()
    }

    fn here(&self) -> [f32; 3] {
        self.map_positions
            .get(self.system)
            .copied()
            .unwrap_or([0.0; 3])
    }

    fn anchor_pos(&self) -> [f32; 3] {
        self.body
            .and_then(|b| self.placed.iter().find(|p| p.body == b))
            .map_or([0.0; 3], |p| p.pos)
    }

    /// The globe of planet `body` is loaded: `globe_radius` tiles and the planet camera's
    /// farthest distance `max_distance` describe it, `dir` is where the planet camera looks.
    /// The log-distance scale is rebuilt for the planet's size; a camera anchored on the planet
    /// takes it over from the pose it shows now and flies in.
    pub fn surface_ready(
        &mut self,
        body: u32,
        globe_radius: f32,
        max_distance: f32,
        dir: [f32; 3],
        orbit: &mut OrbitCamera,
    ) {
        let Some(index) = self.scene.body_index(body) else {
            return;
        };
        let shown = self.pose(orbit);
        let drawn = crate::universe::body_radius(&self.scene.bodies[index]);
        let anchored = self.body == Some(index);
        // The system camera keeps its distance: the picture does not move when the scale between
        // the frames changes to the new planet's.
        let distance = self.scale.system_distance(self.l);
        let in_space = self.l >= self.space_floor();
        self.scale = ZoomScale::new(
            globe_radius * HOME_PLANET_RADIUS / drawn.max(1.0),
            max_distance,
        );
        self.surface_body = Some(index);
        if self.want_surface == Some(body) {
            self.want_surface = None;
        }
        if anchored {
            // The planet camera starts from the pose the system view shows (once the new planet's
            // frame is known), so the flight down continues without a turn.
            self.l = self.keep_distance(distance);
            self.align_pending = Some(shown);
            self.interrupt();
            if Some(index) == self.home_body {
                self.legs.push_back(Leg::Look(dir));
            }
            let to = if Some(index) == self.home_body {
                (2.0 * globe_radius).max(MIN_DISTANCE).ln()
            } else {
                max_distance.max(MIN_DISTANCE).ln() - 0.2
            };
            self.legs.push_back(Leg::Zoom {
                to,
                out_only: false,
            });
        } else {
            orbit.look_at(dir);
            if in_space {
                self.l = self.keep_distance(distance);
            }
        }
        self.l = self.l.clamp(self.l_min(), self.l_cap());
    }

    /// The log-distance at which the system camera is `distance` away, above the floor.
    fn keep_distance(&self, distance: f32) -> f32 {
        (self.scale.l_globe_system + (distance / self.scale.system_min).ln())
            .max(self.space_floor())
    }

    /// Stops a flight under way: any input of the player does.
    pub fn interrupt(&mut self) {
        self.goal = None;
        self.legs.clear();
        self.leg_started = false;
        self.look = None;
    }

    /// Log-distance per wheel step at `l`: the ground's step up to a little below the planet
    /// camera's limit, growing smoothly to the space step by the planet's hand-over.
    pub fn wheel_log(&self, l: f32) -> f32 {
        let s = &self.scale;
        let to = s.l_globe_system + FADE_HALF;
        let from = s.globe_max.max(MIN_DISTANCE).ln() - SPACE_WHEEL_RAMP;
        let t = smoothstep((l - from) / (to - from).max(1e-3));
        WHEEL_LOG + (SPACE_WHEEL_LOG - WHEEL_LOG) * t
    }

    /// Zooms in for positive `steps`, out for negative ones, within the limits, about the centre.
    pub fn wheel(&mut self, steps: f32) {
        self.interrupt();
        let sign = -steps.signum();
        let mut left = steps.abs();
        let mut l = self.l;
        // Step by step, each sized at its middle, so a step out and a step in nearly cancel.
        while left > 1e-6 {
            let part = left.min(1.0);
            let guess = self.wheel_log(l);
            l += sign * part * self.wheel_log(l + sign * part * guess * 0.5);
            left -= part;
        }
        self.l = l.clamp(self.l_min(), self.l_cap());
    }

    /// The view of the frame the level shows: the system or the star map.
    pub fn space_view_at(&self, orbit: &OrbitCamera, canvas: (f32, f32), centre_y: f32) -> View {
        if self.level() >= Level::StarMap {
            self.map_view(orbit).view(canvas, centre_y)
        } else {
            self.system_view(orbit).view(canvas, centre_y)
        }
    }

    /// The point of the plane under `cursor` in system units of the anchored system, through
    /// `view` of the system frame or, for `map`, of the star-map frame.
    fn plane_point(&self, view: &View, cursor: (f32, f32), map: bool) -> Option<[f32; 3]> {
        Some(if map {
            // On the star map, the plane through the anchored system, wherever it lies.
            let here = self.here();
            let hit = plane_hit_at(view, cursor.0, cursor.1, here[1])?;
            scale3(sub3(hit, here), 1.0 / self.k())
        } else {
            plane_hit(view, cursor.0, cursor.1)?
        })
    }

    /// Whether zooming keeps the point under the cursor in place: on the star map, and in a
    /// system once the focus floats free of any body.
    fn zooms_to_cursor(&self) -> bool {
        match self.level() {
            Level::StarMap => true,
            Level::System => !self.pans_planet() && self.pan_weight() > 0.2,
            Level::Surface | Level::Globe => false,
        }
    }

    /// The wheel over the canvas at `cursor`: zooms toward the point under it. Zooming in over a
    /// body anchors the view on it (a planet with a globe is entered, another one is asked for);
    /// zooming in past the star map enters the system nearest the focus.
    pub fn wheel_at(
        &mut self,
        steps: f32,
        cursor: Option<(f32, f32)>,
        canvas: (f32, f32),
        centre_y: f32,
        orbit: &mut OrbitCamera,
    ) {
        self.interrupt();
        let level = self.level();
        if steps > 0.0
            && level == Level::System
            && !self.pans_planet()
            && let Some(cursor) = cursor
        {
            let view = self.system_view(orbit).view(canvas, centre_y);
            self.choose_anchor(cursor, &view, orbit);
        }
        // On the planet the point of the ground under the cursor stays under it.
        let ground = if self.pans_planet() && self.anchored_at_surface() {
            cursor.and_then(|c| {
                let view = View::from_orbit(&self.draw_orbit(orbit), canvas, centre_y);
                Some((c, sphere_hit(&view, c.0, c.1, orbit.radius)?))
            })
        } else {
            None
        };
        let map = level >= Level::StarMap;
        let grip = if self.zooms_to_cursor() {
            cursor.and_then(|c| {
                let view = self.space_view_at(orbit, canvas, centre_y);
                Some((c, self.plane_point(&view, c, map)?))
            })
        } else {
            None
        };
        let threshold = self.scale.l_system_map + FADE_HALF;
        let before = self.l;
        self.wheel(steps);
        if let Some((cursor, held)) = ground
            && self.level() <= Level::Globe
        {
            orbit.distance = self
                .scale
                .globe_distance(self.l)
                .clamp(MIN_DISTANCE, orbit.max_distance);
            let view = View::from_orbit(&self.draw_orbit(orbit), canvas, centre_y);
            if let Some(now) = sphere_hit(&view, cursor.0, cursor.1, orbit.radius) {
                orbit.drag_surface(now, held);
            }
        }
        if let Some((cursor, held)) = grip {
            let view = if map {
                self.map_view(orbit).view(canvas, centre_y)
            } else {
                self.system_view(orbit).view(canvas, centre_y)
            };
            if let Some(now) = self.plane_point(&view, cursor, map) {
                let weight = self.pan_weight().max(0.2);
                self.pan = add3(self.pan, scale3(sub3(held, now), 1.0 / weight));
                self.clamp_pan_in(map);
            }
        }
        if before >= threshold && self.l < threshold {
            self.enter_nearest_system(orbit);
        }
    }

    /// Zooming in over the plane of a system: the body under the cursor becomes the anchor;
    /// over empty space a free-floating focus replaces the anchor without moving the picture.
    fn choose_anchor(&mut self, cursor: (f32, f32), view: &View, orbit: &mut OrbitCamera) {
        let hit = match self.target_at(view, cursor.0, cursor.1) {
            Some(Target::Body(i)) if self.scene.bodies[i].kind != BodyViewKind::Star => Some(i),
            _ => None,
        };
        match hit {
            Some(index) if self.body != Some(index) => {
                self.anchor_keeping_picture(index, orbit);
                let body = &self.scene.bodies[index];
                if body.kind == BodyViewKind::Habitable && self.surface_body != Some(index) {
                    self.want_surface = Some(body.id);
                }
            }
            Some(_) => {}
            None => {
                // Far from the anchored body the focus floats free, so zooming in on empty space
                // does not slide back to the body.
                let far = self
                    .body
                    .and_then(|b| self.placed.iter().find(|p| p.body == b))
                    .zip(plane_hit(view, cursor.0, cursor.1))
                    .is_some_and(|(p, at)| {
                        (at[0] - p.pos[0]).hypot(at[2] - p.pos[2]) > (p.radius * 4.0).max(600.0)
                    });
                if far && self.handover_weight() >= 1.0 && self.pan_weight() >= 1.0 {
                    self.free_focus(orbit);
                }
            }
        }
    }

    /// Anchors on `body` without moving the picture: the pan away from it fades out as the
    /// camera comes down to it, so zooming in toward it ends centred on it.
    fn anchor_keeping_picture(&mut self, body: usize, orbit: &mut OrbitCamera) {
        let before = self.system_target(orbit);
        let shown = self.pose(orbit);
        let tracking = self.anchored_at_surface();
        self.body = Some(body);
        self.pan = [0.0; 3];
        if self.anchored_at_surface() && !tracking {
            self.align_orbit(orbit, shown);
        }
        let base = self.system_target(orbit);
        let weight = self.pan_weight();
        if weight > 0.05 {
            self.pan = scale3(sub3(before, base), 1.0 / weight);
        } else {
            self.offset = add3(self.offset, sub3(before, base));
        }
    }

    /// Lets go of the anchored body without moving the picture: the focus floats on the plane.
    fn free_focus(&mut self, orbit: &OrbitCamera) {
        if self.body.is_some() {
            let target = sub3(self.system_target(orbit), self.offset);
            self.body = None;
            self.pan = target;
        }
    }

    /// Re-anchors on the system nearest the star-map focus, keeping the picture.
    fn enter_nearest_system(&mut self, orbit: &OrbitCamera) {
        let focus = sub3(self.map_target(orbit), scale3(self.offset, self.k()));
        let nearest = self
            .map_positions
            .iter()
            .enumerate()
            .map(|(i, p)| (i, length3(sub3(*p, focus))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        if let Some(index) = nearest
            && index != self.system
        {
            let wanted = scale3(sub3(focus, self.map_positions[index]), 1.0 / self.k());
            let kept = scale3(self.offset, 1.0);
            self.set_system(index);
            self.pan = clamp_disc(wanted, SYSTEM_RADIUS * 2.0);
            // Whatever the disc does not hold glides in, in both frames alike.
            self.offset = add3(kept, sub3(wanted, self.pan));
        }
    }

    fn set_system(&mut self, index: usize) {
        self.system = index;
        self.placed = self
            .scene
            .systems
            .get(index)
            .map(|s| self.scene.place_system(s.id))
            .unwrap_or_default();
        self.body = None;
        self.pan = [0.0; 3];
        self.offset = [0.0; 3];
    }

    /// Makes `orbit` match the camera: its distance follows `l` up to the planet camera's limit.
    /// Also runs the flight's legs, turns and glides the planet camera, and keeps the space
    /// heading on the planet camera's while the planet frame shows.
    pub fn sync_orbit(&mut self, orbit: &mut OrbitCamera) {
        if let Some(pose) = self.align_pending.take()
            && self.anchored_at_surface()
        {
            self.align_orbit(orbit, pose);
        }
        self.run_legs(orbit);
        if let Some(goal) = self.look {
            let settle = 1.0 - (-FLY_RATE * self.seconds.max(0.0)).exp();
            let angle = dot3(orbit.dir, goal).clamp(-1.0, 1.0).acos();
            if angle < 0.002 {
                orbit.look_at(goal);
                self.look = None;
            } else if settle > 0.0 {
                let axis = unit3(cross3(orbit.dir, goal)).unwrap_or(UP);
                orbit.look_at(rotate_about(orbit.dir, axis, angle * settle));
            }
        }
        if self.anchored_at_surface() {
            if self.spin != 0.0 {
                orbit.turn_about(self.planet_frame.uncarry(UP), self.spin);
            }
            if self.l <= self.scale.l_globe_system + FADE_HALF
                && let Some(yaw) = yaw_of(self.planet_to_system(orbit.dir))
            {
                self.yaw = yaw;
            }
        }
        self.spin = 0.0;
        orbit.distance = self
            .scale
            .globe_distance(self.l)
            .clamp(MIN_DISTANCE, orbit.max_distance);
    }

    /// Takes over a change that the planet camera made itself (a camera reset): in the planet
    /// range the orbit distance is the truth.
    pub fn adopt_orbit(&mut self, orbit: &OrbitCamera) {
        if self.l <= orbit.max_distance.ln() + 1e-3 && self.goal.is_none() && self.legs.is_empty() {
            self.l = orbit.distance.ln().clamp(self.l_min(), self.l_cap());
        }
    }

    /// The planet camera as drawn: beyond its own limit the distance keeps growing, so the
    /// planet shrinks as the system frame takes over.
    pub fn draw_orbit(&self, orbit: &OrbitCamera) -> OrbitCamera {
        OrbitCamera {
            distance: self.scale.globe_distance(self.l).max(MIN_DISTANCE),
            ..*orbit
        }
    }

    /// Turns the loaded planet about its axis so the light that represents the sun, `toward`
    /// in the planet frame, comes from the system's star: the planet's day side faces the star.
    pub fn follow_sun(&mut self, toward: [f32; 3]) {
        let pos = self
            .surface_body
            .and_then(|b| self.placed.iter().find(|p| p.body == b))
            .map(|p| p.pos);
        let star = self
            .placed
            .iter()
            .find(|p| self.scene.bodies[p.body].kind == BodyViewKind::Star)
            .map_or([0.0; 3], |p| p.pos);
        self.planet_frame = pos.map_or_else(PlanetFrame::default, |pos| {
            PlanetFrame::aligned(toward, pos, star)
        });
    }

    /// Carries a vector from the loaded planet frame into the system frame.
    pub fn planet_to_system(&self, vector: [f32; 3]) -> [f32; 3] {
        self.planet_frame.carry(vector)
    }

    /// Records a pan key being held or released (`index`: forward, back, left, right).
    pub fn hold(&mut self, index: usize, pressed: bool) {
        if index < self.held.len() {
            if pressed {
                self.interrupt();
            }
            self.held[index] = pressed;
        }
    }

    /// The pointer over the canvas (`None` when it left), for edge panning.
    pub fn set_pointer(&mut self, pointer: Option<(f32, f32)>) {
        self.pointer = pointer;
    }

    /// The pointer over the canvas, as last seen.
    pub fn pointer(&self) -> Option<(f32, f32)> {
        self.pointer
    }

    /// The canvas size in pixels, for edge panning.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
    }

    /// Whether the planet camera should handle pan input: in the planet range, and in the
    /// hand-over band where the planet camera still decides the picture.
    pub fn pans_planet(&self) -> bool {
        self.level() <= Level::Globe
            || (self.anchored_at_surface() && self.l < self.scale.l_globe_system + FADE_HALF + 0.05)
    }

    /// How far the view has left the planet camera's pose: 0 in the hand-over band, 1 once the
    /// plane's own pose has taken over (always 1 away from the loaded planet).
    pub fn handover_weight(&self) -> f32 {
        if !self.anchored_at_surface() {
            return 1.0;
        }
        smoothstep((self.l - (self.scale.l_globe_system + FADE_HALF)) / HANDOVER_SPAN)
    }

    /// How much of the pan away from an anchored body shows: none at the body's hand-over.
    fn pan_weight(&self) -> f32 {
        if self.body.is_none() {
            return 1.0;
        }
        smoothstep((self.l - (self.scale.l_globe_system + FADE_HALF)) / PAN_FADE_SPAN)
    }

    /// Elevation of the plane's own pose above the plane at `l`, in radians: it rises with the
    /// distance, as the planet camera turns to look straight down.
    pub fn elevation_at(&self, l: f32) -> f32 {
        let s = &self.scale;
        let (near, mid, far) = (
            NEAR_ELEVATION.to_radians(),
            MID_ELEVATION.to_radians(),
            FAR_ELEVATION.to_radians(),
        );
        if l <= s.l_system_map {
            let span = (s.l_system_map - s.l_globe_system).max(1e-3);
            near + (mid - near) * smoothstep((l - s.l_globe_system) / span)
        } else {
            let span = (s.l_end - s.l_system_map).max(1e-3);
            mid + (far - mid) * smoothstep((l - s.l_system_map) / span)
        }
    }

    /// Elevation of the plane's own pose now: the level's, tilted by the player.
    fn elevation(&self) -> f32 {
        (self.elevation_at(self.l) + self.tilt)
            .clamp(MIN_ELEVATION.to_radians(), MAX_ELEVATION.to_radians())
    }

    fn plane_pose(&self) -> Pose {
        let eye = eye_direction(self.yaw, self.elevation());
        Pose {
            eye,
            up: level_up(eye, self.yaw),
        }
    }

    /// The planet camera's pose carried into the system frame (it looks straight down from
    /// beyond its own limit, so its up is the direction it faces).
    fn planet_pose(&self, orbit: &OrbitCamera) -> Pose {
        Pose {
            eye: self.planet_to_system(orbit.dir),
            up: self.planet_to_system(orbit.forward_tangent()),
        }
    }

    /// Where the space camera looks from now: the planet camera's pose in the hand-over band,
    /// easing to the plane's own pose (the heading kept, no roll) as it moves out.
    pub fn pose(&self, orbit: &OrbitCamera) -> Pose {
        let plane = self.plane_pose();
        let w = self.handover_weight();
        if w >= 1.0 {
            return plane;
        }
        let planet = self.planet_pose(orbit);
        let eye = unit3(lerp3(planet.eye, plane.eye, w)).unwrap_or(plane.eye);
        let roll = angle_about(planet.eye, level_up(planet.eye, self.yaw), planet.up);
        Pose {
            eye,
            up: rotate_about(level_up(eye, self.yaw), eye, roll * (1.0 - w)),
        }
    }

    /// Makes the planet camera show `pose`: it looks along the pose's eye direction, its up
    /// toward the pose's up.
    fn align_orbit(&self, orbit: &mut OrbitCamera, pose: Pose) {
        orbit.orient(
            self.planet_frame.uncarry(pose.eye),
            self.planet_frame.uncarry(pose.up),
        );
    }

    /// Where the system camera looks, in the system frame.
    pub fn system_target(&self, orbit: &OrbitCamera) -> [f32; 3] {
        let anchor = self.anchor_pos();
        let near = if self.anchored_at_surface() {
            let surface =
                globe_to_system(&self.scale, anchor, self.planet_to_system(orbit.target()));
            lerp3(surface, anchor, self.handover_weight())
        } else {
            anchor
        };
        add3(add3(near, scale3(self.pan, self.pan_weight())), self.offset)
    }

    /// Where the star-map camera looks, in the star-map frame: the system camera's focus carried
    /// onto the map.
    pub fn map_target(&self, orbit: &OrbitCamera) -> [f32; 3] {
        system_to_map(&self.scale, self.here(), self.system_target(orbit))
    }

    /// The system camera.
    pub fn system_view(&self, orbit: &OrbitCamera) -> SpaceView {
        let target = self.system_target(orbit);
        space_view(
            target,
            self.scale.system_distance(self.l),
            self.pose(orbit),
            SYSTEM_RADIUS * 3.0 + target[0].hypot(target[2]),
        )
    }

    /// The star-map camera: the system camera seen at the star map's scale.
    pub fn map_view(&self, orbit: &OrbitCamera) -> SpaceView {
        let target = self.map_target(orbit);
        space_view(
            target,
            self.scale.map_distance(self.l),
            self.pose(orbit),
            MAP_RADIUS * 3.0 + length3(target),
        )
    }

    /// The focus's horizontal axes: (right, forward) along the plane for the current heading.
    fn plane_axes(&self) -> ([f32; 3], [f32; 3]) {
        let (sin, cos) = self.yaw.sin_cos();
        ([cos, 0.0, -sin], [-sin, 0.0, -cos])
    }

    /// Moves the focus by `right` and `forward` system units across the plane, relative to the
    /// heading.
    pub fn pan_by(&mut self, right: f32, forward: f32) {
        let (r, f) = self.plane_axes();
        let weight = self.pan_weight().max(0.05);
        let step = add3(scale3(r, right), scale3(f, forward));
        self.pan = add3(self.pan, scale3(step, 1.0 / weight));
        self.clamp_pan();
    }

    /// Turns the space camera's heading by `delta` radians (clockwise seen from above), about the
    /// focus; the planet camera turns with it about the planet's axis.
    pub fn rotate_heading(&mut self, delta: f32) {
        self.yaw -= delta;
        self.spin -= delta;
    }

    /// Tilts the space camera about the focus by `delta` radians of elevation (positive looks
    /// more steeply down), within the elevations allowed.
    pub fn tilt_by(&mut self, delta: f32) {
        let base = self.elevation_at(self.l);
        let low = MIN_ELEVATION.to_radians() - base;
        let high = MAX_ELEVATION.to_radians() - base;
        self.tilt = (self.tilt + delta).clamp(low.min(0.0), high.max(0.0));
    }

    fn clamp_pan(&mut self) {
        self.clamp_pan_in(self.level() >= Level::StarMap);
    }

    /// Keeps the focus on the star map (`map`) or near the anchored system.
    fn clamp_pan_in(&mut self, map: bool) {
        let anchor = self.anchor_pos();
        if map {
            let (here, k) = (self.here(), self.k());
            let at = add3(here, scale3(add3(anchor, self.pan), k));
            let kept = clamp_disc(at, MAP_RADIUS * 1.2);
            self.pan = sub3(scale3(sub3(kept, here), 1.0 / k), anchor);
        } else {
            let kept = clamp_disc(add3(anchor, self.pan), SYSTEM_RADIUS * 2.0);
            self.pan = sub3(kept, anchor);
        }
    }

    /// Starts a pointer drag over space (middle button; with Shift it turns the heading).
    pub fn drag_start(&mut self, x: f32, y: f32, rotate: bool) {
        self.interrupt();
        self.drag = Some(if rotate {
            Drag::Rotate { x, y }
        } else {
            Drag::Pan { x, y }
        });
    }

    /// Whether a pointer drag over space is under way.
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Ends a pointer drag over space.
    pub fn drag_end(&mut self) {
        self.drag = None;
    }

    /// Moves a pointer drag to `(x, y)`: the grabbed point of the plane stays under the pointer,
    /// or the heading turns.
    pub fn drag_to(&mut self, x: f32, y: f32, view: &View) {
        match self.drag {
            Some(Drag::Pan { x: lx, y: ly }) => {
                let map = self.level() >= Level::StarMap;
                if let (Some(from), Some(to)) = (
                    self.plane_point(view, (lx, ly), map),
                    self.plane_point(view, (x, y), map),
                ) {
                    let weight = self.pan_weight().max(0.05);
                    self.pan = add3(self.pan, scale3(sub3(from, to), 1.0 / weight));
                    self.clamp_pan();
                }
                self.drag = Some(Drag::Pan { x, y });
            }
            Some(Drag::Rotate { x: lx, y: ly }) => {
                self.rotate_heading(-(x - lx) * ROTATE_RATE);
                self.tilt_by((y - ly) * TILT_RATE);
                self.drag = Some(Drag::Rotate { x, y });
            }
            None => {}
        }
    }

    /// Moves the camera for `seconds`: flights settle, glides fade, and held pan keys and the
    /// pointer at the canvas edge move the focus across the plane of the level shown.
    pub fn advance(&mut self, seconds: f32) {
        self.seconds = seconds;
        let settle = 1.0 - (-FLY_RATE * seconds).exp();
        if let Some(goal) = self.goal {
            let most = MAX_LOG_RATE * seconds;
            self.l += ((goal - self.l) * settle).clamp(-most, most);
            if (goal - self.l).abs() < 0.003 {
                self.l = goal;
                self.goal = None;
            }
        }
        self.offset = scale3(self.offset, 1.0 - settle);
        if self.pans_planet() || self.level() < Level::System {
            return;
        }
        let [forward_key, back_key, left_key, right_key] = self.held.map(f32::from);
        let (edge_right, edge_forward) = self.edge_direction();
        let right = (right_key - left_key + edge_right).clamp(-1.0, 1.0);
        let forward = (forward_key - back_key + edge_forward).clamp(-1.0, 1.0);
        if right == 0.0 && forward == 0.0 {
            return;
        }
        if self.drag.is_none() {
            self.goal = None;
            self.legs.clear();
        }
        let step = self.scale.system_distance(self.l) * SPACE_PAN_RATE * seconds;
        self.pan_by(right * step, forward * step);
    }

    fn edge_direction(&self) -> (f32, f32) {
        let (Some((x, y)), (width, height)) = (self.pointer, self.viewport) else {
            return (0.0, 0.0);
        };
        if self.drag.is_some() || width <= 0.0 || height <= 0.0 {
            return (0.0, 0.0);
        }
        let axis = |position: f32, extent: f32| {
            if position <= EDGE_PIXELS {
                -1.0
            } else if position >= extent - EDGE_PIXELS {
                1.0
            } else {
                0.0
            }
        };
        (axis(x, width), -axis(y, height))
    }

    /// Whether a flight is under way.
    pub fn flying(&self) -> bool {
        self.goal.is_some() || !self.legs.is_empty() || self.look.is_some()
    }

    fn fly_to(&mut self, l: f32) {
        self.goal = Some(l.clamp(self.l_min(), self.l_cap()));
    }

    /// Starts a flight of `legs`, replacing any flight under way.
    fn fly(&mut self, legs: impl IntoIterator<Item = Leg>, orbit: &mut OrbitCamera) {
        self.interrupt();
        self.legs.extend(legs);
        self.run_legs(orbit);
    }

    /// Applies the legs that take no time and starts the next zoom.
    fn run_legs(&mut self, orbit: &mut OrbitCamera) {
        while let Some(&leg) = self.legs.front() {
            match leg {
                Leg::Zoom { to, out_only } => {
                    if !self.leg_started {
                        self.leg_started = true;
                        let to = if out_only { to.max(self.l) } else { to };
                        self.fly_to(to);
                    }
                    if self.goal.is_some() {
                        return;
                    }
                    self.leg_started = false;
                }
                Leg::System { index, at } => self.anchor_system_at(index, at, orbit),
                Leg::Body(index) => self.anchor_to(Some(index), orbit),
                Leg::Point(at) => {
                    let before = self.system_target(orbit);
                    self.pan = sub3(at, self.anchor_pos());
                    let after = sub3(self.system_target(orbit), self.offset);
                    self.offset = sub3(before, after);
                }
                Leg::Free => self.free_focus(orbit),
                Leg::Look(dir) => self.look = Some(dir),
                Leg::Surface(id) => self.want_surface = Some(id),
            }
            self.legs.pop_front();
        }
    }

    /// Reanchors the view on `body` (or the star) without moving the picture: the difference is
    /// kept as an offset that settles, so the camera glides. A camera that starts following the
    /// loaded planet takes the planet camera over from the pose it shows.
    fn anchor_to(&mut self, body: Option<usize>, orbit: &mut OrbitCamera) {
        let before = self.system_target(orbit);
        let shown = self.pose(orbit);
        let tracking = self.anchored_at_surface();
        self.body = body;
        self.pan = [0.0; 3];
        if self.anchored_at_surface() && !tracking {
            self.align_orbit(orbit, shown);
        }
        let after = sub3(self.system_target(orbit), self.offset);
        self.offset = sub3(before, after);
    }

    /// Reanchors the view on system `index` with the focus at `at` of its plane, gliding on the
    /// star map from where it looks now.
    fn anchor_system_at(&mut self, index: usize, at: [f32; 3], orbit: &OrbitCamera) {
        let before = self.map_target(orbit);
        self.set_system(index);
        self.pan = at;
        let after = self.map_target(orbit);
        self.offset = scale3(sub3(before, after), 1.0 / self.k());
    }

    /// The log-distance of a view of a whole system around a point of it.
    fn system_focus_l(&self) -> f32 {
        let l = self.scale.l_globe_system + (SYSTEM_FOCUS_DISTANCE / self.scale.system_min).ln();
        l.max(self.scale.l_globe_system + FADE_HALF + HANDOVER_SPAN + 0.05)
            .min(self.scale.l_system_map - FADE_HALF - 0.05)
    }

    /// The log-distance of the star map, high enough that the system frame is gone.
    fn map_focus_l(&self) -> f32 {
        let l = self.scale.l_system_map + (MAP_FOCUS_DISTANCE / MAP_MIN_DISTANCE).ln();
        l.max(self.scale.l_system_map + FADE_HALF + 0.05)
            .min(self.scale.l_end)
    }

    /// Flies out of the planet until the plane's own pose shows, if needed.
    fn leave_planet(&self) -> Leg {
        Leg::Zoom {
            to: self.space_floor() + HANDOVER_SPAN + 0.05,
            out_only: true,
        }
    }

    /// Flies out to the star map, if needed, before the camera crosses to system `index`: high
    /// enough that the crossing glides over a picture wider than the way.
    fn out_to_map(&self, index: usize, orbit: &OrbitCamera) -> Leg {
        let from = self.map_target(orbit);
        let to = self.map_positions.get(index).copied().unwrap_or(from);
        let way = length3(sub3(to, from));
        let fit = self.scale.l_system_map + (way * 1.5 / MAP_MIN_DISTANCE).max(1.0).ln();
        Leg::Zoom {
            to: fit
                .max(self.scale.l_system_map + FADE_HALF + 0.1)
                .min(self.scale.l_end),
            out_only: true,
        }
    }

    /// Flies to a planet, a star or a system found by a double click and zooms in to the next
    /// level. A planet without a generated surface is shown as a plain sphere: the zoom stops at
    /// the system level until its globe is loaded.
    pub fn fly_to_target(&mut self, target: Target, orbit: &mut OrbitCamera) {
        match target {
            Target::System(index) => {
                let legs = [
                    Leg::System {
                        index,
                        at: [0.0; 3],
                    },
                    Leg::Zoom {
                        to: self.scale.l_system_map - FADE_HALF - 0.1,
                        out_only: false,
                    },
                ];
                self.fly(legs, orbit);
            }
            Target::Body(index) => {
                let body = &self.scene.bodies[index];
                let (kind, id) = (body.kind, body.id);
                if kind == BodyViewKind::Star {
                    let to = self.system_focus_l().max(self.l);
                    let legs = [
                        self.leave_planet(),
                        Leg::Free,
                        Leg::Point([0.0; 3]),
                        Leg::Zoom {
                            to,
                            out_only: false,
                        },
                    ];
                    self.fly(legs, orbit);
                    return;
                }
                if Some(index) == self.surface_body {
                    let to = self.scale.globe_max.max(MIN_DISTANCE).ln() - 0.2;
                    self.fly(
                        [
                            Leg::Body(index),
                            Leg::Zoom {
                                to,
                                out_only: false,
                            },
                        ],
                        orbit,
                    );
                } else {
                    let mut legs = vec![Leg::Body(index)];
                    if kind == BodyViewKind::Habitable {
                        legs.push(Leg::Surface(id));
                    }
                    legs.push(Leg::Zoom {
                        to: self.space_floor(),
                        out_only: false,
                    });
                    self.fly(legs, orbit);
                }
            }
        }
    }

    /// Flies to `place`: out of the planet and across the star map when it lies in another
    /// system, then in to the level it is seen at. Refused when the place lies beyond the cap.
    pub fn focus(
        &mut self,
        place: FocusPlace,
        orbit: &mut OrbitCamera,
    ) -> Result<(), FocusRefused> {
        match place {
            FocusPlace::Globe(dir) => {
                let surface = self.surface_body.ok_or(FocusRefused::Unknown)?;
                let planet_l = PLANET_FOCUS_DISTANCE
                    .ln()
                    .min(self.scale.globe_max.max(MIN_DISTANCE).ln());
                if self.anchored_at_surface() && self.level() <= Level::Globe {
                    self.fly([Leg::Look(dir)], orbit);
                    return Ok(());
                }
                let system = self
                    .scene
                    .system_index(self.scene.bodies[surface].system)
                    .ok_or(FocusRefused::Unknown)?;
                let mut legs = Vec::new();
                if system != self.system {
                    if self.cap < Level::StarMap {
                        return Err(FocusRefused::BeyondReach);
                    }
                    let at = self
                        .scene
                        .place_system(self.scene.systems[system].id)
                        .iter()
                        .find(|p| p.body == surface)
                        .map_or([0.0; 3], |p| p.pos);
                    legs.push(self.out_to_map(system, orbit));
                    legs.push(Leg::System { index: system, at });
                }
                legs.extend([
                    Leg::Body(surface),
                    Leg::Look(dir),
                    Leg::Zoom {
                        to: planet_l,
                        out_only: false,
                    },
                ]);
                self.fly(legs, orbit);
            }
            FocusPlace::System { system, at } => {
                if self.cap < Level::System {
                    return Err(FocusRefused::BeyondReach);
                }
                let index = self
                    .scene
                    .system_index(system)
                    .ok_or(FocusRefused::Unknown)?;
                let to = self.system_focus_l();
                if index == self.system {
                    let legs = [
                        self.leave_planet(),
                        Leg::Free,
                        Leg::Point(at),
                        Leg::Zoom {
                            to,
                            out_only: false,
                        },
                    ];
                    self.fly(legs, orbit);
                } else {
                    if self.cap < Level::StarMap {
                        return Err(FocusRefused::BeyondReach);
                    }
                    let legs = [
                        self.out_to_map(index, orbit),
                        Leg::System { index, at },
                        Leg::Zoom {
                            to,
                            out_only: false,
                        },
                    ];
                    self.fly(legs, orbit);
                }
            }
            FocusPlace::Planet(index) => {
                let body = self.scene.bodies.get(index).ok_or(FocusRefused::Unknown)?;
                let id = body.id;
                let system = self
                    .scene
                    .system_index(body.system)
                    .ok_or(FocusRefused::Unknown)?;
                if self.cap < Level::System {
                    return Err(FocusRefused::BeyondReach);
                }
                if system == self.system {
                    self.fly_to_target(Target::Body(index), orbit);
                } else {
                    if self.cap < Level::StarMap {
                        return Err(FocusRefused::BeyondReach);
                    }
                    let at = self
                        .scene
                        .place_system(self.scene.systems[system].id)
                        .iter()
                        .find(|p| p.body == index)
                        .map_or([0.0; 3], |p| p.pos);
                    let legs = [
                        self.out_to_map(system, orbit),
                        Leg::System { index: system, at },
                        Leg::Body(index),
                        Leg::Surface(id),
                        Leg::Zoom {
                            to: self.space_floor(),
                            out_only: false,
                        },
                    ];
                    self.fly(legs, orbit);
                }
            }
            FocusPlace::Map(at) => {
                if self.cap < Level::StarMap {
                    return Err(FocusRefused::BeyondReach);
                }
                let point = scale3(sub3(at, self.here()), 1.0 / self.k());
                let from = self.map_target(orbit);
                let way = length3(sub3(at, from));
                let fit = self.scale.l_system_map + (way * 1.5 / MAP_MIN_DISTANCE).max(1.0).ln();
                let to = self.map_focus_l().max(fit.min(self.scale.l_end));
                let legs = [Leg::Zoom { to, out_only: true }, Leg::Point(point)];
                self.fly(legs, orbit);
            }
        }
        Ok(())
    }

    /// Glides the focus to `at` without changing level: a point of the anchored system's plane,
    /// or of the star map when `map` (a click on the minimap).
    pub fn glide_to(&mut self, at: [f32; 3], map: bool, orbit: &mut OrbitCamera) {
        let point = if map {
            scale3(sub3(at, self.here()), 1.0 / self.k())
        } else {
            at
        };
        self.fly([Leg::Point(point)], orbit);
    }

    /// Keeps `place` centred without changing level (the follow toggle): only when the camera
    /// already shows it and no flight is under way.
    pub fn follow(&mut self, place: FocusPlace) {
        if self.goal.is_some() || !self.legs.is_empty() || self.drag.is_some() {
            return;
        }
        let settle = 1.0 - (-FLY_RATE * self.seconds.max(0.0)).exp();
        match place {
            FocusPlace::Globe(dir) => {
                if self.anchored_at_surface() && self.pans_planet() {
                    self.look = Some(dir);
                }
            }
            FocusPlace::System { system, at } => {
                if self.level() == Level::System
                    && !self.pans_planet()
                    && self.system_id() == Some(system)
                {
                    let wanted = sub3(at, self.anchor_pos());
                    self.pan = lerp3(self.pan, wanted, settle);
                }
            }
            FocusPlace::Map(at) => {
                if self.level() == Level::StarMap {
                    let point = scale3(sub3(at, self.here()), 1.0 / self.k());
                    let wanted = sub3(point, self.anchor_pos());
                    self.pan = lerp3(self.pan, wanted, settle);
                }
            }
            // Following waits for the planet's globe; then the ship is followed on it.
            FocusPlace::Planet(_) => {}
        }
    }

    /// Returns to the home globe: out to the star map if the camera is in another system, across
    /// to the home system, then down to the home planet looking at the first start.
    pub fn fly_home(&mut self, orbit: &mut OrbitCamera) {
        let Some(home_body) = self.home_body else {
            return;
        };
        let system = self
            .scene
            .system_index(self.scene.bodies[home_body].system)
            .unwrap_or(0);
        let mut legs = Vec::new();
        if system != self.system {
            let at = self
                .scene
                .place_system(self.scene.systems[system].id)
                .iter()
                .find(|p| p.body == home_body)
                .map_or([0.0; 3], |p| p.pos);
            legs.push(self.out_to_map(system, orbit));
            legs.push(Leg::System { index: system, at });
        }
        legs.push(Leg::Body(home_body));
        if self.surface_body == Some(home_body) {
            legs.push(Leg::Look(self.home_dir));
            legs.push(Leg::Zoom {
                to: (2.0 * self.scale.planet_radius).max(MIN_DISTANCE).ln(),
                out_only: false,
            });
        } else {
            // The home globe is not loaded: ask for it and fly in once it is.
            legs.push(Leg::Surface(self.scene.bodies[home_body].id));
            legs.push(Leg::Zoom {
                to: self.space_floor(),
                out_only: false,
            });
        }
        self.fly(legs, orbit);
    }

    /// Places the camera for a reproducible view (`--view`): `level` over the home planet.
    pub fn set_view(&mut self, level: Level, orbit: &mut OrbitCamera) {
        self.interrupt();
        if let Some(home) = self.home_body {
            let system = self
                .scene
                .system_index(self.scene.bodies[home].system)
                .unwrap_or(0);
            if system != self.system {
                self.set_system(system);
            }
            self.body = Some(home);
            if self.surface_body == Some(home) {
                orbit.look_at(self.home_dir);
            } else {
                self.want_surface = Some(self.scene.bodies[home].id);
            }
        }
        self.pan = [0.0; 3];
        self.offset = [0.0; 3];
        let l = match level {
            Level::Surface => 12.0_f32.ln(),
            Level::Globe => self.scale.l_cap(Level::Globe),
            Level::System => self.scale.l_globe_system + (8000.0 / self.scale.system_min).ln(),
            Level::StarMap => self.scale.l_end,
        };
        self.l = l.clamp(self.l_min(), self.l_cap());
        if let Some(yaw) = yaw_of(self.planet_to_system(orbit.dir)) {
            self.yaw = yaw;
        }
        self.sync_orbit(orbit);
    }

    /// The screen markers `(x, y, radius)` of what a double click can hit at the current level,
    /// with the target of each.
    pub fn markers(&self, view: &View) -> Vec<(Target, (f32, f32, f32))> {
        let canvas = view.size();
        let mut out = Vec::new();
        match self.level() {
            Level::System => {
                let known = self.knows_system();
                // Each body as large as it is drawn, its kind's floor on screen included.
                let seen: Vec<(&PlacedBody, BodyViewKind, (f32, f32, f32))> = self
                    .placed
                    .iter()
                    .filter_map(|p| {
                        let kind = self.scene.bodies[p.body].kind;
                        if !known && kind != BodyViewKind::Star {
                            return None;
                        }
                        Some((p, kind, view.project(p.pos)?))
                    })
                    .collect();
                let kinds: Vec<BodyViewKind> = seen.iter().map(|s| s.1).collect();
                let pixels: Vec<f32> = seen
                    .iter()
                    .map(|(p, _, (_, _, depth))| p.radius / depth * canvas.1 / (2.0 * view.tan()))
                    .collect();
                let space = self.fades().space;
                let shown = crate::universe::shown_pixels(&kinds, &pixels, space);
                for ((p, _, (x, y, depth)), on_screen) in seen.iter().zip(shown) {
                    let per_pixel = depth * 2.0 * view.tan() / canvas.1;
                    let on_screen = crate::universe::drawn_pixels(on_screen, p.room, per_pixel);
                    out.push((Target::Body(p.body), (*x, *y, on_screen)));
                }
            }
            Level::StarMap => {
                for (i, pos) in self.map_positions.iter().enumerate() {
                    if let Some((x, y, _)) = view.project(*pos) {
                        out.push((Target::System(i), (x, y, PICK_PIXELS)));
                    }
                }
            }
            Level::Surface | Level::Globe => {}
        }
        out
    }

    /// The target under the pixel `(x, y)` of `view`, at the current level.
    pub fn target_at(&self, view: &View, x: f32, y: f32) -> Option<Target> {
        let markers = self.markers(view);
        let rects: Vec<(f32, f32, f32)> = markers.iter().map(|m| m.1).collect();
        nearest_marker(&rects, (x, y)).map(|i| markers[i].0)
    }

    /// Handles a left press at `(x, y)` at `now_ms`: a double click on a planet or a system
    /// flies there. Returns whether it started a flight.
    pub fn press(
        &mut self,
        now_ms: u64,
        (x, y): (f32, f32),
        view: &View,
        orbit: &mut OrbitCamera,
    ) -> bool {
        if !self.clicks.press(now_ms, x, y) {
            return false;
        }
        match self.target_at(view, x, y) {
            Some(target) => {
                self.fly_to_target(target, orbit);
                true
            }
            None => false,
        }
    }
}

fn clamp_disc(p: [f32; 3], limit: f32) -> [f32; 3] {
    let r = p[0].hypot(p[2]);
    if r > limit {
        [p[0] / r * limit, p[1], p[2] / r * limit]
    } else {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universe::sample_scene;

    const R: f32 = 122.0;

    fn scale() -> ZoomScale {
        ZoomScale::new(R, 3.0 * R)
    }

    fn camera(cap: Level) -> (ZoomCamera, OrbitCamera) {
        let scene = Arc::new(sample_scene());
        let home = Home {
            system: 1,
            body: 2,
            dir: [0.0, 0.0, 1.0],
        };
        let orbit = OrbitCamera::at(0.0, 0.0, 3.0 * R, 0.0, R, 3.0 * R);
        (ZoomCamera::new(scene, scale(), Some(home), cap), orbit)
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-2 * (1.0 + a[i].abs()))
    }

    /// Runs `frames` frames of 50 ms, as the render thread does.
    fn run(zoom: &mut ZoomCamera, orbit: &mut OrbitCamera, frames: usize) {
        for _ in 0..frames {
            zoom.adopt_orbit(orbit);
            zoom.advance(0.05);
            zoom.sync_orbit(orbit);
        }
    }

    fn close(a: [f32; 3], b: [f32; 3], eps: f32) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() <= eps)
    }

    fn length(v: [f32; 3]) -> f32 {
        dot3(v, v).sqrt()
    }

    #[test]
    fn log_distance_maps_to_levels_monotonically() {
        let s = scale();
        let mut last = Level::Surface;
        let mut seen = [false; 4];
        let mut l = s.l_min();
        let mut last_distances = (0.0, 0.0, 0.0);
        while l <= s.l_end {
            let level = s.level_of(l);
            assert!(level >= last, "level went back at l = {l}");
            last = level;
            seen[level.index()] = true;
            let d = (s.globe_distance(l), s.system_distance(l), s.map_distance(l));
            assert!(d.0 > last_distances.0 && d.1 > last_distances.1 && d.2 > last_distances.2);
            last_distances = d;
            l += 0.01;
        }
        assert_eq!(seen, [true; 4]);
        // The hand-overs lie in order and the frames meet: the planet looks equally large in
        // the planet frame and in the system frame at the same `l`.
        assert!(s.l_min() < s.l_globe_system && s.l_globe_system < s.l_system_map);
        assert!(s.l_system_map < s.l_end);
        let l = s.l_globe_system;
        let in_planet = R / s.globe_distance(l);
        let in_system = HOME_PLANET_RADIUS / s.system_distance(l);
        assert!((in_planet - in_system).abs() < 1e-4);
        assert!((s.system_distance(s.l_system_map) - SYSTEM_MAX_DISTANCE).abs() < 1.0);
        assert!((s.map_distance(s.l_system_map) - MAP_MIN_DISTANCE).abs() < 1e-2);
        assert!((s.map_distance(s.l_end) - MAP_MAX_DISTANCE).abs() < 1.0);
    }

    #[test]
    fn fades_change_smoothly_with_the_distance() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        let s = *zoom.scale();
        let mut l = s.l_min();
        zoom.set_l(l);
        let mut last = zoom.fades();
        while l <= s.l_end {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            let f = zoom.fades();
            for v in [f.space, f.map] {
                assert!((0.0..=1.0).contains(&v), "l = {l}");
            }
            // Detail only fades in going out.
            assert!(f.space >= last.space - 1e-6 && f.map >= last.map - 1e-6);
            assert!((f.space - last.space).abs() < 0.05 && (f.map - last.map).abs() < 0.05);
            last = f;
            l += 0.01;
        }
        zoom.set_l(s.l_cap(Level::Globe));
        assert_eq!(zoom.fades().space, 0.0);
        zoom.set_l(s.l_cap(Level::System));
        assert_eq!(zoom.fades().map, 0.0);
        zoom.set_l(s.l_end);
        assert_eq!(zoom.fades().map, 1.0);
        // A system's own detail goes with how wide it looks, whichever system it is: all of it
        // at the whole-system view, none out on the map.
        let angle = |l: f32| crate::universe::SYSTEM_RADIUS / s.system_distance(l);
        assert_eq!(crate::layers::system_detail(angle(s.l_system_map)), 1.0);
        assert_eq!(crate::layers::system_detail(angle(s.l_end)), 0.0);
    }

    /// Where `point` of the system frame shows through the one camera drawn in frame `frame`
    /// (0 the planet's, 1 the system's, 2 the star map's), and how large a sphere of `radius`
    /// there looks, in pixels.
    fn seen(one: &OneView, frame: usize, point: [f32; 3], radius: f32) -> Option<(f32, f32, f32)> {
        let size = (1280.0, 720.0);
        let (view, at, r) = match frame {
            0 => {
                let place = one.place?;
                (
                    one.planet?,
                    place.to_planet_point(point),
                    radius / place.per_tile,
                )
            }
            1 => (one.system, point, radius),
            _ => (
                one.map,
                one.system_place.to_map_point(point),
                radius * one.system_place.per_unit,
            ),
        };
        let (x, y, _) = view.view(size, 0.0).project(at)?;
        Some((x, y, view.sphere_pixels(at, r, size.1)))
    }

    #[test]
    fn one_camera_from_the_street_to_the_star_map_without_jumps() {
        let (mut zoom, mut orbit) = turned_camera();
        let s = *zoom.scale();
        let surface = zoom.surface_body().unwrap();
        let planet = *zoom.placed().iter().find(|p| p.body == surface).unwrap();
        let place = zoom.one_view(&orbit).place.unwrap();
        // The home planet, the star and a ship in orbit above the point looked at.
        let ship = place.to_system_point(scale3(orbit.dir, R + 8.0));
        let objects = [
            (planet.pos, planet.radius),
            ([0.0; 3], crate::universe::STAR_RADIUS),
            (ship, 6.0),
        ];
        let mut ls = Vec::new();
        let mut l = s.l_min();
        while l <= s.l_end {
            ls.push(l);
            l += 0.02;
        }
        let back: Vec<f32> = ls.iter().rev().copied().collect();
        ls.extend(back);
        let mut last: Vec<Option<(f32, f32, f32)>> = vec![None; objects.len()];
        let (mut compared, mut stepped) = ([0; 3], 0);
        for &l in &ls {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            let one = zoom.one_view(&orbit);
            for (i, &(point, radius)) in objects.iter().enumerate() {
                let now = seen(&one, 1, point, radius);
                // Every frame shows the same picture: the planet's, the system's and the map's.
                for frame in [0, 2] {
                    if let (Some(a), Some(b)) = (now, seen(&one, frame, point, radius))
                        && a.0.abs() < 4000.0
                        && a.1.abs() < 4000.0
                    {
                        let tolerance = 0.5 + 1e-3 * a.2;
                        assert!(
                            (a.0 - b.0).abs() < tolerance && (a.1 - b.1).abs() < tolerance,
                            "frame {frame} object {i} at l = {l}: {a:?} vs {b:?}"
                        );
                        assert!((a.2 - b.2).abs() < 0.01 * a.2.max(1.0), "size at l = {l}");
                        compared[frame] += 1;
                    }
                }
                // From one small step to the next nothing jumps, here or at the old hand-overs.
                if let (Some(a), Some(b)) = (last[i], now)
                    && a.0.abs() < 1280.0
                    && a.1.abs() < 720.0
                    && b.0.abs() < 1280.0
                    && b.1.abs() < 720.0
                {
                    let moved = (a.0 - b.0).hypot(a.1 - b.1);
                    assert!(
                        moved < 30.0 + 0.1 * a.2,
                        "object {i} moved {moved} px at l = {l}"
                    );
                    let grew = (b.2.max(0.5) / a.2.max(0.5)).ln().abs();
                    assert!(grew < 0.1, "object {i} changed size by {grew} at l = {l}");
                    stepped += 1;
                }
                last[i] = now;
            }
        }
        // The sweep compared the frames all along the way, near and far.
        assert!(compared[0] > 300 && compared[2] > 600, "{compared:?}");
        assert!(stepped > 600, "{stepped}");
        // At the old hand-overs the frames meet exactly.
        for l in [s.l_globe_system, s.l_system_map] {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            let one = zoom.one_view(&orbit);
            let a = seen(&one, 1, planet.pos, planet.radius).unwrap();
            let b = seen(&one, 2, planet.pos, planet.radius).unwrap();
            assert!((a.0 - b.0).abs() < 0.5 && (a.1 - b.1).abs() < 0.5);
        }
    }

    #[test]
    fn the_star_map_is_ready_for_a_third_coordinate() {
        let size = (1280.0, 720.0);
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        zoom.set_view(Level::StarMap, &mut orbit);
        // A system lifted off the plane is drawn, hovered and picked where it shows.
        zoom.map_positions[1][1] = 9_000.0;
        let view = zoom.map_view(&orbit).view(size, 0.0);
        let (x, y, _) = view.project(zoom.map_positions[1]).unwrap();
        assert_eq!(zoom.target_at(&view, x, y), Some(Target::System(1)));
        // Shift + middle drag up and down tilts the camera about the focus, within limits,
        // so the map can be seen from a low angle or from straight above.
        let before = zoom.pose(&orbit).eye[1].asin();
        zoom.drag_start(600.0, 300.0, true);
        zoom.drag_to(600.0, 200.0, &view);
        let lower = zoom.pose(&orbit).eye[1].asin();
        assert!(lower < before - 0.3, "{before} {lower}");
        zoom.drag_to(600.0, 2_000.0, &view);
        zoom.drag_end();
        let top = zoom.pose(&orbit).eye[1].asin();
        assert!((top - MAX_ELEVATION.to_radians()).abs() < 1e-4, "{top}");
        // Zooming in past the map's threshold enters the system nearest the focus in space,
        // not on the plane.
        let focus = zoom.map_target(&orbit);
        zoom.map_positions[2] = add3(focus, [5.0, 5_000.0, 0.0]);
        zoom.map_positions[1] = add3(focus, [30.0, 0.0, 0.0]);
        zoom.set_l(zoom.scale().l_system_map + FADE_HALF + 0.01);
        zoom.wheel_at(1.0, None, size, 0.0, &mut orbit);
        assert_eq!(zoom.system(), 1);
    }

    #[test]
    fn the_star_stands_in_the_sky_where_the_light_comes_from() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        orbit.look_at(unit3([0.2, 0.4, 0.9]).unwrap());
        zoom.set_l(zoom.scale().l_min());
        zoom.sync_orbit(&mut orbit);
        // The light that represents the sun stands 8 degrees over the horizon ahead.
        let (up, ahead) = (orbit.dir, orbit.forward_tangent());
        let (sin, cos) = 8.0_f32.to_radians().sin_cos();
        zoom.follow_sun(unit3(add3(scale3(ahead, cos), scale3(up, sin))).unwrap());
        let one = zoom.one_view(&orbit);
        let size = (1280.0, 720.0);
        let (x, y, _) = one.system.view(size, 0.0).project([0.0; 3]).expect("ahead");
        assert!((x - 640.0).abs() < 40.0, "{x}");
        // Above the middle of the picture: the street camera looks down on the ground.
        assert!(y < 360.0, "{y}");
        // And through the planet frame the same place.
        let star = one.place.unwrap().to_planet_point([0.0; 3]);
        let (px, py, _) = one.planet.unwrap().view(size, 0.0).project(star).unwrap();
        assert!((px - x).abs() < 1.0 && (py - y).abs() < 1.0);
    }

    #[test]
    fn wheel_steps_from_the_street_to_the_star_map() {
        let (mut zoom, _) = camera(Level::StarMap);
        let s = *zoom.scale();
        let count = |zoom: &mut ZoomCamera, from: f32, to: f32| {
            zoom.set_l(from);
            let mut n = 0;
            while zoom.l() < to - 1e-4 && n < 1000 {
                zoom.wheel(-1.0);
                n += 1;
            }
            n
        };
        // Where the whole system fits and where the whole star map does.
        let whole = s.l_system_map;
        let street_globe = count(&mut zoom, s.l_min(), s.l_cap(Level::Globe));
        let street_system = count(&mut zoom, s.l_min(), whole);
        let globe_system = count(&mut zoom, s.l_cap(Level::Globe), whole);
        let system_map = count(&mut zoom, whole, s.l_end);
        println!(
            "wheel steps: street->globe {street_globe}, street->whole system {street_system}, \
             globe->whole system {globe_system}, whole system->star map {system_map}"
        );
        // Tasks 53 and 56 needed 30, 52, 22 and 20 on this scale.
        assert!(street_globe <= 30);
        assert!(street_system <= 40);
        assert!(globe_system <= 13);
        assert!(system_map <= 11);
        // The step never changes abruptly.
        let mut last = zoom.wheel_log(s.l_min());
        let mut l = s.l_min();
        while l < s.l_end {
            let now = zoom.wheel_log(l);
            assert!((now - last).abs() < 0.01, "l = {l}");
            last = now;
            l += 0.01;
        }
    }

    #[test]
    fn system_view_and_globe_agree_on_the_sun() {
        use crate::sun::{SunClock, SunSettings};
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        let home = [0.3_f32, -0.2, 0.9];
        let length = home.iter().map(|c| c * c).sum::<f32>().sqrt();
        let surface = zoom.surface_body().expect("the home globe is loaded");
        let planet = zoom
            .placed()
            .iter()
            .find(|p| p.body == surface)
            .unwrap()
            .pos;
        let star = zoom
            .placed()
            .iter()
            .find(|p| zoom.scene().bodies[p.body].kind == BodyViewKind::Star)
            .unwrap()
            .pos;
        let to_star = sub3(star, planet);
        let to_star = scale3(
            to_star,
            1.0 / to_star.iter().map(|c| c * c).sum::<f32>().sqrt(),
        );
        let s = *zoom.scale();
        for settings in [
            SunSettings::default(),
            SunSettings {
                axial_tilt_mdeg: 23_440,
                ..SunSettings::default()
            },
            SunSettings {
                fixed: true,
                ..SunSettings::default()
            },
        ] {
            let clock = SunClock::new(settings, Some(home.map(|c| c / length)));
            for tick in [0, 1, 1_500, 2_999, 6_000, 12_345, 1_000_001] {
                // The globe's light direction at this tick, carried into the system frame the way the
                // planet is drawn there, points from the planet to its star.
                let toward = clock.toward(tick);
                zoom.follow_sun(toward);
                let carried = zoom.planet_to_system(toward);
                assert!(
                    (0..3).all(|i| (carried[i] - to_star[i]).abs() < 1e-4),
                    "tick {tick}: {carried:?} vs {to_star:?}"
                );
                // The point under the light on the globe is the system-view planet's point nearest
                // the star, and the camera's centre follows the same turn.
                orbit.look_at(toward);
                zoom.set_l(s.l_globe_system);
                let under = zoom.system_target(&orbit);
                let surface_distance = sub3(star, under).iter().map(|c| c * c).sum::<f32>().sqrt();
                let centre_distance = sub3(star, planet).iter().map(|c| c * c).sum::<f32>().sqrt();
                let drawn_radius = R * s.system_per_tile();
                assert!((centre_distance - surface_distance - drawn_radius).abs() < 0.5);
            }
        }
    }

    #[test]
    fn frame_switch_keeps_the_view_centre() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        orbit.look_at([0.3, 0.5, 0.8]);
        let s = *zoom.scale();
        let planet = zoom.placed().iter().find(|p| p.body == 1).unwrap().pos;
        // Planet frame to system frame: the point under the centre stays the centre.
        zoom.set_l(s.l_globe_system);
        let centre = globe_to_system(&s, planet, orbit.target());
        assert!(near(zoom.system_target(&orbit), centre));
        // Just below and above the hand-over the system frame's centre moves continuously.
        let mut last = None;
        for step in -10..=10 {
            zoom.set_l(s.l_globe_system + step as f32 * 0.002);
            let t = zoom.system_target(&orbit);
            if let Some(prev) = last {
                let jump = sub3(t, prev);
                assert!(jump[0].hypot(jump[2]).hypot(jump[1]) < 5.0, "{jump:?}");
            }
            last = Some(t);
        }
        // System frame to star-map frame: the star-map centre is the system's place.
        zoom.set_l(s.l_system_map);
        let here = zoom.map_positions[zoom.system()];
        assert!(near(
            zoom.map_target(&orbit),
            system_to_map(&s, here, zoom.system_target(&orbit))
        ));
        // The planet frame's centre point projects to the same place in the system frame's
        // picture: the centre of the screen.
        zoom.set_l(s.l_globe_system);
        let size = (1280.0, 720.0);
        let view = zoom.system_view(&orbit).view(size, 0.0);
        let (x, y, _) = view.project(centre).unwrap();
        assert!(
            (x - 640.0).abs() < 0.5 && (y - 360.0).abs() < 0.5,
            "{x} {y}"
        );
    }

    #[test]
    fn max_level_caps_zoom() {
        let (mut zoom, mut orbit) = camera(Level::Globe);
        for _ in 0..500 {
            zoom.wheel(-1.0);
        }
        assert_eq!(zoom.level(), Level::Globe);
        assert!((zoom.l() - zoom.scale().l_cap(Level::Globe)).abs() < 1e-5);
        assert_eq!(zoom.fades().space, 0.0);
        // Flights and views stay under the cap too.
        zoom.set_view(Level::StarMap, &mut orbit);
        assert_eq!(zoom.level(), Level::Globe);
        zoom.set_cap(Level::StarMap);
        for _ in 0..500 {
            zoom.wheel(-1.0);
        }
        assert_eq!(zoom.level(), Level::StarMap);
        assert!((zoom.l() - zoom.scale().l_end).abs() < 1e-5);
        // Lowering the cap brings the camera back.
        zoom.set_cap(Level::System);
        assert_eq!(zoom.level(), Level::System);
        assert_eq!(zoom.fades().map, 0.0);
        zoom.set_cap(Level::Globe);
        assert_eq!(zoom.level(), Level::Globe);
        // Zooming in is limited by the closest planet camera.
        for _ in 0..500 {
            zoom.wheel(1.0);
        }
        assert_eq!(zoom.level(), Level::Surface);
        assert!((zoom.l() - zoom.scale().l_min()).abs() < 1e-5);
    }

    #[test]
    fn double_click_targets_the_body_under_the_cursor() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        zoom.set_view(Level::System, &mut orbit);
        assert_eq!(zoom.level(), Level::System);
        let size = (1280.0, 720.0);
        let view = zoom.system_view(&orbit).view(size, 0.0);
        let placed = zoom.placed().to_vec();
        for p in &placed {
            let (x, y, _) = view.project(p.pos).expect("in front");
            let hit = zoom.target_at(&view, x + 1.0, y - 1.0);
            assert_eq!(hit, Some(Target::Body(p.body)), "{p:?}");
        }
        // Empty space hits nothing.
        assert_eq!(zoom.target_at(&view, 5.0, 5.0), None);
        // A double click flies to the planet; a single one does not.
        let (x, y, _) = view.project(placed[2].pos).unwrap();
        assert!(!zoom.press(1000, (x, y), &view, &mut orbit));
        assert!(!zoom.press(1700, (x, y), &view, &mut orbit));
        assert!(zoom.press(1800, (x, y), &view, &mut orbit));
        assert_eq!(zoom.anchor_body(), Some(placed[2].body));
        assert!(zoom.flying());
        // A resource body has no surface: the flight stops at the system level.
        run(&mut zoom, &mut orbit, 400);
        assert_eq!(zoom.level(), Level::System);
        assert!(!zoom.anchored_at_surface());
        // The nearest marker wins and the pick radius has a floor.
        assert_eq!(
            nearest_marker(&[(10.0, 10.0, 1.0), (14.0, 10.0, 1.0)], (13.0, 10.0)),
            Some(1)
        );
        assert_eq!(nearest_marker(&[(10.0, 10.0, 1.0)], (200.0, 10.0)), None);
        // On the star map a double click flies to a system and zooms in to its level.
        zoom.set_view(Level::StarMap, &mut orbit);
        let view = zoom.map_view(&orbit).view(size, 0.0);
        let (x, y, _) = view.project(zoom.map_positions[1]).unwrap();
        assert_eq!(zoom.target_at(&view, x, y), Some(Target::System(1)));
        zoom.press(5000, (x, y), &view, &mut orbit);
        assert!(zoom.press(5100, (x, y), &view, &mut orbit));
        assert_eq!(zoom.system(), 1);
        run(&mut zoom, &mut orbit, 400);
        assert_eq!(zoom.level(), Level::System);
    }

    #[test]
    fn home_returns_to_the_home_globe() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        zoom.set_view(Level::StarMap, &mut orbit);
        zoom.fly_to_target(Target::System(1), &mut orbit);
        run(&mut zoom, &mut orbit, 400);
        assert_eq!(zoom.system(), 1);
        zoom.fly_home(&mut orbit);
        run(&mut zoom, &mut orbit, 600);
        assert_eq!(zoom.level(), Level::Globe);
        assert_eq!(zoom.system(), 0);
        assert!(zoom.anchored_at_surface());
        assert!((orbit.distance - 2.0 * R).abs() < 1.0);
        // The flight ends looking at the first start.
        assert!(dot3(orbit.dir, [0.0, 0.0, 1.0]) > 0.999);
    }

    #[test]
    fn click_tracker_needs_two_close_presses() {
        let mut tracker = ClickTracker::default();
        assert!(!tracker.press(0, 10.0, 10.0));
        assert!(tracker.press(200, 12.0, 11.0));
        assert!(!tracker.press(300, 12.0, 11.0));
        assert!(!tracker.press(1000, 12.0, 11.0));
        assert!(!tracker.press(1100, 100.0, 11.0));
    }

    /// The planet camera as the system frame sees it: eye, target and up carried over.
    fn planet_in_system(zoom: &ZoomCamera, orbit: &OrbitCamera) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let s = zoom.scale();
        let anchor = zoom
            .placed()
            .iter()
            .find(|p| Some(p.body) == zoom.surface_body())
            .unwrap()
            .pos;
        let drawn = zoom.draw_orbit(orbit);
        let carry = |p: [f32; 3]| globe_to_system(s, anchor, zoom.planet_to_system(p));
        (
            carry(drawn.position()),
            carry(drawn.target()),
            zoom.planet_to_system(drawn.view_up()),
        )
    }

    fn turned_camera() -> (ZoomCamera, OrbitCamera) {
        let (mut zoom, _) = camera(Level::StarMap);
        let mut orbit = OrbitCamera::at(40.0, -25.0, 3.0 * R, 70.0, R, 3.0 * R);
        zoom.follow_sun(unit3([0.3, 0.2, -0.9]).unwrap());
        zoom.set_l(orbit.max_distance.ln());
        zoom.sync_orbit(&mut orbit);
        (zoom, orbit)
    }

    #[test]
    fn planet_hand_over_shows_the_planet_cameras_own_picture() {
        let (mut zoom, mut orbit) = turned_camera();
        let s = *zoom.scale();
        // Through the whole hand-over band the system camera is the planet camera, scaled: the
        // same eye, the same point looked at and the same up, so the cross-fade shows one picture.
        let mut l = orbit.max_distance.ln();
        while l <= s.l_globe_system + FADE_HALF {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            let view = zoom.system_view(&orbit);
            let (eye, target, up) = planet_in_system(&zoom, &orbit);
            let distance = length(sub3(view.eye, view.target));
            assert!(close(view.target, target, 1e-3 * distance), "l = {l}");
            assert!(close(view.eye, eye, 1e-3 * distance), "l = {l}");
            assert!(close(view.up, unit3(up).unwrap(), 1e-4), "l = {l}");
            l += 0.01;
        }
        // Beyond it the view turns and tilts continuously to the plane's pose: no step jumps.
        let mut last = zoom.pose(&orbit);
        let mut last_target = zoom.map_target(&orbit);
        let mut l = s.l_globe_system;
        while l <= s.l_end {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            let pose = zoom.pose(&orbit);
            assert!(dot3(pose.eye, last.eye) > 0.995, "eye jumps at l = {l}");
            assert!(dot3(pose.up, last.up) > 0.995, "up jumps at l = {l}");
            let target = zoom.map_target(&orbit);
            let step = length(sub3(target, last_target));
            assert!(step < 0.05 * s.map_distance(l), "focus jumps at l = {l}");
            last = pose;
            last_target = target;
            l += 0.01;
        }
        // Out there the plane's own pose: no roll, the elevation of the level.
        let pose = zoom.pose(&orbit);
        assert!(close(pose.up, level_up(pose.eye, zoom.yaw), 1e-5));
        assert!((pose.eye[1].asin() - zoom.elevation_at(s.l_end)).abs() < 1e-4);
    }

    #[test]
    fn heading_is_kept_from_planet_to_star_map_and_back() {
        let (mut zoom, mut orbit) = turned_camera();
        let s = *zoom.scale();
        let start = orbit;
        // Leaving the planet, the space heading is the planet camera's.
        zoom.set_l(s.l_globe_system + FADE_HALF);
        zoom.sync_orbit(&mut orbit);
        let carried = zoom.planet_to_system(orbit.dir);
        let heading = zoom.heading();
        assert!(
            (carried[0].atan2(carried[2]) + heading).rem_euclid(TAU_F) < 1e-4
                || (carried[0].atan2(carried[2]) + heading).rem_euclid(TAU_F) > TAU_F - 1e-4
        );
        // The system and the star map share it, and the two frames meet exactly at the hand-over.
        for l in [s.l_globe_system + 1.5, s.l_system_map, s.l_end] {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            assert!((zoom.heading() - heading).abs() < 1e-6);
        }
        zoom.set_l(s.l_system_map);
        let system = zoom.system_view(&orbit);
        let map = zoom.map_view(&orbit);
        let here = zoom.map_positions()[zoom.system()];
        let distance = length(sub3(map.eye, map.target));
        assert!(close(
            map.eye,
            system_to_map(&s, here, system.eye),
            1e-3 * distance
        ));
        assert!(close(
            map.target,
            system_to_map(&s, here, system.target),
            1e-3 * distance
        ));
        assert!(close(map.up, system.up, 1e-6));
        // Back down without other input: the planet camera is where it was.
        let mut l = s.l_end;
        while l > start.max_distance.ln() {
            zoom.set_l(l);
            zoom.sync_orbit(&mut orbit);
            l -= 0.05;
        }
        assert!(dot3(orbit.dir, start.dir) > 0.99999);
        assert!(dot3(orbit.forward_tangent(), start.forward_tangent()) > 0.99999);
        // A turn in space turns the planet camera about the planet's axis by as much, so the
        // heading seen on the way back in is the one the player chose.
        zoom.set_l(s.l_globe_system + 1.6);
        zoom.rotate_heading(0.6);
        zoom.sync_orbit(&mut orbit);
        let turned = zoom.heading();
        assert!((turned - (heading + 0.6).rem_euclid(TAU_F)).abs() < 1e-4);
        zoom.set_l(s.l_globe_system);
        zoom.sync_orbit(&mut orbit);
        assert!((zoom.heading() - turned).abs() < 1e-3);
    }

    const TAU_F: f32 = std::f32::consts::TAU;

    fn free_system_view(zoom: &mut ZoomCamera, orbit: &mut OrbitCamera) {
        zoom.set_view(Level::System, orbit);
        let size = (1280.0, 720.0);
        let view = zoom.system_view(orbit).view(size, 0.0);
        // A wheel step over empty space lets go of the planet.
        zoom.wheel_at(0.5, Some((40.0, 40.0)), size, 0.0, orbit);
        zoom.sync_orbit(orbit);
        assert_eq!(zoom.anchor_body(), None);
        let _ = view;
    }

    #[test]
    fn zoom_follows_the_cursor_on_the_system_plane_and_the_star_map() {
        let size = (1280.0, 720.0);
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        free_system_view(&mut zoom, &mut orbit);
        for (steps, cursor) in [(1.0, (900.0, 500.0)), (-1.0, (200.0, 150.0))] {
            let before = zoom.system_view(&orbit).view(size, 0.0);
            let held = plane_hit(&before, cursor.0, cursor.1).unwrap();
            zoom.wheel_at(steps, Some(cursor), size, 0.0, &mut orbit);
            zoom.sync_orbit(&mut orbit);
            let after = zoom.system_view(&orbit).view(size, 0.0);
            let (x, y, _) = after.project(held).unwrap();
            assert!((x - cursor.0).hypot(y - cursor.1) < 0.5, "{x} {y}");
        }
        zoom.set_l(zoom.scale().l_end - 1.0);
        let cursor = (300.0, 420.0);
        let before = zoom.map_view(&orbit).view(size, 0.0);
        let held = plane_hit(&before, cursor.0, cursor.1).unwrap();
        zoom.wheel_at(1.0, Some(cursor), size, 0.0, &mut orbit);
        let after = zoom.map_view(&orbit).view(size, 0.0);
        let (x, y, _) = after.project(held).unwrap();
        assert!((x - cursor.0).hypot(y - cursor.1) < 0.5, "{x} {y}");
    }

    #[test]
    fn drag_pans_the_plane_and_shift_drag_turns_the_heading() {
        let size = (1280.0, 720.0);
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        free_system_view(&mut zoom, &mut orbit);
        let view = zoom.system_view(&orbit).view(size, 0.0);
        let grabbed = plane_hit(&view, 500.0, 300.0).unwrap();
        zoom.drag_start(500.0, 300.0, false);
        zoom.drag_to(650.0, 420.0, &view);
        zoom.drag_end();
        let after = zoom.system_view(&orbit).view(size, 0.0);
        let (x, y, _) = after.project(grabbed).unwrap();
        assert!((x - 650.0).hypot(y - 420.0) < 1.0, "{x} {y}");
        // Turning keeps the focus and changes the heading as the planet camera's drag does.
        let target = zoom.system_target(&orbit);
        let heading = zoom.heading();
        zoom.drag_start(600.0, 300.0, true);
        zoom.drag_to(700.0, 300.0, &after);
        zoom.sync_orbit(&mut orbit);
        assert!(close(zoom.system_target(&orbit), target, 1e-3));
        let expected = (heading - 100.0 * ROTATE_RATE).rem_euclid(TAU_F);
        assert!((zoom.heading() - expected).abs() < 1e-4);
        // Pan keys move across the plane relative to that heading: forward is up the screen.
        let before = zoom.system_target(&orbit);
        zoom.hold(0, true);
        zoom.advance(0.1);
        zoom.hold(0, false);
        let moved = sub3(zoom.system_target(&orbit), before);
        let view = zoom.system_view(&orbit);
        let screen_up = view.up;
        assert!(dot3(moved, screen_up) > 0.0 && moved[1].abs() < 1e-3);
    }

    #[test]
    fn zooming_in_on_a_body_lands_on_it() {
        let size = (1280.0, 720.0);
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        free_system_view(&mut zoom, &mut orbit);
        // A resource body: the camera anchors on it and stops above it.
        // The cursor rests where the rock first shows, as a player's does.
        let rock = *zoom.placed().iter().find(|p| p.body == 2).unwrap();
        let view = zoom.system_view(&orbit).view(size, 0.0);
        let (x, y, _) = view.project(rock.pos).unwrap();
        for _ in 0..80 {
            zoom.wheel_at(1.0, Some((x, y)), size, 0.0, &mut orbit);
            zoom.sync_orbit(&mut orbit);
            run(&mut zoom, &mut orbit, 2);
        }
        run(&mut zoom, &mut orbit, 100);
        assert_eq!(zoom.anchor_body(), Some(2));
        assert_eq!(zoom.l(), zoom.l_min());
        assert!(close(zoom.system_target(&orbit), rock.pos, 1.0));
        // The loaded planet: the camera goes on down to its surface.
        let planet = *zoom.placed().iter().find(|p| p.body == 1).unwrap();
        zoom.set_l(zoom.scale().l_globe_system + 2.0);
        for _ in 0..120 {
            let view = zoom.system_view(&orbit).view(size, 0.0);
            let (x, y, _) = view.project(planet.pos).unwrap_or((640.0, 360.0, 1.0));
            zoom.wheel_at(1.0, Some((x, y)), size, 0.0, &mut orbit);
            zoom.sync_orbit(&mut orbit);
            run(&mut zoom, &mut orbit, 2);
        }
        assert!(zoom.anchored_at_surface());
        assert_eq!(zoom.level(), Level::Surface);
        // A planet of another system without a globe: the camera anchors and asks for it.
        zoom.fly_to_target(Target::System(1), &mut orbit);
        run(&mut zoom, &mut orbit, 300);
        let other = *zoom.placed().iter().find(|p| p.body == 5).unwrap();
        for _ in 0..60 {
            let view = zoom.system_view(&orbit).view(size, 0.0);
            let Some((x, y, _)) = view.project(other.pos) else {
                break;
            };
            zoom.wheel_at(1.0, Some((x, y)), size, 0.0, &mut orbit);
            zoom.sync_orbit(&mut orbit);
            run(&mut zoom, &mut orbit, 2);
        }
        assert_eq!(zoom.anchor_body(), Some(5));
        assert_eq!(zoom.want_surface(), Some(6));
        assert_eq!(zoom.l(), zoom.l_min());
    }

    #[test]
    fn a_globe_that_loads_keeps_the_picture_and_flies_in() {
        let size = (1280.0, 720.0);
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        zoom.fly_to_target(Target::System(1), &mut orbit);
        run(&mut zoom, &mut orbit, 300);
        let planet = *zoom.placed().iter().find(|p| p.body == 5).unwrap();
        zoom.fly_to_target(Target::Body(5), &mut orbit);
        run(&mut zoom, &mut orbit, 300);
        assert_eq!(zoom.want_surface(), Some(6));
        let view = zoom.system_view(&orbit).view(size, 0.0);
        let before = view.project(planet.pos).unwrap();
        let distance = zoom.scale().system_distance(zoom.l());
        // The globe of a planet of 140 tiles a face arrives.
        let radius = crate::globe::planet_radius(140);
        let mut globe = OrbitCamera::new(radius, 3.0 * radius);
        zoom.surface_ready(6, radius, 3.0 * radius, [0.0, 0.0, 1.0], &mut globe);
        assert!((zoom.scale().system_distance(zoom.l()) - distance).abs() < 0.5);
        zoom.sync_orbit(&mut globe);
        let view = zoom.system_view(&globe).view(size, 0.0);
        let after = view.project(planet.pos).unwrap();
        assert!((after.0 - before.0).hypot(after.1 - before.1) < 1.0);
        // Then the camera goes down onto it.
        run(&mut zoom, &mut globe, 300);
        assert!(zoom.anchored_at_surface());
        assert!(zoom.level() <= Level::Globe);
    }

    #[test]
    fn zooming_in_on_a_system_of_the_star_map_enters_it() {
        let size = (1280.0, 720.0);
        // Small and large wheel steps alike: the step that crosses into the system frame may
        // land anywhere below the hand-over.
        for steps in [1.0, 3.0, 5.0] {
            let (mut zoom, mut orbit) = camera(Level::StarMap);
            zoom.set_view(Level::StarMap, &mut orbit);
            let mut entered = false;
            let view = zoom.map_view(&orbit).view(size, 0.0);
            let (x, y, _) = view.project(zoom.map_positions()[1]).unwrap();
            for _ in 0..80 {
                zoom.wheel_at(steps, Some((x, y)), size, 0.0, &mut orbit);
                zoom.sync_orbit(&mut orbit);
                run(&mut zoom, &mut orbit, 1);
                entered |= zoom.system() == 1 && zoom.level() == Level::System;
            }
            assert!(entered, "steps {steps}");
            assert_eq!(zoom.system(), 1);
            // The picture never jumped: the star stays where the cursor held it.
            let view = zoom.system_view(&orbit).view(size, 0.0);
            let (sx, sy, _) = view.project([0.0; 3]).unwrap();
            assert!((sx - x).hypot(sy - y) < 2.0, "{sx} {sy} vs {x} {y}");
        }
    }

    #[test]
    fn the_tier_caps_bound_zoom_and_flights() {
        let size = (1280.0, 720.0);
        let (mut zoom, mut orbit) = camera(Level::System);
        for _ in 0..200 {
            zoom.wheel_at(-1.0, Some((100.0, 100.0)), size, 0.0, &mut orbit);
        }
        assert_eq!(zoom.level(), Level::System);
        assert_eq!(
            zoom.focus(FocusPlace::Map([0.0; 3]), &mut orbit),
            Err(FocusRefused::BeyondReach)
        );
        assert_eq!(
            zoom.focus(
                FocusPlace::System {
                    system: 2,
                    at: [0.0; 3]
                },
                &mut orbit
            ),
            Err(FocusRefused::BeyondReach)
        );
        // The own system is in reach.
        assert!(
            zoom.focus(
                FocusPlace::System {
                    system: 1,
                    at: [900.0, 0.0, 300.0]
                },
                &mut orbit
            )
            .is_ok()
        );
        run(&mut zoom, &mut orbit, 400);
        assert_eq!(zoom.system(), 0);
        let (mut planet_only, mut orbit) = camera(Level::Globe);
        assert_eq!(
            planet_only.focus(
                FocusPlace::System {
                    system: 1,
                    at: [0.0; 3]
                },
                &mut orbit
            ),
            Err(FocusRefused::BeyondReach)
        );
    }

    /// Runs a flight and checks every frame for a cut: the log-distance and the star-map focus
    /// move by small steps only.
    fn fly_without_cuts(zoom: &mut ZoomCamera, orbit: &mut OrbitCamera, frames: usize) {
        let mut last = (zoom.l(), zoom.map_target(orbit));
        for frame in 0..frames {
            run(zoom, orbit, 1);
            let now = (zoom.l(), zoom.map_target(orbit));
            assert!((now.0 - last.0).abs() < 0.5, "zoom cut at frame {frame}");
            let step = length(sub3(now.1, last.1));
            let seen = zoom.scale().map_distance(zoom.l().max(last.0));
            assert!(
                step < 0.25 * seen,
                "focus cut at frame {frame}: {step} vs {seen}"
            );
            last = now;
        }
    }

    #[test]
    fn focus_flies_to_a_ship_in_orbit_in_system_space_and_in_transit() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        zoom.set_view(Level::Globe, &mut orbit);
        // A ship in another system's space: out to the star map, across, in to the system.
        let at = [1800.0, 0.0, -700.0];
        zoom.focus(FocusPlace::System { system: 2, at }, &mut orbit)
            .unwrap();
        fly_without_cuts(&mut zoom, &mut orbit, 500);
        assert_eq!(zoom.system(), 1);
        assert_eq!(zoom.level(), Level::System);
        assert!(close(zoom.system_target(&orbit), at, 1.0));
        // A ship between systems: out to the star map, onto its place on the lane.
        let lane = transit_like(&zoom, 0, 1, 0.4);
        zoom.focus(FocusPlace::Map(lane), &mut orbit).unwrap();
        fly_without_cuts(&mut zoom, &mut orbit, 400);
        assert_eq!(zoom.level(), Level::StarMap);
        assert!(close(zoom.map_target(&orbit), lane, 1.0));
        // A ship in orbit of the home planet: back across and down to the globe, looking at it.
        let dir = unit3([0.5, 0.4, 0.7]).unwrap();
        zoom.focus(FocusPlace::Globe(dir), &mut orbit).unwrap();
        fly_without_cuts(&mut zoom, &mut orbit, 600);
        assert_eq!(zoom.system(), 0);
        assert!(zoom.level() <= Level::Globe);
        assert!(dot3(orbit.dir, dir) > 0.999);
        assert!((orbit.distance - PLANET_FOCUS_DISTANCE).abs() < 1.0);
        // Already on the planet, a focus only turns the planet camera.
        let next = unit3([-0.2, 0.3, 0.9]).unwrap();
        zoom.focus(FocusPlace::Globe(next), &mut orbit).unwrap();
        run(&mut zoom, &mut orbit, 200);
        assert!(dot3(orbit.dir, next) > 0.999);
        // Following keeps a moving ship centred.
        zoom.set_view(Level::System, &mut orbit);
        let mut point = [600.0, 0.0, 600.0];
        zoom.focus(
            FocusPlace::System {
                system: 1,
                at: point,
            },
            &mut orbit,
        )
        .unwrap();
        run(&mut zoom, &mut orbit, 300);
        for _ in 0..200 {
            point[0] += 2.0;
            zoom.follow(FocusPlace::System {
                system: 1,
                at: point,
            });
            run(&mut zoom, &mut orbit, 1);
        }
        assert!(length(sub3(zoom.system_target(&orbit), point)) < 30.0);
    }

    fn transit_like(zoom: &ZoomCamera, a: usize, b: usize, progress: f32) -> [f32; 3] {
        lerp3(zoom.map_positions()[a], zoom.map_positions()[b], progress)
    }
}

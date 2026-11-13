//! Continuous zoom across four presentation levels: surface, globe, star system and star map.
//!
//! The camera keeps one log-distance value `l`. Each of the three frames (the planet frame, the
//! system frame and the star-map frame) has its own coordinates and its own camera distance,
//! derived from `l`; crossing a level threshold hands the view over to the next frame, and a
//! short range of `l` around the threshold cross-fades the two, so no frame needs coordinates
//! far from its origin and nothing pops. Pure maths: no window, no GPU.

use std::sync::Arc;

use crate::orbit::{MIN_DISTANCE, OrbitCamera};
use crate::pick::View;
use crate::sun::{planet_spin, rotate_y};
use crate::universe::{BodyViewKind, HOME_PLANET_RADIUS, PlacedBody, UniverseScene};

/// Half the width of a cross-fade, in log-distance units.
pub const FADE_HALF: f32 = 0.3;
/// Orbit distance in tiles below which the view counts as street level.
pub const SURFACE_DISTANCE: f32 = 24.0;
/// System camera distance at which the whole system fits the view, in system units.
pub const SYSTEM_MAX_DISTANCE: f32 = 12_000.0;
/// Star-map camera distance at the hand-over from the system frame, in map units.
pub const MAP_MIN_DISTANCE: f32 = 5000.0;
/// Star-map camera distance at which the whole map fits the view, in map units.
pub const MAP_MAX_DISTANCE: f32 = 230_000.0;
/// Elevation of the system camera above the orbit plane, in degrees.
pub const SYSTEM_ELEVATION: f32 = 50.0;
/// Elevation of the star-map camera above the map plane, in degrees.
pub const MAP_ELEVATION: f32 = 62.0;
/// Log-distance change per wheel step on the globe (a factor of 1 / 0.88).
const WHEEL_LOG: f32 = 0.127_833_37;
/// Wheel steps count this much more in space, where the distances are larger.
const SPACE_WHEEL_FACTOR: f32 = 1.5;
/// Per second rate at which flights and offsets settle.
const FLY_RATE: f32 = 3.5;
/// Pan speed in space as a fraction of the camera distance per second.
const SPACE_PAN_RATE: f32 = 0.5;
/// Seconds between two presses that make a double click.
const DOUBLE_CLICK_MS: u64 = 450;
/// Pixels between two presses that make a double click.
const DOUBLE_CLICK_PIXELS: f32 = 8.0;
/// Smallest pick radius of a body or system on screen, in pixels.
const PICK_PIXELS: f32 = 14.0;

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

/// Weights of the three frames while drawing; they sum to one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameWeights {
    /// The planet frame.
    pub globe: f32,
    /// The system frame.
    pub system: f32,
    /// The star-map frame.
    pub map: f32,
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

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

    /// The weights of the three frames at `l`; they always sum to one.
    pub fn weights(&self, l: f32) -> FrameWeights {
        let rise = |centre: f32| smoothstep((l - (centre - FADE_HALF)) / (2.0 * FADE_HALF));
        let past_globe = rise(self.l_globe_system);
        let past_system = rise(self.l_system_map);
        FrameWeights {
            globe: 1.0 - past_globe,
            system: past_globe - past_system,
            map: past_system,
        }
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

fn space_view(target: [f32; 3], distance: f32, elevation_deg: f32, extent: f32) -> SpaceView {
    let (sin, cos) = elevation_deg.to_radians().sin_cos();
    SpaceView {
        eye: add3(target, scale3([0.0, sin, cos], distance)),
        target,
        up: [0.0, 1.0, 0.0],
        near: (distance * 0.05).max(1.0),
        far: distance + extent,
    }
}

/// What a double click can fly to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The body at this index of [`UniverseScene::bodies`].
    Body(usize),
    /// The system at this index of [`UniverseScene::systems`].
    System(usize),
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

/// The continuous-zoom camera above the planet camera.
#[derive(Clone, Debug)]
pub struct ZoomCamera {
    scene: Arc<UniverseScene>,
    scale: ZoomScale,
    map_positions: Vec<[f32; 3]>,
    /// The system the view is anchored to (an index), with its bodies placed.
    system: usize,
    placed: Vec<PlacedBody>,
    /// The planet the view is anchored to (an index into the scene's bodies).
    body: Option<usize>,
    home_body: Option<usize>,
    /// The planet whose globe is loaded: the only one that can be entered.
    surface_body: Option<usize>,
    /// A planet whose globe the camera waits for.
    want_surface: Option<u32>,
    home_dir: [f32; 3],
    l: f32,
    cap: Level,
    goal: Option<f32>,
    system_pan: [f32; 3],
    map_pan: [f32; 3],
    system_offset: [f32; 3],
    map_offset: [f32; 3],
    held: [bool; 4],
    clicks: ClickTracker,
    /// Turn of the loaded planet about its axis from the planet frame into the system frame.
    spin: f32,
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
            goal: None,
            system_pan: [0.0; 3],
            map_pan: [0.0; 3],
            system_offset: [0.0; 3],
            map_offset: [0.0; 3],
            held: [false; 4],
            clicks: ClickTracker::default(),
            spin: 0.0,
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

    /// The log-distance of the camera.
    pub fn l(&self) -> f32 {
        self.l
    }

    /// Sets the log-distance, within the limits.
    pub fn set_l(&mut self, l: f32) {
        self.goal = None;
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

    /// Largest `l` the cap allows.
    pub fn l_cap(&self) -> f32 {
        self.scale.l_cap(self.cap)
    }

    /// Smallest `l`: a planet without a generated surface cannot be entered.
    pub fn l_min(&self) -> f32 {
        if self.anchored_at_surface() {
            self.scale.l_min()
        } else {
            self.scale.l_globe_system + FADE_HALF
        }
    }

    /// The weights of the frames now.
    pub fn weights(&self) -> FrameWeights {
        self.scale.weights(self.l)
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

    /// The id of the system the view is anchored to.
    pub fn system_id(&self) -> Option<u32> {
        self.scene.systems.get(self.system).map(|s| s.id)
    }

    /// The globe of planet `body` is loaded: `globe_radius` tiles and the planet camera's
    /// farthest distance `max_distance` describe it, `dir` is where the planet camera looks.
    /// The log-distance scale is rebuilt for the planet's size; a camera anchored on the planet
    /// flies in to it.
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
        let drawn = crate::universe::body_radius(&self.scene.bodies[index]);
        let anchored = self.body == Some(index);
        self.scale = ZoomScale::new(
            globe_radius * HOME_PLANET_RADIUS / drawn.max(1.0),
            max_distance,
        );
        self.surface_body = Some(index);
        if self.want_surface == Some(body) {
            self.want_surface = None;
        }
        orbit.look_at(dir);
        self.l = self.l.clamp(self.l_min(), self.l_cap());
        if anchored {
            let target = if Some(index) == self.home_body {
                (2.0 * globe_radius).max(MIN_DISTANCE).ln()
            } else {
                max_distance.max(MIN_DISTANCE).ln() - 0.2
            };
            self.l = self.scale.l_globe_system + FADE_HALF;
            self.fly_to(target);
        }
    }

    /// Zooms in for positive `steps`, out for negative ones, within the limits.
    pub fn wheel(&mut self, steps: f32) {
        self.goal = None;
        let factor = if self.l > self.scale.l_globe_system {
            SPACE_WHEEL_FACTOR
        } else {
            1.0
        };
        self.l = (self.l - steps * WHEEL_LOG * factor).clamp(self.l_min(), self.l_cap());
    }

    /// Makes `orbit` match the camera: its distance follows `l` up to the planet camera's limit.
    pub fn sync_orbit(&self, orbit: &mut OrbitCamera) {
        orbit.distance = self
            .scale
            .globe_distance(self.l)
            .clamp(MIN_DISTANCE, orbit.max_distance);
    }

    /// Takes over a change that the planet camera made itself (a camera reset): in the planet
    /// range the orbit distance is the truth.
    pub fn adopt_orbit(&mut self, orbit: &OrbitCamera) {
        if self.l <= orbit.max_distance.ln() + 1e-3 && self.goal.is_none() {
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
        self.spin = pos.map_or(0.0, |pos| planet_spin(toward, pos, star));
    }

    /// Turn about the planet axis, in radians, from the loaded planet's frame into the system
    /// frame (see [`Self::follow_sun`]).
    pub fn spin(&self) -> f32 {
        self.spin
    }

    /// Records a pan key being held or released (`index`: forward, back, left, right).
    pub fn hold(&mut self, index: usize, pressed: bool) {
        if let Some(h) = self.held.get_mut(index) {
            *h = pressed;
        }
    }

    /// Whether the planet camera should handle pan input: only in the planet range.
    pub fn pans_planet(&self) -> bool {
        self.level() <= Level::Globe
    }

    fn pan_fade(&self, start: f32) -> f32 {
        smoothstep((self.l - start) / 0.5)
    }

    /// Where the system camera looks, in the system frame.
    pub fn system_target(&self, orbit: &OrbitCamera) -> [f32; 3] {
        let anchor = self
            .body
            .and_then(|b| self.placed.iter().find(|p| p.body == b))
            .map_or([0.0; 3], |p| p.pos);
        let base = if self.anchored_at_surface() {
            globe_to_system(&self.scale, anchor, rotate_y(orbit.target(), self.spin))
        } else {
            anchor
        };
        let t = smoothstep(
            (self.l - self.scale.l_globe_system)
                / (self.scale.l_system_map - self.scale.l_globe_system),
        );
        let pan = scale3(self.system_pan, self.pan_fade(self.scale.l_globe_system));
        add3(add3(lerp3(base, [0.0; 3], t), pan), self.system_offset)
    }

    /// Where the star-map camera looks, in the star-map frame.
    pub fn map_target(&self) -> [f32; 3] {
        let here = self
            .map_positions
            .get(self.system)
            .copied()
            .unwrap_or([0.0; 3]);
        // The hand-over keeps the centre: the system frame's own target maps to the map.
        let carried = system_to_map(&self.scale, here, [0.0; 3]);
        let t = smoothstep(
            (self.l - self.scale.l_system_map) / (self.scale.l_end - self.scale.l_system_map),
        );
        let pan = scale3(self.map_pan, self.pan_fade(self.scale.l_system_map));
        add3(add3(lerp3(carried, [0.0; 3], t), pan), self.map_offset)
    }

    /// The system camera.
    pub fn system_view(&self, orbit: &OrbitCamera) -> SpaceView {
        space_view(
            self.system_target(orbit),
            self.scale.system_distance(self.l),
            SYSTEM_ELEVATION,
            crate::universe::SYSTEM_RADIUS * 3.0,
        )
    }

    /// The star-map camera.
    pub fn map_view(&self) -> SpaceView {
        space_view(
            self.map_target(),
            self.scale.map_distance(self.l),
            MAP_ELEVATION,
            crate::universe::MAP_RADIUS * 3.0,
        )
    }

    /// Moves the camera for `seconds`: flights settle and held pan keys move the view across the
    /// frame of the level shown.
    pub fn advance(&mut self, seconds: f32) {
        let settle = 1.0 - (-FLY_RATE * seconds).exp();
        if let Some(goal) = self.goal {
            self.l += (goal - self.l) * settle;
            if (goal - self.l).abs() < 0.003 {
                self.l = goal;
                self.goal = None;
            }
        }
        let keep = 1.0 - settle;
        self.system_offset = scale3(self.system_offset, keep);
        self.map_offset = scale3(self.map_offset, keep);
        let right = f32::from(u8::from(self.held[3])) - f32::from(u8::from(self.held[2]));
        let forward = f32::from(u8::from(self.held[0])) - f32::from(u8::from(self.held[1]));
        if right == 0.0 && forward == 0.0 {
            return;
        }
        match self.level() {
            Level::System => {
                let step = self.scale.system_distance(self.l) * SPACE_PAN_RATE * seconds;
                self.system_pan = add3(self.system_pan, [right * step, 0.0, -forward * step]);
                let limit = crate::universe::SYSTEM_RADIUS * 2.0;
                self.system_pan = clamp_disc(self.system_pan, limit);
            }
            Level::StarMap => {
                let step = self.scale.map_distance(self.l) * SPACE_PAN_RATE * seconds;
                self.map_pan = add3(self.map_pan, [right * step, 0.0, -forward * step]);
                self.map_pan = clamp_disc(self.map_pan, crate::universe::MAP_RADIUS * 1.2);
            }
            Level::Surface | Level::Globe => {}
        }
    }

    /// Whether a flight is under way.
    pub fn flying(&self) -> bool {
        self.goal.is_some()
    }

    fn fly_to(&mut self, l: f32) {
        self.goal = Some(l.clamp(self.l_min(), self.l_cap()));
    }

    /// Reanchors the view on `body` (or the star) without moving the picture: the difference is
    /// kept as an offset that settles, so the camera glides.
    fn anchor_to(&mut self, body: Option<usize>, orbit: &OrbitCamera) {
        let before = self.system_target(orbit);
        self.body = body;
        self.system_pan = [0.0; 3];
        let after = self.system_target(orbit);
        self.system_offset = add3(self.system_offset, sub3(before, after));
    }

    /// Reanchors the view on system `index`, gliding on the star map.
    fn anchor_system(&mut self, index: usize) {
        let before = self.map_target();
        self.system = index;
        self.placed = self
            .scene
            .systems
            .get(index)
            .map(|s| self.scene.place_system(s.id))
            .unwrap_or_default();
        self.body = None;
        self.system_pan = [0.0; 3];
        self.system_offset = [0.0; 3];
        self.map_pan = [0.0; 3];
        let after = self.map_target();
        self.map_offset = add3(self.map_offset, sub3(before, after));
    }

    /// Flies to a planet, a star or a system found by a double click and zooms in to the next
    /// level. A planet without a generated surface is shown as a plain sphere: the zoom stops at
    /// the system level.
    pub fn fly_to_target(&mut self, target: Target, orbit: &mut OrbitCamera) {
        match target {
            Target::System(index) => {
                self.anchor_system(index);
                self.fly_to(self.scale.l_system_map - FADE_HALF - 0.1);
            }
            Target::Body(index) => {
                let body = &self.scene.bodies[index];
                let star = body.kind == BodyViewKind::Star;
                let habitable = body.kind == BodyViewKind::Habitable;
                let id = body.id;
                self.anchor_to((!star).then_some(index), orbit);
                if !star && Some(index) == self.surface_body {
                    self.fly_to(self.scale.globe_max.max(MIN_DISTANCE).ln() - 0.2);
                } else {
                    if habitable {
                        self.want_surface = Some(id);
                    }
                    self.fly_to(self.scale.l_globe_system + FADE_HALF);
                }
            }
        }
    }

    /// Returns to the home globe: anchors on the home planet, looks at the first start and flies
    /// to a view of the whole planet.
    pub fn fly_home(&mut self, orbit: &mut OrbitCamera) {
        let Some(home_body) = self.home_body else {
            return;
        };
        let system = self
            .scene
            .system_index(self.scene.bodies[home_body].system)
            .unwrap_or(0);
        if system != self.system {
            self.anchor_system(system);
        }
        self.anchor_to(Some(home_body), orbit);
        if self.surface_body == Some(home_body) {
            orbit.look_at(self.home_dir);
            self.fly_to((2.0 * self.scale.planet_radius).max(MIN_DISTANCE).ln());
        } else {
            // The home globe is not loaded: ask for it and fly in once it is.
            self.want_surface = Some(self.scene.bodies[home_body].id);
            self.fly_to(self.scale.l_globe_system + FADE_HALF);
        }
    }

    /// Places the camera for a reproducible view (`--view`): `level` over the home planet.
    pub fn set_view(&mut self, level: Level, orbit: &mut OrbitCamera) {
        self.fly_home(orbit);
        self.goal = None;
        let l = match level {
            Level::Surface => 12.0_f32.ln(),
            Level::Globe => self.scale.l_cap(Level::Globe),
            Level::System => self.scale.l_globe_system + (8000.0 / self.scale.system_min).ln(),
            Level::StarMap => self.scale.l_end,
        };
        self.system_offset = [0.0; 3];
        self.map_offset = [0.0; 3];
        self.l = l.clamp(self.l_min(), self.l_cap());
        self.sync_orbit(orbit);
    }

    /// The screen markers `(x, y, radius)` of what a double click can hit at the current level,
    /// with the target of each.
    pub fn markers(&self, view: &View) -> Vec<(Target, (f32, f32, f32))> {
        let canvas = view.size();
        let mut out = Vec::new();
        match self.level() {
            Level::System => {
                for p in &self.placed {
                    if let Some((x, y, depth)) = view.project(p.pos) {
                        let on_screen = p.radius / depth * canvas.1 / (2.0 * view.tan());
                        out.push((Target::Body(p.body), (x, y, on_screen)));
                    }
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
    fn crossfade_weights_sum_to_one() {
        let s = scale();
        let mut l = s.l_min() - 0.5;
        let mut last = s.weights(l);
        while l <= s.l_end + 0.5 {
            let w = s.weights(l);
            assert!((w.globe + w.system + w.map - 1.0).abs() < 1e-5, "l = {l}");
            assert!(w.globe >= 0.0 && w.system >= 0.0 && w.map >= 0.0);
            // The planet frame only fades out, the star-map frame only in.
            assert!(w.globe <= last.globe + 1e-6 && w.map >= last.map - 1e-6);
            last = w;
            l += 0.005;
        }
        // Each frame is alone away from the hand-overs and half-way at them.
        let w = s.weights(s.l_globe_system);
        assert!((w.globe - 0.5).abs() < 1e-5 && (w.system - 0.5).abs() < 1e-5);
        let w = s.weights(s.l_system_map);
        assert!((w.map - 0.5).abs() < 1e-5);
        assert_eq!(s.weights(s.l_cap(Level::Globe)).globe, 1.0);
        assert_eq!(s.weights(s.l_cap(Level::System)).system, 1.0);
        assert_eq!(s.weights(s.l_end).map, 1.0);
    }

    #[test]
    fn system_view_and_globe_agree_on_the_sun() {
        use crate::sun::{SunClock, SunSettings};
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        let home = [0.3_f32, -0.2, 0.9];
        let length = home.iter().map(|c| c * c).sum::<f32>().sqrt();
        let clock = SunClock::new(SunSettings::default(), Some(home.map(|c| c / length)));
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
        for tick in [0, 1, 1_500, 2_999, 6_000, 12_345, 1_000_001] {
            // The globe's light direction at this tick, carried into the system frame the way the
            // planet is drawn there, points from the planet to its star.
            let toward = clock.toward(tick);
            zoom.follow_sun(toward);
            let carried = crate::sun::rotate_y(toward, zoom.spin());
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
            zoom.map_target(),
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
        assert_eq!(zoom.weights().globe, 1.0);
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
        assert_eq!(zoom.weights().system, 1.0);
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
        for _ in 0..400 {
            zoom.advance(0.05);
        }
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
        let view = zoom.map_view().view(size, 0.0);
        let (x, y, _) = view.project(zoom.map_positions[1]).unwrap();
        assert_eq!(zoom.target_at(&view, x, y), Some(Target::System(1)));
        zoom.press(5000, (x, y), &view, &mut orbit);
        assert!(zoom.press(5100, (x, y), &view, &mut orbit));
        assert_eq!(zoom.system(), 1);
        for _ in 0..400 {
            zoom.advance(0.05);
        }
        assert_eq!(zoom.level(), Level::System);
    }

    #[test]
    fn home_returns_to_the_home_globe() {
        let (mut zoom, mut orbit) = camera(Level::StarMap);
        zoom.set_view(Level::StarMap, &mut orbit);
        zoom.fly_to_target(Target::System(1), &mut orbit);
        for _ in 0..400 {
            zoom.advance(0.05);
        }
        zoom.fly_home(&mut orbit);
        for _ in 0..600 {
            zoom.advance(0.05);
        }
        zoom.sync_orbit(&mut orbit);
        assert_eq!(zoom.level(), Level::Globe);
        assert_eq!(zoom.system(), 0);
        assert!(zoom.anchored_at_surface());
        assert!((orbit.distance - 2.0 * R).abs() < 1.0);
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
}

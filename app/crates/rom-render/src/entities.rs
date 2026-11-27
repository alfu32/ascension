//! Entity snapshots, their interpolation and the instance data for drawing them.
//!
//! Everything here is plain data and arithmetic so it can be tested without a GPU. The render
//! thread blends the last two snapshots, samples the terrain surface and turns each entity into
//! a few unit-cube instances; the colour of an instance travels in the three matrix cells that
//! affine transforms leave unused (`m3`, `m7`, `m11`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use rom_geo::SurfacePos;

use crate::bars::BarAnchor;
use crate::globe::Globe;
use crate::layers::{ORBIT_ALTITUDE, ShipScales, hover_altitude, planet_factor, ship_on_screen};
use crate::rig::Activity;
use crate::terrain::{Shape, TerrainScene, vertex_height};

/// Number of owner colours; owner 0 (neutral) is grey.
pub const OWNER_COLORS: usize = 9;
/// Colour of node entities.
pub const NODE_COLOR: [f32; 3] = [0.98, 0.84, 0.16];
/// Colour of the pips that count the units aboard a carrier (task 167): pale, beside the gold of
/// carried nodes.
const ABOARD_COLOR: [f32; 3] = [0.92, 0.95, 1.0];
/// Pips in one row over a carrier; more stack in rows above.
const ABOARD_ROW: u8 = 5;
/// Nominal time between two simulation ticks, used until two snapshots have arrived.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(50);
/// Arrival gaps shorter than this come from catch-up bursts and are ignored for the interval.
const MIN_GAP: Duration = Duration::from_millis(5);
/// Longest interval the interpolation stretches over.
const MAX_INTERVAL: Duration = Duration::from_millis(1000);

/// How an airborne aircraft is drawn while it holds a point (task 119).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hover {
    /// Idle, it flies a circle round its spot (a jet or a biplane).
    #[default]
    Circle,
    /// It hangs where it is (a helicopter).
    Hold,
    /// It hangs where it is and flies lower than the rest (a ducted-fan craft).
    Low,
}

/// The grid of a drone operator (tasks 120 and 128), as the views draw it: a square of slots
/// `spacing` apart hovering `hover` above its centre, turned the way it faces, a small quadrotor
/// in each slot that holds a drone; the operator's strikes dive from there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudView {
    /// Face of the grid's centre.
    pub face: u8,
    /// East position of the grid's centre, tiles.
    pub x: f32,
    /// South position of the grid's centre, tiles.
    pub y: f32,
    /// The way the grid faces, degrees clockwise from the positive x axis of its face.
    pub facing_deg: f32,
    /// Slots along a side of the square.
    pub side: u8,
    /// Tiles between neighbouring slots.
    pub spacing: f32,
    /// Tiles above the ground.
    pub hover: f32,
    /// The slots holding a drone: bit k for slot k.
    pub ready: u64,
    /// The slots whose drone left since the snapshot before (set while blending): where a
    /// drone that dives now starts.
    pub left: u64,
    /// The drone flying to its slot: the slot, its face and its east and south position, tiles.
    pub joining: Option<(u8, u8, f32, f32)>,
}

impl CloudView {
    /// Drones drawn in the grid: in their slots and on their way to one.
    pub fn drones(&self) -> u32 {
        self.ready.count_ones() + u32::from(self.joining.is_some())
    }

    /// Where slot `slot` lies from the centre, tiles `(ahead, right)` of the way the grid faces:
    /// rows from the front, each from the left (the simulation's `DroneCloud::slot_offset`).
    pub fn slot_offset(&self, slot: u32) -> (f32, f32) {
        let side = u32::from(self.side.max(1));
        let (row, column) = ((slot / side) as f32, (slot % side) as f32);
        let last = (side - 1) as f32;
        (
            (last - 2.0 * row) * self.spacing / 2.0,
            (2.0 * column - last) * self.spacing / 2.0,
        )
    }

    /// Slot `slot`'s offset from the centre along the face's x and y axes, tiles.
    pub fn slot_delta(&self, slot: u32) -> (f32, f32) {
        let (ahead, right) = self.slot_offset(slot);
        let (sin, cos) = self.facing_deg.to_radians().sin_cos();
        (ahead * cos - right * sin, ahead * sin + right * cos)
    }
}

/// One entity as the render thread sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityView {
    /// Stable id; snapshots are matched by it.
    pub id: u32,
    /// Entity type id, used to vary building heights.
    pub kind: u16,
    /// Owner; 0 is neutral.
    pub owner: u8,
    /// Face of a sphere planet; 0 on a flat map.
    pub face: u8,
    /// Position in tiles (east), local to the face.
    pub x: f32,
    /// Position in tiles (south).
    pub y: f32,
    /// Orbital height in tiles above sea level; zero places the entity on the surface.
    pub altitude: f32,
    /// Its latest shot while the projectile may still be in flight (task 95).
    pub shot: Option<crate::projectiles::ShotView>,
    /// A ship that flies through space: drawn as a hull in orbit and when it has landed.
    pub spacecraft: bool,
    /// An aircraft's share of its flight height (task 123): 1 airborne, 0 landed (and for
    /// everything that is not an aircraft), between while it takes off or lands.
    pub flight: f32,
    /// An aircraft that lands and takes off (task 123): placed by [`lift_aircraft`] with its
    /// model's underside on the ground or at its flight height.
    pub aircraft: bool,
    /// How an airborne aircraft holds a point (task 119): jets circle when idle, rotorcraft hover.
    pub hover: Hover,
    /// A drone operator's cloud (task 120).
    pub cloud: Option<CloudView>,
    /// Facing in degrees, clockwise from the positive x axis.
    pub facing_deg: f32,
    /// Hit points as a fraction of the maximum, 0 to 1.
    pub hp_frac: f32,
    /// True for buildings (drawn as boxes covering the footprint).
    pub building: bool,
    /// Footprint in tiles.
    pub footprint: (u8, u8),
    /// Collision radius in tiles.
    pub radius: f32,
    /// True when this unit contains a carried node.
    pub carrying: bool,
    /// Number of nodes in this keeper.
    pub kept: u8,
    /// Keeper capacity, zero for other entities.
    pub capacity: u8,
    /// Units aboard an own carrier, drawn as pips above it (task 167); zero otherwise.
    pub aboard: u8,
    /// A unit swimming shallow water (task 168): drawn lowered to the water surface, waist-deep.
    pub swimming: bool,
    /// Tiles above the ground of a unit in the air after a jump or a drop (task 169); 0 on it.
    pub lift: f32,
    /// A unit falling from orbit in a drop pod (task 169): the share of its fall done, above 0
    /// and up to 1; 0 for everything else. The pod streaks down with a re-entry trail.
    pub pod: f32,
    /// A building in the air (task 170): drawn with lift engines under it, firing.
    pub engines: bool,
    /// A carry-mode node drawn as a gold-capped plinth.
    pub carry_node: bool,
    /// Construction progress 0 to 1 for a site, `None` when complete.
    pub under_construction: Option<f32>,
    /// The composed model it is drawn with; `None` keeps the plain boxes.
    pub model: Option<ModelRef>,
    /// What it is doing, for its motion.
    pub activity: Activity,
}

/// A composed model an entity is drawn with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelRef {
    /// Index into [`crate::SceneLooks::models`].
    pub index: u16,
    /// Extent of the model in tiles, around its origin: across (x), height (y) and deep (z).
    /// Picking, health bars and selection use it.
    pub size: [f32; 3],
    /// Height in tiles of the model's lowest point above its origin (0 when it reaches the
    /// origin or below): an aircraft model is built flying (task 123).
    pub floor: f32,
}

/// All entities at one simulation tick, ordered by id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EntitySnapshot {
    /// Simulation tick the snapshot was taken at.
    pub tick: u64,
    /// The entities, ascending by id.
    pub entities: Vec<EntityView>,
}

impl EntitySnapshot {
    /// True when the entities are in strictly ascending id order.
    fn is_sorted(&self) -> bool {
        self.entities.windows(2).all(|pair| pair[0].id < pair[1].id)
    }
}

/// The two newest snapshots and when the newer one arrived.
#[derive(Clone, Debug, Default)]
pub struct SnapshotPair {
    previous: Option<Arc<EntitySnapshot>>,
    latest: Option<Arc<EntitySnapshot>>,
    arrived: Option<Instant>,
    interval: Option<Duration>,
}

impl SnapshotPair {
    /// Adds a snapshot that arrived at `at`; the interval estimate follows the arrival gaps.
    pub fn push(&mut self, snapshot: Arc<EntitySnapshot>, at: Instant) {
        let snapshot = if snapshot.is_sorted() {
            snapshot
        } else {
            let mut sorted = (*snapshot).clone();
            sorted.entities.sort_by_key(|view| view.id);
            sorted.entities.dedup_by_key(|view| view.id);
            Arc::new(sorted)
        };
        if let Some(arrived) = self.arrived {
            let gap = at.saturating_duration_since(arrived);
            if gap >= MIN_GAP {
                let gap = gap.min(MAX_INTERVAL);
                self.interval = Some(match self.interval {
                    Some(old) => old.mul_f32(0.7) + gap.mul_f32(0.3),
                    None => gap,
                });
            }
        }
        self.previous = self.latest.take();
        self.latest = Some(snapshot);
        self.arrived = Some(at);
    }

    /// Tick of the newest snapshot.
    pub fn latest_tick(&self) -> Option<u64> {
        self.latest.as_ref().map(|snapshot| snapshot.tick)
    }

    /// Time the blend takes to go from the older to the newer snapshot.
    pub fn interval(&self) -> Duration {
        self.interval.unwrap_or(DEFAULT_INTERVAL)
    }

    /// The simulation time the blend at the wall-clock time `now` shows, in ticks: the older
    /// snapshot's tick moved towards the newer one's as [`Self::blend`] moves the entities.
    pub fn render_tick(&self, now: Instant) -> f64 {
        let Some(latest) = &self.latest else {
            return 0.0;
        };
        let elapsed = self
            .arrived
            .map_or(Duration::ZERO, |at| now.saturating_duration_since(at));
        let alpha = (elapsed.as_secs_f64() / self.interval().as_secs_f64()).clamp(0.0, 1.0);
        match &self.previous {
            Some(previous) if latest.tick >= previous.tick => {
                previous.tick as f64 + (latest.tick - previous.tick) as f64 * alpha
            }
            _ => latest.tick as f64,
        }
    }

    /// The entities blended by the wall-clock time `now` (see [`Self::blend`]).
    pub fn blend_at(&self, now: Instant, out: &mut Vec<EntityView>) {
        self.blend_at_on(now, None, out);
    }

    /// Like [`Self::blend_at`], on the sphere `globe` when there is one.
    pub fn blend_at_on(&self, now: Instant, globe: Option<&Globe>, out: &mut Vec<EntityView>) {
        let elapsed = self
            .arrived
            .map_or(Duration::ZERO, |at| now.saturating_duration_since(at));
        self.blend_on(elapsed, globe, out);
    }

    /// Fills `out` with the newest entities, positions and facing moved from the older snapshot
    /// by `elapsed` of one interval (clamped to 0..=1). Entities absent from the older snapshot
    /// stay where they are.
    pub fn blend(&self, elapsed: Duration, out: &mut Vec<EntityView>) {
        self.blend_on(elapsed, None, out);
    }

    /// Like [`Self::blend`]; with a `globe`, an entity that changed face between the snapshots is
    /// moved along the sphere instead of across the faces' unrelated coordinates.
    pub fn blend_on(&self, elapsed: Duration, globe: Option<&Globe>, out: &mut Vec<EntityView>) {
        out.clear();
        let Some(latest) = &self.latest else {
            return;
        };
        let alpha = (elapsed.as_secs_f32() / self.interval().as_secs_f32()).clamp(0.0, 1.0);
        let Some(previous) = self.previous.as_ref().filter(|_| alpha < 1.0) else {
            out.extend_from_slice(&latest.entities);
            return;
        };
        out.extend(latest.entities.iter().map(|view| {
            match previous
                .entities
                .binary_search_by_key(&view.id, |old| old.id)
            {
                Ok(index) => match globe {
                    Some(globe) => interpolate_on(globe, &previous.entities[index], view, alpha),
                    None => interpolate(&previous.entities[index], view, alpha),
                },
                Err(_) => *view,
            }
        }));
    }
}

/// `from` moved towards `to` by `alpha`; only position and facing change.
pub fn interpolate(from: &EntityView, to: &EntityView, alpha: f32) -> EntityView {
    EntityView {
        x: from.x + (to.x - from.x) * alpha,
        y: from.y + (to.y - from.y) * alpha,
        facing_deg: lerp_angle(from.facing_deg, to.facing_deg, alpha),
        flight: from.flight + (to.flight - from.flight) * alpha,
        lift: from.lift + (to.lift - from.lift) * alpha,
        pod: blend_pod(from.pod, to.pod, alpha),
        cloud: blend_cloud(from.cloud, to.cloud, alpha),
        ..*to
    }
}

/// A pod's fall blended between snapshots; none once either side has none.
fn blend_pod(from: f32, to: f32, alpha: f32) -> f32 {
    if from > 0.0 && to > 0.0 {
        from + (to - from) * alpha
    } else {
        to
    }
}

/// A grid's centre and facing moved from `from` towards `to` by `alpha` on one face (its
/// joining drone too), and the slots whose drones left between them; the newer one when it
/// crossed a face edge or appeared.
fn blend_cloud(from: Option<CloudView>, to: Option<CloudView>, alpha: f32) -> Option<CloudView> {
    match (from, to) {
        (Some(a), Some(b)) if a.face == b.face => Some(CloudView {
            x: a.x + (b.x - a.x) * alpha,
            y: a.y + (b.y - a.y) * alpha,
            facing_deg: lerp_angle(a.facing_deg, b.facing_deg, alpha),
            left: a.ready & !b.ready,
            joining: match (a.joining, b.joining) {
                (Some(ja), Some(jb)) if ja.0 == jb.0 && ja.1 == jb.1 => Some((
                    jb.0,
                    jb.1,
                    ja.2 + (jb.2 - ja.2) * alpha,
                    ja.3 + (jb.3 - ja.3) * alpha,
                )),
                (_, joining) => joining,
            },
            ..b
        }),
        _ => to,
    }
}

/// [`interpolate`] on a sphere. Within a face the positions blend in face coordinates; across
/// a face edge the move follows the unfolded vector from one face to the next and the
/// facing blends as a heading on the surface, so nothing jumps where the coordinates do.
pub fn interpolate_on(globe: &Globe, from: &EntityView, to: &EntityView, alpha: f32) -> EntityView {
    if from.face == to.face {
        return interpolate(from, to, alpha);
    }
    let milli = |v: &EntityView| SurfacePos {
        face: v.face,
        x: (v.x * 1000.0).round() as i64,
        y: (v.y * 1000.0).round() as i64,
    };
    let (a, b) = (milli(from), milli(to));
    let Some((dx, dy)) = globe.surface().unfold(a, b) else {
        return *to;
    };
    let at = globe.step_on(
        a,
        (dx as f32 * alpha).round() as i64,
        (dy as f32 * alpha).round() as i64,
    );
    let (x, y) = (at.x as f32 / 1000.0, at.y as f32 / 1000.0);
    let heading = |v: &EntityView| heading_of(&globe.ground_frame(v.face, v.x, v.y), v.facing_deg);
    let (hf, ht) = (heading(from), heading(to));
    let blended: [f32; 3] = std::array::from_fn(|i| hf[i] + (ht[i] - hf[i]) * alpha);
    let frame = globe.ground_frame(at.face, x, y);
    let facing = dot3(blended, frame.ey)
        .atan2(dot3(blended, frame.ex))
        .to_degrees()
        .rem_euclid(360.0);
    EntityView {
        face: at.face,
        x,
        y,
        facing_deg: facing,
        flight: from.flight + (to.flight - from.flight) * alpha,
        lift: from.lift + (to.lift - from.lift) * alpha,
        pod: blend_pod(from.pod, to.pod, alpha),
        cloud: blend_cloud(from.cloud, to.cloud, alpha),
        ..*to
    }
}

/// Angle in degrees from `from` towards `to` along the shorter way round, in 0..360.
pub fn lerp_angle(from: f32, to: f32, alpha: f32) -> f32 {
    let delta = (to - from + 180.0).rem_euclid(360.0) - 180.0;
    (from + delta * alpha).rem_euclid(360.0)
}

/// The terrain surface units and buildings stand on, sampled exactly as the meshes are built.
#[derive(Clone, Debug)]
pub struct Surface {
    scene: Arc<TerrainScene>,
    shape: Shape,
}

impl Surface {
    /// Prepares height sampling for `scene`.
    pub fn new(scene: Arc<TerrainScene>) -> Self {
        let shape = Shape::new(&scene);
        Self { scene, shape }
    }

    /// The terrain scene this surface was built from.
    pub fn scene(&self) -> &TerrainScene {
        &self.scene
    }

    /// The per-tile elevations and colours of the scene.
    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    fn corner(&self, vx: i64, vy: i64) -> f32 {
        vertex_height(&self.scene, &self.shape, vx, vy).unwrap_or(0.0)
    }

    /// Height of the land surface at (`x`, `y`) in tiles, never below sea level.
    ///
    /// Each tile is two triangles split along the diagonal from its (1,0) to its (0,1) corner,
    /// as in the meshes; the height is interpolated on the triangle the point lies in.
    pub fn height(&self, x: f32, y: f32) -> f32 {
        let (width, height) = (self.scene.width as f32, self.scene.height as f32);
        let (x, y) = (x.clamp(0.0, width - 1e-3), y.clamp(0.0, height - 1e-3));
        let (tx, ty) = (x.floor(), y.floor());
        let (u, v) = (x - tx, y - ty);
        let (vx, vy) = (tx as i64, ty as i64);
        let h00 = self.corner(vx, vy);
        let h10 = self.corner(vx + 1, vy);
        let h01 = self.corner(vx, vy + 1);
        let h11 = self.corner(vx + 1, vy + 1);
        let surface = if u + v <= 1.0 {
            h00 + u * (h10 - h00) + v * (h01 - h00)
        } else {
            h11 + (1.0 - u) * (h01 - h11) + (1.0 - v) * (h10 - h11)
        };
        surface.max(0.0)
    }
}

/// Owner colour from the fixed palette, RGB in 0..=1; owner 0 is grey.
pub fn owner_color(owner: u8) -> [f32; 3] {
    const PALETTE: [[u8; 3]; OWNER_COLORS] = [
        [140, 140, 140],
        [220, 60, 60],
        [60, 110, 230],
        [240, 200, 50],
        [70, 200, 90],
        [190, 90, 220],
        [240, 140, 50],
        [60, 210, 220],
        [235, 235, 235],
    ];
    let [r, g, b] = PALETTE[usize::from(owner) % OWNER_COLORS];
    [
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    ]
}

fn scaled(color: [f32; 3], factor: f32) -> [f32; 3] {
    [color[0] * factor, color[1] * factor, color[2] * factor]
}

/// Column-major 4×4 matrix of a box whose axes are the unit vectors `ax`, `ay` and `az`,
/// scaled by `size`, centred at `centre`, with `color` stored in the cells affine transforms
/// leave unused.
pub fn frame_box(
    centre: [f32; 3],
    axes: [[f32; 3]; 3],
    size: [f32; 3],
    color: [f32; 3],
) -> [f32; 16] {
    let [ax, ay, az] = axes;
    [
        ax[0] * size[0],
        ax[1] * size[0],
        ax[2] * size[0],
        color[0],
        ay[0] * size[1],
        ay[1] * size[1],
        ay[2] * size[1],
        color[1],
        az[0] * size[2],
        az[1] * size[2],
        az[2] * size[2],
        color[2],
        centre[0],
        centre[1],
        centre[2],
        1.0,
    ]
}

/// A point on the ground with its local axes: `ex` east, `up` away from the ground, `ey` south.
/// `ex × up = ey`; on a flat map they are the world axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundFrame {
    /// The point on the ground (including its height).
    pub origin: [f32; 3],
    /// East along the face's x axis.
    pub ex: [f32; 3],
    /// Away from the ground.
    pub up: [f32; 3],
    /// South along the face's y axis.
    pub ey: [f32; 3],
}

impl GroundFrame {
    /// `origin + ex a + up b + ey c` for `[a, b, c]` in tiles.
    pub fn at(&self, [a, b, c]: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|i| self.origin[i] + self.ex[i] * a + self.up[i] * b + self.ey[i] * c)
    }
}

/// The surface entities stand on: a flat map or a sphere planet. Positions are face-local
/// tiles (face 0 and map tiles on a flat map).
pub trait Footing {
    /// A sampled route between surface positions, lifted above the terrain.
    fn route_point(&self, from: SurfacePos, to: SurfacePos, t: f32, lift: f32) -> [f32; 3] {
        self.point(
            from.face,
            from.x as f32 / 1000.0,
            from.y as f32 / 1000.0,
            (
                (to.x - from.x) as f32 / 1000.0 * t,
                (to.y - from.y) as f32 / 1000.0 * t,
            ),
            lift,
        )
    }
    /// A point at a fixed height above sea level, independent of the terrain below.
    fn datum_point(&self, face: u8, x: f32, y: f32, delta: (f32, f32), height: f32) -> [f32; 3] {
        self.point(
            face,
            x,
            y,
            delta,
            height - self.height_at(face, x + delta.0, y + delta.1),
        )
    }
    /// Whether the ground is a sphere centred on the origin (a bar behind it is hidden).
    fn is_round(&self) -> bool {
        false
    }
    /// Height above sea level in tiles at (`x`, `y`), never below zero.
    fn height_at(&self, face: u8, x: f32, y: f32) -> f32;
    /// The frame standing on the ground at (`x`, `y`).
    fn frame(&self, face: u8, x: f32, y: f32) -> GroundFrame;
    /// The point `lift` above the ground at (`x` + `dx`, `y` + `dy`), the offset taken along
    /// the surface (it may cross a face edge on a sphere).
    fn point(&self, face: u8, x: f32, y: f32, delta: (f32, f32), lift: f32) -> [f32; 3];
}

impl Footing for Surface {
    fn height_at(&self, _face: u8, x: f32, y: f32) -> f32 {
        self.height(x, y)
    }

    fn frame(&self, _face: u8, x: f32, y: f32) -> GroundFrame {
        GroundFrame {
            origin: [x, self.height(x, y), y],
            ex: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            ey: [0.0, 0.0, 1.0],
        }
    }

    fn point(&self, _face: u8, x: f32, y: f32, delta: (f32, f32), lift: f32) -> [f32; 3] {
        let (px, py) = (x + delta.0, y + delta.1);
        [px, self.height(px, py) + lift, py]
    }
}

/// A flat ground given by a height function, for overlays and picking.
pub struct FlatGround<'a>(pub &'a dyn Fn(f32, f32) -> f32);

impl Footing for FlatGround<'_> {
    fn height_at(&self, _face: u8, x: f32, y: f32) -> f32 {
        (self.0)(x, y)
    }

    fn frame(&self, _face: u8, x: f32, y: f32) -> GroundFrame {
        GroundFrame {
            origin: [x, (self.0)(x, y), y],
            ex: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            ey: [0.0, 0.0, 1.0],
        }
    }

    fn point(&self, _face: u8, x: f32, y: f32, delta: (f32, f32), lift: f32) -> [f32; 3] {
        let (px, py) = (x + delta.0, y + delta.1);
        [px, (self.0)(px, py) + lift, py]
    }
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let length = dot3(v, v).sqrt().max(1e-9);
    v.map(|c| c / length)
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

/// The heading of an entity facing `facing_deg` (clockwise from the face's x axis).
pub(crate) fn heading_of(frame: &GroundFrame, facing_deg: f32) -> [f32; 3] {
    let (sin, cos) = facing_deg.to_radians().sin_cos();
    std::array::from_fn(|i| frame.ex[i] * cos + frame.ey[i] * sin)
}

/// Where the lift engines of a building in the air hang (task 170): four points under its
/// underside, near the corners of a square within its footprint, along its facing; and the
/// size of each engine in tiles (an engine model is one tile across).
pub(crate) fn lift_engine_spots(view: &EntityView, frame: &GroundFrame) -> ([[f32; 3]; 4], f32) {
    let side = f32::from(view.footprint.0.min(view.footprint.1).max(1));
    let size = (side * 0.3).clamp(0.6, 2.4);
    let reach = (side / 2.0 - size * 0.55).max(size * 0.3);
    let along = heading_of(frame, view.facing_deg);
    let across = heading_of(frame, view.facing_deg + 90.0);
    let spots = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].map(|(a, b)| {
        std::array::from_fn(|i| frame.origin[i] + (along[i] * a + across[i] * b) * reach)
    });
    (spots, size)
}

/// Height of a building body in tiles; it varies with the entity type and the footprint.
pub fn building_height(view: &EntityView) -> f32 {
    if let Some(model) = view.model {
        return model.size[1].max(0.2);
    }
    let (w, h) = (f32::from(view.footprint.0), f32::from(view.footprint.1));
    1.0 + f32::from(view.kind % 4) * 0.4 + 0.15 * w.max(h)
}

/// Width of a unit body in tiles; drawn larger than the collision circle so it stays visible.
pub fn unit_width(view: &EntityView) -> f32 {
    if let Some(model) = view.model {
        return model.size[0].max(model.size[2]).max(0.6);
    }
    (view.radius * 2.0 * 1.6).max(if view.altitude > 0.0 || view.spacecraft {
        3.0
    } else {
        0.6
    })
}

/// Height of a unit body in tiles.
pub fn unit_height(view: &EntityView) -> f32 {
    if let Some(model) = view.model {
        return model.size[1].max(0.3);
    }
    if view.altitude > 0.0 || view.spacecraft {
        unit_width(view) * 0.35
    } else {
        (unit_width(view) * 1.8).max(0.9)
    }
}

/// The box that stands for `view` when picking: centre `[east, up, south]`, size and yaw in
/// degrees, matching how [`build_instances`] draws the entity. `ground` samples the surface.
pub fn pick_box(view: &EntityView, ground: &dyn Fn(f32, f32) -> f32) -> ([f32; 3], [f32; 3], f32) {
    let base = if view.altitude > 0.0 {
        view.altitude
    } else {
        ground(view.x, view.y)
    };
    if view.building {
        let (w, h) = (
            f32::from(view.footprint.0.max(1)),
            f32::from(view.footprint.1.max(1)),
        );
        let progress = view.under_construction.map_or(1.0, |p| p.clamp(0.0, 1.0));
        let body = building_height(view) * progress.max(0.12) + 0.2;
        ([view.x, base + body / 2.0, view.y], [w, body, h], 0.0)
    } else if view.carry_node {
        (
            [view.x, base + 0.54, view.y],
            [1.0, 1.08, 1.0],
            view.facing_deg,
        )
    } else if view.owner == 0 {
        (
            [view.x, base + 0.35, view.y],
            [0.7, 0.7, 0.7],
            view.facing_deg,
        )
    } else {
        let (width, height) = (unit_width(view), unit_height(view));
        (
            [view.x, base + height / 2.0, view.y],
            [width, height, width],
            view.facing_deg,
        )
    }
}

/// [`pick_box`] on any ground: the centre, the box axes and the size.
pub fn pick_frame(view: &EntityView, ground: &dyn Footing) -> ([f32; 3], [[f32; 3]; 3], [f32; 3]) {
    let frame = entity_frame(view, ground);
    let (offset, size, yaw) = {
        let (centre, size, yaw) = pick_box(view, &|_, _| 0.0);
        ([0.0, centre[1] - view.altitude, 0.0], size, yaw)
    };
    let forward = heading_of(&frame, yaw);
    let axes = [forward, frame.up, cross3(forward, frame.up)];
    (frame.at(offset), axes, size)
}

/// The surface frame for a ground entity or a constant-radius frame for an orbital one.
pub(crate) fn entity_frame(view: &EntityView, ground: &dyn Footing) -> GroundFrame {
    let frame = ground.frame(view.face, view.x, view.y);
    // An aircraft's origin is where `lift_aircraft` put it, under the ground for a model built
    // flying (task 123).
    if view.aircraft {
        return GroundFrame {
            origin: ground.datum_point(view.face, view.x, view.y, (0.0, 0.0), view.altitude),
            ..frame
        };
    }
    if view.altitude > 0.0 || (view.spacecraft && !view.building) {
        // A ship never sinks below the ground it lands on; a big one stands on the highest
        // ground under its hull.
        let height = view.altitude.max(ship_floor(view, ground));
        GroundFrame {
            origin: ground.datum_point(view.face, view.x, view.y, (0.0, 0.0), height),
            ..frame
        }
    } else if view.lift > 0.0 {
        // In the air after a jump or a drop (task 169): above the ground below it.
        GroundFrame {
            origin: std::array::from_fn(|k| frame.origin[k] + frame.up[k] * view.lift),
            ..frame
        }
    } else if view.swimming {
        // Waist-deep (task 168): the model sinks below the water surface it stands on.
        let depth = SWIM_DEPTH * unit_height(view);
        GroundFrame {
            origin: std::array::from_fn(|k| frame.origin[k] - frame.up[k] * depth),
            ..frame
        }
    } else {
        frame
    }
}

/// Share of a swimming unit's height drawn below the water surface (task 168).
pub const SWIM_DEPTH: f32 = 0.45;

/// The highest ground under a ship's hull: at its point and on a ring of its half length.
fn ship_floor(view: &EntityView, ground: &dyn Footing) -> f32 {
    let here = ground.height_at(view.face, view.x, view.y);
    let reach = unit_width(view) * 0.5;
    if !view.spacecraft || reach < 1.0 {
        return here;
    }
    (0..8)
        .flat_map(|k| {
            let (sin, cos) = (k as f32 * std::f32::consts::FRAC_PI_4).sin_cos();
            [0.5, 1.0].map(|r| (cos * reach * r, sin * reach * r))
        })
        .map(|(dx, dy)| ground.height_at(view.face, view.x + dx, view.y + dy))
        .fold(here, f32::max)
}

/// The eight corners of a box from [`pick_box`].
pub fn box_corners(centre: [f32; 3], size: [f32; 3], yaw_deg: f32) -> [[f32; 3]; 8] {
    let (sin, cos) = yaw_deg.to_radians().sin_cos();
    box_corners_axes(
        centre,
        [[cos, 0.0, sin], [0.0, 1.0, 0.0], [-sin, 0.0, cos]],
        size,
    )
}

/// The eight corners of the box with `axes` and `size` around `centre`.
pub fn box_corners_axes(centre: [f32; 3], axes: [[f32; 3]; 3], size: [f32; 3]) -> [[f32; 3]; 8] {
    let mut corners = [[0.0; 3]; 8];
    for (i, corner) in corners.iter_mut().enumerate() {
        let sx = if i & 1 == 0 { -0.5 } else { 0.5 } * size[0];
        let sy = if i & 2 == 0 { -0.5 } else { 0.5 } * size[1];
        let sz = if i & 4 == 0 { -0.5 } else { 0.5 } * size[2];
        *corner = std::array::from_fn(|k| {
            centre[k] + axes[0][k] * sx + axes[1][k] * sy + axes[2][k] * sz
        });
    }
    corners
}

/// Appends the instances that draw `views` on a flat map.
pub fn build_instances(views: &[EntityView], surface: &Surface, out: &mut Vec<[f32; 16]>) {
    build_instances_on(views, surface, out);
}

/// Appends the instances that draw `views` standing on `ground`.
pub fn build_instances_on(views: &[EntityView], ground: &dyn Footing, out: &mut Vec<[f32; 16]>) {
    instances_on(views, ground, out, None, None, &|_| false);
}

/// Builds display instances and records which ones are bodies that cast shadows. Buildings
/// whose type `drawn_as_objects` get only a ground plate and a marker post; their look is
/// drawn by the feature objects (crop rows). `anchors` gets where each entity's health bar
/// hangs; the bars themselves are drawn on the 2D batch, so no instance is a bar.
pub(crate) fn build_instances_with_casters_on(
    views: &[EntityView],
    ground: &dyn Footing,
    out: &mut Vec<[f32; 16]>,
    casters: &mut Vec<std::ops::Range<usize>>,
    anchors: &mut Vec<BarAnchor>,
    drawn_as_objects: &dyn Fn(u16) -> bool,
) {
    casters.clear();
    anchors.clear();
    instances_on(
        views,
        ground,
        out,
        Some(casters),
        Some(anchors),
        drawn_as_objects,
    );
}

fn instances_on(
    views: &[EntityView],
    ground: &dyn Footing,
    out: &mut Vec<[f32; 16]>,
    mut casters: Option<&mut Vec<std::ops::Range<usize>>>,
    mut anchors: Option<&mut Vec<BarAnchor>>,
    drawn_as_objects: &dyn Fn(u16) -> bool,
) {
    out.clear();
    for view in views {
        let frame = entity_frame(view, ground);
        let base = ground.height_at(view.face, view.x, view.y);
        let body_start = out.len();
        let top = if let Some(model) = view.model {
            push_model_base(view, &frame, model, out)
        } else if view.building && drawn_as_objects(view.kind) {
            push_plot(view, &frame, out)
        } else if view.building {
            push_building(view, ground, &frame, base, out)
        } else if view.owner == 0 || view.carry_node {
            push_node(view, &frame, out)
        } else if view.altitude > 0.0 || view.spacecraft {
            push_orbital_ship(view, &frame, out)
        } else {
            push_unit(view, &frame, out)
        };
        if let Some(casters) = casters.as_mut() {
            casters.push(body_start..out.len());
        }
        let axes = [frame.ex, frame.up, frame.ey];
        if view.carrying {
            out.push(frame_box(
                frame.at([0.0, top + 0.8, 0.0]),
                axes,
                [0.28, 0.28, 0.28],
                NODE_COLOR,
            ));
        }
        for i in 0..view.kept.min(view.capacity) {
            let across = (f32::from(i) - f32::from(view.kept.min(view.capacity) - 1) / 2.0) * 0.38;
            out.push(frame_box(
                frame.at([across, top + 0.22, 0.0]),
                axes,
                [0.22, 0.22, 0.22],
                NODE_COLOR,
            ));
        }
        // Units aboard: rows of up to `ABOARD_ROW` pips over the hull, nearest row first.
        for i in 0..view.aboard {
            let (row, column) = (i / ABOARD_ROW, i % ABOARD_ROW);
            let in_row = (view.aboard - row * ABOARD_ROW).min(ABOARD_ROW);
            let across = (f32::from(column) - f32::from(in_row - 1) / 2.0) * 0.3;
            out.push(frame_box(
                frame.at([across, top + 0.75 + f32::from(row) * 0.3, 0.0]),
                axes,
                [0.18, 0.18, 0.18],
                ABOARD_COLOR,
            ));
        }
        if let Some(anchors) = anchors.as_mut() {
            let width = if view.building {
                f32::from(view.footprint.0.max(view.footprint.1)).max(1.0)
            } else {
                (view.radius * 4.0).max(0.8)
            };
            let ground_radius = ground.is_round().then(|| {
                let floor = if view.altitude > 0.0
                    || view.aircraft
                    || (view.spacecraft && !view.building)
                {
                    ground.frame(view.face, view.x, view.y).origin
                } else {
                    frame.origin
                };
                dot3(floor, floor).sqrt()
            });
            anchors.push(BarAnchor {
                id: view.id,
                at: frame.at([0.0, top + 0.4, 0.0]),
                width,
                frac: view.hp_frac.clamp(0.0, 1.0),
                ground_radius,
            });
        }
    }
}

/// Pushes a unit body and facing wedge; returns the height of the unit's top above the ground.
fn push_unit(view: &EntityView, frame: &GroundFrame, out: &mut Vec<[f32; 16]>) -> f32 {
    let color = owner_color(view.owner);
    let width = unit_width(view);
    let height = unit_height(view);
    let forward = heading_of(frame, view.facing_deg);
    let axes = [forward, frame.up, cross3(forward, frame.up)];
    out.push(frame_box(
        frame.at([0.0, height / 2.0, 0.0]),
        axes,
        [width, height, width],
        color,
    ));
    let reach = width * 0.5;
    let mut wedge = frame.at([0.0, height * 0.62, 0.0]);
    for (c, f) in wedge.iter_mut().zip(forward) {
        *c += f * reach;
    }
    out.push(frame_box(
        wedge,
        axes,
        [width * 0.55, height * 0.26, width * 0.4],
        scaled(color, 0.45),
    ));
    height
}

/// A compact hull, swept wings and twin engines for ships without a composed model.
fn push_orbital_ship(view: &EntityView, frame: &GroundFrame, out: &mut Vec<[f32; 16]>) -> f32 {
    let height = unit_height(view);
    let hull = Hull {
        origin: frame.origin,
        forward: heading_of(frame, view.facing_deg),
        up: frame.up,
        length: unit_width(view),
        height,
    };
    hull.push(owner_color(view.owner), out);
    height
}

/// The hull of a ship without a composed model, placed in any frame: the same model above a
/// planet and on a star system's plane; only its size differs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hull {
    /// The point under the hull.
    pub(crate) origin: [f32; 3],
    /// Unit direction the bow points to.
    pub(crate) forward: [f32; 3],
    /// Unit up.
    pub(crate) up: [f32; 3],
    /// Length from stern to bow.
    pub(crate) length: f32,
    /// Height of the hull.
    pub(crate) height: f32,
}

impl Hull {
    /// Height of a hull of `length`, in the proportions it has above a planet.
    pub(crate) fn height_of(length: f32) -> f32 {
        length * 0.35
    }

    /// Pushes the hull's boxes, its trim in the owner's colour `team`.
    pub(crate) fn push(&self, team: [f32; 3], out: &mut Vec<[f32; 16]>) {
        let (width, height, forward, up) = (self.length, self.height, self.forward, self.up);
        let side = cross3(forward, up);
        let axes = [forward, up, side];
        let at = |along: f32, rise: f32, across: f32| -> [f32; 3] {
            std::array::from_fn(|i| {
                self.origin[i] + forward[i] * along + up[i] * rise + side[i] * across
            })
        };
        out.push(frame_box(
            at(0.0, height * 0.5, 0.0),
            axes,
            [width, height * 0.55, width * 0.3],
            [0.58, 0.64, 0.72],
        ));
        out.push(frame_box(
            at(width * 0.25, height * 0.66, 0.0),
            axes,
            [width * 0.4, height * 0.45, width * 0.2],
            team,
        ));
        out.push(frame_box(
            at(-width * 0.18, height * 0.3, 0.0),
            axes,
            [width * 0.36, height * 0.18, width],
            team,
        ));
        for sign in [-1.0, 1.0] {
            out.push(frame_box(
                at(-width * 0.31, height * 0.3, sign * width * 0.35),
                axes,
                [width * 0.35, height * 0.4, width * 0.18],
                [0.22, 0.27, 0.34],
            ));
            out.push(frame_box(
                at(-width * 0.5, height * 0.3, sign * width * 0.35),
                axes,
                [width * 0.035, height * 0.28, width * 0.12],
                [0.2, 0.8, 1.0],
            ));
        }
    }
}

/// Makes a ship drawn `factor` times its size: a composed model is drawn at its `size` (see
/// `ModelGpu::place`), a ship without one from its radius, so the drawn ship, picking, its
/// health bar and its hull length all follow.
pub(crate) fn scale_ship_view(view: &mut EntityView, factor: f32) {
    if let Some(model) = &mut view.model {
        model.size = model.size.map(|c| c * factor);
    } else {
        view.radius = unit_width(view) * factor / 3.2;
    }
}

/// Dresses the ships among `views` (ascending by id) for the globe `globe` (task 72): each is
/// drawn at its type's planet scale (in orbit and, for a starship, on the ground), holds at the
/// height its drawn length gives it ([`hover_altitude`], bigger ships higher), raised further
/// where a smaller ship of another class holds under it, and ships that would still overlap
/// (of one class over one point) are spread sideways by their size. Presentation only.
pub(crate) fn dress_ships(views: &mut [EntityView], scales: &ShipScales, globe: &Globe) {
    let mut aloft = Vec::new();
    for (i, view) in views.iter_mut().enumerate() {
        if !view.spacecraft {
            continue;
        }
        let nominal = view.altitude;
        scale_ship_view(view, planet_factor(scales.of(view.kind), nominal));
        if nominal > 0.0 {
            aloft.push((i, (nominal / ORBIT_ALTITUDE).clamp(0.0, 1.0), nominal));
        }
    }
    if aloft.is_empty() {
        return;
    }
    let radius = globe.radius();
    let points: Vec<[f32; 3]> = aloft
        .iter()
        .map(|&(i, _, _)| {
            let v = &views[i];
            let pos = SurfacePos {
                face: v.face,
                x: (v.x * 1000.0).round() as i64,
                y: (v.y * 1000.0).round() as i64,
            };
            globe.dir_of(pos).map(|c| c * radius)
        })
        .collect();
    let heights = stack_by_size(views, &aloft, &points);
    for (n, &(i, rise, nominal)) in aloft.iter().enumerate() {
        views[i].altitude = (rise * heights[n]).max(nominal.min(0.05));
    }
    spread_aloft(views, &aloft, &points, globe);
}

/// Keeps the ships aloft among `views` (dressed by [`dress_ships`]) at least their floor on
/// screen (task 96), seen from `eye` (planet frame) on a canvas `height` pixels high: a ship
/// that would look shorter than [`crate::layers::SHIP_FLOOR_PIXELS`] times its class is drawn
/// at that length, so it stays seen and its pick box with it. Places and heights stay as they
/// were. Ships on the ground keep their size.
pub(crate) fn floor_ships(
    views: &mut [EntityView],
    scales: &ShipScales,
    globe: &Globe,
    eye: [f32; 3],
    height: f32,
) {
    let tan = (crate::hud_minimap::FOV_DEGREES.to_radians() / 2.0).tan();
    let radius = globe.radius();
    for view in views.iter_mut() {
        if !view.spacecraft || view.altitude <= 0.0 {
            continue;
        }
        let dir = globe.dir_of(SurfacePos {
            face: view.face,
            x: (view.x * 1000.0).round() as i64,
            y: (view.y * 1000.0).round() as i64,
        });
        let at = dir.map(|c| c * (radius + view.altitude));
        let d: [f32; 3] = std::array::from_fn(|i| at[i] - eye[i]);
        let per_pixel = dot3(d, d).sqrt() * 2.0 * tan / height.max(1.0);
        let length = unit_width(view);
        let drawn = ship_on_screen(length, per_pixel, scales.class(view.kind));
        let factor = drawn.pixels * per_pixel / length.max(1e-6);
        if factor > 1.0 {
            scale_ship_view(view, factor);
        }
    }
}

/// The hold heights of the ships `aloft` over the points `points`: each its
/// [`hover_altitude`], taken smallest first, raised above any smaller ship of another class
/// whose hull it would meet (nearer than [`SPREAD_SPACING`] of their mean length, overlapping
/// in height), so a mixed fleet over one point stacks bigger above smaller.
fn stack_by_size(
    views: &[EntityView],
    aloft: &[(usize, f32, f32)],
    points: &[[f32; 3]],
) -> Vec<f32> {
    const GAP: f32 = 0.5;
    let sizes: Vec<(f32, f32)> = aloft
        .iter()
        .map(|&(i, _, _)| (unit_width(&views[i]), unit_height(&views[i])))
        .collect();
    let mut heights: Vec<f32> = sizes.iter().map(|(w, _)| hover_altitude(*w)).collect();
    let mut order: Vec<usize> = (0..aloft.len()).collect();
    order.sort_by(|a, b| sizes[*a].0.total_cmp(&sizes[*b].0).then(a.cmp(b)));
    for (rank, &a) in order.iter().enumerate() {
        // Each raise can meet another ship lower down the order: settle in a few rounds.
        for _ in 0..rank {
            let mut raised = false;
            for &b in &order[..rank] {
                let ((wa, ha), (wb, hb)) = (sizes[a], sizes[b]);
                let d: [f32; 3] = std::array::from_fn(|k| points[a][k] - points[b][k]);
                let near = dot3(d, d).sqrt() < (wa + wb) * 0.5 * SPREAD_SPACING;
                let (za, zb) = (heights[a], heights[b]);
                let meets = za < zb + hb && zb < za + ha;
                if views[aloft[a].0].kind != views[aloft[b].0].kind && near && meets {
                    heights[a] = zb + hb + GAP;
                    raised = true;
                }
            }
            if !raised {
                break;
            }
        }
    }
    heights
}

/// Pushes apart the ships `aloft` (indices into `views`, over `points`) whose hulls would still
/// overlap: two ships nearer than [`SPREAD_SPACING`] of the mean of their lengths, at heights
/// that meet, each move half the overlap away from the other, a few rounds over (ships over the
/// very same point first part along directions of their own, by their order in `aloft`). The
/// push fades to nothing as they part.
fn spread_aloft(
    views: &mut [EntityView],
    aloft: &[(usize, f32, f32)],
    start: &[[f32; 3]],
    globe: &Globe,
) {
    const GOLDEN: f32 = 2.399_963;
    const ROUNDS: usize = 4;
    if aloft.len() < 2 {
        return;
    }
    let radius = globe.radius();
    let mut points = start.to_vec();
    let sizes: Vec<(f32, f32, f32)> = aloft
        .iter()
        .map(|&(i, _, _)| {
            let v = &views[i];
            (unit_width(v), unit_height(v), v.altitude)
        })
        .collect();
    for _ in 0..ROUNDS {
        let mut pushes = vec![[0.0_f32; 3]; aloft.len()];
        let mut moved = false;
        for a in 0..aloft.len() {
            for b in 0..aloft.len() {
                let ((wa, ha, za), (wb, hb, zb)) = (sizes[a], sizes[b]);
                if a == b || za >= zb + hb || zb >= za + ha {
                    continue;
                }
                let apart = (wa + wb) * 0.5 * SPREAD_SPACING;
                let d: [f32; 3] = std::array::from_fn(|k| points[a][k] - points[b][k]);
                let distance = dot3(d, d).sqrt();
                if distance >= apart {
                    continue;
                }
                let away = if distance > apart * 0.02 {
                    d.map(|c| c / distance)
                } else {
                    // Over the same point: a direction of its own along the ground.
                    let v = &views[aloft[a].0];
                    let frame = globe.ground_frame(v.face, v.x, v.y);
                    let (sin, cos) = (a as f32 * GOLDEN).sin_cos();
                    std::array::from_fn(|k| frame.ex[k] * cos + frame.ey[k] * sin)
                };
                let by = (apart - distance) * 0.5;
                for k in 0..3 {
                    pushes[a][k] += away[k] * by;
                }
                moved = true;
            }
        }
        if !moved {
            break;
        }
        for (point, push) in points.iter_mut().zip(&pushes) {
            let p: [f32; 3] = std::array::from_fn(|k| point[k] + push[k]);
            *point = normalize3(p).map(|c| c * radius);
        }
    }
    for (n, &(i, _, _)) in aloft.iter().enumerate() {
        if points[n] == start[n] {
            continue;
        }
        let pos = globe.pos_of(normalize3(points[n]));
        let view = &mut views[i];
        view.face = pos.face;
        view.x = pos.x as f32 / 1000.0;
        view.y = pos.y as f32 / 1000.0;
    }
}

/// How far apart ships at one height are kept, as a share of the mean of their lengths (they are
/// slender: about two fifths as wide as long).
const SPREAD_SPACING: f32 = 0.45;

/// Height above the ground at which aircraft fly over a planet, in tiles (task 123): about the
/// height of the orbital craft over a planet ([`ORBIT_ALTITUDE`]), well above the rooftops; they
/// rise over buildings taller than that.
pub const FLIGHT_ALTITUDE: f32 = 7.0;
/// How much higher an aircraft flies per tile of its drawn length: bigger aircraft higher.
pub const FLIGHT_PER_TILE: f32 = 0.5;
/// Share of the usual flight height at which a low-flying craft hangs (task 119).
pub const LOW_FLIGHT_SHARE: f32 = 0.45;
/// How far above a building the aircraft keep, tiles.
const FLIGHT_CLEARANCE: f32 = 1.5;
/// How far from a tall building's footprint an aircraft begins to rise over it, tiles.
const FLIGHT_ROOF_REACH: f32 = 2.0;
/// Radius of the circle an idle aircraft is drawn flying round its spot, tiles.
const CIRCLE_RADIUS: f32 = 1.2;
/// Seconds an idle aircraft takes to fly once round its spot.
const CIRCLE_SECONDS: f32 = 10.0;

/// Lifts the aircraft among `views` into the air above `ground` (task 123): an airborne one
/// flies [`FLIGHT_ALTITUDE`] and [`FLIGHT_PER_TILE`] of its length over the highest ground
/// under it, rising smoothly over a taller building it nears (to [`FLIGHT_CLEARANCE`] above its
/// roof), one taking off or landing
/// at its share of that height (eased), a landed one stands on the ground; the height is that of
/// the model's underside (a model built flying is lowered by its [`ModelRef::floor`]). An idle airborne one
/// is drawn circling its spot, `seconds` into the match, while the circle stays on its face
/// (`face_side` tiles across; `None` on a flat map, where it does not circle). Sets
/// [`EntityView::altitude`], the height above sea level the drawing, picking and shots use.
/// Presentation only.
pub(crate) fn lift_aircraft(
    views: &mut [EntityView],
    ground: &dyn Footing,
    seconds: f32,
    face_side: Option<f32>,
) {
    if !views.iter().any(|v| v.aircraft) {
        return;
    }
    // The buildings an aircraft must rise over: face, centre, half its footprint, height.
    let tall: Vec<(u8, f32, f32, f32, f32)> = views
        .iter()
        .filter(|v| {
            v.building && building_height(v) + FLIGHT_CLEARANCE > FLIGHT_ALTITUDE * LOW_FLIGHT_SHARE
        })
        .map(|v| {
            let half = f32::from(v.footprint.0.max(v.footprint.1)) * 0.5;
            (
                v.face,
                v.x,
                v.y,
                half,
                building_height(v) + FLIGHT_CLEARANCE,
            )
        })
        .collect();
    for view in views.iter_mut() {
        if !view.aircraft || view.building || view.spacecraft {
            continue;
        }
        let length = unit_width(view);
        let share = if view.hover == Hover::Low {
            LOW_FLIGHT_SHARE
        } else {
            1.0
        };
        let cruise = FLIGHT_ALTITUDE * share;
        if view.flight >= 1.0
            && view.hover == Hover::Circle
            && view.activity == Activity::Idle
            && let Some(side) = face_side
        {
            let phase = (view.id % 97) as f32 / 97.0;
            let turn = (seconds / CIRCLE_SECONDS + phase) * std::f32::consts::TAU;
            let (sin, cos) = turn.sin_cos();
            let (x, y) = (view.x + CIRCLE_RADIUS * cos, view.y + CIRCLE_RADIUS * sin);
            if (0.5..side - 0.5).contains(&x) && (0.5..side - 0.5).contains(&y) {
                view.x = x;
                view.y = y;
                view.facing_deg = (turn.to_degrees() + 90.0).rem_euclid(360.0);
            }
        }
        let reach = length * 0.5;
        let floor = (0..8)
            .map(|k| {
                let (sin, cos) = (k as f32 * std::f32::consts::FRAC_PI_4).sin_cos();
                ground.height_at(view.face, view.x + cos * reach, view.y + sin * reach)
            })
            .fold(ground.height_at(view.face, view.x, view.y), f32::max);
        let base = tall
            .iter()
            .filter(|t| t.0 == view.face)
            .map(|&(_, x, y, half, top)| {
                let off = ((view.x - x).abs().max((view.y - y).abs()) - half).max(0.0);
                let near = 1.0 - (off / FLIGHT_ROOF_REACH).min(1.0);
                cruise + (top - cruise).max(0.0) * crate::layers::smoothstep(near)
            })
            .fold(cruise, f32::max);
        let rise = crate::layers::smoothstep(view.flight.clamp(0.0, 1.0));
        // The model's underside at that height (on the ground when landed).
        let built = view.model.map_or(0.0, |m| m.floor);
        view.altitude = floor + (base + FLIGHT_PER_TILE * length * share) * rise - built;
    }
}

/// Whether `view` is a ship above the ground: in orbit, landing or lifting off.
pub(crate) fn is_aloft_ship(view: &EntityView) -> bool {
    view.spacecraft && view.altitude > 0.0
}

/// The hulls of the ships above the ground among `views`, where the globe `ground` draws them,
/// with their entity ids.
pub(crate) fn aloft_hulls(views: &[EntityView], ground: &dyn Footing) -> Vec<(u32, Hull)> {
    views
        .iter()
        .filter(|v| is_aloft_ship(v))
        .map(|view| {
            let frame = entity_frame(view, ground);
            let hull = Hull {
                origin: frame.origin,
                forward: heading_of(&frame, view.facing_deg),
                up: frame.up,
                length: unit_width(view),
                height: unit_height(view),
            };
            (view.id, hull)
        })
        .collect()
}

/// Pushes a building body and roof; returns the height of its roof above the ground.
fn push_building(
    view: &EntityView,
    ground: &dyn Footing,
    frame: &GroundFrame,
    base: f32,
    out: &mut Vec<[f32; 16]>,
) -> f32 {
    let (w, h) = (
        f32::from(view.footprint.0.max(1)),
        f32::from(view.footprint.1.max(1)),
    );
    let (half_w, half_h) = (w / 2.0, h / 2.0);
    let (mut low, mut high) = (f32::MAX, 0.0_f32);
    for (dx, dy) in [
        (-half_w, -half_h),
        (half_w, -half_h),
        (-half_w, half_h),
        (half_w, half_h),
        (0.0, 0.0),
    ] {
        let at = ground.height_at(view.face, view.x + dx * 0.98, view.y + dy * 0.98);
        low = low.min(at);
        high = high.max(at);
    }
    let progress = view.under_construction.map_or(1.0, |p| p.clamp(0.0, 1.0));
    let body = building_height(view) * progress.max(0.12);
    let color = owner_color(view.owner);
    let roof_thickness = 0.18;
    let axes = [frame.ex, frame.up, frame.ey];
    // The base reaches down to the lowest corner so slopes do not leave gaps.
    let bottom = low - 0.05 - base;
    let top = high - base + body;
    out.push(frame_box(
        frame.at([0.0, (bottom + top) / 2.0, 0.0]),
        axes,
        [w * 0.94, top - bottom, h * 0.94],
        scaled(
            color,
            if view.under_construction.is_some() {
                0.8
            } else {
                0.62
            },
        ),
    ));
    out.push(frame_box(
        frame.at([0.0, top + roof_thickness / 2.0, 0.0]),
        axes,
        [w, roof_thickness, h],
        color,
    ));
    top + roof_thickness
}

/// Pushes what an entity drawn as a composed model needs besides the model: the foundation
/// plate of a building site. Returns the height of the model's top above the ground (of the
/// part raised so far on a site).
fn push_model_base(
    view: &EntityView,
    frame: &GroundFrame,
    model: ModelRef,
    out: &mut Vec<[f32; 16]>,
) -> f32 {
    const FOUNDATION: [f32; 3] = [0.55, 0.52, 0.47];
    let Some(progress) = view.under_construction else {
        return model.size[1];
    };
    let (w, h) = (
        f32::from(view.footprint.0.max(1)),
        f32::from(view.footprint.1.max(1)),
    );
    out.push(frame_box(
        frame.at([0.0, 0.0, 0.0]),
        [frame.ex, frame.up, frame.ey],
        [w * 0.94, 0.12, h * 0.94],
        FOUNDATION,
    ));
    (model.size[1] * progress.clamp(0.0, 1.0)).max(0.12)
}

/// Pushes the plate and corner post of a building drawn as objects (a field): the soil under
/// its rows and its owner's colour; returns the height of the post.
fn push_plot(view: &EntityView, frame: &GroundFrame, out: &mut Vec<[f32; 16]>) -> f32 {
    const SOIL: [f32; 3] = [0.34, 0.25, 0.16];
    const POST: f32 = 0.7;
    let (w, h) = (
        f32::from(view.footprint.0.max(1)),
        f32::from(view.footprint.1.max(1)),
    );
    let axes = [frame.ex, frame.up, frame.ey];
    let color = owner_color(view.owner);
    let soil: [f32; 3] = std::array::from_fn(|i| SOIL[i] * 0.8 + color[i] * 0.08);
    out.push(frame_box(
        frame.at([0.0, -0.03, 0.0]),
        axes,
        [w * 0.98, 0.08, h * 0.98],
        soil,
    ));
    out.push(frame_box(
        frame.at([-w * 0.47, POST / 2.0, -h * 0.47]),
        axes,
        [0.12, POST, 0.12],
        color,
    ));
    POST
}

/// Pushes a node marker; returns the height of its top above the ground.
fn push_node(view: &EntityView, frame: &GroundFrame, out: &mut Vec<[f32; 16]>) -> f32 {
    if view.carry_node {
        let axes = [frame.ex, frame.up, frame.ey];
        out.push(frame_box(
            frame.at([0.0, 0.4, 0.0]),
            axes,
            [0.85, 0.8, 0.85],
            [0.48, 0.46, 0.40],
        ));
        out.push(frame_box(
            frame.at([0.0, 0.94, 0.0]),
            axes,
            [1.0, 0.28, 1.0],
            NODE_COLOR,
        ));
        return 1.08;
    }
    let size = 0.7;
    let forward = heading_of(frame, view.facing_deg);
    out.push(frame_box(
        frame.at([0.0, size / 2.0, 0.0]),
        [forward, frame.up, cross3(forward, frame.up)],
        [size, size, size],
        NODE_COLOR,
    ));
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TileKind;
    use rom_geo::SurfacePos;

    fn view(id: u32, x: f32, y: f32, facing: f32) -> EntityView {
        EntityView {
            id,
            kind: 1,
            owner: 1,
            face: 0,
            x,
            y,
            facing_deg: facing,
            hp_frac: 1.0,
            building: false,
            footprint: (1, 1),
            radius: 0.25,
            carrying: false,
            kept: 0,
            capacity: 0,
            aboard: 0,
            swimming: false,
            lift: 0.0,
            pod: 0.0,
            engines: false,
            carry_node: false,
            under_construction: None,
            model: None,
            altitude: 0.0,
            shot: None,
            spacecraft: false,
            flight: 0.0,
            aircraft: false,
            hover: Hover::Circle,
            cloud: None,
            activity: Default::default(),
        }
    }

    #[test]
    fn a_unit_in_the_air_is_drawn_above_the_ground() {
        let ground = FlatGround(&|_, _| 1.0);
        let down = view(1, 3.5, 3.5, 0.0);
        let up = EntityView { lift: 2.5, ..down };
        let (a, b) = (entity_frame(&down, &ground), entity_frame(&up, &ground));
        assert!((b.origin[1] - a.origin[1] - 2.5).abs() < 1e-5);
        let half = interpolate(&down, &up, 0.5);
        assert!((half.lift - 1.25).abs() < 1e-6);
    }

    #[test]
    fn a_swimming_unit_stands_waist_deep_in_the_water() {
        let ground = FlatGround(&|_, _| 0.0);
        let dry = view(1, 3.5, 3.5, 0.0);
        let wet = EntityView {
            swimming: true,
            ..dry
        };
        let (dry_frame, wet_frame) = (entity_frame(&dry, &ground), entity_frame(&wet, &ground));
        let depth = SWIM_DEPTH * unit_height(&dry);
        assert!((dry_frame.origin[1] - wet_frame.origin[1] - depth).abs() < 1e-5);
        assert_eq!(dry_frame.origin[0], wet_frame.origin[0]);
        // Its top stays above the water surface.
        assert!(wet_frame.origin[1] + unit_height(&wet) > 0.0);
    }

    fn pair(older: Vec<EntityView>, newer: Vec<EntityView>) -> SnapshotPair {
        let start = Instant::now();
        let mut pair = SnapshotPair::default();
        pair.push(
            Arc::new(EntitySnapshot {
                tick: 1,
                entities: older,
            }),
            start,
        );
        pair.push(
            Arc::new(EntitySnapshot {
                tick: 2,
                entities: newer,
            }),
            start + Duration::from_millis(100),
        );
        pair
    }

    #[test]
    fn aircraft_fly_at_their_height_rise_over_tall_buildings_and_land_on_the_ground() {
        let ground = FlatGround(&|_, _| 2.0);
        let plane = |id, x, flight| EntityView {
            flight,
            aircraft: true,
            hover: Hover::Circle,
            cloud: None,
            ..view(id, x, 10.0, 0.0)
        };
        let airborne = plane(1, 10.0, 1.0);
        let climbing = plane(2, 30.0, 0.5);
        // A model built flying: its underside 1.5 tiles over its origin.
        let mut landed = plane(3, 50.0, 0.0);
        landed.model = Some(ModelRef {
            index: 0,
            size: [1.0, 2.0, 1.0],
            floor: 1.5,
        });
        let over_tower = plane(4, 70.0, 1.0);
        let mut tower = view(5, 70.0, 10.0, 0.0);
        tower.building = true;
        tower.footprint = (2, 2);
        tower.kind = 3;
        let tower_height = building_height(&tower);
        let length = unit_width(&airborne);
        let mut views = [airborne, climbing, landed, over_tower, tower];
        lift_aircraft(&mut views, &ground, 0.0, None);
        let cruise = 2.0 + FLIGHT_ALTITUDE + FLIGHT_PER_TILE * length;
        assert!(
            (views[0].altitude - cruise).abs() < 1e-4,
            "{}",
            views[0].altitude
        );
        assert!(views[1].altitude > 2.0 && views[1].altitude < cruise);
        assert!(
            (views[2].altitude - 0.5).abs() < 1e-4,
            "landed: underside on the ground"
        );
        let frame = entity_frame(&views[2], &ground);
        assert!((frame.origin[1] - 0.5).abs() < 1e-4, "{:?}", frame.origin);
        assert!(views[3].altitude >= 2.0 + tower_height.max(FLIGHT_ALTITUDE));
        assert!(views[3].altitude < ORBIT_ALTITUDE + 2.0 + tower_height);
    }

    #[test]
    fn idle_rotorcraft_hang_in_place_and_the_low_flyer_hangs_lower() {
        let ground = FlatGround(&|_, _| 2.0);
        let craft = |id, hover| EntityView {
            flight: 1.0,
            aircraft: true,
            hover,
            ..view(id, 10.0, 10.0, 0.0)
        };
        let mut views = [
            craft(1, Hover::Circle),
            craft(2, Hover::Hold),
            craft(3, Hover::Low),
        ];
        lift_aircraft(&mut views, &ground, 3.0, Some(64.0));
        assert_ne!((views[0].x, views[0].y), (10.0, 10.0), "a jet circles");
        for held in &views[1..] {
            assert_eq!((held.x, held.y), (10.0, 10.0));
        }
        assert!((views[1].altitude - views[0].altitude).abs() < 1e-4);
        assert!(views[2].altitude > 2.0 && views[2].altitude < views[1].altitude - 2.0);
    }

    #[test]
    fn carried_marker_sits_above_the_unit() {
        let mut unit = view(1, 8.0, 10.0, 0.0);
        unit.carrying = true;
        let ground = FlatGround(&|_, _| 2.0);
        let mut instances = Vec::new();
        build_instances_on(&[unit], &ground, &mut instances);
        assert_eq!(instances.len(), 3);
        let marker = instances[2];
        assert_eq!([marker[3], marker[7], marker[11]], NODE_COLOR);
        let top = instances[0][13] + instances[0][5] / 2.0;
        assert!(
            marker[13] - marker[5] / 2.0 > top + 0.6,
            "cargo clears the health bar"
        );
        assert_eq!((marker[12], marker[14]), (8.0, 10.0));
        let globe = Globe::new(&crate::PlanetScene {
            sea_level: None,
            n: 64,
            kinds: vec![TileKind::Land; 6 * 64 * 64],
            heights: vec![0; 6 * 64 * 64],
            feature_colors: Default::default(),
            starts: vec![],
            nodes: vec![],
            sun: Default::default(),
        });
        build_instances_on(&[unit], &globe, &mut instances);
        let frame = globe.frame(unit.face, unit.x, unit.y);
        let marker = instances[2];
        let lift: f32 = (0..3)
            .map(|i| (marker[12 + i] - frame.origin[i]) * frame.up[i])
            .sum();
        assert!(lift > unit_height(&unit));
    }

    #[test]
    fn interpolation_is_linear_between_snapshots_and_clamped() {
        // The first gap of 100 ms becomes the interval.
        let pair = pair(vec![view(1, 0.0, 10.0, 0.0)], vec![view(1, 4.0, 14.0, 0.0)]);
        assert_eq!(pair.interval(), Duration::from_millis(100));
        let mut out = Vec::new();
        for (ms, expected) in [(0, 0.0), (25, 1.0), (50, 2.0), (100, 4.0), (400, 4.0)] {
            pair.blend(Duration::from_millis(ms), &mut out);
            assert_eq!(out.len(), 1);
            assert!(
                (out[0].x - expected).abs() < 1e-5,
                "x at {ms} ms: {}",
                out[0].x
            );
            assert!((out[0].y - (10.0 + expected)).abs() < 1e-5);
        }
    }

    #[test]
    fn render_tick_moves_with_the_blend() {
        let start = Instant::now();
        let mut pair = SnapshotPair::default();
        assert_eq!(pair.render_tick(start), 0.0);
        for (tick, ms) in [(10, 0), (11, 100)] {
            let snapshot = EntitySnapshot {
                tick,
                entities: Vec::new(),
            };
            pair.push(Arc::new(snapshot), start + Duration::from_millis(ms));
        }
        let at = |ms: u64| pair.render_tick(start + Duration::from_millis(100 + ms));
        assert!((at(0) - 10.0).abs() < 1e-9);
        assert!((at(25) - 10.25).abs() < 1e-6);
        // Held at the newest snapshot while no newer one comes (a paused game).
        assert!((at(5_000) - 11.0).abs() < 1e-9);
    }

    #[test]
    fn facing_interpolates_across_the_wraparound() {
        assert!((lerp_angle(350.0, 10.0, 0.5) - 0.0).abs() < 1e-4);
        assert!((lerp_angle(10.0, 350.0, 0.25) - 5.0).abs() < 1e-4);
        assert!((lerp_angle(350.0, 10.0, 0.25) - 355.0).abs() < 1e-4);
        assert!((lerp_angle(90.0, 180.0, 0.5) - 135.0).abs() < 1e-4);
        let pair = pair(
            vec![view(1, 0.0, 0.0, 350.0)],
            vec![view(1, 0.0, 0.0, 10.0)],
        );
        let mut out = Vec::new();
        pair.blend(Duration::from_millis(50), &mut out);
        let facing = out[0].facing_deg;
        assert!(facing < 1.0 || facing > 359.0, "facing {facing}");
    }

    #[test]
    fn entity_missing_from_older_snapshot_appears_at_its_position() {
        let pair = pair(
            vec![view(1, 0.0, 0.0, 0.0)],
            vec![view(1, 2.0, 0.0, 0.0), view(2, 7.0, 8.0, 90.0)],
        );
        let mut out = Vec::new();
        pair.blend(Duration::from_millis(50), &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0].x - 1.0).abs() < 1e-5);
        assert_eq!((out[1].x, out[1].y, out[1].facing_deg), (7.0, 8.0, 90.0));
    }

    fn hill() -> Surface {
        // 4x4 tiles; the heights rise by 20 per tile column (elevation = height scale).
        let heights = (0..16).map(|i| (i % 4) as u8 * 20 + 40).collect();
        Surface::new(Arc::new(TerrainScene {
            width: 4,
            height: 4,
            heights,
            kinds: vec![TileKind::Land; 16],
            feature_colors: Default::default(),
            starts: Vec::new(),
            nodes: Vec::new(),
        }))
    }

    #[test]
    fn health_bars_are_not_instances_and_never_cast_shadows() {
        let ground = hill();
        let mut unit = view(1, 1.7, 2.2, 90.0);
        let mut building = view(2, 2.0, 2.0, 0.0);
        building.building = true;
        building.footprint = (2, 2);
        let mut node = view(3, 0.5, 0.5, 0.0);
        node.owner = 0;
        let mut display = Vec::new();
        let mut ranges = Vec::new();
        let mut anchors = Vec::new();
        let mut bodies = |views: &[EntityView]| {
            build_instances_with_casters_on(
                views,
                &ground,
                &mut display,
                &mut ranges,
                &mut anchors,
                &|_| false,
            );
            let casters: Vec<_> = ranges
                .iter()
                .flat_map(|r| display[r.clone()].iter().copied())
                .collect();
            (display.clone(), casters, anchors.clone())
        };
        let (healthy, expected, first) = bodies(&[unit, building, node]);
        assert_eq!(healthy, expected);
        unit.hp_frac = 0.7;
        building.hp_frac = 0.3;
        let (injured, casters, second) = bodies(&[unit, building, node]);
        assert_eq!(casters, expected);
        assert_eq!(injured, healthy, "a bar adds no instance");
        assert_eq!(second.len(), 3, "every entity has an anchor");
        assert_eq!(first[0].frac, 1.0);
        assert_eq!((second[0].frac, second[1].frac), (0.7, 0.3));
        assert!(second.iter().all(|a| a.ground_radius.is_none()));
        unit.x += 0.2;
        let (_, moved, _) = bodies(&[unit, building, node]);
        assert_ne!(moved, expected, "moving a body invalidates the shadow map");
    }

    #[test]
    fn instance_transforms_sit_on_terrain_height() {
        let surface = hill();
        let unit = view(1, 1.7, 2.2, 90.0);
        let mut out = Vec::new();
        build_instances(&[unit], &surface, &mut out);
        let body = out[0];
        let ground = surface.height(1.7, 2.2);
        assert!(ground > 0.1, "test terrain should be above sea level");
        let bottom = body[13] - body[5] / 2.0;
        assert!(
            (bottom - ground).abs() < 1e-4,
            "bottom {bottom} ground {ground}"
        );
        assert_eq!((body[12], body[14]), (1.7, 2.2));

        // A building's base reaches the lowest corner and its top clears the highest.
        let mut building = view(2, 2.0, 2.0, 0.0);
        building.building = true;
        building.footprint = (2, 2);
        build_instances(&[building], &surface, &mut out);
        let base = out[0];
        let bottom = base[13] - base[5] / 2.0;
        let lowest = surface.height(1.02, 1.02).min(surface.height(2.98, 1.02));
        assert!(bottom <= lowest);
        let highest = surface.height(2.98, 2.98);
        assert!(base[13] + base[5] / 2.0 > highest);
        // Owner colour is carried in the unused cells.
        assert!(out.iter().all(|m| m[15] == 1.0));
    }

    /// A planet with hills, for placing entities on the sphere.
    fn hilly_planet(n: u32) -> crate::globe::PlanetScene {
        let tiles = 6 * (n * n) as usize;
        crate::globe::PlanetScene {
            sea_level: None,
            n,
            kinds: vec![TileKind::Land; tiles],
            heights: (0..tiles).map(|i| 90 + (i * 29 % 90) as u8).collect(),
            feature_colors: Default::default(),
            starts: Vec::new(),
            nodes: Vec::new(),
            sun: Default::default(),
        }
    }

    fn norm(v: [f32; 3]) -> f32 {
        dot3(v, v).sqrt()
    }

    #[test]
    fn orbital_hull_and_pick_box_share_the_raised_frame() {
        let globe = Globe::new(&hilly_planet(64));
        let mut ship = view(9, 32.5, 32.5, 20.0);
        ship.altitude = 8.0;
        let (centre, axes, size) = pick_frame(&ship, &globe);
        assert_eq!(
            size,
            [unit_width(&ship), unit_height(&ship), unit_width(&ship)]
        );
        let ground = globe.ground_frame(ship.face, ship.x, ship.y);
        let foot: [f32; 3] = std::array::from_fn(|i| centre[i] - axes[1][i] * size[1] * 0.5);
        let raised = ground.up.map(|v| v * (globe.radius() + ship.altitude));
        assert!(norm(std::array::from_fn(|i| foot[i] - raised[i])) < 0.001);
        let mut instances = Vec::new();
        build_instances_on(&[ship], &globe, &mut instances);
        assert!(!instances.is_empty());
        for matrix in instances {
            let at = [matrix[12], matrix[13], matrix[14]];
            assert!(
                norm(at) >= globe.radius() + ship.altitude,
                "all hull pieces clear the orbit's base"
            );
        }
    }

    /// A ship of `kind` with a composed model `length` tiles long, `altitude` up (of the
    /// snapshots' orbit).
    fn ship(id: u32, kind: u16, length: f32, altitude: f32) -> EntityView {
        EntityView {
            kind,
            spacecraft: true,
            flight: 0.0,
            aircraft: false,
            hover: Hover::Circle,
            cloud: None,
            altitude,
            model: Some(ModelRef {
                index: 0,
                size: [length * 0.4, length * 0.3, length],
                floor: 0.0,
            }),
            ..view(id, 32.5, 32.5, 0.0)
        }
    }

    #[test]
    fn ships_are_drawn_big_hover_by_size_and_spread_sideways() {
        use crate::layers::{ShipScale, ShipScales};
        let globe = Globe::new(&hilly_planet(64));
        let scales = ShipScales(
            [
                (1, ShipScale::from_pct(700, 25)),
                (2, ShipScale::from_pct(1000, 50)),
                (3, ShipScale::from_pct(300, 25).true_size_on_the_ground()),
                (4, ShipScale::from_pct(750, 30)),
            ]
            .into_iter()
            .collect(),
        );
        // A small and a big ship over one point, two more small ones there, a landed big one
        // and a landed craft.
        let mut views = vec![
            ship(1, 1, 3.0, ORBIT_ALTITUDE),
            ship(2, 2, 5.0, ORBIT_ALTITUDE),
            ship(3, 1, 3.0, ORBIT_ALTITUDE),
            ship(4, 1, 3.0, ORBIT_ALTITUDE),
            EntityView {
                x: 10.5,
                ..ship(5, 2, 5.0, 0.0)
            },
            EntityView {
                x: 50.5,
                ..ship(6, 3, 2.0, 0.0)
            },
            ship(7, 4, 3.0, ORBIT_ALTITUDE),
        ];
        dress_ships(&mut views, &scales, &globe);
        // A ship of another class a little bigger than the small ones holds just above them.
        let small_top = views[0].altitude + unit_height(&views[0]);
        assert!(views[6].altitude >= small_top, "{}", views[6].altitude);
        assert!(views[6].altitude > hover_altitude(22.5));
        assert_eq!((views[6].x, views[6].y), (32.5, 32.5));
        // Seven and ten times their size, in orbit and (a starship) on the ground; a craft
        // comes down at its own size.
        assert!((unit_width(&views[0]) - 21.0).abs() < 1e-3);
        assert!((unit_width(&views[1]) - 50.0).abs() < 1e-3);
        assert!((unit_width(&views[4]) - 50.0).abs() < 1e-3);
        assert!((unit_width(&views[5]) - 2.0).abs() < 1e-3);
        assert_eq!((views[4].altitude, views[5].altitude), (0.0, 0.0));
        // The bigger ship holds higher, clear of the smaller one's top.
        assert_eq!(views[0].altitude, hover_altitude(21.0));
        assert_eq!(views[1].altitude, hover_altitude(50.0));
        assert!(views[1].altitude > views[0].altitude + unit_height(&views[0]));
        // The big one keeps its point; the three small ones spread apart by their size.
        assert_eq!((views[1].x, views[1].y), (32.5, 32.5));
        let at = |v: &EntityView| {
            globe.dir_of(SurfacePos {
                face: v.face,
                x: (v.x * 1000.0) as i64,
                y: (v.y * 1000.0) as i64,
            })
        };
        for (a, b) in [(0, 2), (0, 3), (2, 3)] {
            let d: [f32; 3] = std::array::from_fn(|k| at(&views[a])[k] - at(&views[b])[k]);
            let tiles = dot3(d, d).sqrt() * globe.radius();
            assert!(tiles > 21.0 * 0.3, "{a} and {b}: {tiles} tiles apart");
        }
        // Dressing is a pure function of the views: the same every frame.
        let mut again = vec![
            ship(1, 1, 3.0, ORBIT_ALTITUDE),
            ship(2, 2, 5.0, ORBIT_ALTITUDE),
            ship(3, 1, 3.0, ORBIT_ALTITUDE),
            ship(4, 1, 3.0, ORBIT_ALTITUDE),
        ];
        dress_ships(&mut again, &scales, &globe);
        assert_eq!(again[..], views[..4]);
    }

    #[test]
    fn ships_aloft_keep_their_floor_on_screen_and_their_places() {
        use crate::layers::{SHIP_FLOOR_PIXELS, ShipScale, ShipScales};
        let globe = Globe::new(&hilly_planet(64));
        let scales = ShipScales(
            [
                (1, ShipScale::from_pct(200, 25)),
                (2, ShipScale::from_pct(300, 50)),
            ]
            .into_iter()
            .collect(),
        );
        let dressed = || {
            let mut views = vec![
                ship(1, 1, 3.0, ORBIT_ALTITUDE),
                ship(2, 2, 3.0, ORBIT_ALTITUDE),
                EntityView {
                    x: 10.5,
                    ..ship(3, 1, 3.0, 0.0)
                },
            ];
            dress_ships(&mut views, &scales, &globe);
            views
        };
        let near = dressed();
        let tan = (crate::hud_minimap::FOV_DEGREES.to_radians() / 2.0).tan();
        let height = 800.0;
        let pixels = |v: &EntityView, eye: [f32; 3]| {
            let dir = globe.dir_of(SurfacePos {
                face: v.face,
                x: (v.x * 1000.0).round() as i64,
                y: (v.y * 1000.0).round() as i64,
            });
            let at = dir.map(|c| c * (globe.radius() + v.altitude));
            let d: [f32; 3] = std::array::from_fn(|i| at[i] - eye[i]);
            unit_width(v) / (dot3(d, d).sqrt() * 2.0 * tan / height)
        };
        // Close by, the floor does not bind: sizes stay as dressed.
        let close = globe
            .dir_of(SurfacePos {
                face: near[0].face,
                x: 3000,
                y: 3000,
            })
            .map(|c| c * (globe.radius() + 30.0));
        let mut views = near.clone();
        floor_ships(&mut views, &scales, &globe, close, height);
        assert_eq!(views, near);
        // Far out, ships aloft are drawn at their floor, by class; places, heights and the
        // landed ship stay as they were.
        let far = close.map(|c| c * 40.0);
        let mut views = near.clone();
        floor_ships(&mut views, &scales, &globe, far, height);
        assert!((pixels(&views[0], far) - SHIP_FLOOR_PIXELS).abs() < 0.5);
        assert!((pixels(&views[1], far) - 2.0 * SHIP_FLOOR_PIXELS).abs() < 1.0);
        for (a, b) in views.iter().zip(&near) {
            assert_eq!((a.x, a.y, a.altitude), (b.x, b.y, b.altitude));
        }
        assert_eq!(views[2], near[2]);
        // The same every frame.
        let mut again = near.clone();
        floor_ships(&mut again, &scales, &globe, far, height);
        assert_eq!(again, views);
    }

    #[test]
    fn entity_on_the_globe_sits_on_the_surface_at_its_tile() {
        let n = 48;
        let globe = Globe::new(&hilly_planet(n));
        for (face, tx, ty) in [
            (0, 10, 20),
            (1, 3, 40),
            (2, 47, 0),
            (3, 24, 24),
            (4, 5, 5),
            (5, 30, 12),
        ] {
            let mut unit = view(7, tx as f32 + 0.5, ty as f32 + 0.5, 30.0);
            unit.face = face;
            let mut out = Vec::new();
            build_instances_on(&[unit], &globe, &mut out);
            let body = out[0];
            let (width, height) = (unit_width(&unit), unit_height(&unit));
            // The body's up axis is the local up, and its foot rests on the ground of the tile.
            let up = [body[4] / height, body[5] / height, body[6] / height];
            let centre = [body[12], body[13], body[14]];
            let foot: [f32; 3] = std::array::from_fn(|i| centre[i] - up[i] * height / 2.0);
            let ground = globe.radius() + globe.surface_height(face, unit.x, unit.y);
            assert!(
                (norm(foot) - ground).abs() < 1e-3,
                "face {face}: {} vs {ground}",
                norm(foot)
            );
            let dir = globe.dir_of(SurfacePos {
                face,
                x: i64::from(tx) * 1000 + 500,
                y: i64::from(ty) * 1000 + 500,
            });
            let foot_dir = foot.map(|c| c / norm(foot));
            assert!(dot3(foot_dir, dir) > 0.999_99, "face {face}");
            assert!(dot3(up, dir) > 0.999_99);
            assert!((norm([body[0], body[1], body[2]]) - width).abs() < 1e-4);
            // The tile found under the foot is the entity's tile.
            let tile = globe.tile_of(foot);
            assert_eq!((tile.face, tile.x, tile.y), (face, tx, ty));
        }
    }

    #[test]
    fn ground_frames_are_right_handed_and_follow_the_face_axes() {
        let globe = Globe::new(&hilly_planet(32));
        for (face, x, y) in [
            (0, 16.5, 16.5),
            (1, 3.2, 28.9),
            (4, 30.1, 2.4),
            (5, 0.2, 0.3),
        ] {
            let f = globe.ground_frame(face, x, y);
            assert!((norm(f.ex) - 1.0).abs() < 1e-4 && (norm(f.ey) - 1.0).abs() < 1e-4);
            assert!(dot3(f.ex, f.up).abs() < 1e-4 && dot3(f.ey, f.up).abs() < 1e-4);
            let right_handed = cross3(f.ex, f.up);
            assert!(dot3(right_handed, f.ey) > 0.999, "face {face}");
            // Moving east on the face moves along ex.
            let there = globe.world_point(face, (x + 0.2).min(31.9), y, 0.0);
            let here = globe.world_point(face, x, y, 0.0);
            let step: [f32; 3] = std::array::from_fn(|i| there[i] - here[i]);
            assert!(dot3(step, f.ex) > 0.0);
        }
    }

    #[test]
    fn interpolation_across_a_face_edge_has_no_jump() {
        let n = 32;
        let globe = Globe::new(&hilly_planet(n));
        // A unit walking east over the right edge of face 0, 0.9 tiles in all.
        let from_pos = SurfacePos {
            face: 0,
            x: 31_400,
            y: 9_000,
        };
        let to_pos = globe.surface().step(from_pos, 900, 0);
        assert_ne!(to_pos.face, 0);
        let heading_from = heading_of(&globe.ground_frame(0, 31.4, 9.0), 0.0);
        let frame_to = globe.ground_frame(
            to_pos.face,
            to_pos.x as f32 / 1000.0,
            to_pos.y as f32 / 1000.0,
        );
        let facing_to = dot3(heading_from, frame_to.ey)
            .atan2(dot3(heading_from, frame_to.ex))
            .to_degrees()
            .rem_euclid(360.0);
        let mut a = view(1, 31.4, 9.0, 0.0);
        a.face = 0;
        let mut b = view(
            1,
            to_pos.x as f32 / 1000.0,
            to_pos.y as f32 / 1000.0,
            facing_to,
        );
        b.face = to_pos.face;
        let steps = 30;
        let mut last: Option<([f32; 3], [f32; 3])> = None;
        let mut faces = std::collections::BTreeSet::new();
        let mut travelled = 0.0;
        for i in 0..=steps {
            let at = interpolate_on(&globe, &a, &b, i as f32 / steps as f32);
            faces.insert(at.face);
            let frame = globe.ground_frame(at.face, at.x, at.y);
            let heading = heading_of(&frame, at.facing_deg);
            if let Some((point, last_heading)) = last {
                let jump: [f32; 3] = std::array::from_fn(|k| frame.origin[k] - point[k]);
                travelled += norm(jump);
                // 0.9 tiles over 30 steps is 0.03 per step; nothing may be far beyond that.
                assert!(norm(jump) < 0.06, "jump {} at step {i}", norm(jump));
                assert!(
                    dot3(heading, last_heading) > 0.99,
                    "heading jump at step {i}"
                );
            }
            last = Some((frame.origin, heading));
        }
        assert_eq!(faces.len(), 2, "the walk crosses the edge");
        // Hills add to the path length, never shorten it.
        assert!(travelled > 0.85, "travelled {travelled}");
        // The ends are the snapshots themselves.
        let start = interpolate_on(&globe, &a, &b, 0.0);
        let end = interpolate_on(&globe, &a, &b, 1.0);
        assert_eq!((start.face, end.face), (0, to_pos.face));
        assert!((end.x - b.x).abs() < 0.01 && (end.y - b.y).abs() < 0.01);
    }
}

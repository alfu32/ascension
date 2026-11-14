//! Entity snapshots, their interpolation and the instance data for drawing them.
//!
//! Everything here is plain data and arithmetic so it can be tested without a GPU. The render
//! thread blends the last two snapshots, samples the terrain surface and turns each entity into
//! a few unit-cube instances; the colour of an instance travels in the three matrix cells that
//! affine transforms leave unused (`m3`, `m7`, `m11`).

use std::sync::Arc;
use std::time::{Duration, Instant};

use rom_geo::SurfacePos;

use crate::globe::Globe;
use crate::terrain::{Shape, TerrainScene, vertex_height};

/// Number of owner colours; owner 0 (neutral) is grey.
pub const OWNER_COLORS: usize = 9;
/// Colour of node entities.
pub const NODE_COLOR: [f32; 3] = [0.98, 0.84, 0.16];
/// Nominal time between two simulation ticks, used until two snapshots have arrived.
const DEFAULT_INTERVAL: Duration = Duration::from_millis(50);
/// Arrival gaps shorter than this come from catch-up bursts and are ignored for the interval.
const MIN_GAP: Duration = Duration::from_millis(5);
/// Longest interval the interpolation stretches over.
const MAX_INTERVAL: Duration = Duration::from_millis(1000);

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
    /// A carry-mode node drawn as a gold-capped plinth.
    pub carry_node: bool,
    /// Construction progress 0 to 1 for a site, `None` when complete.
    pub under_construction: Option<f32>,
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
        ..*to
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

/// `right` made perpendicular to `up` and of unit length (unchanged when already so).
pub(crate) fn side_of(right: [f32; 3], up: [f32; 3]) -> [f32; 3] {
    let d = dot3(right, up);
    let v = [
        right[0] - up[0] * d,
        right[1] - up[1] * d,
        right[2] - up[2] * d,
    ];
    let length = dot3(v, v).sqrt();
    if length < 1e-6 {
        return [1.0, 0.0, 0.0];
    }
    v.map(|c| c / length)
}

/// The heading of an entity facing `facing_deg` (clockwise from the face's x axis).
pub(crate) fn heading_of(frame: &GroundFrame, facing_deg: f32) -> [f32; 3] {
    let (sin, cos) = facing_deg.to_radians().sin_cos();
    std::array::from_fn(|i| frame.ex[i] * cos + frame.ey[i] * sin)
}

/// Height of a building body in tiles; it varies with the entity type and the footprint.
pub fn building_height(view: &EntityView) -> f32 {
    let (w, h) = (f32::from(view.footprint.0), f32::from(view.footprint.1));
    1.0 + f32::from(view.kind % 4) * 0.4 + 0.15 * w.max(h)
}

/// Width of a unit body in tiles; drawn larger than the collision circle so it stays visible.
pub fn unit_width(view: &EntityView) -> f32 {
    (view.radius * 2.0 * 1.6).max(0.6)
}

/// Height of a unit body in tiles.
pub fn unit_height(view: &EntityView) -> f32 {
    (unit_width(view) * 1.8).max(0.9)
}

/// The box that stands for `view` when picking: centre `[east, up, south]`, size and yaw in
/// degrees, matching how [`build_instances`] draws the entity. `ground` samples the surface.
pub fn pick_box(view: &EntityView, ground: &dyn Fn(f32, f32) -> f32) -> ([f32; 3], [f32; 3], f32) {
    let base = ground(view.x, view.y);
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
    let frame = ground.frame(view.face, view.x, view.y);
    let (offset, size, yaw) = {
        let (centre, size, yaw) = pick_box(view, &|_, _| 0.0);
        ([0.0, centre[1], 0.0], size, yaw)
    };
    let forward = heading_of(&frame, yaw);
    let axes = [forward, frame.up, cross3(forward, frame.up)];
    (frame.at(offset), axes, size)
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

/// Appends the instances that draw `views` on a flat map; `camera_yaw` (radians) turns health
/// bars to the camera.
pub fn build_instances(
    views: &[EntityView],
    surface: &Surface,
    camera_yaw: f32,
    out: &mut Vec<[f32; 16]>,
) {
    build_instances_on(
        views,
        surface,
        [camera_yaw.cos(), 0.0, -camera_yaw.sin()],
        out,
    );
}

/// Appends the instances that draw `views` standing on `ground`; `right` is the camera's right
/// vector, which turns health bars to the viewer.
pub fn build_instances_on(
    views: &[EntityView],
    ground: &dyn Footing,
    right: [f32; 3],
    out: &mut Vec<[f32; 16]>,
) {
    instances_on(views, ground, right, out, None, &|_| false);
}

/// Builds display instances and records which ones are bodies that cast shadows. Buildings
/// whose type `drawn_as_objects` get only a ground plate and a marker post; their look is
/// drawn by the feature objects (crop rows).
pub(crate) fn build_instances_with_casters_on(
    views: &[EntityView],
    ground: &dyn Footing,
    right: [f32; 3],
    out: &mut Vec<[f32; 16]>,
    casters: &mut Vec<std::ops::Range<usize>>,
    drawn_as_objects: &dyn Fn(u16) -> bool,
) {
    casters.clear();
    instances_on(views, ground, right, out, Some(casters), drawn_as_objects);
}

fn instances_on(
    views: &[EntityView],
    ground: &dyn Footing,
    right: [f32; 3],
    out: &mut Vec<[f32; 16]>,
    mut casters: Option<&mut Vec<std::ops::Range<usize>>>,
    drawn_as_objects: &dyn Fn(u16) -> bool,
) {
    out.clear();
    for view in views {
        let frame = ground.frame(view.face, view.x, view.y);
        let base = ground.height_at(view.face, view.x, view.y);
        let body_start = out.len();
        let top = if view.building && drawn_as_objects(view.kind) {
            push_plot(view, &frame, out)
        } else if view.building {
            push_building(view, ground, &frame, base, out)
        } else if view.owner == 0 || view.carry_node {
            push_node(view, &frame, out)
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
        if view.hp_frac < 1.0 {
            let width = if view.building {
                f32::from(view.footprint.0.max(view.footprint.1)).max(1.0)
            } else {
                (view.radius * 4.0).max(0.8)
            };
            let side = side_of(right, frame.up);
            let axes = [side, frame.up, cross3(side, frame.up)];
            let frac = view.hp_frac.clamp(0.0, 1.0);
            let centre = frame.at([0.0, top + 0.4, 0.0]);
            out.push(frame_box(
                centre,
                axes,
                [width + 0.08, 0.14, 0.06],
                [0.08, 0.08, 0.08],
            ));
            if frac > 0.0 {
                let shift = -(width - width * frac) / 2.0;
                let fill_centre: [f32; 3] = std::array::from_fn(|i| centre[i] + side[i] * shift);
                let color = if frac > 0.5 {
                    [0.25, 0.85, 0.3]
                } else if frac > 0.25 {
                    [0.95, 0.75, 0.2]
                } else {
                    [0.9, 0.2, 0.15]
                };
                out.push(frame_box(
                    fill_centre,
                    axes,
                    [width * frac, 0.1, 0.1],
                    color,
                ));
            }
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
            carry_node: false,
            under_construction: None,
        }
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
    fn carried_marker_sits_above_the_unit() {
        let mut unit = view(1, 8.0, 10.0, 0.0);
        unit.carrying = true;
        let ground = FlatGround(&|_, _| 2.0);
        let mut instances = Vec::new();
        build_instances_on(&[unit], &ground, [1.0, 0.0, 0.0], &mut instances);
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
            n: 64,
            kinds: vec![TileKind::Land; 6 * 64 * 64],
            heights: vec![0; 6 * 64 * 64],
            feature_colors: Default::default(),
            starts: vec![],
            nodes: vec![],
            sun: Default::default(),
        });
        build_instances_on(&[unit], &globe, [1.0, 0.0, 0.0], &mut instances);
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
    fn health_bars_do_not_cast_shadows_or_change_caster_geometry() {
        let ground = hill();
        let mut unit = view(1, 1.7, 2.2, 90.0);
        let mut building = view(2, 2.0, 2.0, 0.0);
        building.building = true;
        building.footprint = (2, 2);
        let mut node = view(3, 0.5, 0.5, 0.0);
        node.owner = 0;
        let mut display = Vec::new();
        let mut ranges = Vec::new();
        let mut bodies = |views: &[EntityView], right| {
            build_instances_with_casters_on(
                views,
                &ground,
                right,
                &mut display,
                &mut ranges,
                &|_| false,
            );
            let casters: Vec<_> = ranges
                .iter()
                .flat_map(|r| display[r.clone()].iter().copied())
                .collect();
            (display.clone(), casters)
        };
        let (healthy, expected) = bodies(&[unit, building, node], [1.0, 0.0, 0.0]);
        assert_eq!(healthy, expected);
        unit.hp_frac = 0.7;
        building.hp_frac = 0.3;
        let (injured, casters) = bodies(&[unit, building, node], [1.0, 0.0, 0.0]);
        assert_eq!(casters, expected);
        assert_eq!(injured.len(), healthy.len() + 4);
        let (turned, casters) = bodies(&[unit, building, node], [0.0, 0.0, 1.0]);
        assert_ne!(turned, injured, "health bars turn toward the camera");
        assert_eq!(casters, expected, "bodies keep the cached shadow map");
        unit.x += 0.2;
        let (_, moved) = bodies(&[unit, building, node], [0.0, 0.0, 1.0]);
        assert_ne!(moved, expected, "moving a body invalidates the shadow map");
    }

    #[test]
    fn instance_transforms_sit_on_terrain_height() {
        let surface = hill();
        let unit = view(1, 1.7, 2.2, 90.0);
        let mut out = Vec::new();
        build_instances(&[unit], &surface, 0.0, &mut out);
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
        build_instances(&[building], &surface, 0.0, &mut out);
        let base = out[0];
        let bottom = base[13] - base[5] / 2.0;
        let lowest = surface.height(1.02, 1.02).min(surface.height(2.98, 1.02));
        assert!(bottom <= lowest);
        let highest = surface.height(2.98, 2.98);
        assert!(base[13] + base[5] / 2.0 > highest);
        // Owner colour is carried in the unused cells.
        assert!(out.iter().all(|m| m[15] == 1.0));
    }

    #[test]
    fn health_bars_only_for_damaged_entities() {
        let surface = hill();
        let mut damaged = view(1, 1.0, 1.0, 0.0);
        let mut out = Vec::new();
        build_instances(&[damaged], &surface, 0.0, &mut out);
        let healthy = out.len();
        damaged.hp_frac = 0.5;
        build_instances(&[damaged], &surface, 0.0, &mut out);
        assert_eq!(out.len(), healthy + 2);
    }

    /// A planet with hills, for placing entities on the sphere.
    fn hilly_planet(n: u32) -> crate::globe::PlanetScene {
        let tiles = 6 * (n * n) as usize;
        crate::globe::PlanetScene {
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
            build_instances_on(&[unit], &globe, [1.0, 0.0, 0.0], &mut out);
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

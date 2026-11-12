//! Entity snapshots, their interpolation and the instance data for drawing them.
//!
//! Everything here is plain data and arithmetic so it can be tested without a GPU. The render
//! thread blends the last two snapshots, samples the terrain surface and turns each entity into
//! a few unit-cube instances; the colour of an instance travels in the three matrix cells that
//! affine transforms leave unused (`m3`, `m7`, `m11`).

use std::sync::Arc;
use std::time::{Duration, Instant};

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
    /// Position in tiles (east).
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
        let elapsed = self
            .arrived
            .map_or(Duration::ZERO, |at| now.saturating_duration_since(at));
        self.blend(elapsed, out);
    }

    /// Fills `out` with the newest entities, positions and facing moved from the older snapshot
    /// by `elapsed` of one interval (clamped to 0..=1). Entities absent from the older snapshot
    /// stay where they are.
    pub fn blend(&self, elapsed: Duration, out: &mut Vec<EntityView>) {
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
                Ok(index) => interpolate(&previous.entities[index], view, alpha),
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

/// Column-major 4×4 matrix of a box with its horizontal axes turned by `yaw_deg` around the
/// vertical axis (the box's x axis points along (cos, sin) in the x/y tile plane), `size` along
/// its axes, its centre at `centre` (x, up, y) and `color` stored in the unused cells.
pub fn box_instance(centre: [f32; 3], size: [f32; 3], yaw_deg: f32, color: [f32; 3]) -> [f32; 16] {
    let (sin, cos) = yaw_deg.to_radians().sin_cos();
    [
        cos * size[0],
        0.0,
        sin * size[0],
        color[0],
        0.0,
        size[1],
        0.0,
        color[1],
        -sin * size[2],
        0.0,
        cos * size[2],
        color[2],
        centre[0],
        centre[1],
        centre[2],
        1.0,
    ]
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

/// Appends the instances that draw `views`; `camera_yaw` (radians) turns health bars to the camera.
pub fn build_instances(
    views: &[EntityView],
    surface: &Surface,
    camera_yaw: f32,
    out: &mut Vec<[f32; 16]>,
) {
    out.clear();
    let (right_x, right_z) = (camera_yaw.cos(), -camera_yaw.sin());
    let bar_yaw_deg = right_z.atan2(right_x).to_degrees();
    for view in views {
        let top = if view.building {
            push_building(view, surface, out)
        } else if view.owner == 0 {
            push_node(view, surface, out)
        } else {
            push_unit(view, surface, out)
        };
        if view.hp_frac < 1.0 {
            let width = if view.building {
                f32::from(view.footprint.0.max(view.footprint.1)).max(1.0)
            } else {
                (view.radius * 4.0).max(0.8)
            };
            let height = top + 0.4;
            let frac = view.hp_frac.clamp(0.0, 1.0);
            let centre = [view.x, height, view.y];
            out.push(box_instance(
                centre,
                [width + 0.08, 0.14, 0.06],
                bar_yaw_deg,
                [0.08, 0.08, 0.08],
            ));
            if frac > 0.0 {
                let shift = -(width - width * frac) / 2.0;
                let fill_centre = [
                    centre[0] + right_x * shift,
                    height,
                    centre[2] + right_z * shift,
                ];
                let color = if frac > 0.5 {
                    [0.25, 0.85, 0.3]
                } else if frac > 0.25 {
                    [0.95, 0.75, 0.2]
                } else {
                    [0.9, 0.2, 0.15]
                };
                out.push(box_instance(
                    fill_centre,
                    [width * frac, 0.1, 0.1],
                    bar_yaw_deg,
                    color,
                ));
            }
        }
    }
}

/// Pushes a unit body and facing wedge; returns the height of the unit's top above the sea.
fn push_unit(view: &EntityView, surface: &Surface, out: &mut Vec<[f32; 16]>) -> f32 {
    let ground = surface.height(view.x, view.y);
    let color = owner_color(view.owner);
    let width = unit_width(view);
    let height = unit_height(view);
    out.push(box_instance(
        [view.x, ground + height / 2.0, view.y],
        [width, height, width],
        view.facing_deg,
        color,
    ));
    let (sin, cos) = view.facing_deg.to_radians().sin_cos();
    let reach = width * 0.5;
    out.push(box_instance(
        [
            view.x + cos * reach,
            ground + height * 0.62,
            view.y + sin * reach,
        ],
        [width * 0.55, height * 0.26, width * 0.4],
        view.facing_deg,
        scaled(color, 0.45),
    ));
    ground + height
}

/// Pushes a building body and roof; returns the height of its roof above the sea.
fn push_building(view: &EntityView, surface: &Surface, out: &mut Vec<[f32; 16]>) -> f32 {
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
        let ground = surface.height(view.x + dx * 0.98, view.y + dy * 0.98);
        low = low.min(ground);
        high = high.max(ground);
    }
    let progress = view.under_construction.map_or(1.0, |p| p.clamp(0.0, 1.0));
    let body = building_height(view) * progress.max(0.12);
    let color = owner_color(view.owner);
    let roof_thickness = 0.18;
    // The base reaches down to the lowest corner so slopes do not leave gaps.
    let bottom = low - 0.05;
    let top = high + body;
    out.push(box_instance(
        [view.x, (bottom + top) / 2.0, view.y],
        [w * 0.94, top - bottom, h * 0.94],
        0.0,
        scaled(
            color,
            if view.under_construction.is_some() {
                0.8
            } else {
                0.62
            },
        ),
    ));
    out.push(box_instance(
        [view.x, top + roof_thickness / 2.0, view.y],
        [w, roof_thickness, h],
        0.0,
        color,
    ));
    top + roof_thickness
}

/// Pushes a node marker; returns the height of its top above the sea.
fn push_node(view: &EntityView, surface: &Surface, out: &mut Vec<[f32; 16]>) -> f32 {
    let ground = surface.height(view.x, view.y);
    let size = 0.7;
    out.push(box_instance(
        [view.x, ground + size / 2.0, view.y],
        [size, size, size],
        view.facing_deg,
        NODE_COLOR,
    ));
    ground + size
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TileKind;

    fn view(id: u32, x: f32, y: f32, facing: f32) -> EntityView {
        EntityView {
            id,
            kind: 1,
            owner: 1,
            x,
            y,
            facing_deg: facing,
            hp_frac: 1.0,
            building: false,
            footprint: (1, 1),
            radius: 0.25,
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
}

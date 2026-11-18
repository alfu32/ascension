//! Geometry for the world overlays: selection rings and outlines, order markers and the building placement footprint. Plain lines and triangles in
//! world space, built from snapshots; the scene draws them after the entities. They stand on
//! any [`Footing`], a flat map or a sphere, so rings, outlines and the placement grid follow
//! the curvature.

use crate::entities::{
    EntityView, FlatGround, Footing, building_height, entity_frame, owner_color, unit_width,
};
use crate::hud::{HudSnapshot, MarkerKind};

mod effects;

/// A coloured line segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line3 {
    /// Start `[east, up, south]`.
    pub a: [f32; 3],
    /// End.
    pub b: [f32; 3],
    /// RGBA.
    pub colour: [u8; 4],
}

/// A coloured triangle; both windings are drawn so it is visible from either side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tri3 {
    /// Corners `[east, up, south]`.
    pub v: [[f32; 3]; 3],
    /// RGBA.
    pub colour: [u8; 4],
}

/// Lines and triangles of one frame.
#[derive(Clone, Debug, Default)]
pub struct OverlayGeometry {
    /// Rings, outlines, marker strokes.
    pub lines: Vec<Line3>,
    /// The placement fill and other flat shapes.
    pub tris: Vec<Tri3>,
}

const RING_SEGMENTS: usize = 28;
const LIFT: f32 = 0.07;
const MARKER_TICKS: f32 = 10.0;
const GREEN: [u8; 4] = [70, 230, 90, 255];
const RED: [u8; 4] = [240, 60, 50, 255];
const CYAN: [u8; 4] = [63, 208, 255, 255];
const AMBER: [u8; 4] = [240, 176, 48, 255];

fn rgba(c: [f32; 3]) -> [u8; 4] {
    [
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
        255,
    ]
}

/// A place on a face that offsets are measured from.
#[derive(Clone, Copy)]
struct Origin<'a> {
    ground: &'a dyn Footing,
    face: u8,
    x: f32,
    y: f32,
    altitude: f32,
}

impl Origin<'_> {
    /// The point `lift` above the ground at the offset (`dx`, `dy`) tiles from the origin.
    fn at(&self, dx: f32, dy: f32, lift: f32) -> [f32; 3] {
        if self.altitude > 0.0 {
            self.ground
                .datum_point(self.face, self.x, self.y, (dx, dy), lift + self.altitude)
        } else {
            self.ground.point(self.face, self.x, self.y, (dx, dy), lift)
        }
    }
}

impl OverlayGeometry {
    fn line(&mut self, a: [f32; 3], b: [f32; 3], colour: [u8; 4]) {
        self.lines.push(Line3 { a, b, colour });
    }

    fn tri(&mut self, v: [[f32; 3]; 3], colour: [u8; 4]) {
        self.tris.push(Tri3 { v, colour });
    }

    fn quad(&mut self, v: [[f32; 3]; 4], colour: [u8; 4]) {
        self.tri([v[0], v[1], v[2]], colour);
        self.tri([v[0], v[2], v[3]], colour);
    }

    /// A closed polyline lying on the ground, given as offsets from the origin.
    fn ground_loop(&mut self, o: Origin, offsets: &[(f32, f32)], colour: [u8; 4]) {
        for i in 0..offsets.len() {
            let (a, b) = (offsets[i], offsets[(i + 1) % offsets.len()]);
            self.line(o.at(a.0, a.1, LIFT), o.at(b.0, b.1, LIFT), colour);
        }
    }

    fn ring(&mut self, o: Origin, radius: f32, colour: [u8; 4]) {
        let points: Vec<_> = (0..RING_SEGMENTS)
            .map(|i| {
                let angle = i as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
                (radius * angle.cos(), radius * angle.sin())
            })
            .collect();
        self.ground_loop(o, &points, colour);
    }

    fn dashed_ring(&mut self, o: Origin, radius: f32, colour: [u8; 4]) {
        let count = ((radius * std::f32::consts::TAU * 2.0).ceil() as usize).max(64);
        for i in (0..count).step_by(2) {
            let a = i as f32 / count as f32 * std::f32::consts::TAU;
            let b = (i + 1) as f32 / count as f32 * std::f32::consts::TAU;
            self.line(
                o.at(radius * a.cos(), radius * a.sin(), LIFT),
                o.at(radius * b.cos(), radius * b.sin(), LIFT),
                colour,
            );
        }
    }

    fn building_ghost(&mut self, o: Origin, view: &EntityView, contour: [u8; 4], dashed: bool) {
        let w = f32::from(view.footprint.0.max(1)) / 2.0;
        let h = f32::from(view.footprint.1.max(1)) / 2.0;
        let height = building_height(view);
        let mut tint = rgba(owner_color(view.owner));
        tint[3] = 48;
        let corners = [(-w, -h), (w, -h), (w, h), (-w, h)];
        for edge in 0..4 {
            let (a, b) = (corners[edge], corners[(edge + 1) % 4]);
            let length = (b.0 - a.0).hypot(b.1 - a.1);
            let steps = (length * 4.0).ceil().max(1.0) as usize;
            let normal = (-(b.1 - a.1) / length * 0.055, (b.0 - a.0) / length * 0.055);
            for i in 0..steps {
                let point = |t: f32| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                let (a, b) = (
                    point(i as f32 / steps as f32),
                    point((i + 1) as f32 / steps as f32),
                );
                self.quad(
                    [
                        o.at(a.0, a.1, LIFT),
                        o.at(b.0, b.1, LIFT),
                        o.at(b.0, b.1, height),
                        o.at(a.0, a.1, height),
                    ],
                    tint,
                );
                if !dashed || i % 2 == 0 {
                    self.quad(
                        [
                            o.at(a.0 - normal.0, a.1 - normal.1, LIFT + 0.04),
                            o.at(b.0 - normal.0, b.1 - normal.1, LIFT + 0.04),
                            o.at(b.0 + normal.0, b.1 + normal.1, LIFT + 0.04),
                            o.at(a.0 + normal.0, a.1 + normal.1, LIFT + 0.04),
                        ],
                        contour,
                    );
                }
                self.line(o.at(a.0, a.1, height), o.at(b.0, b.1, height), tint);
            }
            self.line(o.at(a.0, a.1, LIFT), o.at(a.0, a.1, height), tint);
        }
        // Tile-sized roof panels follow the same ground samples as the footprint.
        for y in 0..view.footprint.1.max(1) {
            for x in 0..view.footprint.0.max(1) {
                let (x, y) = (-w + f32::from(x), -h + f32::from(y));
                self.quad(
                    [
                        o.at(x, y, height),
                        o.at(x + 1.0, y, height),
                        o.at(x + 1.0, y + 1.0, height),
                        o.at(x, y + 1.0, height),
                    ],
                    tint,
                );
            }
        }
    }

    /// A rectangle outline on the ground, sampled every tile so it follows slopes and curvature.
    fn rect_outline(&mut self, o: Origin, min: (f32, f32), max: (f32, f32), colour: [u8; 4]) {
        let mut points = Vec::new();
        let steps = |a: f32, b: f32| ((b - a).abs().ceil() as usize).max(1);
        let (nx, ny) = (steps(min.0, max.0), steps(min.1, max.1));
        for i in 0..nx {
            points.push((min.0 + (max.0 - min.0) * i as f32 / nx as f32, min.1));
        }
        for i in 0..ny {
            points.push((max.0, min.1 + (max.1 - min.1) * i as f32 / ny as f32));
        }
        for i in 0..nx {
            points.push((max.0 - (max.0 - min.0) * i as f32 / nx as f32, max.1));
        }
        for i in 0..ny {
            points.push((min.0, max.1 - (max.1 - min.1) * i as f32 / ny as f32));
        }
        self.ground_loop(o, &points, colour);
    }
}

/// Builds the overlay geometry for one frame on a flat map. `camera_yaw` (radians) turns beams to
/// the camera.
pub fn build_overlays(
    views: &[EntityView],
    ground: &dyn Fn(f32, f32) -> f32,
    hud: &HudSnapshot,
    camera_yaw: f32,
    out: &mut OverlayGeometry,
) {
    build_overlays_on(
        views,
        &FlatGround(ground),
        [camera_yaw.cos(), 0.0, -camera_yaw.sin()],
        hud,
        out,
    );
}

/// Builds the overlay geometry for one frame on any ground; `right` is the camera's right
/// vector, which turns the bombardment beams to the viewer.
pub fn build_overlays_on(
    views: &[EntityView],
    ground: &dyn Footing,
    right: [f32; 3],
    hud: &HudSnapshot,
    out: &mut OverlayGeometry,
) {
    out.lines.clear();
    out.tris.clear();
    let gold = [250, 214, 41, 255];
    // Patrols: a dashed cyan path, all the way round the planet for a ship in orbit.
    for patrol in &hud.patrols {
        let span = patrol.span_milli as f32 / 1000.0;
        let drawn = views
            .binary_search_by_key(&patrol.entity, |v| v.id)
            .ok()
            .map(|i| views[i].altitude)
            .filter(|a| *a > 0.0 && patrol.lift_milli > 0);
        let lift = drawn.unwrap_or(patrol.lift_milli as f32 / 1000.0).max(LIFT);
        let steps = ((span * 48.0).ceil() as usize).clamp(8, 512);
        for i in (0..steps).step_by(2) {
            let t = |n: usize| n as f32 / steps as f32 * span;
            out.line(
                ground.route_point(patrol.from, patrol.to, t(i), lift),
                ground.route_point(patrol.from, patrol.to, t(i + 1), lift),
                [90, 220, 255, 230],
            );
        }
    }
    for route in &hud.trade_routes {
        let steps = (route.length.div_ceil(500) as usize).clamp(2, 4096);
        for i in (0..steps).step_by(2) {
            out.line(
                ground.route_point(route.home, route.target, i as f32 / steps as f32, LIFT),
                ground.route_point(
                    route.home,
                    route.target,
                    (i + 1) as f32 / steps as f32,
                    LIFT,
                ),
                gold,
            );
        }
        let end = if route.to_target {
            route.target
        } else {
            route.home
        };
        let o = Origin {
            ground,
            face: end.face,
            x: end.x as f32 / 1000.0,
            y: end.y as f32 / 1000.0,
            altitude: 0.0,
        };
        // Keep the destination ring outside the city's roof so it remains visible.
        let radius = views
            .iter()
            .find(|v| {
                v.building
                    && v.face == end.face
                    && (v.x - o.x).abs() < 0.001
                    && (v.y - o.y).abs() < 0.001
            })
            .map_or(0.8, |v| {
                f32::from(v.footprint.0.max(v.footprint.1)) * 0.75 + 0.5
            });
        out.ring(o, radius, gold);
        out.line(o.at(0.0, 0.0, LIFT), o.at(0.0, 0.0, 1.4), gold);
    }
    for city in &hud.city_radii {
        let origin = Origin {
            ground,
            face: city.face,
            x: city.x as f32 / 1000.0,
            y: city.y as f32 / 1000.0,
            altitude: 0.0,
        };
        if city.economic > 0 {
            out.dashed_ring(origin, city.economic as f32 / 1000.0, [235, 235, 235, 210]);
        }
        if city.spacing > 0 {
            out.dashed_ring(origin, city.spacing as f32 / 1000.0, RED);
        }
    }
    for view in views
        .iter()
        .filter(|v| v.building && v.under_construction == Some(0.0))
    {
        let origin = Origin {
            ground,
            face: view.face,
            x: view.x,
            y: view.y,
            altitude: view.altitude,
        };
        out.building_ghost(origin, view, rgba(owner_color(view.owner)), true);
    }
    for view in views {
        let Some(target) = view.beam_target.filter(|_| view.altitude > 0.0) else {
            continue;
        };
        let from = entity_frame(view, ground).at([0.0, unit_width(view) * 0.15, 0.0]);
        let target_height = views
            .iter()
            .find(|v| {
                v.face == target.face
                    && (v.x * 1000.0 - target.x as f32).abs() < 1.0
                    && (v.y * 1000.0 - target.y as f32).abs() < 1.0
                    && v.altitude == 0.0
            })
            .map_or(0.8, |v| {
                if v.building {
                    building_height(v) + 0.2
                } else {
                    crate::entities::unit_height(v) * 0.5
                }
            });
        let to = ground.point(
            target.face,
            target.x as f32 / 1000.0,
            target.y as f32 / 1000.0,
            (0.0, 0.0),
            target_height,
        );
        let direction = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let mut side = [
            direction[1] * right[2] - direction[2] * right[1],
            direction[2] * right[0] - direction[0] * right[2],
            direction[0] * right[1] - direction[1] * right[0],
        ];
        let length = side.iter().map(|v| v * v).sum::<f32>().sqrt().max(0.001);
        // A wide glow round the shot from orbit (task 75), the hot core drawn over it.
        for (half, colour) in [(0.6, [255, 170, 80, 60]), (0.25, [255, 220, 150, 140])] {
            let glow = side.map(|v| v / length * half);
            let offset = |p: [f32; 3], sign: f32| -> [f32; 3] {
                std::array::from_fn(|i| p[i] + glow[i] * sign)
            };
            out.quad(
                [
                    offset(from, -1.0),
                    offset(from, 1.0),
                    offset(to, 1.0),
                    offset(to, -1.0),
                ],
                colour,
            );
        }
        side = side.map(|v| v / length * 0.07);
        let offset = |p: [f32; 3], sign: f32| std::array::from_fn(|i| p[i] + side[i] * sign);
        out.quad(
            [
                offset(from, -1.0),
                offset(from, 1.0),
                offset(to, 1.0),
                offset(to, -1.0),
            ],
            [90, 200, 255, 220],
        );
        out.line(from, to, [230, 250, 255, 255]);
        let impact = Origin {
            ground,
            face: target.face,
            x: target.x as f32 / 1000.0,
            y: target.y as f32 / 1000.0,
            altitude: 0.0,
        };
        out.ring(impact, 0.8, [255, 190, 70, 255]);
        out.line(to, impact.at(0.0, 0.0, 2.0), [255, 235, 160, 255]);
    }
    effects::bombardment(out, ground, right, views, hud);
    // Shots to or from what is aloft (batteries firing up, dogfights in orbit) and from
    // defences: a tracer for what hits at once, an arrow in flight for a projectile.
    for view in views {
        let Some(target) = view
            .shot_at
            .and_then(|id| views.iter().find(|v| v.id == id))
            .filter(|t| view.altitude > 0.0 || t.altitude > 0.0 || view.building)
        else {
            continue;
        };
        let centre = |v: &EntityView| {
            let height = if v.building {
                building_height(v) * 0.8
            } else {
                crate::entities::unit_height(v) * 0.5
            };
            entity_frame(v, ground).at([0.0, height, 0.0])
        };
        let (from, to) = (centre(view), centre(target));
        let along =
            |t: f32| -> [f32; 3] { std::array::from_fn(|i| from[i] + (to[i] - from[i]) * t) };
        if let Some(flight) = view.shot_flight {
            // An arrow: a short dark shaft on a shallow arc, its head bright.
            let length = (0..3)
                .map(|i| (to[i] - from[i]).powi(2))
                .sum::<f32>()
                .sqrt();
            let up = ground.frame(view.face, view.x, view.y).up;
            let arc = |t: f32| -> [f32; 3] {
                let p = along(t);
                let lift = 4.0 * t * (1.0 - t) * length * 0.06;
                std::array::from_fn(|i| p[i] + up[i] * lift)
            };
            // A shaft of a tile and a half with flat sides across and up, so that it shows from
            // above and from the side.
            let (tail, head) = (arc((flight - 1.5 / length.max(1.0)).max(0.0)), arc(flight));
            let d: [f32; 3] = std::array::from_fn(|i| head[i] - tail[i]);
            let across = [
                d[1] * up[2] - d[2] * up[1],
                d[2] * up[0] - d[0] * up[2],
                d[0] * up[1] - d[1] * up[0],
            ];
            let norm = across.iter().map(|v| v * v).sum::<f32>().sqrt().max(0.001);
            for side in [across.map(|v| v / norm * 0.05), up.map(|v| v * 0.05)] {
                let offset =
                    |p: [f32; 3], sign: f32| std::array::from_fn(|i| p[i] + side[i] * sign);
                out.quad(
                    [
                        offset(tail, -1.0),
                        offset(tail, 1.0),
                        offset(head, 1.0),
                        offset(head, -1.0),
                    ],
                    [60, 40, 22, 255],
                );
            }
            let tip = arc((flight - 0.3 / length.max(1.0)).max(0.0));
            out.line(tip, head, [250, 240, 215, 255]);
            continue;
        }
        // Thin lines only: their width never follows the ship view scale. A bright streak
        // marks the head of the shot.
        out.line(from, to, [255, 150, 60, 200]);
        out.line(along(0.7), to, [255, 240, 200, 255]);
        let hit = Origin {
            ground,
            face: target.face,
            x: target.x,
            y: target.y,
            altitude: target.altitude,
        };
        out.ring(hit, 0.6, [255, 200, 90, 255]);
    }
    for id in &hud.selected {
        let Ok(index) = views.binary_search_by_key(id, |v| v.id) else {
            continue;
        };
        let view = &views[index];
        let colour = rgba(owner_color(view.owner));
        let origin = Origin {
            ground,
            face: view.face,
            x: view.x,
            y: view.y,
            altitude: view.altitude,
        };
        if view.building {
            let (w, h) = (
                f32::from(view.footprint.0.max(1)),
                f32::from(view.footprint.1.max(1)),
            );
            for pad in [0.12, 0.2] {
                out.rect_outline(
                    origin,
                    (-w / 2.0 - pad, -h / 2.0 - pad),
                    (w / 2.0 + pad, h / 2.0 + pad),
                    colour,
                );
            }
        } else {
            let radius = unit_width(view) * 0.75;
            for extra in [0.0, 0.05, 0.1] {
                out.ring(origin, radius + extra, colour);
            }
        }
    }
    // Cities a trader could run a route to, while one is picked: gold rings round them.
    for id in &hud.trade_targets {
        let Ok(index) = views.binary_search_by_key(id, |v| v.id) else {
            continue;
        };
        let view = &views[index];
        let origin = Origin {
            ground,
            face: view.face,
            x: view.x,
            y: view.y,
            altitude: 0.0,
        };
        let (w, h) = (
            f32::from(view.footprint.0.max(1)) / 2.0,
            f32::from(view.footprint.1.max(1)) / 2.0,
        );
        for pad in [0.6, 0.75, 0.9] {
            out.rect_outline(origin, (-w - pad, -h - pad), (w + pad, h + pad), gold);
        }
    }
    for marker in &hud.markers {
        let (x, y) = (marker.x as f32 / 1000.0, marker.y as f32 / 1000.0);
        let origin = Origin {
            ground,
            face: marker.face,
            x,
            y,
            altitude: 0.0,
        };
        let base = origin.at(0.0, 0.0, LIFT);
        let frame = ground.frame(marker.face, x, y);
        let lifted = |h: f32| -> [f32; 3] { std::array::from_fn(|i| base[i] + frame.up[i] * h) };
        match marker.kind {
            MarkerKind::Move | MarkerKind::Attack => {
                let colour = if marker.kind == MarkerKind::Attack {
                    RED
                } else {
                    GREEN
                };
                let progress = (marker.age_ticks as f32 / MARKER_TICKS).clamp(0.0, 1.0);
                let radius = 0.75 - 0.4 * progress;
                out.ring(origin, radius, colour);
                out.ring(origin, radius * 0.45, colour);
                let arm = radius + 0.25;
                out.line(
                    origin.at(-arm, 0.0, LIFT),
                    origin.at(arm, 0.0, LIFT),
                    colour,
                );
                out.line(
                    origin.at(0.0, -arm, LIFT),
                    origin.at(0.0, arm, LIFT),
                    colour,
                );
            }
            MarkerKind::Rally => {
                let step = |dx: f32, h: f32| -> [f32; 3] {
                    std::array::from_fn(|i| base[i] + frame.up[i] * h + frame.ex[i] * dx)
                };
                if let Some((face, fx, fy)) = marker.from {
                    let home = Origin {
                        ground,
                        face,
                        x: fx as f32 / 1000.0,
                        y: fy as f32 / 1000.0,
                        altitude: 0.0,
                    };
                    out.line(home.at(0.0, 0.0, LIFT), base, CYAN);
                }
                out.line(base, lifted(1.6), CYAN);
                out.line(step(0.02, 0.0), step(0.02, 1.6), CYAN);
                out.tri([lifted(1.6), step(0.9, 1.3), lifted(1.0)], CYAN);
            }
        }
    }
    if let Some(p) = &hud.placement {
        let (cx, cy) = (p.x as f32 / 1000.0, p.y as f32 / 1000.0);
        let origin = Origin {
            ground,
            face: p.face,
            x: cx,
            y: cy,
            altitude: 0.0,
        };
        let (w, h) = (f32::from(p.width.max(1)), f32::from(p.height.max(1)));
        let (min, max) = ((-w / 2.0, -h / 2.0), (w / 2.0, h / 2.0));
        let (line, fill) = match p.valid {
            Some(true) => (GREEN, [60, 220, 80, 105]),
            Some(false) => (RED, [235, 60, 50, 115]),
            None => (AMBER, [240, 176, 48, 90]),
        };
        let at = |x: f32, y: f32| origin.at(x, y, LIFT + 0.03);
        for j in 0..p.height.max(1) {
            for i in 0..p.width.max(1) {
                let (x0, y0) = (min.0 + f32::from(i), min.1 + f32::from(j));
                out.quad(
                    [
                        at(x0, y0),
                        at(x0 + 1.0, y0),
                        at(x0 + 1.0, y0 + 1.0),
                        at(x0, y0 + 1.0),
                    ],
                    fill,
                );
            }
        }
        let view = EntityView {
            id: 0,
            kind: p.kind,
            owner: p.owner,
            face: p.face,
            x: cx,
            y: cy,
            facing_deg: 0.0,
            hp_frac: 1.0,
            building: true,
            footprint: (p.width, p.height),
            radius: 0.0,
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: None,
            model: None,
            altitude: 0.0,
            beam_target: None,
            shot_at: None,
            shot_flight: None,
            spacecraft: false,
            activity: Default::default(),
        };
        out.building_ghost(origin, &view, line, false);
        out.rect_outline(origin, min, max, line);
        out.rect_outline(
            origin,
            (min.0 + 0.05, min.1 + 0.05),
            (max.0 - 0.05, max.1 - 0.05),
            line,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{HudMarker, HudPlacement};

    fn unit(id: u32) -> EntityView {
        EntityView {
            id,
            kind: 1,
            owner: 1,
            face: 0,
            x: 10.0,
            y: 12.0,
            facing_deg: 0.0,
            hp_frac: 0.6,
            building: false,
            footprint: (1, 1),
            radius: 0.3,
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: None,
            model: None,
            altitude: 0.0,
            beam_target: None,
            shot_at: None,
            shot_flight: None,
            spacecraft: false,
            activity: Default::default(),
        }
    }

    #[test]
    fn route_line_conforms_to_the_globe() {
        let globe = crate::Globe::new(&crate::PlanetScene {
            n: 64,
            kinds: vec![crate::TileKind::Land; 6 * 64 * 64],
            heights: vec![0; 6 * 64 * 64],
            feature_colors: Default::default(),
            starts: vec![],
            nodes: vec![],
            sun: Default::default(),
        });
        let home = crate::SurfacePos {
            face: 0,
            x: 62_000,
            y: 32_000,
        };
        let target = globe.step_on(home, 25_000, 0);
        assert_ne!(home.face, target.face, "route crosses a face seam");
        let mut hud = HudSnapshot {
            trade_routes: vec![crate::HudTradeRoute {
                home,
                target,
                to_target: true,
                length: 25_000,
            }],
            ..HudSnapshot::default()
        };
        let mut out = OverlayGeometry::default();
        build_overlays_on(&[], &globe, [1.0, 0.0, 0.0], &hud, &mut out);
        assert!(out.lines.len() > 50);
        for line in out.lines.iter().take(25) {
            for p in [line.a, line.b] {
                let radius = p.iter().map(|x| x * x).sum::<f32>().sqrt();
                assert!((radius - globe.radius() - LIFT).abs() < 0.002);
            }
        }
        // Flat routes follow slopes too, and changing direction moves the endpoint marker.
        hud.trade_routes[0].home = crate::SurfacePos {
            face: 0,
            x: 1000,
            y: 1000,
        };
        hud.trade_routes[0].target = crate::SurfacePos {
            face: 0,
            x: 26_000,
            y: 1000,
        };
        let slope = |x: f32, y: f32| x * 0.1 + y * 0.05;
        let city = EntityView {
            building: true,
            footprint: (7, 7),
            x: 26.0,
            y: 1.0,
            ..unit(9)
        };
        build_overlays(&[city], &slope, &hud, 0.0, &mut out);
        assert!(
            (out.lines[25].a[0] - 26.0).abs() > 5.0,
            "destination ring clears the city roof"
        );
        for line in out.lines.iter().take(25) {
            for p in [line.a, line.b] {
                assert!((p[1] - slope(p[0], p[2]) - LIFT).abs() < 0.001);
            }
        }
        let pole = out.lines.last().unwrap();
        assert_eq!(pole.a[0], 26.0);
        hud.trade_routes[0].to_target = false;
        build_overlays(&[], &slope, &hud, 0.0, &mut out);
        assert_eq!(out.lines.last().unwrap().a[0], 1.0);
    }

    #[test]
    fn contour_and_ghost_follow_the_footprint() {
        let mut hud = HudSnapshot {
            placement: Some(HudPlacement {
                face: 0,
                x: 20_000,
                y: 30_000,
                width: 4,
                height: 2,
                kind: 3,
                owner: 1,
                valid: Some(true),
            }),
            ..HudSnapshot::default()
        };
        let slope = |x: f32, y: f32| x * 0.02 + y * 0.01;
        let mut out = OverlayGeometry::default();
        build_overlays(&[], &slope, &hud, 0.0, &mut out);
        let ghost: Vec<_> = out.tris.iter().filter(|t| t.colour[3] == 48).collect();
        assert!(!ghost.is_empty());
        let vertices: Vec<_> = ghost.iter().flat_map(|t| t.v).collect();
        assert!(
            vertices
                .iter()
                .all(|p| (18.0..=22.0).contains(&p[0]) && (29.0..=31.0).contains(&p[2]))
        );
        let top = 1.0 + 3.0 * 0.4 + 0.15 * 4.0;
        for p in &vertices {
            let lift = p[1] - slope(p[0], p[2]);
            assert!((lift - LIFT).abs() < 1e-4 || (lift - top).abs() < 1e-4);
        }
        assert!(out.tris.iter().any(|t| t.colour == GREEN));
        // At zero construction work, sites get the same full-height ghost and dashed contour.
        let site = EntityView {
            building: true,
            footprint: (4, 2),
            kind: 3,
            x: 20.0,
            y: 30.0,
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: Some(0.0),
            model: None,
            activity: Default::default(),
            ..unit(7)
        };
        hud.placement = None;
        build_overlays(std::slice::from_ref(&site), &slope, &hud, 0.0, &mut out);
        assert!(out.tris.iter().any(|t| t.colour[3] == 48));
        let site = EntityView {
            under_construction: Some(0.1),
            model: None,
            activity: Default::default(),
            ..site
        };
        build_overlays(&[site], &slope, &hud, 0.0, &mut out);
        assert!(out.tris.is_empty());
    }

    #[test]
    fn city_radius_circle_has_the_right_radius() {
        let hud = HudSnapshot {
            city_radii: vec![crate::HudCityRadius {
                face: 0,
                x: 20_000,
                y: 30_000,
                economic: 16_000,
                spacing: 20_000,
                minimap: true,
            }],
            ..HudSnapshot::default()
        };
        let mut out = OverlayGeometry::default();
        build_overlays(&[], &|_, _| 0.0, &hud, 0.0, &mut out);
        assert!(out.lines.len() > 100);
        for line in &out.lines {
            let radius = if line.colour == RED { 20.0 } else { 16.0 };
            for p in [line.a, line.b] {
                assert!(((p[0] - 20.0).hypot(p[2] - 30.0) - radius).abs() < 1e-4);
                assert_eq!(p[1], LIFT);
            }
        }
    }

    #[test]
    fn ghost_and_city_circles_conform_to_the_globe() {
        let n = 64;
        let globe = crate::Globe::new(&crate::PlanetScene {
            n,
            kinds: vec![crate::TileKind::Land; 6 * (n * n) as usize],
            heights: vec![0; 6 * (n * n) as usize],
            feature_colors: Default::default(),
            starts: vec![],
            nodes: vec![],
            sun: Default::default(),
        });
        let hud = HudSnapshot {
            placement: Some(HudPlacement {
                face: 2,
                x: 32_000,
                y: 32_000,
                width: 7,
                height: 7,
                kind: 1,
                owner: 1,
                valid: Some(true),
            }),
            city_radii: vec![crate::HudCityRadius {
                face: 2,
                x: 32_000,
                y: 32_000,
                economic: 16_000,
                spacing: 0,
                minimap: false,
            }],
            ..HudSnapshot::default()
        };
        let mut out = OverlayGeometry::default();
        build_overlays_on(&[], &globe, [1.0, 0.0, 0.0], &hud, &mut out);
        let radius = crate::globe::planet_radius(n);
        for line in &out.lines {
            for p in [line.a, line.b] {
                let length = p.iter().map(|v| v * v).sum::<f32>().sqrt();
                assert!(length >= radius - 0.001 && length < radius + 4.0);
            }
        }
        assert!(out.tris.iter().any(|t| t.colour[3] == 48));
    }

    #[test]
    fn selected_unit_gets_a_ring_and_a_health_bar() {
        let hud = HudSnapshot {
            selected: vec![1],
            ..HudSnapshot::default()
        };
        let mut out = OverlayGeometry::default();
        build_overlays(&[unit(1), unit(2)], &|_, _| 0.0, &hud, 0.0, &mut out);
        assert_eq!(out.lines.len(), RING_SEGMENTS * 3);
        // Background and fill, two triangles each.
        assert_eq!(out.tris.len(), 4);
        assert!(out.lines.iter().all(|l| l.colour == rgba(owner_color(1))));
        // An unselected unit draws nothing.
        build_overlays(&[unit(2)], &|_, _| 0.0, &hud, 0.0, &mut out);
        assert!(out.lines.is_empty() && out.tris.is_empty());
    }

    #[test]
    fn orbital_selection_and_bombard_beam_follow_the_ships_altitude() {
        let mut ship = unit(1);
        ship.altitude = 8.0;
        ship.beam_target = Some(rom_geo::SurfacePos {
            face: 0,
            x: 32_000,
            y: 32_000,
        });
        let hud = HudSnapshot {
            selected: vec![1],
            ..HudSnapshot::default()
        };
        let mut out = OverlayGeometry::default();
        build_overlays(&[ship], &|_, _| 2.0, &hud, 0.0, &mut out);
        let selection = rgba(owner_color(ship.owner));
        let rings: Vec<_> = out.lines.iter().filter(|l| l.colour == selection).collect();
        assert_eq!(rings.len(), RING_SEGMENTS * 3);
        assert!(rings.iter().all(|l| (l.a[1] - (8.0 + LIFT)).abs() < 0.001));
        let beam = out
            .lines
            .iter()
            .find(|l| l.colour == [230, 250, 255, 255])
            .unwrap();
        assert!(beam.a[1] > 8.0);
        assert!((beam.b[1] - 2.8).abs() < 0.001);
        assert_eq!((beam.b[0], beam.b[2]), (32.0, 32.0));
        ship.altitude = 0.0;
        build_overlays(&[ship], &|_, _| 2.0, &hud, 0.0, &mut out);
        assert!(!out.lines.iter().any(|l| l.colour == [230, 250, 255, 255]));
    }

    #[test]
    fn placement_footprint_is_green_or_red() {
        let mut hud = HudSnapshot {
            placement: Some(HudPlacement {
                face: 0,
                width: 3,
                height: 2,
                x: 20_500,
                y: 30_000,
                kind: 1,
                owner: 1,
                valid: Some(true),
            }),
            ..HudSnapshot::default()
        };
        let mut out = OverlayGeometry::default();
        build_overlays(&[], &|_, _| 0.0, &hud, 0.0, &mut out);
        // 6 tiles, 2 triangles each.
        assert!(out.tris.len() > 12);
        let green = out.tris[0].colour;
        assert!(green[1] > green[0]);
        hud.placement.as_mut().unwrap().valid = Some(false);
        build_overlays(&[], &|_, _| 0.0, &hud, 0.0, &mut out);
        let red = out.tris[0].colour;
        assert!(red[0] > red[1]);
    }

    #[test]
    fn move_and_attack_markers_shrink() {
        let marker = |kind, age_ticks| HudMarker {
            face: 0,
            x: 5000,
            y: 5000,
            kind,
            age_ticks,
            from: None,
        };
        let radius_of = |age| {
            let hud = HudSnapshot {
                markers: vec![marker(MarkerKind::Move, age)],
                ..HudSnapshot::default()
            };
            let mut out = OverlayGeometry::default();
            build_overlays(&[], &|_, _| 0.0, &hud, 0.0, &mut out);
            out.lines[0].a[0] - 5.0
        };
        assert!(radius_of(8) < radius_of(0));
    }
}

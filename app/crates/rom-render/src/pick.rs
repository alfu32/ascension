//! Picking: the ray through the cursor, terrain and entity hit tests, box selection and HUD
//! hit tests, plus the state machine that turns pointer input into [`PickEvent`]s.
//!
//! Everything is plain arithmetic on a [`View`] so it runs without a window. The render thread
//! owns the camera and the interpolated entity positions, so it alone picks; the events go to
//! the simulation thread, which owns the selection. Positions in events are tiles (east, south).

use rom_geo::SurfacePos;

use crate::camera::{Camera, InputEvent, Modifiers, PointerButton};
use crate::entities::{EntityView, box_corners, pick_box};
use crate::globe::{Globe, SurfaceHit};
use crate::hud::HudSnapshot;
use crate::hud_layout::{HudLayout, Rect, Region};
use crate::hud_minimap::{FOV_DEGREES, fit_map, ndc_centre_y};
use crate::orbit::{OrbitCamera, dir_from_lon_lat};

/// Pointer travel, in canvas pixels, after which a press becomes a box drag.
pub const DRAG_PIXELS: f32 = 6.0;
/// Longest time between two clicks that make a double click.
pub const DOUBLE_CLICK_MS: u64 = 400;
/// Extra reach, in pixels, around a projected entity that still counts as a hit.
const HIT_PADDING: f32 = 5.0;
/// Farthest distance a terrain ray is followed, in tiles.
const RAY_LIMIT: f32 = 6000.0;
/// Pointer movement, in tiles, that sends a new placement hover.
const HOVER_STEP: f32 = 0.2;

/// Which kind of HUD element a press landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudRegionKind {
    /// One of the four palette tabs; the index is the tab.
    PaletteTab,
    /// Palette page arrow; 0 for previous, 1 for next.
    PaletteArrow,
    /// Command slot, 0 to 14.
    CommandSlot,
    /// Production queue row, 0 to 2.
    QueueItem,
    /// Multi-selection tile, 0 to 11.
    SelectionTile,
    /// The minimap picture.
    Minimap,
    /// Minimap button: zoom in, zoom out, centre on selection, signal.
    MinimapButton,
    /// Any other HUD surface; it consumes the press.
    Panel,
}

/// A HUD element under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudTarget {
    /// The kind of element.
    pub region: HudRegionKind,
    /// Element number within its kind.
    pub index: usize,
}

/// What the render thread tells the simulation thread about the pointer and keys.
#[derive(Clone, Debug, PartialEq)]
pub enum PickEvent {
    /// A left click in the world.
    Click {
        /// Ground point under the cursor, or the entity's position when the ray missed.
        world_point: (f32, f32),
        /// The same point as a surface position (face 0 on the flat map).
        surface: SurfacePos,
        /// The entity under the cursor, nearest to the camera.
        entity: Option<u32>,
        /// Modifiers held.
        modifiers: Modifiers,
        /// True for the second click of a double click.
        double: bool,
        /// With `double`: every entity whose centre is on screen.
        on_screen: Vec<u32>,
    },
    /// A rectangle dragged in the world.
    BoxSelect {
        /// Entities whose projected centres lie inside the rectangle.
        entities: Vec<u32>,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// A right click in the world or on the minimap.
    RightClick {
        /// Ground point under the cursor, or the entity's position when the ray missed.
        world_point: (f32, f32),
        /// The same point as a surface position (face 0 on the flat map).
        surface: SurfacePos,
        /// The entity under the cursor, nearest to the camera.
        entity: Option<u32>,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// A press on a HUD element.
    HudClick {
        /// The kind of element.
        region: HudRegionKind,
        /// Element number within its kind.
        index: usize,
        /// The button pressed.
        button: PointerButton,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// The pointer moved over the world while a building is being placed.
    Hover {
        /// Ground point under the cursor.
        world_point: (f32, f32),
        /// The same point as a surface position (face 0 on the flat map).
        surface: SurfacePos,
    },
    /// A letter or digit key.
    Hotkey {
        /// The character, lower case.
        ch: char,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// Escape.
    Escape,
}

/// A camera, a canvas size and the vanishing-point shift, enough to cast and project rays.
#[derive(Clone, Copy, Debug)]
pub struct View {
    eye: [f32; 3],
    forward: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    tan: f32,
    aspect: f32,
    centre_y: f32,
    size: (f32, f32),
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt().max(1e-6);
    v.map(|c| c / length)
}

impl View {
    /// The view of `camera` onto a canvas of `size` pixels whose projection centre is shifted to
    /// NDC height `centre_y` (0 without a HUD).
    pub fn new(camera: &Camera, size: (f32, f32), centre_y: f32) -> Self {
        let eye = camera.position();
        let forward = unit(sub(camera.target, eye));
        let right = unit(cross(forward, [0.0, 1.0, 0.0]));
        let up = cross(right, forward);
        Self {
            eye,
            forward,
            right,
            up,
            tan: (FOV_DEGREES.to_radians() / 2.0).tan(),
            aspect: size.0 / size.1.max(1.0),
            centre_y,
            size,
        }
    }

    /// The view of an orbit camera onto a canvas of `size` pixels; see [`View::new`].
    pub fn from_orbit(orbit: &OrbitCamera, size: (f32, f32), centre_y: f32) -> Self {
        let eye = orbit.position();
        let forward = orbit.forward();
        let right = unit(cross(forward, orbit.view_up()));
        let up = cross(right, forward);
        Self {
            eye,
            forward,
            right,
            up,
            tan: (FOV_DEGREES.to_radians() / 2.0).tan(),
            aspect: size.0 / size.1.max(1.0),
            centre_y,
            size,
        }
    }

    /// Eye position in world space.
    pub fn eye(&self) -> [f32; 3] {
        self.eye
    }

    /// Unit direction of the ray through canvas pixel (`px`, `py`).
    pub fn ray(&self, px: f32, py: f32) -> [f32; 3] {
        let nx = 2.0 * px / self.size.0.max(1.0) - 1.0;
        let ny = 1.0 - 2.0 * py / self.size.1.max(1.0);
        let sx = nx * self.aspect * self.tan;
        let sy = (ny - self.centre_y) * self.tan;
        unit([
            self.forward[0] + self.right[0] * sx + self.up[0] * sy,
            self.forward[1] + self.right[1] * sx + self.up[1] * sy,
            self.forward[2] + self.right[2] * sx + self.up[2] * sy,
        ])
    }

    /// Canvas pixel and depth of a world point `[east, up, south]`; `None` behind the camera.
    pub fn project(&self, point: [f32; 3]) -> Option<(f32, f32, f32)> {
        let d = sub(point, self.eye);
        let depth = dot(d, self.forward);
        if depth <= 0.05 {
            return None;
        }
        let nx = dot(d, self.right) / (depth * self.tan * self.aspect);
        let ny = dot(d, self.up) / (depth * self.tan) + self.centre_y;
        Some((
            (nx + 1.0) / 2.0 * self.size.0,
            (1.0 - ny) / 2.0 * self.size.1,
            depth,
        ))
    }

    /// THE camera-ray to ground computation: the only place that intersects a ray with the
    /// ground. Click picks, placement hover and every other ground lookup go through it (via
    /// [`PickContext::ground_hit`]). On the flat map it marches the ray against the height
    /// function ([`View::terrain_point`]) and returns face 0 with the point's x/y; on a sphere
    /// it intersects the displaced surface and returns the face and position under the hit.
    /// Entity hit tests and box selection use projected entity positions instead.
    pub fn surface_point(&self, px: f32, py: f32, ground: Ground) -> Option<SurfaceHit> {
        match ground {
            Ground::Flat(height) => self.terrain_point(px, py, height).map(|point| SurfaceHit {
                point,
                pos: flat_surface((point[0], point[2])),
            }),
            Ground::Globe(globe) => globe.raycast(self.eye, self.ray(px, py)),
        }
    }

    /// Flat-map ray cast. Where the ray through pixel (`px`, `py`) meets the surface `ground` (height in tiles at
    /// east, south): marched in growing steps, then refined by bisection. Returns
    /// `[east, up, south]`, or `None` when it points at the sky or leaves the range.
    pub fn terrain_point(
        &self,
        px: f32,
        py: f32,
        ground: &dyn Fn(f32, f32) -> f32,
    ) -> Option<[f32; 3]> {
        let dir = self.ray(px, py);
        if dir[1] >= -1e-4 {
            return None;
        }
        let at = |t: f32| {
            [
                self.eye[0] + dir[0] * t,
                self.eye[1] + dir[1] * t,
                self.eye[2] + dir[2] * t,
            ]
        };
        let above = |t: f32| {
            let p = at(t);
            p[1] - ground(p[0], p[2])
        };
        let mut t = 0.0_f32;
        let (mut near, mut far) = loop {
            let next = t + (t * 0.02).max(0.25);
            if next > RAY_LIMIT {
                return None;
            }
            if above(next) <= 0.0 {
                break (t, next);
            }
            t = next;
        };
        for _ in 0..28 {
            let mid = (near + far) / 2.0;
            if above(mid) > 0.0 {
                near = mid;
            } else {
                far = mid;
            }
        }
        let mut point = at((near + far) / 2.0);
        point[1] = ground(point[0], point[2]);
        Some(point)
    }
}

/// The ground a ray is cast against.
#[derive(Clone, Copy)]
pub enum Ground<'a> {
    /// A flat map: terrain height in tiles at (east, south).
    Flat(&'a dyn Fn(f32, f32) -> f32),
    /// A sphere planet.
    Globe(&'a Globe),
}

/// The surface position of a flat-map point in tiles: face 0 and the point in milli-tiles.
pub fn flat_surface(point: (f32, f32)) -> SurfacePos {
    SurfacePos {
        face: 0,
        x: (point.0 * 1000.0).round() as i64,
        y: (point.1 * 1000.0).round() as i64,
    }
}

/// Convex hull of `points` (monotone chain), counter-clockwise in a y-up sense.
fn convex_hull(mut points: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    points.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let turn = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f32, f32)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &(f32, f32)>> = if pass == 0 {
            Box::new(points.iter())
        } else {
            Box::new(points.iter().rev())
        };
        for &p in iter {
            while hull.len() >= start + 2
                && turn(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// True when `p` is inside the convex polygon or within `pad` of its outline.
fn near_hull(hull: &[(f32, f32)], p: (f32, f32), pad: f32) -> bool {
    match hull.len() {
        0 => false,
        1 => (hull[0].0 - p.0).hypot(hull[0].1 - p.1) <= pad,
        n => {
            let mut inside = true;
            for i in 0..n {
                let (a, b) = (hull[i], hull[(i + 1) % n]);
                let edge = (b.0 - a.0, b.1 - a.1);
                let to = (p.0 - a.0, p.1 - a.1);
                if edge.0 * to.1 - edge.1 * to.0 < 0.0 {
                    inside = false;
                }
                let length2 = edge.0 * edge.0 + edge.1 * edge.1;
                let t = if length2 > 0.0 {
                    ((to.0 * edge.0 + to.1 * edge.1) / length2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if (to.0 - edge.0 * t).hypot(to.1 - edge.1 * t) <= pad {
                    return true;
                }
            }
            inside
        }
    }
}

/// Everything a pick needs to know about the frame.
pub struct PickContext<'a> {
    /// HUD layout in canvas pixels.
    pub layout: HudLayout,
    /// View of the canvas.
    pub view: View,
    /// Interpolated entities, ascending by id.
    pub entities: &'a [EntityView],
    /// Terrain height in tiles at (east, south).
    pub ground: &'a dyn Fn(f32, f32) -> f32,
    /// The sphere planet, when the match is on one; rays are cast against it instead of `ground`.
    pub globe: Option<&'a Globe>,
    /// Latest interface snapshot.
    pub hud: &'a HudSnapshot,
    /// Whether the HUD is drawn (and so consumes presses).
    pub hud_visible: bool,
    /// Map size in tiles.
    pub map_size: (u32, u32),
    /// Monotonic milliseconds, for double clicks.
    pub now_ms: u64,
}

/// The HUD element at canvas pixel (`x`, `y`), if the HUD covers it.
pub fn hud_target(
    layout: &HudLayout,
    hud: &HudSnapshot,
    map_size: (u32, u32),
    x: f32,
    y: f32,
) -> Option<HudTarget> {
    if !layout.contains_hud(x, y) {
        return None;
    }
    let target = |region, index| Some(HudTarget { region, index });
    for i in 0..4 {
        if layout.minimap_button(i).contains(x, y) {
            return target(HudRegionKind::MinimapButton, i);
        }
    }
    let view = layout.rect(Region::MinimapView);
    if fit_map(map_size.0, map_size.1, view).contains(x, y) {
        return target(HudRegionKind::Minimap, 0);
    }
    for i in 0..4 {
        if layout.palette_tab(i).contains(x, y) {
            return target(HudRegionKind::PaletteTab, i);
        }
    }
    for i in 0..2 {
        if layout.palette_arrow(i).contains(x, y) {
            return target(HudRegionKind::PaletteArrow, i);
        }
    }
    for row in 0..crate::hud_layout::GRID_ROWS {
        for col in 0..crate::hud_layout::GRID_COLUMNS {
            if layout.command_slot(col, row).contains(x, y) {
                return target(HudRegionKind::CommandSlot, row * 5 + col);
            }
        }
    }
    let queued = hud.queue.as_ref().map_or(0, |q| q.items.len());
    for i in 0..queued.min(3) {
        if layout.queue_row(i).contains(x, y) {
            return target(HudRegionKind::QueueItem, i);
        }
    }
    if let crate::hud::HudSelection::Multiple { tiles, .. } = &hud.selection {
        for i in 0..tiles.len().min(12) {
            if layout.multi_tile(i).contains(x, y) {
                return target(HudRegionKind::SelectionTile, i);
            }
        }
    }
    target(HudRegionKind::Panel, 0)
}

impl PickContext<'_> {
    fn centre_of(&self, view: &EntityView) -> [f32; 3] {
        pick_box(view, self.ground).0
    }

    /// The entity under canvas pixel (`x`, `y`): the one nearest to the camera whose projected
    /// box covers the pixel.
    pub fn entity_at(&self, x: f32, y: f32) -> Option<u32> {
        let mut best: Option<(f32, u32)> = None;
        for entity in self.entities {
            let (centre, size, yaw) = pick_box(entity, self.ground);
            let corners = box_corners(centre, size, yaw);
            let projected: Vec<_> = corners
                .iter()
                .filter_map(|c| self.view.project(*c))
                .collect();
            if projected.len() < corners.len() {
                continue;
            }
            let hull = convex_hull(projected.iter().map(|p| (p.0, p.1)).collect());
            if !near_hull(&hull, (x, y), HIT_PADDING) {
                continue;
            }
            let Some((_, _, depth)) = self.view.project(centre) else {
                continue;
            };
            if best.is_none_or(|(nearest, _)| depth < nearest) {
                best = Some((depth, entity.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Entities whose projected centres lie inside the pixel rectangle, in id order.
    pub fn entities_in_rect(&self, rect: Rect) -> Vec<u32> {
        self.entities
            .iter()
            .filter(|entity| {
                self.view
                    .project(self.centre_of(entity))
                    .is_some_and(|(x, y, _)| rect.contains(x, y))
            })
            .map(|entity| entity.id)
            .collect()
    }

    /// The ground under a pixel: the point in tiles (face-local milli-tiles / 1000 on a sphere)
    /// and the surface position.
    pub fn ground_hit(&self, x: f32, y: f32) -> Option<((f32, f32), SurfacePos)> {
        let ground = match self.globe {
            Some(globe) => Ground::Globe(globe),
            None => Ground::Flat(self.ground),
        };
        self.view.surface_point(x, y, ground).map(|hit| {
            let point = match self.globe {
                Some(_) => (hit.pos.x as f32 / 1000.0, hit.pos.y as f32 / 1000.0),
                None => (hit.point[0], hit.point[2]),
            };
            (point, hit.pos)
        })
    }

    /// The ground under a pixel, or the position of `entity` when the ray missed.
    fn ground_or_entity(
        &self,
        x: f32,
        y: f32,
        entity: Option<u32>,
    ) -> Option<((f32, f32), SurfacePos)> {
        self.ground_hit(x, y).or_else(|| {
            entity
                .and_then(|id| self.entity_position(id))
                .map(|point| (point, flat_surface(point)))
        })
    }

    fn entity_position(&self, id: u32) -> Option<(f32, f32)> {
        self.entities
            .binary_search_by_key(&id, |e| e.id)
            .ok()
            .map(|i| (self.entities[i].x, self.entities[i].y))
    }
}

#[derive(Clone, Copy, Debug)]
struct Press {
    x: f32,
    y: f32,
    mods: Modifiers,
    dragging: bool,
}

/// Turns pointer input into pick events and tracks what the pointer is over.
#[derive(Clone, Debug, Default)]
pub struct PickController {
    pointer: Option<(f32, f32)>,
    press: Option<Press>,
    last_click: Option<(u64, f32, f32)>,
    last_hover: Option<(f32, f32)>,
}

impl PickController {
    /// The HUD element under the pointer, for rollover help.
    pub fn hover(&self, ctx: &PickContext) -> Option<HudTarget> {
        let (x, y) = self.pointer?;
        if !ctx.hud_visible {
            return None;
        }
        hud_target(&ctx.layout, ctx.hud, ctx.map_size, x, y)
    }

    /// The box being dragged, in canvas pixels.
    pub fn drag_rect(&self) -> Option<Rect> {
        let press = self.press.filter(|p| p.dragging)?;
        let (x, y) = self.pointer?;
        Some(Rect::new(
            press.x.min(x),
            press.y.min(y),
            (x - press.x).abs(),
            (y - press.y).abs(),
        ))
    }

    /// Applies one input event; the returned pick events go to the simulation thread. Minimap
    /// clicks and buttons move `camera` directly, since the render thread owns it.
    pub fn handle(
        &mut self,
        event: InputEvent,
        ctx: &PickContext,
        camera: &mut Camera,
    ) -> Vec<PickEvent> {
        self.handle_with_orbit(event, ctx, camera, None)
    }

    /// Like [`Self::handle`], with the orbit camera of a sphere planet: minimap clicks and
    /// zoom buttons then move it instead of the flat camera.
    pub fn handle_with_orbit(
        &mut self,
        event: InputEvent,
        ctx: &PickContext,
        camera: &mut Camera,
        orbit: Option<&mut OrbitCamera>,
    ) -> Vec<PickEvent> {
        let mut out = Vec::new();
        match event {
            InputEvent::PointerMoved { x, y } => {
                self.pointer = Some((x, y));
                if let Some(press) = &mut self.press
                    && (x - press.x).hypot(y - press.y) > DRAG_PIXELS
                {
                    press.dragging = true;
                }
                self.hover_event(ctx, x, y, &mut out);
            }
            InputEvent::PointerLeft => self.pointer = None,
            InputEvent::ButtonDown { button, x, y, mods } => {
                self.pointer = Some((x, y));
                self.button_down(button, (x, y), mods, ctx, camera, orbit, &mut out);
            }
            InputEvent::ButtonUp {
                button: PointerButton::Left,
                x,
                y,
                mods,
            } => {
                self.pointer = Some((x, y));
                if let Some(press) = self.press.take() {
                    self.finish_press(press, (x, y), mods, ctx, &mut out);
                }
            }
            InputEvent::Hotkey { ch, mods } => out.push(PickEvent::Hotkey {
                ch,
                modifiers: mods,
            }),
            InputEvent::Escape => out.push(PickEvent::Escape),
            InputEvent::ReleaseAll => self.press = None,
            _ => {}
        }
        out
    }

    fn hover_event(&mut self, ctx: &PickContext, x: f32, y: f32, out: &mut Vec<PickEvent>) {
        if ctx.hud.placement.is_none() || self.over_hud(ctx, x, y) {
            return;
        }
        if let Some((point, surface)) = ctx.ground_hit(x, y)
            && self
                .last_hover
                .is_none_or(|last| (last.0 - point.0).hypot(last.1 - point.1) >= HOVER_STEP)
        {
            self.last_hover = Some(point);
            out.push(PickEvent::Hover {
                world_point: point,
                surface,
            });
        }
    }

    fn over_hud(&self, ctx: &PickContext, x: f32, y: f32) -> bool {
        ctx.hud_visible && ctx.layout.contains_hud(x, y)
    }

    #[allow(clippy::too_many_arguments)]
    fn button_down(
        &mut self,
        button: PointerButton,
        (x, y): (f32, f32),
        mods: Modifiers,
        ctx: &PickContext,
        camera: &mut Camera,
        orbit: Option<&mut OrbitCamera>,
        out: &mut Vec<PickEvent>,
    ) {
        if self.over_hud(ctx, x, y) {
            self.press = None;
            let Some(target) = hud_target(&ctx.layout, ctx.hud, ctx.map_size, x, y) else {
                return;
            };
            self.hud_press(button, target, (x, y), mods, ctx, camera, orbit, out);
            return;
        }
        match button {
            PointerButton::Left => {
                self.press = Some(Press {
                    x,
                    y,
                    mods,
                    dragging: false,
                });
            }
            PointerButton::Right => {
                let entity = ctx.entity_at(x, y);
                if let Some((world_point, surface)) = ctx.ground_or_entity(x, y, entity) {
                    out.push(PickEvent::RightClick {
                        world_point,
                        surface,
                        entity,
                        modifiers: mods,
                    });
                }
            }
            PointerButton::Middle => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn hud_press(
        &mut self,
        button: PointerButton,
        target: HudTarget,
        (x, y): (f32, f32),
        mods: Modifiers,
        ctx: &PickContext,
        camera: &mut Camera,
        orbit: Option<&mut OrbitCamera>,
        out: &mut Vec<PickEvent>,
    ) {
        match (target.region, button) {
            (HudRegionKind::Minimap, _) => {
                let area = fit_map(
                    ctx.map_size.0,
                    ctx.map_size.1,
                    ctx.layout.rect(Region::MinimapView),
                );
                let fraction = ((x - area.x) / area.w, (y - area.y) / area.h);
                if let Some(globe) = ctx.globe {
                    // The sphere minimap is an equirectangular image.
                    let lon = (fraction.0.clamp(0.0, 1.0) - 0.5) * std::f32::consts::TAU;
                    let lat = (0.5 - fraction.1.clamp(0.0, 1.0)) * std::f32::consts::PI;
                    let dir = dir_from_lon_lat(lon, lat);
                    match (button, orbit) {
                        (PointerButton::Left, Some(orbit)) => orbit.look_at(dir),
                        (PointerButton::Right, _) => {
                            let surface = globe.pos_of(dir);
                            out.push(PickEvent::RightClick {
                                world_point: (surface.x as f32 / 1000.0, surface.y as f32 / 1000.0),
                                surface,
                                entity: None,
                                modifiers: mods,
                            });
                        }
                        _ => {}
                    }
                    return;
                }
                let tile = (
                    fraction.0 * ctx.map_size.0 as f32,
                    fraction.1 * ctx.map_size.1 as f32,
                );
                match button {
                    PointerButton::Left => {
                        camera.target = [tile.0, 0.0, tile.1];
                        camera.pan(0.0, 0.0);
                    }
                    PointerButton::Right => out.push(PickEvent::RightClick {
                        world_point: tile,
                        surface: flat_surface(tile),
                        entity: None,
                        modifiers: mods,
                    }),
                    PointerButton::Middle => {}
                }
            }
            (HudRegionKind::MinimapButton, PointerButton::Left) => match target.index {
                0 | 1 => {
                    let steps = if target.index == 0 { 2.0 } else { -2.0 };
                    match orbit {
                        Some(orbit) => orbit.zoom(steps),
                        None => camera.zoom(steps),
                    }
                }
                2 => {
                    let mut sum = (0.0, 0.0, 0);
                    for id in &ctx.hud.selected {
                        if let Some((ex, ey)) = ctx.entity_position(*id) {
                            sum = (sum.0 + ex, sum.1 + ey, sum.2 + 1);
                        }
                    }
                    if sum.2 > 0 {
                        camera.target = [sum.0 / sum.2 as f32, 0.0, sum.1 / sum.2 as f32];
                        camera.pan(0.0, 0.0);
                    }
                }
                _ => {}
            },
            (HudRegionKind::Panel | HudRegionKind::MinimapButton, _) => {}
            (region, button) => out.push(PickEvent::HudClick {
                region,
                index: target.index,
                button,
                modifiers: mods,
            }),
        }
    }

    fn finish_press(
        &mut self,
        press: Press,
        (x, y): (f32, f32),
        mods: Modifiers,
        ctx: &PickContext,
        out: &mut Vec<PickEvent>,
    ) {
        let mods = Modifiers {
            shift: mods.shift || press.mods.shift,
            ctrl: mods.ctrl || press.mods.ctrl,
            alt: mods.alt || press.mods.alt,
        };
        if press.dragging {
            let rect = Rect::new(
                press.x.min(x),
                press.y.min(y),
                (x - press.x).abs(),
                (y - press.y).abs(),
            );
            out.push(PickEvent::BoxSelect {
                entities: ctx.entities_in_rect(rect),
                modifiers: mods,
            });
            self.last_click = None;
            return;
        }
        let entity = ctx.entity_at(x, y);
        let Some((world_point, surface)) = ctx.ground_or_entity(x, y, entity) else {
            return;
        };
        let double = self.last_click.is_some_and(|(at, lx, ly)| {
            ctx.now_ms.saturating_sub(at) <= DOUBLE_CLICK_MS
                && (lx - x).hypot(ly - y) <= DRAG_PIXELS
        });
        self.last_click = if double {
            None
        } else {
            Some((ctx.now_ms, x, y))
        };
        let on_screen = if double {
            let area = ctx.layout.world_area();
            ctx.entities_in_rect(area)
        } else {
            Vec::new()
        };
        out.push(PickEvent::Click {
            world_point,
            surface,
            entity,
            modifiers: mods,
            double,
            on_screen,
        });
    }
}

/// The canvas-space view of an orbit camera; see [`canvas_view`].
pub fn canvas_view_orbit(orbit: &OrbitCamera, layout: &HudLayout, hud_visible: bool) -> View {
    let centre_y = if hud_visible {
        ndc_centre_y(layout.world_area(), layout.height)
    } else {
        0.0
    };
    View::from_orbit(orbit, (layout.width, layout.height), centre_y)
}

/// The canvas-space view for a frame: camera, canvas size and whether a HUD shifts the centre.
pub fn canvas_view(camera: &Camera, layout: &HudLayout, hud_visible: bool) -> View {
    let centre_y = if hud_visible {
        ndc_centre_y(layout.world_area(), layout.height)
    } else {
        0.0
    };
    View::new(camera, (layout.width, layout.height), centre_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT: &dyn Fn(f32, f32) -> f32 = &|_, _| 0.0;

    fn camera() -> Camera {
        Camera {
            target: [50.0, 0.0, 60.0],
            yaw: 0.7,
            pitch: 0.9,
            distance: 30.0,
            bounds: [0.0, 0.0, 200.0, 200.0],
        }
    }

    fn unit_view(id: u32, x: f32, y: f32) -> EntityView {
        EntityView {
            id,
            kind: 1,
            owner: 1,
            x,
            y,
            facing_deg: 0.0,
            hp_frac: 1.0,
            building: false,
            footprint: (1, 1),
            radius: 0.3,
            under_construction: None,
        }
    }

    fn layout() -> HudLayout {
        HudLayout::new(1672, 941, None)
    }

    fn context<'a>(
        entities: &'a [EntityView],
        hud: &'a HudSnapshot,
        cam: &Camera,
    ) -> PickContext<'a> {
        let layout = layout();
        PickContext {
            layout,
            view: canvas_view(cam, &layout, true),
            entities,
            ground: FLAT,
            globe: None,
            hud,
            hud_visible: true,
            map_size: (200, 200),
            now_ms: 0,
        }
    }

    #[test]
    fn ray_hits_terrain_at_the_expected_point() {
        let cam = camera();
        let layout = layout();
        let view = canvas_view(&cam, &layout, true);
        // The middle of the world band looks at the camera target.
        let area = layout.world_area();
        let (cx, cy) = area.centre();
        let hit = view.terrain_point(cx, cy, FLAT).expect("hit");
        assert!(
            (hit[0] - 50.0).abs() < 0.05 && (hit[2] - 60.0).abs() < 0.05,
            "{hit:?}"
        );
        assert!(hit[1].abs() < 1e-3);
        // Projecting the hit gives the pixel back.
        let (px, py, _) = view.project(hit).expect("in front");
        assert!((px - cx).abs() < 0.1 && (py - cy).abs() < 0.1);
        // A raised plateau is met earlier along the ray, so the hit moves toward the eye.
        let plateau: &dyn Fn(f32, f32) -> f32 = &|_, _| 3.0;
        let high = view.terrain_point(cx, cy, plateau).expect("hit");
        assert!((high[1] - 3.0).abs() < 1e-3);
        let eye = cam.position();
        let d_flat = (hit[0] - eye[0]).hypot(hit[2] - eye[2]);
        let d_high = (high[0] - eye[0]).hypot(high[2] - eye[2]);
        assert!(d_high < d_flat);
        // A ray at the very top of the canvas may miss the ground entirely.
        let sky_camera = Camera {
            pitch: Camera::MIN_PITCH,
            ..cam
        };
        let sky = canvas_view(&sky_camera, &layout, false);
        assert!(sky.terrain_point(800.0, 0.0, FLAT).is_none());
    }

    #[test]
    fn box_select_uses_projected_centres() {
        let cam = camera();
        let hud = HudSnapshot::default();
        // Three units close together at the target, one far to the side.
        let entities = vec![
            unit_view(1, 50.0, 60.0),
            unit_view(2, 51.0, 60.0),
            unit_view(3, 50.0, 61.5),
            unit_view(4, 90.0, 60.0),
        ];
        let ctx = context(&entities, &hud, &cam);
        let centre = |e: &EntityView| ctx.view.project(pick_box(e, FLAT).0).expect("visible");
        let (x1, y1, _) = centre(&entities[0]);
        let (x3, y3, _) = centre(&entities[2]);
        let (x2, y2, _) = centre(&entities[1]);
        let left = x1.min(x2).min(x3) - 3.0;
        let right = x1.max(x2).max(x3) + 3.0;
        let top = y1.min(y2).min(y3) - 3.0;
        let bottom = y1.max(y2).max(y3) + 3.0;
        let picked = ctx.entities_in_rect(Rect::new(left, top, right - left, bottom - top));
        assert_eq!(picked, vec![1, 2, 3]);
        // A rectangle that covers an entity's edge but not its centre does not select it.
        let (x4, y4, _) = centre(&entities[3]);
        let edge = ctx.entities_in_rect(Rect::new(x4 + 1.0, y4 - 20.0, 40.0, 40.0));
        assert!(edge.is_empty());
        // Dragging with the controller produces the same selection.
        let mut controller = PickController::default();
        let mut camera = cam;
        let down = InputEvent::ButtonDown {
            button: PointerButton::Left,
            x: left,
            y: top,
            mods: Modifiers::default(),
        };
        assert!(controller.handle(down, &ctx, &mut camera).is_empty());
        controller.handle(
            InputEvent::PointerMoved {
                x: right,
                y: bottom,
            },
            &ctx,
            &mut camera,
        );
        assert!(controller.drag_rect().is_some());
        let events = controller.handle(
            InputEvent::ButtonUp {
                button: PointerButton::Left,
                x: right,
                y: bottom,
                mods: Modifiers {
                    shift: true,
                    ..Modifiers::default()
                },
            },
            &ctx,
            &mut camera,
        );
        match events.as_slice() {
            [
                PickEvent::BoxSelect {
                    entities,
                    modifiers,
                },
            ] => {
                assert_eq!(entities, &vec![1, 2, 3]);
                assert!(modifiers.shift);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn press_on_hud_region_is_a_hud_click() {
        let cam = camera();
        let hud = HudSnapshot {
            palette: Some(crate::hud_model::HudPalette {
                pages: 1,
                ..Default::default()
            }),
            ..HudSnapshot::default()
        };
        let entities = vec![unit_view(1, 50.0, 60.0)];
        let ctx = context(&entities, &hud, &cam);
        let slot = layout().command_slot(2, 1);
        let (x, y) = slot.centre();
        let mut controller = PickController::default();
        let mut camera = cam;
        let events = controller.handle(
            InputEvent::ButtonDown {
                button: PointerButton::Left,
                x,
                y,
                mods: Modifiers::default(),
            },
            &ctx,
            &mut camera,
        );
        assert_eq!(
            events,
            vec![PickEvent::HudClick {
                region: HudRegionKind::CommandSlot,
                index: 7,
                button: PointerButton::Left,
                modifiers: Modifiers::default(),
            }]
        );
        // Releasing over the world afterwards sends no world click.
        let up = controller.handle(
            InputEvent::ButtonUp {
                button: PointerButton::Left,
                x: 800.0,
                y: 400.0,
                mods: Modifiers::default(),
            },
            &ctx,
            &mut camera,
        );
        assert!(up.is_empty());
        // Presses on inert HUD chrome are consumed without any event; the top bar is one.
        let down = controller.handle(
            InputEvent::ButtonDown {
                button: PointerButton::Right,
                x: 400.0,
                y: 40.0,
                mods: Modifiers::default(),
            },
            &ctx,
            &mut camera,
        );
        assert!(down.is_empty());
        // The tabs and a tile of a multi-selection are HUD clicks too.
        let tab = layout().palette_tab(2).centre();
        let target = hud_target(&layout(), &hud, (200, 200), tab.0, tab.1).expect("tab");
        assert_eq!(
            target,
            HudTarget {
                region: HudRegionKind::PaletteTab,
                index: 2
            }
        );
        // The world just above the bottom groups is not HUD.
        let (ax, ay) = layout().world_area().centre();
        assert!(hud_target(&layout(), &hud, (200, 200), ax, ay).is_none());
    }

    #[test]
    fn nearest_entity_wins_pick() {
        let cam = Camera {
            yaw: 0.0,
            ..camera()
        };
        let hud = HudSnapshot::default();
        // Yaw 0 puts the eye on +south: the unit with the larger south coordinate is nearer.
        let entities = vec![unit_view(1, 50.0, 60.0), unit_view(2, 50.0, 60.3)];
        let ctx = context(&entities, &hud, &cam);
        let (x, y, _) = ctx
            .view
            .project(pick_box(&entities[0], FLAT).0)
            .expect("visible");
        assert_eq!(ctx.entity_at(x, y), Some(2));
        // Far from every entity nothing is picked.
        assert_eq!(ctx.entity_at(x + 200.0, y), None);
        // Swapping the depth order swaps the winner.
        let swapped = vec![unit_view(1, 50.0, 60.3), unit_view(2, 50.0, 60.0)];
        let ctx = context(&swapped, &hud, &cam);
        assert_eq!(ctx.entity_at(x, y), Some(1));
    }
}

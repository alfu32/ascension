//! Picking: the ray through the cursor, terrain and entity hit tests, box selection and HUD
//! hit tests, plus the state machine that turns pointer input into [`PickEvent`]s.
//!
//! Everything is plain arithmetic on a [`View`] so it runs without a window. The render thread
//! owns the camera and the interpolated entity positions, so it alone picks; the events go to
//! the simulation thread, which owns the selection. Positions in events are tiles (east, south).

use rom_geo::SurfacePos;

use crate::camera::{Camera, InputEvent, Modifiers, PointerButton};
use crate::entities::{EntityView, box_corners, box_corners_axes, pick_box, pick_frame};
use crate::globe::{Globe, SurfaceHit};
use crate::hud::HudSnapshot;
use crate::hud_layout::{HudLayout, Rect, Region};
use crate::hud_minimap::{FOV_DEGREES, fit_map, ndc_centre_y};
use crate::orbit::{OrbitCamera, dir_from_lon_lat};

/// Pointer travel, in canvas pixels, after which a press becomes a box drag.
pub const DRAG_PIXELS: f32 = 6.0;
/// Longest time between two clicks that make a double click.
pub const DOUBLE_CLICK_MS: u64 = 400;
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
    /// A group of the palette's pages listed where the queue stands (a brush family, task 114).
    PaletteGroup,
    /// Multi-selection tile, 0 to 11.
    SelectionTile,
    /// Portrait of a worker inside the selected building, 0 to 11.
    InsideWorker,
    /// The second press of a double click on an inside portrait: every worker of that type.
    InsideWorkerAll,
    /// The minimap picture.
    Minimap,
    /// The name heading the selection panel for an own city: a click opens the rename field
    /// (task 144).
    CityName,
    /// Minimap button: zoom in, zoom out, centre on selection, signal.
    MinimapButton,
    /// One of the five data tabs.
    DataTab,
    /// A button beside the clock: diplomacy, chat, menu, pause (0 to 3).
    TopButton,
    /// The chat line under the event line: a click gives it the keyboard.
    ChatLine,
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
        /// True when the click hit a place in space that a double click flies to (a planet, a
        /// star, a system, a cluster or a route's end): such a click keeps the selection.
        place: bool,
    },
    /// A rectangle dragged in the world.
    BoxSelect {
        /// Entities whose projected centres lie inside the rectangle.
        entities: Vec<u32>,
        /// Modifiers held.
        modifiers: Modifiers,
    },
    /// With a brush in hand (`HudSnapshot::painting`): the left button went down on the ground
    /// (`start`) or moved on while held. No selection box is drawn for such a press.
    Stroke {
        /// Ground point under the cursor.
        world_point: (f32, f32),
        /// The same point as a surface position.
        surface: SurfacePos,
        /// True for the press, false for a move with the button held.
        start: bool,
    },
    /// The left button of a brush stroke was released, or the stroke was cancelled.
    StrokeEnd,
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
    /// A right click in system space or on the star map. At the system level `surface` is the
    /// point of the system plane in orbit-radius units times 1000 east and south of the star;
    /// `body` names the body under the cursor, `system` the system on the star map.
    SpaceRightClick {
        /// The point under the cursor, see above.
        surface: SurfacePos,
        /// The ship under the cursor.
        entity: Option<u32>,
        /// The planet or resource body under the cursor (never a star).
        body: Option<u32>,
        /// The star system under the cursor, on the star map.
        system: Option<u32>,
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
    /// The pointer moved over the world or left it for the HUD.
    Hover {
        /// Entity under the pointer, if any.
        entity: Option<u32>,
        /// Ground point under the cursor.
        world_point: (f32, f32),
        /// The same point as a surface position (face 0 on the flat map).
        surface: SurfacePos,
    },
    /// The pointer left the world view.
    HoverExit,
    /// The entity the pointer rests on changed (task 151): the render thread picks once a
    /// frame, on the planet, in orbit and in space, and says so whenever the answer changes, even
    /// when the pointer stands still and the entity moves.
    Pointed {
        /// Entity under the pointer, if any.
        entity: Option<u32>,
    },
    /// Ctrl and the wheel over a building being placed (task 165): turn it by `steps` × 45°,
    /// positive clockwise.
    Turn {
        /// Steps of 45 degrees.
        steps: i32,
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

/// Ctrl and the wheel turn the building being placed when there is one to turn (task 165);
/// otherwise they zoom as the plain wheel does.
pub fn turn_or_zoom(event: InputEvent, hud: &HudSnapshot) -> InputEvent {
    match event {
        InputEvent::TurnWheel { steps } if !hud.turnable => InputEvent::Wheel { steps },
        other => other,
    }
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

    /// The view of a camera at `eye` looking at `target` with the given `up`, onto a canvas of
    /// `size` pixels; see [`View::new`].
    pub fn looking(
        eye: [f32; 3],
        target: [f32; 3],
        up: [f32; 3],
        size: (f32, f32),
        centre_y: f32,
    ) -> Self {
        let forward = unit(sub(target, eye));
        let right = unit(cross(forward, up));
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

    /// Canvas size in pixels.
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// Tangent of half the vertical field of view.
    pub fn tan(&self) -> f32 {
        self.tan
    }

    /// Eye position in world space.
    pub fn eye(&self) -> [f32; 3] {
        self.eye
    }

    /// World units per pixel at `point`, as [`crate::layers::FrameView::per_pixel`] reckons it.
    pub fn per_pixel(&self, point: [f32; 3]) -> f32 {
        let d = sub(point, self.eye);
        dot(d, d).sqrt() * 2.0 * self.tan / self.size.1.max(1.0)
    }

    /// Unit direction the camera looks along.
    pub fn forward(&self) -> [f32; 3] {
        self.forward
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

/// Convex hull of the first `n` of `points` (monotone chain), counter-clockwise in a y-up
/// sense, written into `hull`; returns its length. Sorts `points` in place; allocates nothing.
fn convex_hull(points: &mut [(f32, f32); 8], hull: &mut [(f32, f32); 16]) -> usize {
    points.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut unique = 0;
    for i in 0..points.len() {
        if unique == 0 || points[i] != points[unique - 1] {
            points[unique] = points[i];
            unique += 1;
        }
    }
    let points = &points[..unique];
    if points.len() < 3 {
        hull[..points.len()].copy_from_slice(points);
        return points.len();
    }
    let turn = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut len = 0;
    for pass in 0..2 {
        let start = len;
        for k in 0..points.len() {
            let p = if pass == 0 {
                points[k]
            } else {
                points[points.len() - 1 - k]
            };
            while len >= start + 2 && turn(hull[len - 2], hull[len - 1], p) <= 0.0 {
                len -= 1;
            }
            hull[len] = p;
            len += 1;
        }
        len -= 1;
    }
    len
}

/// Distance in pixels from `p` to the convex polygon `hull`: 0 inside it.
fn hull_distance(hull: &[(f32, f32)], p: (f32, f32)) -> f32 {
    match hull.len() {
        0 => f32::INFINITY,
        1 => (hull[0].0 - p.0).hypot(hull[0].1 - p.1),
        n => {
            let mut inside = n > 2;
            let mut nearest = f32::INFINITY;
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
                nearest = nearest.min((to.0 - edge.0 * t).hypot(to.1 - edge.1 * t));
            }
            if inside { 0.0 } else { nearest }
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
    for i in 0..4 {
        if layout
            .map(
                crate::hud_layout::Group::Top,
                Rect::new(1432.0 + i as f32 * 54.0, 42.0, 50.0, 50.0),
            )
            .contains(x, y)
        {
            return target(HudRegionKind::TopButton, i);
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
    for i in 0..5 {
        if layout
            .map(
                crate::hud_layout::Group::BottomRight,
                Rect::new(1350.0, 695.0 + i as f32 * 42.0, 132.0, 38.0),
            )
            .contains(x, y)
        {
            return target(HudRegionKind::DataTab, i);
        }
    }
    let groups = hud.palette.as_ref().map_or(0, |p| p.groups.len());
    for i in 0..groups.min(crate::hud_layout::PALETTE_GROUPS) {
        if layout.palette_group(i).contains(x, y) {
            return target(HudRegionKind::PaletteGroup, i);
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
    if let crate::hud::HudSelection::Single(card) = &hud.selection {
        for i in 0..card.inside.len().min(12) {
            if layout.inside_tile(i).contains(x, y) {
                return target(HudRegionKind::InsideWorker, i);
            }
        }
        if card.renamable
            && layout
                .map(
                    crate::hud_layout::Group::BottomRight,
                    crate::hud::SELECTION_HEADING,
                )
                .contains(x, y)
        {
            return target(HudRegionKind::CityName, 0);
        }
    }
    if layout
        .map(
            crate::hud_layout::Group::BottomLeft,
            Rect::new(16.0, 578.0, 362.0, 28.0),
        )
        .contains(x, y)
    {
        return target(HudRegionKind::ChatLine, 0);
    }
    target(HudRegionKind::Panel, 0)
}

impl PickContext<'_> {
    fn centre_of(&self, view: &EntityView) -> [f32; 3] {
        match self.globe {
            Some(globe) => pick_frame(view, globe).0,
            None => pick_box(view, self.ground).0,
        }
    }

    /// The eight corners of the box that stands for `view`, and its centre.
    fn box_of(&self, view: &EntityView) -> ([f32; 3], [[f32; 3]; 8]) {
        match self.globe {
            Some(globe) => {
                let (centre, axes, size) = pick_frame(view, globe);
                (centre, box_corners_axes(centre, axes, size))
            }
            None => {
                let (centre, size, yaw) = pick_box(view, self.ground);
                (centre, box_corners(centre, size, yaw))
            }
        }
    }

    /// The entity under canvas pixel (`x`, `y`): the one nearest to the camera whose projected
    /// box covers the pixel. When no box covers it, the entity whose box is nearest on screen,
    /// within [`crate::PICK_SLACK`] design pixels at the HUD's scale; ties by id (task 151).
    /// Allocates nothing, so the render thread can ask once a frame.
    pub fn entity_at(&self, x: f32, y: f32) -> Option<u32> {
        let slack = crate::PICK_SLACK * self.layout.scale.max(0.25);
        let mut on: Option<(f32, u32)> = None;
        let mut near: Option<(f32, u32)> = None;
        let mut projected = [(0.0_f32, 0.0_f32); 8];
        let mut hull = [(0.0_f32, 0.0_f32); 16];
        'entities: for entity in self.entities {
            let (centre, corners) = self.box_of(entity);
            for (slot, corner) in projected.iter_mut().zip(corners) {
                match self.view.project(corner) {
                    Some((px, py, _)) => *slot = (px, py),
                    None => continue 'entities,
                }
            }
            let n = convex_hull(&mut projected, &mut hull);
            let distance = hull_distance(&hull[..n], (x, y));
            if distance > slack {
                continue;
            }
            let Some((_, _, depth)) = self.view.project(centre) else {
                continue;
            };
            if self.hidden_by_planet(centre) {
                continue;
            }
            if distance <= 0.0 {
                if on.is_none_or(|(nearest, _)| depth < nearest) {
                    on = Some((depth, entity.id));
                }
            } else if near.is_none_or(|(d, id)| distance < d || (distance == d && entity.id < id)) {
                near = Some((distance, entity.id));
            }
        }
        on.or(near).map(|(_, id)| id)
    }

    /// Where entity `id` stands on the canvas: the centre of its pick box, if it is drawn and
    /// in front of the camera (a script points at it, task 151).
    pub fn screen_of(&self, id: u32) -> Option<(f32, f32)> {
        let index = self.entities.binary_search_by_key(&id, |e| e.id).ok()?;
        let centre = self.centre_of(&self.entities[index]);
        self.view.project(centre).map(|(x, y, _)| (x, y))
    }

    /// True when the planet's own surface lies between the eye and `point` (an entity on the far
    /// side of the globe).
    fn hidden_by_planet(&self, point: [f32; 3]) -> bool {
        let Some(globe) = self.globe else {
            return false;
        };
        let eye = self.view.eye();
        let to = sub(point, eye);
        let length = dot(to, to).sqrt().max(1e-6);
        let ray = to.map(|c| c / length);
        globe
            .raycast(eye, ray)
            .is_some_and(|hit| dot(sub(hit.point, eye), ray) < length - 0.5)
    }

    /// Entities whose projected centres lie inside the pixel rectangle, in id order.
    pub fn entities_in_rect(&self, rect: Rect) -> Vec<u32> {
        self.entities
            .iter()
            .filter(|entity| {
                let centre = self.centre_of(entity);
                self.view
                    .project(centre)
                    .is_some_and(|(x, y, _)| rect.contains(x, y) && !self.hidden_by_planet(centre))
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
        self.ground_hit(x, y)
            .or_else(|| entity.and_then(|id| self.entity_surface(id)))
    }

    /// The position of an entity as a point and a surface position.
    fn entity_surface(&self, id: u32) -> Option<((f32, f32), SurfacePos)> {
        let index = self.entities.binary_search_by_key(&id, |e| e.id).ok()?;
        let e = &self.entities[index];
        let point = (e.x, e.y);
        Some((
            point,
            SurfacePos {
                face: e.face,
                x: (e.x * 1000.0).round() as i64,
                y: (e.y * 1000.0).round() as i64,
            },
        ))
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

/// Translates the flat camera by the ground distance between two screen rays.
fn drag_pan(
    camera: &mut Camera,
    view: &View,
    from: (f32, f32),
    to: (f32, f32),
    ground: &dyn Fn(f32, f32) -> f32,
) {
    let Some(anchor) = view.terrain_point(from.0, from.1, ground) else {
        return;
    };
    let ray = view.ray(to.0, to.1);
    let eye = view.eye();
    if ray[1] >= -1e-6 {
        return;
    }
    let t = (anchor[1] - eye[1]) / ray[1];
    if t < 0.0 {
        return;
    }
    camera.target[0] += anchor[0] - (eye[0] + ray[0] * t);
    camera.target[2] += anchor[2] - (eye[2] + ray[2] * t);
    *camera = camera.limited();
}

/// Turns pointer input into pick events and tracks what the pointer is over.
#[derive(Clone, Debug, Default)]
pub struct PickController {
    pointer: Option<(f32, f32)>,
    press: Option<Press>,
    last_click: Option<(u64, f32, f32)>,
    last_inside: Option<(u64, usize)>,
    last_hover: Option<(f32, f32)>,
    pan: Option<(PointerButton, f32, f32)>,
    /// A brush stroke is under way: the left button is held with a brush in hand.
    brushing: bool,
}

impl PickController {
    /// Whether a ground drag currently owns the pointer.
    pub fn is_panning(&self) -> bool {
        self.pan.is_some()
    }

    /// Where the pointer is, in canvas pixels.
    pub fn pointer(&self) -> Option<(f32, f32)> {
        self.pointer
    }

    /// The entity under the pointer this frame (task 151): none with no pointer, over the HUD,
    /// while the ground is dragged or a selection box is drawn.
    pub fn pointed(&self, ctx: &PickContext) -> Option<u32> {
        let (x, y) = self.pointer?;
        if self.pan.is_some() || self.press.is_some_and(|p| p.dragging) || self.over_hud(ctx, x, y)
        {
            return None;
        }
        ctx.entity_at(x, y)
    }

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
        mut orbit: Option<&mut OrbitCamera>,
    ) -> Vec<PickEvent> {
        let mut out = Vec::new();
        match event {
            InputEvent::PointerMoved { x, y } => {
                if let Some((button, last_x, last_y)) = self.pan {
                    if let Some(orbit) = orbit.as_deref_mut() {
                        if let Some(globe) = ctx.globe {
                            let a = ctx.view.surface_point(last_x, last_y, Ground::Globe(globe));
                            let b = ctx.view.surface_point(x, y, Ground::Globe(globe));
                            if let (Some(a), Some(b)) = (a, b) {
                                orbit.drag_surface(b.point, a.point);
                            }
                        }
                    } else {
                        drag_pan(camera, &ctx.view, (last_x, last_y), (x, y), ctx.ground);
                    }
                    self.pan = Some((button, x, y));
                    self.pointer = Some((x, y));
                    return out;
                }
                self.pointer = Some((x, y));
                if self.brushing
                    && !self.over_hud(ctx, x, y)
                    && let Some((world_point, surface)) = ctx.ground_hit(x, y)
                {
                    out.push(PickEvent::Stroke {
                        world_point,
                        surface,
                        start: false,
                    });
                }
                if let Some(press) = &mut self.press
                    && (x - press.x).hypot(y - press.y) > DRAG_PIXELS
                {
                    press.dragging = true;
                }
                self.hover_event(ctx, x, y, &mut out);
            }
            InputEvent::PointerLeft => {
                self.pointer = None;
                self.pan = None;
                self.end_stroke(&mut out);
            }
            InputEvent::ButtonDown { button, x, y, mods } => {
                self.pointer = Some((x, y));
                if !self.over_hud(ctx, x, y)
                    && ((button == PointerButton::Middle && !mods.shift)
                        || (button == PointerButton::Right
                            && orbit.is_none()
                            && ctx.hud.selected.is_empty()
                            && ctx.hud.placement.is_none()))
                {
                    self.pan = Some((button, x, y));
                    return out;
                }
                self.button_down(button, (x, y), mods, ctx, camera, orbit, &mut out);
            }
            InputEvent::ButtonUp { button, x, y, .. }
                if self.pan.is_some_and(|p| p.0 == button) =>
            {
                self.pan = None;
                self.pointer = Some((x, y));
            }
            InputEvent::ButtonUp {
                button: PointerButton::Left,
                x,
                y,
                mods,
            } => {
                self.pointer = Some((x, y));
                self.end_stroke(&mut out);
                if let Some(press) = self.press.take() {
                    self.finish_press(press, (x, y), mods, ctx, &mut out);
                }
            }
            InputEvent::Hotkey { ch, mods } => out.push(PickEvent::Hotkey {
                ch,
                modifiers: mods,
            }),
            InputEvent::Escape => out.push(PickEvent::Escape),
            InputEvent::TurnWheel { steps } if steps != 0.0 => out.push(PickEvent::Turn {
                steps: if steps > 0.0 { 1 } else { -1 },
            }),
            InputEvent::ReleaseAll => {
                self.press = None;
                self.pan = None;
                self.end_stroke(&mut out);
            }
            _ => {}
        }
        out
    }

    fn end_stroke(&mut self, out: &mut Vec<PickEvent>) {
        if std::mem::take(&mut self.brushing) {
            out.push(PickEvent::StrokeEnd);
        }
    }

    fn hover_event(&mut self, ctx: &PickContext, x: f32, y: f32, out: &mut Vec<PickEvent>) {
        if self.over_hud(ctx, x, y) {
            self.last_hover = None;
            out.push(PickEvent::HoverExit);
            return;
        }
        if let Some((point, surface)) = ctx.ground_hit(x, y)
            && self
                .last_hover
                .is_none_or(|last| (last.0 - point.0).hypot(last.1 - point.1) >= HOVER_STEP)
        {
            self.last_hover = Some(point);
            out.push(PickEvent::Hover {
                entity: ctx.entity_at(x, y),
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
            PointerButton::Left if ctx.hud.painting => {
                // The left drag paints instead of drawing a selection box; every other button
                // and the camera keys keep their jobs.
                self.press = None;
                self.brushing = true;
                if let Some((world_point, surface)) = ctx.ground_hit(x, y) {
                    out.push(PickEvent::Stroke {
                        world_point,
                        surface,
                        start: true,
                    });
                }
            }
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
                2 if ctx.globe.is_some() => {
                    let globe = ctx.globe.expect("checked");
                    let mut sum = ([0.0_f32; 3], 0);
                    for id in &ctx.hud.selected {
                        if let Some(e) = ctx
                            .entities
                            .binary_search_by_key(id, |e| e.id)
                            .ok()
                            .map(|i| &ctx.entities[i])
                        {
                            let dir = globe.dir_of(SurfacePos {
                                face: e.face,
                                x: (e.x * 1000.0).round() as i64,
                                y: (e.y * 1000.0).round() as i64,
                            });
                            sum = (
                                [sum.0[0] + dir[0], sum.0[1] + dir[1], sum.0[2] + dir[2]],
                                sum.1 + 1,
                            );
                        }
                    }
                    if let (Some(orbit), true) = (orbit, sum.1 > 0) {
                        orbit.look_at(sum.0);
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
            (HudRegionKind::InsideWorker, PointerButton::Left) => {
                let double = self.last_inside.is_some_and(|(at, index)| {
                    index == target.index && ctx.now_ms.saturating_sub(at) <= DOUBLE_CLICK_MS
                });
                self.last_inside = if double {
                    None
                } else {
                    Some((ctx.now_ms, target.index))
                };
                out.push(PickEvent::HudClick {
                    region: if double {
                        HudRegionKind::InsideWorkerAll
                    } else {
                        HudRegionKind::InsideWorker
                    },
                    index: target.index,
                    button,
                    modifiers: mods,
                });
            }
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
            place: false,
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
            face: 0,
            x,
            y,
            facing_deg: 0.0,
            hp_frac: 1.0,
            building: false,
            footprint: (1, 1),
            radius: 0.3,
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
            hover: crate::Hover::Circle,
            cloud: None,
            activity: Default::default(),
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
    fn drag_pan_keeps_the_point_under_the_cursor() {
        for (size, centre_y) in [((1672.0, 941.0), 0.2), ((1280.0, 800.0), 0.0)] {
            let mut camera = camera();
            let from = (size.0 * 0.5, size.1 * 0.45);
            let to = (from.0 + 90.0, from.1 + 55.0);
            let before = View::new(&camera, size, centre_y);
            let anchor = before.terrain_point(from.0, from.1, FLAT).unwrap();
            drag_pan(&mut camera, &before, from, to, FLAT);
            let after = View::new(&camera, size, centre_y);
            let moved = after.terrain_point(to.0, to.1, FLAT).unwrap();
            assert!((anchor[0] - moved[0]).abs() < 0.002);
            assert!((anchor[2] - moved[2]).abs() < 0.002);
        }
    }

    #[test]
    fn drag_pan_preserves_a_point_on_a_hill() {
        let mut camera = camera();
        let slope = |x: f32, y: f32| 0.02 * x + 0.01 * y;
        let from = (650.0, 360.0);
        let to = (710.0, 390.0);
        let view = View::new(&camera, (1280.0, 800.0), 0.2);
        let anchor = view.terrain_point(from.0, from.1, &slope).unwrap();
        drag_pan(&mut camera, &view, from, to, &slope);
        let view = View::new(&camera, (1280.0, 800.0), 0.2);
        let after = view.terrain_point(to.0, to.1, &slope).unwrap();
        assert!((anchor[0] - after[0]).hypot(anchor[2] - after[2]) < 0.002);
    }

    #[test]
    fn globe_drag_keeps_the_surface_point_under_the_cursor() {
        let n = 64;
        let globe = Globe::new(&crate::PlanetScene {
            sea_level: None,
            n,
            kinds: vec![crate::TileKind::Land; 6 * (n * n) as usize],
            heights: vec![0; 6 * (n * n) as usize],
            feature_colors: Default::default(),
            starts: vec![],
            nodes: vec![],
            sun: Default::default(),
        });
        let layout = layout();
        let hud = HudSnapshot::default();
        let from = layout.world_area().centre();
        let to = (from.0 + 70.0, from.1 + 35.0);
        for (lon, lat) in [(35.0, 25.0), (179.0, 80.0), (-90.0, -60.0)] {
            let mut orbit = OrbitCamera::at(lon, lat, 35.0, 20.0, globe.radius(), 120.0);
            let ctx = PickContext {
                layout,
                view: canvas_view_orbit(&orbit, &layout, true),
                entities: &[],
                ground: FLAT,
                globe: Some(&globe),
                hud: &hud,
                hud_visible: true,
                map_size: (n, n),
                now_ms: 0,
            };
            let anchor = ctx
                .view
                .surface_point(from.0, from.1, Ground::Globe(&globe))
                .unwrap()
                .point;
            let mut picker = PickController::default();
            let mut flat_camera = camera();
            for event in [
                InputEvent::ButtonDown {
                    button: PointerButton::Middle,
                    x: from.0,
                    y: from.1,
                    mods: Modifiers::default(),
                },
                InputEvent::PointerMoved { x: to.0, y: to.1 },
            ] {
                assert!(
                    picker
                        .handle_with_orbit(event, &ctx, &mut flat_camera, Some(&mut orbit))
                        .is_empty()
                );
            }
            let after = canvas_view_orbit(&orbit, &layout, true);
            let (x, y, _) = after.project(anchor).unwrap();
            assert!((x - to.0).hypot(y - to.1) < 0.1, "{lon}, {lat}: {x}, {y}");
        }
    }

    #[test]
    fn middle_drag_pans_and_right_drag_requires_an_empty_selection() {
        let mut cam = camera();
        let layout = HudLayout::new(1280, 800, None);
        let mut hud = HudSnapshot::default();
        // Use explicit contexts because they borrow the snapshot.
        for button in [PointerButton::Middle, PointerButton::Right] {
            let mut picker = PickController::default();
            let ctx = PickContext {
                layout,
                view: canvas_view(&cam, &layout, false),
                entities: &[],
                ground: FLAT,
                globe: None,
                hud: &hud,
                hud_visible: false,
                map_size: (200, 200),
                now_ms: 0,
            };
            assert!(
                picker
                    .handle(
                        InputEvent::ButtonDown {
                            button,
                            x: 640.0,
                            y: 400.0,
                            mods: Modifiers::default()
                        },
                        &ctx,
                        &mut cam
                    )
                    .is_empty()
            );
            assert!(picker.is_panning());
            let previous = cam.target;
            picker.handle(
                InputEvent::PointerMoved { x: 690.0, y: 420.0 },
                &ctx,
                &mut cam,
            );
            assert_ne!(cam.target, previous);
            picker.handle(InputEvent::ReleaseAll, &ctx, &mut cam);
            assert!(!picker.is_panning());
        }
        hud.selected = vec![1];
        let ctx = PickContext {
            layout,
            view: canvas_view(&cam, &layout, false),
            entities: &[],
            ground: FLAT,
            globe: None,
            hud: &hud,
            hud_visible: false,
            map_size: (200, 200),
            now_ms: 0,
        };
        let mut picker = PickController::default();
        let events = picker.handle(
            InputEvent::ButtonDown {
                button: PointerButton::Right,
                x: 640.0,
                y: 400.0,
                mods: Modifiers::default(),
            },
            &ctx,
            &mut cam,
        );
        assert!(!picker.is_panning());
        assert!(matches!(events.as_slice(), [PickEvent::RightClick { .. }]));
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
    fn ctrl_and_the_wheel_turn_a_building_being_placed_and_else_zoom() {
        let cam = camera();
        let armed = HudSnapshot {
            turnable: true,
            ..HudSnapshot::default()
        };
        let idle = HudSnapshot::default();
        let wheel = |steps| InputEvent::TurnWheel { steps };
        assert_eq!(turn_or_zoom(wheel(1.0), &armed), wheel(1.0));
        assert_eq!(
            turn_or_zoom(wheel(-2.0), &idle),
            InputEvent::Wheel { steps: -2.0 }
        );
        assert_eq!(
            turn_or_zoom(InputEvent::Wheel { steps: 1.0 }, &armed),
            InputEvent::Wheel { steps: 1.0 },
            "the plain wheel keeps zooming"
        );
        let entities = vec![unit_view(1, 50.0, 60.0)];
        let ctx = context(&entities, &armed, &cam);
        let mut controller = PickController::default();
        let mut camera = cam;
        let up = controller.handle(wheel(1.0), &ctx, &mut camera);
        let down = controller.handle(wheel(-1.0), &ctx, &mut camera);
        assert_eq!(up, vec![PickEvent::Turn { steps: 1 }]);
        assert_eq!(down, vec![PickEvent::Turn { steps: -1 }]);
        assert_eq!(camera.target, cam.target, "turning moves no camera");
    }

    #[test]
    fn a_left_drag_with_a_brush_paints_and_the_camera_keeps_its_buttons() {
        let cam = camera();
        let hud = HudSnapshot {
            painting: true,
            ..HudSnapshot::default()
        };
        let entities = vec![unit_view(1, 50.0, 60.0)];
        let ctx = context(&entities, &hud, &cam);
        let mut controller = PickController::default();
        let mut camera = cam;
        let (x0, y0) = (800.0, 450.0);
        let button = |button, x, y, down| {
            let mods = Modifiers::default();
            if down {
                InputEvent::ButtonDown { button, x, y, mods }
            } else {
                InputEvent::ButtonUp { button, x, y, mods }
            }
        };
        let starts = |events: &[PickEvent]| {
            events
                .iter()
                .filter_map(|e| match e {
                    PickEvent::Stroke { start, .. } => Some(Some(*start)),
                    PickEvent::StrokeEnd => Some(None),
                    PickEvent::Hover { .. } => None,
                    other => panic!("unexpected {other:?}"),
                })
                .collect::<Vec<_>>()
        };
        let mut events =
            controller.handle(button(PointerButton::Left, x0, y0, true), &ctx, &mut camera);
        for step in 1..=3 {
            events.extend(controller.handle(
                InputEvent::PointerMoved {
                    x: x0 + 40.0 * step as f32,
                    y: y0,
                },
                &ctx,
                &mut camera,
            ));
            assert!(controller.drag_rect().is_none(), "no selection box");
        }
        events.extend(controller.handle(
            button(PointerButton::Left, x0 + 120.0, y0, false),
            &ctx,
            &mut camera,
        ));
        let kinds = starts(&events);
        assert_eq!(kinds.first(), Some(&Some(true)), "{events:?}");
        assert_eq!(kinds.last(), Some(&None), "{events:?}");
        assert!(kinds[1..kinds.len() - 1].iter().all(|k| *k == Some(false)));
        assert_eq!(kinds.iter().filter(|k| **k == Some(false)).count(), 3);
        assert_eq!(camera.target, cam.target, "painting moves no camera");
        // The middle-button drag still pans, with the brush in hand.
        let mut controller = PickController::default();
        assert!(
            controller
                .handle(
                    button(PointerButton::Middle, x0, y0, true),
                    &ctx,
                    &mut camera
                )
                .is_empty()
        );
        assert!(controller.is_panning());
        let out = controller.handle(
            InputEvent::PointerMoved {
                x: x0 + 80.0,
                y: y0 + 30.0,
            },
            &ctx,
            &mut camera,
        );
        assert!(
            out.iter().all(|e| matches!(e, PickEvent::Hover { .. })),
            "a pan paints nothing: {out:?}"
        );
        assert_ne!(camera.target, cam.target, "the middle drag pans");
        controller.handle(
            button(PointerButton::Middle, x0 + 80.0, y0 + 30.0, false),
            &ctx,
            &mut camera,
        );
        assert!(!controller.is_panning());
        // Without a brush the same left drag still draws the selection box.
        let plain = HudSnapshot::default();
        let ctx = context(&entities, &plain, &cam);
        let mut controller = PickController::default();
        controller.handle(button(PointerButton::Left, x0, y0, true), &ctx, &mut camera);
        controller.handle(
            InputEvent::PointerMoved {
                x: x0 + 40.0,
                y: y0,
            },
            &ctx,
            &mut camera,
        );
        assert!(controller.drag_rect().is_some());
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

    /// The projected pick box of `entity` as a hull, and its centre on screen.
    fn screen_hull(
        ctx: &PickContext,
        entity: &EntityView,
    ) -> ([(f32, f32); 16], usize, (f32, f32)) {
        let (centre, corners) = ctx.box_of(entity);
        let (cx, cy, _) = ctx.view.project(centre).expect("visible");
        let mut projected = [(0.0, 0.0); 8];
        for (slot, c) in projected.iter_mut().zip(corners) {
            let (x, y, _) = ctx.view.project(c).expect("visible");
            *slot = (x, y);
        }
        let mut hull = [(0.0, 0.0); 16];
        let n = convex_hull(&mut projected, &mut hull);
        (hull, n, (cx, cy))
    }

    /// The pixel where the projected pick box of `entity` ends, going right from its centre.
    fn right_edge(ctx: &PickContext, entity: &EntityView) -> (f32, f32) {
        let (hull, n, (mut x, y)) = screen_hull(ctx, entity);
        while hull_distance(&hull[..n], (x, y)) <= 0.0 {
            x += 0.25;
        }
        (x, y)
    }

    #[test]
    fn a_pointer_just_beside_a_unit_picks_it_within_the_slack() {
        let hud = HudSnapshot::default();
        let entities = vec![unit_view(4, 50.0, 60.0)];
        let ctx = context(&entities, &hud, &camera());
        let slack = crate::PICK_SLACK * ctx.layout.scale;
        let (edge, y) = right_edge(&ctx, &entities[0]);
        assert_eq!(ctx.entity_at(edge - 1.0, y), Some(4), "on the box");
        assert_eq!(ctx.entity_at(edge + slack - 0.5, y), Some(4));
        assert_eq!(ctx.entity_at(edge + slack + 0.5, y), None);
        // The slack follows the HUD scale: half the scale, half the pixels.
        let mut small = context(&entities, &hud, &camera());
        small.layout.scale = ctx.layout.scale / 2.0;
        assert_eq!(small.entity_at(edge + slack / 2.0 - 0.5, y), Some(4));
        assert_eq!(small.entity_at(edge + slack / 2.0 + 0.5, y), None);
    }

    #[test]
    fn the_box_under_the_pointer_beats_a_nearer_one_beside_it_and_ties_go_by_id() {
        let hud = HudSnapshot::default();
        // Unit 2 east of unit 1, as far as leaves a gap narrower than two slacks between their
        // boxes on screen: points there are beside both.
        let mut checked = false;
        for step in 0..60 {
            let entities = vec![
                unit_view(1, 50.0, 60.0),
                unit_view(2, 50.8 + step as f32 * 0.02, 60.0),
            ];
            let ctx = context(&entities, &hud, &camera());
            let slack = crate::PICK_SLACK * ctx.layout.scale;
            let (one, n1, a) = screen_hull(&ctx, &entities[0]);
            let (two, n2, b) = screen_hull(&ctx, &entities[1]);
            let samples: Vec<_> = (0..=800)
                .map(|i| {
                    let t = i as f32 / 800.0;
                    let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                    (
                        p,
                        hull_distance(&one[..n1], p),
                        hull_distance(&two[..n2], p),
                    )
                })
                .collect();
            let on_one = samples
                .iter()
                .find(|(_, d1, d2)| *d1 <= 0.0 && *d2 > 0.0 && *d2 <= slack);
            let between = samples
                .iter()
                .find(|(_, d1, d2)| *d1 > 0.0 && *d2 > 0.0 && *d2 < *d1 && *d1 <= slack);
            let (Some((p, ..)), Some((q, ..))) = (on_one, between) else {
                continue;
            };
            // On unit 1's box and beside unit 2's: the box under the pointer wins.
            assert_eq!(ctx.entity_at(p.0, p.1), Some(1));
            // On neither box, beside both: the nearer box on screen.
            assert_eq!(ctx.entity_at(q.0, q.1), Some(2));
            checked = true;
            break;
        }
        assert!(checked, "no spacing put a point beside both boxes");
        // Two units drawn in one place, the pointer beside both: the lower id.
        let twins = vec![unit_view(3, 50.0, 60.0), unit_view(5, 50.0, 60.0)];
        let ctx = context(&twins, &hud, &camera());
        let (edge, y) = right_edge(&ctx, &twins[0]);
        assert_eq!(ctx.entity_at(edge + 1.0, y), Some(3));
    }

    #[test]
    fn the_pointed_entity_is_none_over_the_hud_and_while_dragging() {
        let hud = HudSnapshot::default();
        let entities = vec![unit_view(4, 50.0, 60.0)];
        let ctx = context(&entities, &hud, &camera());
        let (x, y, _) = ctx
            .view
            .project(pick_box(&entities[0], FLAT).0)
            .expect("visible");
        let mut picker = PickController::default();
        let mut cam = camera();
        assert_eq!(picker.pointed(&ctx), None, "no pointer yet");
        picker.handle(InputEvent::PointerMoved { x, y }, &ctx, &mut cam);
        assert_eq!(picker.pointed(&ctx), Some(4));
        // Over the HUD's bottom panels nothing in the world is pointed at.
        picker.handle(
            InputEvent::PointerMoved { x: 100.0, y: 900.0 },
            &ctx,
            &mut cam,
        );
        assert_eq!(picker.pointed(&ctx), None);
        // A selection box being dragged points at nothing.
        let mods = Modifiers::default();
        picker.handle(
            InputEvent::ButtonDown {
                button: PointerButton::Left,
                x: x - 40.0,
                y: y - 40.0,
                mods,
            },
            &ctx,
            &mut cam,
        );
        picker.handle(InputEvent::PointerMoved { x, y }, &ctx, &mut cam);
        assert_eq!(picker.pointed(&ctx), None);
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
    #[test]
    fn orbital_ships_are_picked_above_the_surface_and_occluded_on_the_far_side() {
        let n = 64;
        let globe = Globe::new(&crate::PlanetScene {
            sea_level: None,
            n,
            kinds: vec![crate::TileKind::Land; 6 * (n * n) as usize],
            heights: vec![0; 6 * (n * n) as usize],
            feature_colors: Default::default(),
            starts: vec![],
            nodes: vec![],
            sun: Default::default(),
        });
        let mut surface = unit_view(1, 32.0, 32.0);
        let mut orbit = surface;
        orbit.id = 2;
        orbit.altitude = 8.0;
        surface.face = 2;
        let mut far_orbit = orbit;
        far_orbit.id = 3;
        far_orbit.face = 2;
        let views = [surface, orbit, far_orbit];
        let hud = HudSnapshot::default();
        let ctx = PickContext {
            layout: layout(),
            view: View::looking(
                [0.0, 0.0, globe.radius() + 50.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                (1280.0, 800.0),
                0.0,
            ),
            entities: &views,
            ground: FLAT,
            globe: Some(&globe),
            hud: &hud,
            hud_visible: false,
            map_size: (n, n),
            now_ms: 0,
        };
        let centre = pick_frame(&orbit, &globe).0;
        let (x, y, _) = ctx.view.project(centre).unwrap();
        assert_eq!(ctx.entity_at(x, y), Some(2));
        assert!(!ctx.hidden_by_planet(centre));
        assert!(ctx.hidden_by_planet(pick_frame(&far_orbit, &globe).0));
        assert!(
            ctx.entities_in_rect(Rect::new(0.0, 0.0, 1280.0, 800.0))
                .contains(&2)
        );
        assert!(
            !ctx.entities_in_rect(Rect::new(0.0, 0.0, 1280.0, 800.0))
                .contains(&3)
        );
    }
}

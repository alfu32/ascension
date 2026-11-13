//! Ships, claims and transits in space: the plain data the simulation publishes, where each of
//! them stands in the system frame and the star-map frame, and picking among them. Nothing here
//! touches raylib.
//!
//! A ship stands in system space. When it holds a body (it is within orbit reach of one) its
//! position is the offset from that body in units of the orbit reach, so ships cluster around the
//! body however the frame is scaled; otherwise it is the offset from the star in orbit-radius
//! units, the units of [`crate::universe::BodyView::orbit_radius`]. The system frame maps both to
//! the same layout the bodies use.

use rom_geo::SurfacePos;

use crate::camera::{InputEvent, Modifiers, PointerButton};
use crate::entities::owner_color;
use crate::hud_layout::Rect;
use crate::overlay::Tri3;
use crate::pick::{DOUBLE_CLICK_MS, DRAG_PIXELS, PickEvent, View};
use crate::universe::{PlacedBody, UniverseScene, frame_to_orbit, orbit_to_frame};
use crate::zoom::Target;

/// Radius of the ring ships hold around a body, in radii of that body, at one orbit reach.
pub const CLUSTER_RADII: f32 = 1.9;
/// Smallest pick radius of a ship on screen, in pixels.
const SHIP_PICK_PIXELS: f32 = 13.0;

/// One ship in system space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShipView {
    /// Entity id.
    pub id: u32,
    /// Entity type id.
    pub kind: u16,
    /// Owning player.
    pub owner: u8,
    /// The body it holds, if any: then `x`, `y` are in orbit reaches from that body.
    pub body: Option<u32>,
    /// East offset, see the module documentation.
    pub x: f32,
    /// South offset, see the module documentation.
    pub y: f32,
    /// Heading in degrees clockwise from east.
    pub facing_deg: f32,
    /// Health 0..=1.
    pub hp_frac: f32,
}

/// A claimed resource body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimView {
    /// The body.
    pub body: u32,
    /// The claimant.
    pub owner: u8,
}

/// A ship between two systems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitView {
    /// Entity id.
    pub id: u32,
    /// Entity type id.
    pub kind: u16,
    /// Owning player.
    pub owner: u8,
    /// The system it left.
    pub from: u32,
    /// The system it is bound for.
    pub to: u32,
    /// Fraction of the leg flown, 0..=1.
    pub progress: f32,
}

/// What the render thread draws in space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpaceSnapshot {
    /// Tick of the simulation the data stems from.
    pub tick: u64,
    /// The system whose space the ships and claims belong to.
    pub system: Option<u32>,
    /// Ships in that system's space, ascending by id.
    pub ships: Vec<ShipView>,
    /// Claimed bodies of that system.
    pub claims: Vec<ClaimView>,
    /// Ships in transit anywhere on the star map.
    pub transits: Vec<TransitView>,
}

/// Where a ship stands in the system frame. `placed` are the bodies of the system as laid out by
/// [`UniverseScene::place_system`] and `outermost` its [`UniverseScene::system_outermost`].
pub fn ship_frame_pos(
    scene: &UniverseScene,
    placed: &[PlacedBody],
    outermost: f32,
    ship: &ShipView,
) -> [f32; 3] {
    if let Some(body) = ship.body.and_then(|id| scene.body_index(id))
        && let Some(at) = placed.iter().find(|p| p.body == body)
    {
        let step = at.radius * CLUSTER_RADII;
        return [
            at.pos[0] + ship.x * step,
            at.pos[1],
            at.pos[2] + ship.y * step,
        ];
    }
    let radius = ship.x.hypot(ship.y);
    if radius <= f32::EPSILON {
        return [0.0; 3];
    }
    let frame = orbit_to_frame(radius, outermost);
    [ship.x / radius * frame, 0.0, ship.y / radius * frame]
}

/// The orbit-radius-unit offset from the star of a point of the system plane.
pub fn frame_to_orbit_point(frame: [f32; 3], outermost: f32) -> (f32, f32) {
    let radius = frame[0].hypot(frame[2]);
    if radius <= f32::EPSILON {
        return (0.0, 0.0);
    }
    let orbit = frame_to_orbit(radius, outermost);
    (frame[0] / radius * orbit, frame[2] / radius * orbit)
}

/// The place of a transit on the star map: `progress` of the way from `from` to `to`.
pub fn transit_point(from: [f32; 3], to: [f32; 3], progress: f32) -> [f32; 3] {
    let t = progress.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * t)
}

/// A ring around a claimed body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClaimRing {
    /// The body's id.
    pub body: u32,
    /// Centre in the system frame.
    pub centre: [f32; 3],
    /// Radius of the ring's middle in system units.
    pub radius: f32,
    /// Colour of the claimant.
    pub colour: [u8; 3],
}

/// The rings of the claimed bodies of the system laid out as `placed`, each in its claimant's
/// colour; claims of bodies outside the system are left out.
pub fn claim_rings(
    scene: &UniverseScene,
    placed: &[PlacedBody],
    claims: &[ClaimView],
) -> Vec<ClaimRing> {
    claims
        .iter()
        .filter_map(|claim| {
            let index = scene.body_index(claim.body)?;
            let at = placed.iter().find(|p| p.body == index)?;
            let [r, g, b] = owner_color(claim.owner);
            Some(ClaimRing {
                body: claim.body,
                centre: at.pos,
                radius: at.radius * CLUSTER_RADII * 1.2,
                colour: [r, g, b].map(|c| (c * 255.0).round() as u8),
            })
        })
        .collect()
}

/// A flat annulus in the plane `y = centre.y` as triangles of `segments` steps.
pub fn annulus(
    centre: [f32; 3],
    radius: f32,
    thickness: f32,
    colour: [u8; 4],
    segments: u32,
) -> Vec<Tri3> {
    let (inner, outer) = (radius - thickness / 2.0, radius + thickness / 2.0);
    let point = |i: u32, r: f32| {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        [centre[0] + r * a.cos(), centre[1], centre[2] + r * a.sin()]
    };
    (0..segments)
        .flat_map(|i| {
            let (a, b, c, d) = (
                point(i, inner),
                point(i, outer),
                point(i + 1, outer),
                point(i + 1, inner),
            );
            [
                Tri3 {
                    v: [a, b, c],
                    colour,
                },
                Tri3 {
                    v: [a, c, d],
                    colour,
                },
            ]
        })
        .collect()
}

/// Where the ray through pixel (`px`, `py`) meets the system plane `y = 0`.
pub fn plane_point(view: &View, px: f32, py: f32) -> Option<[f32; 3]> {
    let dir = view.ray(px, py);
    let eye = view.eye();
    if dir[1] >= -1e-5 {
        return None;
    }
    let t = -eye[1] / dir[1];
    (t > 0.0).then(|| [eye[0] + dir[0] * t, 0.0, eye[2] + dir[2] * t])
}

/// The ship nearest to `cursor` among `ships` (id and frame position) whose projection lies
/// within `reach_px` (at least [`SHIP_PICK_PIXELS`]) of it.
pub fn nearest_ship(
    view: &View,
    ships: &[(u32, [f32; 3])],
    cursor: (f32, f32),
    reach_px: f32,
) -> Option<u32> {
    let reach = reach_px.max(SHIP_PICK_PIXELS);
    ships
        .iter()
        .filter_map(|&(id, at)| {
            let (x, y, _) = view.project(at)?;
            let d = (x - cursor.0).hypot(y - cursor.1);
            (d <= reach).then_some((d, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// The ships whose projected centres lie inside `rect`, in id order.
pub fn ships_in_rect(view: &View, ships: &[(u32, [f32; 3])], rect: Rect) -> Vec<u32> {
    let mut ids: Vec<u32> = ships
        .iter()
        .filter(|&&(_, at)| {
            view.project(at)
                .is_some_and(|(x, y, _)| rect.contains(x, y))
        })
        .map(|&(id, _)| id)
        .collect();
    ids.sort_unstable();
    ids
}

/// What a space pick needs to know about the frame.
pub struct SpaceContext<'a> {
    /// View of the canvas through the active space camera.
    pub view: View,
    /// True at the star-map level, false at the system level.
    pub star_map: bool,
    /// Ships of the system with their frame positions (system level).
    pub ships: &'a [(u32, [f32; 3])],
    /// What a click can hit: bodies at the system level, systems on the star map, with screen
    /// markers as in [`crate::zoom::ZoomCamera::markers`].
    pub markers: &'a [(Target, (f32, f32, f32))],
    /// The universe, to turn markers into ids.
    pub scene: &'a UniverseScene,
    /// The system's outermost orbit, for ground points.
    pub outermost: f32,
}

impl SpaceContext<'_> {
    fn target_at(&self, cursor: (f32, f32)) -> Option<Target> {
        let rects: Vec<(f32, f32, f32)> = self.markers.iter().map(|m| m.1).collect();
        crate::zoom::nearest_marker(&rects, cursor).map(|i| self.markers[i].0)
    }

    fn ship_at(&self, cursor: (f32, f32)) -> Option<u32> {
        if self.star_map {
            return None;
        }
        nearest_ship(&self.view, self.ships, cursor, 0.0)
    }

    fn ground(&self, cursor: (f32, f32)) -> (f32, f32) {
        plane_point(&self.view, cursor.0, cursor.1)
            .map_or((0.0, 0.0), |p| frame_to_orbit_point(p, self.outermost))
    }
}

fn surface_of(point: (f32, f32)) -> SurfacePos {
    SurfacePos {
        face: 0,
        x: (point.0 * 1000.0).round() as i64,
        y: (point.1 * 1000.0).round() as i64,
    }
}

#[derive(Clone, Copy, Debug)]
struct Press {
    x: f32,
    y: f32,
    mods: Modifiers,
    dragging: bool,
}

/// Turns pointer input over system space into pick events: clicks select ships, drags box-select
/// them, and a right click names a ship, a body, a system or a point to order against.
#[derive(Clone, Debug, Default)]
pub struct SpacePicker {
    pointer: Option<(f32, f32)>,
    press: Option<Press>,
    last_click: Option<(u64, f32, f32)>,
}

impl SpacePicker {
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

    /// Drops a press in progress (a double click started a flight).
    pub fn cancel(&mut self) {
        self.press = None;
    }

    /// Applies one input event over the space view (not over the HUD); the returned events go to
    /// the simulation thread.
    pub fn handle(&mut self, event: InputEvent, ctx: &SpaceContext, now_ms: u64) -> Vec<PickEvent> {
        let mut out = Vec::new();
        match event {
            InputEvent::PointerMoved { x, y } => {
                self.pointer = Some((x, y));
                if let Some(press) = &mut self.press
                    && (x - press.x).hypot(y - press.y) > DRAG_PIXELS
                {
                    press.dragging = true;
                }
            }
            InputEvent::PointerLeft => self.pointer = None,
            InputEvent::ButtonDown {
                button: PointerButton::Left,
                x,
                y,
                mods,
            } => {
                self.pointer = Some((x, y));
                if !ctx.star_map {
                    self.press = Some(Press {
                        x,
                        y,
                        mods,
                        dragging: false,
                    });
                }
            }
            InputEvent::ButtonDown {
                button: PointerButton::Right,
                x,
                y,
                mods,
            } => {
                self.pointer = Some((x, y));
                out.push(self.right_click(ctx, (x, y), mods));
            }
            InputEvent::ButtonUp {
                button: PointerButton::Left,
                x,
                y,
                mods,
            } => {
                self.pointer = Some((x, y));
                if let Some(press) = self.press.take() {
                    out.push(self.finish(press, ctx, (x, y), mods, now_ms));
                }
            }
            InputEvent::ReleaseAll => self.press = None,
            _ => {}
        }
        out
    }

    fn right_click(&self, ctx: &SpaceContext, cursor: (f32, f32), mods: Modifiers) -> PickEvent {
        let entity = ctx.ship_at(cursor);
        let (mut body, mut system) = (None, None);
        match ctx.target_at(cursor) {
            Some(Target::Body(i)) if entity.is_none() => {
                body = ctx
                    .scene
                    .bodies
                    .get(i)
                    .filter(|b| b.kind != crate::universe::BodyViewKind::Star)
                    .map(|b| b.id);
            }
            Some(Target::System(i)) => system = ctx.scene.systems.get(i).map(|s| s.id),
            _ => {}
        }
        PickEvent::SpaceRightClick {
            surface: surface_of(ctx.ground(cursor)),
            entity,
            body,
            system,
            modifiers: mods,
        }
    }

    fn finish(
        &mut self,
        press: Press,
        ctx: &SpaceContext,
        (x, y): (f32, f32),
        mods: Modifiers,
        now_ms: u64,
    ) -> PickEvent {
        let mods = Modifiers {
            shift: mods.shift || press.mods.shift,
            ctrl: mods.ctrl || press.mods.ctrl,
            alt: mods.alt || press.mods.alt,
        };
        if press.dragging {
            self.last_click = None;
            let rect = Rect::new(
                press.x.min(x),
                press.y.min(y),
                (x - press.x).abs(),
                (y - press.y).abs(),
            );
            return PickEvent::BoxSelect {
                entities: ships_in_rect(&ctx.view, ctx.ships, rect),
                modifiers: mods,
            };
        }
        let double = self.last_click.is_some_and(|(at, lx, ly)| {
            now_ms.saturating_sub(at) <= DOUBLE_CLICK_MS && (lx - x).hypot(ly - y) <= DRAG_PIXELS
        });
        self.last_click = if double { None } else { Some((now_ms, x, y)) };
        let point = ctx.ground((x, y));
        let on_screen = if double {
            let (w, h) = ctx.view.size();
            ships_in_rect(&ctx.view, ctx.ships, Rect::new(0.0, 0.0, w, h))
        } else {
            Vec::new()
        };
        PickEvent::Click {
            world_point: point,
            surface: surface_of(point),
            entity: ctx.ship_at((x, y)),
            modifiers: mods,
            double,
            on_screen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universe::{BodyViewKind, SYSTEM_RADIUS, sample_scene};

    fn ship(id: u32, body: Option<u32>, x: f32, y: f32) -> ShipView {
        ShipView {
            id,
            kind: 1,
            owner: 1,
            body,
            x,
            y,
            facing_deg: 0.0,
            hp_frac: 1.0,
        }
    }

    #[test]
    fn claimed_body_ring_takes_the_claimants_colour() {
        let scene = sample_scene();
        let placed = scene.place_system(1);
        let claims = [
            ClaimView { body: 3, owner: 1 },
            ClaimView { body: 4, owner: 2 },
            // A body of another system has no ring here.
            ClaimView { body: 6, owner: 3 },
        ];
        let rings = claim_rings(&scene, &placed, &claims);
        assert_eq!(rings.len(), 2);
        let colour_of = |owner: u8| owner_color(owner).map(|c| (c * 255.0).round() as u8);
        assert_eq!(rings[0].colour, colour_of(1));
        assert_eq!(rings[1].colour, colour_of(2));
        assert_ne!(rings[0].colour, rings[1].colour);
        // The ring is centred on its body and wider than the body.
        for ring in &rings {
            let index = scene.body_index(ring.body).unwrap();
            let at = placed.iter().find(|p| p.body == index).unwrap();
            assert_eq!(ring.centre, at.pos);
            assert!(ring.radius > at.radius);
        }
        // The annulus triangles carry the colour.
        let tris = annulus(rings[0].centre, rings[0].radius, 10.0, [1, 2, 3, 255], 24);
        assert_eq!(tris.len(), 48);
        assert!(tris.iter().all(|t| t.colour == [1, 2, 3, 255]));
        assert!(claim_rings(&scene, &placed, &[]).is_empty());
    }

    #[test]
    fn transit_point_moves_along_its_route() {
        let from = [-1000.0, 0.0, 200.0];
        let to = [3000.0, 0.0, -600.0];
        assert_eq!(transit_point(from, to, 0.0), from);
        assert_eq!(transit_point(from, to, 1.0), to);
        assert_eq!(transit_point(from, to, 0.5), [1000.0, 0.0, -200.0]);
        let mut last = 0.0_f32;
        for step in 0..=10 {
            let p = transit_point(from, to, step as f32 / 10.0);
            let travelled = (p[0] - from[0]).hypot(p[2] - from[2]);
            assert!(travelled >= last, "the point only advances");
            last = travelled;
            // It stays on the straight line between the systems.
            let cross = (p[0] - from[0]) * (to[2] - from[2]) - (p[2] - from[2]) * (to[0] - from[0]);
            assert!(cross.abs() < 1.0);
        }
        assert_eq!(transit_point(from, to, 2.0), to);
        assert_eq!(transit_point(from, to, -1.0), from);
    }

    #[test]
    fn ships_cluster_around_their_body_and_free_ones_follow_the_layout() {
        let scene = sample_scene();
        let placed = scene.place_system(1);
        let outermost = scene.system_outermost(1);
        let body = placed
            .iter()
            .find(|p| scene.bodies[p.body].id == 3)
            .unwrap();
        let held = ship_frame_pos(&scene, &placed, outermost, &ship(10, Some(3), 1.0, 0.0));
        let distance = (held[0] - body.pos[0]).hypot(held[2] - body.pos[2]);
        assert!((distance - body.radius * CLUSTER_RADII).abs() < 1e-2);
        // A free ship at the orbit radius of body 3 is on that body's orbit ring.
        let orbit = f32::from(2000_u16);
        let free = ship_frame_pos(&scene, &placed, outermost, &ship(11, None, orbit, 0.0));
        assert!((free[0].hypot(free[2]) - body.orbit).abs() < 1e-2);
        // The inverse maps a frame point back to orbit units.
        let (x, y) = frame_to_orbit_point(free, outermost);
        assert!((x - orbit).abs() < 1e-2 && y.abs() < 1e-2);
        assert!(placed.iter().all(|p| p.orbit <= SYSTEM_RADIUS + 0.5));
        assert_eq!(
            ship_frame_pos(&scene, &placed, outermost, &ship(12, None, 0.0, 0.0)),
            [0.0; 3]
        );
        assert_eq!(scene.bodies[body.body].kind, BodyViewKind::Resource);
    }

    #[test]
    fn picker_selects_ships_and_names_orders() {
        let scene = sample_scene();
        let placed = scene.place_system(1);
        let outermost = scene.system_outermost(1);
        let view = View::looking(
            [0.0, 7000.0, 6000.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            (1280.0, 720.0),
            0.0,
        );
        let ships = [
            (
                10,
                ship_frame_pos(&scene, &placed, outermost, &ship(10, None, 1200.0, 300.0)),
            ),
            (
                11,
                ship_frame_pos(&scene, &placed, outermost, &ship(11, None, -1500.0, -900.0)),
            ),
        ];
        let at = |id: u32| {
            let p = ships.iter().find(|s| s.0 == id).unwrap().1;
            let (x, y, _) = view.project(p).unwrap();
            (x, y)
        };
        let markers: Vec<_> = placed
            .iter()
            .filter_map(|p| {
                let (x, y, depth) = view.project(p.pos)?;
                Some((
                    Target::Body(p.body),
                    (x, y, p.radius / depth * 720.0 / (2.0 * view.tan())),
                ))
            })
            .collect();
        let ctx = SpaceContext {
            view,
            star_map: false,
            ships: &ships,
            markers: &markers,
            scene: &scene,
            outermost,
        };
        let mut picker = SpacePicker::default();
        let (x, y) = at(10);
        let down = InputEvent::ButtonDown {
            button: PointerButton::Left,
            x,
            y,
            mods: Modifiers::default(),
        };
        let up = InputEvent::ButtonUp {
            button: PointerButton::Left,
            x,
            y,
            mods: Modifiers::default(),
        };
        assert!(picker.handle(down, &ctx, 0).is_empty());
        let events = picker.handle(up, &ctx, 10);
        assert!(matches!(
            events.as_slice(),
            [PickEvent::Click {
                entity: Some(10),
                double: false,
                ..
            }]
        ));
        // A drag over both ships box-selects them.
        let (x2, y2) = at(11);
        let drag_down = InputEvent::ButtonDown {
            button: PointerButton::Left,
            x: x.min(x2) - 40.0,
            y: y.min(y2) - 40.0,
            mods: Modifiers::default(),
        };
        picker.handle(drag_down, &ctx, 100);
        picker.handle(
            InputEvent::PointerMoved {
                x: x.max(x2) + 40.0,
                y: y.max(y2) + 40.0,
            },
            &ctx,
            110,
        );
        assert!(picker.drag_rect().is_some());
        let events = picker.handle(
            InputEvent::ButtonUp {
                button: PointerButton::Left,
                x: x.max(x2) + 40.0,
                y: y.max(y2) + 40.0,
                mods: Modifiers::default(),
            },
            &ctx,
            120,
        );
        assert!(matches!(
            events.as_slice(),
            [PickEvent::BoxSelect { entities, .. }] if entities == &vec![10, 11]
        ));
        // A right click on a rock names its body; on empty space a point.
        let rock = placed
            .iter()
            .find(|p| scene.bodies[p.body].id == 3)
            .unwrap();
        let (rx, ry, _) = ctx.view.project(rock.pos).unwrap();
        let right = |x: f32, y: f32| InputEvent::ButtonDown {
            button: PointerButton::Right,
            x,
            y,
            mods: Modifiers::default(),
        };
        let events = picker.handle(right(rx, ry), &ctx, 200);
        assert!(matches!(
            events.as_slice(),
            [PickEvent::SpaceRightClick {
                body: Some(3),
                system: None,
                entity: None,
                ..
            }]
        ));
        let events = picker.handle(right(20.0, 700.0), &ctx, 210);
        assert!(matches!(
            events.as_slice(),
            [PickEvent::SpaceRightClick {
                body: None,
                system: None,
                entity: None,
                ..
            }]
        ));
        // On a ship, it names the ship.
        let events = picker.handle(right(x, y), &ctx, 220);
        assert!(matches!(
            events.as_slice(),
            [PickEvent::SpaceRightClick {
                entity: Some(10),
                ..
            }]
        ));
    }

    #[test]
    fn star_map_right_click_names_the_system() {
        let scene = sample_scene();
        let view = View::looking(
            [0.0, 60_000.0, 40_000.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            (1280.0, 720.0),
            0.0,
        );
        let positions = scene.place_systems();
        let markers: Vec<_> = positions
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let (x, y, _) = view.project(*p)?;
                Some((Target::System(i), (x, y, 14.0)))
            })
            .collect();
        let ctx = SpaceContext {
            view,
            star_map: true,
            ships: &[],
            markers: &markers,
            scene: &scene,
            outermost: 1.0,
        };
        let (x, y, _) = view.project(positions[1]).unwrap();
        let mut picker = SpacePicker::default();
        let events = picker.handle(
            InputEvent::ButtonDown {
                button: PointerButton::Right,
                x,
                y,
                mods: Modifiers::default(),
            },
            &ctx,
            0,
        );
        assert!(matches!(
            events.as_slice(),
            [PickEvent::SpaceRightClick {
                system: Some(2),
                ..
            }]
        ));
    }
}

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
/// Smallest body radius the cluster ring is reckoned from, in system units: small moons and
/// asteroids still hold their ships clear of the body.
pub const CLUSTER_MIN_RADIUS: f32 = 100.0;
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
    /// The system whose plane it is in (an id).
    pub system: u32,
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
    /// The composed model it is drawn with (its owner's faction and age); `None` keeps the
    /// plain hull.
    pub model: Option<crate::entities::ModelRef>,
}

/// A claimed resource body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimView {
    /// The body.
    pub body: u32,
    /// The claimant.
    pub owner: u8,
}

/// A habitable planet where a player has a city.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColonyView {
    /// Its system.
    pub system: u32,
    /// The planet.
    pub body: u32,
    /// The player with a city there.
    pub owner: u8,
}

/// Who holds a resource body, as player one hears of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimState {
    /// Nobody keeps station there.
    Unclaimed,
    /// Player one, or an ally of player one, holds it and is paid.
    Yours,
    /// Another player holds it.
    Other(u8),
    /// Rivals keep station there together; nobody is paid.
    Contested,
}

/// What a resource body holds and what it pays, as the simulation reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBodyView {
    /// The body.
    pub body: u32,
    /// Display name of the kind of body ("Tar Moon").
    pub kind_name: String,
    /// Display name of the resource it yields.
    pub resource_name: String,
    /// HUD atlas icon of the resource (`icons/res_...`).
    pub icon: String,
    /// The resource's colour, which tints the body and its marker.
    pub colour: [u8; 3],
    /// What holding it pays player one per income period, with player one's modifiers.
    pub per_period: i64,
    /// Length of the income period in game seconds.
    pub period_secs: i64,
    /// Who holds it.
    pub claim: ClaimState,
}

impl ResourceBodyView {
    /// "Oil +42 / 30 s": the resource and its yield.
    pub fn yield_text(&self) -> String {
        format!(
            "{} +{} / {} s",
            self.resource_name, self.per_period, self.period_secs
        )
    }
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
    /// The system whose claims are listed.
    pub system: Option<u32>,
    /// Ships the player sees in the space of every system, ascending by id.
    pub ships: Vec<ShipView>,
    /// Claimed bodies of that system.
    pub claims: Vec<ClaimView>,
    /// The resource bodies of that system with what they hold, ascending by body id.
    pub resources: Vec<ResourceBodyView>,
    /// Ships in transit anywhere on the star map.
    pub transits: Vec<TransitView>,
    /// Planets with cities, anywhere, that the player knows of; by system, body and owner.
    pub colonies: Vec<ColonyView>,
    /// The systems the player knows, ascending ids; `None` before the first report (all shown).
    pub known: Option<Vec<u32>>,
}

impl SpaceSnapshot {
    /// What the resource body `id` holds, if it is listed.
    pub fn resource_body(&self, id: u32) -> Option<&ResourceBodyView> {
        self.resources.iter().find(|r| r.body == id)
    }

    /// Whether the player knows system `id`: its bodies are drawn and named.
    pub fn knows(&self, id: u32) -> bool {
        self.known
            .as_ref()
            .is_none_or(|known| known.binary_search(&id).is_ok())
    }
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
        let step = at.radius.max(CLUSTER_MIN_RADIUS) * CLUSTER_RADII;
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

/// A ship, or the ships of one owner in one system, marked on the star map.
#[derive(Clone, Debug, PartialEq)]
pub struct MapShipMark {
    /// The ships it stands for, ascending.
    pub ids: Vec<u32>,
    /// Owning player.
    pub owner: u8,
    /// Place on the star map.
    pub at: [f32; 3],
    /// Unit direction of travel along the lane in 3D, for a ship in transit.
    pub heading: Option<[f32; 3]>,
    /// The system it is bound for (an index of the scene's systems), for a ship in transit.
    pub bound_for: Option<usize>,
    /// The system it is in (an index), for ships in a system's space.
    pub system: Option<usize>,
}

/// The ship marks of the star map: every ship in transit at its place on its lane, heading for
/// its destination, and the ships in each system's space grouped by owner beside the system,
/// `beside` map units apart (a step toward the screen's right).
pub fn map_ship_marks(
    scene: &UniverseScene,
    positions: &[[f32; 3]],
    snapshot: &SpaceSnapshot,
    beside: [f32; 3],
) -> Vec<MapShipMark> {
    let mut marks = Vec::new();
    for transit in &snapshot.transits {
        let (Some(a), Some(b)) = (
            scene.system_index(transit.from),
            scene.system_index(transit.to),
        ) else {
            continue;
        };
        let (from, to) = (positions[a], positions[b]);
        let way = [0, 1, 2].map(|i| to[i] - from[i]);
        let length = (way[0] * way[0] + way[1] * way[1] + way[2] * way[2]).sqrt();
        marks.push(MapShipMark {
            ids: vec![transit.id],
            owner: transit.owner,
            at: transit_point(from, to, transit.progress),
            heading: (length > 1e-3).then(|| way.map(|c| c / length)),
            bound_for: Some(b),
            system: None,
        });
    }
    let mut groups: std::collections::BTreeMap<(usize, u8), Vec<u32>> = Default::default();
    for ship in &snapshot.ships {
        if let Some(system) = scene.system_index(ship.system) {
            groups
                .entry((system, ship.owner))
                .or_default()
                .push(ship.id);
        }
    }
    let mut slot: std::collections::BTreeMap<usize, f32> = Default::default();
    for ((system, owner), mut ids) in groups {
        ids.sort_unstable();
        let n = slot.entry(system).or_insert(0.0);
        let step = 1.0 + *n;
        *n += 1.0;
        let base = positions[system];
        marks.push(MapShipMark {
            ids,
            owner,
            at: [0, 1, 2].map(|i| base[i] + beside[i] * step),
            heading: None,
            bound_for: None,
            system: Some(system),
        });
    }
    marks
}

/// The size of a ship mark on the star map seen through `view` from camera distance `distance`,
/// in map units (a fixed share of the picture's height, about five pixels of 720), and the step
/// between the marks beside a system, toward the screen's right.
pub fn map_mark_scale(view: &crate::zoom::SpaceView, distance: f32) -> (f32, [f32; 3]) {
    let per_pixel = distance * 2.0 * 22.5_f32.to_radians().tan() / 720.0;
    let icon = per_pixel * 5.0;
    let f = [
        view.target[0] - view.eye[0],
        view.target[1] - view.eye[1],
        view.target[2] - view.eye[2],
    ];
    let u = view.up;
    let right = [f[1] * u[2] - f[2] * u[1], 0.0, f[0] * u[1] - f[1] * u[0]];
    let length = right[0].hypot(right[2]).max(1e-6);
    (icon, right.map(|c| c / length * icon * 4.5))
}

/// Where a ship between systems shows in the plane of system `index`: at the rim, toward the
/// other end of its lane, with the direction it moves (outward when it left this system, inward
/// when it is bound for it). `None` for a transit that does not touch the system.
pub fn transit_at_rim(
    scene: &UniverseScene,
    positions: &[[f32; 3]],
    index: usize,
    transit: &TransitView,
) -> Option<([f32; 3], [f32; 3])> {
    let (a, b) = (
        scene.system_index(transit.from)?,
        scene.system_index(transit.to)?,
    );
    let (other, outward) = if a == index {
        (b, true)
    } else if b == index {
        (a, false)
    } else {
        return None;
    };
    let way = [
        positions[other][0] - positions[index][0],
        0.0,
        positions[other][2] - positions[index][2],
    ];
    let length = way[0].hypot(way[2]);
    if length <= 1e-3 {
        return None;
    }
    let toward = way.map(|c| c / length);
    let at = toward.map(|c| c * crate::universe::SYSTEM_RADIUS * 1.12);
    Some((at, if outward { toward } else { toward.map(|c| -c) }))
}

/// A flat arrowhead in the plane `y = at.y`, `size` units long, pointing along `dir`.
pub fn chevron(at: [f32; 3], dir: [f32; 3], size: f32, colour: [u8; 4]) -> Vec<Tri3> {
    let side = [-dir[2], 0.0, dir[0]];
    let point = |along: f32, across: f32| {
        [
            at[0] + dir[0] * along * size + side[0] * across * size,
            at[1],
            at[2] + dir[2] * along * size + side[2] * across * size,
        ]
    };
    let (tip, notch) = (point(0.75, 0.0), point(-0.2, 0.0));
    let (left, right) = (point(-0.6, -0.6), point(-0.6, 0.6));
    vec![
        Tri3 {
            v: [tip, left, notch],
            colour,
        },
        Tri3 {
            v: [tip, notch, right],
            colour,
        },
    ]
}

/// An arrowhead pointing along the 3D direction `dir`: two flat heads crossed at right angles
/// (one in the plane of `dir` and the horizontal, one perpendicular to it), so a ship climbing
/// between systems reads from above and from the side.
pub fn chevron3(at: [f32; 3], dir: [f32; 3], size: f32, colour: [u8; 4]) -> Vec<Tri3> {
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let unit = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
        v.map(|c| c / l)
    };
    let dir = unit(dir);
    let reference = if dir[1].abs() < 0.95 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let side = unit(cross(reference, dir));
    let normal = unit(cross(dir, side));
    let mut out = Vec::new();
    for across in [side, normal] {
        let point = |along: f32, wide: f32| {
            [0, 1, 2].map(|i| at[i] + (dir[i] * along + across[i] * wide) * size)
        };
        let (tip, notch) = (point(0.75, 0.0), point(-0.2, 0.0));
        let (left, right) = (point(-0.6, -0.6), point(-0.6, 0.6));
        out.push(Tri3 {
            v: [tip, left, notch],
            colour,
        });
        out.push(Tri3 {
            v: [tip, notch, right],
            colour,
        });
    }
    out
}

/// A flat diamond of half-diagonal `size` in the plane `y = at.y`.
pub fn diamond(at: [f32; 3], size: f32, colour: [u8; 4]) -> Vec<Tri3> {
    let p = |dx: f32, dz: f32| [at[0] + dx * size, at[1], at[2] + dz * size];
    vec![
        Tri3 {
            v: [p(1.0, 0.0), p(0.0, 1.0), p(-1.0, 0.0)],
            colour,
        },
        Tri3 {
            v: [p(1.0, 0.0), p(-1.0, 0.0), p(0.0, -1.0)],
            colour,
        },
    ]
}

/// What the pointer rests on in space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceHover {
    /// A ship, by entity id.
    Ship(u32),
    /// A body or a system.
    Target(Target),
}

/// The two lines of help for what the pointer rests on: its name, then what can be done with it.
/// `loaded` is the planet whose globe is loaded; `ships` tells whether ships are selected;
/// `known` tells whether the player knows a system (by id).
pub fn hover_help(
    scene: &UniverseScene,
    hover: SpaceHover,
    loaded: Option<usize>,
    ships: bool,
    known: impl Fn(u32) -> bool,
    resource: impl Fn(u32) -> Option<ResourceBodyView>,
) -> (String, String) {
    let send = if ships {
        " · right-click: send the selected ships"
    } else {
        ""
    };
    match hover {
        SpaceHover::Ship(_) => (
            "Ship".to_owned(),
            "Click to select · Space flies to the selection, Shift + Space follows it".to_owned(),
        ),
        SpaceHover::Target(Target::System(i)) => match scene.systems.get(i) {
            Some(system) if known(system.id) => (
                format!("{} (star system)", system.name),
                format!("Double-click or scroll in to enter{send}"),
            ),
            Some(system) => (
                format!("{} (unexplored)", system.name),
                format!("Send a ship to learn what it holds{send}"),
            ),
            None => Default::default(),
        },
        SpaceHover::Target(Target::Body(i)) => {
            let Some(body) = scene.bodies.get(i) else {
                return Default::default();
            };
            match body.kind {
                crate::universe::BodyViewKind::Star => (
                    format!("{} (star)", body.name),
                    "Double-click to centre the view on the star".to_owned(),
                ),
                crate::universe::BodyViewKind::Habitable => (
                    format!("{} (planet)", body.name),
                    if loaded == Some(i) {
                        format!("Double-click or scroll in to land the camera{send}")
                    } else {
                        format!("Double-click or scroll in to fly there and land the camera{send}")
                    },
                ),
                crate::universe::BodyViewKind::Resource => {
                    let action = if ships {
                        "Double-click to fly there · right-click: orbit and claim it"
                    } else {
                        "Double-click to fly there · ships orbiting it claim it"
                    };
                    match resource(body.id) {
                        Some(r) => (
                            format!("{} · {}", r.kind_name, r.yield_text()),
                            format!("{} · {action}", claim_text(r.claim)),
                        ),
                        None => (format!("{} (resource body)", body.name), action.to_owned()),
                    }
                }
            }
        }
    }
}

/// The claim state in words for the tooltip.
pub fn claim_text(claim: ClaimState) -> String {
    match claim {
        ClaimState::Unclaimed => "Unclaimed".to_owned(),
        ClaimState::Yours => "Claimed by you".to_owned(),
        ClaimState::Other(player) => format!("Claimed by player {player}"),
        ClaimState::Contested => "Contested".to_owned(),
    }
}

/// The help lines of a level when the pointer rests on nothing: how to move about and between
/// levels. `cap` is the farthest level the player may reach.
pub fn level_help(level: crate::zoom::Level, cap: crate::zoom::Level) -> Option<(String, String)> {
    use crate::zoom::Level;
    let lines = |a: &str, b: &str| Some((a.to_owned(), b.to_owned()));
    match level {
        Level::Surface | Level::Globe if cap >= Level::System => lines(
            "Scroll out past the globe to see the star system",
            "Home comes back to your planet",
        ),
        Level::Surface | Level::Globe => None,
        Level::System => Some((
            "Star system: scroll zooms to the cursor; double-click a planet to land".to_owned(),
            format!(
                "Middle-drag pans · Shift + middle-drag turns and tilts · Home: your planet{}",
                if cap >= Level::StarMap {
                    " · scroll out: star map"
                } else {
                    ""
                }
            ),
        )),
        Level::StarMap => lines(
            "Star map: scroll in on a system, or double-click it, to enter",
            "Middle-drag pans · Shift + middle-drag turns and tilts · Home: your planet",
        ),
    }
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
                radius: at.radius.max(CLUSTER_MIN_RADIUS) * CLUSTER_RADII * 1.2,
                colour: [r, g, b].map(|c| (c * 255.0).round() as u8),
            })
        })
        .collect()
}

/// The rings around the planets of `system` laid out as `placed` where players have cities,
/// one per owner in the owner's colour, each owner's a little wider than the one before.
pub fn colony_rings(
    scene: &UniverseScene,
    placed: &[PlacedBody],
    colonies: &[ColonyView],
    system: u32,
) -> Vec<ClaimRing> {
    let mut out: Vec<ClaimRing> = Vec::new();
    for colony in colonies.iter().filter(|c| c.system == system) {
        let Some(at) = scene
            .body_index(colony.body)
            .and_then(|index| placed.iter().find(|p| p.body == index))
        else {
            continue;
        };
        let rank = out.iter().filter(|r| r.body == colony.body).count() as f32;
        let [r, g, b] = owner_color(colony.owner);
        out.push(ClaimRing {
            body: colony.body,
            centre: at.pos,
            radius: at.radius * (COLONY_RING + COLONY_RING_STEP * rank),
            colour: [r, g, b].map(|c| (c * 255.0).round() as u8),
        });
    }
    out
}

/// Radius of a colony ring in planet radii.
pub const COLONY_RING: f32 = 1.8;
/// How much wider each further owner's ring on the same planet is, in planet radii.
pub const COLONY_RING_STEP: f32 = 0.45;

/// For the star map: each system with colonies, as `(system index, owner, rank)`, the rank
/// counting the owners already ringed around that system.
pub fn map_colony_rings(scene: &UniverseScene, colonies: &[ColonyView]) -> Vec<(usize, u8, u8)> {
    let mut out: Vec<(usize, u8, u8)> = Vec::new();
    for colony in colonies {
        let Some(index) = scene.system_index(colony.system) else {
            continue;
        };
        if out
            .iter()
            .any(|(i, o, _)| *i == index && *o == colony.owner)
        {
            continue;
        }
        let rank = out.iter().filter(|(i, _, _)| *i == index).count() as u8;
        out.push((index, colony.owner, rank));
    }
    out
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
        nearest_ship(&self.view, self.ships, cursor, 0.0)
    }

    /// Every ship drawn at the same mark as `id` (a fleet beside a system on the star map).
    fn mark_of(&self, id: u32) -> Vec<u32> {
        let Some(&(_, at)) = self.ships.iter().find(|s| s.0 == id) else {
            return vec![id];
        };
        let mut ids: Vec<u32> = self
            .ships
            .iter()
            .filter(|s| s.1 == at)
            .map(|s| s.0)
            .collect();
        ids.sort_unstable();
        ids
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

    /// Records the pointer without picking (it moves a camera drag).
    pub fn track(&mut self, x: f32, y: f32) {
        self.pointer = Some((x, y));
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
                self.press = Some(Press {
                    x,
                    y,
                    mods,
                    dragging: false,
                });
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
        if ctx.star_map {
            // On the star map a click takes the ships of the mark under it: a ship between
            // systems, or the fleet beside a system.
            let entities = ctx
                .ship_at((x, y))
                .map_or_else(Vec::new, |id| ctx.mark_of(id));
            return PickEvent::BoxSelect {
                entities,
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
            system: 1,
            body,
            x,
            y,
            facing_deg: 0.0,
            hp_frac: 1.0,
            model: None,
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

    fn transit(id: u32, from: u32, to: u32, progress: f32) -> TransitView {
        TransitView {
            id,
            kind: 1,
            owner: 1,
            from,
            to,
            progress,
        }
    }

    #[test]
    fn a_ship_on_a_climbing_lane_heads_up_along_it() {
        let mut scene = sample_scene();
        scene.systems[1].z = 90_000;
        let positions = scene.place_systems();
        let snapshot = SpaceSnapshot {
            transits: vec![transit(20, 1, 2, 0.5)],
            ..SpaceSnapshot::default()
        };
        let marks = map_ship_marks(&scene, &positions, &snapshot, [0.0; 3]);
        let heading = marks[0].heading.unwrap();
        let way: Vec<f32> = (0..3).map(|i| positions[1][i] - positions[0][i]).collect();
        let length = way.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!(heading[1] > 0.3, "{heading:?}");
        for i in 0..3 {
            assert!((heading[i] - way[i] / length).abs() < 1e-4);
        }
        // Its arrowhead has a point on the lane ahead and lies along the heading.
        let tris = chevron3(marks[0].at, heading, 100.0, [1, 2, 3, 4]);
        assert_eq!(tris.len(), 4);
        let tip = tris[0].v[0];
        let ahead: Vec<f32> = (0..3).map(|i| tip[i] - marks[0].at[i]).collect();
        let dot: f32 = (0..3).map(|i| ahead[i] * heading[i]).sum();
        assert!(dot > 70.0, "{dot}");
    }

    #[test]
    fn star_map_marks_every_transit_and_every_fleet() {
        let scene = sample_scene();
        let positions = scene.place_systems();
        let mut snapshot = SpaceSnapshot {
            transits: vec![transit(20, 1, 2, 0.25)],
            ships: vec![ship(10, None, 100.0, 0.0), ship(11, Some(3), 0.5, 0.0)],
            ..SpaceSnapshot::default()
        };
        let mut enemy = ship(12, None, 0.0, 0.0);
        enemy.owner = 2;
        enemy.system = 2;
        snapshot.ships.push(enemy);
        let marks = map_ship_marks(&scene, &positions, &snapshot, [500.0, 0.0, 0.0]);
        assert_eq!(marks.len(), 3);
        // The ship in transit stands on its lane, a quarter of the way, heading for system 2.
        let lane = &marks[0];
        assert_eq!(lane.ids, vec![20]);
        assert_eq!(lane.bound_for, Some(1));
        assert_eq!(lane.at, transit_point(positions[0], positions[1], 0.25));
        let heading = lane.heading.unwrap();
        let way = [
            positions[1][0] - positions[0][0],
            positions[1][2] - positions[0][2],
        ];
        assert!((heading[0] * way[1] - heading[2] * way[0]).abs() < 1e-2);
        assert!(heading[0] * way[0] + heading[2] * way[1] > 0.0);
        // The two ships of player one in system 1 are one mark beside it; the enemy another.
        let home = marks.iter().find(|m| m.system == Some(0)).unwrap();
        assert_eq!(home.ids, vec![10, 11]);
        assert_eq!(home.at, [positions[0][0] + 500.0, 0.0, positions[0][2]]);
        let far = marks.iter().find(|m| m.system == Some(1)).unwrap();
        assert_eq!((far.owner, far.ids.clone()), (2, vec![12]));
        // In the system plane the transit shows at the rim: outward from 1, inward to 2.
        let t = transit(20, 1, 2, 0.5);
        let (at, dir) = transit_at_rim(&scene, &positions, 0, &t).unwrap();
        assert!((at[0].hypot(at[2]) - crate::universe::SYSTEM_RADIUS * 1.12).abs() < 1.0);
        assert!(dir[0] * at[0] + dir[2] * at[2] > 0.0);
        let (at, dir) = transit_at_rim(&scene, &positions, 1, &t).unwrap();
        assert!(dir[0] * at[0] + dir[2] * at[2] < 0.0);
        assert!(transit_at_rim(&scene, &positions, 2, &t).is_none());
        assert_eq!(
            chevron([0.0; 3], [1.0, 0.0, 0.0], 10.0, [1, 2, 3, 4]).len(),
            2
        );
    }

    #[test]
    fn hover_help_names_the_object_and_its_actions() {
        use crate::zoom::Level;
        let scene = sample_scene();
        let (name, action) = hover_help(
            &scene,
            SpaceHover::Target(Target::System(1)),
            None,
            true,
            |_| true,
            |_| None,
        );
        assert_eq!(name, "S2 (star system)");
        assert!(action.contains("Double-click or scroll in to enter"));
        assert!(action.contains("send the selected ships"));
        let (name, action) = hover_help(
            &scene,
            SpaceHover::Target(Target::Body(1)),
            Some(1),
            false,
            |_| true,
            |_| None,
        );
        assert_eq!(name, "B2 (planet)");
        assert!(action.contains("land the camera") && !action.contains("send"));
        let (_, action) = hover_help(
            &scene,
            SpaceHover::Target(Target::Body(5)),
            Some(1),
            false,
            |_| true,
            |_| None,
        );
        assert!(action.contains("fly there and land"));
        let (name, _) = hover_help(
            &scene,
            SpaceHover::Target(Target::Body(2)),
            None,
            false,
            |_| true,
            |_| None,
        );
        assert!(name.contains("resource body"));
        assert_eq!(
            hover_help(&scene, SpaceHover::Ship(9), None, false, |_| true, |_| None).0,
            "Ship"
        );
        let (name, action) = hover_help(
            &scene,
            SpaceHover::Target(Target::System(1)),
            None,
            false,
            |id| id != 2,
            |_| None,
        );
        assert_eq!(name, "S2 (unexplored)");
        assert!(action.contains("Send a ship"));
        // A listed resource body names its kind, resource, yield and claim state.
        let listed = |claim| {
            Some(ResourceBodyView {
                body: 3,
                kind_name: "Tar Moon".into(),
                resource_name: "Oil".into(),
                icon: "icons/res_oil".into(),
                colour: [200, 160, 60],
                per_period: 42,
                period_secs: 30,
                claim,
            })
        };
        for (claim, words) in [
            (ClaimState::Unclaimed, "Unclaimed"),
            (ClaimState::Yours, "Claimed by you"),
            (ClaimState::Other(2), "Claimed by player 2"),
            (ClaimState::Contested, "Contested"),
        ] {
            let (name, action) = hover_help(
                &scene,
                SpaceHover::Target(Target::Body(2)),
                None,
                true,
                |_| true,
                |id| listed(claim).filter(|r| r.body == id),
            );
            assert_eq!(name, "Tar Moon · Oil +42 / 30 s");
            assert!(action.starts_with(words), "{action}");
            assert!(action.contains("orbit and claim it"));
        }
        assert!(level_help(Level::Globe, Level::Globe).is_none());
        assert!(
            level_help(Level::Globe, Level::System)
                .unwrap()
                .0
                .contains("Scroll out")
        );
        assert!(
            !level_help(Level::System, Level::System)
                .unwrap()
                .1
                .contains("star map")
        );
        assert!(
            level_help(Level::System, Level::StarMap)
                .unwrap()
                .1
                .contains("star map")
        );
        assert!(
            level_help(Level::StarMap, Level::StarMap)
                .unwrap()
                .0
                .contains("enter")
        );
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
        assert!((distance - body.radius.max(CLUSTER_MIN_RADIUS) * CLUSTER_RADII).abs() < 1e-2);
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

//! Projectiles you can see (task 95): what each weapon throws flies from the shooter's muzzle to
//! its target over the time the simulation gives it (task 102: the weapon's `projectile_speed`,
//! data; nothing here has a speed of its own), along a line or an arc, turned along its way, and
//! leaves a small effect where it lands. Presentation only: the render thread keeps the flights
//! from the shots the snapshots report ([`ShotView`]), and nothing here reaches the simulation.
//!
//! Bullets and bolts are short bright streaks and plasma a glowing bolt; arrows, stones, shells,
//! rockets, missiles and torpedoes are small models of the part kit drawn instanced, rockets and
//! missiles with a smoke trail; beams and rail shots are glowing lines drawn with additive
//! blending, a beam with a bright core, a soft halo and a flicker. A cap bounds the flights kept,
//! the oldest small ones dropped first.

use std::collections::HashSet;
use std::sync::Arc;

use crate::entities::{
    CloudView, EntityView, Footing, GroundFrame, building_height, entity_frame, heading_of,
    unit_height, unit_width,
};
use crate::models::{ModelLook, ModelSpec, PartSpec, Shape};
use crate::overlay::{Line3, Tri3};
use rom_kit::{MaterialLook, Program};

/// Ticks a beam fired by a weapon that hits at once is held from muzzle to target.
const BEAM_TICKS: f32 = 6.0;
/// Ticks a rail shot's streak takes to fade.
const RAIL_TICKS: f32 = 8.0;
/// Ticks the effect where a projectile lands lasts.
const IMPACT_TICKS: f64 = 10.0;
/// Ticks a shot is remembered after it was fired, so a later snapshot that still reports it
/// does not start it again.
const MEMORY_TICKS: f64 = 400.0;
/// Most flights kept at once, landing effects included.
pub const MAX_FLIGHTS: usize = 1500;
/// Most projectiles drawn for one shot of a weapon that fires several (`et_weapon.shots`).
const MAX_VOLLEY: u8 = 4;
/// How much larger projectiles are drawn to or from what is aloft (ships are drawn larger).
const ALOFT_SCALE: f32 = 2.5;

/// What a weapon throws, as drawn (`et_weapon.projectile`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProjectileKind {
    /// An arrow: a shaft with fletching on an arc.
    Arrow,
    /// A crossbow bolt: a short streak on a shallow arc.
    Bolt,
    /// A stone from a sling or a siege engine, on a high arc.
    Stone,
    /// A bullet: a short bright streak in a straight line.
    Bullet,
    /// A cannon or artillery shell on an arc.
    Shell,
    /// A rocket with a smoke trail.
    Rocket,
    /// A missile with a smoke trail.
    Missile,
    /// A torpedo running low and straight.
    Torpedo,
    /// A laser: a glowing beam held from muzzle to target.
    Beam,
    /// A glowing plasma bolt.
    Plasma,
    /// A rail shot: a bright streak that fades.
    Railgun,
    /// A kamikaze drone diving from its cloud (task 120): a short streak and a small blast.
    Drone,
}

/// What is drawn in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Body {
    /// A bright streak `length` tiles long.
    Streak { length: f32, colour: [u8; 3] },
    /// Model `index` of [`projectile_looks`], drawn `length` tiles long.
    Model { index: u16, length: f32 },
    /// A glowing bolt.
    Glow { colour: [u8; 3] },
    /// A glowing line held from muzzle to target, flickering.
    Beam { colour: [u8; 3] },
    /// A bright line from muzzle to target that fades.
    Rail { colour: [u8; 3] },
}

/// What a projectile leaves behind in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Trail {
    None,
    /// Smoke widening and fading back towards the launcher, a flame at the tail.
    Smoke,
    /// A short glowing streak, hot from the barrel.
    Hot,
    /// A pale wake on the water.
    Wake,
}

/// What is drawn where it lands.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Impact {
    /// A puff of dust of `size` tiles.
    Dust(f32),
    /// Sparks.
    Spark,
    /// A flash and smoke of `size` tiles.
    Flash(f32),
    /// A glow in a colour.
    Glow([u8; 3]),
}

/// How a kind is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Look {
    /// Height of the arc at mid-flight, as a share of the distance.
    arc: f32,
    body: Body,
    /// What it leaves behind.
    trail: Trail,
    impact: Impact,
    /// Dropped first when the cap is reached.
    small: bool,
}

const BEAM_COLOUR: [u8; 3] = [90, 210, 255];
const PLASMA_COLOUR: [u8; 3] = [200, 110, 255];
const RAIL_COLOUR: [u8; 3] = [190, 230, 255];
/// The flash of a projectile a shield shot down (task 157).
const SHIELD_COLOUR: [u8; 3] = [120, 215, 255];
/// The share of its way at which a projectile a shield shoots down is drawn hit (task 157;
/// the simulation only says that it was).
pub const INTERCEPT_AT: f32 = 0.75;

impl ProjectileKind {
    /// Every kind, in a fixed order.
    pub const ALL: [Self; 12] = [
        Self::Arrow,
        Self::Bolt,
        Self::Stone,
        Self::Bullet,
        Self::Shell,
        Self::Rocket,
        Self::Missile,
        Self::Torpedo,
        Self::Beam,
        Self::Plasma,
        Self::Railgun,
        Self::Drone,
    ];

    /// The `et_weapon.projectile` value of this kind.
    pub fn key(self) -> &'static str {
        match self {
            Self::Arrow => "arrow",
            Self::Bolt => "bolt",
            Self::Stone => "stone",
            Self::Bullet => "bullet",
            Self::Shell => "shell",
            Self::Rocket => "rocket",
            Self::Missile => "missile",
            Self::Torpedo => "torpedo",
            Self::Beam => "beam",
            Self::Plasma => "plasma",
            Self::Railgun => "railgun",
            Self::Drone => "drone",
        }
    }

    /// The kind an `et_weapon.projectile` value names; `None` for `none` and unknown values.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }

    fn look(self) -> Look {
        let model = |index: u16, length: f32| Body::Model { index, length };
        let (arc, body, trail, impact, small) = match self {
            Self::Arrow => (0.14, model(0, 1.0), Trail::None, Impact::Dust(0.3), true),
            Self::Bolt => (
                0.05,
                Body::Streak {
                    length: 0.7,
                    colour: [255, 225, 170],
                },
                Trail::None,
                Impact::Dust(0.25),
                true,
            ),
            Self::Stone => (0.25, model(1, 0.35), Trail::None, Impact::Dust(0.6), true),
            Self::Bullet => (
                0.0,
                Body::Streak {
                    length: 1.5,
                    colour: [255, 235, 160],
                },
                Trail::None,
                Impact::Spark,
                true,
            ),
            Self::Shell => (0.15, model(2, 0.6), Trail::Hot, Impact::Flash(1.0), false),
            Self::Rocket => (0.06, model(3, 0.9), Trail::Smoke, Impact::Flash(1.2), false),
            Self::Missile => (0.1, model(4, 1.2), Trail::Smoke, Impact::Flash(1.3), false),
            Self::Torpedo => (0.0, model(5, 1.2), Trail::Wake, Impact::Flash(1.0), false),
            Self::Beam => (
                0.0,
                Body::Beam {
                    colour: BEAM_COLOUR,
                },
                Trail::None,
                Impact::Glow(BEAM_COLOUR),
                false,
            ),
            Self::Plasma => (
                0.0,
                Body::Glow {
                    colour: PLASMA_COLOUR,
                },
                Trail::None,
                Impact::Glow(PLASMA_COLOUR),
                true,
            ),
            Self::Railgun => (
                0.0,
                Body::Rail {
                    colour: RAIL_COLOUR,
                },
                Trail::None,
                Impact::Spark,
                false,
            ),
            Self::Drone => (
                0.0,
                Body::Streak {
                    length: 0.6,
                    colour: [255, 200, 120],
                },
                Trail::None,
                Impact::Flash(0.6),
                true,
            ),
        };
        Look {
            arc,
            body,
            trail,
            impact,
            small,
        }
    }
}

/// An entity's latest shot as a snapshot reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotView {
    /// What the weapon throws.
    pub kind: ProjectileKind,
    /// The entity it was fired at, when known.
    pub target: Option<u32>,
    /// Face where the target stood at the snapshot.
    pub face: u8,
    /// East position of the target, tiles.
    pub x: f32,
    /// South position of the target, tiles.
    pub y: f32,
    /// Height of the target above sea level when it is aloft, tiles; 0 on the ground.
    pub altitude: f32,
    /// Height above the ground of a point in the air the shot flies to without a target entity
    /// (a drone of a grid, task 128), tiles; 0 otherwise.
    pub lift: f32,
    /// Tick the simulation fired it in.
    pub fired_tick: u64,
    /// Tick the simulation lands it in; the same as `fired_tick` for a hit at once.
    pub land_tick: u64,
    /// Projectiles of one shot (`et_weapon.shots`).
    pub count: u8,
    /// The share of its way at which a shield shot it down (task 157); `None` when it reaches
    /// its target.
    pub cut: Option<f32>,
}

/// The ticks, in the render thread's simulation time, at which a shot of `kind` fired at
/// `fired` and landing at `land` leaves the muzzle and reaches its target. A projectile the
/// simulation flies lands when the snapshot after its landing tick (the first one that shows
/// the hit) is reached, so its drawn speed is the data's; a hit at once (a beam, a rail shot)
/// is held as a line for a moment, anything else drawn for a tick.
pub fn flight_span(kind: ProjectileKind, fired: u64, land: u64) -> (f64, f64) {
    let start = fired as f64;
    if land > fired {
        return (start, (land + 1) as f64);
    }
    let ticks = match kind.look().body {
        Body::Beam { .. } => BEAM_TICKS,
        Body::Rail { .. } => RAIL_TICKS,
        _ => 1.0,
    };
    (start, start + f64::from(ticks))
}

/// One projectile on its way (or landing).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Flight {
    shooter: u32,
    fired: u64,
    kind: ProjectileKind,
    target: Option<u32>,
    /// Render ticks of leaving the muzzle and of reaching the target, or of being shot down.
    start: f64,
    end: f64,
    /// Render ticks of the whole way to the target.
    span: f64,
    /// The share of its way at which a shield shot it down (task 157); 1 when it reaches its
    /// target.
    cut: f32,
    from: [f32; 3],
    to: [f32; 3],
    /// Away from the ground under the shooter.
    up: [f32; 3],
    /// To or from what is aloft: a straight line.
    aloft: bool,
    /// To or from a ship aloft (ships are drawn larger): drawn larger too. Aircraft in the air
    /// are not (task 123).
    large: bool,
    count: u8,
}

impl Flight {
    fn distance(&self) -> f32 {
        length(sub(self.to, self.from))
    }

    /// Peak height of the arc, tiles.
    fn peak(&self) -> f32 {
        if self.aloft {
            0.0
        } else {
            (self.kind.look().arc * self.distance()).min(6.0)
        }
    }

    /// Where it is `t` (0 to 1) along its way.
    fn point(&self, t: f32) -> [f32; 3] {
        let lift = self.peak() * 4.0 * t * (1.0 - t);
        std::array::from_fn(|i| self.from[i] + (self.to[i] - self.from[i]) * t + self.up[i] * lift)
    }

    /// Its direction of travel at `t`.
    fn heading(&self, t: f32) -> [f32; 3] {
        let rise = self.peak() * 4.0 * (1.0 - 2.0 * t);
        normalize(std::array::from_fn(|i| {
            self.to[i] - self.from[i] + self.up[i] * rise
        }))
    }

    /// Share of its way done at render tick `now`: no farther than where it was shot down.
    fn progress(&self, now: f64) -> f32 {
        let span = self.span.max(1e-6);
        (((now - self.start) / span) as f32).clamp(0.0, self.cut)
    }

    /// Whether a shield shot it down on its way (task 157).
    fn intercepted(&self) -> bool {
        self.cut < 1.0
    }

    fn scale(&self) -> f32 {
        if self.large { ALOFT_SCALE } else { 1.0 }
    }

    /// A number in [0, 1) that tells this flight's copies and flickers apart.
    fn noise(&self, index: u32) -> f32 {
        noise(self.shooter ^ (self.fired as u32).rotate_left(16), index)
    }
}

/// A projectile model placed this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectileInstance {
    /// Index into [`projectile_looks`].
    pub model: u16,
    /// Centre.
    pub origin: [f32; 3],
    /// Front of the model (+z): its direction of travel.
    pub forward: [f32; 3],
    /// Up of the model.
    pub up: [f32; 3],
    /// Length drawn, tiles.
    pub length: f32,
    /// The owner colour its owner-coloured faces wear (a drone of a grid, task 128); grey for
    /// projectiles.
    pub team: [f32; 3],
    /// Where in its idle cycle it is, 0 to 1 (a drone's rotors, task 128).
    pub phase: f32,
}

/// What the projectiles of one frame draw.
#[derive(Clone, Debug, Default)]
pub struct ProjectileGeometry {
    /// Bright streaks, beams and glows, drawn with additive blending.
    pub glow: Vec<Tri3>,
    /// Smoke and dust, blended without writing depth.
    pub smoke: Vec<Tri3>,
    /// Thin lines (streak cores).
    pub lines: Vec<Line3>,
    /// Models in flight.
    pub models: Vec<ProjectileInstance>,
}

impl ProjectileGeometry {
    fn clear(&mut self) {
        self.glow.clear();
        self.smoke.clear();
        self.lines.clear();
        self.models.clear();
    }
}

/// The projectiles in flight, kept on the render thread from the snapshots' shots.
#[derive(Debug, Default)]
pub struct Projectiles {
    flights: Vec<Flight>,
    known: HashSet<(u32, u64)>,
    last_tick: Option<u64>,
}

impl Projectiles {
    /// Flights kept, landing effects included.
    pub fn len(&self) -> usize {
        self.flights.len()
    }

    /// Whether no flight is kept.
    pub fn is_empty(&self) -> bool {
        self.flights.is_empty()
    }

    /// Forgets every flight.
    pub fn clear(&mut self) {
        self.flights.clear();
        self.known.clear();
        self.last_tick = None;
    }

    /// Takes the shots of the snapshot at `latest_tick` (once per snapshot), follows moving
    /// targets and drops what has landed, at render tick `now`; `views` are the blended
    /// entities in id order, standing on `ground`.
    pub fn update(
        &mut self,
        now: f64,
        latest_tick: u64,
        views: &[EntityView],
        ground: &dyn Footing,
    ) {
        if self.last_tick.is_some_and(|last| latest_tick < last) {
            // A loaded game or a new match: time went back.
            self.clear();
        }
        if self.last_tick != Some(latest_tick) {
            self.last_tick = Some(latest_tick);
            for view in views {
                if let Some(shot) = view.shot {
                    self.launch(now, view, &shot, views, ground);
                }
            }
        }
        for flight in &mut self.flights {
            if now >= flight.end {
                continue;
            }
            if let Some(target) = flight.target.and_then(|id| find(views, id)) {
                flight.to = aim_point(target, ground);
            }
        }
        self.flights.retain(|f| now < f.end + IMPACT_TICKS);
        self.known
            .retain(|&(_, fired)| fired as f64 + MEMORY_TICKS > now);
        self.cap(MAX_FLIGHTS);
    }

    /// Starts the flight of `shot` fired by `view`, unless it is known or already over.
    fn launch(
        &mut self,
        now: f64,
        view: &EntityView,
        shot: &ShotView,
        views: &[EntityView],
        ground: &dyn Footing,
    ) {
        if !self.known.insert((view.id, shot.fired_tick)) {
            return;
        }
        let target = shot.target.and_then(|id| find(views, id));
        let to = match target {
            Some(target) => aim_point(target, ground),
            None if shot.altitude > 0.0 => {
                ground.datum_point(shot.face, shot.x, shot.y, (0.0, 0.0), shot.altitude)
            }
            None if shot.lift > 0.0 => {
                ground.point(shot.face, shot.x, shot.y, (0.0, 0.0), shot.lift)
            }
            None => ground.point(shot.face, shot.x, shot.y, (0.0, 0.0), 0.5),
        };
        // A drone operator's strike dives from its grid (tasks 120, 128).
        let from = match view.cloud {
            Some(cloud) => launch_point(&cloud, ground, to),
            None => muzzle(view, ground),
        };
        let aloft = is_aloft(view) || target.is_some_and(is_aloft) || shot.altitude > 0.0;
        let ship = |v: &EntityView| is_aloft(v) && v.spacecraft;
        let large = ship(view) || target.is_some_and(ship) || shot.altitude > 0.0;
        let (start, whole) = flight_span(shot.kind, shot.fired_tick, shot.land_tick);
        // A projectile a shield shot down ends where it was hit (task 157).
        let cut = shot.cut.map_or(1.0, |c| c.clamp(0.0, 1.0));
        let end = start + (whole - start) * f64::from(cut);
        if now >= end + IMPACT_TICKS {
            return;
        }
        self.flights.push(Flight {
            shooter: view.id,
            fired: shot.fired_tick,
            kind: shot.kind,
            target: shot.target,
            start,
            end,
            span: whole - start,
            cut,
            from,
            to,
            up: ground.frame(view.face, view.x, view.y).up,
            aloft,
            large,
            count: shot.count.clamp(1, MAX_VOLLEY),
        });
    }

    /// Keeps at most `cap` flights: the oldest small ones go first, then the oldest of all.
    fn cap(&mut self, cap: usize) {
        let Some(mut over) = self.flights.len().checked_sub(cap).filter(|n| *n > 0) else {
            return;
        };
        self.flights.sort_by(|a, b| a.start.total_cmp(&b.start));
        let mut keep = vec![true; self.flights.len()];
        for small_first in [true, false] {
            for (i, flight) in self.flights.iter().enumerate() {
                if over == 0 {
                    break;
                }
                if keep[i] && (!small_first || flight.kind.look().small) {
                    keep[i] = false;
                    over -= 1;
                }
            }
        }
        let mut index = 0;
        self.flights.retain(|_| {
            index += 1;
            keep[index - 1]
        });
    }

    /// Builds what the flights draw at render tick `now`.
    pub fn build(&self, now: f64, out: &mut ProjectileGeometry) {
        out.clear();
        for flight in &self.flights {
            if now < flight.start {
                continue;
            }
            if now < flight.end {
                in_flight(flight, now, out);
            } else {
                landed(flight, ((now - flight.end) / IMPACT_TICKS) as f32, out);
            }
        }
    }

    /// Where the projectiles of the flight fired by `shooter` at `fired` are at `now`, and
    /// whether it has landed (for tests).
    #[cfg(test)]
    fn position(&self, shooter: u32, fired: u64, now: f64) -> Option<([f32; 3], bool)> {
        self.flights
            .iter()
            .find(|f| f.shooter == shooter && f.fired == fired)
            .map(|f| (f.point(f.progress(now)), now >= f.end))
    }
}

fn find(views: &[EntityView], id: u32) -> Option<&EntityView> {
    views
        .binary_search_by_key(&id, |v| v.id)
        .ok()
        .map(|i| &views[i])
}

fn is_aloft(view: &EntityView) -> bool {
    if view.aircraft {
        return view.flight > 0.0;
    }
    view.altitude > 0.0
}

/// Size a drone of a grid is drawn, tiles across (task 128): the card's 0.18 read as specks at
/// the normal camera height, so half a slot's spacing.
pub const DRONE_SIZE: f32 = 0.3;
/// Index of the drone among [`projectile_looks`].
const DRONE_MODEL: u16 = 6;
/// Height in tiles of a drone's gentle bob.
const DRONE_BOB: f32 = 0.05;

/// Where slot `slot` of `cloud` is drawn, standing on `ground`: its place in the grid, lifted
/// to the grid's height, bobbing a little at render tick `now` (from the slot alone, the same
/// on every frame of the same moment).
fn slot_point(cloud: &CloudView, ground: &dyn Footing, slot: u32, now: f64) -> [f32; 3] {
    let frame = ground.frame(cloud.face, cloud.x, cloud.y);
    let (dx, dy) = cloud.slot_delta(slot);
    let bob = DRONE_BOB * (now as f32 * 0.12 + slot as f32 * 1.7).sin();
    frame.at([dx, cloud.hover + bob, dy])
}

/// Where a drone that dives at `to` leaves `cloud`: the slot nearest `to` among those whose
/// drone just left (else among those holding one), else the grid's centre.
fn launch_point(cloud: &CloudView, ground: &dyn Footing, to: [f32; 3]) -> [f32; 3] {
    let slots = if cloud.left != 0 {
        cloud.left
    } else {
        cloud.ready
    };
    (0..64_u32)
        .filter(|slot| slots >> slot & 1 == 1)
        .map(|slot| slot_point(cloud, ground, slot, 0.0))
        .min_by(|a, b| length(sub(*a, to)).total_cmp(&length(sub(*b, to))))
        .unwrap_or_else(|| ground.point(cloud.face, cloud.x, cloud.y, (0.0, 0.0), cloud.hover))
}

/// Adds the drones of every operator's grid in `views` to `out` at render tick `now` (task 128):
/// a quadrotor in each slot that holds a drone, facing the way the grid faces, in its owner's
/// colour, bobbing gently, its rotors a little out of step with its neighbours'; the drone on its
/// way to its slot where it flies. Everything comes from the view, the tick and the slot.
/// Presentation only.
pub fn drone_grids(
    views: &[EntityView],
    now: f64,
    ground: &dyn Footing,
    out: &mut ProjectileGeometry,
) {
    for view in views {
        let Some(cloud) = view.cloud.filter(|c| c.drones() > 0) else {
            continue;
        };
        let frame = ground.frame(cloud.face, cloud.x, cloud.y);
        let (sin, cos) = cloud.facing_deg.to_radians().sin_cos();
        let forward: [f32; 3] = std::array::from_fn(|k| frame.ex[k] * cos + frame.ey[k] * sin);
        let team = crate::entities::owner_color(view.owner);
        let phase = |slot: u32| (slot as f32 * 0.37 + view.id as f32 * 0.13).fract();
        for slot in (0..64_u32).filter(|slot| cloud.ready >> slot & 1 == 1) {
            out.models.push(ProjectileInstance {
                model: DRONE_MODEL,
                origin: slot_point(&cloud, ground, slot, now),
                forward,
                up: frame.up,
                length: DRONE_SIZE,
                team,
                phase: phase(slot),
            });
        }
        if let Some((slot, face, x, y)) = cloud.joining {
            let at = ground.point(face, x, y, (0.0, 0.0), cloud.hover);
            let to = slot_point(&cloud, ground, u32::from(slot), now);
            let way = sub(to, at);
            out.models.push(ProjectileInstance {
                model: DRONE_MODEL,
                origin: at,
                forward: if length(way) > 1e-3 {
                    normalize(way)
                } else {
                    forward
                },
                up: frame.up,
                length: DRONE_SIZE,
                team,
                phase: phase(u32::from(slot)),
            });
        }
    }
}

/// Where a shot leaves `view`: high on a building, at the bow of a ship aloft, at the
/// shoulder and a little ahead of a unit on the ground.
fn muzzle(view: &EntityView, ground: &dyn Footing) -> [f32; 3] {
    let frame = entity_frame(view, ground);
    if view.building {
        return frame.at([0.0, building_height(view) * 0.8, 0.0]);
    }
    let ahead = heading_of(&frame, view.facing_deg);
    let (rise, reach) = if is_aloft(view) {
        (unit_height(view) * 0.3, unit_width(view) * 0.3)
    } else {
        (unit_height(view) * 0.6, unit_width(view) * 0.35)
    };
    let base = frame.at([0.0, rise, 0.0]);
    std::array::from_fn(|i| base[i] + ahead[i] * reach)
}

/// Where a shot at `view` strikes it: halfway up.
fn aim_point(view: &EntityView, ground: &dyn Footing) -> [f32; 3] {
    let frame: GroundFrame = entity_frame(view, ground);
    let height = if view.building {
        building_height(view) * 0.5
    } else {
        unit_height(view) * 0.5
    };
    frame.at([0.0, height, 0.0])
}

/// The flight `f` on its way at `now`.
fn in_flight(f: &Flight, now: f64, out: &mut ProjectileGeometry) {
    let look = f.kind.look();
    let t = f.progress(now);
    let scale = f.scale();
    match look.body {
        Body::Beam { colour } => {
            let flicker = 0.75 + 0.25 * (now as f32 * 2.7 + f.noise(0) * 6.0).sin();
            // The halo, then the hot core over it, then the impact glow.
            ribbon(
                &mut out.glow,
                f.from,
                f.to,
                0.3 * scale * flicker,
                f.up,
                with_alpha(colour, 0.35),
            );
            ribbon(
                &mut out.glow,
                f.from,
                f.to,
                0.07 * scale,
                f.up,
                with_alpha(brighten(colour, 0.7), 0.9),
            );
            out.lines.push(Line3 {
                a: f.from,
                b: f.to,
                colour: [240, 252, 255, 255],
            });
            star(
                &mut out.glow,
                f.to,
                0.6 * scale * flicker,
                f.up,
                colour,
                0.8,
            );
            star(&mut out.glow, f.from, 0.3 * scale, f.up, colour, 0.6);
        }
        Body::Rail { colour } => {
            let fade = 1.0 - t;
            ribbon(
                &mut out.glow,
                f.from,
                f.to,
                0.12 * scale,
                f.up,
                with_alpha(colour, 0.6 * fade),
            );
            out.lines.push(Line3 {
                a: f.from,
                b: f.to,
                colour: [255, 255, 255, (255.0 * fade) as u8],
            });
            if t < 0.5 {
                star(
                    &mut out.glow,
                    f.from,
                    0.4 * scale,
                    f.up,
                    colour,
                    1.0 - 2.0 * t,
                );
            }
        }
        _ => {
            for copy in 0..u32::from(f.count) {
                // A volley: each copy a little behind and beside the first.
                let lag = f.noise(copy) * 0.08 * copy.min(1) as f32;
                let tc = (t - lag).max(0.0);
                if tc <= 0.0 && copy > 0 {
                    continue;
                }
                one_projectile(f, &look, tc, scale, out);
            }
        }
    }
}

/// One projectile of a flight `t` along its way.
fn one_projectile(f: &Flight, look: &Look, t: f32, scale: f32, out: &mut ProjectileGeometry) {
    let head = f.point(t);
    let dir = f.heading(t);
    let distance = f.distance().max(0.01);
    match look.body {
        Body::Streak { length, colour } => {
            let back = (length * scale / distance).min(t);
            let tail = f.point(t - back);
            out.lines.push(Line3 {
                a: tail,
                b: head,
                colour: [colour[0], colour[1], colour[2], 255],
            });
            ribbon(
                &mut out.glow,
                tail,
                head,
                0.05 * scale,
                f.up,
                with_alpha(colour, 0.8),
            );
        }
        Body::Glow { colour } => {
            let tail: [f32; 3] = std::array::from_fn(|i| head[i] - dir[i] * 0.7 * scale);
            ribbon(
                &mut out.glow,
                tail,
                head,
                0.22 * scale,
                f.up,
                with_alpha(colour, 0.55),
            );
            ribbon(
                &mut out.glow,
                tail,
                head,
                0.07 * scale,
                f.up,
                with_alpha(brighten(colour, 0.7), 0.95),
            );
            star(&mut out.glow, head, 0.3 * scale, f.up, colour, 0.7);
        }
        Body::Model { index, length } => {
            out.models.push(ProjectileInstance {
                model: index,
                origin: head,
                forward: dir,
                up: f.up,
                length: length * scale,
                team: [0.5; 3],
                phase: 0.0,
            });
            match look.trail {
                Trail::None => {}
                Trail::Smoke => trail(f, t, scale, out),
                Trail::Hot => {
                    let back = |k: f32| -> [f32; 3] {
                        std::array::from_fn(|i| head[i] - dir[i] * length * scale * k)
                    };
                    ribbon(
                        &mut out.glow,
                        back(2.2),
                        back(0.35),
                        0.07 * scale,
                        f.up,
                        [255, 160, 60, 150],
                    );
                }
                Trail::Wake => {
                    for k in 1..=4 {
                        let back = t - 0.35 * scale * k as f32 / distance;
                        if back > 0.0 {
                            puff(
                                &mut out.smoke,
                                f.point(back),
                                0.12 * k as f32 * scale,
                                f.up,
                                [235, 240, 245],
                                0.45 - 0.08 * k as f32,
                            );
                        }
                    }
                }
            }
        }
        Body::Beam { .. } | Body::Rail { .. } => {}
    }
}

/// Smoke behind a rocket or a missile `t` along its way, widening and fading back towards the
/// launcher, and the flame at its tail.
fn trail(f: &Flight, t: f32, scale: f32, out: &mut ProjectileGeometry) {
    let distance = f.distance().max(0.01);
    let step = 0.45 * scale / distance;
    let dir = f.heading(t);
    let half = f.kind.look().body_length() * scale * 0.5;
    let tail: [f32; 3] = std::array::from_fn(|i| f.point(t)[i] - dir[i] * half);
    star(&mut out.glow, tail, 0.25 * scale, f.up, [255, 170, 60], 0.9);
    for k in 1..=8 {
        let back = t - step * k as f32;
        if back <= 0.0 {
            break;
        }
        let fade = 1.0 - k as f32 / 9.0;
        puff(
            &mut out.smoke,
            f.point(back),
            (0.12 + 0.05 * k as f32) * scale,
            f.up,
            [200, 200, 195],
            0.5 * fade,
        );
    }
}

/// The flash of a projectile a shield shot down (task 157), `age` (0 to 1) of the way through:
/// a hard white-blue burst where it was hit, sparks thrown off and a glow fading.
fn shot_down(f: &Flight, age: f32, out: &mut ProjectileGeometry) {
    let scale = f.scale();
    let at = f.point(f.cut);
    let fade = (1.0 - age).clamp(0.0, 1.0);
    if age < 0.5 {
        let flash = 1.0 - age / 0.5;
        star(
            &mut out.glow,
            at,
            1.0 * scale * (0.5 + age),
            f.up,
            SHIELD_COLOUR,
            flash,
        );
        star(
            &mut out.glow,
            at,
            0.35 * scale,
            f.up,
            [245, 252, 255],
            flash,
        );
    }
    for k in 0..8 {
        let ray = normalize([
            f.noise(k * 3 + 40) - 0.5,
            f.noise(k * 3 + 41) - 0.5,
            f.noise(k * 3 + 42) - 0.5,
        ]);
        let reach = 1.2 * scale * (0.3 + age);
        let end: [f32; 3] = std::array::from_fn(|i| at[i] + ray[i] * reach);
        out.lines.push(Line3 {
            a: at,
            b: end,
            colour: [200, 240, 255, (255.0 * fade) as u8],
        });
        ribbon(
            &mut out.glow,
            at,
            end,
            0.04 * scale,
            f.up,
            with_alpha(SHIELD_COLOUR, 0.7 * fade),
        );
    }
    puff(
        &mut out.smoke,
        at,
        0.6 * scale * (0.5 + age),
        f.up,
        [120, 130, 140],
        0.35 * fade,
    );
}

/// The effect of the flight `f` where it landed, `age` (0 to 1) of the way through.
fn landed(f: &Flight, age: f32, out: &mut ProjectileGeometry) {
    if f.intercepted() {
        shot_down(f, age, out);
        return;
    }
    let look = f.kind.look();
    let scale = f.scale();
    let at = f.to;
    let fade = (1.0 - age).clamp(0.0, 1.0);
    match look.impact {
        Impact::Dust(size) => {
            for k in 0..3 {
                let rise = f.up.map(|u| u * age * 0.4 * size * (1.0 + k as f32 * 0.5));
                let centre: [f32; 3] =
                    std::array::from_fn(|i| at[i] + rise[i] + (f.noise(k + 10) - 0.5) * size * 0.6);
                puff(
                    &mut out.smoke,
                    centre,
                    size * scale * (0.4 + 0.6 * age),
                    f.up,
                    [150, 128, 96],
                    0.6 * fade,
                );
            }
        }
        Impact::Spark => {
            for k in 0..5 {
                let ray = normalize([
                    f.noise(k * 3) - 0.5,
                    f.noise(k * 3 + 1) - 0.5,
                    f.noise(k * 3 + 2) - 0.5,
                ]);
                let reach = 0.35 * scale * (0.3 + age);
                let end: [f32; 3] = std::array::from_fn(|i| at[i] + ray[i] * reach);
                let alpha = (255.0 * fade) as u8;
                out.lines.push(Line3 {
                    a: at,
                    b: end,
                    colour: [255, 220, 140, alpha],
                });
            }
            if age < 0.4 {
                star(
                    &mut out.glow,
                    at,
                    0.2 * scale,
                    f.up,
                    [255, 210, 120],
                    1.0 - age * 2.5,
                );
            }
        }
        Impact::Flash(size) => {
            if age < 0.45 {
                let flash = 1.0 - age / 0.45;
                star(
                    &mut out.glow,
                    at,
                    size * scale * (0.5 + age),
                    f.up,
                    [255, 180, 80],
                    flash,
                );
                star(
                    &mut out.glow,
                    at,
                    size * scale * 0.4,
                    f.up,
                    [255, 245, 210],
                    flash,
                );
            }
            let rise = f.up.map(|u| u * age * size * 0.6);
            puff(
                &mut out.smoke,
                std::array::from_fn(|i| at[i] + rise[i]),
                size * scale * (0.4 + 0.5 * age),
                f.up,
                [90, 86, 80],
                0.6 * fade,
            );
        }
        Impact::Glow(colour) => {
            star(&mut out.glow, at, 0.6 * scale * fade, f.up, colour, fade);
        }
    }
}

impl Look {
    /// Length of a model body, tiles; 0 for the others.
    fn body_length(&self) -> f32 {
        match self.body {
            Body::Model { length, .. } => length,
            _ => 0.0,
        }
    }
}

/// Two quads crossing along `a`–`b`, `half` tiles to each side: a line with width seen from any
/// side.
fn ribbon(out: &mut Vec<Tri3>, a: [f32; 3], b: [f32; 3], half: f32, up: [f32; 3], colour: [u8; 4]) {
    let d = sub(b, a);
    if length(d) < 1e-5 || half <= 0.0 {
        return;
    }
    let (s1, s2) = across(normalize(d), up);
    for side in [s1, s2] {
        let o = side.map(|v| v * half);
        let p =
            |q: [f32; 3], sign: f32| -> [f32; 3] { std::array::from_fn(|i| q[i] + o[i] * sign) };
        quad(out, [p(a, -1.0), p(a, 1.0), p(b, 1.0), p(b, -1.0)], colour);
    }
}

/// A glow of `radius` tiles round `at`: three crossed diamonds.
fn star(out: &mut Vec<Tri3>, at: [f32; 3], radius: f32, up: [f32; 3], colour: [u8; 3], alpha: f32) {
    if radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let colour = with_alpha(colour, alpha);
    let (s1, s2) = across(normalize(up), [1.0, 0.0, 0.0]);
    let axes = [normalize(up), s1, s2];
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let p = |u: f32, v: f32| -> [f32; 3] {
            std::array::from_fn(|i| at[i] + (axes[a][i] * u + axes[b][i] * v) * radius)
        };
        quad(
            out,
            [p(1.0, 0.0), p(0.0, 1.0), p(-1.0, 0.0), p(0.0, -1.0)],
            colour,
        );
    }
}

/// A soft blob of `radius` tiles round `at`: two crossed hexagons.
fn puff(out: &mut Vec<Tri3>, at: [f32; 3], radius: f32, up: [f32; 3], colour: [u8; 3], alpha: f32) {
    if radius <= 0.0 || alpha <= 0.0 {
        return;
    }
    let colour = with_alpha(colour, alpha);
    let (s1, s2) = across(normalize(up), [1.0, 0.0, 0.0]);
    for (u, v) in [(normalize(up), s1), (s1, s2)] {
        let rim = |k: usize| -> [f32; 3] {
            let a = k as f32 / 6.0 * std::f32::consts::TAU;
            std::array::from_fn(|i| at[i] + (u[i] * a.cos() + v[i] * a.sin()) * radius)
        };
        for k in 0..6 {
            out.push(Tri3 {
                v: [at, rim(k), rim(k + 1)],
                colour,
            });
        }
    }
}

fn quad(out: &mut Vec<Tri3>, v: [[f32; 3]; 4], colour: [u8; 4]) {
    out.push(Tri3 {
        v: [v[0], v[1], v[2]],
        colour,
    });
    out.push(Tri3 {
        v: [v[0], v[2], v[3]],
        colour,
    });
}

/// Two unit vectors square to `d` and to each other, the first also square to `hint` when it
/// can be.
fn across(d: [f32; 3], hint: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let mut s1 = cross(d, hint);
    if length(s1) < 1e-3 {
        s1 = cross(d, [0.0, 0.0, 1.0]);
        if length(s1) < 1e-3 {
            s1 = cross(d, [1.0, 0.0, 0.0]);
        }
    }
    let s1 = normalize(s1);
    (s1, normalize(cross(d, s1)))
}

fn with_alpha(c: [u8; 3], alpha: f32) -> [u8; 4] {
    [c[0], c[1], c[2], (alpha.clamp(0.0, 1.0) * 255.0) as u8]
}

/// `c` moved `t` of the way to white.
fn brighten(c: [u8; 3], t: f32) -> [u8; 3] {
    c.map(|v| (f32::from(v) + (255.0 - f32::from(v)) * t) as u8)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = length(v);
    if l < 1e-9 {
        [0.0, 1.0, 0.0]
    } else {
        v.map(|c| c / l)
    }
}

/// A number in [0, 1) from a seed and an index, the same every frame.
fn noise(seed: u32, index: u32) -> f32 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B1)
        .wrapping_add(index.wrapping_mul(0x85EB_CA77));
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn material(program: Program, colour: [f32; 3], colour_alt: [f32; 3]) -> MaterialLook {
    MaterialLook {
        program,
        colour,
        colour_alt,
        scale: 0.25,
    }
}

/// A part lying along the model's front (+z), its foot `z` tiles from the origin.
fn lying(name: &str, shape: Shape, look: MaterialLook, z: f32) -> PartSpec {
    PartSpec::new(name, shape, look)
        .knob("pitch", 90.0)
        .knob("z", z)
}

/// The models of the big projectiles, built from parts of the kit, each about one tile long,
/// pointing along +z and centred on the origin; [`Body::Model`] indexes them.
pub fn projectile_looks() -> Vec<ModelLook> {
    let wood = material(Program::Timber, [0.55, 0.38, 0.2], [0.4, 0.27, 0.14]);
    let iron = material(Program::Metal, [0.32, 0.32, 0.34], [0.2, 0.2, 0.22]);
    let brass = material(Program::Metal, [0.72, 0.56, 0.3], [0.55, 0.42, 0.22]);
    let steel = material(Program::Metal, [0.55, 0.56, 0.58], [0.4, 0.4, 0.42]);
    let olive = material(Program::Metal, [0.36, 0.4, 0.26], [0.26, 0.3, 0.18]);
    let white = material(Program::Metal, [0.86, 0.87, 0.88], [0.7, 0.7, 0.72]);
    let feather = material(Program::Cloth, [0.92, 0.9, 0.84], [0.8, 0.78, 0.7]);
    let stone = material(Program::Stone, [0.52, 0.5, 0.46], [0.4, 0.38, 0.35]);
    let flame = material(Program::Emissive, [1.0, 0.62, 0.2], [1.0, 0.85, 0.4]);
    let dark = material(Program::Metal, [0.18, 0.19, 0.2], [0.12, 0.12, 0.13]);
    // A drone's rotor: a two-bladed propeller lying face up, turning all the time.
    let rotor = |name: &str, x: f32, z: f32| {
        PartSpec::new(name, Shape::Propeller, steel)
            .knob("w", 0.36)
            .knob("blades", 2.0)
            .knob("chord", 0.06)
            .knob("pitch", -90.0)
            .knob("x", x)
            .knob("z", z)
            .knob("y", 0.08)
    };
    let fin = |name: &str, look: MaterialLook, z: f32, w: f32, d: f32, roll: f32| {
        PartSpec::new(name, Shape::Box, look)
            .knob("w", w)
            .knob("d", d)
            .knob("h", 0.02)
            .knob("y", -0.01)
            .knob("z", z)
            .knob("roll", roll)
    };
    let models: [(&str, Vec<PartSpec>); 7] = [
        (
            "projectile.arrow",
            vec![
                lying("shaft", Shape::Cylinder, wood, -0.5)
                    .knob("w", 0.05)
                    .knob("h", 0.86)
                    .knob("segments", 4.0),
                lying("head", Shape::Cone, iron, 0.36)
                    .knob("w", 0.11)
                    .knob("h", 0.14)
                    .knob("segments", 4.0),
                fin("fletching", feather, -0.4, 0.17, 0.18, 0.0),
                fin("fletching_up", feather, -0.4, 0.17, 0.18, 90.0),
            ],
        ),
        (
            "projectile.stone",
            vec![
                PartSpec::new("top", Shape::Dome, stone)
                    .knob("w", 1.0)
                    .knob("h", 0.45)
                    .knob("segments", 8.0),
                PartSpec::new("bottom", Shape::Dome, stone)
                    .knob("w", 1.0)
                    .knob("h", 0.45)
                    .knob("segments", 8.0)
                    .knob("pitch", 180.0),
            ],
        ),
        (
            "projectile.shell",
            vec![
                lying("case", Shape::Cylinder, brass, -0.5)
                    .knob("w", 0.4)
                    .knob("h", 0.6)
                    .knob("segments", 8.0)
                    .knob("bottom", 1.0),
                lying("nose", Shape::Cone, steel, 0.1)
                    .knob("w", 0.4)
                    .knob("h", 0.4)
                    .knob("segments", 8.0),
            ],
        ),
        (
            "projectile.rocket",
            vec![
                lying("body", Shape::Cylinder, olive, -0.42)
                    .knob("w", 0.12)
                    .knob("h", 0.72)
                    .knob("segments", 6.0),
                lying("nose", Shape::Cone, olive, 0.3)
                    .knob("w", 0.12)
                    .knob("h", 0.2)
                    .knob("segments", 6.0),
                fin("fins", dark, -0.38, 0.3, 0.12, 0.0),
                fin("fins_up", dark, -0.38, 0.3, 0.12, 90.0),
                lying("flame", Shape::Cone, flame, -0.42)
                    .knob("w", 0.1)
                    .knob("h", 0.08)
                    .knob("pitch", -90.0),
            ],
        ),
        (
            "projectile.missile",
            vec![
                lying("body", Shape::Cylinder, white, -0.45)
                    .knob("w", 0.1)
                    .knob("h", 0.8)
                    .knob("segments", 8.0),
                lying("nose", Shape::Cone, steel, 0.35)
                    .knob("w", 0.1)
                    .knob("h", 0.15)
                    .knob("segments", 8.0),
                fin("wings", steel, 0.0, 0.26, 0.1, 0.0),
                fin("tail", dark, -0.4, 0.24, 0.1, 0.0),
                fin("tail_up", dark, -0.4, 0.24, 0.1, 90.0),
                lying("flame", Shape::Cone, flame, -0.45)
                    .knob("w", 0.08)
                    .knob("h", 0.06)
                    .knob("pitch", -90.0),
            ],
        ),
        (
            "projectile.torpedo",
            vec![
                lying("body", Shape::Cylinder, dark, -0.45)
                    .knob("w", 0.14)
                    .knob("h", 0.8)
                    .knob("segments", 8.0),
                PartSpec::new("nose", Shape::Dome, dark)
                    .knob("w", 0.14)
                    .knob("h", 0.1)
                    .knob("segments", 8.0)
                    .knob("pitch", 90.0)
                    .knob("z", 0.35),
                fin("rudder", steel, -0.42, 0.2, 0.06, 90.0),
                fin("planes", steel, -0.42, 0.2, 0.06, 0.0),
                lying("screw", Shape::Cone, steel, -0.45)
                    .knob("w", 0.1)
                    .knob("h", 0.06)
                    .knob("pitch", -90.0),
            ],
        ),
        // Task 128: a quadrotor of a drone grid, about a tile across before it is scaled: a body
        // in its owner's colour, two crossed arms, four rotors turning, a small charge below.
        (
            "projectile.drone",
            vec![
                PartSpec {
                    team: true,
                    ..PartSpec::new("body", Shape::Box, olive)
                        .knob("w", 0.4)
                        .knob("d", 0.44)
                        .knob("h", 0.16)
                },
                PartSpec::new("charge", Shape::Box, dark)
                    .knob("w", 0.16)
                    .knob("d", 0.2)
                    .knob("h", 0.08)
                    .knob("y", -0.08),
                fin("arms", dark, 0.0, 0.82, 0.06, 0.0).knob("yaw", 45.0),
                fin("arms_cross", dark, 0.0, 0.82, 0.06, 0.0).knob("yaw", -45.0),
                rotor("rotor_ne", 0.29, 0.29),
                rotor("rotor_nw", -0.29, 0.29),
                rotor("rotor_se", 0.29, -0.29),
                rotor("rotor_sw", -0.29, -0.29),
            ],
        ),
    ];
    models
        .into_iter()
        .map(|(id, parts)| ModelLook {
            id: id.into(),
            spec: Arc::new(ModelSpec { parts }),
            scale: 1.0,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::FlatGround;

    fn view(id: u32, x: f32) -> EntityView {
        EntityView {
            id,
            kind: 1,
            owner: 1,
            face: 0,
            x,
            y: 10.0,
            facing_deg: 0.0,
            altitude: 0.0,
            shot: None,
            spacecraft: false,
            flight: 0.0,
            aircraft: false,
            hover: crate::Hover::Circle,
            cloud: None,
            hp_frac: 1.0,
            building: false,
            footprint: (1, 1),
            radius: 0.4,
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
            activity: Default::default(),
        }
    }

    fn shot(kind: ProjectileKind, target: u32, fired: u64, land: u64) -> ShotView {
        ShotView {
            kind,
            target: Some(target),
            face: 0,
            x: 20.0,
            y: 10.0,
            altitude: 0.0,
            lift: 0.0,
            fired_tick: fired,
            land_tick: land,
            count: 1,
            cut: None,
        }
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-3)
    }

    #[test]
    fn kinds_round_trip_their_keys() {
        for kind in ProjectileKind::ALL {
            assert_eq!(ProjectileKind::from_key(kind.key()), Some(kind));
        }
        assert_eq!(ProjectileKind::from_key("none"), None);
    }

    #[test]
    fn a_flight_ends_at_the_target_when_the_simulation_lands_it() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        let mut shooter = view(1, 10.0);
        shooter.shot = Some(shot(ProjectileKind::Arrow, 2, 100, 108));
        let target = view(2, 20.0);
        let views = [shooter, target];
        let mut projectiles = Projectiles::default();
        projectiles.update(100.2, 101, &views, &ground);
        assert_eq!(projectiles.len(), 1);
        // The snapshot after the landing tick shows the hit: the flight ends there.
        assert_eq!(flight_span(ProjectileKind::Arrow, 100, 108), (100.0, 109.0));
        let at_target = aim_point(&target, &ground);
        let (mid, landed) = projectiles.position(1, 100, 104.5).unwrap();
        assert!(!landed && !close(mid, at_target));
        // On an arc: above the straight line half way.
        assert!(mid[1] > at_target[1] + 0.5);
        let (end, landed) = projectiles.position(1, 100, 109.0).unwrap();
        assert!(landed && close(end, at_target));
        // A later snapshot that still reports the shot does not start it again.
        projectiles.update(102.0, 102, &views, &ground);
        assert_eq!(projectiles.len(), 1);
        // It follows a target that moves.
        let moved = [shooter, view(2, 22.0)];
        projectiles.update(103.0, 103, &moved, &ground);
        let (end, _) = projectiles.position(1, 100, 109.0).unwrap();
        assert!(close(end, aim_point(&moved[1], &ground)));
        // Gone once its landing effect is over.
        projectiles.update(109.0 + IMPACT_TICKS, 120, &moved, &ground);
        assert!(projectiles.is_empty());
    }

    #[test]
    fn every_kind_flies_for_the_time_the_simulation_gives_it() {
        // Task 102: the drawn flight of every kind ends with the snapshot that shows the hit,
        // whatever the kind; the renderer has no speed of its own.
        for kind in ProjectileKind::ALL {
            assert_eq!(flight_span(kind, 50, 57), (50.0, 58.0), "{kind:?}");
        }
        // A hit at once: a beam held, a rail streak fading, anything else for one tick.
        let (start, beam) = flight_span(ProjectileKind::Beam, 50, 50);
        assert_eq!((start, beam - 50.0), (50.0, f64::from(BEAM_TICKS)));
        let (_, rail) = flight_span(ProjectileKind::Railgun, 50, 50);
        assert_eq!(rail - 50.0, f64::from(RAIL_TICKS));
        let (_, bullet) = flight_span(ProjectileKind::Bullet, 50, 50);
        assert_eq!(bullet - 50.0, 1.0);
    }

    #[test]
    fn the_cap_drops_the_oldest_small_flights_first() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        let mut projectiles = Projectiles::default();
        let target = view(1_000_000, 20.0);
        // Shells first (oldest), then many more bullets than the cap allows.
        let mut views: Vec<EntityView> = Vec::new();
        for id in 0..10 {
            let mut v = view(id, 10.0);
            v.shot = Some(shot(ProjectileKind::Shell, target.id, 10, 10));
            views.push(v);
        }
        views.push(target);
        projectiles.update(10.0, 11, &views, &ground);
        let mut bullets: Vec<EntityView> = (100..100 + MAX_FLIGHTS as u32 + 50)
            .map(|id| {
                let mut v = view(id, 10.0);
                v.shot = Some(shot(ProjectileKind::Bullet, target.id, 11, 11));
                v
            })
            .collect();
        bullets.push(target);
        projectiles.update(11.0, 12, &bullets, &ground);
        assert_eq!(projectiles.len(), MAX_FLIGHTS);
        // Every shell stayed; the oldest bullets went.
        let kinds = |k: ProjectileKind| projectiles.flights.iter().filter(|f| f.kind == k).count();
        assert_eq!(kinds(ProjectileKind::Shell), 10);
        assert!(!projectiles.flights.iter().any(|f| f.shooter == 100));
        assert!(
            projectiles
                .flights
                .iter()
                .any(|f| f.shooter == 100 + MAX_FLIGHTS as u32 + 49)
        );
        // Fewer kept still: every small one goes before a shell, then the oldest shells.
        let mut fewer = Projectiles {
            flights: projectiles.flights.clone(),
            ..Projectiles::default()
        };
        fewer.cap(100);
        assert_eq!(fewer.len(), 100);
        assert_eq!(
            fewer
                .flights
                .iter()
                .filter(|f| f.kind == ProjectileKind::Shell)
                .count(),
            10
        );
        fewer.cap(5);
        assert!(
            fewer
                .flights
                .iter()
                .all(|f| f.kind == ProjectileKind::Shell)
        );
        assert_eq!(fewer.flights[0].shooter, 5);
    }

    #[test]
    fn each_kind_draws_something_in_flight_and_where_it_lands() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        for kind in ProjectileKind::ALL {
            let mut shooter = view(1, 10.0);
            shooter.shot = Some(shot(kind, 2, 100, 100));
            let views = [shooter, view(2, 20.0)];
            let mut projectiles = Projectiles::default();
            projectiles.update(100.0, 101, &views, &ground);
            let mut out = ProjectileGeometry::default();
            let flight = projectiles.flights[0];
            projectiles.build((flight.start + flight.end) / 2.0, &mut out);
            let drawn = out.glow.len() + out.lines.len() + out.models.len();
            assert!(drawn > 0, "{kind:?} in flight");
            match kind.look().body {
                Body::Model { .. } => assert_eq!(out.models.len(), 1, "{kind:?}"),
                _ => assert!(out.models.is_empty(), "{kind:?}"),
            }
            if kind.look().trail == Trail::Smoke {
                assert!(!out.smoke.is_empty(), "{kind:?} leaves smoke");
            }
            projectiles.build(flight.end + 1.0, &mut out);
            assert!(
                out.glow.len() + out.lines.len() + out.smoke.len() > 0,
                "{kind:?} where it lands"
            );
            assert!(out.models.is_empty());
        }
    }

    #[test]
    fn a_model_turns_along_its_way() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        let mut shooter = view(1, 10.0);
        shooter.shot = Some(shot(ProjectileKind::Shell, 2, 100, 100));
        let views = [shooter, view(2, 30.0)];
        let mut projectiles = Projectiles::default();
        projectiles.update(100.0, 101, &views, &ground);
        let f = projectiles.flights[0];
        let mut out = ProjectileGeometry::default();
        let at = |t: f64| f.start + (f.end - f.start) * t;
        projectiles.build(at(0.1), &mut out);
        let rising = out.models[0].forward;
        projectiles.build(at(0.9), &mut out);
        let falling = out.models[0].forward;
        // East along the line, nose up after launch and down before it lands.
        assert!(rising[0] > 0.5 && falling[0] > 0.5);
        assert!(rising[1] > 0.1 && falling[1] < -0.1);
    }

    #[test]
    fn beams_and_shots_to_orbit_are_straight() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        let mut ship = view(1, 10.0);
        ship.altitude = 8.0;
        ship.shot = Some(shot(ProjectileKind::Shell, 2, 100, 100));
        let views = [ship, view(2, 14.0)];
        let mut projectiles = Projectiles::default();
        projectiles.update(100.0, 101, &views, &ground);
        let f = projectiles.flights[0];
        assert!(f.aloft && f.peak() == 0.0);
        let mid = f.point(0.5);
        let line: [f32; 3] = std::array::from_fn(|i| (f.from[i] + f.to[i]) / 2.0);
        assert!(close(mid, line));
    }

    #[test]
    fn projectile_models_point_forward_and_are_about_a_tile_long() {
        let looks = projectile_looks();
        assert_eq!(looks.len(), 7);
        for kind in ProjectileKind::ALL {
            if let Body::Model { index, .. } = kind.look().body {
                assert!(usize::from(index) < looks.len(), "{kind:?}");
            }
        }
        for look in &looks {
            let baked = crate::models::bake_model(&look.spec);
            assert!(
                baked.warnings.is_empty(),
                "{}: {:?}",
                look.id,
                baked.warnings
            );
            let [lo, hi] = baked.bounds();
            let long = hi[2] - lo[2];
            assert!((0.8..=1.1).contains(&long), "{} is {long} long", look.id);
            // A stone is round and a drone of a cloud (task 120) square.
            if look.id != "projectile.stone" && look.id != "projectile.drone" {
                // Longer than wide, nose ahead.
                assert!(hi[0] - lo[0] < long * 0.6, "{}", look.id);
                assert!(hi[2] > 0.3 && lo[2] < -0.3, "{}", look.id);
            }
            assert!(baked.near.triangles() <= 160, "{}", look.id);
        }
    }

    /// Task 128: a grid draws one quadrotor per drone in its slots, the drone on its way where it
    /// flies, in its owner's colour, the same at the same moment; a dive starts at the slot its
    /// drone left.
    #[test]
    fn a_grid_draws_its_drones_in_their_slots() {
        let flat = |_: f32, _: f32| 0.0;
        let ground = FlatGround(&flat);
        let mut operator = view(3, 4.0);
        let cloud = crate::CloudView {
            face: 0,
            x: 9.0,
            y: 10.0,
            facing_deg: 0.0,
            side: 4,
            spacing: 0.6,
            hover: 1.5,
            ready: 0b1011,
            left: 0,
            joining: Some((2, 0, 5.0, 10.0)),
        };
        operator.cloud = Some(cloud);
        let views = [view(1, 2.0), operator];
        let mut out = ProjectileGeometry::default();
        drone_grids(&views, 120.0, &ground, &mut out);
        assert_eq!(out.models.len(), 4);
        let team = crate::entities::owner_color(operator.owner);
        for drone in &out.models {
            assert_eq!(drone.model, DRONE_MODEL);
            assert_eq!(drone.team, team);
            assert!((drone.origin[1] - 1.5).abs() <= DRONE_BOB + 1e-4);
        }
        // Slot 0 is the front row's left: ahead (east) 0.9 and to the left (north) 0.9.
        assert!((out.models[0].origin[0] - 9.9).abs() < 1e-3);
        assert!((out.models[0].origin[2] - 9.1).abs() < 1e-3);
        // The joining drone flies from where it is toward its slot.
        assert!((out.models[3].origin[0] - 5.0).abs() < 1e-3);
        let mut again = ProjectileGeometry::default();
        drone_grids(&views, 120.0, &ground, &mut again);
        assert_eq!(out.models, again.models);
        // A drone that left slot 3 dives from there.
        let left = crate::CloudView {
            ready: 0b0011,
            left: 0b1000,
            ..cloud
        };
        let from = launch_point(&left, &ground, [0.0, 0.0, 0.0]);
        assert!((from[0] - 9.9).abs() < 1e-3 && (from[2] - 10.9).abs() < 1e-3);
    }
}

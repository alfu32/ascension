//! The universe as the renderer needs it: plain data for the star map and the star systems, and
//! the layouts that place them in the system frame and the star-map frame. Nothing here touches
//! raylib.
//!
//! The system frame has the star at the origin and the orbit plane at `y = 0`; the whole system
//! fits a disc of [`SYSTEM_RADIUS`] units. The star-map frame spreads the systems over a disc of
//! [`MAP_RADIUS`] units. Orbit radii keep their proportions (they grow geometrically); the star
//! and the planets are drawn much smaller than their orbits but larger than true scale, and the
//! renderer keeps every body at least a few pixels wide.

use std::f32::consts::TAU;

/// Radius in units of the disc that holds every orbit of one system.
pub const SYSTEM_RADIUS: f32 = 12_000.0;
/// Distance from the star to the innermost possible orbit ring, in system units.
pub const STAR_CLEAR: f32 = 1200.0;
/// Drawn radius of a star in system units.
pub const STAR_RADIUS: f32 = 600.0;
/// Drawn radius of the home planet in system units (a planet 600 units wide).
pub const HOME_PLANET_RADIUS: f32 = 300.0;
/// Drawn radius of the smallest habitable planet (a face of 96 tiles), in system units.
pub const SMALLEST_PLANET_RADIUS: f32 = HOME_PLANET_RADIUS * 96.0 / 192.0;
/// Drawn radius of a resource body of size 0, and what each step of size adds, in system units:
/// far larger than true, a third to a half of a median planet, always under the smallest one.
pub const ROCK_RADIUS: f32 = 85.0;
/// What each step of a resource body's size adds to its drawn radius, in system units.
pub const ROCK_RADIUS_STEP: f32 = 20.0;
/// Largest drawn radius of a resource body (size 3), in system units.
pub const LARGEST_ROCK_RADIUS: f32 = ROCK_RADIUS + 3.0 * ROCK_RADIUS_STEP;
/// Least drawn gap between two bodies, in system units.
pub const BODY_GAP: f32 = 60.0;
/// Radius in units of the disc that holds every system of the star map.
pub const MAP_RADIUS: f32 = 40_000.0;

/// What a body is, for drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyViewKind {
    /// The star at the centre of its system.
    Star,
    /// A habitable planet.
    Habitable,
    /// A resource body: a small rock or moon.
    Resource,
}

/// One star system on the star map.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemView {
    /// Id, unique within the scene.
    pub id: u32,
    /// Name shown next to the system on the star map.
    pub name: String,
    /// Star-map position in abstract units; the map is centred on 0, 0.
    pub x: i32,
    /// Star-map position in abstract units.
    pub y: i32,
    /// Colour of the star.
    pub star_colour: [u8; 3],
}

/// One body of a system.
#[derive(Clone, Debug, PartialEq)]
pub struct BodyView {
    /// Id, unique within the scene.
    pub id: u32,
    /// Name shown when the pointer rests on it.
    pub name: String,
    /// The system it belongs to.
    pub system: u32,
    /// Star, habitable planet or resource body.
    pub kind: BodyViewKind,
    /// Distance from the star in system units; 0 for the star.
    pub orbit_radius: i32,
    /// Position on the orbit in millidegrees.
    pub orbit_angle: i32,
    /// Cube face size of a habitable planet, or a size class 1..=3 of a resource body.
    pub n_or_size: u32,
    /// The colour of the race at home here, for home planets.
    pub home_race_colour: Option<[u8; 3]>,
}

/// The star map and its systems, published once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UniverseScene {
    /// Star systems.
    pub systems: Vec<SystemView>,
    /// Bodies of every system, in any order.
    pub bodies: Vec<BodyView>,
    /// Routes as pairs of system ids.
    pub routes: Vec<(u32, u32)>,
}

/// One body placed in its system frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedBody {
    /// Index into [`UniverseScene::bodies`].
    pub body: usize,
    /// Centre in the system frame.
    pub pos: [f32; 3],
    /// Drawn radius in system units.
    pub radius: f32,
    /// Radius of its orbit ring in system units, about the star or, for a moon, its planet.
    pub orbit: f32,
    /// The planet (an index into [`UniverseScene::bodies`]) a resource body circles as a moon.
    pub moon_of: Option<usize>,
}

/// Largest angle in millidegrees between a moon and its planet on their shared orbit.
pub const MOON_LINK_MDEG: i32 = 1100;
/// Largest radial gap between a moon and its planet, as a share of the system's outermost orbit
/// in percent.
pub const MOON_LINK_PCT: f32 = 15.0;

impl UniverseScene {
    /// True when ids are unique and every body and route names a system of the scene.
    pub fn is_valid(&self) -> bool {
        let mut systems: Vec<u32> = self.systems.iter().map(|s| s.id).collect();
        systems.sort_unstable();
        if systems.windows(2).any(|w| w[0] == w[1]) {
            return false;
        }
        let known = |id: u32| systems.binary_search(&id).is_ok();
        let mut bodies: Vec<u32> = self.bodies.iter().map(|b| b.id).collect();
        bodies.sort_unstable();
        !bodies.windows(2).any(|w| w[0] == w[1])
            && self.bodies.iter().all(|b| known(b.system))
            && self.routes.iter().all(|&(a, b)| known(a) && known(b))
    }

    /// Index of the system with `id`.
    pub fn system_index(&self, id: u32) -> Option<usize> {
        self.systems.iter().position(|s| s.id == id)
    }

    /// Index of the body with `id`.
    pub fn body_index(&self, id: u32) -> Option<usize> {
        self.bodies.iter().position(|b| b.id == id)
    }

    /// The bodies of `system` placed in its frame: orbit radii are compressed so the outermost
    /// orbit lies at [`SYSTEM_RADIUS`] and the innermost clears the star.
    pub fn place_system(&self, system: u32) -> Vec<PlacedBody> {
        let members = || {
            self.bodies
                .iter()
                .enumerate()
                .filter(move |(_, b)| b.system == system)
        };
        let outermost = self.system_outermost(system);
        let mut placed: Vec<PlacedBody> = members()
            .map(|(index, b)| {
                let (orbit, pos) = if b.kind == BodyViewKind::Star {
                    (0.0, [0.0; 3])
                } else {
                    let orbit = orbit_to_frame(b.orbit_radius.max(0) as f32, outermost);
                    // `orbit_angle` is in millidegrees.
                    let angle = (b.orbit_angle as f32 / 1000.0).to_radians().rem_euclid(TAU);
                    (orbit, [orbit * angle.cos(), 0.0, orbit * angle.sin()])
                };
                PlacedBody {
                    body: index,
                    pos,
                    radius: body_radius(b),
                    orbit,
                    moon_of: None,
                }
            })
            .collect();
        // A moon circles its planet at a small drawn distance instead of keeping the tiny gap
        // that its true orbit would give at this scale.
        let moons: Vec<(usize, usize)> = (0..placed.len())
            .filter_map(|i| Some((i, self.moon_parent(placed[i].body, outermost)?)))
            .collect();
        for (i, parent) in moons {
            let Some(at) = placed.iter().position(|p| p.body == parent) else {
                continue;
            };
            let rank = placed.iter().filter(|p| p.moon_of == Some(parent)).count();
            let (centre, planet) = (placed[at].pos, placed[at].radius);
            // Each further moon takes a ring wide enough to clear the one inside it.
            let step = (planet * 1.1).max(2.0 * LARGEST_ROCK_RADIUS + BODY_GAP);
            let orbit = planet * 2.6 + step * rank as f32 + placed[i].radius;
            let heading = (self.bodies[placed[i].body].id as f32 * 2.399_963).rem_euclid(TAU);
            placed[i].orbit = orbit;
            placed[i].moon_of = Some(parent);
            placed[i].pos = [
                centre[0] + orbit * heading.cos(),
                0.0,
                centre[2] + orbit * heading.sin(),
            ];
        }
        spread_rocks(&mut placed, |i| self.bodies[i].kind);
        placed
    }

    /// The habitable planet that the resource body at `index` circles as a moon: the generator
    /// puts a moon a few tiles off its planet's orbit radius and a fraction of a degree from its
    /// angle, which no other body does. `None` for anything that is not a moon.
    pub fn moon_parent(&self, index: usize, outermost: f32) -> Option<usize> {
        let body = self.bodies.get(index)?;
        if body.kind != BodyViewKind::Resource {
            return None;
        }
        let near = |a: i32, b: i32| {
            let d = (a - b).rem_euclid(360_000);
            d.min(360_000 - d)
        };
        self.bodies
            .iter()
            .enumerate()
            .filter(|(_, p)| p.system == body.system && p.kind == BodyViewKind::Habitable)
            .map(|(i, p)| {
                (
                    near(p.orbit_angle, body.orbit_angle),
                    (p.orbit_radius - body.orbit_radius).abs(),
                    i,
                )
            })
            .filter(|&(angle, gap, _)| {
                angle <= MOON_LINK_MDEG && gap as f32 <= outermost * MOON_LINK_PCT / 100.0
            })
            .min()
            .map(|(_, _, i)| i)
    }

    /// The largest orbit radius of `system` (at least 1), the scale of its frame layout.
    pub fn system_outermost(&self, system: u32) -> f32 {
        self.bodies
            .iter()
            .filter(|b| b.system == system)
            .map(|b| b.orbit_radius.max(0))
            .max()
            .unwrap_or(0)
            .max(1) as f32
    }

    /// Star-map positions of the systems, indexed like [`Self::systems`], scaled so the farthest
    /// system lies at [`MAP_RADIUS`] (a single system sits at the origin). The map's `y` runs
    /// along the frame's `z`.
    pub fn place_systems(&self) -> Vec<[f32; 3]> {
        let farthest = self
            .systems
            .iter()
            .map(|s| (s.x as f32).hypot(s.y as f32))
            .fold(0.0_f32, f32::max);
        let scale = if farthest > 0.0 {
            MAP_RADIUS / farthest
        } else {
            0.0
        };
        self.systems
            .iter()
            .map(|s| [s.x as f32 * scale, 0.0, s.y as f32 * scale])
            .collect()
    }
}

/// The distance from the star in the system frame of an orbit of radius `orbit` (the same units
/// as [`BodyView::orbit_radius`]) in a system whose outermost orbit is `outermost`.
pub fn orbit_to_frame(orbit: f32, outermost: f32) -> f32 {
    STAR_CLEAR + (SYSTEM_RADIUS - STAR_CLEAR) * (orbit / outermost.max(1.0))
}

/// The inverse of [`orbit_to_frame`]; points inside the clear zone of the star map to 0.
pub fn frame_to_orbit(frame: f32, outermost: f32) -> f32 {
    ((frame - STAR_CLEAR) / (SYSTEM_RADIUS - STAR_CLEAR)).max(0.0) * outermost.max(1.0)
}

/// Turns resource bodies along their rings (about the star, or a moon's about its planet) until
/// no two drawn bodies come closer than [`BODY_GAP`]: they are drawn far larger than true, so
/// neighbours on the belt or a moon and its planet's other moons could touch. Stars and planets
/// never move; the order of the bodies decides who gives way, so the layout is the same every
/// time.
fn spread_rocks(placed: &mut [PlacedBody], kind: impl Fn(usize) -> BodyViewKind) {
    let rock = |p: &PlacedBody| kind(p.body) == BodyViewKind::Resource;
    for _ in 0..64 {
        let mut moved = false;
        for i in 0..placed.len() {
            for j in 0..placed.len() {
                if i == j || !rock(&placed[i]) {
                    continue;
                }
                // A pair of rocks is settled once, by the first of them.
                if rock(&placed[j]) && j < i {
                    continue;
                }
                let (a, b) = (placed[i].pos, placed[j].pos);
                let apart = (a[0] - b[0]).hypot(a[2] - b[2]);
                let least = placed[i].radius + placed[j].radius + BODY_GAP;
                if apart >= least {
                    continue;
                }
                // Both rocks give way half each; a rock beside a star or planet all of it.
                let share = if rock(&placed[j]) { 0.5 } else { 1.0 };
                let push = (least - apart) * share + 1.0;
                let away = |p: &PlacedBody, from: [f32; 3]| -> f32 {
                    // Which way along its ring takes `p` away from `from`.
                    let centre = ring_centre(placed, p);
                    let (dx, dz) = (p.pos[0] - centre[0], p.pos[2] - centre[2]);
                    let along = [-dz, dx];
                    let sign = along[0] * (p.pos[0] - from[0]) + along[1] * (p.pos[2] - from[2]);
                    if sign >= 0.0 { 1.0 } else { -1.0 }
                };
                let turn_i = away(&placed[i], b);
                let turn_j = away(&placed[j], a);
                turn(placed, i, turn_i * push);
                if share < 1.0 {
                    turn(placed, j, turn_j * push);
                }
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }
}

/// The smallest gap between the drawn edges of two of `placed`, in system units (infinite for
/// fewer than two bodies).
pub fn closest_gap(placed: &[PlacedBody]) -> f32 {
    let mut least = f32::INFINITY;
    for (i, a) in placed.iter().enumerate() {
        for b in &placed[i + 1..] {
            let apart = (a.pos[0] - b.pos[0]).hypot(a.pos[2] - b.pos[2]);
            least = least.min(apart - a.radius - b.radius);
        }
    }
    least
}

/// The centre of the ring `p` is drawn on: its planet for a moon, the star otherwise.
fn ring_centre(placed: &[PlacedBody], p: &PlacedBody) -> [f32; 3] {
    p.moon_of
        .and_then(|parent| placed.iter().find(|q| q.body == parent))
        .map_or([0.0; 3], |q| q.pos)
}

/// Moves body `i` `by` units along its ring.
fn turn(placed: &mut [PlacedBody], i: usize, by: f32) {
    let centre = ring_centre(placed, &placed[i]);
    let p = &mut placed[i];
    let (dx, dz) = (p.pos[0] - centre[0], p.pos[2] - centre[2]);
    let radius = dx.hypot(dz).max(1.0);
    let angle = dz.atan2(dx) + by / radius;
    p.pos = [
        centre[0] + radius * angle.cos(),
        p.pos[1],
        centre[2] + radius * angle.sin(),
    ];
}

/// Drawn radius of a body in system units.
pub fn body_radius(body: &BodyView) -> f32 {
    match body.kind {
        BodyViewKind::Star => STAR_RADIUS,
        // A face of 192 tiles (the median) gives the home planet radius.
        BodyViewKind::Habitable => {
            HOME_PLANET_RADIUS * body.n_or_size.clamp(96, 288) as f32 / 192.0
        }
        BodyViewKind::Resource => {
            ROCK_RADIUS + ROCK_RADIUS_STEP * body.n_or_size.clamp(1, 3) as f32
        }
    }
}

const PLANET_PALETTE: [[u8; 3]; 6] = [
    [150, 170, 200],
    [200, 160, 120],
    [120, 170, 140],
    [190, 130, 120],
    [170, 150, 190],
    [210, 200, 160],
];
const ROCK_PALETTE: [[u8; 3]; 4] = [
    [150, 140, 130],
    [120, 118, 125],
    [170, 150, 120],
    [135, 125, 110],
];

/// The neutral colour of a planet that has no generated surface.
pub fn planet_colour(body_id: u32) -> [u8; 3] {
    PLANET_PALETTE[body_id as usize % PLANET_PALETTE.len()]
}

/// The colour of a resource body.
pub fn rock_colour(body_id: u32) -> [u8; 3] {
    ROCK_PALETTE[body_id as usize % ROCK_PALETTE.len()]
}

/// A unit sphere as triangles with an equirectangular texture mapping (north at the top, the
/// longitude of `+z` in the middle), so it matches the planet image of the minimap.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SphereMesh {
    /// Vertex positions on the unit sphere.
    pub positions: Vec<[f32; 3]>,
    /// Unit normals (equal to the positions).
    pub normals: Vec<[f32; 3]>,
    /// Texture coordinates.
    pub texcoords: Vec<[f32; 2]>,
    /// Triangle indices.
    pub indices: Vec<u16>,
}

/// A unit sphere of `rings` bands and `slices` segments: `2 * rings * slices` triangles.
pub fn uv_sphere(rings: u32, slices: u32) -> SphereMesh {
    let (rings, slices) = (rings.max(2), slices.max(3));
    let mut mesh = SphereMesh::default();
    for ring in 0..=rings {
        let lat = (0.5 - ring as f32 / rings as f32) * std::f32::consts::PI;
        for slice in 0..=slices {
            let lon = (slice as f32 / slices as f32 - 0.5) * TAU;
            let p = crate::orbit::dir_from_lon_lat(lon, lat);
            mesh.positions.push(p);
            mesh.normals.push(p);
            mesh.texcoords
                .push([slice as f32 / slices as f32, ring as f32 / rings as f32]);
        }
    }
    let stride = slices + 1;
    for ring in 0..rings {
        for slice in 0..slices {
            let a = ring * stride + slice;
            let b = a + stride;
            mesh.indices
                .extend([a, b, a + 1, a + 1, b, b + 1].map(|i| i as u16));
        }
    }
    mesh
}

/// Points of a horizontal circle of `radius` about the origin in the plane `y = 0`, closed.
pub fn ring_points(radius: f32, segments: u32) -> Vec<[f32; 3]> {
    (0..=segments)
        .map(|i| {
            let a = i as f32 / segments as f32 * TAU;
            [radius * a.cos(), 0.0, radius * a.sin()]
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn sample_scene() -> UniverseScene {
    let system = |id, x, y| SystemView {
        id,
        name: format!("S{id}"),
        x,
        y,
        star_colour: [255, 240, 200],
    };
    let body = |id, system, kind, orbit_radius, orbit_angle, n_or_size| BodyView {
        id,
        name: format!("B{id}"),
        system,
        kind,
        orbit_radius,
        orbit_angle,
        n_or_size,
        home_race_colour: None,
    };
    use BodyViewKind::{Habitable, Resource, Star};
    UniverseScene {
        systems: vec![
            system(1, 0, 0),
            system(2, 90_000, -30_000),
            system(3, -50, 70),
        ],
        bodies: vec![
            body(1, 1, Star, 0, 0, 0),
            body(2, 1, Habitable, 1000, 0, 192),
            body(3, 1, Resource, 2000, 90_000, 2),
            body(4, 1, Resource, 7300, 200_000, 1),
            body(5, 2, Star, 0, 0, 0),
            body(6, 2, Habitable, 4000, 30_000, 140),
            body(7, 3, Star, 0, 0, 0),
            // A moon of body 2: a little outside its orbit, a fraction of a degree from it.
            body(8, 1, Resource, 1100, 600, 1),
        ],
        routes: vec![(1, 2), (1, 3)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_layout_fits_every_orbit() {
        let scene = sample_scene();
        assert!(scene.is_valid());
        for system in &scene.systems {
            let placed = scene.place_system(system.id);
            assert!(!placed.is_empty());
            for p in &placed {
                let kind = scene.bodies[p.body].kind;
                if p.moon_of.is_some() {
                    // A moon lies on a small ring about its planet.
                    let parent = placed.iter().find(|q| Some(q.body) == p.moon_of).unwrap();
                    let distance = (p.pos[0] - parent.pos[0]).hypot(p.pos[2] - parent.pos[2]);
                    assert!((distance - p.orbit).abs() < 0.5, "{p:?}");
                    assert!(p.orbit > parent.radius + p.radius, "{p:?}");
                    continue;
                }
                let distance = p.pos[0].hypot(p.pos[2]);
                assert!((distance - p.orbit).abs() < 0.5, "{p:?}");
                assert!(p.orbit <= SYSTEM_RADIUS + 0.5, "{p:?}");
                if kind != BodyViewKind::Star {
                    assert!(p.orbit >= STAR_CLEAR);
                    // A body never reaches the star.
                    assert!(p.orbit - p.radius > STAR_RADIUS);
                }
            }
        }
        // The outermost orbit of a system with bodies is exactly the system radius.
        let outer = scene
            .place_system(1)
            .iter()
            .map(|p| p.orbit)
            .fold(0.0, f32::max);
        assert!((outer - SYSTEM_RADIUS).abs() < 0.5);
        // Orbit order is kept.
        let orbits: Vec<f32> = scene
            .place_system(1)
            .iter()
            .filter(|p| p.moon_of.is_none())
            .map(|p| p.orbit)
            .collect();
        assert!(orbits.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn a_moon_is_told_by_its_place_next_to_its_planet() {
        let scene = sample_scene();
        let outermost = scene.system_outermost(1);
        let moon = scene.body_index(8).unwrap();
        assert_eq!(scene.moon_parent(moon, outermost), scene.body_index(2));
        // A belt body far from every planet's angle, a planet and a star are no moons.
        for id in [3, 4, 2, 1] {
            let index = scene.body_index(id).unwrap();
            assert_eq!(scene.moon_parent(index, outermost), None, "body {id}");
        }
        let placed = scene.place_system(1);
        let parent = placed.iter().find(|p| p.body == 1).unwrap();
        let at = placed.iter().find(|p| p.body == moon).unwrap();
        assert_eq!(at.moon_of, Some(1));
        // Drawn close to the planet, well inside the gap to the next orbit.
        assert!(at.orbit < parent.radius * 6.0);
    }

    #[test]
    fn the_star_and_planets_are_small_against_their_orbits() {
        // The system disc is many times wider than the star and a planet.
        const { assert!(SYSTEM_RADIUS / STAR_RADIUS >= 15.0) };
        const { assert!(SYSTEM_RADIUS / HOME_PLANET_RADIUS >= 30.0) };
    }

    #[test]
    fn star_map_layout_contains_every_system() {
        let scene = sample_scene();
        let positions = scene.place_systems();
        assert_eq!(positions.len(), scene.systems.len());
        for p in &positions {
            assert!(p[0].hypot(p[2]) <= MAP_RADIUS + 0.5, "{p:?}");
            assert_eq!(p[1], 0.0);
        }
        let farthest = positions
            .iter()
            .map(|p| p[0].hypot(p[2]))
            .fold(0.0, f32::max);
        assert!((farthest - MAP_RADIUS).abs() < 1.0);
        // Directions are kept: system 2 is east and north of the centre (map y toward +z).
        assert!(positions[1][0] > 0.0 && positions[1][2] < 0.0);
        let single = UniverseScene {
            systems: vec![scene.systems[0].clone()],
            ..UniverseScene::default()
        };
        assert_eq!(single.place_systems(), vec![[0.0; 3]]);
    }

    #[test]
    fn habitable_planets_are_a_few_hundred_units_wide() {
        let scene = sample_scene();
        let home = &scene.bodies[1];
        assert_eq!(body_radius(home) * 2.0, 2.0 * HOME_PLANET_RADIUS);
        assert!(body_radius(home) * 2.0 >= 200.0 && body_radius(home) * 2.0 <= 700.0);
        assert!(body_radius(&scene.bodies[2]) < body_radius(home));
    }

    #[test]
    fn resource_bodies_are_drawn_large_but_under_every_planet() {
        let scene = sample_scene();
        let planet = |n| BodyView {
            kind: BodyViewKind::Habitable,
            n_or_size: n,
            ..scene.bodies[1].clone()
        };
        let smallest = body_radius(&planet(1));
        assert_eq!(smallest, SMALLEST_PLANET_RADIUS);
        for size in 1..=3 {
            let rock = BodyView {
                kind: BodyViewKind::Resource,
                n_or_size: size,
                ..scene.bodies[2].clone()
            };
            let r = body_radius(&rock);
            // About three times the old 15 × (1 + size), a third to a half of a median planet.
            let old = 15.0 * (1 + size) as f32;
            assert!((old * 2.3..=old * 3.6).contains(&r), "size {size}: {r}");
            assert!(
                (HOME_PLANET_RADIUS / 3.0..=HOME_PLANET_RADIUS / 2.0).contains(&r),
                "{r}"
            );
            assert!(r < smallest && r <= LARGEST_ROCK_RADIUS, "{r}");
        }
    }

    #[test]
    fn crowded_rocks_and_moons_are_drawn_apart() {
        let mut scene = sample_scene();
        let rock = |id, orbit_radius, orbit_angle, size| BodyView {
            id,
            name: format!("B{id}"),
            system: 1,
            kind: BodyViewKind::Resource,
            orbit_radius,
            orbit_angle,
            n_or_size: size,
            home_race_colour: None,
        };
        // A tight belt: neighbours a degree apart on nearly the same ring.
        for k in 0..8 {
            scene
                .bodies
                .push(rock(100 + k, 3000 + k as i32, 120_000 + k as i32 * 1000, 3));
        }
        // Three more moons of the planet, on both sides of its orbit.
        scene.bodies.push(rock(200, 1103, 400, 3));
        scene.bodies.push(rock(201, 900, 359_500, 2));
        scene.bodies.push(rock(202, 1097, 800, 3));
        let placed = scene.place_system(1);
        assert!(
            closest_gap(&placed) >= BODY_GAP - 1.0,
            "{}",
            closest_gap(&placed)
        );
        let planet = scene.body_index(2).unwrap();
        let moons: Vec<&PlacedBody> = placed
            .iter()
            .filter(|p| p.moon_of == Some(planet))
            .collect();
        assert_eq!(moons.len(), 4);
        let at = placed.iter().find(|p| p.body == planet).unwrap();
        for moon in moons {
            // Still on its ring about the planet, clear of it.
            let d = (moon.pos[0] - at.pos[0]).hypot(moon.pos[2] - at.pos[2]);
            assert!((d - moon.orbit).abs() < 0.5, "{moon:?}");
            assert!(d - moon.radius - at.radius >= at.radius, "{moon:?}");
        }
        // Belt rocks keep their distance from the star.
        for p in placed
            .iter()
            .filter(|p| p.moon_of.is_none() && p.orbit > 0.0)
        {
            assert!((p.pos[0].hypot(p.pos[2]) - p.orbit).abs() < 0.5, "{p:?}");
        }
        // The same scene gives the same layout.
        assert_eq!(placed, scene.place_system(1));
    }

    #[test]
    fn sphere_mesh_is_a_closed_unit_sphere() {
        let mesh = uv_sphere(12, 24);
        assert_eq!(mesh.indices.len(), 12 * 24 * 6);
        assert_eq!(mesh.positions.len(), mesh.texcoords.len());
        for p in &mesh.positions {
            assert!((p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - 1.0).abs() < 1e-4);
        }
        assert!(
            mesh.indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
        );
        // The texture spans the image once, north at the top.
        let v: Vec<f32> = mesh.texcoords.iter().map(|t| t[1]).collect();
        assert!(v.iter().cloned().fold(f32::MAX, f32::min) == 0.0);
        let north = mesh
            .positions
            .iter()
            .zip(&mesh.texcoords)
            .find(|(_, t)| t[1] == 0.0)
            .map(|(p, _)| p[1]);
        assert_eq!(north, Some(1.0));
    }
}

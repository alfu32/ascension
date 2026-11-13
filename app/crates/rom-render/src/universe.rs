//! The universe as the renderer needs it: plain data for the star map and the star systems, and
//! the layouts that place them in the system frame and the star-map frame. Nothing here touches
//! raylib.
//!
//! The system frame has the star at the origin and the orbit plane at `y = 0`; the whole system
//! fits a disc of [`SYSTEM_RADIUS`] units. The star-map frame spreads the systems over a disc of
//! [`MAP_RADIUS`] units. Planets are drawn with a radius that is not to scale.

use std::f32::consts::TAU;

/// Radius in units of the disc that holds every orbit of one system.
pub const SYSTEM_RADIUS: f32 = 3000.0;
/// Distance from the star to the innermost possible orbit ring, in system units.
pub const STAR_CLEAR: f32 = 500.0;
/// Drawn radius of a star in system units.
pub const STAR_RADIUS: f32 = 350.0;
/// Drawn radius of the home planet in system units (a planet 600 units wide).
pub const HOME_PLANET_RADIUS: f32 = 300.0;
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
    /// Radius of its orbit ring in system units.
    pub orbit: f32,
}

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
        let outermost = members()
            .map(|(_, b)| b.orbit_radius.max(0))
            .max()
            .unwrap_or(0)
            .max(1) as f32;
        members()
            .map(|(index, b)| {
                let (orbit, pos) = if b.kind == BodyViewKind::Star {
                    (0.0, [0.0; 3])
                } else {
                    let orbit = STAR_CLEAR
                        + (SYSTEM_RADIUS - STAR_CLEAR) * (b.orbit_radius.max(0) as f32 / outermost);
                    // `orbit_angle` is in millidegrees.
                    let angle = (b.orbit_angle as f32 / 1000.0).to_radians().rem_euclid(TAU);
                    (orbit, [orbit * angle.cos(), 0.0, orbit * angle.sin()])
                };
                PlacedBody {
                    body: index,
                    pos,
                    radius: body_radius(b),
                    orbit,
                }
            })
            .collect()
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

/// Drawn radius of a body in system units.
pub fn body_radius(body: &BodyView) -> f32 {
    match body.kind {
        BodyViewKind::Star => STAR_RADIUS,
        // A face of 192 tiles (the median) gives the home planet radius.
        BodyViewKind::Habitable => {
            HOME_PLANET_RADIUS * body.n_or_size.clamp(96, 288) as f32 / 192.0
        }
        BodyViewKind::Resource => 40.0 * (1 + body.n_or_size.clamp(1, 3)) as f32,
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
                let distance = p.pos[0].hypot(p.pos[2]);
                assert!((distance - p.orbit).abs() < 0.5, "{p:?}");
                assert!(p.orbit <= SYSTEM_RADIUS + 0.5, "{p:?}");
                if scene.bodies[p.body].kind != BodyViewKind::Star {
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
        let orbits: Vec<f32> = scene.place_system(1).iter().map(|p| p.orbit).collect();
        assert!(orbits.windows(2).all(|w| w[0] <= w[1]));
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

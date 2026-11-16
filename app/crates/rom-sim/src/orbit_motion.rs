//! Geosynchronous orbit (task 68). A ship or craft in a planet's orbit holds over one ground
//! point; ordered to move, attack, patrol or land within that orbit, its ground point travels
//! along a great circle at the unit's own speed and it never descends on its way.
//!
//! Far from its goal a ship steers at a point a few tiles ahead on the great circle, found with
//! the integer cube-sphere mapping of `rom-geo`; near it, it heads straight there.

use crate::{EntityId, Pos, Topology, World};
use rom_geo::{CubeSurface, SPHERE_SCALE, SurfacePos};

/// How far ahead along a great circle a ship in orbit steers, in tiles.
const LOOKAHEAD_TILES: i128 = 6;

type V3 = [i128; 3];

fn dot(a: V3, b: V3) -> i128 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `v` scaled to the length of the unit sphere's points; `None` for a zero vector.
fn unit(v: V3) -> Option<V3> {
    let length = (dot(v, v) as u128).isqrt() as i128;
    (length > 0).then(|| v.map(|c| c * i128::from(SPHERE_SCALE) / length))
}

impl World {
    /// The cube of a planet's surface, if it is a sphere.
    fn cube(&self, body: crate::BodyId) -> Option<CubeSurface> {
        match self.surface(body)?.geo.topology {
            Topology::Cube(cube) => Some(cube),
            Topology::Flat { .. } => None,
        }
    }

    fn sphere_of(cube: CubeSurface, pos: Pos) -> V3 {
        cube.sphere_point(SurfacePos {
            face: pos.face,
            x: i64::from(pos.x),
            y: i64::from(pos.y),
        })
        .map(i128::from)
    }

    /// The surface position of a direction from the planet's centre, to a sixteenth of a tile.
    fn surface_at(cube: CubeSurface, body: crate::BodyId, point: V3) -> Pos {
        let largest = point.iter().map(|c| c.abs()).max().unwrap_or(1).max(1);
        let scaled = point.map(|c| (c * i128::from(SPHERE_SCALE) / largest) as i32);
        let k = (4096 / cube.n).clamp(1, 16);
        let fine = CubeSurface { n: cube.n * k }.tile_at_point(scaled);
        let at = |v: u32| ((i64::from(v) * 1000 + 500) / i64::from(k)) as i32;
        Pos {
            x: at(fine.x),
            y: at(fine.y),
            face: fine.face,
            body,
        }
    }

    /// The point `LOOKAHEAD_TILES` from `p` in the direction `tangent`.
    fn ahead(cube: CubeSurface, body: crate::BodyId, p: V3, tangent: V3) -> Pos {
        let Some(t) = unit(tangent) else {
            return Self::surface_at(cube, body, p);
        };
        // The angle of the lookahead: tiles over the radius, 4 n / 2π tiles.
        let angle_num = LOOKAHEAD_TILES * 6283;
        let angle_den = 4000 * i128::from(cube.n);
        let point = std::array::from_fn(|i| p[i] * angle_den + t[i] * angle_num);
        Self::surface_at(cube, body, point)
    }

    /// Speed in milli-tiles per second of an entity moving within its orbit.
    fn orbit_speed(&self, id: EntityId) -> i64 {
        let e = self.entity(id).expect("orbiting entity");
        match &self.rules.entity_types[&e.type_id].movement {
            Some(m) => self.effective_stat(id, "speed", m.speed).max(0),
            None => self.rules.constant("space.orbit_speed").unwrap_or(0),
        }
    }

    /// Moves an orbiting entity's ground point one tick toward `goal` on its planet, along the
    /// great circle between them; true once it is there.
    pub(crate) fn orbit_move(&mut self, id: EntityId, goal: Pos) -> bool {
        let Some(orbit) = self.orbit(id) else {
            return false;
        };
        let (body, pos) = (orbit.body, orbit.pos);
        if pos == goal {
            return true;
        }
        let steer = match self.cube(body) {
            Some(cube) if self.geo(body).vector(pos, goal).is_none() => {
                let (p, g) = (Self::sphere_of(cube, pos), Self::sphere_of(cube, goal));
                // The tangent at p toward g; opposite points go by the north.
                let along = std::array::from_fn(|i| g[i] * dot(p, p) - p[i] * dot(p, g));
                let tangent = if unit(along).is_some() {
                    along
                } else {
                    let north = cross(p, [0, i128::from(SPHERE_SCALE), 0]);
                    if unit(north).is_some() {
                        cross(north, p)
                    } else {
                        [i128::from(SPHERE_SCALE), 0, 0]
                    }
                };
                Self::ahead(cube, body, p, tangent)
            }
            _ => goal,
        };
        self.orbit_step(id, steer, steer == goal)
    }

    /// Moves an orbiting entity's ground point one tick along the great circle through `from`
    /// and `to`, in the direction from `from` to `to`, back onto that circle if it strayed.
    pub(crate) fn orbit_patrol(&mut self, id: EntityId, from: Pos, to: Pos) {
        let Some(orbit) = self.orbit(id) else {
            return;
        };
        let (body, pos) = (orbit.body, orbit.pos);
        let Some(cube) = self.cube(body) else {
            self.orbit_move(id, to);
            return;
        };
        let (a, b) = (Self::sphere_of(cube, from), Self::sphere_of(cube, to));
        let normal = cross(a, b);
        if unit(normal).is_none() {
            self.orbit_move(id, to);
            return;
        }
        let p = Self::sphere_of(cube, pos);
        // Back onto the circle's plane, then a step along it.
        let off = dot(p, normal);
        let square = dot(normal, normal);
        let on: V3 = std::array::from_fn(|i| p[i] - normal[i] * off / square);
        let on = unit(on).unwrap_or(p);
        let steer = Self::ahead(cube, body, on, cross(normal, on));
        self.orbit_step(id, steer, false);
    }

    /// One tick of straight movement of the ground point toward `target` (within half a face);
    /// with `stop` it ends exactly there and reports arrival.
    fn orbit_step(&mut self, id: EntityId, target: Pos, stop: bool) -> bool {
        let speed = self.orbit_speed(id);
        let orbit = self.orbit(id).expect("orbiting entity");
        let (body, pos, facing) = (orbit.body, orbit.pos, orbit.facing);
        let numerator = orbit.move_remainder + speed;
        let budget = numerator / self.tick_rate;
        let remainder = numerator % self.tick_rate;
        let geo = self.geo(body);
        let (dx, dy) = geo.vector(pos, target).unwrap_or((0, 0));
        let length =
            crate::integer_sqrt((i128::from(dx).pow(2) + i128::from(dy).pow(2)) as u128) as i64;
        let (new, facing, arrived) = if stop && length <= budget {
            (target, facing, true)
        } else if length == 0 {
            (pos, facing, false)
        } else {
            let step = budget.min(length);
            let (moved, turns) = geo.offset(pos, dx * step / length, dy * step / length);
            let heading = rom_geo::longitude_mdeg([dy as i32, 0, dx as i32])
                .div_euclid(1000)
                .rem_euclid(360) as u16;
            (moved, crate::geometry::turn_facing(heading, turns), false)
        };
        let orbit = self.space.orbits.get_mut(&id).expect("orbiting entity");
        orbit.pos = new;
        orbit.facing = facing;
        orbit.move_remainder = if arrived { 0 } else { remainder };
        if new != pos {
            self.refresh_viewer(id);
        }
        arrived
    }
}

impl World {
    /// The direction of a surface position from its planet's centre, scaled to
    /// `rom_geo::SPHERE_SCALE`; `None` off a sphere.
    pub fn sphere_point(&self, pos: Pos) -> Option<[i32; 3]> {
        let cube = self.cube(pos.body)?;
        Some(Self::sphere_of(cube, pos).map(|c| c as i32))
    }

    /// Whether `order` moves its executor within the orbit it holds: a move, attack-move or
    /// patrol to a point of the planet it orbits.
    pub(crate) fn orbit_order(&self, id: EntityId, order: &crate::Order) -> bool {
        matches!(order.kind.as_str(), "move" | "attack_move" | "patrol")
            && self.orbit(id).is_some_and(|orbit| {
                order
                    .target_pos
                    .is_some_and(|p| p.body == orbit.body && self.geo(orbit.body).contains(p))
            })
    }
}

//! Where the listener is (task 125): the planet camera as the sound of the game needs it. The
//! render thread publishes one each frame while a planet is in view; the simulation thread
//! places sounds with it and reads the ground under the view for the ambience.

use rom_geo::{CubeSurface, SurfacePos};

use crate::globe::{Globe, to_render};
use crate::orbit::{OrbitCamera, tangent_frame};
use crate::terrain::normalize;

/// Ground points sampled across the view on each side: [`EAR_GRID`]² samples.
pub const EAR_GRID: usize = 5;

/// The planet camera, for sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ear {
    /// The cube surface of the planet in view.
    pub surface: CubeSurface,
    /// Planet radius (sea level) in tiles.
    pub radius: f32,
    /// The eye, in tiles from the planet's centre (render axes).
    pub eye: [f32; 3],
    /// Unit vector to the right of the screen.
    pub right: [f32; 3],
    /// The point looked at, on the ground.
    pub target: SurfacePos,
    /// Height of the eye above sea level, in tiles.
    pub altitude: f32,
    /// Ground half-width shown across the screen at the point looked at, in tiles.
    pub half_width: f32,
    /// Height of the sun over the point looked at: the cosine of its angle from straight up
    /// (1 noon, 0 on the horizon, below 0 night); `None` without a sun.
    pub sun_height: Option<f32>,
    /// Ground points spread over the view, row by row, the centre in the middle.
    pub ground: [SurfacePos; EAR_GRID * EAR_GRID],
}

impl Ear {
    /// The ear of `orbit` over `globe`, with the direction toward the sun in the planet's frame.
    pub fn of(orbit: &OrbitCamera, globe: &Globe, toward_sun: Option<[f32; 3]>) -> Ear {
        let half_width = orbit.half_width_at(orbit.distance);
        let (north, east) = tangent_frame(orbit.dir);
        // Spread over at most a quarter of the globe each way.
        let reach = (half_width / orbit.radius.max(1.0)).min(1.0);
        let ground = std::array::from_fn(|i| {
            let step = |k: usize| (k as f32 / (EAR_GRID - 1) as f32 * 2.0 - 1.0) * reach;
            let (u, v) = (step(i % EAR_GRID), step(i / EAR_GRID));
            let dir = normalize([
                orbit.dir[0] + east[0] * u + north[0] * v,
                orbit.dir[1] + east[1] * u + north[1] * v,
                orbit.dir[2] + east[2] * u + north[2] * v,
            ]);
            globe.pos_of(dir)
        });
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        Ear {
            surface: globe.surface(),
            radius: orbit.radius,
            eye: orbit.position(),
            right: orbit.right(),
            target: globe.pos_of(orbit.dir),
            altitude: orbit.altitude(),
            half_width,
            sun_height: toward_sun.map(|sun| dot(orbit.dir, normalize(sun))),
            ground,
        }
    }

    /// A point of the planet in the ear's frame: `face`, `x`, `y` in milli-tiles, `height` in
    /// tiles above sea level.
    pub fn point(&self, face: u8, x: i64, y: i64, height: f32) -> [f32; 3] {
        let unit = self.surface.sphere_point(SurfacePos { face, x, y });
        let dir = normalize(to_render(unit.map(|c| c as f32)));
        dir.map(|c| c * (self.radius + height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_lie_on_the_sphere_and_the_target_below_the_eye() {
        let surface = CubeSurface { n: 64 };
        let ear = Ear {
            surface,
            radius: 40.0,
            eye: [0.0, 0.0, 60.0],
            right: [1.0, 0.0, 0.0],
            target: SurfacePos {
                face: 0,
                x: 32_000,
                y: 32_000,
            },
            altitude: 20.0,
            half_width: 10.0,
            sun_height: None,
            ground: [SurfacePos {
                face: 0,
                x: 0,
                y: 0,
            }; EAR_GRID * EAR_GRID],
        };
        let p = ear.point(0, 32_000, 32_000, 0.0);
        let length = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        assert!((length - 40.0).abs() < 1e-3);
        let q = ear.point(3, 1_000, 63_000, 2.0);
        let length = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
        assert!((length - 42.0).abs() < 1e-3);
    }
}

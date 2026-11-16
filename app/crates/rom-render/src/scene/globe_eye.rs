//! Where the globe is seen from: the planet camera itself, or the one camera of the continuous
//! zoom carried into the planet frame when the space camera decides the picture.

use super::*;
use crate::layers::FrameView;
use crate::terrain::{cross, normalize};

/// The eye the globe is drawn for, in the planet frame (tiles, the planet at the origin).
#[derive(Clone, Copy, Debug)]
pub(super) struct GlobeEye {
    /// Eye position.
    pub(super) eye: [f32; 3],
    /// Point looked at.
    pub(super) target: [f32; 3],
    /// Up vector.
    pub(super) up: [f32; 3],
    /// Distance that sets the level of detail: the planet camera's, else the eye's height.
    pub(super) distance: f32,
    /// The planet camera, when it is the camera (the shadow pass works from it).
    pub(super) orbit: Option<OrbitCamera>,
}

impl GlobeEye {
    /// The planet camera's eye.
    pub(super) fn of_orbit(orbit: &OrbitCamera) -> Self {
        Self {
            eye: orbit.position(),
            target: orbit.target(),
            up: orbit.view_up(),
            distance: orbit.distance,
            orbit: Some(*orbit),
        }
    }

    /// The one camera seen in the planet frame, for a planet of `radius` tiles.
    pub(super) fn of_view(view: &FrameView, radius: f32) -> Self {
        let height = (0..3)
            .map(|i| view.eye[i] * view.eye[i])
            .sum::<f32>()
            .sqrt()
            - radius;
        Self {
            eye: view.eye,
            target: view.target,
            up: view.up,
            distance: height.max(crate::orbit::MIN_DISTANCE),
            orbit: None,
        }
    }

    /// Height of the eye above the sea level of the planet camera's planet.
    pub(super) fn altitude(&self) -> f32 {
        match &self.orbit {
            Some(orbit) => orbit.altitude(),
            None => self.distance,
        }
    }

    /// The local vertical where the sky is seen: under the point looked at for the planet
    /// camera, above the eye otherwise.
    pub(super) fn local_up(&self) -> [f32; 3] {
        match &self.orbit {
            Some(orbit) => orbit.dir,
            None => normalize(self.eye),
        }
    }

    /// The view pyramid on a canvas of `aspect`, as [`CullView::of_orbit`] builds it.
    pub(super) fn cull(&self, aspect: f32, fov_deg: f32, slack: f32) -> CullView {
        if let Some(orbit) = &self.orbit {
            return CullView::of_orbit(orbit, aspect, fov_deg, slack);
        }
        let forward = normalize([0, 1, 2].map(|i| self.target[i] - self.eye[i]));
        let right = normalize(cross(forward, self.up));
        let up = cross(right, forward);
        let tan_y = (fov_deg.to_radians() / 2.0).tan() * slack;
        CullView {
            eye: self.eye,
            forward,
            right,
            up,
            tan_x: tan_y * aspect,
            tan_y,
        }
    }
}

/// What the globe's backdrop pass worked out for its world pass.
#[derive(Clone, Copy, Debug)]
pub(super) struct GlobeFrame {
    /// Direction toward the light.
    pub(super) toward: [f32; 3],
    /// The shadow map's uniforms, when one was drawn.
    pub(super) shadow: Option<shadow::ShadowUniforms>,
    /// How much of the atmosphere's halo shows.
    pub(super) halo: f32,
    /// The view pyramid.
    pub(super) cull: CullView,
    /// How much the stars show through the sky: 0 by day near the ground, 1 in space.
    pub(super) stars: f32,
}

/// Near and far clip distances for an eye at `eye` over a planet of `radius` with `relief`,
/// as [`OrbitCamera::clip_planes`] chooses them; far out the near plane follows the eye.
pub(super) fn eye_clip_planes(eye: [f32; 3], radius: f32, relief: f32) -> (f32, f32) {
    let eye_radius = (0..3).map(|i| eye[i] * eye[i]).sum::<f32>().sqrt();
    let altitude = (eye_radius - radius - relief).max(0.0);
    let near = (altitude * 0.3).max(0.1);
    let far = eye_radius + radius + 1.0;
    (near, far.max(near * 2.0))
}

/// Clears the depth buffer of the target being drawn, keeping its colours: the next layer
/// draws in its own depth range over what is there.
pub(super) fn clear_depth() {
    // SAFETY: the render thread has an active GL context inside a texture mode; the colour mask
    // keeps the clear to the depth buffer and is restored right after.
    unsafe {
        raylib::ffi::rlDrawRenderBatchActive();
        raylib::ffi::rlColorMask(false, false, false, false);
        raylib::ffi::rlClearScreenBuffers();
        raylib::ffi::rlColorMask(true, true, true, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_planet_camera_and_its_view_agree_beyond_its_range() {
        // Straight down from beyond the planet camera's range, both eyes give the same picture
        // and the same detail.
        let orbit = OrbitCamera::at(30.0, 20.0, 400.0, 10.0, 120.0, 360.0);
        let drawn = OrbitCamera {
            distance: 500.0,
            ..orbit
        };
        let a = GlobeEye::of_orbit(&drawn);
        let view = FrameView {
            eye: a.eye,
            target: a.target,
            up: a.up,
        };
        let b = GlobeEye::of_view(&view, drawn.radius);
        assert!((a.distance - b.distance).abs() < 1e-2);
        assert!((0..3).all(|i| (a.local_up()[i] - b.local_up()[i]).abs() < 1e-4));
        let (near, far) = drawn.clip_planes(RELIEF);
        let (n, f) = eye_clip_planes(b.eye, drawn.radius, RELIEF);
        assert!((near - n).abs() < 1e-3 && (far - f).abs() < 1e-3);
        let height = (0..3).map(|i| b.eye[i] * b.eye[i]).sum::<f32>().sqrt() - drawn.radius;
        assert!((height - drawn.altitude()).abs() < 1e-2);
        // The view pyramid of the carried view is the planet camera's.
        let (p, q) = (a.cull(1.6, 45.0, 1.5), b.cull(1.6, 45.0, 1.5));
        assert!((0..3).all(|i| (p.forward[i] - q.forward[i]).abs() < 1e-4));
        assert!((0..3).all(|i| (p.up[i] - q.up[i]).abs() < 1e-4));
    }
}

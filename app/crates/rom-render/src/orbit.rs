//! Orbit camera for sphere planets: pure maths, no window.
//!
//! The planet is centred on the origin with y up. The camera looks at a point on the sea-level
//! sphere (`dir * radius`); `heading` turns it around the local up and `distance` is the
//! eye-to-target distance. Zoom is logarithmic; the tilt eases from a low street-level angle
//! to straight down as the camera moves out, so at the largest distance it looks at the centre.

use crate::camera::{InputEvent, Key, PointerButton};
use crate::terrain::{cross, normalize};

/// Closest zoom: eye-to-target distance in tiles.
pub const MIN_DISTANCE: f32 = 8.0;
/// Angle from straight down at the closest zoom, in radians (about 70 degrees: toward the horizon).
pub const STREET_TILT: f32 = 1.22;
/// Distance of the default home view, in tiles.
pub const HOME_DISTANCE: f32 = 48.0;
/// Zoom factor per wheel step.
const ZOOM_STEP: f32 = 0.88;
/// Pan speed as a fraction of the camera distance per second.
const PAN_RATE: f32 = 0.4;
/// Pointer distance from the canvas edge, in physical pixels, that starts edge panning.
const EDGE_PIXELS: f32 = 8.0;
/// Radians of heading per physical pixel of middle-button drag.
const ROTATE_RATE: f32 = 0.005;

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    a.map(|c| c * s)
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Unit vector for a longitude and latitude in radians: lon 0, lat 0 is +z, lat is toward +y.
pub fn dir_from_lon_lat(lon: f32, lat: f32) -> [f32; 3] {
    let ring = lat.cos();
    [ring * lon.sin(), lat.sin(), ring * lon.cos()]
}

/// Longitude and latitude in radians of a direction.
pub fn lon_lat_of(dir: [f32; 3]) -> (f32, f32) {
    let d = normalize(dir);
    (d[0].atan2(d[2]), d[1].clamp(-1.0, 1.0).asin())
}

/// North and east tangent vectors at the unit direction `dir`; north points toward +y.
pub fn tangent_frame(dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let y = [0.0, 1.0, 0.0];
    let mut north = sub(y, scale(dir, dot(y, dir)));
    if dot(north, north) < 1e-8 {
        // At a pole every direction is north; keep the frame continuous with the -z axis.
        north = sub([0.0, 0.0, -1.0], scale(dir, -dir[2]));
    }
    let north = normalize(north);
    (north, cross(north, dir))
}

fn rotate_about(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    let k = cross(axis, v);
    let along = dot(axis, v) * (1.0 - cos);
    [0, 1, 2].map(|i| v[i] * cos + k[i] * sin + axis[i] * along)
}

/// An orbit camera around a planet of a given radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    /// Unit vector from the planet centre to the point the camera looks at.
    pub dir: [f32; 3],
    /// Heading in radians, clockwise from north around the local up.
    pub heading: f32,
    /// Eye-to-target distance in tiles.
    pub distance: f32,
    /// Planet radius in tiles.
    pub radius: f32,
    /// Largest allowed distance in tiles (a parameter; the world model limits it by age).
    pub max_distance: f32,
}

impl OrbitCamera {
    /// A camera over longitude/latitude 0, facing north, at the largest distance.
    pub fn new(radius: f32, max_distance: f32) -> Self {
        Self {
            dir: [0.0, 0.0, 1.0],
            heading: 0.0,
            distance: max_distance,
            radius,
            max_distance: max_distance.max(MIN_DISTANCE),
        }
    }

    /// The default largest distance for a planet: three radii.
    pub fn default_max_distance(radius: f32) -> f32 {
        3.0 * radius
    }

    /// A camera at the given longitude and latitude (degrees), distance and heading (degrees).
    pub fn at(
        lon_deg: f32,
        lat_deg: f32,
        distance: f32,
        heading_deg: f32,
        radius: f32,
        max_distance: f32,
    ) -> Self {
        let mut camera = Self::new(radius, max_distance);
        camera.dir = dir_from_lon_lat(lon_deg.to_radians(), lat_deg.to_radians());
        camera.heading = heading_deg.to_radians();
        camera.distance = distance;
        camera.clamp();
        camera
    }

    /// Distance clamped into `MIN_DISTANCE..=max_distance`, heading wrapped.
    fn clamp(&mut self) {
        self.dir = normalize(self.dir);
        self.distance = self.distance.clamp(MIN_DISTANCE, self.max_distance);
        self.heading = self.heading.rem_euclid(std::f32::consts::TAU);
    }

    /// Angle from straight down in radians: [`STREET_TILT`] at the closest zoom, 0 at the largest.
    pub fn tilt(&self) -> f32 {
        let span = (self.max_distance / MIN_DISTANCE).ln().max(1e-6);
        let f = ((self.distance / MIN_DISTANCE).ln() / span).clamp(0.0, 1.0);
        let eased = f * f * (3.0 - 2.0 * f);
        STREET_TILT * (1.0 - eased)
    }

    /// Unit tangent the camera faces, at the target.
    pub fn forward_tangent(&self) -> [f32; 3] {
        let (north, east) = tangent_frame(self.dir);
        let (sin, cos) = self.heading.sin_cos();
        add(scale(north, cos), scale(east, sin))
    }

    /// Unit tangent to the right of the view, at the target.
    pub fn right_tangent(&self) -> [f32; 3] {
        cross(self.forward_tangent(), self.dir)
    }

    /// The point on the sea-level sphere the camera looks at.
    pub fn target(&self) -> [f32; 3] {
        scale(self.dir, self.radius)
    }

    /// Eye position in world space.
    pub fn position(&self) -> [f32; 3] {
        let tilt = self.tilt();
        let offset = sub(
            scale(self.dir, tilt.cos()),
            scale(self.forward_tangent(), tilt.sin()),
        );
        add(self.target(), scale(offset, self.distance))
    }

    /// Height of the eye above the sea-level sphere.
    pub fn altitude(&self) -> f32 {
        let eye = self.position();
        dot(eye, eye).sqrt() - self.radius
    }

    /// Unit vector from the eye to the target.
    pub fn forward(&self) -> [f32; 3] {
        let tilt = self.tilt();
        normalize(add(
            scale(self.dir, -tilt.cos()),
            scale(self.forward_tangent(), tilt.sin()),
        ))
    }

    /// The camera's up vector: local up at the horizon, the heading direction looking straight down.
    pub fn view_up(&self) -> [f32; 3] {
        let tilt = self.tilt();
        add(
            scale(self.dir, tilt.sin()),
            scale(self.forward_tangent(), tilt.cos()),
        )
    }

    /// Zooms in for positive `steps`, out for negative ones.
    pub fn zoom(&mut self, steps: f32) {
        self.distance *= ZOOM_STEP.powf(steps);
        self.clamp();
    }

    /// Turns the heading by `delta` radians.
    pub fn rotate(&mut self, delta: f32) {
        self.heading += delta;
        self.clamp();
    }

    /// Moves the view along great circles by arc lengths (tiles): `right` and `forward` are
    /// relative to the heading. The heading is carried along the circle.
    pub fn pan(&mut self, right: f32, forward: f32) {
        let forward_tangent = self.forward_tangent();
        let motion = add(
            scale(forward_tangent, forward),
            scale(cross(forward_tangent, self.dir), right),
        );
        let arc = dot(motion, motion).sqrt();
        if arc < 1e-9 {
            return;
        }
        let axis = normalize(cross(self.dir, scale(motion, 1.0 / arc)));
        let angle = arc / self.radius;
        let dir = normalize(rotate_about(self.dir, axis, angle));
        let carried = rotate_about(forward_tangent, axis, angle);
        let (north, east) = tangent_frame(dir);
        self.dir = dir;
        self.heading = dot(carried, east).atan2(dot(carried, north));
        self.clamp();
    }

    /// Looks at another point of the surface, keeping zoom and heading.
    pub fn look_at(&mut self, dir: [f32; 3]) {
        let forward_tangent = self.forward_tangent();
        let new_dir = normalize(dir);
        // Carry the heading along the great circle between the two points.
        let axis = cross(self.dir, new_dir);
        if dot(axis, axis) > 1e-12 {
            let axis = normalize(axis);
            let angle = dot(self.dir, new_dir).clamp(-1.0, 1.0).acos();
            let carried = rotate_about(forward_tangent, axis, angle);
            let (north, east) = tangent_frame(new_dir);
            self.heading = dot(carried, east).atan2(dot(carried, north));
        }
        self.dir = new_dir;
        self.clamp();
    }

    /// The whole planet in view: the largest distance, straight down.
    pub fn frame_globe(&mut self) {
        self.distance = self.max_distance;
    }

    /// Near and far clip distances that keep depth precision at every zoom.
    pub fn clip_planes(&self, relief: f32) -> (f32, f32) {
        let eye = self.position();
        let eye_radius = dot(eye, eye).sqrt();
        let altitude = (eye_radius - self.radius - relief).max(0.0);
        let near = (altitude * 0.3).clamp(0.1, 1000.0);
        let far = eye_radius + self.radius + 1.0;
        (near, far.max(near * 2.0))
    }
}

/// Turns input events and elapsed time into orbit camera movement.
#[derive(Clone, Debug, Default)]
pub struct OrbitController {
    held: [bool; 4],
    pointer: Option<[f32; 2]>,
    rotating: bool,
    viewport: [f32; 2],
    home: Option<OrbitCamera>,
}

impl OrbitController {
    /// Sets the canvas size in physical pixels, used for edge panning.
    pub fn set_viewport(&mut self, width: u32, height: u32) {
        self.viewport = [width as f32, height as f32];
    }

    /// Sets the camera the reset key (End) goes back to.
    pub fn set_home(&mut self, home: OrbitCamera) {
        self.home = Some(home);
    }

    /// Applies one input event.
    pub fn apply(&mut self, camera: &mut OrbitCamera, event: InputEvent) {
        match event {
            InputEvent::PointerMoved { x, y } => {
                if self.rotating
                    && let Some([last_x, _]) = self.pointer
                {
                    camera.rotate(-(x - last_x) * ROTATE_RATE);
                }
                self.pointer = Some([x, y]);
            }
            InputEvent::PointerLeft => self.pointer = None,
            InputEvent::ButtonDown { button, x, y, .. } => {
                self.pointer = Some([x, y]);
                if button == PointerButton::Middle {
                    self.rotating = true;
                }
            }
            InputEvent::ButtonUp { button, x, y, .. } => {
                self.pointer = Some([x, y]);
                if button == PointerButton::Middle {
                    self.rotating = false;
                }
            }
            InputEvent::Wheel { steps } => camera.zoom(steps),
            InputEvent::KeyDown(Key::ResetCamera) => {
                if let Some(home) = self.home {
                    *camera = home;
                }
            }
            InputEvent::KeyDown(Key::FrameMap) => camera.frame_globe(),
            InputEvent::KeyDown(key) => {
                if let Some(index) = pan_index(key) {
                    self.held[index] = true;
                }
            }
            InputEvent::KeyUp(key) => {
                if let Some(index) = pan_index(key) {
                    self.held[index] = false;
                }
            }
            InputEvent::Hotkey { .. } | InputEvent::Escape => {}
            InputEvent::ReleaseAll => {
                self.held = [false; 4];
                self.rotating = false;
            }
        }
    }

    /// Moves the camera for held keys and edge panning over `seconds`.
    pub fn advance(&self, camera: &mut OrbitCamera, seconds: f32) {
        let [forward_key, back_key, left_key, right_key] = self.held.map(f32::from);
        let (edge_right, edge_forward) = self.edge_direction();
        let right = (right_key - left_key + edge_right).clamp(-1.0, 1.0);
        let forward = (forward_key - back_key + edge_forward).clamp(-1.0, 1.0);
        if right != 0.0 || forward != 0.0 {
            let step = camera.distance * PAN_RATE * seconds;
            camera.pan(right * step, forward * step);
        }
    }

    fn edge_direction(&self) -> (f32, f32) {
        let Some([x, y]) = self.pointer else {
            return (0.0, 0.0);
        };
        let [width, height] = self.viewport;
        if self.rotating || width <= 0.0 || height <= 0.0 {
            return (0.0, 0.0);
        }
        let axis = |position: f32, extent: f32| {
            if position <= EDGE_PIXELS {
                -1.0
            } else if position >= extent - EDGE_PIXELS {
                1.0
            } else {
                0.0
            }
        };
        (axis(x, width), -axis(y, height))
    }
}

fn pan_index(key: Key) -> Option<usize> {
    match key {
        Key::PanForward => Some(0),
        Key::PanBack => Some(1),
        Key::PanLeft => Some(2),
        Key::PanRight => Some(3),
        Key::ResetCamera | Key::FrameMap => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: f32 = 122.0;

    fn camera(distance: f32) -> OrbitCamera {
        OrbitCamera::at(30.0, 20.0, distance, 40.0, R, 3.0 * R)
    }

    fn length(v: [f32; 3]) -> f32 {
        dot(v, v).sqrt()
    }

    #[test]
    fn orbit_camera_zoom_is_monotonic_and_clamped() {
        let mut c = camera(100.0);
        let mut last = (c.distance, c.tilt());
        for _ in 0..80 {
            c.zoom(-0.5);
            assert!(c.distance >= last.0 && c.tilt() <= last.1 + 1e-6);
            last = (c.distance, c.tilt());
        }
        assert_eq!(c.distance, 3.0 * R);
        assert!(c.tilt().abs() < 1e-6);
        for _ in 0..200 {
            c.zoom(0.5);
            assert!(c.distance <= last.0 && c.tilt() >= last.1 - 1e-6);
            last = (c.distance, c.tilt());
        }
        assert_eq!(c.distance, MIN_DISTANCE);
        assert!((c.tilt() - STREET_TILT).abs() < 1e-6);
        // The zoom is logarithmic: equal steps change the distance by an equal factor.
        let mut c = camera(100.0);
        let before = c.distance;
        c.zoom(1.0);
        let first = c.distance / before;
        let before = c.distance;
        c.zoom(1.0);
        assert!((c.distance / before - first).abs() < 1e-5);
        // A smaller maximum distance is honoured.
        let mut small = OrbitCamera::at(0.0, 0.0, 1000.0, 0.0, R, 200.0);
        assert_eq!(small.distance, 200.0);
        small.zoom(-10.0);
        assert_eq!(small.distance, 200.0);
    }

    #[test]
    fn pan_moves_along_great_circles() {
        let mut c = camera(60.0);
        let start = c.dir;
        let heading = c.forward_tangent();
        let plane = normalize(cross(start, heading));
        for _ in 0..40 {
            c.pan(0.0, 6.0);
            assert!(dot(c.dir, plane).abs() < 1e-4, "left the great circle");
            assert!((length(c.dir) - 1.0).abs() < 1e-5);
        }
        // 240 tiles of arc on a radius of 122 tiles.
        let angle = dot(start, c.dir).clamp(-1.0, 1.0).acos();
        let expected = (240.0_f32 / R) % std::f32::consts::TAU;
        let expected = expected.min(std::f32::consts::TAU - expected);
        assert!((angle - expected).abs() < 1e-3, "{angle} vs {expected}");
        // Sideways motion follows a great circle too, and the heading stays continuous.
        let mut c = camera(60.0);
        let side = normalize(cross(c.dir, c.right_tangent()));
        for _ in 0..10 {
            c.pan(5.0, 0.0);
            assert!(dot(c.dir, side).abs() < 1e-4);
        }
        // Panning over a pole keeps the camera facing the same way on the sphere.
        let mut c = OrbitCamera::at(0.0, 85.0, 60.0, 0.0, R, 3.0 * R);
        let before = c.forward_tangent();
        c.pan(0.0, 15.0);
        assert!(c.dir[1] > 0.99 && dot(before, c.forward_tangent()) > 0.98);
    }

    #[test]
    fn globe_zoom_looks_at_the_centre() {
        let c = camera(f32::MAX);
        assert_eq!(c.distance, c.max_distance);
        let eye = c.position();
        assert!((length(eye) - (R + c.max_distance)).abs() < 0.01);
        let forward = c.forward();
        // The forward ray passes through the planet centre.
        let toward_centre = normalize(scale(eye, -1.0));
        assert!(dot(forward, toward_centre) > 0.9999);
        assert!(c.altitude() > 2.0 * R);
        // The view up is the heading direction, tangent to the sphere.
        assert!(dot(c.view_up(), c.dir).abs() < 1e-5);
    }

    #[test]
    fn street_zoom_looks_toward_the_horizon() {
        let c = camera(0.0);
        assert_eq!(c.distance, MIN_DISTANCE);
        let forward = c.forward();
        // Mostly sideways, a little downward, and toward the heading.
        let down = -dot(forward, c.dir);
        assert!(down > 0.0 && down < 0.5, "looking {down} downward");
        assert!(dot(forward, c.forward_tangent()) > 0.85);
        // The eye is a few tiles above the ground and the view up is close to local up.
        let altitude = c.altitude();
        assert!((1.0..6.0).contains(&altitude), "altitude {altitude}");
        assert!(dot(c.view_up(), c.dir) > 0.9);
    }

    #[test]
    fn controller_zooms_pans_and_resets() {
        let mut controller = OrbitController::default();
        let mut c = camera(60.0);
        controller.set_home(c);
        controller.set_viewport(800, 600);
        controller.apply(&mut c, InputEvent::Wheel { steps: -50.0 });
        assert_eq!(c.distance, c.max_distance);
        controller.apply(&mut c, InputEvent::KeyDown(Key::PanForward));
        let before = c.dir;
        controller.advance(&mut c, 0.1);
        assert!(dot(before, c.dir) < 0.9999);
        controller.apply(&mut c, InputEvent::KeyUp(Key::PanForward));
        controller.apply(&mut c, InputEvent::KeyDown(Key::ResetCamera));
        assert_eq!(c, camera(60.0));
        controller.apply(&mut c, InputEvent::KeyDown(Key::FrameMap));
        assert_eq!(c.distance, c.max_distance);
    }

    #[test]
    fn clip_planes_follow_the_altitude() {
        let near_far = |d: f32| camera(d).clip_planes(5.0);
        let (street_near, street_far) = near_far(0.0);
        let (globe_near, globe_far) = near_far(f32::MAX);
        assert!(street_near <= 1.0 && globe_near > 50.0);
        assert!(street_far > street_near && globe_far > globe_near);
        assert!(globe_far > 4.0 * R);
    }

    #[test]
    fn lon_lat_round_trip() {
        for (lon, lat) in [(0.3_f32, 0.2_f32), (-2.0, -1.0), (3.0, 1.2)] {
            let (l, a) = lon_lat_of(dir_from_lon_lat(lon, lat));
            assert!((l - lon).abs() < 1e-5 && (a - lat).abs() < 1e-5);
        }
    }
}

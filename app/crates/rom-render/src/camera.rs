//! Strategy-view camera and the input rules that move it.

/// Half the width of the proof map in tiles; the camera target stays inside it.
const MAP_HALF_EXTENT: f32 = 128.0;
/// Pointer distance from the canvas edge, in physical pixels, that starts edge panning.
const EDGE_PIXELS: f32 = 8.0;
/// Pan speed as a fraction of the camera distance per second.
const PAN_RATE: f32 = 1.0;
/// Zoom factor per wheel step.
const ZOOM_STEP: f32 = 0.88;
/// Radians of rotation per physical pixel of middle-button drag.
const ROTATE_RATE: f32 = 0.005;

/// A camera looking at a point on the ground plane (y = 0) from `distance` away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Point on the ground the camera looks at; `target[1]` is always 0.
    pub target: [f32; 3],
    /// Rotation around the vertical axis, in radians.
    pub yaw: f32,
    /// Angle above the ground plane, in radians.
    pub pitch: f32,
    /// Distance from the target to the eye, in tiles.
    pub distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            yaw: 45_f32.to_radians(),
            pitch: 50_f32.to_radians(),
            distance: 60.0,
        }
    }
}

impl Camera {
    /// Closest allowed zoom, in tiles.
    pub const MIN_DISTANCE: f32 = 8.0;
    /// Farthest allowed zoom, in tiles.
    pub const MAX_DISTANCE: f32 = 220.0;
    /// Flattest allowed view, in radians.
    pub const MIN_PITCH: f32 = 0.35;
    /// Steepest allowed view (almost straight down), in radians.
    pub const MAX_PITCH: f32 = 1.48;

    /// Eye position in world space.
    pub fn position(&self) -> [f32; 3] {
        let horizontal = self.distance * self.pitch.cos();
        [
            self.target[0] + horizontal * self.yaw.sin(),
            self.target[1] + self.distance * self.pitch.sin(),
            self.target[2] + horizontal * self.yaw.cos(),
        ]
    }

    /// Moves the target along the ground: `right` and `forward` are in tiles,
    /// relative to the direction the camera faces.
    pub fn pan(&mut self, right: f32, forward: f32) {
        let (sin, cos) = self.yaw.sin_cos();
        self.target[0] += right * cos - forward * sin;
        self.target[2] += -right * sin - forward * cos;
        self.clamp();
    }

    /// Zooms in for positive `steps`, out for negative ones.
    pub fn zoom(&mut self, steps: f32) {
        self.distance *= ZOOM_STEP.powf(steps);
        self.clamp();
    }

    /// Turns the camera around its target.
    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw = (self.yaw + yaw).rem_euclid(std::f32::consts::TAU);
        self.pitch += pitch;
        self.clamp();
    }

    fn clamp(&mut self) {
        self.distance = self.distance.clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
        self.pitch = self.pitch.clamp(Self::MIN_PITCH, Self::MAX_PITCH);
        self.target[0] = self.target[0].clamp(-MAP_HALF_EXTENT, MAP_HALF_EXTENT);
        self.target[1] = 0.0;
        self.target[2] = self.target[2].clamp(-MAP_HALF_EXTENT, MAP_HALF_EXTENT);
    }
}

/// Mouse buttons the world view reacts to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Wheel button; dragging with it rotates the camera.
    Middle,
}

/// Keys the camera reacts to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Move the view forward (W, up arrow).
    PanForward,
    /// Move the view back (S, down arrow).
    PanBack,
    /// Move the view left (A, left arrow).
    PanLeft,
    /// Move the view right (D, right arrow).
    PanRight,
    /// Restore the default camera (R).
    ResetCamera,
}

impl Key {
    /// Maps a key name as the UI reports it ("w", "up", "r", …) to a camera key.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "w" | "up" => Some(Self::PanForward),
            "s" | "down" => Some(Self::PanBack),
            "a" | "left" => Some(Self::PanLeft),
            "d" | "right" => Some(Self::PanRight),
            "r" => Some(Self::ResetCamera),
            _ => None,
        }
    }

    fn held_index(self) -> Option<usize> {
        match self {
            Self::PanForward => Some(0),
            Self::PanBack => Some(1),
            Self::PanLeft => Some(2),
            Self::PanRight => Some(3),
            Self::ResetCamera => None,
        }
    }
}

/// Input forwarded from the UI. Positions are physical pixels relative to the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    /// The pointer moved inside the canvas.
    PointerMoved {
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// The pointer left the canvas.
    PointerLeft,
    /// A button was pressed.
    ButtonDown {
        /// Which button.
        button: PointerButton,
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// A button was released.
    ButtonUp {
        /// Which button.
        button: PointerButton,
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// Wheel movement in steps; positive zooms in.
    Wheel {
        /// Number of wheel steps.
        steps: f32,
    },
    /// A key went down.
    KeyDown(Key),
    /// A key went up.
    KeyUp(Key),
    /// Forget every held key and button (for example when the window loses focus).
    ReleaseAll,
}

/// Turns input events and elapsed time into camera movement.
#[derive(Clone, Debug, Default)]
pub struct CameraController {
    held: [bool; 4],
    pointer: Option<[f32; 2]>,
    rotating: bool,
    viewport: [f32; 2],
}

impl CameraController {
    /// Sets the canvas size in physical pixels, used for edge panning.
    pub fn set_viewport(&mut self, width: u32, height: u32) {
        self.viewport = [width as f32, height as f32];
    }

    /// Applies one input event.
    pub fn apply(&mut self, camera: &mut Camera, event: InputEvent) {
        match event {
            InputEvent::PointerMoved { x, y } => {
                if self.rotating
                    && let Some([last_x, last_y]) = self.pointer
                {
                    camera.rotate(-(x - last_x) * ROTATE_RATE, (y - last_y) * ROTATE_RATE);
                }
                self.pointer = Some([x, y]);
            }
            InputEvent::PointerLeft => self.pointer = None,
            InputEvent::ButtonDown { button, x, y } => {
                self.pointer = Some([x, y]);
                if button == PointerButton::Middle {
                    self.rotating = true;
                }
            }
            InputEvent::ButtonUp { button, x, y } => {
                self.pointer = Some([x, y]);
                if button == PointerButton::Middle {
                    self.rotating = false;
                }
            }
            InputEvent::Wheel { steps } => camera.zoom(steps),
            InputEvent::KeyDown(Key::ResetCamera) => *camera = Camera::default(),
            InputEvent::KeyDown(key) => {
                if let Some(index) = key.held_index() {
                    self.held[index] = true;
                }
            }
            InputEvent::KeyUp(key) => {
                if let Some(index) = key.held_index() {
                    self.held[index] = false;
                }
            }
            InputEvent::ReleaseAll => {
                self.held = [false; 4];
                self.rotating = false;
            }
        }
    }

    /// Moves the camera for held keys and edge panning over `seconds`.
    pub fn advance(&self, camera: &mut Camera, seconds: f32) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn camera_zoom_is_clamped() {
        let mut camera = Camera::default();
        camera.zoom(1_000.0);
        assert_eq!(camera.distance, Camera::MIN_DISTANCE);
        camera.zoom(-1_000.0);
        assert_eq!(camera.distance, Camera::MAX_DISTANCE);
    }

    #[test]
    fn camera_pan_moves_target_on_ground_plane() {
        let mut camera = Camera {
            yaw: 0.0,
            ..Camera::default()
        };
        // Yaw 0 puts the eye on +z, so forward is -z and right is +x.
        camera.pan(0.0, 10.0);
        assert!(close(camera.target[0], 0.0) && close(camera.target[2], -10.0));
        camera.pan(5.0, 0.0);
        assert!(close(camera.target[0], 5.0) && close(camera.target[2], -10.0));
        assert_eq!(camera.target[1], 0.0);

        let mut turned = Camera {
            yaw: std::f32::consts::FRAC_PI_2,
            ..Camera::default()
        };
        turned.pan(0.0, 10.0);
        assert!(close(turned.target[0], -10.0) && close(turned.target[2], 0.0));
        assert_eq!(turned.target[1], 0.0);
    }

    #[test]
    fn held_key_and_screen_edge_pan_the_camera() {
        let mut camera = Camera {
            yaw: 0.0,
            ..Camera::default()
        };
        let mut controller = CameraController::default();
        controller.set_viewport(800, 600);
        controller.apply(&mut camera, InputEvent::KeyDown(Key::PanRight));
        controller.advance(&mut camera, 0.5);
        assert!(camera.target[0] > 0.0);
        controller.apply(&mut camera, InputEvent::KeyUp(Key::PanRight));
        controller.apply(&mut camera, InputEvent::PointerMoved { x: 400.0, y: 1.0 });
        let before = camera.target[2];
        controller.advance(&mut camera, 0.5);
        assert!(camera.target[2] < before, "top edge pans forward");
    }

    #[test]
    fn reset_key_restores_default_camera() {
        let mut camera = Camera::default();
        camera.zoom(3.0);
        camera.pan(4.0, 4.0);
        CameraController::default().apply(&mut camera, InputEvent::KeyDown(Key::ResetCamera));
        assert_eq!(camera, Camera::default());
    }
}

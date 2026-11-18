//! Strategy-view camera and the input rules that move it.

/// Ground half-width seen per unit of eye distance looking straight down.
const HALF_WIDTH_PER_DISTANCE: f32 = 0.6;

/// Half the width of the proof map in tiles; the default bounds of a camera.
const DEFAULT_HALF_EXTENT: f32 = 128.0;
/// Pointer distance from the canvas edge, in physical pixels, that starts edge panning.
const EDGE_PIXELS: f32 = 8.0;
/// Pan speed as a fraction of the camera distance per second.
const PAN_RATE: f32 = 1.0;
/// Zoom factor per wheel step.
const ZOOM_STEP: f32 = 0.88;
/// Radians of rotation per physical pixel of middle-button drag.
const ROTATE_RATE: f32 = 0.005;
/// Milliseconds after a wheel notch within which further notches belong to the same gesture
/// and are dropped: a misfiring or free-spinning wheel moves the camera one step.
pub const WHEEL_GESTURE_MS: u64 = 80;

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
    /// Area the target is kept in, as `[min_x, min_z, max_x, max_z]` in tiles.
    pub bounds: [f32; 4],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            yaw: 45_f32.to_radians(),
            pitch: 50_f32.to_radians(),
            distance: 60.0,
            bounds: [
                -DEFAULT_HALF_EXTENT,
                -DEFAULT_HALF_EXTENT,
                DEFAULT_HALF_EXTENT,
                DEFAULT_HALF_EXTENT,
            ],
        }
    }
}

impl Camera {
    /// Closest allowed zoom, in tiles.
    pub const MIN_DISTANCE: f32 = 8.0;
    /// Farthest allowed zoom, in tiles.
    pub const MAX_DISTANCE: f32 = 1500.0;
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

    /// Brings the distance down so that the ground half-width seen across the screen at the
    /// target stays within `half_width` tiles (`None`: no limit beyond [`Camera::MAX_DISTANCE`]).
    pub fn limit_half_width(&mut self, half_width: Option<f32>) {
        if let Some(width) = half_width {
            let limit = (width / HALF_WIDTH_PER_DISTANCE).max(Self::MIN_DISTANCE);
            self.distance = self.distance.min(limit);
        }
    }

    /// This camera with distance, pitch and target brought inside their limits.
    pub fn limited(mut self) -> Self {
        self.clamp();
        self
    }

    fn clamp(&mut self) {
        self.distance = self.distance.clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
        self.pitch = self.pitch.clamp(Self::MIN_PITCH, Self::MAX_PITCH);
        let [min_x, min_z, max_x, max_z] = self.bounds;
        self.target[0] = self.target[0].clamp(min_x, max_x.max(min_x));
        self.target[1] = 0.0;
        self.target[2] = self.target[2].clamp(min_z, max_z.max(min_z));
    }
}

/// How the player likes the camera to turn (settings, not game state).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TurnSettings {
    /// The middle button pans and Shift + middle turns, at every level; by default the middle
    /// button turns the view (on the planet it turns the globe under the camera, in space it
    /// orbits the focus) and Shift + middle does the other job.
    pub middle_pans: bool,
    /// Dragging right swings the camera right, the scene against the pointer.
    pub invert_horizontal: bool,
    /// Dragging up raises the camera, the scene against the pointer.
    pub invert_vertical: bool,
}

impl TurnSettings {
    /// A horizontal drag of `dx` pixels as the turn reads it.
    pub fn horizontal(&self, dx: f32) -> f32 {
        if self.invert_horizontal { -dx } else { dx }
    }

    /// A vertical drag of `dy` pixels as the turn reads it.
    pub fn vertical(&self, dy: f32) -> f32 {
        if self.invert_vertical { -dy } else { dy }
    }

    /// The event as the default mapping reads it: with [`TurnSettings::middle_pans`] the Shift
    /// of a middle-button press or release is swapped, so the two jobs trade buttons everywhere.
    pub fn map_event(&self, event: InputEvent) -> InputEvent {
        let swap = |mods: Modifiers| Modifiers {
            shift: !mods.shift,
            ..mods
        };
        match event {
            InputEvent::ButtonDown {
                button: PointerButton::Middle,
                x,
                y,
                mods,
            } if self.middle_pans => InputEvent::ButtonDown {
                button: PointerButton::Middle,
                x,
                y,
                mods: swap(mods),
            },
            InputEvent::ButtonUp {
                button: PointerButton::Middle,
                x,
                y,
                mods,
            } if self.middle_pans => InputEvent::ButtonUp {
                button: PointerButton::Middle,
                x,
                y,
                mods: swap(mods),
            },
            other => other,
        }
    }

    /// Packs the settings into bits, for sharing between threads.
    pub fn bits(&self) -> u8 {
        u8::from(self.middle_pans)
            | u8::from(self.invert_horizontal) << 1
            | u8::from(self.invert_vertical) << 2
    }

    /// The settings packed by [`TurnSettings::bits`].
    pub fn from_bits(bits: u8) -> Self {
        Self {
            middle_pans: bits & 1 != 0,
            invert_horizontal: bits & 2 != 0,
            invert_vertical: bits & 4 != 0,
        }
    }
}

/// Turns raw wheel movement into zoom steps: one step per gesture of a notched wheel, smooth
/// scrolling (fractional notches, as a touchpad gives) passed on as it comes.
#[derive(Clone, Copy, Debug, Default)]
pub struct WheelGate {
    last_ms: Option<u64>,
}

impl WheelGate {
    /// The zoom steps for `notches` of wheel movement at `now_ms` (positive zooms in). A whole
    /// number of notches is a notched wheel: the first event of a gesture gives one step and
    /// every event within [`WHEEL_GESTURE_MS`] of the one before belongs to the same gesture
    /// and gives none. Fractional notches give themselves.
    pub fn pass(&mut self, now_ms: u64, notches: f32) -> f32 {
        if notches == 0.0 || !notches.is_finite() {
            return 0.0;
        }
        if (notches - notches.round()).abs() > 1e-3 {
            return notches;
        }
        let same_gesture = self
            .last_ms
            .is_some_and(|last| now_ms.saturating_sub(last) <= WHEEL_GESTURE_MS);
        self.last_ms = Some(now_ms);
        if same_gesture { 0.0 } else { notches.signum() }
    }
}

/// Keyboard modifiers held during an event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Shift: add to the selection, queue orders.
    pub shift: bool,
    /// Control: store control groups, keep every unit type in a box.
    pub ctrl: bool,
    /// Alt.
    pub alt: bool,
}

/// Mouse buttons the world view reacts to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    /// Primary button.
    Left,
    /// Secondary button.
    Right,
    /// Wheel button; dragging pans, Shift-drag rotates the camera.
    Middle,
}

/// Keys the camera reacts to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Move the view forward (up arrow).
    PanForward,
    /// Move the view back (down arrow).
    PanBack,
    /// Move the view left (left arrow).
    PanLeft,
    /// Move the view right (right arrow).
    PanRight,
    /// Go back to the camera the scene started with (End).
    ResetCamera,
    /// Frame the whole map (Home).
    FrameMap,
    /// Fly the camera to the selection, at the level it is seen at (Space).
    FocusSelection,
    /// Keep the selection centred until the key is pressed again (Shift + Space).
    FollowSelection,
    /// Switch the "Sky from the Sun" preset of the star map (F2).
    SkyFromSun,
    /// Show or hide the constellation figures on the star map (F3).
    ToggleConstellations,
    /// Fly up the screen on the star map (Shift + up arrow); elsewhere as the up arrow.
    FlyUp,
    /// Fly down the screen on the star map (Shift + down arrow); elsewhere as the down arrow.
    FlyDown,
    /// Turn the star-map camera upright again (F4).
    LevelView,
    /// Fly to the celestial home at the galactic centre (F6, task 76), where the star map
    /// reaches that far.
    CelestialHome,
}

impl Key {
    /// Maps a key name as the UI reports it ("up", "end", …) to a camera key. Letters belong to
    /// the command palette, so only arrows, `end` and `home` move the camera.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "up" => Some(Self::PanForward),
            "down" => Some(Self::PanBack),
            "left" => Some(Self::PanLeft),
            "right" => Some(Self::PanRight),
            "end" => Some(Self::ResetCamera),
            "home" => Some(Self::FrameMap),
            "space" => Some(Self::FocusSelection),
            "f2" => Some(Self::SkyFromSun),
            "f3" => Some(Self::ToggleConstellations),
            "f4" => Some(Self::LevelView),
            "f6" => Some(Self::CelestialHome),
            "shift+up" => Some(Self::FlyUp),
            "shift+down" => Some(Self::FlyDown),
            _ => None,
        }
    }

    /// The camera key of a key press named `name` with Shift held or not: Shift + the up and
    /// down arrows fly up and down the screen on the star map, other keys ignore Shift.
    pub fn from_keystroke(name: &str, shift: bool) -> Option<Self> {
        match Self::from_name(name)? {
            Self::PanForward if shift => Some(Self::FlyUp),
            Self::PanBack if shift => Some(Self::FlyDown),
            key => Some(key),
        }
    }

    /// The other key a release of this one ends too: Shift may be let go before the arrow, so
    /// releasing an arrow also releases its Shift form, and the other way round.
    pub fn released_with(self) -> Option<Self> {
        match self {
            Self::PanForward => Some(Self::FlyUp),
            Self::PanBack => Some(Self::FlyDown),
            Self::FlyUp => Some(Self::PanForward),
            Self::FlyDown => Some(Self::PanBack),
            _ => None,
        }
    }

    fn held_index(self) -> Option<usize> {
        match self {
            Self::PanForward | Self::FlyUp => Some(0),
            Self::PanBack | Self::FlyDown => Some(1),
            Self::PanLeft => Some(2),
            Self::PanRight => Some(3),
            Self::ResetCamera
            | Self::FrameMap
            | Self::FocusSelection
            | Self::FollowSelection
            | Self::SkyFromSun
            | Self::ToggleConstellations
            | Self::LevelView
            | Self::CelestialHome => None,
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
        /// Modifiers held.
        mods: Modifiers,
    },
    /// A button was released.
    ButtonUp {
        /// Which button.
        button: PointerButton,
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
        /// Modifiers held.
        mods: Modifiers,
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
    /// A letter or digit key for the command palette, control groups or attack-move.
    Hotkey {
        /// The character, lower case.
        ch: char,
        /// Modifiers held.
        mods: Modifiers,
    },
    /// Escape was pressed.
    Escape,
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
    home: Option<Camera>,
    overview: Option<Camera>,
    turn: TurnSettings,
}

impl CameraController {
    /// Sets the player's turning preferences.
    pub fn set_turn(&mut self, turn: TurnSettings) {
        self.turn = turn;
    }

    /// Sets the canvas size in physical pixels, used for edge panning.
    pub fn set_viewport(&mut self, width: u32, height: u32) {
        self.viewport = [width as f32, height as f32];
    }

    /// Sets the cameras the reset and frame-map keys go to.
    pub fn set_scene_cameras(&mut self, home: Camera, overview: Camera) {
        self.home = Some(home);
        self.overview = Some(overview);
    }

    /// Applies one input event.
    pub fn apply(&mut self, camera: &mut Camera, event: InputEvent) {
        match event {
            InputEvent::PointerMoved { x, y } => {
                if self.rotating
                    && let Some([last_x, last_y]) = self.pointer
                {
                    // The camera swings about its target so the scene follows the pointer.
                    camera.rotate(
                        -self.turn.horizontal(x - last_x) * ROTATE_RATE,
                        self.turn.vertical(y - last_y) * ROTATE_RATE,
                    );
                }
                self.pointer = Some([x, y]);
            }
            InputEvent::PointerLeft => self.pointer = None,
            InputEvent::ButtonDown { button, x, y, mods } => {
                self.pointer = Some([x, y]);
                if button == PointerButton::Middle {
                    self.rotating = mods.shift;
                }
            }
            InputEvent::ButtonUp { button, x, y, .. } => {
                self.pointer = Some([x, y]);
                if button == PointerButton::Middle {
                    self.rotating = false;
                }
            }
            InputEvent::Wheel { steps } => camera.zoom(steps),
            InputEvent::KeyDown(Key::ResetCamera) => *camera = self.home.unwrap_or_default(),
            InputEvent::KeyDown(Key::FrameMap) => {
                if let Some(overview) = self.overview {
                    *camera = overview;
                }
            }
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
            InputEvent::Hotkey { .. } | InputEvent::Escape => {}
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
    fn the_star_map_presets_are_on_function_keys() {
        assert_eq!(Key::from_name("F2"), Some(Key::SkyFromSun));
        assert_eq!(Key::from_name("f3"), Some(Key::ToggleConstellations));
        assert_eq!(Key::from_name("f4"), Some(Key::LevelView));
    }

    #[test]
    fn shift_and_the_vertical_arrows_fly_up_and_down() {
        assert_eq!(Key::from_keystroke("up", true), Some(Key::FlyUp));
        assert_eq!(Key::from_keystroke("down", true), Some(Key::FlyDown));
        assert_eq!(Key::from_keystroke("up", false), Some(Key::PanForward));
        assert_eq!(Key::from_keystroke("left", true), Some(Key::PanLeft));
        assert_eq!(Key::from_name("shift+up"), Some(Key::FlyUp));
        // Releasing the arrow ends both forms, whichever was let go first.
        assert_eq!(Key::PanForward.released_with(), Some(Key::FlyUp));
        assert_eq!(Key::FlyDown.released_with(), Some(Key::PanBack));
        assert_eq!(Key::LevelView.released_with(), None);
        // Off the star map the Shift forms pan as the arrows do.
        let mut camera = Camera::default();
        let before = camera.target;
        let mut controller = CameraController::default();
        controller.apply(&mut camera, InputEvent::KeyDown(Key::FlyUp));
        controller.advance(&mut camera, 0.1);
        assert_ne!(camera.target, before);
    }

    #[test]
    fn rotation_requires_shift_middle_drag() {
        for shift in [false, true] {
            let mut camera = Camera::default();
            let before = camera;
            let mut controller = CameraController::default();
            controller.apply(
                &mut camera,
                InputEvent::ButtonDown {
                    button: PointerButton::Middle,
                    x: 400.0,
                    y: 300.0,
                    mods: Modifiers {
                        shift,
                        ..Modifiers::default()
                    },
                },
            );
            controller.apply(&mut camera, InputEvent::PointerMoved { x: 450.0, y: 320.0 });
            assert_eq!(camera != before, shift);
        }
    }

    #[test]
    fn a_burst_of_wheel_events_is_one_step() {
        let mut gate = WheelGate::default();
        // Six notches fired within a few milliseconds of each other (a misfire or a free spin).
        let burst: f32 = [0, 5, 12, 30, 70, 140]
            .iter()
            .map(|&t| gate.pass(1_000 + t, -1.0))
            .sum();
        assert_eq!(burst, -1.0);
        // The next gesture, after a pause, is a step of its own, whatever the wheel reported.
        assert_eq!(gate.pass(1_140 + WHEEL_GESTURE_MS + 1, 3.0), 1.0);
        assert_eq!(gate.pass(1_300 + WHEEL_GESTURE_MS, 1.0), 1.0);
        // Smooth scrolling (fractions of a notch) zooms continuously, never held back.
        let mut gate = WheelGate::default();
        let smooth: f32 = (0..10).map(|t| gate.pass(t, 0.25)).sum();
        assert!((smooth - 2.5).abs() < 1e-6);
        assert_eq!(gate.pass(20, 0.0), 0.0);
    }

    #[test]
    fn the_middle_drag_setting_swaps_the_two_jobs() {
        let press = |shift| InputEvent::ButtonDown {
            button: PointerButton::Middle,
            x: 1.0,
            y: 2.0,
            mods: Modifiers {
                shift,
                ..Modifiers::default()
            },
        };
        let default = TurnSettings::default();
        assert_eq!(default.map_event(press(false)), press(false));
        let pans = TurnSettings {
            middle_pans: true,
            ..default
        };
        assert_eq!(pans.map_event(press(false)), press(true));
        assert_eq!(pans.map_event(press(true)), press(false));
        // Other buttons and events pass unchanged.
        let left = InputEvent::ButtonDown {
            button: PointerButton::Left,
            x: 1.0,
            y: 2.0,
            mods: Modifiers::default(),
        };
        assert_eq!(pans.map_event(left), left);
        for bits in 0..8 {
            assert_eq!(TurnSettings::from_bits(bits).bits(), bits);
        }
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

    #[test]
    fn target_stays_inside_the_scene_bounds_and_home_key_frames_the_map() {
        let home = Camera {
            bounds: [0.0, 0.0, 40.0, 30.0],
            target: [10.0, 0.0, 10.0],
            ..Camera::default()
        };
        let overview = Camera {
            target: [20.0, 0.0, 15.0],
            distance: 90.0,
            ..home
        };
        let mut camera = home;
        camera.pan(1_000.0, -1_000.0);
        assert!(camera.target[0] <= 40.0 && camera.target[2] <= 30.0);
        camera.pan(-1_000.0, 1_000.0);
        assert!(camera.target[0] >= 0.0 && camera.target[2] >= 0.0);
        let mut controller = CameraController::default();
        controller.set_scene_cameras(home, overview);
        controller.apply(&mut camera, InputEvent::KeyDown(Key::FrameMap));
        assert_eq!(camera, overview);
        controller.apply(&mut camera, InputEvent::KeyDown(Key::ResetCamera));
        assert_eq!(camera, home);
    }
}

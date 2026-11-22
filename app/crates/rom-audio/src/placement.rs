//! Where a sound is heard: a gain and a stereo pan, computed by the caller.

/// How loud and where left–right a sound plays. Gain 1 is the file's own level; pan −1 is fully
/// left, 0 centred, 1 fully right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// Linear gain, 0 or more.
    pub gain: f32,
    /// −1 (left) … 1 (right).
    pub pan: f32,
}

impl Placement {
    /// Centred at the given gain (interface cues, voices of the selection).
    pub fn centre(gain: f32) -> Placement {
        Placement { gain, pan: 0.0 }
    }

    /// Clamped to valid values.
    pub fn clamped(self) -> Placement {
        Placement {
            gain: if self.gain.is_finite() {
                self.gain.max(0.0)
            } else {
                0.0
            },
            pan: if self.pan.is_finite() {
                self.pan.clamp(-1.0, 1.0)
            } else {
                0.0
            },
        }
    }

    /// Left and right gains with an equal-power pan law.
    pub fn channel_gains(self) -> (f32, f32) {
        let angle = (self.pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
        // At the centre both sides are at 1, so a centred mono sound keeps its level.
        let scale = self.gain * std::f32::consts::SQRT_2;
        (angle.cos() * scale, angle.sin() * scale)
    }
}

/// The ear in the world: turns a source position into a [`Placement`]. Positions are plain
/// vectors in any unit, as long as listener, source and distances use the same one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    /// Where the listener is.
    pub position: [f32; 3],
    /// A unit vector pointing to the listener's right (the screen's right for a camera).
    pub right: [f32; 3],
    /// Up to this distance a sound plays at full gain.
    pub ref_distance: f32,
    /// Beyond this distance a sound is not heard.
    pub max_distance: f32,
}

impl Listener {
    /// The placement of a sound at `source` with its own `gain`.
    pub fn place(&self, source: [f32; 3], gain: f32) -> Placement {
        let d = [
            source[0] - self.position[0],
            source[1] - self.position[1],
            source[2] - self.position[2],
        ];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if dist >= self.max_distance {
            return Placement {
                gain: 0.0,
                pan: 0.0,
            };
        }
        let near = self.ref_distance.max(1e-6);
        // Inverse distance beyond the reference, faded to silence at the maximum distance.
        let falloff = near / near.max(dist);
        let edge = 1.0 - (dist - near).max(0.0) / (self.max_distance - near).max(1e-6);
        let along = if dist > 1e-6 {
            (d[0] * self.right[0] + d[1] * self.right[1] + d[2] * self.right[2]) / dist
        } else {
            0.0
        };
        // Close sounds stay near the centre; far ones take their full side.
        let pan = along * (dist / near).min(1.0);
        Placement {
            gain: gain * falloff * edge.clamp(0.0, 1.0),
            pan,
        }
        .clamped()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listener() -> Listener {
        Listener {
            position: [0.0, 0.0, 0.0],
            right: [1.0, 0.0, 0.0],
            ref_distance: 10.0,
            max_distance: 100.0,
        }
    }

    #[test]
    fn near_sounds_are_full_and_far_ones_silent() {
        let l = listener();
        assert_eq!(l.place([0.0, 0.0, 5.0], 1.0).gain, 1.0);
        assert!(l.place([0.0, 0.0, 50.0], 1.0).gain < 0.2);
        assert_eq!(l.place([0.0, 0.0, 150.0], 1.0).gain, 0.0);
    }

    #[test]
    fn pan_follows_the_right_vector() {
        let l = listener();
        assert!(l.place([40.0, 0.0, 0.0], 1.0).pan > 0.99);
        assert!(l.place([-40.0, 0.0, 0.0], 1.0).pan < -0.99);
        assert_eq!(l.place([0.0, 0.0, 40.0], 1.0).pan, 0.0);
    }

    #[test]
    fn centred_pan_keeps_the_level() {
        let (left, right) = Placement::centre(1.0).channel_gains();
        assert!((left - 1.0).abs() < 1e-5 && (right - 1.0).abs() < 1e-5);
        let (left, right) = Placement {
            gain: 1.0,
            pan: 1.0,
        }
        .channel_gains();
        assert!(left.abs() < 1e-5 && right > 1.4);
    }
}

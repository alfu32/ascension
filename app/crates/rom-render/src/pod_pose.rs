//! Drop pods (task 174): how the pod model of `rom_kit::pods` is posed while it falls and after
//! it lands. Falling, it is drawn in place of the unit, white-hot shield down, turning slowly,
//! larger high up so it reads from the sky; landed, it stands in its crater, its petals fold
//! out, it cools, and after about eight seconds it sinks out of sight.

use rom_kit::pods::{DROP_POD_STAGES, stage_of};

/// Seconds a landed pod stays, the last [`SINK_SECS`] of them sinking into the ground.
pub(crate) const POD_SECS: f32 = 8.0;
/// Seconds the landed pod's shield still glows before its petals open.
pub(crate) const HOT_SECS: f32 = 0.4;
/// Seconds the petals take to fold out.
pub(crate) const OPEN_SECS: f32 = 0.5;
/// Seconds the pod takes to sink out of sight at the end.
pub(crate) const SINK_SECS: f32 = 1.5;
/// Degrees a pod turns about its axis over its fall.
const FALL_SPIN: f32 = 360.0;

/// A falling pod's pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Falling {
    /// Its turn about the up axis, degrees clockwise from the ground frame's x axis; a whole
    /// number of quarter turns at the end of the fall, as the landed pod stands.
    pub yaw_deg: f32,
    /// How much larger than true it is drawn: twice at the top of its fall, true on landing.
    pub size: f32,
}

/// The pose of a pod `share` of the way down its fall (above 0, up to 1); `id` varies where
/// its turn starts.
pub(crate) fn falling(share: f32, id: u32) -> Falling {
    let t = share.clamp(0.0, 1.0);
    let start = (id.wrapping_mul(2_654_435_761) >> 24) as f32 / 256.0 * 90.0;
    Falling {
        yaw_deg: start * (1.0 - t) + FALL_SPIN * t,
        size: 2.0 - t * t,
    }
}

/// A landed pod's pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Landed {
    /// Which pod model it wears, in the order of `rom_kit::pods::drop_pods`: 0 the hot closed
    /// pod, then the opening stages.
    pub model: usize,
    /// The share of its height it has sunk into the ground at the end (0 to 1).
    pub sink: f32,
}

/// The pose of a pod `t` seconds after it struck; `None` once it has gone.
pub(crate) fn landed(t: f32) -> Option<Landed> {
    if !(0.0..POD_SECS).contains(&t) {
        return None;
    }
    let model = if t < HOT_SECS {
        0
    } else {
        1 + stage_of((t - HOT_SECS) / OPEN_SECS).min(DROP_POD_STAGES - 1)
    };
    let sink = ((t - (POD_SECS - SINK_SECS)) / SINK_SECS).clamp(0.0, 1.0);
    Some(Landed {
        model,
        sink: sink * sink,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_falling_pod_turns_and_shrinks_to_its_landing_pose() {
        let high = falling(0.01, 9);
        let low = falling(1.0, 9);
        assert!(high.size > 1.9 && (low.size - 1.0).abs() < 1e-6);
        assert!((low.yaw_deg % 90.0).abs() < 1e-3, "{}", low.yaw_deg);
        assert_ne!(falling(0.01, 9).yaw_deg, falling(0.01, 10).yaw_deg);
        assert!(falling(0.5, 9).yaw_deg > high.yaw_deg);
    }

    #[test]
    fn a_landed_pod_glows_opens_stays_and_sinks() {
        let model = |t: f32| landed(t).map(|l| l.model);
        assert_eq!(model(0.0), Some(0));
        assert_eq!(model(HOT_SECS), Some(1));
        let mid = model(HOT_SECS + OPEN_SECS / 2.0).unwrap();
        assert!(1 < mid && mid < DROP_POD_STAGES, "{mid}");
        assert_eq!(model(HOT_SECS + OPEN_SECS), Some(DROP_POD_STAGES));
        assert_eq!(model(5.0), Some(DROP_POD_STAGES));
        assert_eq!(landed(5.0).unwrap().sink, 0.0);
        assert!(landed(POD_SECS - 0.05).unwrap().sink > 0.9);
        assert_eq!(landed(POD_SECS), None);
    }
}

//! Drop pods (task 174): the capsule a unit falls from orbit in and lands with. Like the lift
//! engines of task 170 it is a kit model of its own, drawn by the renderer in place of the
//! falling unit and, after the impact, where it struck; no entity has it.
//!
//! A blunt heat shield at the bottom (white-hot while falling, scorched after), a squat capsule
//! of four hatch petals between corner ribs, a band in the owner's colour round the petals,
//! four stabiliser fins, and a cap with a ring of retro thrusters and the drogue-chute housing.
//! The petals hinge at their foot and fold out flat on landing while the cap is blown off,
//! leaving the soldiers standing free; the model comes in stages of that opening, closed to
//! flat.

use crate::material::{MaterialLook, Program};
use crate::models::{ModelLook, ModelSpec, PartSpec, Shape};
use std::sync::Arc;

/// The id of the closed pod, as it stands after landing.
pub const DROP_POD_ID: &str = "kit.drop_pod";
/// The id of the falling pod: closed, its heat shield white-hot.
pub const DROP_POD_HOT_ID: &str = "kit.drop_pod_hot";
/// The id of the pod with its petals open flat.
pub const DROP_POD_OPEN_ID: &str = "kit.drop_pod_open";
/// Stages of the petals opening, closed (0) to flat (the last) included.
pub const DROP_POD_STAGES: usize = 6;
/// Size factor the pod is drawn at: about 1.3 tiles tall, one tile across its cap.
pub const DROP_POD_SCALE: f32 = 0.8;
/// How deep the landed pod sits in its crater, in its own units: the heat shield's depth.
pub const DROP_POD_SINK: f32 = 0.2;

/// Half the side of the capsule: where the petals hinge.
const HALF: f32 = 0.4;
/// Height of the hinges above the shield's tip: the top of the heat shield.
const HINGE: f32 = 0.33;
/// Height of the petals and the corner ribs.
const WALL: f32 = 0.88;
/// Degrees a petal turns from upright to lying flat on the ground.
const OPEN_DEG: f32 = 100.0;

fn metal(colour: [f32; 3], alt: [f32; 3], scale: f32) -> MaterialLook {
    MaterialLook {
        program: Program::Metal,
        colour,
        colour_alt: alt,
        scale,
    }
}

/// The white armour of the space age (`mat.white_composite`).
fn composite() -> MaterialLook {
    MaterialLook {
        program: Program::Plaster,
        colour: [0.93, 0.94, 0.94],
        colour_alt: [0.87, 0.88, 0.88],
        scale: 1.6,
    }
}

/// Dark structural metal (`mat.graphite`).
fn graphite() -> MaterialLook {
    metal([0.30, 0.33, 0.36], [0.19, 0.21, 0.24], 0.35)
}

/// The heat shield: soot over rust-brown char, or white-hot while falling.
fn shield_look(hot: bool) -> MaterialLook {
    if hot {
        MaterialLook {
            program: Program::Emissive,
            colour: [1.0, 0.95, 0.8],
            colour_alt: [1.0, 0.62, 0.22],
            scale: 0.4,
        }
    } else {
        metal([0.15, 0.13, 0.12], [0.36, 0.22, 0.13], 0.25)
    }
}

/// The cap in its own frame, its foot at the origin: a rim in the owner's colour, a chamfered
/// lid, the ring of retro thrusters and the drogue-chute housing.
fn cap_assembly() -> ModelSpec {
    let mut band = PartSpec::new("band", Shape::Cylinder, composite())
        .knob("w", 1.2)
        .knob("h", 0.06)
        .knob("segments", 8.0)
        .knob("far", 0.0);
    band.team = true;
    ModelSpec {
        parts: vec![
            band,
            PartSpec::new("lid", Shape::Cylinder, composite())
                .knob("w", 1.2)
                .knob("h", 0.12)
                .knob("taper", 0.3)
                .knob("y", 0.06)
                .knob("segments", 8.0),
            PartSpec::new("thrusters", Shape::Torus, graphite())
                .knob("w", 0.74)
                .knob("t", 0.12)
                .knob("y", 0.16)
                .knob("segments", 12.0)
                .knob("sides", 4.0)
                .knob("far", 0.0),
            PartSpec::new("chute_housing", Shape::Cylinder, graphite())
                .knob("w", 0.42)
                .knob("h", 0.26)
                .knob("taper", 0.2)
                .knob("y", 0.16)
                .knob("segments", 8.0)
                .knob("far", 0.0),
            PartSpec::new("chute_lid", Shape::Dome, composite())
                .knob("w", 0.34)
                .knob("h", 0.1)
                .knob("y", 0.42)
                .knob("segments", 8.0)
                .knob("far", 0.0),
        ],
    }
}

/// One hatch petal in its own frame: hinged along x at the origin (the foot of its outer
/// face), rising up y, its outside towards +z; a white panel with two seams, the owner's band
/// across it and a dark lining inside.
fn petal() -> ModelSpec {
    let width = 2.0 * HALF - 0.18;
    let mut band = PartSpec::new("band", Shape::Box, composite())
        .knob("w", width + 0.01)
        .knob("d", 0.02)
        .knob("h", 0.17)
        .knob("y", 0.5)
        .knob("z", 0.006);
    band.team = true;
    // The same band inside, upwards once the petal lies open.
    let mut inner = PartSpec::new("inner_band", Shape::Box, composite())
        .knob("w", width - 0.1)
        .knob("d", 0.02)
        .knob("h", 0.17)
        .knob("y", 0.5)
        .knob("z", -0.095)
        .knob("far", 0.0);
    inner.team = true;
    ModelSpec {
        parts: vec![
            PartSpec::new("panel", Shape::Box, composite())
                .knob("w", width)
                .knob("d", 0.07)
                .knob("h", WALL)
                .knob("z", -0.035),
            band,
            PartSpec::new("seam", Shape::Box, graphite())
                .knob("w", 0.025)
                .knob("d", 0.016)
                .knob("h", WALL - 0.12)
                .knob("x", -width / 4.0)
                .knob("y", 0.06)
                .knob("z", 0.004)
                .knob("repeat", 2.0)
                .knob("dx", width / 2.0)
                .knob("far", 0.0),
            PartSpec::new("lining", Shape::Box, graphite())
                .knob("w", width - 0.1)
                .knob("d", 0.02)
                .knob("h", WALL - 0.14)
                .knob("y", 0.07)
                .knob("z", -0.08)
                .knob("far", 0.0),
            inner,
        ],
    }
}

/// The parts of a pod with its petals `open` of the way out (0 closed, 1 flat), its heat
/// shield white-hot when `hot`. The origin is the tip of the heat shield; y is the pod's axis.
fn pod_parts(open: f32, hot: bool) -> Vec<PartSpec> {
    let open = open.clamp(0.0, 1.0);
    let shield = shield_look(hot);
    let rim = if hot {
        MaterialLook {
            colour: [1.0, 0.7, 0.3],
            colour_alt: [1.0, 0.45, 0.12],
            ..shield
        }
    } else {
        metal([0.22, 0.2, 0.19], [0.12, 0.11, 0.1], 0.3)
    };
    let top = HINGE + WALL;
    let mut parts = vec![
        // Turned over: the wide end at the hinges, the blunt tip at the origin.
        PartSpec::new("shield", Shape::Cylinder, shield)
            .knob("w", 1.12)
            .knob("h", HINGE - 0.03)
            .knob("taper", 0.6)
            .knob("roll", 180.0)
            .knob("y", HINGE - 0.03)
            .knob("bottom", 0.0)
            .knob("segments", 12.0),
        PartSpec::new("shield_rim", Shape::Cylinder, rim)
            .knob("w", 1.16)
            .knob("h", 0.07)
            .knob("y", HINGE - 0.05)
            .knob("segments", 12.0)
            .knob("far", 0.0),
        PartSpec::new("floor", Shape::Cylinder, graphite())
            .knob("w", 0.84)
            .knob("h", 0.04)
            .knob("y", HINGE - 0.02)
            .knob("segments", 8.0)
            .knob("far", 0.0),
    ];
    for k in 0..4 {
        let yaw = 45.0 + 90.0 * k as f32;
        let (s, c) = yaw.to_radians().sin_cos();
        // The corner ribs hold the cap; they go with it when the pod opens.
        if open <= 0.0 {
            parts.push(
                PartSpec::new(&format!("rib_{k}"), Shape::Box, graphite())
                    .knob("w", 0.1)
                    .knob("d", 0.1)
                    .knob("h", WALL)
                    .knob("x", 0.36 * s.signum())
                    .knob("y", HINGE)
                    .knob("z", 0.36 * c.signum())
                    .knob("far", 0.0),
            );
        }
        // A short fin on each corner, its root on the rib.
        let reach = 0.72;
        parts.push(
            PartSpec::new(&format!("fin_{k}"), Shape::Box, graphite())
                .knob("w", 0.06)
                .knob("d", 0.3)
                .knob("h", 0.62)
                .knob("taper", 0.4)
                .knob("x", reach * s)
                .knob("y", 0.52)
                .knob("z", reach * c)
                .knob("yaw", yaw),
        );
    }
    // The cap is blown off as the petals open: it climbs away, tipping, and is gone once they
    // lie flat.
    if open < 1.0 {
        let mut cap = PartSpec::new("cap", Shape::Include, composite())
            .knob("x", 0.7 * open)
            .knob("y", top + 2.0 * open);
        if open > 0.0 {
            cap = cap.knob("roll", 35.0 * open);
        }
        cap.include = Some(Arc::new(cap_assembly()));
        parts.push(cap);
    }
    let spec = Arc::new(petal());
    for k in 0..4 {
        let yaw = 90.0 * k as f32;
        let (s, c) = yaw.to_radians().sin_cos();
        let mut hatch = PartSpec::new(&format!("petal_{k}"), Shape::Include, composite())
            .knob("x", HALF * s)
            .knob("y", HINGE)
            .knob("z", HALF * c)
            .knob("yaw", yaw);
        if open > 0.0 {
            hatch = hatch.knob("pitch", OPEN_DEG * open);
        }
        hatch.include = Some(spec.clone());
        parts.push(hatch);
    }
    parts
}

fn look(id: String, open: f32, hot: bool) -> ModelLook {
    ModelLook {
        id,
        spec: Arc::new(ModelSpec {
            parts: pod_parts(open, hot),
        }),
        scale: DROP_POD_SCALE,
    }
}

/// The share of the way open of stage `stage` (0 closed, the last flat).
pub fn stage_open(stage: usize) -> f32 {
    stage.min(DROP_POD_STAGES - 1) as f32 / (DROP_POD_STAGES - 1) as f32
}

/// The id of opening stage `stage`.
pub fn stage_id(stage: usize) -> String {
    match stage {
        0 => DROP_POD_ID.into(),
        s if s + 1 >= DROP_POD_STAGES => DROP_POD_OPEN_ID.into(),
        s => format!("{DROP_POD_ID}_{}", (stage_open(s) * 100.0).round()),
    }
}

/// The falling pod: closed, its heat shield white-hot.
pub fn drop_pod_hot() -> ModelLook {
    look(DROP_POD_HOT_ID.into(), 0.0, true)
}

/// The landed pod at opening stage `stage` (0 closed, `DROP_POD_STAGES - 1` flat), scorched.
pub fn drop_pod(stage: usize) -> ModelLook {
    look(stage_id(stage), stage_open(stage), false)
}

/// Every pod model in the order the renderer uploads them: the falling pod, then the opening
/// stages from closed to flat.
pub fn drop_pods() -> Vec<ModelLook> {
    std::iter::once(drop_pod_hot())
        .chain((0..DROP_POD_STAGES).map(drop_pod))
        .collect()
}

/// The stage the petals have reached `open` of the way out.
pub fn stage_of(open: f32) -> usize {
    let last = (DROP_POD_STAGES - 1) as f32;
    (open.clamp(0.0, 1.0) * last).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TEAM_BIT, bake_model};

    #[test]
    fn the_pod_stands_on_its_shield_and_keeps_to_its_budget() {
        for look in drop_pods() {
            let baked = bake_model(&look.spec);
            assert!(
                baked.warnings.is_empty(),
                "{}: {:?}",
                look.id,
                baked.warnings
            );
            let [lo, hi] = baked.bounds();
            assert!(
                lo[1].abs() < 1e-3,
                "{}: the shield's tip is the origin: {lo:?}",
                look.id
            );
            let tall = hi[1] * look.scale;
            if look.id == DROP_POD_ID || look.id == DROP_POD_HOT_ID {
                assert!((1.1..1.6).contains(&tall), "{}: {tall} tiles tall", look.id);
            } else {
                assert!(tall < 3.0, "{}: {tall} tiles tall", look.id);
            }
            let (near, far) = (baked.near.triangles(), baked.far.triangles());
            assert!(
                near <= 600 && far <= 150,
                "{}: {near} near, {far} far",
                look.id
            );
            assert!(far > 40, "{}: {far} far", look.id);
            let team = |m: &crate::ModelMesh| m.colors.iter().any(|c| c[3] & TEAM_BIT != 0);
            assert!(team(&baked.near) && team(&baked.far), "{}", look.id);
        }
    }

    #[test]
    fn the_petals_fold_out_flat_and_the_shield_glows_only_while_falling() {
        let bake = |look: ModelLook| bake_model(&look.spec);
        let closed = bake(drop_pod(0));
        let open = bake(drop_pod(DROP_POD_STAGES - 1));
        let [lo, hi] = closed.bounds();
        let [olo, ohi] = open.bounds();
        // Open, the petals reach out on all four sides beyond the fins' span and lie low.
        assert!(
            [olo[0], olo[2], -ohi[0], -ohi[2]]
                .iter()
                .all(|r| *r < -1.15),
            "open {olo:?} {ohi:?}, closed {lo:?} {hi:?}"
        );
        // Nothing stands over the soldiers once it is open: the cap is gone.
        assert!(ohi[1] < 1.2, "{ohi:?}");
        // The petals' tips come down near the ground the landed pod sinks into.
        let low_far = open
            .near
            .positions
            .iter()
            .filter(|p| p[0].abs().max(p[2].abs()) > 1.1)
            .map(|p| p[1])
            .fold(f32::MAX, f32::min);
        assert!(
            (DROP_POD_SINK - 0.2..DROP_POD_SINK + 0.1).contains(&low_far),
            "{low_far}"
        );
        // Stages run closed to open, each with its own id.
        let ids: Vec<String> = drop_pods().into_iter().map(|l| l.id).collect();
        let unique: std::collections::BTreeSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "{ids:?}");
        assert_eq!(stage_of(0.0), 0);
        assert_eq!(stage_of(1.0), DROP_POD_STAGES - 1);
        // Only the falling pod has self-lit faces.
        let lit = |m: &crate::ModelMesh| {
            m.colors
                .iter()
                .filter(|c| c[3] & !TEAM_BIT == Program::Emissive.id() as u8)
                .count()
        };
        assert!(lit(&bake(drop_pod_hot()).far) > 0);
        assert_eq!(lit(&closed.near), 0);
        assert_eq!(lit(&open.near), 0);
    }
}

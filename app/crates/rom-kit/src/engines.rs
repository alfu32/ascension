//! Lift engines (task 170): the parts a building in the air is drawn with under it. A locked
//! building model never changes; the engines are a model of their own, drawn below it.

use crate::material::{MaterialLook, Program};
use crate::models::{ModelLook, ModelSpec, PartSpec, Shape};
use std::sync::Arc;

/// The id of the lift engine model.
pub const LIFT_ENGINE_ID: &str = "kit.lift_engine";

/// How far below its origin the engine's nozzle ends, in its own tiles: the exhaust starts
/// there.
pub const LIFT_ENGINE_DEPTH: f32 = 1.45;

fn metal(colour: [f32; 3], alt: [f32; 3]) -> MaterialLook {
    MaterialLook {
        program: Program::Metal,
        colour,
        colour_alt: alt,
        scale: 0.5,
    }
}

/// One lift engine, a tile across, hanging below its origin (the underside of the building):
/// a strut, a nacelle with a band in its owner's colour, a flared nozzle and its glowing throat.
pub fn lift_engine() -> ModelLook {
    let hull = metal([0.58, 0.6, 0.63], [0.4, 0.41, 0.45]);
    let dark = metal([0.22, 0.22, 0.24], [0.14, 0.14, 0.15]);
    let glow = MaterialLook {
        program: Program::Emissive,
        colour: [1.0, 0.66, 0.3],
        colour_alt: [1.0, 0.86, 0.55],
        scale: 1.0,
    };
    let mut band = PartSpec::new("band", Shape::Cylinder, hull)
        .knob("w", 1.04)
        .knob("h", 0.14)
        .knob("y", -0.62)
        .knob("segments", 14.0);
    band.team = true;
    let parts = vec![
        PartSpec::new("strut", Shape::Box, dark)
            .knob("w", 0.42)
            .knob("d", 0.42)
            .knob("h", 0.32)
            .knob("y", -0.32),
        PartSpec::new("nacelle", Shape::Cylinder, hull)
            .knob("w", 1.0)
            .knob("h", 0.78)
            .knob("y", -1.1)
            .knob("segments", 14.0),
        band,
        PartSpec::new("nozzle", Shape::Cylinder, dark)
            .knob("w", 0.86)
            .knob("h", 0.35)
            .knob("taper", 0.25)
            .knob("y", -LIFT_ENGINE_DEPTH)
            .knob("segments", 14.0),
        PartSpec::new("throat", Shape::Cylinder, glow)
            .knob("w", 0.6)
            .knob("h", 0.04)
            .knob("y", -LIFT_ENGINE_DEPTH - 0.01)
            .knob("bottom", 1.0)
            .knob("segments", 14.0),
    ];
    ModelLook {
        id: LIFT_ENGINE_ID.into(),
        spec: Arc::new(ModelSpec { parts }),
        scale: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::bake_model;

    #[test]
    fn the_engine_hangs_below_its_origin() {
        let baked = bake_model(&lift_engine().spec);
        assert!(baked.warnings.is_empty(), "{:?}", baked.warnings);
        let [lo, hi] = baked.bounds();
        assert!(
            hi[1] <= 1e-4,
            "nothing above the building's underside: {hi:?}"
        );
        assert!((lo[1] + LIFT_ENGINE_DEPTH + 0.01).abs() < 0.02, "{lo:?}");
        assert!((hi[0] - 0.52).abs() < 0.02 && (lo[0] + 0.52).abs() < 0.02);
        assert!(baked.near.triangles() > 0 && baked.far.triangles() > 0);
    }
}

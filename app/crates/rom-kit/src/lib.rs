//! The part kit: composed models of buildings and units, built from named parts and baked
//! into near and far vertex data before play. Plain Rust with no graphics library, so the
//! renderer, the command-line tools and the model files share one bake and one fingerprint.

pub mod custom;
pub mod engines;
pub mod kit;
pub mod material;
pub mod models;
pub mod rig;

pub use material::{MaterialLook, Program};
pub use models::{
    BakedModel, ModelLook, ModelMesh, ModelSpec, PartInfo, PartSpec, Shape, bake_model,
};

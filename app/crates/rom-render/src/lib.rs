//! Render the game world and match interface.
//!
//! Every raylib object lives on the render thread started by [`RenderWorker`].
//! Other threads talk to it only through the cloneable [`RenderBridge`]: they
//! publish the layout, render scale, camera and input, and take the newest
//! [`Frame`] from a one-slot mailbox. The terrain is described by a plain
//! [`TerrainScene`], published once and meshed on the render thread.

mod camera;
mod mailbox;
mod pixels;
mod scene;
mod stats;
mod terrain;
mod worker;

pub use camera::{Camera, CameraController, InputEvent, Key, PointerButton};
pub use image;
pub use mailbox::{Frame, FrameMailbox, FramePixels};
pub use pixels::rgba_to_bgra_flipped;
pub use stats::RenderStats;
pub use terrain::{
    CHUNK_TILES, ChunkMesh, Shape, Star, TerrainScene, TileKind, build_sea_chunks,
    build_terrain_chunks, home_camera, overview_camera, star_field, star_triangles,
};
pub use worker::{
    MAX_RENDER_SCALE, MIN_RENDER_SCALE, RenderBridge, RenderWorker, ShutdownHandle, scaled_size,
};

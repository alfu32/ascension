//! Render the game world and match interface.
//!
//! Every raylib object lives on the render thread started by [`RenderWorker`].
//! Other threads talk to it only through the cloneable [`RenderBridge`]: they
//! publish the layout, render scale, camera and input, and take the newest
//! [`Frame`] from a one-slot mailbox. The terrain is described by a plain
//! [`TerrainScene`], published once and meshed on the render thread. Entities arrive as plain
//! [`EntitySnapshot`]s; the render thread keeps the last two and interpolates between them.

mod camera;
mod entities;
pub mod hud;
pub mod hud_atlas;
mod hud_gpu;
pub mod hud_layout;
pub mod hud_minimap;
pub mod hud_preview;
pub mod hud_text;
mod mailbox;
mod pixels;
mod scene;
mod stats;
mod terrain;
mod worker;

pub use camera::{Camera, CameraController, InputEvent, Key, PointerButton};
pub use entities::{
    EntitySnapshot, EntityView, OWNER_COLORS, SnapshotPair, Surface, build_instances, owner_color,
};
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

pub use hud::{HudResearch, HudResource, HudSelection, HudSnapshot};

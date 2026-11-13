//! Render the game world and match interface.
//!
//! Every raylib object lives on the render thread started by [`RenderWorker`].
//! Other threads talk to it only through the cloneable [`RenderBridge`]: they
//! publish the layout, render scale, camera and input, and take the newest
//! [`Frame`] from a one-slot mailbox. The terrain is described by a plain
//! [`TerrainScene`], published once and meshed on the render thread; a sphere planet is described by
//! a [`PlanetScene`], drawn as a globe with an [`OrbitCamera`]. Entities arrive as plain
//! [`EntitySnapshot`]s; the render thread keeps the last two and interpolates between them.

mod camera;
mod entities;
mod globe;
pub mod graticule;
pub mod hud;
pub mod hud_atlas;
mod hud_gpu;
pub mod hud_layout;
pub mod hud_minimap;
pub mod hud_model;
pub mod hud_preview;
pub mod hud_text;
mod mailbox;
mod orbit;
mod overlay;
pub mod pick;
mod pixels;
mod scene;
pub mod shadow;
pub mod sky;
pub mod space;
mod stats;
pub mod sun;
mod terrain;
pub mod universe;
pub mod vision;
mod worker;
pub mod zoom;

pub use camera::{Camera, CameraController, InputEvent, Key, Modifiers, PointerButton};
pub use entities::{
    EntitySnapshot, EntityView, FlatGround, Footing, GroundFrame, OWNER_COLORS, SnapshotPair,
    Surface, build_instances, build_instances_on, interpolate_on, owner_color,
};
pub use globe::{ChunkSpec, Globe, PlanetScene, SurfaceHit, planet_radius, to_render};
pub use image;
pub use mailbox::{Frame, FrameMailbox, FramePixels};
pub use orbit::{
    HOME_DISTANCE, MIN_DISTANCE, OrbitCamera, OrbitController, dir_from_lon_lat, lon_lat_of,
};
pub use overlay::{OverlayGeometry, build_overlays, build_overlays_on};
pub use pixels::rgba_to_bgra_flipped;
pub use rom_geo::SurfacePos;
pub use space::{ClaimView, ShipView, SpaceSnapshot, TransitView};
pub use stats::RenderStats;
pub use sun::{SunClock, SunSettings};
pub use terrain::{
    CHUNK_TILES, ChunkMesh, Shape, Star, TerrainScene, TileKind, build_sea_chunks,
    build_terrain_chunks, home_camera, overview_camera, star_field, star_triangles,
};
pub use vision::{EXPLORED, UNEXPLORED, VISIBLE, VisionMap};
pub use worker::{
    MAX_RENDER_SCALE, MIN_RENDER_SCALE, OrbitSettings, RenderBridge, RenderWorker, ShutdownHandle,
    UniverseSettings, scaled_size,
};
pub use zoom::Level;

pub use hud::{HudResearch, HudResource, HudSelection, HudSnapshot};
pub use hud_model::{
    HudButton, HudCard, HudMarker, HudPalette, HudPlacement, HudQueue, HudQueueItem, HudTile,
    MarkerKind, PaletteTab, SLOT_KEYS, slot_of_key,
};
pub use pick::{
    Ground, HudRegionKind, HudTarget, PickContext, PickController, PickEvent, View, flat_surface,
};

//! Render the game world and match interface.
//!
//! Every raylib object lives on the render thread started by [`RenderWorker`].
//! Other threads talk to it only through the cloneable [`RenderBridge`]: they
//! publish the layout, render scale, camera and input, and take the newest
//! [`Frame`] from a one-slot mailbox. The terrain is described by a plain
//! [`TerrainScene`], published once and meshed on the render thread; a sphere planet is described by
//! a [`PlanetScene`], drawn as a globe with an [`OrbitCamera`]. Entities arrive as plain
//! [`EntitySnapshot`]s; the render thread keeps the last two and interpolates between them.

mod bars;
mod camera;
mod capitals;
mod ear;
mod entities;
pub mod features;
pub mod galaxy;
mod globe;
mod gpu_census;
pub mod grass;
pub mod graticule;
pub mod hud;
pub mod hud_atlas;
mod hud_gpu;
pub mod hud_icons;
pub mod hud_layout;
pub mod hud_minimap;
pub mod hud_model;
pub mod hud_preview;
pub mod hud_text;
mod instancing;
pub mod layers;
pub mod looks;
mod mailbox;
mod materials;
mod orbit;
mod overlay;
pub mod pick;
mod pixels;
pub mod pointer;
pub mod projectiles;
mod readback;
mod response;
pub mod review;
mod scene;
pub mod shadow;
pub mod sky;
pub mod space;
mod stains;
mod stats;
pub mod sun;
mod terrain;
pub mod universe;
pub mod vision;
mod worker;
pub mod zoom;

pub use rom_kit::{kit, models, rig};

pub use camera::{
    Camera, CameraController, InputEvent, Key, Modifiers, PointerButton, TurnSettings, WheelGate,
};
pub use ear::{EAR_GRID, Ear};
pub use entities::{
    CloudView, EntitySnapshot, EntityView, FlatGround, Footing, GroundFrame, Hover, ModelRef,
    OWNER_COLORS, SnapshotPair, Surface, build_instances, build_instances_on, interpolate_on,
    owner_color,
};
pub use globe::{ChunkSpec, Globe, PlanetScene, SurfaceHit, planet_radius, to_render};
pub use gpu_census::GpuCensus;
pub use grass::Detail;
pub use image;
pub use looks::{
    Generator, MATERIAL_FAR, MATERIAL_NEAR, MaterialLook, ObjectLook, Program, SceneLooks,
    SnowLook, TileLook, lod_weight,
};
pub use mailbox::{Frame, FrameMailbox, FramePixels};
pub use models::{
    BakedModel, ModelLook, ModelMesh, ModelSpec, PartInfo, PartSpec, Shape as PartShape, bake_model,
};
pub use orbit::{
    HOME_DISTANCE, MIN_DISTANCE, OrbitCamera, OrbitController, dir_from_lon_lat, lon_lat_of,
};
pub use overlay::{Line3, OverlayGeometry, build_overlays, build_overlays_on};
pub use pixels::rgba_to_bgra_flipped;
pub use pointer::{HudFlash, HudPointer, OrderBadge, PICK_SLACK, Side};
pub use projectiles::{INTERCEPT_AT, ProjectileKind, ShotView, flight_span};
pub use response::ResponseTimes;
pub use review::{Review, SheetView, WorldLabel};
pub use rig::Activity;
pub use rom_geo::SurfacePos;
pub use space::{
    AlertView, CapitalView, ChargeView, ClaimState, ClaimView, ColonyView, CountdownView,
    FLASH_TICKS, FlashView, HoldView, ResidenceView, ResourceBodyView, STRIKE_FLASH_TICKS,
    ShipView, SpaceSnapshot, StrikeView, TransitView,
};
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

pub use hud::{
    CHAT_HISTORY_ROWS, HudAgeDialog, HudChat, HudChatHistory, HudChatLine, HudPanel, HudRanking,
    HudResearch, HudResource, HudSelection, HudSnapshot, RankStanding,
};
pub use hud_model::{
    AlertMark, ButtonState, HudAlert, HudBlast, HudButton, HudCapital, HudCard, HudCityName,
    HudCityRadius, HudLineSite, HudMarker, HudPalette, HudPaletteGroup, HudPatrol, HudPlacement,
    HudQueue, HudQueueItem, HudRename, HudShield, HudShieldHit, HudStain, HudTile, HudTradeRoute,
    HudWealth, HudWreck, IconMark, MarkerKind, PaletteTab, ResearchCell, ResearchOverview,
    ResearchTrack, SLOT_KEYS, slot_of_key,
};
pub use pick::{
    Ground, HudRegionKind, HudTarget, PickContext, PickController, PickEvent, View, flat_surface,
};

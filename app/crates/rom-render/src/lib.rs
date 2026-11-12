//! Render the game world and match interface.
//!
//! Every raylib object lives on the render thread started by [`RenderWorker`].
//! Other threads talk to it only through the cloneable [`RenderBridge`]: they
//! publish the layout, render scale, camera and input, and take the newest
//! [`Frame`] from a one-slot mailbox.

mod camera;
mod mailbox;
mod pixels;
mod scene;
mod stats;
mod worker;

pub use camera::{Camera, CameraController, InputEvent, Key, PointerButton};
pub use image;
pub use mailbox::{Frame, FrameMailbox, FramePixels};
pub use pixels::rgba_to_bgra_flipped;
pub use stats::RenderStats;
pub use worker::{
    MAX_RENDER_SCALE, MIN_RENDER_SCALE, RenderBridge, RenderWorker, ShutdownHandle, scaled_size,
};

//! One-slot latest-frame mailbox between the render thread and the UI.

use std::sync::{Arc, Mutex, PoisonError};

/// Pixel container for a frame. The bytes are **BGRA**, premultiplied, top-down;
/// the `Rgba` in the type name is only the container's channel count.
pub type FramePixels = image::RgbaImage;

/// One finished frame from the render thread.
#[derive(Clone, Debug)]
pub struct Frame {
    /// BGRA premultiplied pixels, rows top-down.
    pub pixels: Arc<FramePixels>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Increases by one for every frame the render thread finishes, starting at 1.
    pub sequence: u64,
}

/// Holds at most one frame; publishing overwrites, taking empties the slot.
#[derive(Debug, Default)]
pub struct FrameMailbox {
    slot: Mutex<Option<Frame>>,
}

impl FrameMailbox {
    /// Replaces whatever frame is waiting with `frame`.
    pub fn publish(&self, frame: Frame) {
        let stale = self
            .slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replace(frame);
        drop(stale);
    }

    /// Takes the newest frame, if one arrived since the last take.
    pub fn take(&self) -> Option<Frame> {
        self.slot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(sequence: u64) -> Frame {
        Frame {
            pixels: Arc::new(FramePixels::new(1, 1)),
            width: 1,
            height: 1,
            sequence,
        }
    }

    #[test]
    fn mailbox_keeps_only_latest_frame() {
        let mailbox = FrameMailbox::default();
        mailbox.publish(frame(1));
        mailbox.publish(frame(2));
        mailbox.publish(frame(3));
        assert_eq!(mailbox.take().map(|frame| frame.sequence), Some(3));
        assert!(mailbox.take().is_none());
    }
}

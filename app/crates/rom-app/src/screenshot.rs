//! `--screenshot`: save one frame of the terrain as a PNG and exit.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use gpui::{App, AsyncApp, Entity, Timer};
use rom_render::image::{ImageFormat, RgbaImage};
use rom_render::{FramePixels, RenderBridge, ShutdownHandle};

use crate::bench::wait_until_ready;
use crate::world_view::WorldView;

/// Time allowed for the render thread to hand over the requested frame.
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(10);

/// Starts the capture; it writes the file, stops the render thread and quits the app.
pub fn start(
    path: PathBuf,
    after: Duration,
    view: Entity<WorldView>,
    bridge: RenderBridge,
    shutdown: ShutdownHandle,
    exit_code: Arc<AtomicU8>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        match capture(&path, after, &view, &bridge, cx).await {
            Ok(size) => println!("screenshot {} {}x{}", path.display(), size.0, size.1),
            Err(error) => {
                eprintln!("rise-of-man: screenshot failed: {error}");
                exit_code.store(1, Ordering::Relaxed);
            }
        }
        shutdown.stop_and_wait();
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}

async fn capture(
    path: &Path,
    after: Duration,
    view: &Entity<WorldView>,
    bridge: &RenderBridge,
    cx: &mut AsyncApp,
) -> Result<(u32, u32), String> {
    wait_until_ready(view, bridge, cx).await?;
    Timer::after(after).await;
    bridge.request_snapshot();
    let started = Instant::now();
    let pixels = loop {
        if let Some(pixels) = bridge.take_snapshot() {
            break pixels;
        }
        if started.elapsed() > SNAPSHOT_TIMEOUT {
            return Err("the render thread produced no frame".into());
        }
        Timer::after(Duration::from_millis(20)).await;
    };
    let rgba = bgra_to_rgba(&pixels).ok_or("frame has an inconsistent size")?;
    rgba.save_with_format(path, ImageFormat::Png)
        .map_err(|error| format!("writing {}: {error}", path.display()))?;
    Ok((rgba.width(), rgba.height()))
}

/// Converts the frame (BGRA, top-down) to the RGBA layout PNG files use.
fn bgra_to_rgba(frame: &FramePixels) -> Option<RgbaImage> {
    let mut bytes = frame.as_raw().clone();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    RgbaImage::from_raw(frame.width(), frame.height(), bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_frame_becomes_rgba() {
        let frame = FramePixels::from_raw(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 255]).expect("frame");
        let rgba = bgra_to_rgba(&frame).expect("image");
        assert_eq!(rgba.as_raw(), &vec![3, 2, 1, 255, 6, 5, 4, 255]);
    }
}

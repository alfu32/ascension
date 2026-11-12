//! `--bench`: run the world view for a fixed time and print one line of timings.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use gpui::{App, AsyncApp, Entity, Timer};
use rom_render::{RenderBridge, ShutdownHandle, scaled_size};

use crate::world_view::WorldView;

/// Time allowed for the window to open and deliver its first frame.
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
/// Time after the first correctly sized frame before measuring starts.
pub const WARM_UP: Duration = Duration::from_secs(1);
/// How often the last-second maxima are sampled.
const SAMPLE_INTERVAL: Duration = Duration::from_millis(250);

/// Benchmark parameters from the command line.
#[derive(Clone, Copy, Debug)]
pub struct Bench {
    /// Length of the measurement.
    pub seconds: f64,
    /// Requested canvas size in physical pixels.
    pub requested: (u32, u32),
    /// Requested render scale in percent.
    pub scale: u32,
}

/// Starts the benchmark; it prints its line, stops the render thread and quits the app.
pub fn start(
    bench: Bench,
    view: Entity<WorldView>,
    bridge: RenderBridge,
    shutdown: ShutdownHandle,
    exit_code: Arc<AtomicU8>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        match measure(bench, &view, &bridge, cx).await {
            Ok(line) => println!("{line}"),
            Err(error) => {
                eprintln!("rise-of-man: benchmark failed: {error}");
                exit_code.store(1, Ordering::Relaxed);
            }
        }
        shutdown.stop_and_wait();
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}

/// Frames shown so far and the size of the newest one.
fn presented(view: &Entity<WorldView>, cx: &mut AsyncApp) -> Result<(u64, (u32, u32)), String> {
    view.read_with(cx, |view, _| view.presented())
        .map_err(|error| format!("window closed: {error}"))
}

/// Waits until the terrain is meshed and a frame at the canvas size has been shown.
pub async fn wait_until_ready(
    view: &Entity<WorldView>,
    bridge: &RenderBridge,
    cx: &mut AsyncApp,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        Timer::after(Duration::from_millis(50)).await;
        let (frames, size) = presented(view, cx)?;
        let layout = bridge.layout();
        if frames > 0
            && bridge.terrain_ready()
            && layout.0 > 0
            && size == scaled_size(layout, bridge.render_scale())
        {
            return Ok(());
        }
        if started.elapsed() > STARTUP_TIMEOUT {
            return Err(format!(
                "no frame at the canvas size within {} s (canvas {}x{}, last frame {}x{}, terrain ready: {})",
                STARTUP_TIMEOUT.as_secs(),
                layout.0,
                layout.1,
                size.0,
                size.1,
                bridge.terrain_ready()
            ));
        }
    }
}

async fn measure(
    bench: Bench,
    view: &Entity<WorldView>,
    bridge: &RenderBridge,
    cx: &mut AsyncApp,
) -> Result<String, String> {
    wait_until_ready(view, bridge, cx).await?;
    Timer::after(WARM_UP).await;

    let presented = |cx: &mut AsyncApp| presented(view, cx);
    let first = bridge.stats();
    let (first_presented, _) = presented(cx)?;
    let begin = Instant::now();
    let end = begin + Duration::from_secs_f64(bench.seconds);
    let (mut frame_ms_max, mut readback_ms_max) = (0.0_f32, 0.0_f32);
    loop {
        let now = Instant::now();
        if now >= end {
            break;
        }
        Timer::after(SAMPLE_INTERVAL.min(end - now)).await;
        let stats = bridge.stats();
        frame_ms_max = frame_ms_max.max(stats.frame_ms_max);
        readback_ms_max = readback_ms_max.max(stats.readback_ms_max);
    }
    let elapsed = begin.elapsed().as_secs_f64();
    let last = bridge.stats();
    let (last_presented, (render_width, render_height)) = presented(cx)?;

    let shown = last_presented - first_presented;
    let rendered = last.frames_rendered - first.frames_rendered;
    let per_frame = |total: f64| total / rendered.max(1) as f64;
    let layout = bridge.layout();
    if layout != bench.requested {
        eprintln!(
            "rise-of-man: requested a {}x{} canvas, the window system gave {}x{}",
            bench.requested.0, bench.requested.1, layout.0, layout.1
        );
    }
    eprintln!(
        "bench-detail rendered={rendered} rendered_fps={:.1} render_target={render_width}x{render_height} seconds={elapsed:.2}",
        rendered as f64 / elapsed
    );
    Ok(format!(
        "bench size={}x{} scale={} frames={shown} fps={:.1} frame_ms_avg={:.2} frame_ms_max={:.2} readback_ms_avg={:.2} readback_ms_max={:.2}",
        layout.0,
        layout.1,
        bench.scale,
        shown as f64 / elapsed,
        per_frame(last.frame_ms_total - first.frame_ms_total),
        frame_ms_max,
        per_frame(last.readback_ms_total - first.readback_ms_total),
        readback_ms_max,
    ))
}

//! `--bench`: run the world view for a fixed time and print one line of timings.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use gpui::{App, AsyncApp, Entity, Timer};
use rom_render::{RenderBridge, ShutdownHandle, scaled_size};

use crate::sim_thread::SimLink;
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
    sim: SimLink,
    shutdown: ShutdownHandle,
    exit_code: Arc<AtomicU8>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        match measure(bench, &view, &bridge, &sim, cx).await {
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
    sim: &SimLink,
    cx: &mut AsyncApp,
) -> Result<String, String> {
    wait_until_ready(view, bridge, cx).await?;
    Timer::after(WARM_UP).await;

    let presented = |cx: &mut AsyncApp| presented(view, cx);
    let first = bridge.stats();
    let first_tick = sim.report().tick;
    let first_snapshots = bridge.snapshots_published();
    let first_response = bridge.response_times();
    let first_cpu = thread_cpu();
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
    let last_cpu = thread_cpu();
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
    let report = sim.report();
    eprintln!(
        "bench-detail sim_tick={} sim_ticks_per_second={:.1} entities={}",
        report.tick, report.ticks_per_second, report.entities
    );
    let ticks = report.tick.saturating_sub(first_tick);
    let snapshots = bridge.snapshots_published() - first_snapshots;
    eprintln!(
        "bench-detail sim_ticks_measured={ticks} sim_tps_measured={:.1} snapshots_per_second={:.1} peak_rss_mb={}",
        ticks as f64 / elapsed,
        snapshots as f64 / elapsed,
        peak_rss_mb().map_or_else(|| "unknown".to_string(), |mb| mb.to_string())
    );
    let response = bridge.response_times();
    let clicks = response.count - first_response.count;
    let per_click = |total: f64, first: f64| (total - first) / f64::from(clicks.max(1));
    let stages: Vec<String> = (0..4)
        .map(|i| {
            format!(
                "{:.1}",
                per_click(response.stage_ms[i], first_response.stage_ms[i])
            )
        })
        .collect();
    eprintln!(
        "bench-detail clicks={clicks} click_ms_avg={:.1} click_ms_max={:.1} click_stages_ms={}",
        per_click(response.total_ms, first_response.total_ms),
        response.max_ms,
        stages.join("/")
    );
    // CPU per thread name over the measurement, busiest first (several threads may share a name).
    let mut names: Vec<&str> = last_cpu.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let used = |name: &str, cpu: &[(String, u64)]| -> u64 {
        cpu.iter()
            .filter(|(thread, _)| thread == name)
            .map(|(_, ticks)| ticks)
            .sum()
    };
    let percent = |ticks: u64| ticks as f64 / CLOCK_TICKS_PER_SECOND / elapsed * 100.0;
    let mut busy: Vec<(u64, &str)> = names
        .into_iter()
        .map(|name| {
            (
                used(name, &last_cpu).saturating_sub(used(name, &first_cpu)),
                name,
            )
        })
        .filter(|(ticks, _)| *ticks > 0)
        .collect();
    busy.sort_unstable_by(|a, b| b.cmp(a));
    let total: u64 = busy.iter().map(|(ticks, _)| ticks).sum();
    let threads: Vec<String> = busy
        .iter()
        .take(6)
        .map(|(ticks, name)| format!("{}={:.0}", name.replace(' ', "_"), percent(*ticks)))
        .collect();
    eprintln!(
        "bench-detail cpu_pct process={:.0} {}",
        percent(total),
        threads.join(" ")
    );
    eprintln!(
        "bench-detail computers={} think_ms_avg={:.2} think_ms_max={:.2} computer_orders={} computer_rejected={}",
        report.computers,
        f64::from(report.think_us_avg) / 1000.0,
        f64::from(report.think_us_max) / 1000.0,
        report.computer_orders,
        report.computer_rejected
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

/// Kernel clock ticks per second of `/proc` CPU times (`USER_HZ`, 100 on Linux).
const CLOCK_TICKS_PER_SECOND: f64 = 100.0;

/// CPU time (user and system, in clock ticks) of each thread of this process, by thread name,
/// where the system reports it (Linux `/proc`).
fn thread_cpu() -> Vec<(String, u64)> {
    let Ok(tasks) = std::fs::read_dir("/proc/self/task") else {
        return Vec::new();
    };
    tasks
        .filter_map(|task| {
            let stat = std::fs::read_to_string(task.ok()?.path().join("stat")).ok()?;
            // The name is in parentheses and may hold spaces; the times follow it.
            let (head, tail) = stat.rsplit_once(')')?;
            let name = head.split_once('(')?.1.to_owned();
            let fields: Vec<&str> = tail.split_whitespace().collect();
            // Fields 14 and 15 of the line (utime, stime) are 11 and 12 after the name and state.
            let user: u64 = fields.get(11)?.parse().ok()?;
            let system: u64 = fields.get(12)?.parse().ok()?;
            Some((name, user + system))
        })
        .collect()
}

/// The process's peak resident memory in MB, where the system reports it (`VmHWM` on Linux).
fn peak_rss_mb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

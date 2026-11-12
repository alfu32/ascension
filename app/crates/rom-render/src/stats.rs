//! Frame timing statistics over a sliding one-second window.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_secs(1);

/// Render-thread timings. Averages and maxima cover the last second.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderStats {
    /// Frames finished since the render thread started.
    pub frames_rendered: u64,
    /// Frames finished during the last second.
    pub frames_last_second: u32,
    /// Average time from frame start to mailbox hand-off, in milliseconds.
    pub frame_ms_avg: f32,
    /// Worst frame time in the last second, in milliseconds.
    pub frame_ms_max: f32,
    /// Average read-back plus pixel conversion time, in milliseconds.
    pub readback_ms_avg: f32,
    /// Worst read-back plus pixel conversion time in the last second, in milliseconds.
    pub readback_ms_max: f32,
    /// Sum of all frame times since start, for averages over longer spans.
    pub frame_ms_total: f64,
    /// Sum of all read-back plus conversion times since start.
    pub readback_ms_total: f64,
    /// Size of the render target of the newest frame.
    pub render_width: u32,
    /// Height of the render target of the newest frame.
    pub render_height: u32,
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    at: Instant,
    frame_ms: f32,
    readback_ms: f32,
}

/// Collects per-frame samples and summarises the last second.
#[derive(Debug, Default)]
pub(crate) struct StatsWindow {
    samples: VecDeque<Sample>,
    frames: u64,
    frame_ms_total: f64,
    readback_ms_total: f64,
    size: (u32, u32),
}

impl StatsWindow {
    pub(crate) fn record(
        &mut self,
        at: Instant,
        frame_ms: f32,
        readback_ms: f32,
        size: (u32, u32),
    ) {
        self.frames += 1;
        self.frame_ms_total += f64::from(frame_ms);
        self.readback_ms_total += f64::from(readback_ms);
        self.size = size;
        self.samples.push_back(Sample {
            at,
            frame_ms,
            readback_ms,
        });
        while self
            .samples
            .front()
            .is_some_and(|sample| at.duration_since(sample.at) > WINDOW)
        {
            self.samples.pop_front();
        }
    }

    pub(crate) fn snapshot(&self) -> RenderStats {
        let count = self.samples.len();
        let (mut frame_sum, mut frame_max, mut readback_sum, mut readback_max) =
            (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
        for sample in &self.samples {
            frame_sum += sample.frame_ms;
            frame_max = frame_max.max(sample.frame_ms);
            readback_sum += sample.readback_ms;
            readback_max = readback_max.max(sample.readback_ms);
        }
        let divisor = count.max(1) as f32;
        RenderStats {
            frames_rendered: self.frames,
            frames_last_second: u32::try_from(count).unwrap_or(u32::MAX),
            frame_ms_avg: frame_sum / divisor,
            frame_ms_max: frame_max,
            readback_ms_avg: readback_sum / divisor,
            readback_ms_max: readback_max,
            frame_ms_total: self.frame_ms_total,
            readback_ms_total: self.readback_ms_total,
            render_width: self.size.0,
            render_height: self.size.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_cover_only_the_last_second() {
        let start = Instant::now();
        let mut window = StatsWindow::default();
        window.record(start, 40.0, 20.0, (4, 4));
        window.record(start + Duration::from_millis(1500), 10.0, 2.0, (8, 8));
        window.record(start + Duration::from_millis(1600), 20.0, 4.0, (8, 8));
        let stats = window.snapshot();
        assert_eq!(stats.frames_rendered, 3);
        assert_eq!(stats.frames_last_second, 2);
        assert_eq!(stats.frame_ms_avg, 15.0);
        assert_eq!(stats.frame_ms_max, 20.0);
        assert_eq!(stats.readback_ms_avg, 3.0);
        assert_eq!(stats.readback_ms_max, 4.0);
        assert_eq!(stats.frame_ms_total, 70.0);
        assert_eq!((stats.render_width, stats.render_height), (8, 8));
    }
}

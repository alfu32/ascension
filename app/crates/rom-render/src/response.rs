//! Click-to-response time (task 108): from a button press to the first frame shown by the window
//! that was drawn after the simulation answered the pick the press made. One press is followed
//! at a time; presses made while one is followed are not counted.

use std::time::Instant;

/// Response times measured since the start, for averages over a span.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResponseTimes {
    /// Presses followed to a shown frame.
    pub count: u32,
    /// Sum of their times, in milliseconds.
    pub total_ms: f64,
    /// The longest, in milliseconds.
    pub max_ms: f32,
    /// Sums of the stages, in milliseconds: press to pick sent, to the first frame started
    /// after the simulation answered, to that frame's publication, to the window showing it.
    pub stage_ms: [f64; 4],
}

/// Where the followed press is on its way to the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Stage {
    #[default]
    Idle,
    /// The pick numbered `pick` went to the simulation.
    Answer { pick: u64 },
    /// Answered; frames read back from number `frame` on show it.
    Drawn { frame: u64 },
    /// In the frame published as `sequence`.
    Shown { sequence: u64 },
}

/// Follows one press at a time through the simulation, the render thread and the window.
#[derive(Debug, Default)]
pub(crate) struct ResponseProbe {
    pressed: Option<Instant>,
    since: Option<Instant>,
    /// When the followed press reached each stage so far.
    marks: Vec<Instant>,
    stage: Stage,
    times: ResponseTimes,
}

impl ResponseProbe {
    /// A button went down at `at`.
    pub(crate) fn pressed(&mut self, at: Instant) {
        if self.stage == Stage::Idle {
            self.pressed = Some(at);
        }
    }

    /// The render thread sent pick number `pick` (counted from 1) to the simulation.
    pub(crate) fn pick_sent(&mut self, pick: u64) {
        if self.stage == Stage::Idle
            && let Some(at) = self.pressed.take()
        {
            self.since = Some(at);
            self.marks.clear();
            self.marks.push(Instant::now());
            self.stage = Stage::Answer { pick };
        }
    }

    /// A frame starts drawing with the simulation's answers up to pick `answered` published;
    /// `frame` is the read-back number the frame will get (or a later frame).
    pub(crate) fn frame_started(&mut self, answered: u64, frame: u64) {
        if let Stage::Answer { pick } = self.stage
            && pick <= answered
        {
            self.marks.push(Instant::now());
            self.stage = Stage::Drawn { frame };
        }
    }

    /// The frame read back as `frame` was published to the window as `sequence`.
    pub(crate) fn frame_published(&mut self, frame: u64, sequence: u64) {
        if let Stage::Drawn { frame: from } = self.stage
            && frame >= from
        {
            self.marks.push(Instant::now());
            self.stage = Stage::Shown { sequence };
        }
    }

    /// The window showed the frame published as `sequence` at `at`.
    pub(crate) fn frame_shown(&mut self, sequence: u64, at: Instant) {
        if let Stage::Shown { sequence: wanted } = self.stage
            && sequence >= wanted
            && let Some(since) = self.since.take()
        {
            let ms = at.saturating_duration_since(since).as_secs_f64() * 1000.0;
            self.times.count += 1;
            self.times.total_ms += ms;
            self.times.max_ms = self.times.max_ms.max(ms as f32);
            self.marks.push(at);
            let mut from = since;
            for (stage, mark) in self.times.stage_ms.iter_mut().zip(&self.marks) {
                *stage += mark.saturating_duration_since(from).as_secs_f64() * 1000.0;
                from = *mark;
            }
            self.stage = Stage::Idle;
        }
    }

    /// Times measured so far.
    pub(crate) fn times(&self) -> ResponseTimes {
        self.times
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_press_is_timed_to_the_first_shown_frame_drawn_after_the_answer() {
        let start = Instant::now();
        let mut probe = ResponseProbe::default();
        probe.pressed(start);
        probe.pick_sent(1);
        // A second press while the first is followed is not counted.
        probe.pressed(start + Duration::from_millis(1));
        probe.pick_sent(2);
        probe.frame_started(0, 10);
        probe.frame_published(10, 5);
        probe.frame_shown(5, start + Duration::from_millis(20));
        assert_eq!(probe.times().count, 0, "not answered yet");
        probe.frame_started(1, 11);
        probe.frame_published(10, 6);
        probe.frame_shown(6, start + Duration::from_millis(30));
        assert_eq!(
            probe.times().count,
            0,
            "frame 10 was drawn before the answer"
        );
        probe.frame_published(11, 7);
        probe.frame_shown(7, start + Duration::from_millis(50));
        let times = probe.times();
        assert_eq!(times.count, 1);
        assert!((times.total_ms - 50.0).abs() < 1e-6);
        assert_eq!(times.max_ms, 50.0);
        // The probe is free for the next press.
        probe.pressed(start + Duration::from_millis(60));
        probe.pick_sent(3);
        probe.frame_started(3, 12);
        probe.frame_published(12, 8);
        probe.frame_shown(9, start + Duration::from_millis(80));
        assert_eq!(probe.times().count, 2);
        assert_eq!(probe.times().max_ms, 50.0);
    }
}

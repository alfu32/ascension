//! Temporary frame profiling: `ROM_PROFILE=1` prints the time between marks every 120 frames.
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Instant;

struct State {
    last: Instant,
    sums: BTreeMap<&'static str, f64>,
    frames: u32,
    on: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        last: Instant::now(),
        sums: BTreeMap::new(),
        frames: 0,
        on: std::env::var_os("ROM_PROFILE").is_some(),
    });
}

/// Starts a frame.
pub fn begin() {
    STATE.with(|s| s.borrow_mut().last = Instant::now());
}

/// Adds the time since the previous mark to `name`.
pub fn mark(name: &'static str) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.on {
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(s.last).as_secs_f64() * 1000.0;
        *s.sums.entry(name).or_default() += dt;
        s.last = now;
    });
}

/// Ends a frame and prints the averages now and then.
pub fn end() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.on {
            return;
        }
        s.frames += 1;
        if s.frames >= 120 {
            let n = f64::from(s.frames);
            let line: Vec<String> = s
                .sums
                .iter()
                .map(|(k, v)| format!("{k}={:.2}", v / n))
                .collect();
            eprintln!("prof ms/frame: {}", line.join(" "));
            s.sums.clear();
            s.frames = 0;
        }
    });
}

//! Sound in the game (task 125): the audio engine of the process, the player's Sound settings,
//! and the director that turns what happens into sounds. Presentation only: nothing here feeds
//! the simulation, and the simulation knows nothing of sound.

mod book;
mod director;

use std::sync::{Arc, Mutex};

pub use director::{Director, UiCue};

use rom_audio::{AudioEngine, Placement, SoundId, Volumes};

/// The Sound settings, each volume 0–100 percent of the gentle default mix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Everything.
    pub master: u32,
    /// Weapons, work, movement, buildings and interface cues.
    pub effects: u32,
    /// Unit lines and the announcer.
    pub voice: u32,
    /// The ambience beds.
    pub ambient: u32,
    /// Silence the game while its window is not focused.
    pub mute_unfocused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            master: 100,
            effects: 100,
            voice: 100,
            ambient: 100,
            mute_unfocused: true,
        }
    }
}

struct State {
    engine: Arc<AudioEngine>,
    settings: Settings,
    focused: bool,
}

static SOUND: Mutex<Option<State>> = Mutex::new(None);

fn state() -> std::sync::MutexGuard<'static, Option<State>> {
    SOUND.lock().unwrap_or_else(|e| e.into_inner())
}

/// Starts the process's audio engine with `settings`; `mute` (`--mute`) makes it silent. Tests
/// never call it: without it every sound call does nothing.
pub fn install(settings: Settings, mute: bool) {
    let engine = Arc::new(if mute {
        AudioEngine::silent()
    } else {
        AudioEngine::new()
    });
    *state() = Some(State {
        engine,
        settings,
        focused: true,
    });
    apply();
}

/// Stops the audio engine (its thread ends once the last user lets it go).
pub fn shutdown() {
    if let Some(state) = state().take() {
        state.engine.stop_all();
    }
}

/// The process's audio engine, if one is installed.
pub fn engine() -> Option<Arc<AudioEngine>> {
    state().as_ref().map(|s| Arc::clone(&s.engine))
}

/// The settings in use.
pub fn settings() -> Settings {
    state().as_ref().map(|s| s.settings).unwrap_or_default()
}

/// Changes the settings (the in-match menu).
pub fn set_settings(settings: Settings) {
    if let Some(state) = state().as_mut() {
        state.settings = settings;
    }
    apply();
}

/// The window gained or lost the focus: the game falls silent while unfocused if the settings
/// say so.
pub fn set_focused(focused: bool) {
    if let Some(state) = state().as_mut() {
        state.focused = focused;
    }
    apply();
}

fn apply() {
    let guard = state();
    let Some(state) = guard.as_ref() else {
        return;
    };
    let base = Volumes::default();
    let s = state.settings;
    let pct = |p: u32| p.min(100) as f32 / 100.0;
    let silent = s.mute_unfocused && !state.focused;
    state.engine.set_volumes(
        if silent {
            0.0
        } else {
            base.master * pct(s.master)
        },
        base.effects * pct(s.effects),
        base.voice * pct(s.voice),
        base.ambient * pct(s.ambient),
    );
}

/// `--sound-check`: starts the engine alone, waits for it and prints whether it plays (the
/// device and rate) or why it is silent. Plays nothing; a machine without a sound device is not
/// an error.
pub fn check() -> std::process::ExitCode {
    let engine = AudioEngine::new();
    let started = std::time::Instant::now();
    while engine.status() == rom_audio::EngineStatus::Starting
        && started.elapsed() < std::time::Duration::from_secs(10)
    {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    match engine.status() {
        rom_audio::EngineStatus::Running { device, rate } => {
            println!("sound: plays on {device} at {rate} Hz");
        }
        rom_audio::EngineStatus::Silent { reason } => println!("sound: silent ({reason})"),
        rom_audio::EngineStatus::Starting => println!("sound: still starting after 10 s"),
    }
    std::process::ExitCode::SUCCESS
}

/// The click of a control in a window of the game (launcher, menus).
pub fn click() {
    if let Some(engine) = engine() {
        let clicks = book::family("ui.click");
        if let Some(id) = clicks.get(pick(clicks.len())) {
            engine.play(
                *id,
                Placement::centre(CLICK_GAIN),
                rom_audio::FOREGROUND_PRIORITY,
            );
        }
    }
}

/// Gain of a control's click.
pub const CLICK_GAIN: f32 = 0.45;

/// A number below `n` that changes from call to call: which variation plays. Not the
/// simulation's generator, which sound never touches.
fn pick(n: usize) -> usize {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);
    let mut x = NEXT.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    NEXT.store(x, Ordering::Relaxed);
    if n == 0 { 0 } else { (x % n as u64) as usize }
}

/// One of a cue's variations.
fn variant(variants: &[SoundId]) -> Option<SoundId> {
    variants.get(pick(variants.len())).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_without_an_engine_do_nothing() {
        // Tests never install the engine.
        click();
        set_focused(false);
        set_focused(true);
        assert_eq!(settings(), Settings::default());
    }

    #[test]
    fn picks_stay_below_n() {
        for n in 1..10 {
            assert!((0..50).all(|_| pick(n) < n));
        }
        assert_eq!(pick(0), 0);
    }
}

//! Sound in the game (task 125): the audio engine of the process, the player's Sound settings
//! (the mixer's categories since task 133), the director that turns what happens into sounds,
//! and the music of the launcher and the match (task 133). Presentation only: nothing here feeds
//! the simulation, and the simulation knows nothing of sound.

mod book;
mod director;
pub mod music;

use std::sync::{Arc, Mutex};

pub use director::{Director, UiCue};

use rom_audio::{AudioEngine, Placement, SoundId, Volumes};

/// The Sound settings (task 133: the owner's categories), each volume 0–100 percent of the
/// gentle default mix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Everything.
    pub master: u32,
    /// The music: the launcher's, the ages' themes and the interludes' fragments.
    pub music: u32,
    /// Units' voices, shots, blows and movement.
    pub units: u32,
    /// Buildings' lines, work sounds, construction, and their trained and researched cues.
    pub buildings: u32,
    /// The ambience beds.
    pub environment: u32,
    /// Alerts, the announcer, age and research announcements and the interface cues.
    pub announcements: u32,
    /// Silence the game while its window is not focused.
    pub mute_unfocused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            master: 100,
            music: 100,
            units: 100,
            buildings: 100,
            environment: 100,
            announcements: 100,
            mute_unfocused: true,
        }
    }
}

/// The Sound settings before task 133: effects (weapons, work, movement, buildings, interface),
/// voices (unit lines, announcer) and ambience, each a percent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OldVolumes {
    /// `effects-volume`.
    pub effects: Option<u32>,
    /// `voice-volume`.
    pub voice: Option<u32>,
    /// `ambient-volume`.
    pub ambient: Option<u32>,
}

impl OldVolumes {
    /// Whether any old value was given.
    pub fn any(&self) -> bool {
        self.effects.is_some() || self.voice.is_some() || self.ambient.is_some()
    }

    /// The categories the old values stand for, never louder than before: units heard both
    /// the effects (shots, steps) and the voices (lines), so they take the lower of the two;
    /// buildings take the effects; the environment the ambience. The announcements and the
    /// music keep their own values.
    pub fn migrate(&self, into: &mut Settings) {
        let effects = self.effects.unwrap_or(100);
        let voice = self.voice.unwrap_or(100);
        if self.effects.is_some() || self.voice.is_some() {
            into.units = effects.min(voice);
        }
        if let Some(effects) = self.effects {
            into.buildings = effects;
        }
        if let Some(ambient) = self.ambient {
            into.environment = ambient;
        }
    }
}

impl Settings {
    /// The engine's volumes for these settings (`silent`: the window is not focused and the
    /// settings say to mute then).
    pub fn volumes(&self, silent: bool) -> Volumes {
        let base = Volumes::default();
        let pct = |p: u32| p.min(100) as f32 / 100.0;
        Volumes {
            master: if silent {
                0.0
            } else {
                base.master * pct(self.master)
            },
            music: base.music * pct(self.music),
            units: base.units * pct(self.units),
            buildings: base.buildings * pct(self.buildings),
            environment: base.environment * pct(self.environment),
            announcements: base.announcements * pct(self.announcements),
        }
    }

    /// The volume of a slider by its option name (`volume`, `music-volume`, …).
    pub fn get(&self, flag: &str) -> Option<u32> {
        Some(match flag {
            "volume" => self.master,
            "music-volume" => self.music,
            "units-volume" => self.units,
            "buildings-volume" => self.buildings,
            "environment-volume" => self.environment,
            "announcements-volume" => self.announcements,
            _ => return None,
        })
    }

    /// Sets a slider by its option name; false for another name.
    pub fn set(&mut self, flag: &str, percent: u32) -> bool {
        let percent = percent.min(100);
        match flag {
            "volume" => self.master = percent,
            "music-volume" => self.music = percent,
            "units-volume" => self.units = percent,
            "buildings-volume" => self.buildings = percent,
            "environment-volume" => self.environment = percent,
            "announcements-volume" => self.announcements = percent,
            _ => return false,
        }
        true
    }
}

/// The sliders of the mixer: option name and label, in order.
pub const SLIDERS: [(&str, &str); 6] = [
    ("volume", "Master"),
    ("music-volume", "Music"),
    ("units-volume", "Units"),
    ("buildings-volume", "Buildings"),
    ("environment-volume", "Environment"),
    ("announcements-volume", "Announcements"),
];

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
    let s = state.settings;
    let silent = s.mute_unfocused && !state.focused;
    state.engine.set_volumes(s.volumes(silent));
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
            engine.play_as(
                rom_audio::Category::Announcements,
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
    fn old_volumes_map_to_the_categories() {
        let mut settings = Settings::default();
        OldVolumes::default().migrate(&mut settings);
        assert_eq!(settings, Settings::default());
        OldVolumes {
            effects: Some(50),
            voice: Some(90),
            ambient: Some(30),
        }
        .migrate(&mut settings);
        assert_eq!(
            (settings.units, settings.buildings, settings.environment),
            (50, 50, 30)
        );
        assert_eq!((settings.music, settings.announcements), (100, 100));
        let mut settings = Settings::default();
        OldVolumes {
            voice: Some(20),
            ..OldVolumes::default()
        }
        .migrate(&mut settings);
        assert_eq!((settings.units, settings.buildings), (20, 100));
    }

    #[test]
    fn sliders_read_and_write_their_settings() {
        let mut settings = Settings::default();
        for (i, (flag, _)) in SLIDERS.iter().enumerate() {
            assert!(settings.set(flag, i as u32 * 10));
        }
        for (i, (flag, _)) in SLIDERS.iter().enumerate() {
            assert_eq!(settings.get(flag), Some(i as u32 * 10));
        }
        assert!(!settings.set("mute", 3));
        let v = settings.volumes(false);
        assert_eq!(v.master, 0.0);
        assert!((v.announcements - 0.5).abs() < 1e-6);
        assert_eq!(settings.volumes(true).master, 0.0);
        assert!((Settings::default().volumes(false).master - 0.8).abs() < 1e-6);
    }

    #[test]
    fn picks_stay_below_n() {
        for n in 1..10 {
            assert!((0..50).all(|_| pick(n) < n));
        }
        assert_eq!(pick(0), 0);
    }
}

//! Window-independent form, argument generation and configuration storage.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::args;

/// Navigation labels in the same order as control group indices.
pub(super) const GROUPS: [&str; 6] = [
    "Game",
    "Window",
    "Camera and light",
    "Model review",
    "Automation",
    "Models",
];
/// The group of model tools (export to and import from Meshcraft), which has no options.
pub(super) const MODELS_GROUP: usize = 5;

/// One command-line option and its form presentation.
pub(super) struct Control {
    /// Flag without its leading dashes.
    pub flag: &'static str,
    /// Index in the navigation labels.
    pub group: usize,
    /// Command-line default, or blank for an unset option.
    pub default: &'static str,
    /// Units, limits and behavior shown below the editor.
    pub help: &'static str,
    /// Whether this flag has no argument.
    pub toggle: bool,
}

macro_rules! controls {
    ($(($flag:literal, $group:literal, $default:literal, $help:literal, $toggle:literal)),* $(,)?) => {
        /// Complete catalogue shared by the UI, argument generator and coverage test.
        pub(super) const CONTROLS: &[Control] = &[$(Control {
            flag: $flag, group: $group, default: $default, help: $help, toggle: $toggle,
        }),*];
    };
}

controls! {
    ("sphere", 0, "", "Face size: 64–512 in a universe; 16–512 on one planet. Blank: flat map.", false),
    ("map-size", 0, "256x256", "Flat map width × height in tiles; ignored for spheres.", false),
    ("single-planet", 0, "false", "Isolated sphere; defaults to face size 192. Excludes universe.", true),
    ("universe", 0, "false", "Generate star systems; also implied by sphere unless isolated.", true),
    ("style", 0, "", "Map style from the rules; blank selects the first in id order.", false),
    ("seed", 0, "1", "Map seed (unsigned integer). Random chooses a fresh seed.", false),
    ("players", 0, "4", "Number of players, at least 1; factions override this count.", false),
    ("factions", 0, "", "One faction per player, in selection order; blank uses the default mix.", false),
    ("ai", 0, "", "Computer players 2, 3, …: profile:difficulty each; blank plays balanced:moderate; none leaves them idle.", false),
    ("watch", 0, "false", "Player one is a computer player too (watch a match); --ai then starts with player one.", true),
    ("age", 0, "", "Starting age for every player; blank uses the first age.", false),
    ("speed", 0, "100", "Simulation speed, 1–10000 percent; normal is 20 ticks per second.", false),
    ("demo", 0, "false", "Run the scripted demo driver.", true),
    ("start-markers", 0, "false", "Draw player-start pillars.", true),
    ("stress-units", 0, "0", "Extra wandering units spawned before play.", false),
    ("boxes", 0, "0", "Number of moving stress-test boxes.", false),
    ("size", 1, "1280x800", "Canvas width × height in physical pixels (1–16384 each).", false),
    ("scale", 1, "100", "Internal render resolution, 50–100 percent.", false),
    ("hud-scale", 1, "", "HUD scale, 25–200 percent; blank uses automatic scaling.", false),
    ("no-hud", 1, "false", "Hide the match HUD.", true),
    ("stats", 1, "false", "Show diagnostic frame and simulation counters.", true),
    ("view", 2, "", "Initial view over home: surface, globe, system or star map.", false),
    ("zoom-out", 2, "false", "Start at the farthest zoom the player's age allows.", true),
    ("zoom-all", 2, "false", "Lift the age-based zoom cap for testing.", true),
    ("orbit", 2, "", "Sphere camera: longitude, latitude, distance, heading (degrees / tiles).", false),
    ("camera", 2, "", "Flat camera: x,y,distance,yaw,pitch (tiles / degrees).", false),
    ("look-at", 2, "", "Terrain, optional nearby terrain and positive distance in tiles.", false),
    ("sun", 2, "orbit", "Sun light: orbit, fixed over the first view, or midnight.", false),
    ("graticule", 2, "on", "Draw latitude and longitude lines on spheres.", false),
    ("gallery", 3, "", "Review all models, units, an age, entity key or composed model id.", false),
    ("gallery-view", 3, "game", "Gallery camera: game, front, side, top or close.", false),
    ("team", 3, "1", "Model owner colour, 1–8.", false),
    ("model", 3, "", "Review one composed model from four sides; overrides gallery.", false),
    ("parts", 3, "false", "Label and outline parts in one-model review.", true),
    ("pose", 3, "idle", "Unit activity: idle, move, attack or work.", false),
    ("anim-time", 3, "", "Fixed nonnegative animation time in seconds; blank runs animation.", false),
    ("gallery-manifest", 3, "", "Write the model manifest to this JSON path and exit without a game window.", false),
    ("screenshot", 4, "", "Write a PNG to this path after the first frame, then exit.", false),
    ("after", 4, "1", "Seconds after the first frame before taking a screenshot.", false),
    ("bench", 4, "", "Run for this many positive seconds, print measurements and exit.", false),
    ("script", 4, "", "Path to a pick-event script replayed on the simulation thread.", false),
}

#[derive(Clone, Debug, PartialEq)]
/// Editable text is retained even while the parser reports it invalid.
pub(super) struct Form {
    values: BTreeMap<String, String>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            values: CONTROLS
                .iter()
                .map(|c| (c.flag.into(), c.default.into()))
                .collect(),
        }
    }
}

impl Form {
    /// Returns the displayed value of a control.
    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map_or("", String::as_str)
    }

    /// Changes one control without discarding other unfinished edits.
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    /// Emits non-default options, or the routing marker for an otherwise empty game command.
    pub fn arguments(&self) -> Vec<String> {
        let mut out = Vec::new();
        for control in CONTROLS {
            let value = self.get(control.flag);
            if control.toggle {
                if value == "true" {
                    out.push(format!("--{}", control.flag));
                }
            } else if !value.is_empty() && value != control.default {
                out.extend([format!("--{}", control.flag), value.into()]);
            }
        }
        if out.is_empty() {
            out.push("--run".into());
        }
        out
    }

    /// Uses the game's existing parser as the form's validation contract.
    pub fn validate(&self) -> Result<args::Options, String> {
        args::parse(self.arguments())
    }

    fn from_json(value: &Value) -> Self {
        let mut form = Self::default();
        for control in CONTROLS {
            if let Some(text) = value.get(control.flag).and_then(Value::as_str)
                && (!control.toggle || matches!(text, "true" | "false"))
            {
                form.set(control.flag, text);
            }
        }
        form
    }
}

impl Form {
    /// The player computer slot 0 plays: one when watching, else two.
    pub fn first_computer(&self) -> usize {
        if self.get("watch") == "true" { 1 } else { 2 }
    }

    /// Computer players: every player but the person (all of them when watching); the
    /// faction count if factions are chosen, else `players`.
    pub fn computer_slots(&self) -> usize {
        let factions = self
            .get("factions")
            .split(',')
            .filter(|s| !s.is_empty())
            .count();
        let players = if factions > 0 {
            factions
        } else {
            self.get("players").parse::<usize>().unwrap_or(1)
        };
        (players + 1).saturating_sub(self.first_computer()).min(16)
    }

    /// The `profile:difficulty` of computer slot `slot` (0 = player two).
    pub fn computer(&self, slot: usize) -> (String, String) {
        let (profile, difficulty) = crate::computer::DEFAULT_SPEC;
        self.get("ai")
            .split(',')
            .nth(slot)
            .filter(|s| !s.is_empty() && *s != "none")
            .and_then(|s| crate::computer::Spec::parse(s).ok())
            .map_or((profile.into(), difficulty.into()), |s| {
                (s.profile, s.difficulty)
            })
    }

    /// Sets one slot's profile and difficulty, writing every slot before it explicitly.
    pub fn set_computer(&mut self, slot: usize, profile: &str, difficulty: &str) {
        let count = self.computer_slots().max(slot + 1);
        let entries: Vec<String> = (0..count)
            .map(|i| {
                let (p, d) = if i == slot {
                    (profile.to_owned(), difficulty.to_owned())
                } else {
                    self.computer(i)
                };
                format!("{p}:{d}")
            })
            .collect();
        self.set("ai", entries.join(","));
    }
}

/// Built-in owner workflows.
pub(super) const PRESETS: [&str; 6] = [
    "Play (universe)",
    "Play (single planet)",
    "Demo",
    "Model gallery",
    "One model",
    "Bench",
];

/// Fills a workflow from parser defaults and the rules' first composed model.
pub(super) fn preset(index: usize, first_model: Option<&str>) -> Form {
    let mut form = Form::default();
    let pairs: &[(&str, &str)] = match index {
        0 => &[("universe", "true")],
        1 => &[("single-planet", "true")],
        2 => &[("single-planet", "true"), ("demo", "true")],
        3 => &[("gallery", "all")],
        4 => &[("model", first_model.unwrap_or(""))],
        _ => &[("bench", "10"), ("single-planet", "true"), ("sphere", "64")],
    };
    for &(key, value) in pairs {
        form.set(key, value);
    }
    form
}

/// Shell quoting is only for display and clipboard; spawning always passes an argument vector.
pub(super) fn command_line(executable: &Path, arguments: &[String]) -> String {
    std::iter::once(executable.to_string_lossy().into_owned())
        .chain(arguments.iter().cloned())
        .map(|s| {
            if !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-./,:@+=%".contains(&b))
            {
                s
            } else {
                format!("'{}'", s.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The two copyable commands use precisely the argument vector passed to the child.
pub(super) fn commands(executable: &Path, arguments: &[String]) -> [String; 2] {
    [
        format!(
            "env -u WAYLAND_DISPLAY {}",
            command_line(executable, arguments)
        ),
        format!(
            "env -u WAYLAND_DISPLAY {}",
            command_line(
                Path::new("cargo"),
                &["run", "--release", "-p", "rom-app", "--"]
                    .into_iter()
                    .map(str::to_owned)
                    .chain(arguments.iter().cloned())
                    .collect::<Vec<_>>(),
            )
        ),
    ]
}

#[derive(Default)]
/// Last form and a bounded collection of named forms.
pub(super) struct Config {
    /// Most recently edited form.
    pub form: Form,
    /// Owner-defined reusable forms, sorted by name.
    pub presets: BTreeMap<String, Form>,
}

impl Config {
    /// Reads known fields and fills missing or mistyped fields from defaults.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        Ok(Self {
            form: Form::from_json(&value["form"]),
            presets: value["presets"]
                .as_object()
                .into_iter()
                .flatten()
                .take(12)
                .map(|(name, form)| (name.clone(), Form::from_json(form)))
                .collect(),
        })
    }

    /// Replaces the configuration atomically within its directory.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let value = json!({
            "form": self.form.values,
            "presets": self.presets.iter().map(|(name, form)| (name, &form.values)).collect::<BTreeMap<_, _>>(),
        });
        let text = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
        let parent = path.parent().ok_or("configuration path has no parent")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, text).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    }
}

/// Resolves the XDG configuration location with the conventional home fallback.
pub(super) fn config_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or("neither XDG_CONFIG_HOME nor HOME is set")?;
    Ok(base.join("rise-of-man/launcher.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_presets_round_trip() {
        assert_eq!(Form::default().validate(), Ok(args::Options::default()));
        for control in CONTROLS
            .iter()
            .filter(|c| !c.toggle && !c.default.is_empty())
        {
            assert_eq!(
                args::parse([format!("--{}", control.flag), control.default.into()]),
                Ok(args::Options::default()),
                "displayed default for --{}",
                control.flag,
            );
        }
        assert_eq!(Form::default().arguments(), ["--run"]);
        let mut changed = Form::default();
        changed.set("seed", "72");
        assert_eq!(changed.arguments(), ["--seed", "72"]);
        let rules = crate::terrain::load_rules().unwrap();
        let models = super::super::Catalog::new(&rules);
        for (index, name) in PRESETS.iter().enumerate() {
            let form = preset(index, models.first_model());
            let mut expected = args::Options::default();
            match index {
                0 => expected.universe = true,
                1 => expected.single_planet = true,
                2 => {
                    expected.single_planet = true;
                    expected.demo = true;
                }
                3 => expected.gallery = Some("all".into()),
                4 => expected.model = models.first_model().map(str::to_owned),
                _ => {
                    expected.bench_seconds = Some(10.0);
                    expected.single_planet = true;
                    expected.sphere = Some(64);
                }
            }
            assert_eq!(form.validate(), Ok(expected), "{name}");
        }
    }

    #[test]
    fn every_non_default_value_round_trips() {
        let values = [
            ("sphere", "128"),
            ("map-size", "300x200"),
            ("universe", "true"),
            ("style", "test.style"),
            ("seed", "44"),
            ("players", "2"),
            ("factions", "test.a,test.b"),
            ("ai", "test.profile:test.difficulty"),
            ("watch", "true"),
            ("age", "3"),
            ("speed", "250"),
            ("demo", "true"),
            ("start-markers", "true"),
            ("stress-units", "99"),
            ("boxes", "15"),
            ("size", "900x600"),
            ("scale", "75"),
            ("hud-scale", "85"),
            ("no-hud", "true"),
            ("stats", "true"),
            ("view", "system,home"),
            ("zoom-out", "true"),
            ("zoom-all", "true"),
            ("orbit", "20,30,40,50"),
            ("camera", "10,20,30,40,50"),
            ("look-at", "test.terrain+test.near,12"),
            ("sun", "night"),
            ("graticule", "off"),
            ("gallery", "units"),
            ("gallery-view", "side"),
            ("team", "4"),
            ("model", "test.model"),
            ("parts", "true"),
            ("pose", "attack"),
            ("anim-time", "0.25"),
            ("gallery-manifest", "manifest.json"),
            ("screenshot", "a path's screenshot.png"),
            ("after", "3"),
            ("bench", "2"),
            ("script", "a script.txt"),
        ];
        let mut form = Form::default();
        assert_eq!(values.len() + 1, CONTROLS.len());
        for (key, value) in values {
            form.set(key, value);
        }
        let expected_args = values
            .into_iter()
            .flat_map(|(k, v)| {
                if v == "true" {
                    vec![format!("--{k}")]
                } else {
                    vec![format!("--{k}"), v.into()]
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            form.validate().unwrap(),
            args::parse(expected_args).unwrap()
        );
        assert_ne!(form.validate().unwrap(), args::Options::default());
        // The two topology flags deliberately cannot both be enabled.
        form.set("universe", "false");
        form.set("single-planet", "true");
        assert!(form.validate().unwrap().single_planet);
        form.set("universe", "true");
        assert!(form.validate().is_err());
    }

    #[test]
    fn usage_options_have_controls() {
        let mut seen = std::collections::BTreeSet::new();
        for control in CONTROLS {
            assert!(
                seen.insert(control.flag),
                "duplicate control: {}",
                control.flag
            );
            assert!(
                control.group < GROUPS.len(),
                "unreachable control: {}",
                control.flag
            );
        }
        let words = args::USAGE.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'));
        for flag in words.filter(|s| s.starts_with("--")) {
            assert!(
                CONTROLS.iter().any(|c| format!("--{}", c.flag) == flag),
                "missing control: {flag}"
            );
        }
        assert!(CONTROLS.iter().any(|c| c.flag == "script"));
        assert_eq!(CONTROLS.len(), 41);
    }

    #[test]
    fn computer_slots_follow_the_player_count() {
        let mut form = Form::default();
        assert_eq!(form.computer_slots(), 3);
        assert_eq!(form.computer(2), ("balanced".into(), "moderate".into()));
        form.set_computer(1, "builder", "tough");
        assert_eq!(
            form.get("ai"),
            "balanced:moderate,builder:tough,balanced:moderate"
        );
        assert_eq!(form.computer(1), ("builder".into(), "tough".into()));
        form.set("factions", "a,b");
        assert_eq!(form.computer_slots(), 1);
        form.set("ai", "none");
        assert_eq!(form.computer(0), ("balanced".into(), "moderate".into()));
        form.set("watch", "true");
        assert_eq!((form.first_computer(), form.computer_slots()), (1, 2));
        let mut form = Form::default();
        form.set_computer(0, "rusher", "easy");
        assert_eq!(form.validate().unwrap().ai[0].to_string(), "rusher:easy");
    }

    #[test]
    fn config_round_trip_and_forward_compatibility() {
        let dir = std::env::temp_dir().join(format!("rom-launcher-test-{}", std::process::id()));
        let path = dir.join("launcher.json");
        let mut config = Config {
            form: preset(2, None),
            ..Default::default()
        };
        config.form.set("screenshot", "a path's image.png");
        config.presets.insert("My demo".into(), config.form.clone());
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.form, config.form);
        assert_eq!(loaded.presets, config.presets);
        std::fs::write(&path, r#"{"form":{"seed":"72","future":"unknown","stats":null},"future":true,"presets":{"old":{"seed":"8"}}}"#).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.form.get("seed"), "72");
        assert_eq!(loaded.form.get("size"), "1280x800");
        assert_eq!(loaded.form.get("stats"), "false");
        assert_eq!(loaded.presets["old"].get("size"), "1280x800");
        assert!(!loaded.form.values.contains_key("future"));
        std::fs::write(&path, "{broken").unwrap();
        assert!(Config::load(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
        assert_eq!(Config::load(&path).unwrap().form, Form::default());
    }

    #[test]
    fn command_quotes_paths_without_interpreting_shell_syntax() {
        assert_eq!(
            command_line(Path::new("/a b/game"), &["a'b;$(no)".into()]),
            "'/a b/game' 'a'\\''b;$(no)'"
        );
        let mut form = Form::default();
        let executable = Path::new("target/release/rise-of-man");
        assert_eq!(
            commands(executable, &form.arguments()),
            [
                "env -u WAYLAND_DISPLAY target/release/rise-of-man --run",
                "env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --run",
            ]
        );
        form.set("seed", "72");
        form.set("screenshot", "a path's image.png");
        assert_eq!(
            commands(executable, &form.arguments()),
            [
                "env -u WAYLAND_DISPLAY target/release/rise-of-man --seed 72 --screenshot 'a path'\\''s image.png'",
                "env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 72 --screenshot 'a path'\\''s image.png'",
            ]
        );
    }
}

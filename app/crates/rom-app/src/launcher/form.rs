//! Window-independent form, argument generation and configuration storage.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::args;

/// Navigation labels in the same order as control group indices.
pub(super) const GROUPS: [&str; 8] = [
    "Game",
    "Window",
    "Camera and light",
    "Model review",
    "Automation",
    "Network",
    "Models",
    "Saved games",
];
/// The group of model tools (export to and import from Meshcraft), which has no options.
pub(super) const MODELS_GROUP: usize = 6;
/// The group listing saved games, which has no options either.
pub(super) const SAVES_GROUP: usize = 7;

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
    ("map-size", 0, "256x256", "Flat map width × height in tiles. A universe match (what the launcher starts) does not use it.", false),
    ("single-planet", 0, "false", "Isolated sphere; defaults to face size 192. Excludes universe.", true),
    ("universe", 0, "false", "Generate star systems; also implied by sphere unless isolated.", true),
    ("style", 0, "", "The map: Planet, Solar, Cluster, Sector, Quadrant or Galaxy. None chosen: Cluster.", false),
    ("start-placement", 0, "", "Where players start in a universe: separate home worlds (default), or players of different races together on as few home planets as fairness allows. Ignored on flat maps and single planets.", false),
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
    ("grass", 1, "on", "Grass detail: blades on the ground near the camera, on or off.", false),
    ("detail", 1, "medium", "Rendering detail: how far the grass reaches; min, medium (as before), max (about 1.5x as far) or ultra (about 2x). Changeable in the game menu.", false),
    ("projectiles", 1, "on", "Projectiles in flight: arrows, shells, rockets and beams, on or off.", false),
    ("hud-scale", 1, "", "HUD scale, 25–200 percent; blank uses automatic scaling.", false),
    ("no-chrome", 1, "false", "Open the game window without the desktop title bar and borders (default: with them).", true),
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
    ("middle-drag", 2, "turn", "Middle drag: turn the view (Shift pans) or pan (Shift turns), at every level.", false),
    ("invert-horizontal-turn", 2, "false", "Turning drags right swing the camera right (the scene against the pointer).", true),
    ("invert-vertical-turn", 2, "false", "Turning drags up raise the camera (the scene against the pointer).", true),
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
    ("script", 0, "", "A setup script: a rich start, a big battle, or your own file.", false),
    ("host", 5, "", "Play and let others join: the port to listen on (1–65535, usually 7400). With --serve, the server's port (default 7400). Blank: standalone.", false),
    ("remote-players", 5, "1", "Other people's seats in a hosted or served match, 1–8.", false),
    ("connect", 5, "", "Join the server at this http:// or https:// address; needs a key or a token. Excludes host and serve.", false),
    ("key", 5, "", "The key that admits you to the server you connect to.", false),
    ("token", 5, "", "The seat token the server gave you, to take the same seat again.", false),
    ("name", 5, "", "The name the other players see.", false),
    ("serve", 5, "false", "Run a server without a game window (a status window).", true),
    ("bind", 5, "127.0.0.1", "The address a hosted or served match listens on; 0.0.0.0 lets the whole network in.", false),
    ("on-disconnect", 5, "pause-then-computer", "When a person drops: pause-then-computer, idle (units stand idle) or computer (at once).", false),
    ("wait-seconds", 5, "120", "How long a paused match waits for a dropped person, 1–86400 seconds.", false),
    ("autosave-minutes", 5, "5", "Server autosave interval in game minutes; 0 turns it off.", false),
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

/// Built-in owner workflows, the advanced launcher's carousel: name, one line and picture.
/// Task 112 folded "Play (single planet)" into Play on the Planet style (one planet in a
/// universe); `--single-planet` still parses for scripts and tests.
pub(super) const PRESETS: [(&str, &str, &str); 6] = [
    (
        "Play",
        "A universe match on a cluster of stars.",
        "maps/map.many_worlds",
    ),
    (
        "Play on one planet",
        "The Planet style: every player on one world.",
        "maps/map.one_world",
    ),
    ("Demo", "The scripted demo on one planet.", "tiles/demo"),
    (
        "Model gallery",
        "Every model of the game, side by side.",
        "tiles/gallery",
    ),
    ("One model", "One model from four sides.", "tiles/model"),
    (
        "Bench",
        "Ten seconds of frame times on one planet.",
        "tiles/custom",
    ),
];

/// Fills a workflow from parser defaults, the rules' one-planet style (`planet`: one planet
/// every player shares) and their first composed model.
pub(super) fn preset(index: usize, first_model: Option<&str>, planet: &str) -> Form {
    let mut form = Form::default();
    let pairs: &[(&str, &str)] = match index {
        0 => &[("universe", "true")],
        1 => &[("universe", "true"), ("style", planet)],
        2 => &[("universe", "true"), ("style", planet), ("demo", "true")],
        3 => &[("gallery", "all")],
        4 => &[("model", first_model.unwrap_or(""))],
        _ => &[("universe", "true"), ("style", planet), ("bench", "10")],
    };
    for &(key, value) in pairs {
        form.set(key, value);
    }
    form
}

/// Options the launcher's form no longer shows (task 112): it always starts a universe match
/// with the chosen style. The flags still parse on the command line.
pub(super) const HIDDEN: [&str; 3] = ["sphere", "single-planet", "universe"];

impl Form {
    /// The form as the launcher starts it: a universe match (unless it reviews models), never
    /// an isolated planet or a flat map.
    pub fn universe_match(mut self) -> Self {
        self.set("single-planet", "false");
        self.set("sphere", "");
        let review = ["gallery", "model", "gallery-manifest"]
            .iter()
            .any(|flag| !self.get(flag).is_empty());
        self.set("universe", if review { "false" } else { "true" });
        self
    }
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
    /// The simple launcher's fields.
    pub simple: super::simple::Simple,
    /// Seat tokens by server address, kept by network play for the Join form's Rejoin.
    pub tokens: BTreeMap<String, String>,
    /// Owner-defined reusable forms, sorted by name.
    pub presets: BTreeMap<String, Form>,
    /// The person's own match presets (task 112), shown as tiles after the bundled ones.
    pub play_presets: Vec<super::presets::Preset>,
    /// The name of the preset tile chosen on the home screen; Play starts it.
    pub chosen: String,
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
            simple: super::simple::Simple::from_json(&value["simple"]),
            tokens: value["tokens"]
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(url, token)| Some((url.clone(), token.as_str()?.to_owned())))
                .collect(),
            presets: value["presets"]
                .as_object()
                .into_iter()
                .flatten()
                .take(12)
                .map(|(name, form)| (name.clone(), Form::from_json(form)))
                .collect(),
            play_presets: value["play_presets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(super::presets::Preset::from_value)
                .take(48)
                .collect(),
            chosen: value["chosen"].as_str().unwrap_or_default().to_owned(),
        })
    }

    /// Replaces the configuration atomically within its directory. Seat tokens a game wrote
    /// meanwhile are kept: the file's tokens win over this copy's.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.write(path, false)
    }

    /// Like [`Config::save`], but this copy's seat tokens win (a game remembering a new one).
    pub fn save_tokens(&self, path: &Path) -> Result<(), String> {
        self.write(path, true)
    }

    fn write(&self, path: &Path, own_tokens_win: bool) -> Result<(), String> {
        let mut tokens = self.tokens.clone();
        if let Ok(on_disk) = Self::load(path) {
            for (url, token) in on_disk.tokens {
                if own_tokens_win {
                    tokens.entry(url).or_insert(token);
                } else {
                    tokens.insert(url, token);
                }
            }
        }
        let value = json!({
            "form": self.form.values,
            "simple": self.simple.values(),
            "tokens": tokens,
            "presets": self.presets.iter().map(|(name, form)| (name, &form.values)).collect::<BTreeMap<_, _>>(),
            "play_presets": self.play_presets.iter().map(super::presets::Preset::json).collect::<Vec<_>>(),
            "chosen": self.chosen,
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
        for (index, (name, _, _)) in PRESETS.iter().enumerate() {
            let form = preset(index, models.first_model(), "map.one_world");
            let mut expected = args::Options::default();
            match index {
                0 => expected.universe = true,
                1 => {
                    expected.universe = true;
                    expected.style = Some("map.one_world".into());
                }
                2 => {
                    expected.universe = true;
                    expected.style = Some("map.one_world".into());
                    expected.demo = true;
                }
                3 => expected.gallery = Some("all".into()),
                4 => expected.model = models.first_model().map(str::to_owned),
                _ => {
                    expected.universe = true;
                    expected.style = Some("map.one_world".into());
                    expected.bench_seconds = Some(10.0);
                }
            }
            assert_eq!(form.validate(), Ok(expected), "{name}");
            // The launcher starts the same form as a universe match: unchanged.
            assert_eq!(
                form.clone().universe_match().validate(),
                form.validate(),
                "{name}"
            );
        }
        // An old form with an isolated planet or a flat map becomes a universe match.
        let mut old = Form::default();
        old.set("single-planet", "true");
        old.set("sphere", "96");
        let options = old.universe_match().validate().unwrap();
        assert!(options.universe && !options.single_planet && options.sphere.is_none());
    }

    #[test]
    fn every_non_default_value_round_trips() {
        let values = [
            ("sphere", "128"),
            ("map-size", "300x200"),
            ("universe", "true"),
            ("style", "test.style"),
            ("start-placement", "together"),
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
            ("grass", "off"),
            ("detail", "ultra"),
            ("projectiles", "off"),
            ("hud-scale", "85"),
            ("no-chrome", "true"),
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
            ("middle-drag", "pan"),
            ("invert-horizontal-turn", "true"),
            ("invert-vertical-turn", "true"),
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
            ("remote-players", "3"),
            ("connect", "https://play.example.org/ascension/"),
            ("key", "k-1"),
            ("token", "t-2"),
            ("name", "Ana"),
            ("bind", "0.0.0.0"),
            ("on-disconnect", "idle"),
            ("wait-seconds", "30"),
            ("autosave-minutes", "9"),
        ];
        let mut form = Form::default();
        // Not listed: single-planet (excludes universe), host and serve (exclude connect).
        assert_eq!(values.len() + 3, CONTROLS.len());
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
        // The server flags, apart from the client's.
        let mut server = Form::default();
        server.set("serve", "true");
        server.set("host", "7401");
        server.set("remote-players", "2");
        let options = server.validate().unwrap();
        assert!(options.serve);
        assert_eq!((options.host, options.remote_players), (Some(7401), 2));
        server.set("connect", "http://127.0.0.1:7400/");
        server.set("key", "k");
        assert!(server.validate().is_err());
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
        assert_eq!(CONTROLS.len(), 60);
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
            form: preset(2, None, "map.one_world"),
            ..Default::default()
        };
        config.form.set("screenshot", "a path's image.png");
        config.simple.set("seed", "31");
        config.simple.set("url", "https://play.example.org/");
        config
            .tokens
            .insert("https://play.example.org/".into(), "tok".into());
        config.form.set("host", "7401");
        config.form.set("on-disconnect", "computer");
        config.presets.insert("My demo".into(), config.form.clone());
        config.play_presets.push(super::super::presets::Preset::of(
            &config.simple,
            "Mine",
            "A line",
        ));
        config.chosen = "Mine".into();
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded.form, config.form);
        assert_eq!(loaded.presets, config.presets);
        assert_eq!(loaded.simple, config.simple);
        assert_eq!(loaded.tokens, config.tokens);
        assert_eq!(loaded.play_presets, config.play_presets);
        assert_eq!(loaded.chosen, "Mine");
        assert_eq!(loaded.form.validate().unwrap().host, Some(7401));
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

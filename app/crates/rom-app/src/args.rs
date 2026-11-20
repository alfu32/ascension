//! Command-line options.

use std::path::PathBuf;

/// Usage text printed for `--help` and after argument errors.
pub const USAGE: &str = "usage: rise-of-man [--bench <seconds>] [--screenshot <file.png>] \
[--size <width>x<height>] [--scale <percent>] [--style <key>] [--seed <n>] [--players <n>] [--factions <key,key,...>] [--ai <profile:difficulty,...|none>] [--watch] \
[--map-size <width>x<height>] [--sphere <n>] [--single-planet] [--orbit <lon_deg>,<lat_deg>,<distance>,<heading_deg>] [--camera <x>,<y>,<distance>,<yaw_deg>,<pitch_deg>] [--boxes <n>] [--demo] \
[--universe] [--start-placement <separate|together>] [--zoom-all] [--zoom-out] [--age <ordinal>] [--view <surface|globe|system|starmap>[,home]] [--speed <percent>] [--stress-units <n>] [--after <seconds>] [--start-markers] [--no-chrome] [--no-hud] [--hud-scale <percent>] [--stats] \
[--sun <orbit|fixed|midnight>] [--graticule <on|off>] [--grass <on|off>] [--projectiles <on|off>] [--look-at <terrain>[+<terrain>][,<distance>]] \
[--middle-drag <turn|pan>] [--invert-horizontal-turn] [--invert-vertical-turn] \
[--gallery <all|units|age|entity key|model id>] [--gallery-view <game|front|side|top|close>] [--team <n>] \
[--model <model id>] [--parts] [--pose <idle|move|attack|work>] [--anim-time <seconds>] \
[--gallery-manifest <file.json>] \
[--host <port>] [--remote-players <n>] [--connect <http(s)://url>] [--key <key>] [--token <token>] [--name <name>] [--serve] [--bind <address>] \
[--on-disconnect <pause-then-computer|idle|computer>] [--wait-seconds <n>] [--autosave-minutes <n>]\n\n--style takes a map style key: map.one_world (Planet), map.solar (Solar), map.many_worlds (Cluster), map.sector (Sector), map.quadrant (Quadrant) or map.galaxy (Galaxy).\n--sphere N builds a universe with home face size N (64..=512).\n--single-planet preserves an isolated sphere (16..=512; default 192).\n\nNetwork (task 87; play arrives with task 88): --host <port> plays and lets --remote-players others join; --connect <url> joins a server with --key (or a remembered --token); --serve runs a server without a game window on --bind <address> and the --host port (default 7400).";

/// The workspace version, shown in titles, menus and the HUD.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The title of the game window: the game, its subtitle and the version.
pub fn window_title() -> String {
    format!("Ascension — Rise of Man {VERSION}")
}

/// What a network match does when a person's connection drops (`--on-disconnect`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisconnectPolicy {
    /// Pause while waiting, then a computer player holds the seat until the person returns.
    #[default]
    PauseThenComputer,
    /// The match keeps going and the person's units stand idle.
    Idle,
    /// A computer player takes the seat at once.
    Computer,
}

impl DisconnectPolicy {
    /// The command-line spelling.
    pub fn name(self) -> &'static str {
        match self {
            Self::PauseThenComputer => "pause-then-computer",
            Self::Idle => "idle",
            Self::Computer => "computer",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::PauseThenComputer, Self::Idle, Self::Computer]
            .into_iter()
            .find(|policy| policy.name() == name)
    }
}

/// Default canvas size in pixels when `--size` is not given.
pub const DEFAULT_SIZE: (u32, u32) = (1280, 800);

/// Game minutes between autosaves unless `--autosave` says otherwise.
pub const DEFAULT_AUTOSAVE_MINUTES: u32 = 5;

/// Parsed command line.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Run the benchmark for this many seconds, print one line and exit.
    pub bench_seconds: Option<f64>,
    /// Canvas size in physical pixels.
    pub size: (u32, u32),
    /// Internal render scale in percent (50–100).
    pub scale: u32,
    /// Write the frame to this PNG file once the terrain is shown, then exit.
    pub screenshot: Option<PathBuf>,
    /// Map style key; `None` picks the first style in id order.
    pub style: Option<String>,
    /// Where players start in a universe; `None` takes the rules' default.
    pub start_placement: Option<rom_mapgen::StartPlacement>,
    /// Map seed.
    pub seed: u64,
    /// Number of players.
    pub players: u32,
    /// Faction keys, one per player; `None` picks the default mix of races.
    pub factions: Option<Vec<String>>,
    /// Computer players for players 2, 3, …: `profile:difficulty` each; missing slots play
    /// balanced:moderate. `--ai none` leaves the other players without one.
    pub ai: Vec<crate::computer::Spec>,
    /// No computer players (`--ai none`).
    pub no_ai: bool,
    /// Player one is a computer player too, so a match can be watched; `--ai` then starts
    /// with player one's choice.
    pub watch: bool,
    /// Map size in tiles.
    pub map_size: (u32, u32),
    /// Home planet face size in a universe (64..=512), or a single sphere (16..=512).
    pub sphere: Option<u32>,
    /// Keep a single sphere for benchmarks, screenshots and surface tests.
    pub single_planet: bool,
    /// Orbit camera placement on a sphere: longitude, latitude, distance and heading.
    pub orbit: Option<[f32; 4]>,
    /// Camera placement: target x and y in tiles, distance, yaw and pitch in degrees.
    pub camera: Option<[f32; 5]>,
    /// Number of moving stress-test boxes.
    pub boxes: u32,
    /// Run the scripted demo driver on the simulation thread.
    pub demo: bool,
    /// Simulation speed in percent of the normal 20 ticks per second.
    pub speed_pct: u32,
    /// Extra wandering units spawned at the start.
    pub stress_units: u32,
    /// Seconds to wait after the first frame before a screenshot is taken.
    pub after_seconds: f64,
    /// Draw the player-start pillars.
    pub start_markers: bool,
    /// Hide the match HUD.
    pub no_hud: bool,
    /// Override automatic HUD scaling (25 to 200 percent of design pixels).
    pub hud_scale: Option<u32>,
    /// Show diagnostic frame and simulation counters.
    pub stats: bool,
    /// Replay pick events from this file on the simulation thread (hidden option).
    pub script: Option<PathBuf>,
    /// Generate a universe with the map style's default home size (also implied by `sphere`).
    pub universe: bool,
    /// Lift the zoom cap by age (testing).
    pub zoom_all: bool,
    /// Start in this level over the home planet.
    pub view: Option<rom_render::Level>,
    /// Keep the light that represents the sun fixed over the first view instead of letting it
    /// orbit the planet orb.
    pub sun_fixed: bool,
    /// With `sun_fixed`, put the light on the far side, so the first view is at midnight.
    pub sun_midnight: bool,
    /// Zoom out to the farthest the player's age allows at the start.
    pub zoom_out: bool,
    /// Start every player in the age with this ordinal (testing).
    pub age: Option<i64>,
    /// Draw the latitude and longitude lines of a sphere planet.
    pub graticule: bool,
    /// Draw grass blades on the ground near the camera.
    pub grass: bool,
    /// Draw projectiles in flight (task 95).
    pub projectiles: bool,
    /// The middle button pans and Shift + middle turns (`--middle-drag pan`); by default the
    /// middle button turns the view at every level.
    pub middle_drag_pans: bool,
    /// Dragging right swings the camera right instead of making the scene follow the pointer.
    pub invert_horizontal_turn: bool,
    /// Dragging up raises the camera instead of making the scene follow the pointer.
    pub invert_vertical_turn: bool,
    /// On a sphere, start the camera over the nearest edge tile of a terrain (optionally one
    /// near a second terrain, which the camera then faces), at this distance.
    pub look_at: Option<LookAt>,
    /// Show the models this filter selects in a gallery instead of a match.
    pub gallery: Option<String>,
    /// Camera of the gallery.
    pub gallery_view: GalleryView,
    /// Owner whose colour the gallery's models wear (1 to 8).
    pub team: u8,
    /// Show one model from four sides instead of a match.
    pub model: Option<String>,
    /// With `--model`, label each part and outline it.
    pub parts: bool,
    /// Write the model manifest to this JSON file and exit (no window).
    pub gallery_manifest: Option<PathBuf>,
    /// What the gallery's units are doing.
    pub pose: rom_render::Activity,
    /// Seconds of motion the gallery shows, the same for every model; `None` lets it run.
    pub anim_time: Option<f32>,
    /// Load this save instead of generating a match (hidden option; the save names the map).
    pub load: Option<PathBuf>,
    /// Autosave every this many game minutes; 0 turns it off (hidden option).
    pub autosave_minutes: u32,
    /// Open the window without the desktop's title bar and borders.
    pub no_chrome: bool,
    /// Play and let others join, listening on this port (`--host`); with `--serve`, the
    /// server's port.
    pub host: Option<u16>,
    /// Other people's seats in a hosted or served match (1 to 8).
    pub remote_players: u32,
    /// Join the server at this `http://` or `https://` address.
    pub connect: Option<String>,
    /// The key that admits this person to the server.
    pub key: Option<String>,
    /// The seat token a server gave, to take the same seat again.
    pub token: Option<String>,
    /// The name shown to the other players.
    pub name: Option<String>,
    /// Run a server without a game window.
    pub serve: bool,
    /// The address a hosted or served match listens on.
    pub bind: String,
    /// What happens when a person drops.
    pub on_disconnect: DisconnectPolicy,
    /// Seconds a paused match waits for a dropped person before the policy acts.
    pub wait_seconds: u32,
    /// Print the version and exit.
    pub version: bool,
    /// Print usage and exit.
    pub help: bool,
}

/// The camera of the model gallery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GalleryView {
    /// The gameplay tilt.
    #[default]
    Game,
    /// Low, from the south.
    Front,
    /// Low, from the east.
    Side,
    /// Straight down.
    Top,
    /// The gameplay tilt, closer.
    Close,
}

impl GalleryView {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "game" => Self::Game,
            "front" => Self::Front,
            "side" => Self::Side,
            "top" => Self::Top,
            "close" => Self::Close,
            _ => return None,
        })
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            bench_seconds: None,
            size: DEFAULT_SIZE,
            scale: 100,
            screenshot: None,
            style: None,
            start_placement: None,
            seed: 1,
            players: 4,
            factions: None,
            ai: Vec::new(),
            no_ai: false,
            watch: false,
            map_size: (256, 256),
            sphere: None,
            single_planet: false,
            orbit: None,
            camera: None,
            boxes: 0,
            demo: false,
            speed_pct: 100,
            stress_units: 0,
            after_seconds: 1.0,
            start_markers: false,
            no_hud: false,
            hud_scale: None,
            stats: false,
            script: None,
            universe: false,
            zoom_all: false,
            view: None,
            sun_fixed: false,
            sun_midnight: false,
            zoom_out: false,
            age: None,
            graticule: true,
            grass: true,
            projectiles: true,
            middle_drag_pans: false,
            invert_horizontal_turn: false,
            invert_vertical_turn: false,
            look_at: None,
            gallery: None,
            gallery_view: GalleryView::Game,
            team: 1,
            model: None,
            parts: false,
            gallery_manifest: None,
            pose: rom_render::Activity::Idle,
            anim_time: None,
            load: None,
            autosave_minutes: DEFAULT_AUTOSAVE_MINUTES,
            no_chrome: false,
            host: None,
            remote_players: 1,
            connect: None,
            key: None,
            token: None,
            name: None,
            serve: false,
            bind: "127.0.0.1".into(),
            on_disconnect: DisconnectPolicy::default(),
            wait_seconds: 120,
            version: false,
            help: false,
        }
    }
}

impl Options {
    /// Sphere games include the universe unless a single planet was explicitly requested.
    pub fn uses_universe(&self) -> bool {
        !self.single_planet && (self.universe || self.sphere.is_some())
    }

    /// The content these options ask for, as the match core takes it.
    pub fn content_request(&self) -> rom_match::ContentRequest {
        rom_match::ContentRequest {
            style: self.style.clone(),
            factions: self.factions.clone(),
            players: self.players,
            seed: self.seed,
            map_size: self.map_size,
            home_size: self.sphere,
            start_placement: self.start_placement,
        }
    }

    /// The whole match these options set up (with a script's setup lines), as the match core
    /// builds it: the person plays player one unless `--watch`, every other player is a computer
    /// unless `--ai none`.
    pub fn match_settings(&self, script: rom_match::ScriptSetup) -> rom_match::MatchSettings {
        let map = if self.uses_universe() {
            rom_match::MapKind::Universe
        } else {
            match self.sphere.or(self.single_planet.then_some(192)) {
                Some(n) => rom_match::MapKind::Sphere(n),
                None => rom_match::MapKind::Flat,
            }
        };
        rom_match::MatchSettings {
            content: self.content_request(),
            map,
            age: self.age,
            script,
            ai: (!self.no_ai).then(|| self.ai.clone()),
            humans: if self.watch {
                Vec::new()
            } else {
                vec![crate::selection::PLAYER]
            },
        }
    }
}

/// Parses the arguments after the program name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            // Lets the launcher start a default game without reopening itself.
            "--run" => {}
            "--bench" => {
                let text = value("--bench")?;
                let seconds = text
                    .parse::<f64>()
                    .ok()
                    .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
                    .ok_or_else(|| format!("--bench expects a positive number, got {text:?}"))?;
                options.bench_seconds = Some(seconds);
            }
            "--size" => options.size = parse_size(&value("--size")?)?,
            "--scale" => {
                let text = value("--scale")?;
                options.scale = text
                    .parse::<u32>()
                    .ok()
                    .filter(|scale| (50..=100).contains(scale))
                    .ok_or_else(|| format!("--scale expects 50 to 100, got {text:?}"))?;
            }
            "--screenshot" => options.screenshot = Some(PathBuf::from(value("--screenshot")?)),
            "--style" => options.style = Some(value("--style")?),
            "--start-placement" => {
                let text = value("--start-placement")?;
                options.start_placement =
                    Some(rom_mapgen::StartPlacement::parse(&text).ok_or_else(|| {
                        format!("--start-placement expects separate or together, got {text:?}")
                    })?);
            }
            "--seed" => options.seed = number("--seed", &value("--seed")?)?,
            "--players" => {
                let players: u32 = number("--players", &value("--players")?)?;
                if players == 0 {
                    return Err("--players expects at least 1".into());
                }
                options.players = players;
            }
            "--factions" => {
                let keys: Vec<String> = value("--factions")?
                    .split(',')
                    .map(|key| key.trim().to_owned())
                    .collect();
                if keys.iter().any(String::is_empty) {
                    return Err("--factions expects keys separated by commas".into());
                }
                options.players = keys.len() as u32;
                options.factions = Some(keys);
            }
            "--ai" => {
                let text = value("--ai")?;
                options.no_ai = text == "none";
                options.ai = if options.no_ai {
                    Vec::new()
                } else {
                    text.split(',')
                        .map(|spec| crate::computer::Spec::parse(spec.trim()))
                        .collect::<Result<_, _>>()
                        .map_err(|e| format!("--ai: {e}"))?
                };
            }
            "--map-size" => options.map_size = parse_map_size(&value("--map-size")?)?,
            "--sphere" => {
                let text = value("--sphere")?;
                options.sphere = Some(
                    text.parse::<u32>()
                        .ok()
                        .filter(|n| (16..=512).contains(n))
                        .ok_or_else(|| format!("--sphere expects 16 to 512, got {text:?}"))?,
                );
            }
            "--orbit" => options.orbit = Some(parse_orbit(&value("--orbit")?)?),
            "--camera" => options.camera = Some(parse_camera(&value("--camera")?)?),
            "--boxes" => options.boxes = number("--boxes", &value("--boxes")?)?,
            "--demo" => options.demo = true,
            "--watch" => options.watch = true,
            "--universe" => options.universe = true,
            "--single-planet" => options.single_planet = true,
            "--zoom-all" => options.zoom_all = true,
            "--zoom-out" => options.zoom_out = true,
            "--age" => {
                let text = value("--age")?;
                options.age = Some(
                    text.parse::<i64>()
                        .ok()
                        .filter(|n| *n >= 0)
                        .ok_or_else(|| format!("--age expects an age ordinal, got {text:?}"))?,
                );
            }
            "--view" => {
                let text = value("--view")?;
                let invalid = || format!("--view expects <level>[,home], got {text:?}");
                let (level, target) = text.split_once(',').unwrap_or((&text, "home"));
                if !target.eq_ignore_ascii_case("home") {
                    return Err(invalid());
                }
                options.view = Some(rom_render::Level::from_name(level).ok_or_else(invalid)?);
            }
            "--sun" => {
                let text = value("--sun")?;
                (options.sun_fixed, options.sun_midnight) = match text.to_ascii_lowercase().as_str()
                {
                    "orbit" => (false, false),
                    "fixed" => (true, false),
                    "night" | "midnight" => (true, true),
                    _ => {
                        return Err(format!(
                            "--sun expects orbit, fixed or midnight, got {text:?}"
                        ));
                    }
                };
            }
            "--graticule" => {
                let text = value("--graticule")?;
                options.graticule = match text.to_ascii_lowercase().as_str() {
                    "on" => true,
                    "off" => false,
                    _ => return Err(format!("--graticule expects on or off, got {text:?}")),
                };
            }
            "--grass" => {
                let text = value("--grass")?;
                options.grass = match text.to_ascii_lowercase().as_str() {
                    "on" => true,
                    "off" => false,
                    _ => return Err(format!("--grass expects on or off, got {text:?}")),
                };
            }
            "--projectiles" => {
                let text = value("--projectiles")?;
                options.projectiles = match text.to_ascii_lowercase().as_str() {
                    "on" => true,
                    "off" => false,
                    _ => return Err(format!("--projectiles expects on or off, got {text:?}")),
                };
            }
            "--middle-drag" => {
                let text = value("--middle-drag")?;
                options.middle_drag_pans = match text.to_ascii_lowercase().as_str() {
                    "turn" => false,
                    "pan" => true,
                    _ => return Err(format!("--middle-drag expects turn or pan, got {text:?}")),
                };
            }
            "--invert-horizontal-turn" => options.invert_horizontal_turn = true,
            "--invert-vertical-turn" => options.invert_vertical_turn = true,
            "--look-at" => options.look_at = Some(parse_look_at(&value("--look-at")?)?),
            "--no-hud" => options.no_hud = true,
            "--no-chrome" => options.no_chrome = true,
            "--version" | "-V" => options.version = true,
            "--stats" => options.stats = true,
            "--hud-scale" => {
                let text = value("--hud-scale")?;
                options.hud_scale = Some(
                    text.parse::<u32>()
                        .ok()
                        .filter(|p| (25..=200).contains(p))
                        .ok_or_else(|| format!("--hud-scale expects 25 to 200, got {text:?}"))?,
                );
            }
            "--start-markers" => options.start_markers = true,
            "--script" => options.script = Some(PathBuf::from(value("--script")?)),
            "--speed" => {
                let text = value("--speed")?;
                options.speed_pct = text
                    .parse::<u32>()
                    .ok()
                    .filter(|speed| (1..=10_000).contains(speed))
                    .ok_or_else(|| format!("--speed expects 1 to 10000, got {text:?}"))?;
            }
            "--stress-units" => {
                options.stress_units = number("--stress-units", &value("--stress-units")?)?;
            }
            "--after" => {
                let text = value("--after")?;
                options.after_seconds = text
                    .parse::<f64>()
                    .ok()
                    .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
                    .ok_or_else(|| format!("--after expects seconds, got {text:?}"))?;
            }
            "--gallery" => options.gallery = Some(value("--gallery")?),
            "--gallery-view" => {
                let text = value("--gallery-view")?;
                options.gallery_view = GalleryView::from_name(&text).ok_or_else(|| {
                    format!("--gallery-view expects game, front, side, top or close, got {text:?}")
                })?;
            }
            "--team" => {
                let text = value("--team")?;
                options.team = text
                    .parse::<u8>()
                    .ok()
                    .filter(|t| (1..=8).contains(t))
                    .ok_or_else(|| format!("--team expects 1 to 8, got {text:?}"))?;
            }
            "--model" => options.model = Some(value("--model")?),
            "--parts" => options.parts = true,
            "--gallery-manifest" => {
                options.gallery_manifest = Some(PathBuf::from(value("--gallery-manifest")?));
            }
            "--pose" => {
                let text = value("--pose")?;
                options.pose = rom_render::Activity::from_name(&text).ok_or_else(|| {
                    format!("--pose expects idle, move, attack or work, got {text:?}")
                })?;
            }
            "--anim-time" => {
                let text = value("--anim-time")?;
                options.anim_time = Some(
                    text.parse::<f32>()
                        .ok()
                        .filter(|t| t.is_finite() && *t >= 0.0)
                        .ok_or_else(|| format!("--anim-time expects seconds, got {text:?}"))?,
                );
            }
            "--load" => options.load = Some(PathBuf::from(value("--load")?)),
            "--autosave" | "--autosave-minutes" => {
                options.autosave_minutes = number(&arg, &value(&arg)?)?;
            }
            "--host" => {
                let text = value("--host")?;
                options.host = Some(
                    text.parse::<u16>()
                        .ok()
                        .filter(|port| *port > 0)
                        .ok_or_else(|| {
                            format!("--host expects a port, 1 to 65535, got {text:?}")
                        })?,
                );
            }
            "--remote-players" => {
                let text = value("--remote-players")?;
                options.remote_players =
                    text.parse::<u32>()
                        .ok()
                        .filter(|n| (1..=8).contains(n))
                        .ok_or_else(|| format!("--remote-players expects 1 to 8, got {text:?}"))?;
            }
            "--connect" => {
                let text = value("--connect")?;
                let rest = text
                    .strip_prefix("http://")
                    .or_else(|| text.strip_prefix("https://"))
                    .filter(|rest| !rest.is_empty() && !rest.starts_with('/'));
                if rest.is_none() || text.contains(char::is_whitespace) {
                    return Err(format!(
                        "--connect expects an http:// or https:// address, got {text:?}"
                    ));
                }
                options.connect = Some(text);
            }
            "--key" => options.key = Some(non_empty("--key", value("--key")?)?),
            "--token" => options.token = Some(non_empty("--token", value("--token")?)?),
            "--name" => options.name = Some(non_empty("--name", value("--name")?)?),
            "--serve" => options.serve = true,
            "--bind" => options.bind = non_empty("--bind", value("--bind")?)?,
            "--on-disconnect" => {
                let text = value("--on-disconnect")?;
                options.on_disconnect = DisconnectPolicy::from_name(&text).ok_or_else(|| {
                    format!(
                        "--on-disconnect expects pause-then-computer, idle or computer, got {text:?}"
                    )
                })?;
            }
            "--wait-seconds" => {
                let text = value("--wait-seconds")?;
                options.wait_seconds = text
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=86_400).contains(n))
                    .ok_or_else(|| format!("--wait-seconds expects 1 to 86400, got {text:?}"))?;
            }
            "-h" | "--help" => options.help = true,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    if options.single_planet && options.universe {
        return Err("--single-planet and --universe are mutually exclusive".into());
    }
    if options.uses_universe() && options.sphere.is_some_and(|n| n < 64) {
        return Err("universe home planets require --sphere 64 to 512; use --single-planet for smaller spheres".into());
    }
    if options.connect.is_some() && (options.serve || options.host.is_some()) {
        return Err(
            "--connect joins a server; it cannot be combined with --host or --serve".into(),
        );
    }
    if options.connect.is_some() && options.key.is_none() && options.token.is_none() {
        return Err("--connect needs --key (or --token to take a seat again)".into());
    }
    Ok(options)
}

/// The arguments that decide which map and match a save was made from, in the order given:
/// what a save keeps so loading can regenerate the same terrain, planets and looks. Window,
/// camera and automation options are left out.
pub fn match_arguments(arguments: &[String]) -> Vec<String> {
    const WITH_VALUE: [&str; 10] = [
        "--style",
        "--start-placement",
        "--seed",
        "--players",
        "--factions",
        "--ai",
        "--map-size",
        "--sphere",
        "--age",
        "--sun",
    ];
    const FLAGS: [&str; 3] = ["--single-planet", "--universe", "--watch"];
    let mut kept = Vec::new();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        if WITH_VALUE.contains(&argument.as_str()) {
            if let Some(value) = rest.next() {
                kept.extend([argument.clone(), value.clone()]);
            }
        } else if FLAGS.contains(&argument.as_str()) {
            kept.push(argument.clone());
        } else if !argument.starts_with("--") {
            continue;
        } else if ![
            "--run",
            "--demo",
            "--stats",
            "--no-hud",
            "--no-chrome",
            "--version",
            "--zoom-out",
            "--zoom-all",
            "--invert-horizontal-turn",
            "--invert-vertical-turn",
            "--start-markers",
            "--parts",
            "--serve",
            "--help",
            "-h",
        ]
        .contains(&argument.as_str())
        {
            // Every other option takes a value: skip it.
            rest.next();
        }
    }
    kept
}

fn non_empty(name: &str, text: String) -> Result<String, String> {
    if text.trim().is_empty() {
        Err(format!("{name} expects a value, not an empty one"))
    } else {
        Ok(text)
    }
}

fn number<T: std::str::FromStr>(name: &str, text: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("{name} expects a whole number, got {text:?}"))
}

fn parse_map_size(text: &str) -> Result<(u32, u32), String> {
    let invalid = || format!("--map-size expects <width>x<height>, got {text:?}");
    let (width, height) = text.split_once(['x', 'X']).ok_or_else(invalid)?;
    let extent = |part: &str| {
        part.parse::<u32>()
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(invalid)
    };
    Ok((extent(width)?, extent(height)?))
}

fn parse_camera(text: &str) -> Result<[f32; 5], String> {
    let invalid = || format!("--camera expects x,y,distance,yaw_deg,pitch_deg, got {text:?}");
    let parts: Vec<f32> = text
        .split(',')
        .map(|part| part.trim().parse::<f32>().ok().filter(|v| v.is_finite()))
        .collect::<Option<_>>()
        .ok_or_else(invalid)?;
    parts.try_into().map_err(|_| invalid())
}

fn parse_orbit(text: &str) -> Result<[f32; 4], String> {
    let invalid = || format!("--orbit expects lon_deg,lat_deg,distance,heading_deg, got {text:?}");
    let parts: Vec<f32> = text
        .split(',')
        .map(|part| part.trim().parse::<f32>().ok().filter(|v| v.is_finite()))
        .collect::<Option<_>>()
        .ok_or_else(invalid)?;
    let [lon, lat, distance, heading]: [f32; 4] = parts.try_into().map_err(|_| invalid())?;
    if !(-90.0..=90.0).contains(&lat) || distance <= 0.0 {
        return Err(invalid());
    }
    Ok([lon, lat, distance, heading])
}

/// Where `--look-at` puts the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct LookAt {
    /// Key of the terrain looked at.
    pub terrain: String,
    /// Key of a terrain that must lie near it, in view.
    pub near: Option<String>,
    /// Camera distance in tiles.
    pub distance: f32,
}

fn parse_look_at(text: &str) -> Result<LookAt, String> {
    let invalid = || format!("--look-at expects <terrain>[+<terrain>][,<distance>], got {text:?}");
    let (keys, distance) = match text.split_once(',') {
        Some((keys, distance)) => (
            keys,
            distance
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|d| d.is_finite() && *d > 0.0)
                .ok_or_else(invalid)?,
        ),
        None => (text, rom_render::MIN_DISTANCE),
    };
    let (terrain, near) = match keys.split_once('+') {
        Some((a, b)) => (a.trim(), Some(b.trim().to_owned())),
        None => (keys.trim(), None),
    };
    if terrain.is_empty() || near.as_deref() == Some("") {
        return Err(invalid());
    }
    Ok(LookAt {
        terrain: terrain.to_owned(),
        near,
        distance,
    })
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let invalid = || format!("--size expects <width>x<height>, got {text:?}");
    let (width, height) = text.split_once(['x', 'X']).ok_or_else(invalid)?;
    let extent = |part: &str| {
        part.parse::<u32>()
            .ok()
            .filter(|value| (1..=16_384).contains(value))
            .ok_or_else(invalid)
    };
    Ok((extent(width)?, extent(height)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn parses_network_options() {
        let host = parse(args(&[
            "--host",
            "7401",
            "--remote-players",
            "3",
            "--on-disconnect",
            "idle",
            "--wait-seconds",
            "30",
            "--bind",
            "0.0.0.0",
            "--autosave-minutes",
            "9",
        ]))
        .unwrap();
        assert_eq!(host.host, Some(7401));
        assert_eq!(host.remote_players, 3);
        assert_eq!(host.on_disconnect, DisconnectPolicy::Idle);
        assert_eq!((host.wait_seconds, host.autosave_minutes), (30, 9));
        assert_eq!(host.bind, "0.0.0.0");
        let join = parse(args(&[
            "--connect",
            "https://play.example.org/ascension/",
            "--key",
            "k-1",
            "--name",
            "Ana",
        ]))
        .unwrap();
        assert_eq!(
            join.connect.as_deref(),
            Some("https://play.example.org/ascension/")
        );
        assert_eq!(
            (join.key.as_deref(), join.name.as_deref()),
            (Some("k-1"), Some("Ana"))
        );
        let rejoin = parse(args(&[
            "--connect",
            "http://127.0.0.1:7400/",
            "--token",
            "t",
        ]))
        .unwrap();
        assert_eq!(rejoin.token.as_deref(), Some("t"));
        assert!(parse(args(&["--serve", "--host", "7400"])).unwrap().serve);
        let defaults = Options::default();
        assert_eq!(
            (
                defaults.host,
                defaults.remote_players,
                defaults.serve,
                defaults.bind.as_str()
            ),
            (None, 1, false, "127.0.0.1")
        );
        assert_eq!(defaults.on_disconnect, DisconnectPolicy::PauseThenComputer);
        assert_eq!(defaults.wait_seconds, 120);
    }

    #[test]
    fn rejects_bad_network_options() {
        for bad in [
            &["--host", "0"][..],
            &["--host", "70000"],
            &["--remote-players", "0"],
            &["--remote-players", "9"],
            &["--connect", "ws://x/", "--key", "k"],
            &["--connect", "play.example.org", "--key", "k"],
            &["--connect", "http://", "--key", "k"],
            &["--connect", "http://x", "--key", ""],
            &["--connect", "http://x"],
            &["--connect", "http://x", "--key", "k", "--serve"],
            &["--connect", "http://x", "--key", "k", "--host", "7400"],
            &["--on-disconnect", "ignore"],
            &["--wait-seconds", "0"],
            &["--bind", ""],
        ] {
            assert!(parse(args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn parses_bench_size_and_scale() {
        let options = parse(args(&[
            "--bench",
            "10",
            "--size",
            "1920x1080",
            "--scale",
            "50",
        ]))
        .expect("valid arguments");
        assert_eq!(options.bench_seconds, Some(10.0));
        assert_eq!(options.size, (1920, 1080));
        assert_eq!(options.scale, 50);
    }

    #[test]
    fn parses_map_and_screenshot_options() {
        let options = parse(args(&[
            "--style",
            "map.many_worlds",
            "--seed",
            "7",
            "--players",
            "2",
            "--map-size",
            "384x192",
            "--screenshot",
            "out.png",
            "--camera",
            "192,96,420,0,70",
            "--boxes",
            "100",
        ]))
        .expect("valid arguments");
        assert_eq!(options.style.as_deref(), Some("map.many_worlds"));
        assert_eq!((options.seed, options.players), (7, 2));
        assert_eq!(options.map_size, (384, 192));
        assert_eq!(options.camera, Some([192.0, 96.0, 420.0, 0.0, 70.0]));
        assert_eq!(options.boxes, 100);
        assert_eq!(options.screenshot, Some(PathBuf::from("out.png")));
        assert!(parse(args(&["--camera", "1,2,3"])).is_err());
        assert!(parse(args(&["--map-size", "0x5"])).is_err());
        assert!(parse(args(&["--players", "0"])).is_err());
    }

    #[test]
    fn parses_sphere_and_orbit_options() {
        let o = parse(args(&["--sphere", "192", "--orbit", "0,10,12,30"])).unwrap();
        assert_eq!(o.sphere, Some(192));
        assert!(o.uses_universe());
        assert_eq!(o.orbit, Some([0.0, 10.0, 12.0, 30.0]));
        assert_eq!(parse(args(&[])).unwrap().sphere, None);
        for bad in [["--sphere", "8"], ["--sphere", "x"], ["--orbit", "1,2,3"]] {
            assert!(parse(args(&bad)).is_err(), "{bad:?}");
        }
        assert!(parse(args(&["--orbit", "0,95,12,0"])).is_err());
    }

    #[test]
    fn single_planet_preserves_the_old_sphere_mode() {
        let o = parse(args(&["--sphere", "24", "--single-planet"])).unwrap();
        assert_eq!(o.sphere, Some(24));
        assert!(!o.uses_universe());
        assert!(!parse(args(&["--single-planet"])).unwrap().uses_universe());
        assert!(parse(args(&["--sphere", "24"])).is_err());
        assert!(parse(args(&["--universe", "--single-planet"])).is_err());
        assert!(parse(args(&["--single-planet", "--universe"])).is_err());
        assert!(
            parse(args(&["--universe", "--sphere", "256"]))
                .unwrap()
                .uses_universe()
        );
    }

    #[test]
    fn parses_universe_options() {
        let o = parse(args(&["--universe", "--zoom-all", "--view", "system,home"])).unwrap();
        assert!(o.universe && o.zoom_all);
        assert_eq!(o.view, Some(rom_render::Level::System));
        let o = parse(args(&["--view", "starmap"])).unwrap();
        assert_eq!(o.view, Some(rom_render::Level::StarMap));
        assert!(parse(args(&["--view", "moon"])).is_err());
        assert!(parse(args(&["--view", "globe,away"])).is_err());
    }

    #[test]
    fn parses_sun_and_graticule_options() {
        let defaults = parse(args(&[])).unwrap();
        assert!(!defaults.sun_fixed && defaults.graticule);
        let o = parse(args(&["--sun", "fixed", "--graticule", "off"])).unwrap();
        assert!(o.sun_fixed && !o.graticule);
        assert!(!o.sun_midnight);
        let o = parse(args(&["--sun", "orbit", "--graticule", "on"])).unwrap();
        assert!(!o.sun_fixed && o.graticule);
        let o = parse(args(&["--sun", "night"])).unwrap();
        assert!(o.sun_fixed && o.sun_midnight);
        assert_eq!(parse(args(&["--sun", "midnight"])).unwrap(), o);
        let o = parse(args(&[
            "--middle-drag",
            "pan",
            "--invert-horizontal-turn",
            "--invert-vertical-turn",
        ]))
        .unwrap();
        assert!(o.middle_drag_pans && o.invert_horizontal_turn && o.invert_vertical_turn);
        assert!(!defaults.middle_drag_pans && !defaults.invert_horizontal_turn);
        assert!(
            !parse(args(&["--middle-drag", "turn"]))
                .unwrap()
                .middle_drag_pans
        );
        for bad in [
            ["--sun", "still"],
            ["--graticule", "maybe"],
            ["--middle-drag", "spin"],
        ] {
            assert!(parse(args(&bad)).is_err(), "{bad:?}");
        }
        assert!(USAGE.contains("--sun") && USAGE.contains("--graticule"));
        assert!(defaults.grass && !parse(args(&["--grass", "off"])).unwrap().grass);
        assert!(parse(args(&["--grass", "On"])).unwrap().grass);
        assert!(parse(args(&["--grass", "maybe"])).is_err());
        assert!(defaults.projectiles);
        assert!(!parse(args(&["--projectiles", "off"])).unwrap().projectiles);
        assert!(parse(args(&["--projectiles", "maybe"])).is_err());
    }

    #[test]
    fn parses_factions() {
        let o = parse(args(&["--factions", "nation.romans,nation.ashscale"])).unwrap();
        assert_eq!(o.players, 2);
        assert_eq!(
            o.factions,
            Some(vec![
                "nation.romans".to_owned(),
                "nation.ashscale".to_owned()
            ])
        );
        assert!(parse(args(&["--factions", "a,,b"])).is_err());
    }

    #[test]
    fn parses_computer_players() {
        let o = parse(args(&["--ai", "builder:tough, rusher"])).unwrap();
        assert_eq!(
            o.ai.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            ["builder:tough", "rusher:moderate"]
        );
        assert!(!o.no_ai);
        let o = parse(args(&["--ai", "none"])).unwrap();
        assert!(o.no_ai && o.ai.is_empty());
        assert!(parse(args(&["--watch"])).unwrap().watch);
        assert!(parse(args(&["--ai", "a::b"])).is_err());
        assert!(parse(args(&["--ai", "a,,b"])).is_err());
    }

    #[test]
    fn parses_simulation_options() {
        let options = parse(args(&[
            "--demo",
            "--speed",
            "250",
            "--stress-units",
            "5000",
            "--after",
            "20",
        ]))
        .expect("valid arguments");
        assert!(options.demo);
        assert_eq!(options.speed_pct, 250);
        assert_eq!(options.stress_units, 5000);
        assert_eq!(options.after_seconds, 20.0);
        assert!(parse(args(&["--speed", "0"])).is_err());
        assert!(parse(args(&["--after", "-1"])).is_err());
    }

    #[test]
    fn parses_hud_options() {
        let o = parse(args(&["--no-hud", "--hud-scale", "85", "--stats"])).unwrap();
        assert!(o.no_hud && o.stats);
        assert!(!o.no_chrome && !o.version);
        assert!(parse(args(&["--no-chrome"])).unwrap().no_chrome);
        assert!(parse(args(&["--version"])).unwrap().version);
        assert_eq!(
            window_title(),
            format!("Ascension — Rise of Man {}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(o.hud_scale, Some(85));
        for value in ["0", "201", "bad"] {
            assert!(parse(args(&["--hud-scale", value])).is_err());
        }
    }

    #[test]
    fn parses_the_hidden_script_option() {
        let o = parse(args(&["--script", "demo.txt"])).unwrap();
        assert_eq!(o.script, Some(PathBuf::from("demo.txt")));
        assert!(parse(args(&["--script"])).is_err());
        assert!(!USAGE.contains("--script"));
    }

    #[test]
    fn parses_the_look_at_option() {
        let o = parse(args(&["--look-at", "ter.forest+ter.mountain,14"])).unwrap();
        assert_eq!(
            o.look_at,
            Some(LookAt {
                terrain: "ter.forest".into(),
                near: Some("ter.mountain".into()),
                distance: 14.0,
            })
        );
        let o = parse(args(&["--look-at", "ter.forest"])).unwrap();
        let look = o.look_at.unwrap();
        assert_eq!((look.near, look.distance), (None, rom_render::MIN_DISTANCE));
        assert!(parse(args(&["--look-at", "ter.forest,-3"])).is_err());
        assert!(parse(args(&["--look-at", "ter.forest+"])).is_err());
    }

    #[test]
    fn parses_the_review_options() {
        let options = parse(args(&[
            "--gallery",
            "ancient",
            "--gallery-view",
            "side",
            "--team",
            "3",
            "--gallery-manifest",
            "m.json",
        ]))
        .unwrap();
        assert_eq!(options.gallery.as_deref(), Some("ancient"));
        assert_eq!(options.gallery_view, GalleryView::Side);
        assert_eq!(options.team, 3);
        assert_eq!(options.gallery_manifest, Some(PathBuf::from("m.json")));
        let options = parse(args(&["--model", "bld.city@ancient", "--parts"])).unwrap();
        assert_eq!(options.model.as_deref(), Some("bld.city@ancient"));
        assert!(options.parts);
        assert_eq!(options.pose, rom_render::Activity::Idle);
        assert_eq!(options.anim_time, None);
        let options = parse(args(&["--pose", "attack", "--anim-time", "0.25"])).unwrap();
        assert_eq!(options.pose, rom_render::Activity::Attack);
        assert_eq!(options.anim_time, Some(0.25));
        assert!(parse(args(&["--pose", "dance"])).is_err());
        assert!(parse(args(&["--anim-time", "-1"])).is_err());
        assert!(parse(args(&["--team", "9"])).is_err());
        assert!(parse(args(&["--gallery-view", "under"])).is_err());
    }

    #[test]
    fn defaults_without_arguments() {
        assert_eq!(parse(args(&[])), Ok(Options::default()));
    }

    #[test]
    fn start_placement_parses_and_is_kept_in_a_save() {
        use rom_mapgen::StartPlacement;
        assert_eq!(parse(args(&[])).unwrap().start_placement, None);
        let o = parse(args(&["--start-placement", "together"])).unwrap();
        assert_eq!(o.start_placement, Some(StartPlacement::Together));
        assert!(parse(args(&["--start-placement", "close"])).is_err());
        let given = args(&["--start-placement", "together", "--stats", "--seed", "3"]);
        assert_eq!(
            match_arguments(&given),
            ["--start-placement", "together", "--seed", "3"]
        );
    }

    #[test]
    fn rejects_bad_values() {
        assert!(parse(args(&["--bench", "0"])).is_err());
        assert!(parse(args(&["--size", "1920"])).is_err());
        assert!(parse(args(&["--scale", "40"])).is_err());
        assert!(parse(args(&["--bench"])).is_err());
        assert!(parse(args(&["--fast"])).is_err());
    }

    #[test]
    fn a_save_keeps_only_the_arguments_that_make_the_map() {
        let given: Vec<String> = [
            "--seed",
            "9",
            "--size",
            "900x600",
            "--stats",
            "--sphere",
            "128",
            "--universe",
            "--ai",
            "builder:tough",
            "--screenshot",
            "a.png",
            "--after",
            "2",
            "--watch",
        ]
        .map(String::from)
        .into();
        assert_eq!(
            match_arguments(&given),
            [
                "--seed",
                "9",
                "--sphere",
                "128",
                "--universe",
                "--ai",
                "builder:tough",
                "--watch"
            ]
        );
        let o = parse(args(&["--load", "x.rom", "--autosave", "0"])).unwrap();
        assert_eq!(o.load, Some(PathBuf::from("x.rom")));
        assert_eq!(o.autosave_minutes, 0);
        assert_eq!(
            parse(args(&[])).unwrap().autosave_minutes,
            DEFAULT_AUTOSAVE_MINUTES
        );
    }
}

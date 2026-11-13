//! Command-line options.

use std::path::PathBuf;

/// Usage text printed for `--help` and after argument errors.
pub const USAGE: &str = "usage: rise-of-man [--bench <seconds>] [--screenshot <file.png>] \
[--size <width>x<height>] [--scale <percent>] [--style <key>] [--seed <n>] [--players <n>] [--factions <key,key,...>] \
[--map-size <width>x<height>] [--sphere <n>] [--orbit <lon_deg>,<lat_deg>,<distance>,<heading_deg>] [--camera <x>,<y>,<distance>,<yaw_deg>,<pitch_deg>] [--boxes <n>] [--demo] \
[--universe] [--zoom-all] [--view <surface|globe|system|starmap>[,home]] [--speed <percent>] [--stress-units <n>] [--after <seconds>] [--start-markers] [--no-hud] [--hud-scale <percent>] [--stats] \
[--sun <orbit|fixed|night>] [--graticule <on|off>]";

/// Default canvas size in pixels when `--size` is not given.
pub const DEFAULT_SIZE: (u32, u32) = (1280, 800);

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
    /// Map seed.
    pub seed: u64,
    /// Number of players.
    pub players: u32,
    /// Faction keys, one per player; `None` picks the default mix of races.
    pub factions: Option<Vec<String>>,
    /// Map size in tiles.
    pub map_size: (u32, u32),
    /// Draw a sphere planet with this many tiles along a cube face edge (16..=512).
    pub sphere: Option<u32>,
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
    /// Generate a universe and zoom from the home globe out to the star map.
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
    /// Draw the latitude and longitude lines of a sphere planet.
    pub graticule: bool,
    /// Print usage and exit.
    pub help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            bench_seconds: None,
            size: DEFAULT_SIZE,
            scale: 100,
            screenshot: None,
            style: None,
            seed: 1,
            players: 4,
            factions: None,
            map_size: (256, 256),
            sphere: None,
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
            graticule: true,
            help: false,
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
            "--universe" => options.universe = true,
            "--zoom-all" => options.zoom_all = true,
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
                    "night" => (true, true),
                    _ => return Err(format!("--sun expects orbit, fixed or night, got {text:?}")),
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
            "--no-hud" => options.no_hud = true,
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
            "-h" | "--help" => options.help = true,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(options)
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
        assert_eq!(o.orbit, Some([0.0, 10.0, 12.0, 30.0]));
        assert_eq!(parse(args(&[])).unwrap().sphere, None);
        for bad in [["--sphere", "8"], ["--sphere", "x"], ["--orbit", "1,2,3"]] {
            assert!(parse(args(&bad)).is_err(), "{bad:?}");
        }
        assert!(parse(args(&["--orbit", "0,95,12,0"])).is_err());
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
        for bad in [["--sun", "still"], ["--graticule", "maybe"]] {
            assert!(parse(args(&bad)).is_err(), "{bad:?}");
        }
        assert!(USAGE.contains("--sun") && USAGE.contains("--graticule"));
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
    fn defaults_without_arguments() {
        assert_eq!(parse(args(&[])), Ok(Options::default()));
    }

    #[test]
    fn rejects_bad_values() {
        assert!(parse(args(&["--bench", "0"])).is_err());
        assert!(parse(args(&["--size", "1920"])).is_err());
        assert!(parse(args(&["--scale", "40"])).is_err());
        assert!(parse(args(&["--bench"])).is_err());
        assert!(parse(args(&["--fast"])).is_err());
    }
}

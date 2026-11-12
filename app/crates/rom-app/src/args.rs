//! Command-line options.

use std::path::PathBuf;

/// Usage text printed for `--help` and after argument errors.
pub const USAGE: &str = "usage: rise-of-man [--bench <seconds>] [--screenshot <file.png>] \
[--size <width>x<height>] [--scale <percent>] [--style <key>] [--seed <n>] [--players <n>] \
[--map-size <width>x<height>] [--camera <x>,<y>,<distance>,<yaw_deg>,<pitch_deg>] [--boxes <n>] [--demo] \
[--speed <percent>] [--stress-units <n>] [--after <seconds>] [--start-markers]";

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
    /// Map size in tiles.
    pub map_size: (u32, u32),
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
            map_size: (256, 256),
            camera: None,
            boxes: 0,
            demo: false,
            speed_pct: 100,
            stress_units: 0,
            after_seconds: 1.0,
            start_markers: false,
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
            "--map-size" => options.map_size = parse_map_size(&value("--map-size")?)?,
            "--camera" => options.camera = Some(parse_camera(&value("--camera")?)?),
            "--boxes" => options.boxes = number("--boxes", &value("--boxes")?)?,
            "--demo" => options.demo = true,
            "--start-markers" => options.start_markers = true,
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

//! Command-line options.

/// Usage text printed for `--help` and after argument errors.
pub const USAGE: &str =
    "usage: rise-of-man [--bench <seconds>] [--size <width>x<height>] [--scale <percent>]";

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
    /// Print usage and exit.
    pub help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            bench_seconds: None,
            size: DEFAULT_SIZE,
            scale: 100,
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
            "-h" | "--help" => options.help = true,
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(options)
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

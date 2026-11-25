//! `rom serve`, `rom keys` and `rom client`: network play from the command line (task 86,
//! `app/docs/network.md`).

use std::io::{BufRead, IsTerminal};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use rom_match::Spec;
use rom_net::{
    ClientOptions, Credential, DisconnectPolicy, MatchPlan, Operator, Server, ServerConfig,
    StateDb, Status,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const SERVE_USAGE: &str = "usage: rom serve [--bind <addr>] [--port <n>] [--path </p/>] \
[--state-dir <dir>] [--resume] [--keys <n>] [--seed <n>] [--style <Planet|Solar|Cluster|Sector|Quadrant|Galaxy|key>] \
[--home-size <n>] [--start-placement <separate|together>] [--city-area <on|off>] [--gather-rate <pct>] [--research-time <pct>] [--age <ordinal>] [--humans <n>] \
[--ai <profile:difficulty,...|none>] [--factions <key,key,...>] [--speed <percent>] \
[--on-disconnect <pause-then-computer|idle|computer>] [--wait-seconds <n>] [--takeover <profile:difficulty>] \
[--autosave-minutes <n>] [--countdown-seconds <n>]";

const KEYS_USAGE: &str = "usage: rom keys <add <n>|list> [--state-dir <dir>]";

const CLIENT_USAGE: &str = "usage: rom client <url> (--key <key> | --token <token>) [--name <name>] \
[--bot [<profile:difficulty>]] [--ticks <n>] [--faction <key>] [--not-ready] [--snapshot-dir <dir>]";

const CONSOLE_HELP: &str =
    "commands: status | keys <n> | start | pause | resume | speed <percent> | save [name] | stop";

/// A map preset's name, or a map style key as given.
fn style_key(text: &str) -> String {
    match text.to_ascii_lowercase().as_str() {
        "planet" => "map.one_world".into(),
        "solar" => "map.solar".into(),
        "cluster" => "map.many_worlds".into(),
        "sector" => "map.sector".into(),
        "quadrant" => "map.quadrant".into(),
        "galaxy" => "map.galaxy".into(),
        _ => text.into(),
    }
}

fn specs(text: &str) -> Result<Vec<Spec>> {
    if text == "none" {
        return Ok(Vec::new());
    }
    Ok(text
        .split(',')
        .map(Spec::parse)
        .collect::<std::result::Result<Vec<_>, _>>()?)
}

/// Sets `flag` on SIGTERM (systemd) and SIGINT (Ctrl-C).
fn on_signal(flag: &Arc<AtomicBool>) -> Result<()> {
    for signal in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
        signal_hook::flag::register(signal, Arc::clone(flag))?;
    }
    Ok(())
}

/// One line about the server.
fn summary(status: &Status) -> String {
    let seats: Vec<String> = status
        .seats
        .iter()
        .map(|s| format!("{} {} {:?}", s.seat, s.name, s.state).to_lowercase())
        .collect();
    format!(
        "{} tick {} speed {} %{}{}; seats: {}; keys unused {} used {}; connections {}; hash checks {} desyncs {}",
        status.phase,
        status.tick,
        status.speed,
        if status.running { ", running" } else { "" },
        status
            .paused_by
            .as_ref()
            .map(|by| format!(", paused by {by}"))
            .unwrap_or_default(),
        seats.join(", "),
        status.keys.unused,
        status.keys.used,
        status.connections,
        status.hash_checks,
        status.desyncs
    )
}

/// `rom serve`.
pub fn serve(mut args: impl Iterator<Item = String>) -> Result<()> {
    let mut config = ServerConfig {
        log: Arc::new(|line| println!("{line}")),
        ..ServerConfig::default()
    };
    let mut plan = MatchPlan::default();
    let mut keys = None;
    while let Some(flag) = args.next() {
        if flag == "--resume" {
            config.resume = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}; {SERVE_USAGE}"))?;
        match flag.as_str() {
            "--bind" => config.bind = value.parse::<IpAddr>()?,
            "--port" => config.port = value.parse()?,
            "--path" => {
                let mut path = value;
                if !path.starts_with('/') {
                    path.insert(0, '/');
                }
                if !path.ends_with('/') {
                    path.push('/');
                }
                config.path = path;
            }
            "--state-dir" => config.state_dir = PathBuf::from(value),
            "--keys" => keys = Some(value.parse::<u32>()?),
            "--seed" => plan.seed = value.parse()?,
            "--style" => plan.style = Some(style_key(&value)),
            "--home-size" | "--sphere" => plan.home_size = Some(value.parse()?),
            "--start-placement" => plan.start_placement = Some(value),
            "--city-area" => {
                crate::ai_match_cmd::parse_city_area(&value)?;
                plan.city_area = Some(value.trim().to_ascii_lowercase());
            }
            "--gather-rate" => {
                plan.pace.gather_rate = crate::ai_match_cmd::parse_gather_rate(&value)?;
            }
            "--research-time" => {
                plan.pace.research_time = crate::ai_match_cmd::parse_research_time(&value)?;
            }
            "--age" => plan.age = Some(value.parse()?),
            "--humans" => plan.humans = value.parse()?,
            "--ai" => plan.ai = specs(&value)?,
            "--factions" => plan.factions = value.split(',').map(str::to_owned).collect(),
            "--speed" => config.speed = value.parse::<u32>()?.clamp(1, 10_000),
            "--on-disconnect" => {
                config.on_disconnect = DisconnectPolicy::parse(&value)
                    .ok_or_else(|| format!("unknown --on-disconnect {value}; {SERVE_USAGE}"))?
            }
            "--wait-seconds" => config.wait = Duration::from_secs(value.parse()?),
            "--takeover" => config.takeover = Spec::parse(&value)?,
            "--autosave-minutes" => {
                let minutes: u64 = value.parse()?;
                config.autosave = (minutes > 0).then(|| Duration::from_secs(minutes * 60));
            }
            "--countdown-seconds" => config.countdown = Duration::from_secs(value.parse()?),
            _ => return Err(format!("unknown argument: {flag}; {SERVE_USAGE}").into()),
        }
    }
    let humans = plan.humans;
    config.plan = plan;
    let stop = Arc::new(AtomicBool::new(false));
    on_signal(&stop)?;
    println!(
        "rom serve {} rules {} (protocol {}), state in {}",
        rom_net::VERSION,
        rom_match::rules_fingerprint_hex(),
        rom_net::PROTOCOL,
        config.state_dir.display()
    );
    let resume = config.resume;
    let path = config.path.clone();
    let server = Server::start(config)?;
    println!(
        "address: http://{}{path}  (status: http://{}{path}status)",
        server.addr(),
        server.addr()
    );
    let first = keys.unwrap_or(if resume { 0 } else { humans });
    for key in server.add_keys(first, false)? {
        println!("key: {key}");
    }
    let (lines_tx, lines) = mpsc::channel::<String>();
    if std::io::stdin().is_terminal() {
        println!("{CONSOLE_HELP}");
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                if lines_tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    loop {
        if stop.load(Ordering::SeqCst) {
            println!("stopping on a signal: saving");
            server.stop();
            return Ok(());
        }
        if server.is_finished() {
            server.wait();
            return Ok(());
        }
        let Ok(line) = lines.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or("");
        let argument = words.next();
        match (command, argument) {
            ("", _) => {}
            ("status", _) => println!("{}", summary(&server.status())),
            ("keys", n) => match n.map(str::parse::<u32>) {
                Some(Ok(n)) => match server.add_keys(n, false) {
                    Ok(keys) => keys.iter().for_each(|k| println!("key: {k}")),
                    Err(error) => println!("cannot add keys: {error}"),
                },
                _ => println!("keys <n>"),
            },
            ("start", _) => {
                server.command(Operator::Start);
            }
            ("pause", _) => {
                server.command(Operator::Pause);
            }
            ("resume", _) => {
                server.command(Operator::Resume);
            }
            ("speed", Some(pct)) => match pct.parse() {
                Ok(pct) => {
                    server.command(Operator::Speed(pct));
                }
                Err(_) => println!("speed <percent>"),
            },
            ("save", name) => {
                server.command(Operator::Save(name.map(str::to_owned)));
            }
            ("stop", _) => {
                server.stop();
                return Ok(());
            }
            _ => println!("{CONSOLE_HELP}"),
        }
    }
}

/// `rom keys add N | list`.
pub fn keys(mut args: impl Iterator<Item = String>) -> Result<()> {
    let action = args.next().ok_or(KEYS_USAGE)?;
    let mut count = None;
    if action == "add" {
        count = Some(args.next().ok_or(KEYS_USAGE)?.parse::<u32>()?);
    } else if action != "list" {
        return Err(KEYS_USAGE.into());
    }
    let mut dir = PathBuf::from("rom-server");
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--state-dir", Some(value)) => dir = PathBuf::from(value),
            _ => return Err(KEYS_USAGE.into()),
        }
    }
    let db = StateDb::open(&dir)?;
    if let Some(n) = count {
        for key in db.add_keys(n, false)? {
            println!("{key}");
        }
    }
    let counts = db.key_counts()?;
    eprintln!(
        "keys in {}: {} unused, {} used",
        dir.display(),
        counts.unused,
        counts.used
    );
    Ok(())
}

/// `rom client`.
pub fn client(mut args: impl Iterator<Item = String>) -> Result<()> {
    let url = args.next().ok_or(CLIENT_USAGE)?;
    if url.starts_with("--") {
        return Err(CLIENT_USAGE.into());
    }
    let mut credential = None;
    let mut options = ClientOptions::new(&url, Credential::Key(String::new()));
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--bot" => {
                let spec = match args.peek() {
                    Some(next) if !next.starts_with("--") => {
                        Spec::parse(&args.next().unwrap_or_default())?
                    }
                    _ => Spec::default(),
                };
                options.bot = Some(spec);
            }
            "--not-ready" => options.ready = false,
            _ => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}; {CLIENT_USAGE}"))?;
                match flag.as_str() {
                    "--key" => credential = Some(Credential::Key(value)),
                    "--token" => credential = Some(Credential::Token(value)),
                    "--name" => options.name = value,
                    "--ticks" => options.ticks = Some(value.parse()?),
                    "--faction" => options.faction = Some(value),
                    "--snapshot-dir" => options.snapshot_dir = PathBuf::from(value),
                    _ => return Err(format!("unknown argument: {flag}; {CLIENT_USAGE}").into()),
                }
            }
        }
    }
    options.credential = credential.ok_or(CLIENT_USAGE)?;
    let stop = Arc::new(AtomicBool::new(false));
    on_signal(&stop)?;
    options.stop = Some(stop);
    options.log = Some(Arc::new(|line| println!("{line}")));
    let report = rom_net::run_client(options)?;
    println!(
        "client seat={} tick={} hash={:016x} hashes={} snapshots={} desyncs={} orders={} rejected={} ended={:?}",
        report.seat,
        report.tick,
        report.hash,
        report.hashes.len(),
        report.snapshots,
        report.desyncs,
        report.orders,
        report.rejected,
        report.ended
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_name_their_styles() {
        assert_eq!(style_key("Cluster"), "map.many_worlds");
        assert_eq!(style_key("solar"), "map.solar");
        assert_eq!(style_key("map.galaxy"), "map.galaxy");
        assert_eq!(specs("none").unwrap(), Vec::new());
        assert_eq!(specs("builder:tough,balanced").unwrap().len(), 2);
        assert!(specs(":x").is_err());
    }
}

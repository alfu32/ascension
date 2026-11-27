//! `rom replay-hash`: a leader with computer players in every seat, replayed by a follower fed
//! its journals; the hashes are compared every 100 ticks and the final one printed. The same
//! command on two machines (x86_64 here, aarch64 on the server) must print the same line.

use rom_match::{ContentRequest, MapKind, MatchSettings, ScriptSetup, Spec};

/// Ticks between two comparisons of the leader's and the follower's hashes.
const CHECK_EVERY: u64 = 100;

/// What a replay found.
#[derive(Debug, PartialEq, Eq)]
pub struct Replay {
    /// The leader's (and follower's) hash at the end.
    pub hash: u64,
    /// Comparisons made.
    pub checks: u64,
    /// Commands journaled.
    pub orders: u64,
    /// Planet surfaces journaled.
    pub surfaces: u64,
}

/// Runs `settings` for `ticks` as a leader and a follower side by side.
pub fn replay(settings: &MatchSettings, ticks: u64) -> Result<Replay, String> {
    let mut leader = settings.build()?.leader();
    let mut follower = settings.build()?.follower();
    let (mut checks, mut orders, mut surfaces) = (0, 0, 0);
    for _ in 0..ticks {
        let journal = leader.lead(Vec::new()).journal;
        for entry in &journal.entries {
            match entry {
                rom_match::Entry::Order(_) => orders += 1,
                rom_match::Entry::SurfaceAdded { .. } => surfaces += 1,
            }
        }
        follower.follow(&journal).map_err(|e| e.to_string())?;
        let tick = leader.world().tick();
        if tick.is_multiple_of(CHECK_EVERY) || tick == ticks {
            let (lead, follow) = (leader.world().hash(), follower.world().hash());
            if lead != follow {
                return Err(format!(
                    "out of step at tick {tick}: leader {lead:016x}, follower {follow:016x}"
                ));
            }
            checks += 1;
        }
    }
    Ok(Replay {
        hash: leader.world().hash(),
        checks,
        orders,
        surfaces,
    })
}

/// `rom replay-hash --seed N --ticks T [--universe] [--players N] [--style KEY]
/// [--ai profile:difficulty,…] [--city-area on|off] [--gather-rate pct] [--research-speed pct]`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let usage = "usage: rom replay-hash --seed <n> --ticks <n> [--universe] [--players <n>] \
                 [--style <key>] [--ai <profile:difficulty,...>] [--city-area <on|off>] \
                 [--gather-rate <pct>] [--research-speed <pct>]";
    let mut settings = MatchSettings {
        content: ContentRequest {
            players: 2,
            ..ContentRequest::default()
        },
        map: MapKind::Flat,
        age: None,
        script: ScriptSetup::default(),
        ai: Some(Vec::new()),
        humans: Vec::new(),
        city_area: Default::default(),
        pace: Default::default(),
    };
    let (mut seed, mut ticks) = (None, None);
    while let Some(flag) = args.next() {
        if flag == "--universe" {
            settings.map = MapKind::Universe;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}; {usage}"))?;
        match flag.as_str() {
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--ticks" => ticks = Some(value.parse::<u64>()?),
            "--players" => settings.content.players = value.parse()?,
            "--style" => settings.content.style = Some(value),
            "--city-area" => settings.city_area = crate::ai_match_cmd::parse_city_area(&value)?,
            "--gather-rate" => {
                settings.pace.gather_rate = crate::ai_match_cmd::parse_gather_rate(&value)?;
            }
            "--research-speed" => {
                settings.pace.research_time = crate::ai_match_cmd::parse_research_speed(&value)?;
            }
            "--ai" => {
                settings.ai = Some(
                    value
                        .split(',')
                        .map(Spec::parse)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            _ => return Err(format!("unknown argument: {flag}; {usage}").into()),
        }
    }
    let seed = seed.ok_or(usage)?;
    let ticks = ticks.ok_or(usage)?;
    settings.content.seed = seed;
    let replay = replay(&settings, ticks)?;
    println!(
        "replay-hash seed={seed} ticks={ticks} map={} players={} rules={} checks={} orders={} \
         surfaces={} hash={:016x}",
        match settings.map {
            MapKind::Universe => "universe",
            MapKind::Sphere(_) => "sphere",
            MapKind::Flat => "flat",
        },
        settings.content.players,
        rom_match::rules_fingerprint_hex(),
        replay.checks,
        replay.orders,
        replay.surfaces,
        replay.hash
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_replay_repeats() {
        let settings = MatchSettings {
            content: ContentRequest {
                players: 2,
                seed: 3,
                map_size: (96, 96),
                ..ContentRequest::default()
            },
            map: MapKind::Flat,
            age: None,
            script: ScriptSetup::default(),
            ai: Some(Vec::new()),
            humans: Vec::new(),
            city_area: Default::default(),
            pace: Default::default(),
        };
        let first = replay(&settings, 600).unwrap();
        assert_eq!(first.checks, 6);
        assert!(first.orders > 0);
        assert_eq!(replay(&settings, 600).unwrap(), first);
    }
}

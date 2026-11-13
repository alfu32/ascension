//! Headless city capture and capital elimination scenario.

use rom_geo::CubeSurface;
use rom_rules::{Rules, VictoryRule};
use rom_sim::{Outcome, PlayerStatus, World};
use std::{path::PathBuf, sync::Arc};

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seed, mut sphere) = (None, None, None);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--sphere" => sphere = Some(value.parse::<u32>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or("missing --db")?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    let surface = sphere.map(|n| CubeSurface { n });
    let mut world = World::siege_scenario(rules, seed.ok_or("missing --seed")?, surface)?;
    while world.tick() < 12_000 && world.outcome() == &Outcome::Playing {
        world.step();
    }
    let outcome = match world.outcome() {
        Outcome::Playing => "playing".to_owned(),
        Outcome::Won { team, rule, .. } => format!(
            "won {team} {}",
            match rule {
                VictoryRule::Conquest => "conquest",
                VictoryRule::Territory => "territory",
                _ => return Err("unsupported siege victory".into()),
            }
        ),
    };
    let eliminated = world
        .players()
        .filter(|p| world.player_status(p.id) == Some(PlayerStatus::Defeated))
        .map(|p| p.id.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "siege tick={} outcome={} captured={} eliminated={} hash={:016x}",
        world.tick(),
        outcome,
        world.stats().cities_captured,
        eliminated,
        world.hash()
    );
    Ok(())
}

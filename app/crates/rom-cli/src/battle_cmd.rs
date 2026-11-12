//! Headless deterministic battle benchmark.

use rom_rules::Rules;
use rom_sim::{PlayerId, World};
use std::{path::PathBuf, sync::Arc};

pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seed, mut ticks, mut per_side) = (None, None, None, None);
    let mut swap = false;
    while let Some(flag) = args.next() {
        if flag == "--swap" {
            swap = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--ticks" => ticks = Some(value.parse::<u64>()?),
            "--per-side" => per_side = Some(value.parse::<u32>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or("missing --db")?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    let mut world = World::battle_scenario_swapped(
        rules,
        seed.ok_or("missing --seed")?,
        per_side.ok_or("missing --per-side")?,
        swap,
    )?;
    for _ in 0..ticks.ok_or("missing --ticks")? {
        world.step();
    }
    let alive = |owner| {
        world
            .entities()
            .filter(|e| e.owner == PlayerId(owner))
            .count()
    };
    let kills = |owner| {
        world
            .stats()
            .kills
            .get(&PlayerId(owner))
            .copied()
            .unwrap_or(0)
    };
    println!(
        "tick={} alive={},{} kills={},{} hash={:016x}",
        world.tick(),
        alive(1),
        alive(2),
        kills(1),
        kills(2),
        world.hash()
    );
    Ok(())
}

//! Headless relic custody and city trade scenario.
use rom_rules::Rules;
use rom_sim::{PlayerId, World};
use std::{path::PathBuf, sync::Arc};

/// Runs the wealth scenario with the arguments following `wealth`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seed, mut sphere) = (None, None, false);
    while let Some(flag) = args.next() {
        if flag == "--sphere" {
            sphere = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or("missing --db")?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    let mut world = World::wealth_scenario(rules, seed.ok_or("missing --seed")?, sphere)?;
    for _ in 0..6000 {
        world.step();
    }
    let kept: usize = world
        .entities()
        .filter(|e| world.rules().entity_types[&e.type_id].keeper.is_some())
        .map(|e| world.kept_nodes(e.id).count())
        .sum();
    let wealth = world
        .rules()
        .id_of("res.wealth")
        .ok_or("missing wealth resource")?;
    let gained: i64 = world
        .players()
        .filter(|p| p.id == PlayerId(1))
        .map(|p| p.resources.get(&wealth).copied().unwrap_or(0))
        .sum();
    println!(
        "wealth relics_kept={} routes={} trips={} wealth_gained={} hash={:016x}",
        kept,
        world.trade_routes(PlayerId(1)).count(),
        world.stats().trade_trips,
        gained,
        world.hash()
    );
    Ok(())
}

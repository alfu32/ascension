use std::{env, path::PathBuf, process::ExitCode, sync::Arc};

use rom_rules::{Rules, build_rules_db, write_rules_db};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args
        .next()
        .ok_or("usage: rom <build-rules|check-rules|run> [options]")?;
    if command == "run" {
        return run_simulation(args);
    }
    if command == "check-rules" {
        if args.next().as_deref() != Some("--db") {
            return Err("usage: rom check-rules --db <path>".into());
        }
        let path = args.next().ok_or("missing --db path")?;
        if args.next().is_some() {
            return Err("usage: rom check-rules --db <path>".into());
        }
        let conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let rules = Rules::load(&conn)?;
        let problems = rules.validate();
        for problem in &problems {
            println!("{problem}");
        }
        if !problems.is_empty() {
            return Err(format!("{} rules problems", problems.len()).into());
        }
        println!(
            "rules ok: {} definitions, {} modifiers",
            rules.definition_count(),
            rules.modifiers.values().map(Vec::len).sum::<usize>()
        );
        return Ok(());
    }
    if command != "build-rules" {
        return Err("usage: rom <build-rules|check-rules|run> [options]".into());
    }

    let mut schema = None;
    let mut seed = None;
    let mut out = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--schema" => schema = Some(PathBuf::from(value)),
            "--seed" => seed = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }

    let schema = std::fs::read_to_string(schema.ok_or("missing --schema")?)?;
    let seed = std::fs::read_to_string(seed.ok_or("missing --seed")?)?;
    let out = out.ok_or("missing --out")?;
    let conn = build_rules_db(&schema, &seed)?;
    write_rules_db(&conn, &out)?;
    let definitions: i64 = conn.query_row("SELECT COUNT(*) FROM def", [], |row| row.get(0))?;
    let tables: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    println!("rules.db: {definitions} definitions, {tables} tables");
    Ok(())
}

fn run_simulation(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut db = None;
    let mut seed = None;
    let mut ticks = None;
    let mut units = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--ticks" => ticks = Some(value.parse::<u64>()?),
            "--units" => units = Some(value.parse::<u32>()?),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let db = db.ok_or("missing --db")?;
    let seed = seed.ok_or("missing --seed")?;
    let ticks = ticks.ok_or("missing --ticks")?;
    let units = units.ok_or("missing --units")?;
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rules = Arc::new(Rules::load(&conn)?);
    let mut world = rom_sim::World::movement_scenario(rules, seed, units)?;
    for _ in 0..ticks {
        world.step();
    }
    println!(
        "tick={} entities={} orders_done={} hash={:016x}",
        world.tick(),
        world.entities().count(),
        world.stats().orders_done,
        world.hash()
    );
    Ok(())
}

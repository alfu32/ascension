use std::{env, path::PathBuf, process::ExitCode};

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
        .ok_or("usage: rom <build-rules|check-rules> [options]")?;
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
        return Err("usage: rom <build-rules|check-rules> [options]".into());
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

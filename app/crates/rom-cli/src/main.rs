use std::{env, path::PathBuf, process::ExitCode};

use rom_rules::{build_rules_db, write_rules_db};

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
    if args.next().as_deref() != Some("build-rules") {
        return Err("usage: rom build-rules --schema <path> --seed <path> --out <path>".into());
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

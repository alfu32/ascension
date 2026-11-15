//! `rom model …`: the model files and Meshcraft round trip (docs/models-in-meshcraft.md).

use std::path::PathBuf;

const USAGE: &str = "usage: rom model write-sql --db <rules.db> [--out <dir>]";

/// Runs a `rom model` subcommand.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let command = args.next().ok_or(USAGE)?;
    let mut flags = Vec::new();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        flags.push((flag, value));
    }
    let flag = |name: &str| {
        flags
            .iter()
            .find(|(f, _)| f == name)
            .map(|(_, v)| PathBuf::from(v))
    };
    match command.as_str() {
        "write-sql" => {
            let db = flag("--db").ok_or(USAGE)?;
            let out = flag("--out").unwrap_or_else(rom_models::files::default_models_dir);
            let conn = rusqlite::Connection::open_with_flags(
                db,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            let written = rom_models::sql::write_all(&conn, &out)?;
            println!(
                "model write-sql: {} files in {}",
                written.len(),
                out.display()
            );
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

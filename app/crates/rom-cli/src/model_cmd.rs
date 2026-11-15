//! `rom model …`: the model files and Meshcraft round trip (docs/models-in-meshcraft.md).

use std::path::PathBuf;

const USAGE: &str = "usage: rom model export <model id> [--out <dir>]
       rom model export-all [--out <dir>]
       rom model import <file.octd> [--model <id>] [--unlock]
       rom model write-sql --db <rules.db> [--out <dir>]
  export and import also take --models <dir> and --assets <dir> (default: db/models and
  assets/models of the source tree), to work on a copy";

/// Runs a `rom model` subcommand.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let command = args.next().ok_or(USAGE)?;
    let mut flags = Vec::new();
    let mut positional = Vec::new();
    let mut unlock = false;
    while let Some(arg) = args.next() {
        if arg == "--unlock" {
            unlock = true;
        } else if arg.starts_with("--") {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {arg}"))?;
            flags.push((arg, value));
        } else {
            positional.push(arg);
        }
    }
    let flag = |name: &str| {
        flags
            .iter()
            .find(|(f, _)| f == name)
            .map(|(_, v)| PathBuf::from(v))
    };
    let mut paths = rom_models::export::Paths::default();
    if let Some(dir) = flag("--models") {
        paths.models = dir;
    }
    if let Some(dir) = flag("--assets") {
        paths.assets = dir;
    }
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
        "export" => {
            let [id] = positional.as_slice() else {
                return Err(USAGE.into());
            };
            let out = flag("--out").unwrap_or_else(rom_models::files::default_export_dir);
            let lib = rom_models::export::Library::open(paths)?;
            for path in rom_models::export::export(&lib, id, &out)? {
                println!("wrote {}", path.display());
            }
            Ok(())
        }
        "export-all" => {
            if !positional.is_empty() {
                return Err(USAGE.into());
            }
            let out = flag("--out").unwrap_or_else(rom_models::files::default_export_dir);
            let lib = rom_models::export::Library::open(paths)?;
            let s = rom_models::export::export_all(&lib, &out)?;
            println!(
                "model export-all: {} models ({} done, {} not done yet), {} shared sub-assemblies, {} files in {}",
                s.done + s.starters,
                s.done,
                s.starters,
                s.shared,
                s.files,
                out.display()
            );
            Ok(())
        }
        "import" => {
            let [file] = positional.as_slice() else {
                return Err(USAGE.into());
            };
            let lib = rom_models::export::Library::open(paths)?;
            let options = rom_models::import::ImportOptions {
                model: flags
                    .iter()
                    .find(|(f, _)| f == "--model")
                    .map(|(_, v)| v.clone()),
                unlock,
            };
            let report = rom_models::import::import(&lib, std::path::Path::new(file), &options)?;
            println!(
                "model import {}: {} kit parts, {} free-form, {} includes -> {}",
                report.id,
                report.counts[0],
                report.counts[1],
                report.counts[2],
                report.sql.display()
            );
            for path in &report.meshes {
                println!("  wrote {}", path.display());
            }
            for path in &report.removed {
                println!("  removed {}", path.display());
            }
            if let (Some((near, far)), Some(fingerprint)) = (report.triangles, &report.fingerprint)
            {
                println!("  {near} triangles near, {far} far; fingerprint {fingerprint}");
            }
            for warning in &report.warnings {
                println!("  warning: {warning}");
            }
            if !report.unlocked.is_empty() {
                println!(
                    "  approved models changed (--unlock); new fingerprints for docs/art/approved.toml:"
                );
                for line in &report.unlocked {
                    println!("  {line}");
                }
            }
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

//! `rom report <save-or-server.db>`: the match report as text (task 117): the players ranked
//! across the report's aspects and the facts of the match. A server's `server.db` (or its state
//! folder) is read through its latest save, with the seats' names.

use std::{path::PathBuf, sync::Arc};

use rom_match::MatchReport;

const USAGE: &str = "usage: rom report <save.rom | server.db | state folder>";

/// Prints the report of a save or of a server's match.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(args.next().ok_or(USAGE)?);
    if args.next().is_some() {
        return Err(USAGE.into());
    }
    let rules = Arc::new(rom_match::load_rules()?);
    let report = if path.is_dir() || is_state_db(&path) {
        rom_net::report::state_report(&path, &rules)?
    } else {
        let (world, info, _) = rom_match::load_match(&path, &rules)?;
        MatchReport::of(&world, &info)
    };
    print!("{}", report.to_text());
    Ok(())
}

/// Whether `path` is a server's state database rather than a save.
fn is_state_db(path: &std::path::Path) -> bool {
    rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='net_match'",
                [],
                |r| r.get::<_, i64>(0),
            )
        })
        .is_ok_and(|n| n > 0)
}

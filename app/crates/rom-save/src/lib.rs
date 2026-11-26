//! Save and load a match: one SQLite file per save in the PART B tables of
//! `db/schema.sql`, written and read between ticks.
//!
//! A save records the rules version and a fingerprint of the loaded rules; loading
//! under other rules is refused. The caller supplies the match settings it wants back
//! and, per computer player, an opaque versioned memory blob.

mod codec;
mod read;
mod write;

use rom_rules::Rules;
use rom_sim::{PlayerId, SimError, World};
use std::{collections::BTreeMap, fmt::Write as _, path::Path, sync::Arc};
use thiserror::Error;

/// Layout version of the PART B tables this crate writes and reads.
pub const FORMAT: i64 = 25;

/// The oldest layout this crate still reads: format 13 saves load without the match end, the
/// players' counters and their history (task 117); formats 13 and 14 with every aircraft in the
/// air (task 123); formats 13 to 15 without drone operators (task 120); format 16 with every
/// drone grid full (task 128); formats 13 to 17 without first contacts and known capitals
/// (task 130: players meet again as they see each other); formats 13 to 18 had no shots at the
/// ground (task 142); formats 13 to 19 name no city (task 144: the cities are named on load from
/// their owners' lists in id order); formats 13 to 20 had no facing on a construction order (task
/// 147: such orders face 0); formats 13 to 21 had no city area option (task 158: they load with
/// the city area limiting building, as it did); formats 13 to 22 had no shields (task 157).
/// Files written before format 25 (task 162: each star system's cluster) are refused: the
/// conversions above apply to such files only once they carry its columns.
pub const OLDEST_FORMAT: i64 = 13;

const PART_B: &str = include_str!("../../../db/schema.sql");

/// Everything that can go wrong saving or loading.
#[derive(Debug, Error)]
pub enum SaveError {
    /// SQLite failed.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    /// The file could not be written or replaced.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The save was made under other rules than the ones supplied.
    #[error(
        "save was made with rules {saved_version} ({saved_fingerprint}), \
         but the loaded rules are {rules_version} ({rules_fingerprint})"
    )]
    RulesMismatch {
        /// Rules version recorded in the save.
        saved_version: String,
        /// Rules fingerprint recorded in the save.
        saved_fingerprint: String,
        /// Version of the rules supplied.
        rules_version: String,
        /// Fingerprint of the rules supplied.
        rules_fingerprint: String,
    },
    /// The save uses a table layout this build does not read.
    #[error(
        "unsupported save format {found}; this build reads formats {OLDEST_FORMAT} to {FORMAT}"
    )]
    UnsupportedFormat {
        /// The format recorded in the file.
        found: i64,
    },
    /// The file is not a complete, consistent save.
    #[error("corrupt save: {0}")]
    Corrupt(&'static str),
    /// The simulation rejected the restored state.
    #[error(transparent)]
    Sim(#[from] SimError),
    /// The schema file has no PART B.
    #[error("schema has no PART B")]
    NoSchema,
}

/// Who controls a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Controller {
    /// A person at this machine.
    Human,
    /// A computer player.
    Ai,
    /// A person on another machine.
    Remote,
    /// Neutral nature.
    Nature,
}

/// A player's seat in the match: what the setup screen chose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat {
    /// The player's id.
    pub player: PlayerId,
    /// Slot on the setup screen.
    pub slot: u32,
    /// Display name.
    pub name: String,
    /// Who plays it.
    pub controller: Controller,
    /// `ai_profile.key` of a computer player, else empty.
    pub ai_profile: String,
    /// Colour index.
    pub color: u32,
}

/// The match settings a save carries beside the simulation state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchInfo {
    /// The seed the match was generated from.
    pub seed: u64,
    /// `map_style.key` of the generated map.
    pub map_style: String,
    /// Game speed in percent of the base 20 ticks per second.
    pub speed_pct: u32,
    /// Setup-screen choices the caller wants back, by its own keys.
    pub settings: BTreeMap<String, String>,
    /// Seats of the players; a player without one gets a default seat.
    pub seats: Vec<Seat>,
}

/// A computer player's saved memory, produced and consumed by the AI crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiState {
    /// `ai_profile.key`.
    pub profile: String,
    /// `ai_difficulty.key`.
    pub difficulty: String,
    /// Format version of `data`, the AI crate's own.
    pub version: u32,
    /// State of the computer player's own random generator.
    pub rng_state: u64,
    /// Index of its current opening-book step.
    pub opening_seq: i64,
    /// The opaque snapshot.
    pub data: Vec<u8>,
}

/// Computer-player memory by player id.
pub type AiStates = BTreeMap<PlayerId, AiState>;

/// What a save file says about itself, readable without the rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveHeader {
    /// Rules version the match ran under.
    pub rules_version: String,
    /// Fingerprint of those rules.
    pub rules_fingerprint: String,
    /// Tick the save was made at.
    pub tick: u64,
    /// The match settings.
    pub info: MatchInfo,
}

/// The rules version, as `schema_meta` names it.
pub fn rules_version(rules: &Rules) -> String {
    rules
        .schema_meta
        .get("rules_version")
        .map_or_else(String::new, |row| row.value.clone())
}

/// A 64-bit FNV-1a digest, in hex, of every loaded rule. Equal rules give equal text;
/// it is only comparable between saves and builds of the same rules model.
pub fn rules_fingerprint(rules: &Rules) -> String {
    struct Fnv(u64);
    impl std::fmt::Write for Fnv {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            for byte in text.bytes() {
                self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
            Ok(())
        }
    }
    let mut hash = Fnv(0xcbf29ce484222325);
    write!(hash, "{rules:?}").expect("hashing does not fail");
    format!("{:016x}", hash.0)
}

/// The PART B section of the schema.
fn part_b() -> Result<&'static str, SaveError> {
    let start = PART_B.find("-- PART B").ok_or(SaveError::NoSchema)?;
    let start = PART_B[..start].rfind("-- ═").unwrap_or(start);
    let end = PART_B.find("-- VIEWS").ok_or(SaveError::NoSchema)?;
    let end = PART_B[..end].rfind("-- ═").unwrap_or(end);
    Ok(&PART_B[start..end])
}

/// Writes a save of `world` to `path`, replacing any file there. The file appears
/// complete or not at all. Call between ticks.
pub fn save(path: &Path, world: &World, info: &MatchInfo, ai: &AiStates) -> Result<(), SaveError> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".part");
    let temporary = std::path::PathBuf::from(temporary);
    let _ = std::fs::remove_file(&temporary);
    let result = write::write(&temporary, world, info, ai)
        .and_then(|()| std::fs::rename(&temporary, path).map_err(SaveError::from));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Reads the header of a save: rules version, fingerprint, tick and match settings.
pub fn read_header(path: &Path) -> Result<SaveHeader, SaveError> {
    read::header(&read::open(path)?)
}

/// Loads a save: the world rebuilt under `rules`, the match settings and the computer
/// players' memory. A save made under other rules is refused.
pub fn load(path: &Path, rules: &Arc<Rules>) -> Result<(World, MatchInfo, AiStates), SaveError> {
    read::read(path, rules)
}

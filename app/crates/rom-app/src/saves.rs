//! Save files in the player's save folder: where they live, what the app stores beside the
//! world, listing them from their headers (without loading the rules) and writing them.
//!
//! The simulation thread writes a save between two ticks (it owns the world); loading starts
//! a fresh game process with `--load <file>`, which regenerates the planet's looks from the
//! seed stored in the save and restores the world on top (see `shell::start_simulation`).

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use rom_rules::Rules;
use rom_save::MatchInfo;
use rom_sim::World;

use crate::computer::{ComputerPlayers, save_match};
use crate::selection::PLAYER;

/// Slots kept for quick saves and for autosaves.
pub const KEEP: usize = 3;
/// File extension of a save.
pub const EXTENSION: &str = "rom";

/// `match_setting` key: the name the player gave the save.
pub const KEY_NAME: &str = "name";
/// `match_setting` key: `manual`, `quick` or `auto`.
pub const KEY_KIND: &str = "kind";
/// `match_setting` key: seconds since the Unix epoch when the save was made.
pub const KEY_SAVED_AT: &str = "saved_at";
/// `match_setting` key: the display name of the player's age.
pub const KEY_AGE: &str = "age";
/// `match_setting` key: the launch arguments that decide the map, one per line.
pub const KEY_ARGS: &str = "args";

/// Why a save was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The player named it.
    Manual,
    /// F5: one of the rotating quick-save slots.
    Quick,
    /// Every few game minutes: one of the rotating autosave slots.
    Auto,
}

impl Kind {
    /// The text stored in the save.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Quick => "quick",
            Self::Auto => "auto",
        }
    }

    /// The label the lists show.
    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "Save",
            Self::Quick => "Quick save",
            Self::Auto => "Autosave",
        }
    }

    fn from_text(text: &str) -> Self {
        match text {
            "quick" => Self::Quick,
            "auto" => Self::Auto,
            _ => Self::Manual,
        }
    }
}

/// A request for the simulation thread to write a save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRequest {
    /// Why it is saved, which decides the file.
    pub kind: Kind,
    /// The player's name for it; blank gets the default name.
    pub name: String,
}

/// What a finished save reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    /// The file written.
    pub path: PathBuf,
    /// The name it carries.
    pub name: String,
}

/// Which rules a game or a save was made with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulesId {
    /// The `schema_meta` rules version.
    pub version: String,
    /// The fingerprint of every loaded rule.
    pub fingerprint: String,
}

impl RulesId {
    /// The identity of `rules`.
    pub fn of(rules: &Rules) -> Self {
        Self {
            version: rom_save::rules_version(rules),
            fingerprint: rom_save::rules_fingerprint(rules),
        }
    }
}

/// What the simulation thread needs to write saves.
#[derive(Clone, Debug)]
pub struct SaveSetup {
    /// Seed, map style, seats and the `match_setting` pairs every save of this match carries.
    pub info: MatchInfo,
    /// The folder saves go to.
    pub folder: PathBuf,
    /// The rules of this game.
    pub rules: RulesId,
    /// Game minutes between autosaves; 0 is off.
    pub autosave_minutes: u32,
}

impl SaveSetup {
    /// A setup for a match of `world` made from `arguments`.
    pub fn new(
        world: &World,
        seed: u64,
        style: &str,
        arguments: &[String],
        autosave_minutes: u32,
        folder: PathBuf,
    ) -> Self {
        let mut info = MatchInfo {
            seed,
            map_style: style.into(),
            speed_pct: 100,
            ..MatchInfo::default()
        };
        info.settings.insert(
            KEY_ARGS.into(),
            crate::args::match_arguments(arguments).join("\n"),
        );
        Self {
            info,
            folder,
            rules: RulesId::of(world.rules()),
            autosave_minutes,
        }
    }
}

/// The folder saves go to: `$XDG_DATA_HOME/ascension/saves`, else the platform's own place
/// for application data.
pub fn folder() -> Result<PathBuf, String> {
    let from = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if let Some(base) = from("XDG_DATA_HOME") {
        PathBuf::from(base)
    } else if cfg!(target_os = "windows") {
        from("APPDATA")
            .map(PathBuf::from)
            .ok_or("APPDATA is not set")?
    } else {
        let home = from("HOME")
            .map(PathBuf::from)
            .ok_or("neither XDG_DATA_HOME nor HOME is set")?;
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support")
        } else {
            home.join(".local/share")
        }
    };
    Ok(base.join("ascension").join("saves"))
}

/// Seconds since the Unix epoch.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `YYYY-MM-DD HH:MM` (UTC) of a Unix time.
pub fn format_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    // Civil date from a day count (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        rest / 3_600,
        rest / 60 % 60
    )
}

/// Game time as `mm:ss`, or `h:mm:ss` from an hour on.
pub fn format_game_time(ticks: u64) -> String {
    let seconds = ticks / u64::from(crate::sim_thread::TICKS_PER_SECOND);
    if seconds >= 3_600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3_600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

/// The map style key without its kind prefix, for display.
pub fn map_label(style: &str) -> &str {
    style.rsplit_once('.').map_or(style, |(_, name)| name)
}

fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    trimmed.chars().take(60).collect::<String>()
}

/// The file a named save goes to.
fn named_path(folder: &Path, name: &str) -> PathBuf {
    let slug = slug(name);
    folder.join(format!(
        "save-{}.{EXTENSION}",
        if slug.is_empty() { "game" } else { &slug }
    ))
}

/// The next slot of a rotation: a free one, else the oldest.
fn slot_path(folder: &Path, prefix: &str) -> PathBuf {
    let slots: Vec<PathBuf> = (1..=KEEP)
        .map(|i| folder.join(format!("{prefix}-{i}.{EXTENSION}")))
        .collect();
    if let Some(free) = slots.iter().find(|p| !p.exists()) {
        return free.clone();
    }
    slots
        .into_iter()
        .min_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
        .expect("KEEP is not zero")
}

/// The name a save gets when the player gives none: map, age and date.
pub fn default_name(world: &World, setup: &SaveSetup, at: u64) -> String {
    format!(
        "{} · {} · {}",
        map_label(&setup.info.map_style),
        age_name(world),
        format_date(at)
    )
}

fn age_name(world: &World) -> String {
    world
        .players()
        .find(|p| p.id == PLAYER)
        .map_or_else(String::new, |p| world.rules().def(p.age).name.clone())
}

/// Writes a save of the running match: the world, the brains and the settings. Call between
/// ticks, on the thread that owns the world.
pub fn write(
    world: &World,
    computers: &ComputerPlayers,
    setup: &SaveSetup,
    speed_pct: u32,
    request: &SaveRequest,
) -> Result<Saved, String> {
    std::fs::create_dir_all(&setup.folder)
        .map_err(|e| format!("creating {}: {e}", setup.folder.display()))?;
    let at = now_secs();
    let (path, name) = match request.kind {
        Kind::Manual => {
            let name = request.name.trim();
            let name = if name.is_empty() {
                default_name(world, setup, at)
            } else {
                name.to_owned()
            };
            (named_path(&setup.folder, &name), name)
        }
        Kind::Quick => (slot_path(&setup.folder, "quicksave"), "Quick save".into()),
        Kind::Auto => (slot_path(&setup.folder, "autosave"), "Autosave".into()),
    };
    let mut info = setup.info.clone();
    info.speed_pct = speed_pct;
    for (key, value) in [
        (KEY_NAME, name.clone()),
        (KEY_KIND, request.kind.as_str().into()),
        (KEY_SAVED_AT, at.to_string()),
        (KEY_AGE, age_name(world)),
    ] {
        info.settings.insert(key.into(), value);
    }
    save_match(&path, world, &info, computers)?;
    Ok(Saved { path, name })
}

/// One save in the folder, read from its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The file.
    pub path: PathBuf,
    /// The name it carries.
    pub name: String,
    /// Why it was made.
    pub kind: Kind,
    /// The display name of the player's age.
    pub age: String,
    /// Game time in ticks.
    pub ticks: u64,
    /// The map style, for display.
    pub map: String,
    /// The map seed.
    pub seed: u64,
    /// Seconds since the Unix epoch when it was made.
    pub saved_at: u64,
    /// Why this game cannot load it, if it cannot.
    pub refusal: Option<String>,
}

impl Entry {
    /// The row's one-line description: age, game time, map and date.
    pub fn summary(&self) -> String {
        format!(
            "{} · {} · {} (seed {}) · {}",
            self.age,
            format_game_time(self.ticks),
            map_label(&self.map),
            self.seed,
            format_date(self.saved_at)
        )
    }
}

/// Every readable save in `folder`, newest first; saves made with other rules than `rules`
/// carry a refusal, and files that are not saves are left out.
pub fn list(folder: &Path, rules: &RulesId) -> Vec<Entry> {
    let Ok(dir) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = dir
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == EXTENSION))
        .filter_map(|path| {
            let header = rom_save::read_header(&path).ok()?;
            let setting = |key: &str| header.info.settings.get(key).cloned().unwrap_or_default();
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            let refusal = (header.rules_version != rules.version
                || header.rules_fingerprint != rules.fingerprint)
                .then(|| {
                    format!(
                        "Made with other rules ({} {}); this game plays {} {}. It cannot be loaded.",
                        header.rules_version,
                        header.rules_fingerprint,
                        rules.version,
                        rules.fingerprint
                    )
                });
            Some(Entry {
                name: Some(setting(KEY_NAME))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| {
                        path.file_stem()
                            .map_or_else(String::new, |s| s.to_string_lossy().into_owned())
                    }),
                kind: Kind::from_text(&setting(KEY_KIND)),
                age: setting(KEY_AGE),
                ticks: header.tick,
                map: header.info.map_style.clone(),
                seed: header.info.seed,
                saved_at: setting(KEY_SAVED_AT).parse().unwrap_or(modified),
                refusal,
                path,
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        b.saved_at
            .cmp(&a.saved_at)
            .then_with(|| a.path.cmp(&b.path))
    });
    entries
}

/// Starts a game process that loads `path`; `display` are window options to pass on.
pub fn relaunch(path: &Path, display: &[String]) -> Result<Child, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Command::new(exe)
        .args(display)
        .arg("--load")
        .arg(path)
        .env_remove("WAYLAND_DISPLAY")
        .spawn()
        .map_err(|e| format!("could not start the game: {e}"))
}

/// The launch arguments of a game that continues the save at `path`: the user's own, then the
/// ones the save carries, so the save's map wins.
pub fn load_arguments(arguments: &[String], path: &Path) -> Result<Vec<String>, String> {
    let header =
        rom_save::read_header(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut all = arguments.to_vec();
    if let Some(saved) = header.info.settings.get(KEY_ARGS) {
        all.extend(saved.lines().map(str::to_owned));
    }
    all.extend(["--seed".into(), header.info.seed.to_string()]);
    if !header.info.map_style.is_empty() {
        all.extend(["--style".into(), header.info.map_style.clone()]);
    }
    Ok(all)
}

/// The window options of this process that a loaded game keeps: size, scale, HUD.
pub fn display_arguments(arguments: &[String]) -> Vec<String> {
    let mut kept = Vec::new();
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--size" | "--scale" | "--hud-scale" | "--grass" => {
                if let Some(value) = rest.next() {
                    kept.extend([argument.clone(), value.clone()]);
                }
            }
            "--no-hud" | "--stats" => kept.push(argument.clone()),
            _ => {}
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_times_read_naturally() {
        assert_eq!(format_date(0), "1970-01-01 00:00");
        assert_eq!(format_date(1_784_000_000), "2026-07-14 03:33");
        assert_eq!(format_game_time(20 * 75), "01:15");
        assert_eq!(format_game_time(20 * 3_725), "1:02:05");
        assert_eq!(map_label("mapstyle.heartland"), "heartland");
    }

    #[test]
    fn names_become_safe_file_names() {
        let folder = Path::new("/saves");
        assert_eq!(
            named_path(folder, "Rome · Bronze Age / 12:30"),
            Path::new("/saves/save-rome-bronze-age-12-30.rom")
        );
        assert_eq!(named_path(folder, "???"), Path::new("/saves/save-game.rom"));
    }

    #[test]
    fn slots_rotate_and_keep_three() {
        let dir = std::env::temp_dir().join(format!("rom-slots-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut used = Vec::new();
        for _ in 0..KEEP {
            let path = slot_path(&dir, "autosave");
            assert!(!used.contains(&path));
            std::fs::write(&path, b"x").unwrap();
            used.push(path);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // All three taken: the oldest is reused, so no fourth file appears.
        assert_eq!(slot_path(&dir, "autosave"), used[0]);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), KEEP);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn display_options_are_the_only_ones_kept() {
        let args: Vec<String> = [
            "--seed", "9", "--size", "900x600", "--stats", "--sphere", "64",
        ]
        .map(String::from)
        .into();
        assert_eq!(display_arguments(&args), ["--size", "900x600", "--stats"]);
    }
}

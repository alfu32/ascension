//! Scripts bundled with the game (task 112): setups a match may start with (`--script`), such
//! as a rich start in any age or a big battle. Their sources are `app/assets/scripts/*.txt`,
//! embedded here; the launcher writes them beside its settings file so a game can read them.
//! Every script begins with two comment lines: its title and one line on what it does.

use std::path::{Path, PathBuf};

/// Every bundled script by name, in the order the launcher lists them.
pub(super) const BUNDLED: &[(&str, &str)] = &[
    (
        "rich-start",
        include_str!("../../../../assets/scripts/rich-start.txt"),
    ),
    (
        "rich-start-00-ancient",
        include_str!("../../../../assets/scripts/rich-start-00-ancient.txt"),
    ),
    (
        "rich-start-01-classical",
        include_str!("../../../../assets/scripts/rich-start-01-classical.txt"),
    ),
    (
        "rich-start-02-medieval",
        include_str!("../../../../assets/scripts/rich-start-02-medieval.txt"),
    ),
    (
        "rich-start-03-gunpowder",
        include_str!("../../../../assets/scripts/rich-start-03-gunpowder.txt"),
    ),
    (
        "rich-start-04-enlightenment",
        include_str!("../../../../assets/scripts/rich-start-04-enlightenment.txt"),
    ),
    (
        "rich-start-05-industrial",
        include_str!("../../../../assets/scripts/rich-start-05-industrial.txt"),
    ),
    (
        "rich-start-06-modern",
        include_str!("../../../../assets/scripts/rich-start-06-modern.txt"),
    ),
    (
        "rich-start-07-information",
        include_str!("../../../../assets/scripts/rich-start-07-information.txt"),
    ),
    (
        "rich-start-08-automation",
        include_str!("../../../../assets/scripts/rich-start-08-automation.txt"),
    ),
    (
        "rich-start-09-fusion",
        include_str!("../../../../assets/scripts/rich-start-09-fusion.txt"),
    ),
    (
        "rich-start-10-starfaring",
        include_str!("../../../../assets/scripts/rich-start-10-starfaring.txt"),
    ),
    (
        "rich-start-11-expansion",
        include_str!("../../../../assets/scripts/rich-start-11-expansion.txt"),
    ),
    (
        "rich-start-12-abundance",
        include_str!("../../../../assets/scripts/rich-start-12-abundance.txt"),
    ),
    (
        "rich-start-13-galactic",
        include_str!("../../../../assets/scripts/rich-start-13-galactic.txt"),
    ),
    (
        "rich-start-14-federation",
        include_str!("../../../../assets/scripts/rich-start-14-federation.txt"),
    ),
    (
        "research-all",
        include_str!("../../../../assets/scripts/research-all.txt"),
    ),
    (
        "big-battle",
        include_str!("../../../../assets/scripts/big-battle.txt"),
    ),
    (
        "fleet-clash",
        include_str!("../../../../assets/scripts/fleet-clash.txt"),
    ),
];

/// A script's title and one-line description, from its first two comment lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct About {
    pub title: String,
    pub line: String,
}

/// The title and description of script `text`; a script without them is named by `fallback`.
pub(super) fn about(text: &str, fallback: &str) -> About {
    let mut comments = text
        .lines()
        .map_while(|line| line.strip_prefix('#'))
        .map(|line| line.trim().to_owned());
    let title = comments.next().filter(|t| !t.is_empty());
    let line = comments.next().unwrap_or_default();
    About {
        title: title.unwrap_or_else(|| fallback.to_owned()),
        line,
    }
}

/// The folder the bundled scripts are written to: `scripts/` beside the launcher's settings.
pub(super) fn folder(settings: &Path) -> PathBuf {
    settings
        .parent()
        .map_or_else(|| PathBuf::from("scripts"), |p| p.join("scripts"))
}

/// Where bundled script `name` lives once written.
pub(super) fn path(settings: &Path, name: &str) -> PathBuf {
    folder(settings).join(format!("{name}.txt"))
}

/// The bundled script a path stands for, if it is one.
pub(super) fn bundled_name(settings: &Path, file: &str) -> Option<&'static str> {
    BUNDLED
        .iter()
        .map(|(name, _)| *name)
        .find(|name| path(settings, name).to_string_lossy() == file)
}

/// Writes every bundled script that is missing or differs.
pub(super) fn install(settings: &Path) -> Result<(), String> {
    let dir = folder(settings);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for (name, text) in BUNDLED {
        let file = path(settings, name);
        if std::fs::read_to_string(&file).ok().as_deref() != Some(*text) {
            std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
        }
    }
    Ok(())
}

/// The title and description of the script at `file` (bundled or the person's own).
pub(super) fn describe(settings: &Path, file: &str) -> Result<About, String> {
    if let Some(name) = bundled_name(settings, file)
        && let Some((_, text)) = BUNDLED.iter().find(|(n, _)| *n == name)
    {
        return Ok(about(text, name));
    }
    let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
    let stem = Path::new(file)
        .file_stem()
        .map_or_else(|| file.to_owned(), |s| s.to_string_lossy().into_owned());
    Ok(about(&text, &stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_script_parses_and_has_a_title() {
        for (name, text) in BUNDLED {
            rom_match::ScriptSetup::scan(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            let about = about(text, name);
            assert_ne!(about.title, *name, "{name} has no title line");
            assert!(!about.line.is_empty(), "{name} has no description line");
        }
        let names: std::collections::BTreeSet<_> = BUNDLED.iter().map(|(n, _)| *n).collect();
        assert_eq!(names.len(), BUNDLED.len());
        let on_disk =
            std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/scripts"))
                .unwrap()
                .count();
        assert_eq!(
            on_disk,
            BUNDLED.len(),
            "every file in assets/scripts is bundled"
        );
    }

    #[test]
    fn installed_scripts_are_found_again() {
        let dir = std::env::temp_dir().join(format!("rom-scripts-{}", std::process::id()));
        let settings = dir.join("launcher.json");
        install(&settings).unwrap();
        let file = path(&settings, "big-battle");
        assert!(file.exists());
        let text = file.to_string_lossy().into_owned();
        assert_eq!(bundled_name(&settings, &text), Some("big-battle"));
        assert_eq!(describe(&settings, &text).unwrap().title, "Big battle");
        let own = dir.join("mine.txt");
        std::fs::write(&own, "0 grant res.food 5\n").unwrap();
        let mine = describe(&settings, &own.to_string_lossy()).unwrap();
        assert_eq!((mine.title.as_str(), mine.line.as_str()), ("mine", ""));
        assert!(describe(&settings, "/no/such/script.txt").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

//! The launcher's Models group: export every model for Meshcraft and import an edited one,
//! with the same code as `rom model export-all` and `rom model import`, on the model files of
//! the source tree this game was built from.

use std::path::{Path, PathBuf};

use rom_models::export::{Library, Paths, export_all};
use rom_models::import::{ImportOptions, import};

/// Folder the export writes to unless the owner picks another: `app/art/models`.
pub(super) fn default_folder() -> PathBuf {
    rom_models::files::default_export_dir()
}

/// Exports every model, done or not, into `folder`; the status line on success.
pub(super) fn export_into(folder: &Path) -> Result<String, String> {
    let lib = Library::open(Paths::default()).map_err(|e| e.to_string())?;
    let s = export_all(&lib, folder).map_err(|e| e.to_string())?;
    Ok(format!(
        "Exported {} models ({} done, {} not done yet) and {} shared sub-assemblies to {}",
        s.done + s.starters,
        s.done,
        s.starters,
        s.shared,
        folder.display()
    ))
}

/// Imports `file` into `db/models`; the status line on success, else what refused it.
pub(super) fn import_file(file: &Path, unlock: bool) -> Result<String, String> {
    let lib = Library::open(Paths::default()).map_err(|e| e.to_string())?;
    let report = import(
        &lib,
        file,
        &ImportOptions {
            model: None,
            unlock,
        },
    )
    .map_err(|e| e.to_string())?;
    let mut line = format!(
        "Imported {}: {} kit parts, {} free-form, {} includes",
        report.id, report.counts[0], report.counts[1], report.counts[2]
    );
    if let (Some((near, far)), Some(fingerprint)) = (report.triangles, &report.fingerprint) {
        line.push_str(&format!(
            "; {near}/{far} triangles, fingerprint {fingerprint}"
        ));
    }
    if !report.unlocked.is_empty() {
        line.push_str(&format!(
            "; approved models changed, new fingerprints: {}",
            report.unlocked.join(", ")
        ));
    }
    if !report.warnings.is_empty() {
        line.push_str(&format!("; warnings: {}", report.warnings.join("; ")));
    }
    line.push_str(". Rebuild the game (cargo run rebuilds) to see it.");
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buttons_run_the_model_tools() {
        let dir = std::env::temp_dir().join(format!("rom-launcher-models-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let status = export_into(&dir).unwrap();
        assert!(status.starts_with("Exported "), "{status}");
        assert!(dir.join("bld.temple@classical.octd").exists());
        assert!(dir.join("bld.temple@classical.md").exists());
        // An unknown file is reported, not a panic; nothing is imported.
        let error = import_file(&dir.join("missing.octd"), false).unwrap_err();
        assert!(error.contains("missing.octd"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

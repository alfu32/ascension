//! The model files: one SQL file per model in `db/models/` (shared sub-assemblies in
//! `db/models/shared/`, the materials models wear in `db/models/materials.sql`) and the
//! geometry of free-form parts in `assets/models/`. They load after the seed, in a fixed order:
//! materials, shared sub-assemblies, then the models, each group by file name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rom_rules::{RulesBuildError, build_rules_db};
use rusqlite::Connection;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// The file of the materials models wear, inside the models folder.
pub const MATERIALS_FILE: &str = "materials.sql";
/// The folder of shared sub-assemblies, inside the models folder.
pub const SHARED_DIR: &str = "shared";

/// The `app/` folder of the source tree this crate was built from.
pub fn app_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Default folder of the model SQL files: `app/db/models`.
pub fn default_models_dir() -> PathBuf {
    app_dir().join("db/models")
}

/// Default folder of free-form part geometry: `app/assets/models`.
pub fn default_assets_dir() -> PathBuf {
    app_dir().join("assets/models")
}

/// Default folder of exported Meshcraft files and briefs: `app/art/models`.
pub fn default_export_dir() -> PathBuf {
    app_dir().join("art/models")
}

/// Default lock file of approved models: `app/docs/art/approved.toml`.
pub fn default_approved_file() -> PathBuf {
    app_dir().join("docs/art/approved.toml")
}

/// Default art direction: `app/docs/art-direction.md`.
pub fn default_art_direction() -> PathBuf {
    app_dir().join("docs/art-direction.md")
}

/// Model SQL files and part meshes, by path relative to their folders (`/`-separated).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelFiles {
    /// SQL files relative to the models folder.
    pub sql: BTreeMap<String, String>,
    /// `.mesh` files relative to the assets folder.
    pub meshes: BTreeMap<String, String>,
}

/// Rank of a model SQL file in the load order.
fn rank(path: &str) -> (u8, &str) {
    if path == MATERIALS_FILE {
        (0, path)
    } else if path.starts_with(&format!("{SHARED_DIR}/")) {
        (1, path)
    } else {
        (2, path)
    }
}

impl ModelFiles {
    /// The files built into this program.
    pub fn embedded() -> Self {
        Self {
            sql: embedded::SQL
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            meshes: embedded::MESHES
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        }
    }

    /// Reads every `.sql` file under `models` and `.mesh` file under `assets` (either folder may
    /// be missing).
    pub fn read(models: &Path, assets: &Path) -> std::io::Result<Self> {
        Ok(Self {
            sql: read_tree(models, "sql")?,
            meshes: read_tree(assets, "mesh")?,
        })
    }

    /// Reads every `.sql` file under `models`, without part meshes (enough to build the rules).
    pub fn read_sql(models: &Path) -> std::io::Result<Self> {
        Ok(Self {
            sql: read_tree(models, "sql")?,
            meshes: BTreeMap::new(),
        })
    }

    /// The SQL files in load order.
    pub fn ordered(&self) -> Vec<(&str, &str)> {
        let mut files: Vec<(&str, &str)> = self
            .sql
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        files.sort_by(|a, b| rank(a.0).cmp(&rank(b.0)));
        files
    }

    /// Every SQL file in load order, as one transaction to run after the seed.
    pub fn script(&self) -> String {
        let mut out = String::from("BEGIN;\n");
        for (path, text) in self.ordered() {
            out.push_str(&format!("-- file: {path}\n"));
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
        }
        out.push_str("COMMIT;\n");
        out
    }

    /// The seed followed by the model files: the text that builds the full rules.
    pub fn seed_with_models(&self, seed: &str) -> String {
        format!("{seed}\n{}", self.script())
    }
}

/// Builds the rules database from the schema, the seed and the model files, checked as
/// [`build_rules_db`] checks it.
pub fn build_rules(
    schema: &str,
    seed: &str,
    files: &ModelFiles,
) -> Result<Connection, RulesBuildError> {
    build_rules_db(schema, &files.seed_with_models(seed))
}

fn read_tree(root: &Path, extension: &str) -> std::io::Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if !root.exists() {
        return Ok(out);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == extension) {
                let relative = path
                    .strip_prefix(root)
                    .map_err(std::io::Error::other)?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(relative, std::fs::read_to_string(&path)?);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_load_first_then_shared_then_models() {
        let mut files = ModelFiles::default();
        for name in [
            "unit.a@any.sql",
            "shared/mesh.b.sql",
            "materials.sql",
            "bld.c@x.sql",
        ] {
            files.sql.insert(name.into(), String::new());
        }
        let order: Vec<&str> = files.ordered().into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            order,
            [
                "materials.sql",
                "shared/mesh.b.sql",
                "bld.c@x.sql",
                "unit.a@any.sql"
            ]
        );
    }

    #[test]
    fn the_embedded_files_are_the_files_on_disk() {
        let disk = ModelFiles::read(&default_models_dir(), &default_assets_dir()).unwrap();
        assert_eq!(ModelFiles::embedded(), disk);
    }
}

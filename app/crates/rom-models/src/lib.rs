//! Building and unit models as files: one SQL file per model (`db/models/`), the rules turned
//! into kit part lists, Meshcraft export and import (`.octd`), the briefs written beside each
//! export, and the lock on approved models. Used by `rom model …` and the launcher alike.

pub mod art;
pub mod book;
pub mod brief;
pub mod export;
pub mod files;
pub mod import;
pub mod lock;
pub mod octd;
pub mod params;
pub mod sql;

use std::path::Path;

use thiserror::Error;

pub use book::{
    COMPOSED, ModelEntry, composed_models, material_look, model_id, model_spec, visual_model_id,
};
pub use files::{ModelFiles, build_rules};

/// Errors of the model tools.
#[derive(Debug, Error)]
pub enum ModelError {
    /// A file could not be read or written.
    #[error("{path}: {error}")]
    Io {
        /// The file.
        path: String,
        /// What went wrong.
        error: std::io::Error,
    },
    /// The rules database refused a query.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    /// The rules could not be built.
    #[error(transparent)]
    Build(#[from] rom_rules::RulesBuildError),
    /// The rules could not be loaded.
    #[error(transparent)]
    Rules(#[from] rom_rules::RulesError),
    /// No model or shared mesh has this id.
    #[error("no model or shared mesh {0:?}")]
    Unknown(String),
    /// A file to import is not usable; every problem found, one per line.
    #[error("{0}")]
    Invalid(String),
    /// The import would change approved models.
    #[error("{0}")]
    Locked(String),
}

impl ModelError {
    /// An I/O error on `path`.
    pub fn io(path: &Path, error: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            error,
        }
    }
}

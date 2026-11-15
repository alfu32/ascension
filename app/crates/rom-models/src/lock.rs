//! The lock on approved models: `docs/art/approved.toml` names each approved model id with the
//! fingerprint of its baked geometry; a model whose fingerprint changes is no longer the one
//! the owner signed off.

use std::collections::BTreeMap;

use crate::book::ModelEntry;

/// The model ids and fingerprints of an `approved.toml` (`"<model id>" = "<fingerprint>"` lines
/// under `[models]`).
pub fn approved(text: &str) -> BTreeMap<String, String> {
    let unquote = |s: &str| s.trim().trim_matches('"').to_owned();
    text.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty() && !line.starts_with('['))
        .filter_map(|line| {
            let (id, fingerprint) = line.split_once('=')?;
            Some((unquote(id), unquote(fingerprint)))
        })
        .collect()
}

/// The fingerprint of `entry` as `approved.toml` writes it.
pub fn fingerprint(entry: &ModelEntry) -> String {
    format!("{:016x}", entry.baked.fingerprint())
}

/// The approved models whose fingerprint no longer matches (or which no longer exist), each
/// with the old and new fingerprint.
pub fn changed(entries: &[ModelEntry], approved: &BTreeMap<String, String>) -> Vec<String> {
    approved
        .iter()
        .filter_map(|(id, locked)| {
            let now = entries.iter().find(|e| &e.id == id).map(fingerprint);
            match now {
                Some(now) if &now == locked => None,
                Some(now) => Some(format!("{id}: approved {locked}, now {now}")),
                None => Some(format!("{id}: approved {locked}, model gone")),
            }
        })
        .collect()
}

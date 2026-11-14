//! Build and validate the rules database.

use std::path::Path;

use rusqlite::{Connection, backup::Backup};
use thiserror::Error;

mod load;
mod model;
mod modifiers;
mod validate;

pub use model::*;
pub use modifiers::Context;
pub use validate::{MAX_STAR_MAP_RADIUS, PLANET_N_RANGE, Problem};

/// Errors encountered while decoding the typed rules model.
#[derive(Debug, Error)]
pub enum RulesError {
    /// SQLite failed to read a table or convert a column to its integer/text type.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    /// A closed vocabulary or boolean column contains an unsupported value.
    #[error("{table} row {row}, {column}: {value:?}; expected {expected}")]
    InvalidValue {
        table: String,
        row: String,
        column: String,
        value: String,
        expected: &'static str,
    },
    /// A component refers to an entity type absent from the database.
    #[error("{table} refers to missing entity type {id:?}")]
    MissingEntity { table: String, id: DefId },
    /// Two parts of one composed mesh share a name.
    #[error("mesh {mesh} has two parts named {name:?}")]
    DuplicatePart { mesh: String, name: String },
    /// Composed meshes include one another in a cycle (the chain of mesh keys).
    #[error("mesh include cycle: {0}")]
    IncludeCycle(String),
    /// A definition id requires a vector larger than the available capacity.
    #[error("cannot allocate definition index for {0:?}")]
    DefinitionCapacity(DefId),
}

impl Rules {
    /// Returns a definition; `id` must belong to this rules catalogue.
    pub fn def(&self, id: DefId) -> &Definition {
        self.definitions[id.0 as usize]
            .as_ref()
            .expect("definition id must belong to this rules catalogue")
    }

    /// Finds a definition by its stable authoring key.
    pub fn id_of(&self, key: &str) -> Option<DefId> {
        self.ids_by_key.get(key).copied()
    }

    /// Returns the stable key of a definition in this catalogue.
    pub fn key(&self, id: DefId) -> &str {
        &self.def(id).key
    }

    /// Counts definitions, excluding unused slots in the id vector.
    pub fn definition_count(&self) -> usize {
        self.ids_by_key.len()
    }

    /// Looks up an entity type and its attached components.
    pub fn entity_type(&self, id: DefId) -> Option<&EntityType> {
        self.entity_types.get(&id)
    }

    /// Tests membership in a tag by its tag-table id.
    pub fn has_tag(&self, def: DefId, tag: u32) -> bool {
        self.tags_by_def
            .get(&def)
            .is_some_and(|tags| tags.contains(&tag))
    }

    /// Returns combat effectiveness, defaulting to 100 percent for a missing row.
    pub fn damage_pct(&self, damage_class: u32, armor_class: u32) -> i64 {
        self.damage_matrix
            .get(&(damage_class, armor_class))
            .map_or(100, |row| row.pct)
    }

    /// Returns a global integer tunable, or `None` when it is absent.
    pub fn constant(&self, key: &str) -> Option<i64> {
        self.game_constants.get(key).map(|row| row.value)
    }

    /// Returns every modifier from a source, including timed, aura and relation effects.
    pub fn modifiers_from(&self, source: DefId) -> &[Modifier] {
        self.modifiers.get(&source).map_or(&[], Vec::as_slice)
    }

    /// The name `def` goes by for `faction`: its own alias, else the nearest ancestor's, else
    /// the definition's own name.
    pub fn display_name(&self, def: DefId, faction: DefId) -> &str {
        if let Some(rows) = self.faction_aliases.get(&def) {
            let mut current = Some(faction);
            let mut visited = std::collections::BTreeSet::new();
            while let Some(id) = current {
                if !visited.insert(id) {
                    break;
                }
                if let Some(row) = rows.iter().find(|row| row.faction_id == id) {
                    return &row.name;
                }
                current = self.factions.get(&id).and_then(|row| row.parent_id);
            }
        }
        &self.def(def).name
    }

    /// The faction of kind `race` at the top of `faction`'s parent chain (the faction itself if
    /// it has no race ancestor).
    pub fn race_of(&self, faction: DefId) -> DefId {
        let mut current = faction;
        let mut visited = std::collections::BTreeSet::new();
        while visited.insert(current) {
            let Some(row) = self.factions.get(&current) else {
                break;
            };
            if row.kind == FactionKind::Race {
                return current;
            }
            match row.parent_id {
                Some(parent) => current = parent,
                None => break,
            }
        }
        faction
    }

    /// Playable factions whose age window contains `age`, in id order.
    pub fn playable_start_factions(&self, age: DefId) -> Vec<DefId> {
        let Some(at) = self.ages.get(&age).map(|a| a.ordinal) else {
            return Vec::new();
        };
        let ordinal = |id: Option<DefId>| id.and_then(|id| self.ages.get(&id)).map(|a| a.ordinal);
        self.factions
            .values()
            .filter(|f| {
                f.playable
                    && ordinal(f.first_age_id).is_none_or(|first| first <= at)
                    && ordinal(f.last_age_id).is_none_or(|last| at <= last)
            })
            .map(|f| f.id)
            .collect()
    }

    /// Tests roster availability, inheriting listings from faction ancestors.
    pub fn is_available(&self, def: DefId, faction: DefId) -> bool {
        let Some(rows) = self.availability.get(&def) else {
            return true;
        };
        let mut current = Some(faction);
        let mut visited = std::collections::BTreeSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                break;
            }
            if rows.iter().any(|row| row.faction_id == id) {
                return true;
            }
            current = self.factions.get(&id).and_then(|row| row.parent_id);
        }
        false
    }

    /// Whether an immediate successor in this upgrade line is unlocked.
    pub fn is_obsolete(&self, def: DefId, active: &ActiveSources, owned: &Owned) -> bool {
        self.entity_types.values().any(|next| {
            next.upgrades_from_id == Some(def) && self.is_unlocked(next.id, active, owned)
        })
    }

    /// Tests every prerequisite OR-group using active techs/ages and owned entity types.
    pub fn is_unlocked(&self, def: DefId, active: &ActiveSources, owned: &Owned) -> bool {
        let mut groups = std::collections::BTreeMap::new();
        for row in self.prerequisites.get(&def).into_iter().flatten() {
            let id = row.requires_def_id;
            let met = if let Some(age) = self.ages.get(&id) {
                active.iter().any(|source| {
                    self.ages
                        .get(source)
                        .is_some_and(|later| later.ordinal >= age.ordinal)
                })
            } else if self.techs.contains_key(&id) {
                active.contains(&id)
            } else {
                self.entity_types.contains_key(&id) && owned.contains(&id)
            };
            *groups.entry(row.alt_group).or_insert(false) |= met;
        }
        groups.values().all(|met| *met)
    }
}

/// Errors encountered while building, checking, or writing the rules database.
#[derive(Debug, Error)]
pub enum RulesBuildError {
    /// A foreign key check found rows that refer to missing records.
    #[error("foreign key check failed: {rows:?}")]
    ForeignKeyViolation { rows: Vec<String> },
    /// A definition is missing its required subtype row.
    #[error("lint_def_without_subtype failed: {rows:?}")]
    MissingSubtype { rows: Vec<String> },
    /// An entity type has no production path.
    #[error("lint_unproducible failed: {rows:?}")]
    Unproducible { rows: Vec<String> },
    /// The seed defined nothing, so there are no rules to ship.
    #[error("rules database is empty: the seed created no definitions")]
    Empty,
    /// SQLite reported an error while processing the database.
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    /// The database file could not be written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Builds an in-memory database and checks its foreign keys and lint views.
pub fn build_rules_db(schema_sql: &str, seed_sql: &str) -> Result<Connection, RulesBuildError> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(schema_sql)?;
    conn.execute_batch("PRAGMA defer_foreign_keys = ON;")?;
    if let Err(error) = conn.execute_batch(seed_sql) {
        let rows = if matches!(
            &error,
            rusqlite::Error::SqliteFailure(code, _)
                if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
        ) {
            foreign_key_rows(&conn).unwrap_or_default()
        } else {
            Vec::new()
        };
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(match error {
            rusqlite::Error::SqliteFailure(code, message)
                if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY =>
            {
                RulesBuildError::ForeignKeyViolation {
                    rows: if rows.is_empty() {
                        vec![message.unwrap_or_else(|| "seed violates a foreign key".into())]
                    } else {
                        rows
                    },
                }
            }
            other => RulesBuildError::Sql(other),
        });
    }

    let definitions: i64 = conn.query_row("SELECT COUNT(*) FROM def", [], |row| row.get(0))?;
    if definitions == 0 {
        return Err(RulesBuildError::Empty);
    }

    let foreign_keys = foreign_key_rows(&conn)?;
    if !foreign_keys.is_empty() {
        return Err(RulesBuildError::ForeignKeyViolation { rows: foreign_keys });
    }

    let missing_subtypes = conn
        .prepare("SELECT id, kind, key FROM lint_def_without_subtype ORDER BY id")?
        .query_map([], |row| {
            Ok(format!(
                "id={}, kind={}, key={}",
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if !missing_subtypes.is_empty() {
        return Err(RulesBuildError::MissingSubtype {
            rows: missing_subtypes,
        });
    }

    let unproducible = conn
        .prepare("SELECT key FROM lint_unproducible ORDER BY key")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if !unproducible.is_empty() {
        return Err(RulesBuildError::Unproducible { rows: unproducible });
    }

    Ok(conn)
}

fn foreign_key_rows(conn: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    conn.prepare("PRAGMA foreign_key_check")?
        .query_map([], |row| {
            Ok(format!(
                "table={}, rowid={}, parent={}, fkid={}",
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?
                    .map_or_else(|| "NULL".into(), |id| id.to_string()),
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect()
}

/// Replaces the database at `path` with a consistent copy of `conn`.
pub fn write_rules_db(conn: &Connection, path: &Path) -> Result<(), RulesBuildError> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    let mut destination = Connection::open(path)?;
    Backup::new(conn, &mut destination)?.run_to_completion(
        128,
        std::time::Duration::from_millis(10),
        None,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RulesBuildError, build_rules_db, write_rules_db};
    use rusqlite::Connection;

    const SCHEMA: &str = include_str!("../../../db/schema.sql");
    const SEED: &str = include_str!("../../../db/seed.sql");

    #[test]
    fn seed_builds_clean() {
        let conn = build_rules_db(SCHEMA, SEED).expect("seed should satisfy the schema");
        let definitions: i64 = conn
            .query_row("SELECT COUNT(*) FROM def", [], |row| row.get(0))
            .unwrap();
        assert!(definitions > 50);
    }

    #[test]
    fn broken_foreign_key_is_reported() {
        let seed =
            format!("{SEED}\nINSERT INTO cost(def_id, resource_id, amount) VALUES (1, 999999, 1);");
        assert!(matches!(
            build_rules_db(SCHEMA, &seed),
            Err(RulesBuildError::ForeignKeyViolation { .. })
        ));
    }

    #[test]
    fn missing_subtype_is_reported() {
        let seed = format!(
            "{SEED}\nINSERT INTO def(kind,key,name) VALUES ('tech','tech.orphan','Orphan');"
        );
        let error = build_rules_db(SCHEMA, &seed).unwrap_err();
        assert!(matches!(error, RulesBuildError::MissingSubtype { .. }));
        assert!(error.to_string().contains("tech.orphan"));
    }

    #[test]
    fn unproducible_is_reported() {
        let seed = format!(
            "{SEED}\nINSERT INTO def(kind,key,name) VALUES ('entity_type','unit.orphan','Orphan');\nINSERT INTO entity_type(id,class) VALUES (last_insert_rowid(),'unit');"
        );
        let error = build_rules_db(SCHEMA, &seed).unwrap_err();
        assert!(matches!(error, RulesBuildError::Unproducible { .. }));
        assert!(error.to_string().contains("unit.orphan"));
    }

    #[test]
    fn empty_seed_is_rejected() {
        assert!(matches!(
            build_rules_db(SCHEMA, ""),
            Err(RulesBuildError::Empty)
        ));
    }

    #[test]
    fn write_and_reopen() {
        let conn = build_rules_db(SCHEMA, SEED).unwrap();
        let expected: i64 = conn
            .query_row("SELECT COUNT(*) FROM def", [], |row| row.get(0))
            .unwrap();
        let path = std::env::temp_dir().join(format!("rom-rules-{}.db", std::process::id()));
        write_rules_db(&conn, &path).unwrap();
        let reopened = Connection::open(&path).unwrap();
        let actual: i64 = reopened
            .query_row("SELECT COUNT(*) FROM def", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, expected);
        std::fs::remove_file(path).unwrap();
    }
}

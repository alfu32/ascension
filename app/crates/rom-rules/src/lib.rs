//! Build and validate the rules database.

use std::path::Path;

use rusqlite::{Connection, backup::Backup};
use thiserror::Error;

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

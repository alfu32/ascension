//! A server's state folder: `server.db` (SQLite: key and seat-token digests, seats, the match
//! setup, the current save's name) and `saves/`. It is the server's own bookkeeping, not the
//! game database: no rules or simulation state live here.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, params};

use crate::NetError;
use crate::keys;

const TABLES: &str = "
CREATE TABLE IF NOT EXISTS net_key (
    digest TEXT PRIMARY KEY,
    created INTEGER NOT NULL,
    used INTEGER,
    host INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS net_seat (
    seat INTEGER PRIMARY KEY,
    token_digest TEXT,
    name TEXT NOT NULL,
    host INTEGER NOT NULL DEFAULT 0,
    faction TEXT NOT NULL,
    human INTEGER NOT NULL,
    spec TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS net_match (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    setup TEXT NOT NULL,
    started INTEGER NOT NULL,
    save TEXT,
    tick INTEGER NOT NULL DEFAULT 0,
    speed INTEGER NOT NULL
);
";

/// The state folder's database.
pub struct StateDb {
    conn: Connection,
    dir: PathBuf,
}

/// What a key admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCheck {
    /// Unused: it admits one person (a host when `host`).
    Fresh {
        /// The key also makes its seat the host.
        host: bool,
    },
    /// Used already.
    Used,
    /// Never issued.
    Unknown,
}

/// Unused and used keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyCounts {
    /// Keys nobody has used yet.
    pub unused: u64,
    /// Keys that admitted someone.
    pub used: u64,
}

/// A seat as the state database keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatRow {
    /// The seat.
    pub seat: u32,
    /// Its person's token digest; none while nobody holds it.
    pub token_digest: Option<String>,
    /// Its person's name.
    pub name: String,
    /// May start and set the speed.
    pub host: bool,
    /// The faction's key.
    pub faction: String,
    /// A person's seat (a computer holds it while nobody does).
    pub human: bool,
    /// The computer player's `profile:difficulty` for it.
    pub spec: String,
}

/// The match as the state database keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchRow {
    /// The `MatchSetup`, as JSON.
    pub setup: String,
    /// Whether the match has started.
    pub started: bool,
    /// The latest save's file name in `saves/`.
    pub save: Option<String>,
    /// The tick of that save.
    pub tick: u64,
    /// Percent of 20 ticks a second.
    pub speed: u32,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

impl StateDb {
    /// Opens (making it if needed) the state folder `dir`, its `saves/` and `server.db`.
    pub fn open(dir: &Path) -> Result<Self, NetError> {
        std::fs::create_dir_all(dir.join("saves"))?;
        let conn = Connection::open(dir.join("server.db"))?;
        // `rom keys add` writes while the server runs.
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(TABLES)?;
        Ok(Self {
            conn,
            dir: dir.to_owned(),
        })
    }

    /// The state folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The folder saves go to.
    pub fn saves(&self) -> PathBuf {
        self.dir.join("saves")
    }

    /// Makes `n` new keys, stores their digests and returns them: the only time they are seen.
    pub fn add_keys(&self, n: u32, host: bool) -> Result<Vec<String>, NetError> {
        let mut out = Vec::new();
        for _ in 0..n {
            let key = keys::new_key()?;
            self.conn.execute(
                "INSERT INTO net_key(digest, created, host) VALUES (?1, ?2, ?3)",
                params![keys::digest(&key), now(), host],
            )?;
            out.push(key);
        }
        Ok(out)
    }

    /// Unused and used keys.
    pub fn key_counts(&self) -> Result<KeyCounts, NetError> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) - COUNT(used), COUNT(used) FROM net_key",
            [],
            |row| {
                Ok(KeyCounts {
                    unused: row.get::<_, i64>(0)? as u64,
                    used: row.get::<_, i64>(1)? as u64,
                })
            },
        )?)
    }

    /// What `key` admits, without using it.
    pub fn check_key(&self, key: &str) -> Result<KeyCheck, NetError> {
        let row: Option<(Option<i64>, bool)> = self
            .conn
            .query_row(
                "SELECT used, host FROM net_key WHERE digest = ?1",
                params![keys::digest(key)],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        Ok(match row {
            None => KeyCheck::Unknown,
            Some((Some(_), _)) => KeyCheck::Used,
            Some((None, host)) => KeyCheck::Fresh { host },
        })
    }

    /// Uses `key`: true when it was unused (and now is used).
    pub fn use_key(&self, key: &str) -> Result<bool, NetError> {
        let changed = self.conn.execute(
            "UPDATE net_key SET used = ?1 WHERE digest = ?2 AND used IS NULL",
            params![now(), keys::digest(key)],
        )?;
        Ok(changed == 1)
    }

    /// Replaces the stored seats.
    pub fn save_seats(&self, seats: &[SeatRow]) -> Result<(), NetError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM net_seat", [])?;
        for seat in seats {
            tx.execute(
                "INSERT INTO net_seat(seat, token_digest, name, host, faction, human, spec) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    seat.seat,
                    seat.token_digest,
                    seat.name,
                    seat.host,
                    seat.faction,
                    seat.human,
                    seat.spec
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The stored seats, in order.
    pub fn seats(&self) -> Result<Vec<SeatRow>, NetError> {
        let mut statement = self.conn.prepare(
            "SELECT seat, token_digest, name, host, faction, human, spec FROM net_seat \
             ORDER BY seat",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(SeatRow {
                seat: row.get(0)?,
                token_digest: row.get(1)?,
                name: row.get(2)?,
                host: row.get(3)?,
                faction: row.get(4)?,
                human: row.get(5)?,
                spec: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Stores the match.
    pub fn save_match(&self, row: &MatchRow) -> Result<(), NetError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO net_match(id, setup, started, save, tick, speed) \
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![row.setup, row.started, row.save, row.tick as i64, row.speed],
        )?;
        Ok(())
    }

    /// The stored match, if any.
    pub fn match_row(&self) -> Result<Option<MatchRow>, NetError> {
        Ok(self
            .conn
            .query_row(
                "SELECT setup, started, save, tick, speed FROM net_match WHERE id = 1",
                [],
                |row| {
                    Ok(MatchRow {
                        setup: row.get(0)?,
                        started: row.get(1)?,
                        save: row.get(2)?,
                        tick: row.get::<_, i64>(3)? as u64,
                        speed: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// Forgets the match and its seats (a new match in this folder); keys stay.
    pub fn clear_match(&self) -> Result<(), NetError> {
        self.conn
            .execute_batch("DELETE FROM net_match; DELETE FROM net_seat;")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rom-net-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_key_is_used_once_and_only_its_digest_is_stored() {
        let dir = folder("keys");
        let db = StateDb::open(&dir).unwrap();
        let keys = db.add_keys(2, false).unwrap();
        assert_eq!(db.key_counts().unwrap(), KeyCounts { unused: 2, used: 0 });
        assert_eq!(
            db.check_key(&keys[0]).unwrap(),
            KeyCheck::Fresh { host: false }
        );
        assert!(db.use_key(&keys[0].to_lowercase()).unwrap());
        assert!(!db.use_key(&keys[0]).unwrap());
        assert_eq!(db.check_key(&keys[0]).unwrap(), KeyCheck::Used);
        assert_eq!(db.check_key("NOPE").unwrap(), KeyCheck::Unknown);
        assert_eq!(db.key_counts().unwrap(), KeyCounts { unused: 1, used: 1 });
        let host = db.add_keys(1, true).unwrap();
        assert_eq!(
            db.check_key(&host[0]).unwrap(),
            KeyCheck::Fresh { host: true }
        );
        // A second connection (another process: `rom keys add`) sees and adds keys.
        let other = StateDb::open(&dir).unwrap();
        other.add_keys(1, false).unwrap();
        assert_eq!(db.key_counts().unwrap().unused, 3);
        let stored: String = db
            .conn
            .query_row("SELECT group_concat(digest) FROM net_key", [], |r| r.get(0))
            .unwrap();
        assert!(!stored.contains(&keys::normalise(&keys[1])));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn seats_and_the_match_are_kept() {
        let dir = folder("seats");
        let db = StateDb::open(&dir).unwrap();
        assert_eq!(db.match_row().unwrap(), None);
        let seats = vec![
            SeatRow {
                seat: 0,
                token_digest: Some("abc".into()),
                name: "Ada".into(),
                host: true,
                faction: "nation.a".into(),
                human: true,
                spec: "balanced:moderate".into(),
            },
            SeatRow {
                seat: 1,
                token_digest: None,
                name: String::new(),
                host: false,
                faction: "nation.b".into(),
                human: false,
                spec: "builder:tough".into(),
            },
        ];
        db.save_seats(&seats).unwrap();
        let row = MatchRow {
            setup: "{}".into(),
            started: true,
            save: Some("autosave.rom".into()),
            tick: 1200,
            speed: 150,
        };
        db.save_match(&row).unwrap();
        drop(db);
        let db = StateDb::open(&dir).unwrap();
        assert_eq!(db.seats().unwrap(), seats);
        assert_eq!(db.match_row().unwrap(), Some(row));
        db.clear_match().unwrap();
        assert!(db.seats().unwrap().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

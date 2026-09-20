//! Forward-only schema migrations driven by SQLite's `user_version` pragma.
//!
//! Each entry runs inside a transaction together with the version bump, so a
//! migration either lands completely or not at all — a power cut mid-upgrade
//! leaves a consistent database at the previous version.

use crate::error::Result;
use rusqlite::Connection;

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../../migrations/001_init.sql"),
    },
    Migration {
        version: 2,
        name: "thinking_tokens",
        sql: include_str!("../../migrations/002_thinking_tokens.sql"),
    },
];

pub fn latest_version() -> i64 {
    MIGRATIONS.last().map(|m| m.version).unwrap_or(0)
}

fn current_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

pub fn migrate(conn: &mut Connection) -> Result<i64> {
    let from = current_version(conn)?;
    let to = latest_version();

    if from > to {
        // Opening a v3 database with a v2 binary would silently misbehave.
        // Refuse instead, so the user can reinstall rather than lose data.
        return Err(crate::error::AppError::internal(format!(
            "This database was created by a newer version of OpenClaude \
             (schema v{from}; this build supports v{to}). Please update the app."
        )));
    }

    for m in MIGRATIONS.iter().filter(|m| m.version > from) {
        tracing::info!(version = m.version, name = m.name, "applying migration");
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql)?;
        // PRAGMA does not accept bound parameters; version is a trusted constant.
        tx.execute_batch(&format!("PRAGMA user_version = {}", m.version))?;
        tx.commit()?;
    }

    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        c
    }

    #[test]
    fn migrates_empty_database_to_latest() {
        let mut c = mem();
        assert_eq!(current_version(&c).unwrap(), 0);
        assert_eq!(migrate(&mut c).unwrap(), latest_version());
        assert_eq!(current_version(&c).unwrap(), latest_version());
    }

    #[test]
    fn migration_is_idempotent() {
        let mut c = mem();
        migrate(&mut c).unwrap();
        // Running again must be a no-op, not an "table already exists" error.
        migrate(&mut c).unwrap();
        assert_eq!(current_version(&c).unwrap(), latest_version());
    }

    #[test]
    fn expected_tables_exist_after_migration() {
        let mut c = mem();
        migrate(&mut c).unwrap();
        for t in [
            "conversations",
            "messages",
            "projects",
            "attachments",
            "settings",
            "mcp_servers",
            "mcp_permissions",
            "messages_fts",
            "conversations_fts",
            "attachments_fts",
        ] {
            let n: i64 = c
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name = ?1",
                    [t],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "missing table {t}");
        }
    }

    #[test]
    fn refuses_to_open_a_future_database() {
        let mut c = mem();
        c.execute_batch("PRAGMA user_version = 9999").unwrap();
        let err = migrate(&mut c).unwrap_err().to_string();
        assert!(err.contains("newer version"), "{err}");
    }

    #[test]
    fn versions_are_unique_and_ascending() {
        let mut prev = 0;
        for m in MIGRATIONS {
            assert!(m.version > prev, "migration versions must ascend");
            prev = m.version;
        }
    }
}

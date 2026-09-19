pub mod migrations;
pub mod models;
pub mod repo;

use crate::error::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// The database handle.
///
/// A single connection behind a mutex rather than a pool: SQLite serialises
/// writers anyway, every operation here is a sub-millisecond indexed query,
/// and the guard is never held across an `.await`. That keeps the dependency
/// list short and makes lock ordering trivial to reason about.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        Self::harden_file_permissions(path)?;
        let mut conn = conn;
        migrations::migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        migrations::migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            -- Concurrent readers during a streaming write, and far better
            -- crash resilience than the rollback journal.
            PRAGMA journal_mode = WAL;
            -- WAL + NORMAL is durable across app crashes; only an OS-level
            -- crash can lose the last transaction, which for us is a few
            -- hundred streamed characters we can recover from.
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA temp_store = MEMORY;
            -- Keep the WAL from growing unboundedly during long sessions.
            PRAGMA journal_size_limit = 67108864;
            PRAGMA cache_size = -16000;
            ",
        )?;
        Ok(())
    }

    /// The database holds full conversation text; keep it owner-only.
    fn harden_file_permissions(path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for p in [
                path.to_path_buf(),
                path.with_extension("db-wal"),
                path.with_extension("db-shm"),
            ] {
                if let Ok(md) = std::fs::metadata(&p) {
                    let mut perms = md.permissions();
                    if perms.mode() & 0o077 != 0 {
                        perms.set_mode(0o600);
                        let _ = std::fs::set_permissions(&p, perms);
                    }
                }
            }
        }
        Ok(())
    }

    /// Panics only if a previous holder panicked mid-transaction, in which
    /// case the data is suspect and failing loudly is correct.
    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Run `f` inside a transaction, rolling back on error.
    pub fn tx<T>(&self, f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>) -> Result<T> {
        let mut guard = self.conn();
        let tx = guard.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    /// Copy the live database to `dest` using SQLite's online backup API, which
    /// is safe to run while the app is writing.
    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.conn();
        let mut out = Connection::open(dest)?;
        let backup = rusqlite::backup::Backup::new(&conn, &mut out)?;
        backup.run_to_completion(256, std::time::Duration::from_millis(10), None)?;
        Ok(())
    }

    /// `PRAGMA integrity_check` — surfaced in Settings → Advanced.
    pub fn integrity_check(&self) -> Result<String> {
        let conn = self.conn();
        Ok(conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?)
    }

    pub fn vacuum(&self) -> Result<()> {
        self.conn().execute_batch("VACUUM")?;
        Ok(())
    }

    pub fn size_bytes(&self) -> Result<i64> {
        let conn = self.conn();
        let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok(pages * size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_database_opens_and_migrates() {
        let db = Db::open_in_memory().unwrap();
        let v: i64 = db
            .conn()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, migrations::latest_version());
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Db::open_in_memory().unwrap();
        let err = db.conn().execute(
            "INSERT INTO messages (id, conversation_id, seq, role, content, created_at, updated_at)
             VALUES ('m1', 'does-not-exist', 0, 'user', 'hi', 0, 0)",
            [],
        );
        assert!(err.is_err(), "orphan message must be rejected");
    }

    #[test]
    fn file_database_uses_wal_and_is_owner_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Db::open(&path).unwrap();
        let mode: String = db
            .conn()
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(m & 0o077, 0, "database must not be group/world readable");
        }
    }

    #[test]
    fn backup_produces_a_readable_copy() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("a.db")).unwrap();
        let dest = dir.path().join("backup/b.db");
        db.backup_to(&dest).unwrap();
        let copy = Db::open(&dest).unwrap();
        assert_eq!(copy.integrity_check().unwrap(), "ok");
    }
}

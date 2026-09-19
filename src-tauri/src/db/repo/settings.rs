//! Key/value application settings.
//!
//! Values are JSON so a setting can grow from a bool into a struct without a
//! migration. Secrets never live here — they go to the keyring.

use crate::db::models::now_ms;
use crate::error::Result;
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};

pub fn get_raw(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    match rows.next()? {
        Some(r) => Ok(Some(r.get(0)?)),
        None => Ok(None),
    }
}

pub fn set_raw(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now_ms()],
    )?;
    Ok(())
}

/// Read a typed setting, falling back to `default` when absent *or corrupt*.
/// A malformed value must not brick the app on launch.
pub fn get_or<T: DeserializeOwned>(conn: &Connection, key: &str, default: T) -> T {
    match get_raw(conn, key) {
        Ok(Some(raw)) => serde_json::from_str(&raw).unwrap_or_else(|e| {
            tracing::warn!(key, error = %e, "ignoring malformed setting");
            default
        }),
        _ => default,
    }
}

pub fn set<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<()> {
    set_raw(conn, key, &serde_json::to_string(value)?)
}

pub fn delete(conn: &Connection, key: &str) -> Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
    Ok(())
}

/// Every setting as one JSON object, for the settings UI and data export.
pub fn all(conn: &Connection) -> Result<serde_json::Map<String, serde_json::Value>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
    let mut out = serde_json::Map::new();
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (k, v) = row?;
        out.insert(
            k,
            serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v)),
        );
    }
    Ok(out)
}

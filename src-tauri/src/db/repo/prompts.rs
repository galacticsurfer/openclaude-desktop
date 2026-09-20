use crate::db::models::{new_id, now_ms, Prompt};
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, Row};

const COLS: &str = "id, title, body, use_count, last_used_at, created_at, updated_at";

fn map(row: &Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        use_count: row.get(3)?,
        last_used_at: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

/// Most recently used first, then never-used ones by name — so the picker
/// opens on what the user actually reaches for.
pub fn list(conn: &Connection) -> Result<Vec<Prompt>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM prompts
          ORDER BY last_used_at DESC NULLS LAST, title COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], map)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, id: &str) -> Result<Prompt> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM prompts WHERE id = ?1"))?;
    let mut rows = stmt.query_map([id], map)?;
    rows.next()
        .transpose()?
        .ok_or_else(|| AppError::NotFound("That prompt no longer exists."))
}

pub fn create(conn: &Connection, title: &str, body: &str) -> Result<Prompt> {
    let now = now_ms();
    let id = new_id();
    conn.execute(
        "INSERT INTO prompts (id, title, body, use_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, 0, ?4, ?4)",
        params![id, title, body, now],
    )?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, title: &str, body: &str) -> Result<Prompt> {
    let n = conn.execute(
        "UPDATE prompts SET title = ?2, body = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, title, body, now_ms()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("That prompt no longer exists."));
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM prompts WHERE id = ?1", [id])?;
    Ok(())
}

/// Record that a prompt was inserted into the composer.
pub fn mark_used(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE prompts SET use_count = use_count + 1, last_used_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    Ok(())
}

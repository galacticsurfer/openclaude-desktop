use crate::db::models::*;
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, Row};

const COLS: &str = "id, conversation_id, message_id, project_id, filename, mime_type,
     size_bytes, kind, storage_path, sha256, text_content, created_at";

fn map(row: &Row<'_>) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        message_id: row.get(2)?,
        project_id: row.get(3)?,
        filename: row.get(4)?,
        mime_type: row.get(5)?,
        size_bytes: row.get(6)?,
        kind: AttachmentKind::parse(&row.get::<_, String>(7)?),
        storage_path: row.get(8)?,
        sha256: row.get(9)?,
        text_content: row.get(10)?,
        created_at: row.get(11)?,
    })
}

pub struct NewAttachment<'a> {
    pub conversation_id: Option<&'a str>,
    pub project_id: Option<&'a str>,
    pub filename: &'a str,
    pub mime_type: &'a str,
    pub size_bytes: i64,
    pub kind: AttachmentKind,
    pub storage_path: &'a str,
    pub sha256: &'a str,
    pub text_content: Option<&'a str>,
}

pub fn insert(conn: &Connection, a: NewAttachment<'_>) -> Result<Attachment> {
    let id = new_id();
    conn.execute(
        "INSERT INTO attachments
            (id, conversation_id, project_id, filename, mime_type, size_bytes, kind,
             storage_path, sha256, text_content, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            a.conversation_id,
            a.project_id,
            a.filename,
            a.mime_type,
            a.size_bytes,
            a.kind.as_str(),
            a.storage_path,
            a.sha256,
            a.text_content,
            now_ms()
        ],
    )?;
    get(conn, &id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Attachment> {
    conn.query_row(
        &format!("SELECT {COLS} FROM attachments WHERE id = ?1"),
        [id],
        map,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Attachment"),
        other => other.into(),
    })
}

/// Bind pending (message-less) attachments to the message being sent.
pub fn attach_to_message(conn: &Connection, ids: &[String], message_id: &str) -> Result<()> {
    for id in ids {
        conn.execute(
            "UPDATE attachments SET message_id = ?2 WHERE id = ?1",
            params![id, message_id],
        )?;
    }
    Ok(())
}

pub fn for_message(conn: &Connection, message_id: &str) -> Result<Vec<Attachment>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM attachments WHERE message_id = ?1 ORDER BY created_at"
    ))?;
    let rows = stmt
        .query_map([message_id], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn for_conversation(conn: &Connection, conversation_id: &str) -> Result<Vec<Attachment>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM attachments WHERE conversation_id = ?1 ORDER BY created_at"
    ))?;
    let rows = stmt
        .query_map([conversation_id], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn for_project(conn: &Connection, project_id: &str) -> Result<Vec<Attachment>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM attachments WHERE project_id = ?1 ORDER BY created_at"
    ))?;
    let rows = stmt
        .query_map([project_id], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
    Ok(())
}

/// How many rows still reference a blob. The store is content-addressed and
/// deduplicated, so a file on disk may only be removed once this hits zero.
pub fn refcount(conn: &Connection, sha256: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM attachments WHERE sha256 = ?1",
        [sha256],
        |r| r.get(0),
    )?)
}

/// Blobs on disk that no row points at any more (orphaned by a purge).
pub fn all_referenced_paths(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT storage_path FROM attachments")?;
    let rows = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Attachments uploaded but never sent — cleaned up on startup.
pub fn delete_dangling(conn: &Connection, older_than: Millis) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM attachments
          WHERE message_id IS NULL AND project_id IS NULL AND created_at < ?1",
        [older_than],
    )?)
}

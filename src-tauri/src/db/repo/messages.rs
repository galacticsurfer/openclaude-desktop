use crate::db::models::*;
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, Row};

const COLS: &str = "id, conversation_id, seq, role, content, thinking, status, model,
     provider_message_id, stop_reason, input_tokens, output_tokens, cache_read_tokens,
     cache_write_tokens, error_kind, error_message, created_at, updated_at, metadata";

fn map(row: &Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        seq: row.get(2)?,
        role: Role::parse(&row.get::<_, String>(3)?),
        content: row.get(4)?,
        thinking: row.get(5)?,
        status: MessageStatus::parse(&row.get::<_, String>(6)?),
        model: row.get(7)?,
        provider_message_id: row.get(8)?,
        stop_reason: row.get(9)?,
        input_tokens: row.get(10)?,
        output_tokens: row.get(11)?,
        cache_read_tokens: row.get(12)?,
        cache_write_tokens: row.get(13)?,
        error_kind: row.get(14)?,
        error_message: row.get(15)?,
        created_at: row.get(16)?,
        updated_at: row.get(17)?,
        metadata: parse_json_object(&row.get::<_, String>(18)?),
        attachments: Vec::new(),
    })
}

/// Next ordering slot. Derived from `max(seq)` rather than a running count so
/// deleting a message never causes a duplicate `seq` on the next insert.
pub fn next_seq(conn: &Connection, conversation_id: &str) -> Result<i64> {
    let max: Option<i64> = conn.query_row(
        "SELECT max(seq) FROM messages WHERE conversation_id = ?1",
        [conversation_id],
        |r| r.get(0),
    )?;
    Ok(max.map_or(0, |m| m + 1))
}

pub struct NewMessage<'a> {
    pub conversation_id: &'a str,
    pub role: Role,
    pub content: &'a str,
    pub status: MessageStatus,
    pub model: Option<&'a str>,
}

pub fn insert(conn: &Connection, m: NewMessage<'_>) -> Result<Message> {
    let id = new_id();
    let now = now_ms();
    let seq = next_seq(conn, m.conversation_id)?;
    conn.execute(
        "INSERT INTO messages
            (id, conversation_id, seq, role, content, status, model, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            id,
            m.conversation_id,
            seq,
            m.role.as_str(),
            m.content,
            m.status.as_str(),
            m.model,
            now
        ],
    )?;
    get(conn, &id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Message> {
    let mut msg = conn
        .query_row(
            &format!("SELECT {COLS} FROM messages WHERE id = ?1"),
            [id],
            map,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Message"),
            other => AppError::from(other),
        })?;
    msg.attachments = super::attachments::for_message(conn, id)?;
    Ok(msg)
}

pub fn list(conn: &Connection, conversation_id: &str) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM messages WHERE conversation_id = ?1 ORDER BY seq"
    ))?;
    let mut msgs: Vec<Message> = stmt
        .query_map([conversation_id], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    // One extra query for all attachments in the conversation, then fan out,
    // rather than one query per message.
    let atts = super::attachments::for_conversation(conn, conversation_id)?;
    for a in atts {
        if let Some(mid) = a.message_id.clone() {
            if let Some(m) = msgs.iter_mut().find(|m| m.id == mid) {
                m.attachments.push(a);
            }
        }
    }
    Ok(msgs)
}

/// Messages up to and including `seq` — the history a branch inherits.
pub fn list_through_seq(
    conn: &Connection,
    conversation_id: &str,
    seq: i64,
) -> Result<Vec<Message>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM messages WHERE conversation_id = ?1 AND seq <= ?2 ORDER BY seq"
    ))?;
    let mut msgs: Vec<Message> = stmt
        .query_map(params![conversation_id, seq], map)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let atts = super::attachments::for_conversation(conn, conversation_id)?;
    for a in atts {
        if let Some(mid) = a.message_id.clone() {
            if let Some(m) = msgs.iter_mut().find(|m| m.id == mid) {
                m.attachments.push(a);
            }
        }
    }
    Ok(msgs)
}

/// Overwrite the accumulated text of a streaming message.
///
/// The caller owns the authoritative buffer, so a full write is both simpler
/// and safer than an `||` append: a retried or duplicated flush can never
/// double up the text.
pub fn update_stream_buffer(
    conn: &Connection,
    id: &str,
    content: &str,
    thinking: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE messages SET content = ?2, thinking = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, content, thinking, now_ms()],
    )?;
    Ok(())
}

#[derive(Debug, Default, Clone)]
pub struct Completion<'a> {
    pub content: &'a str,
    /// Concrete model that produced this, when the backend reports one.
    pub model: Option<&'a str>,
    pub thinking: Option<&'a str>,
    pub status: Option<MessageStatus>,
    pub provider_message_id: Option<&'a str>,
    pub stop_reason: Option<&'a str>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
}

pub fn finalize(conn: &Connection, id: &str, c: Completion<'_>) -> Result<()> {
    conn.execute(
        "UPDATE messages
            SET content = ?2, thinking = ?3, status = ?4, provider_message_id = ?5,
                stop_reason = ?6, input_tokens = ?7, output_tokens = ?8,
                cache_read_tokens = ?9, cache_write_tokens = ?10,
                model = COALESCE(?12, model),
                error_kind = NULL, error_message = NULL, updated_at = ?11
          WHERE id = ?1",
        params![
            id,
            c.content,
            c.thinking,
            c.status.unwrap_or(MessageStatus::Complete).as_str(),
            c.provider_message_id,
            c.stop_reason,
            c.input_tokens,
            c.output_tokens,
            c.cache_read_tokens,
            c.cache_write_tokens,
            now_ms(),
            c.model
        ],
    )?;
    Ok(())
}

/// Record a failure without discarding whatever text already streamed in.
pub fn fail(conn: &Connection, id: &str, kind: &str, message: &str, partial: &str) -> Result<()> {
    conn.execute(
        "UPDATE messages
            SET status = 'error', error_kind = ?2, error_message = ?3,
                content = ?4, updated_at = ?5
          WHERE id = ?1",
        params![id, kind, message, partial, now_ms()],
    )?;
    Ok(())
}

pub fn mark_interrupted(conn: &Connection, id: &str, partial: &str) -> Result<()> {
    conn.execute(
        "UPDATE messages SET status = 'interrupted', content = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, partial, now_ms()],
    )?;
    Ok(())
}

pub fn set_content(conn: &Connection, id: &str, content: &str) -> Result<()> {
    conn.execute(
        "UPDATE messages SET content = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, content, now_ms()],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM messages WHERE id = ?1", [id])?;
    Ok(())
}

/// Drop `from_seq` and everything after it. Used by "retry", which removes the
/// failed assistant turn (and anything below) before regenerating.
pub fn delete_from_seq(conn: &Connection, conversation_id: &str, from_seq: i64) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM messages WHERE conversation_id = ?1 AND seq >= ?2",
        params![conversation_id, from_seq],
    )?)
}

pub fn last(conn: &Connection, conversation_id: &str) -> Result<Option<Message>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM messages WHERE conversation_id = ?1 ORDER BY seq DESC LIMIT 1"
    ))?;
    let mut rows = stmt.query_map([conversation_id], map)?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

/// Crash recovery.
///
/// Any message still marked `pending`/`streaming` at startup belonged to a run
/// that died with the process. Downgrade it to `interrupted` so the UI offers
/// Retry/Continue instead of spinning forever on a stream that will never
/// produce another token.
pub fn recover_in_flight(conn: &Connection) -> Result<usize> {
    let n = conn.execute(
        "UPDATE messages
            SET status = CASE WHEN content = '' AND thinking IS NULL THEN 'error' ELSE 'interrupted' END,
                error_kind = CASE WHEN content = '' AND thinking IS NULL THEN 'interrupted' ELSE error_kind END,
                error_message = CASE WHEN content = '' AND thinking IS NULL
                                     THEN 'This response was interrupted before it began.'
                                     ELSE error_message END,
                updated_at = ?1
          WHERE status IN ('pending', 'streaming')",
        [now_ms()],
    )?;
    if n > 0 {
        tracing::info!(
            count = n,
            "recovered messages left in flight by a previous run"
        );
    }
    Ok(n)
}

pub fn count_in_conversation(conn: &Connection, conversation_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM messages WHERE conversation_id = ?1",
        [conversation_id],
        |r| r.get(0),
    )?)
}

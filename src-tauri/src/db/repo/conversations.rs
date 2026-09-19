use crate::db::models::*;
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, Row};

const COLS: &str = "id, title, title_locked, provider, provider_conversation_id, model,
     system_prompt, project_id, pinned, archived, deleted_at, branched_from_message_id,
     created_at, updated_at, last_message_at, metadata";

/// Same columns as [`COLS`], qualified with the `c` alias for joined queries.
/// The two must stay in the same order — [`map`] reads them positionally.
const COLS_C: &str =
    "c.id, c.title, c.title_locked, c.provider, c.provider_conversation_id, c.model,
     c.system_prompt, c.project_id, c.pinned, c.archived, c.deleted_at, c.branched_from_message_id,
     c.created_at, c.updated_at, c.last_message_at, c.metadata";

fn map(row: &Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        title: row.get(1)?,
        title_locked: row.get::<_, i64>(2)? != 0,
        provider: row.get(3)?,
        provider_conversation_id: row.get(4)?,
        model: row.get(5)?,
        system_prompt: row.get(6)?,
        project_id: row.get(7)?,
        pinned: row.get::<_, i64>(8)? != 0,
        archived: row.get::<_, i64>(9)? != 0,
        deleted_at: row.get(10)?,
        branched_from_message_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        last_message_at: row.get(14)?,
        metadata: parse_json_object(&row.get::<_, String>(15)?),
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewConversation {
    pub title: Option<String>,
    pub model: String,
    pub project_id: Option<String>,
    pub system_prompt: Option<String>,
}

pub fn create(conn: &Connection, input: NewConversation) -> Result<Conversation> {
    let now = now_ms();
    let id = new_id();
    let title = input
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "New conversation".to_string());

    conn.execute(
        "INSERT INTO conversations
            (id, title, model, project_id, system_prompt, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            id,
            title,
            input.model,
            input.project_id,
            input.system_prompt,
            now
        ],
    )?;
    get(conn, &id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Conversation> {
    conn.query_row(
        &format!("SELECT {COLS} FROM conversations WHERE id = ?1"),
        [id],
        map,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Conversation"),
        other => other.into(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListScope {
    Active,
    Archived,
    Trash,
}

impl ListScope {
    fn predicate(self) -> &'static str {
        match self {
            Self::Active => "c.deleted_at IS NULL AND c.archived = 0",
            Self::Archived => "c.deleted_at IS NULL AND c.archived = 1",
            Self::Trash => "c.deleted_at IS NOT NULL",
        }
    }
}

/// One query for the whole sidebar: conversation, counters, preview and
/// project name. Avoids the N+1 that would otherwise show up at 10k rows.
pub fn list(
    conn: &Connection,
    scope: ListScope,
    project_id: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ConversationSummary>> {
    let project_clause = if project_id.is_some() {
        "AND c.project_id = ?3"
    } else {
        ""
    };
    let sql = format!(
        "SELECT {cols},
                (SELECT count(*) FROM messages m WHERE m.conversation_id = c.id) AS message_count,
                (SELECT substr(m.content, 1, 160) FROM messages m
                  WHERE m.conversation_id = c.id AND m.content <> ''
                  ORDER BY m.seq DESC LIMIT 1) AS preview,
                p.name AS project_name
           FROM conversations c
           LEFT JOIN projects p ON p.id = c.project_id
          WHERE {pred} {project_clause}
          ORDER BY c.pinned DESC,
                   COALESCE(c.last_message_at, c.updated_at) DESC
          LIMIT ?1 OFFSET ?2",
        cols = COLS_C,
        pred = scope.predicate(),
    );

    let mut stmt = conn.prepare(&sql)?;
    let mapper = |row: &Row<'_>| -> rusqlite::Result<ConversationSummary> {
        Ok(ConversationSummary {
            conversation: map(row)?,
            message_count: row.get(16)?,
            preview: row.get(17)?,
            project_name: row.get(18)?,
        })
    };

    let rows = match project_id {
        Some(p) => stmt
            .query_map(params![limit, offset, p], mapper)?
            .collect::<rusqlite::Result<Vec<_>>>()?,
        None => stmt
            .query_map(params![limit, offset], mapper)?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    };
    Ok(rows)
}

pub fn rename(conn: &Connection, id: &str, title: &str) -> Result<()> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError::invalid("A conversation title cannot be empty."));
    }
    // A manual rename locks the title so auto-titling never overwrites it.
    let n = conn.execute(
        "UPDATE conversations SET title = ?2, title_locked = 1, updated_at = ?3 WHERE id = ?1",
        params![id, title, now_ms()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("Conversation"));
    }
    Ok(())
}

/// Used by the auto-titler: only writes when the user has not chosen a title.
pub fn set_generated_title(conn: &Connection, id: &str, title: &str) -> Result<bool> {
    let n = conn.execute(
        "UPDATE conversations SET title = ?2, title_locked = 1, updated_at = ?3
          WHERE id = ?1 AND title_locked = 0",
        params![id, title.trim(), now_ms()],
    )?;
    Ok(n > 0)
}

pub fn set_model(conn: &Connection, id: &str, model: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET model = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, model, now_ms()],
    )?;
    Ok(())
}

pub fn set_pinned(conn: &Connection, id: &str, pinned: bool) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET pinned = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, pinned as i64, now_ms()],
    )?;
    Ok(())
}

pub fn set_archived(conn: &Connection, id: &str, archived: bool) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET archived = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, archived as i64, now_ms()],
    )?;
    Ok(())
}

pub fn set_project(conn: &Connection, id: &str, project_id: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET project_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, project_id, now_ms()],
    )?;
    Ok(())
}

pub fn set_system_prompt(conn: &Connection, id: &str, prompt: Option<&str>) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET system_prompt = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, prompt, now_ms()],
    )?;
    Ok(())
}

/// Soft delete — the row moves to Trash and stays recoverable.
pub fn trash(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    Ok(())
}

pub fn restore(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET deleted_at = NULL, updated_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    )?;
    Ok(())
}

/// Irreversible. Messages and attachments cascade.
pub fn purge(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM conversations WHERE id = ?1", [id])?;
    Ok(())
}

pub fn purge_trash_older_than(conn: &Connection, cutoff: Millis) -> Result<usize> {
    Ok(conn.execute(
        "DELETE FROM conversations WHERE deleted_at IS NOT NULL AND deleted_at < ?1",
        [cutoff],
    )?)
}

pub fn touch(conn: &Connection, id: &str, at: Millis) -> Result<()> {
    conn.execute(
        "UPDATE conversations SET last_message_at = ?2, updated_at = ?2 WHERE id = ?1",
        params![id, at],
    )?;
    Ok(())
}

pub fn usage(conn: &Connection, id: &str) -> Result<UsageTotals> {
    Ok(conn.query_row(
        "SELECT COALESCE(sum(input_tokens), 0), COALESCE(sum(output_tokens), 0),
                COALESCE(sum(cache_read_tokens), 0), COALESCE(sum(cache_write_tokens), 0),
                count(*)
           FROM messages WHERE conversation_id = ?1",
        [id],
        |r| {
            Ok(UsageTotals {
                input_tokens: r.get(0)?,
                output_tokens: r.get(1)?,
                cache_read_tokens: r.get(2)?,
                cache_write_tokens: r.get(3)?,
                message_count: r.get(4)?,
            })
        },
    )?)
}

pub fn count(conn: &Connection, scope: ListScope) -> Result<i64> {
    let sql = format!(
        "SELECT count(*) FROM conversations c WHERE {}",
        scope.predicate()
    );
    Ok(conn.query_row(&sql, [], |r| r.get(0))?)
}

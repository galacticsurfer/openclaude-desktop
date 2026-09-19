use crate::db::models::*;
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, Row};

const COLS: &str = "id, name, description, instructions, working_dir, default_model,
     color, sort_order, archived, created_at, updated_at, metadata";

fn map(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        instructions: row.get(3)?,
        working_dir: row.get(4)?,
        default_model: row.get(5)?,
        color: row.get(6)?,
        sort_order: row.get(7)?,
        archived: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        metadata: parse_json_object(&row.get::<_, String>(11)?),
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    pub working_dir: Option<String>,
    pub default_model: Option<String>,
    pub color: Option<String>,
}

pub fn create(conn: &Connection, input: ProjectInput) -> Result<Project> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid("A project needs a name."));
    }
    let id = new_id();
    let now = now_ms();
    conn.execute(
        "INSERT INTO projects
            (id, name, description, instructions, working_dir, default_model, color,
             sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7,
                 (SELECT COALESCE(max(sort_order), 0) + 1 FROM projects), ?8, ?8)",
        params![
            id,
            name,
            input.description,
            input.instructions,
            input.working_dir,
            input.default_model,
            input.color,
            now
        ],
    )?;
    get(conn, &id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Project> {
    conn.query_row(
        &format!("SELECT {COLS} FROM projects WHERE id = ?1"),
        [id],
        map,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Project"),
        other => other.into(),
    })
}

pub fn list(conn: &Connection, include_archived: bool) -> Result<Vec<ProjectSummary>> {
    let pred = if include_archived {
        "1 = 1"
    } else {
        "p.archived = 0"
    };
    let sql = format!(
        "SELECT p.id, p.name, p.description, p.instructions, p.working_dir, p.default_model,
                p.color, p.sort_order, p.archived, p.created_at, p.updated_at, p.metadata,
                (SELECT count(*) FROM conversations c
                  WHERE c.project_id = p.id AND c.deleted_at IS NULL) AS conversation_count
           FROM projects p
          WHERE {pred}
          ORDER BY p.sort_order, p.name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectSummary {
                project: map(row)?,
                conversation_count: row.get(12)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn update(conn: &Connection, id: &str, input: ProjectInput) -> Result<Project> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid("A project needs a name."));
    }
    let n = conn.execute(
        "UPDATE projects
            SET name = ?2, description = ?3, instructions = ?4, working_dir = ?5,
                default_model = ?6, color = ?7, updated_at = ?8
          WHERE id = ?1",
        params![
            id,
            name,
            input.description,
            input.instructions,
            input.working_dir,
            input.default_model,
            input.color,
            now_ms()
        ],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("Project"));
    }
    get(conn, id)
}

pub fn set_archived(conn: &Connection, id: &str, archived: bool) -> Result<()> {
    conn.execute(
        "UPDATE projects SET archived = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, archived as i64, now_ms()],
    )?;
    Ok(())
}

/// Conversations survive: the FK is `ON DELETE SET NULL`, so they fall back to
/// the ungrouped list rather than disappearing with the project.
pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    Ok(())
}

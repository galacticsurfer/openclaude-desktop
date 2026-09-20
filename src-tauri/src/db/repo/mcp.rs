use crate::db::models::{new_id, now_ms};
use crate::error::{AppError, Result};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    /// `stdio`, `sse` or `http`.
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: serde_json::Map<String, serde_json::Value>,
    pub url: Option<String>,
    pub enabled: bool,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMcpServer {
    pub name: String,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: serde_json::Map<String, serde_json::Value>,
    pub url: Option<String>,
    pub project_id: Option<String>,
}

const COLS: &str = "id, name, transport, command, args, env, url, enabled, project_id";

fn map(row: &Row<'_>) -> rusqlite::Result<McpServer> {
    let args: String = row.get(4)?;
    let env: String = row.get(5)?;
    Ok(McpServer {
        id: row.get(0)?,
        name: row.get(1)?,
        transport: row.get(2)?,
        command: row.get(3)?,
        args: serde_json::from_str(&args).unwrap_or_default(),
        env: serde_json::from_str(&env).unwrap_or_default(),
        url: row.get(6)?,
        enabled: row.get::<_, i64>(7)? != 0,
        project_id: row.get(8)?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<McpServer>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM mcp_servers ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map([], map)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get(conn: &Connection, id: &str) -> Result<McpServer> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM mcp_servers WHERE id = ?1"))?;
    let mut rows = stmt.query_map([id], map)?;
    rows.next()
        .transpose()?
        .ok_or_else(|| AppError::NotFound("That MCP server no longer exists."))
}

pub fn create(conn: &Connection, s: &NewMcpServer) -> Result<McpServer> {
    let id = new_id();
    let now = now_ms();
    conn.execute(
        "INSERT INTO mcp_servers
            (id, name, transport, command, args, env, url, enabled, project_id,
             created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?9)",
        params![
            id,
            s.name,
            s.transport,
            s.command,
            serde_json::to_string(&s.args)?,
            serde_json::to_string(&s.env)?,
            s.url,
            s.project_id,
            now
        ],
    )?;
    get(conn, &id)
}

/// Enable or disable a server. Disabled servers are never written into the
/// config the CLI reads, so their tools cannot exist at all.
pub fn set_enabled(conn: &Connection, id: &str, enabled: bool) -> Result<()> {
    let n = conn.execute(
        "UPDATE mcp_servers SET enabled = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, i64::from(enabled), now_ms()],
    )?;
    if n == 0 {
        return Err(AppError::NotFound("That MCP server no longer exists."));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM mcp_servers WHERE id = ?1", [id])?;
    Ok(())
}

// --- permissions ----------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermission {
    pub id: String,
    pub server_id: String,
    pub tool_name: String,
    pub category: String,
    /// `allow`, `deny` or `ask`.
    pub decision: String,
}

pub fn permissions(conn: &Connection) -> Result<Vec<ToolPermission>> {
    let mut stmt = conn.prepare(
        "SELECT id, server_id, tool_name, category, decision
           FROM mcp_permissions ORDER BY tool_name",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ToolPermission {
            id: r.get(0)?,
            server_id: r.get(1)?,
            tool_name: r.get(2)?,
            category: r.get(3)?,
            decision: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Record a decision for one tool, replacing any previous one.
pub fn decide(
    conn: &Connection,
    server_id: &str,
    tool_name: &str,
    category: &str,
    decision: &str,
) -> Result<()> {
    if !matches!(decision, "allow" | "deny" | "ask") {
        return Err(AppError::invalid("Unknown permission decision."));
    }
    conn.execute(
        "INSERT INTO mcp_permissions
            (id, server_id, project_id, tool_name, category, decision, scope, created_at)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, 'global', ?6)
         ON CONFLICT (server_id, project_id, tool_name)
           DO UPDATE SET decision = excluded.decision, category = excluded.category",
        params![new_id(), server_id, tool_name, category, decision, now_ms()],
    )?;
    Ok(())
}

/// Tool names explicitly allowed, as the CLI names them.
pub fn allowed_tool_names(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT s.name, p.tool_name
           FROM mcp_permissions p
           JOIN mcp_servers s ON s.id = p.server_id
          WHERE p.decision = 'allow' AND s.enabled = 1",
    )?;
    let rows = stmt.query_map([], |r| {
        let server: String = r.get(0)?;
        let tool: String = r.get(1)?;
        Ok(format!("mcp__{server}__{tool}"))
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

use crate::db::repo::mcp as repo;
use crate::error::{AppError, Result};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn list_mcp_servers(state: State<'_, Arc<AppState>>) -> Result<Vec<repo::McpServer>> {
    repo::list(&state.db.conn())
}

#[tauri::command]
pub fn add_mcp_server(
    state: State<'_, Arc<AppState>>,
    server: repo::NewMcpServer,
) -> Result<repo::McpServer> {
    if server.name.trim().is_empty() {
        return Err(AppError::invalid("An MCP server needs a name."));
    }
    // The name becomes part of every tool id (`mcp__<name>__<tool>`), so a
    // name with separators in it would make permissions ambiguous.
    if !server
        .name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::invalid(
            "A server name may use letters, digits, dashes and underscores only.",
        ));
    }
    match server.transport.as_str() {
        "stdio" if server.command.trim().is_empty() => {
            Err(AppError::invalid("A stdio server needs a command to run."))
        }
        "sse" | "http" if server.url.as_deref().unwrap_or("").trim().is_empty() => {
            Err(AppError::invalid("A remote server needs a URL."))
        }
        "stdio" | "sse" | "http" => repo::create(&state.db.conn(), &server),
        _ => Err(AppError::invalid("Unknown MCP transport.")),
    }
}

#[tauri::command]
pub fn set_mcp_server_enabled(
    state: State<'_, Arc<AppState>>,
    id: String,
    enabled: bool,
) -> Result<()> {
    repo::set_enabled(&state.db.conn(), &id, enabled)
}

#[tauri::command]
pub fn delete_mcp_server(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::delete(&state.db.conn(), &id)
}

#[tauri::command]
pub fn list_mcp_permissions(state: State<'_, Arc<AppState>>) -> Result<Vec<repo::ToolPermission>> {
    repo::permissions(&state.db.conn())
}

#[tauri::command]
pub fn decide_mcp_tool(
    state: State<'_, Arc<AppState>>,
    server_id: String,
    tool_name: String,
    category: String,
    decision: String,
) -> Result<()> {
    repo::decide(
        &state.db.conn(),
        &server_id,
        &tool_name,
        &category,
        &decision,
    )
}

/// Ask a configured server what tools it offers, without granting any.
///
/// Runs the CLI with only this server configured and reads the tool list it
/// reports at startup. Nothing is approved as a result — the user still has
/// to decide on each tool.
#[tauri::command]
pub async fn discover_mcp_tools(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<Vec<String>> {
    let server = repo::get(&state.db.conn(), &id)?;
    crate::provider::claude_code::discover_mcp_tools(&server).await
}

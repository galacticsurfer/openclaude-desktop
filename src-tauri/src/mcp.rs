//! Turning stored MCP servers into something the CLI will accept, safely.
//!
//! Two rules hold everywhere in here:
//!
//!  * A server the user has not enabled is never written into the config, so
//!    its tools do not exist for that session — not "exist but denied".
//!  * A tool with no explicit `allow` is not in `--allowedTools`, and with
//!    `--permission-prompts none` anything that would have asked is denied
//!    instead of hanging on a prompt no GUI can answer.
//!
//! The result is a pre-approval model: permission is granted before a tool
//! can run, not while it is running. That is a deliberate trade — a
//! just-in-time prompt needs the CLI to call back into a permission tool we
//! would have to host — and it is the safer half of the trade.

use crate::db::repo::mcp as repo;
use crate::db::Db;
use crate::error::Result;
use serde_json::{json, Map, Value};

/// What a session needs in order to use MCP, if anything.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct McpPlan {
    /// Config file for `--mcp-config`. None means no MCP at all.
    pub config: Option<std::path::PathBuf>,
    /// Fully-qualified tool names for `--allowedTools`.
    pub allowed: Vec<String>,
}

impl McpPlan {
    pub fn is_empty(&self) -> bool {
        self.config.is_none()
    }
}

/// Build the `mcpServers` object the CLI reads.
///
/// Only enabled servers appear. Returns None when none are, so the caller
/// can skip writing a file and leave the session with no MCP at all.
pub fn config_for(servers: &[repo::McpServer]) -> Option<Value> {
    let mut map = Map::new();
    for s in servers.iter().filter(|s| s.enabled) {
        let entry = match s.transport.as_str() {
            "sse" | "http" => json!({
                "type": s.transport,
                "url": s.url.clone().unwrap_or_default(),
            }),
            // stdio is the default and the only one that runs a process.
            _ => json!({
                "type": "stdio",
                "command": s.command,
                "args": s.args,
                "env": s.env,
            }),
        };
        map.insert(s.name.clone(), entry);
    }
    if map.is_empty() {
        return None;
    }
    Some(json!({ "mcpServers": map }))
}

/// Write the config for this run and list the tools it may use.
///
/// The file is rewritten per run rather than kept in sync: it must reflect
/// what is enabled *now*, and a stale file would grant a server the user
/// just turned off.
pub fn plan(db: &Db, dir: &std::path::Path, project_id: Option<&str>) -> Result<McpPlan> {
    let (servers, allowed) = {
        let conn = db.conn();
        (
            repo::for_project(&conn, project_id)?,
            repo::allowed_tool_names(&conn, project_id)?,
        )
    };

    let Some(config) = config_for(&servers) else {
        return Ok(McpPlan::default());
    };

    // An enabled server with no approved tool would start a process that can
    // do nothing. Skip the whole thing instead.
    if allowed.is_empty() {
        return Ok(McpPlan::default());
    }

    std::fs::create_dir_all(dir)?;
    let path = dir.join("mcp-config.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&config)?)?;

    Ok(McpPlan {
        config: Some(path),
        allowed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(name: &str, enabled: bool) -> repo::McpServer {
        repo::McpServer {
            id: format!("id-{name}"),
            name: name.into(),
            transport: "stdio".into(),
            command: "/usr/bin/thing".into(),
            args: vec!["--serve".into()],
            env: Map::new(),
            url: None,
            enabled,
            project_id: None,
        }
    }

    #[test]
    fn a_disabled_server_is_absent_rather_than_present_and_denied() {
        let cfg = config_for(&[server("files", false), server("notes", true)]).unwrap();
        let servers = cfg["mcpServers"].as_object().unwrap();
        assert!(servers.contains_key("notes"));
        assert!(
            !servers.contains_key("files"),
            "a disabled server must not reach the CLI at all"
        );
    }

    #[test]
    fn no_enabled_servers_means_no_config_at_all() {
        assert!(config_for(&[server("files", false)]).is_none());
        assert!(config_for(&[]).is_none());
    }

    #[test]
    fn a_stdio_server_carries_its_command_and_a_remote_one_its_url() {
        let cfg = config_for(&[server("notes", true)]).unwrap();
        assert_eq!(cfg["mcpServers"]["notes"]["type"], "stdio");
        assert_eq!(cfg["mcpServers"]["notes"]["command"], "/usr/bin/thing");

        let mut remote = server("api", true);
        remote.transport = "http".into();
        remote.url = Some("https://example.invalid/mcp".into());
        let cfg = config_for(&[remote]).unwrap();
        assert_eq!(cfg["mcpServers"]["api"]["type"], "http");
        assert_eq!(
            cfg["mcpServers"]["api"]["url"],
            "https://example.invalid/mcp"
        );
    }

    #[test]
    fn an_empty_plan_reports_itself_as_empty() {
        assert!(McpPlan::default().is_empty());
    }
}

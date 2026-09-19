use crate::db::repo;
use crate::error::{AppError, Result};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: &'static str,
    pub database_path: String,
    pub data_dir: String,
    pub config_dir: String,
    pub log_dir: String,
    pub schema_version: i64,
    pub dev_mode: bool,
}

#[tauri::command]
pub fn app_info(state: State<'_, Arc<AppState>>) -> Result<AppInfo> {
    let schema_version: i64 = state
        .db
        .conn()
        .query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        database_path: crate::paths::database_path().to_string_lossy().into_owned(),
        data_dir: crate::paths::data_dir().to_string_lossy().into_owned(),
        config_dir: crate::paths::config_dir().to_string_lossy().into_owned(),
        log_dir: crate::paths::log_dir().to_string_lossy().into_owned(),
        schema_version,
        dev_mode: crate::dev_mode(),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStats {
    pub database_bytes: i64,
    pub attachment_bytes: i64,
    pub conversation_count: i64,
    pub message_count: i64,
    pub attachment_count: i64,
    pub integrity: String,
}

#[tauri::command]
pub fn storage_stats(state: State<'_, Arc<AppState>>) -> Result<StorageStats> {
    let conn = state.db.conn();
    let conversation_count: i64 = conn.query_row(
        "SELECT count(*) FROM conversations WHERE deleted_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    let message_count: i64 = conn.query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?;
    let attachment_count: i64 =
        conn.query_row("SELECT count(*) FROM attachments", [], |r| r.get(0))?;
    let attachment_bytes: i64 = conn
        .query_row(
            "SELECT COALESCE(sum(size_bytes), 0) FROM (SELECT DISTINCT sha256, size_bytes FROM attachments)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    drop(conn);

    Ok(StorageStats {
        database_bytes: state.db.size_bytes()?,
        attachment_bytes,
        conversation_count,
        message_count,
        attachment_count,
        integrity: state.db.integrity_check()?,
    })
}

#[tauri::command]
pub fn backup_database(state: State<'_, Arc<AppState>>, path: String) -> Result<()> {
    let p = std::path::Path::new(&path);
    if !p.is_absolute() {
        return Err(AppError::invalid(
            "Choose a location to save the backup to.",
        ));
    }
    state.db.backup_to(p)
}

#[tauri::command]
pub fn vacuum_database(state: State<'_, Arc<AppState>>) -> Result<()> {
    state.db.vacuum()
}

/// Delete every conversation, message and attachment. Settings and the API
/// key are untouched. The UI must confirm first.
#[tauri::command]
pub fn clear_all_conversations(state: State<'_, Arc<AppState>>) -> Result<()> {
    state.db.tx(|tx| {
        tx.execute("DELETE FROM conversations", [])?;
        tx.execute("DELETE FROM attachments", [])?;
        Ok(())
    })?;
    // Now that no rows reference them, drop every blob.
    let root = crate::paths::attachments_dir();
    if root.exists() {
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::create_dir_all(&root);
    }
    state.db.vacuum()
}

/// Everything the app holds about the user, as one JSON document.
/// Deliberately excludes credentials.
#[tauri::command]
pub fn export_all_data(state: State<'_, Arc<AppState>>) -> Result<String> {
    let conn = state.db.conn();
    let conversations = repo::conversations::list(
        &conn,
        repo::conversations::ListScope::Active,
        None,
        100_000,
        0,
    )?;

    let mut out = Vec::new();
    for c in &conversations {
        let messages = repo::messages::list(&conn, &c.conversation.id)?;
        out.push(serde_json::json!({
            "conversation": c.conversation,
            "messages": messages,
        }));
    }

    let doc = serde_json::json!({
        "exportedBy": concat!("OpenClaude Desktop ", env!("CARGO_PKG_VERSION")),
        "exportedAt": crate::db::models::now_ms(),
        "projects": repo::projects::list(&conn, true)?,
        "settings": repo::settings::all(&conn)?,
        "conversations": out,
    });
    Ok(serde_json::to_string_pretty(&doc)?)
}

/// Remove blobs on disk that no row references. Safe to run any time.
#[tauri::command]
pub fn prune_orphan_attachments(state: State<'_, Arc<AppState>>) -> Result<usize> {
    let referenced: std::collections::HashSet<String> =
        repo::attachments::all_referenced_paths(&state.db.conn())?
            .into_iter()
            .collect();
    let root = crate::paths::attachments_dir();
    let mut removed = 0;

    let Ok(shards) = std::fs::read_dir(&root) else {
        return Ok(0);
    };
    for shard in shards.flatten() {
        if !shard.path().is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(shard.path()) else {
            continue;
        };
        for f in files.flatten() {
            let rel = f
                .path()
                .strip_prefix(&root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !referenced.contains(&rel) && std::fs::remove_file(f.path()).is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

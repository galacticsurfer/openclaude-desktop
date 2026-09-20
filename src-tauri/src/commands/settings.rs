use crate::db::repo::settings as repo;
use crate::error::{AppError, Result};
use crate::provider::ModelInfo;
use crate::settings_defaults as sk;
use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

/// Defaults overlaid with anything stored, so the UI always gets a complete
/// object and never has to know a default.
#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Value> {
    let mut out = sk::defaults();
    let stored = repo::all(&state.db.conn())?;
    if let Some(map) = out.as_object_mut() {
        for (k, v) in stored {
            map.insert(k, v);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn set_setting(state: State<'_, Arc<AppState>>, key: String, value: Value) -> Result<()> {
    // Only known keys, so a compromised renderer cannot stuff the table.
    if !sk::defaults()
        .as_object()
        .map(|m| m.contains_key(&key))
        .unwrap_or(false)
    {
        return Err(AppError::invalid(format!("Unknown setting \"{key}\".")));
    }
    repo::set(&state.db.conn(), &key, &value)
}

#[tauri::command]
pub fn set_settings(state: State<'_, Arc<AppState>>, values: Value) -> Result<()> {
    let obj = values
        .as_object()
        .ok_or_else(|| AppError::invalid("Expected an object of settings."))?;
    let known = sk::defaults();
    let known = known.as_object().unwrap();
    state.db.tx(|tx| {
        for (k, v) in obj {
            if known.contains_key(k) {
                repo::set(tx, k, v)?;
            }
        }
        Ok(())
    })
}

#[tauri::command]
pub fn reset_setting(state: State<'_, Arc<AppState>>, key: String) -> Result<()> {
    repo::delete(&state.db.conn(), &key)
}

/// Models available to the configured key.
///
/// Served from a short-lived cache. When the API cannot be reached we return
/// the built-in list flagged `fromFallback`, so the user can still pick a
/// model offline and the UI can say the list may be incomplete.
/// Models the Claude Code CLI accepts.
///
/// Aliases rather than pinned ids: the CLI resolves `opus`/`sonnet`/`haiku`
/// to whatever is current, so this cannot go stale the way a hardcoded list
/// would. No network call and no credential is involved.
#[tauri::command]
pub async fn list_models(_refresh: Option<bool>) -> Result<ModelListResult> {
    Ok(ModelListResult {
        models: crate::provider::claude_code::models(),
        stale: false,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResult {
    pub models: Vec<ModelInfo>,
    /// True when this list came from cache or the fallback rather than a
    /// fresh call, so the UI can label it honestly.
    pub stale: bool,
}

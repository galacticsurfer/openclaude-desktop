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
/// Models the Claude Code CLI accepts, asked of the CLI itself.
///
/// `/model` is answered locally and costs nothing, so this is a real query
/// rather than a hardcoded list — which aliases exist changes with the CLI
/// version. Cached for the process because it only changes on upgrade.
#[tauri::command]
pub async fn list_models(app: tauri::AppHandle, refresh: Option<bool>) -> Result<ModelListResult> {
    use tauri::Manager;
    let state = app.state::<Arc<AppState>>().inner().clone();

    if !refresh.unwrap_or(false) {
        let cache = state.models.lock().unwrap();
        if !cache.models.is_empty() {
            return Ok(ModelListResult {
                models: cache.models.clone(),
                current: cache.current.clone(),
                effort: cache.effort.clone(),
                stale: false,
            });
        }
    }

    let (models, catalog) = crate::provider::claude_code::models().await;
    let discovered = !catalog.available.is_empty();
    {
        let mut cache = state.models.lock().unwrap();
        cache.models = models.clone();
        cache.current = catalog.current.clone();
        cache.effort = catalog.effort.clone();
    }

    Ok(ModelListResult {
        models,
        current: catalog.current,
        effort: catalog.effort,
        // Only "stale" when the CLI could not be asked and we fell back.
        stale: !discovered,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResult {
    pub models: Vec<ModelInfo>,
    /// The model the CLI reports as currently in use, if it said.
    pub current: Option<String>,
    /// The effort the CLI reports as currently applied.
    pub effort: Option<String>,
    /// True when this list came from cache or the fallback rather than a
    /// fresh call, so the UI can label it honestly.
    pub stale: bool,
}

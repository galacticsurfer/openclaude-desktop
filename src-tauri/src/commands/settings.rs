use crate::db::repo::settings as repo;
use crate::error::{AppError, Result};
use crate::provider::{AIProvider, ModelInfo};
use crate::settings_defaults as sk;
use crate::state::AppState;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::State;

/// Model list is refetched at most this often; it changes rarely and the
/// sidebar should not wait on the network.
const MODEL_TTL: Duration = Duration::from_secs(60 * 30);

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
#[tauri::command]
pub async fn list_models(app: tauri::AppHandle, refresh: Option<bool>) -> Result<ModelListResult> {
    use tauri::Manager;
    let state = app.state::<Arc<AppState>>().inner().clone();

    if !refresh.unwrap_or(false) {
        let cache = state.models.lock().unwrap();
        if let Some(at) = cache.fetched_at {
            if at.elapsed() < MODEL_TTL && !cache.models.is_empty() {
                return Ok(ModelListResult {
                    models: cache.models.clone(),
                    stale: false,
                });
            }
        }
    }

    let (key, base_url) = {
        let conn = state.db.conn();
        (
            crate::secrets::get_api_key("anthropic")?,
            repo::get_or(&conn, sk::BASE_URL, None),
        )
    };

    let Some(key) = key else {
        return Ok(ModelListResult {
            models: crate::provider::anthropic::fallback_models(),
            stale: true,
        });
    };

    let provider = crate::provider::anthropic::AnthropicProvider::new(key, base_url)?;
    match provider.list_models().await {
        Ok(models) if !models.is_empty() => {
            let mut cache = state.models.lock().unwrap();
            cache.models = models.clone();
            cache.fetched_at = Some(Instant::now());
            Ok(ModelListResult {
                models,
                stale: false,
            })
        }
        Ok(_) => Ok(ModelListResult {
            models: crate::provider::anthropic::fallback_models(),
            stale: true,
        }),
        Err(e) => {
            tracing::warn!(error = %e, "falling back to the built-in model list");
            let cache = state.models.lock().unwrap();
            let models = if cache.models.is_empty() {
                crate::provider::anthropic::fallback_models()
            } else {
                cache.models.clone()
            };
            Ok(ModelListResult {
                models,
                stale: true,
            })
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResult {
    pub models: Vec<ModelInfo>,
    /// True when this list came from cache or the fallback rather than a
    /// fresh call, so the UI can label it honestly.
    pub stale: bool,
}

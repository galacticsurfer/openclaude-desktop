//! Credential commands.
//!
//! Note what is *absent*: there is no command that returns the API key. The
//! frontend can set it, test it, delete it, and ask whether one exists — it
//! can never read one back.

use crate::error::{AppError, Result};
use crate::provider::{AIProvider, ModelInfo};
use crate::secrets::{self, CredentialStatus};
use crate::settings_defaults as sk;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn credential_status() -> CredentialStatus {
    secrets::status("anthropic")
}

#[tauri::command]
pub fn set_api_key(app: AppHandle, key: String) -> Result<CredentialStatus> {
    secrets::looks_like_anthropic_key(&key).map_err(AppError::invalid)?;
    secrets::set_api_key("anthropic", &key)?;
    // A new key may unlock a different model set.
    app.state::<Arc<AppState>>()
        .models
        .lock()
        .unwrap()
        .fetched_at = None;
    Ok(secrets::status("anthropic"))
}

#[tauri::command]
pub fn delete_api_key(app: AppHandle) -> Result<CredentialStatus> {
    secrets::delete_api_key("anthropic")?;
    app.state::<Arc<AppState>>()
        .models
        .lock()
        .unwrap()
        .fetched_at = None;
    Ok(secrets::status("anthropic"))
}

/// Validate a key without storing it — the onboarding "Test connection"
/// button, which must not save a bad key just to check it.
#[tauri::command]
pub async fn test_api_key(app: AppHandle, key: Option<String>) -> Result<TestResult> {
    let key = match key {
        Some(k) if !k.trim().is_empty() => {
            secrets::looks_like_anthropic_key(&k).map_err(AppError::invalid)?;
            k.trim().to_string()
        }
        _ => secrets::get_api_key("anthropic")?.ok_or(AppError::MissingCredentials)?,
    };

    let base_url: Option<String> = {
        let state = app.state::<Arc<AppState>>();
        let conn = state.db.conn();
        crate::db::repo::settings::get_or(&conn, sk::BASE_URL, None)
    };

    let provider = crate::provider::anthropic::AnthropicProvider::new(key, base_url)?;
    let models = provider.list_models().await?;
    Ok(TestResult {
        ok: true,
        model_count: models.len(),
        models,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub model_count: usize,
    pub models: Vec<ModelInfo>,
}

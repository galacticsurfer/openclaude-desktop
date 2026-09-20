//! Credential commands.
//!
//! Note what is *absent*: there is no command that returns the API key. The
//! frontend can set it, test it, delete it, and ask whether one exists — it
//! can never read one back.

use crate::error::{AppError, Result};
use crate::provider::{AIProvider, ModelInfo};
use crate::secrets::{self, AuthMode, Credential, CredentialStatus};
use crate::settings_defaults as sk;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// The *effective* credential state — the single answer to "can this app
/// talk to Claude right now".
///
/// It must account for the sign-in mode, not just the keyring: with browser
/// sign-in configured there is no stored key at all, and reporting
/// `configured: false` would leave the composer disabled and the sidebar
/// claiming "No API key" while the app is perfectly able to send.
#[tauri::command]
pub fn credential_status(app: AppHandle) -> CredentialStatus {
    let state = app.state::<Arc<AppState>>();
    let mode: AuthMode = {
        let conn = state.db.conn();
        crate::db::repo::settings::get_or(&conn, sk::AUTH_MODE, AuthMode::ApiKey)
    };

    let mut status = secrets::status("anthropic");
    status.mode = mode;

    if mode == AuthMode::Oauth {
        let profile: Option<String> = {
            let conn = state.db.conn();
            crate::db::repo::settings::get_or(&conn, sk::OAUTH_PROFILE, None)
        };
        let cli = crate::oauth::status(profile.as_deref());
        status.configured = cli.signed_in;
        // There is no stored key to hint at; the profile name is the label.
        status.hint = cli.profile.or_else(|| Some("signed in".to_string()));
    }

    status
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
    let state = app.state::<Arc<AppState>>().inner().clone();

    // An explicitly supplied key is being tested before it is saved; with no
    // key, test whatever the app is currently configured to use — which may
    // be an OAuth profile rather than a stored key.
    let credential = match key {
        Some(k) if !k.trim().is_empty() => {
            secrets::looks_like_anthropic_key(&k).map_err(AppError::invalid)?;
            Credential::ApiKey(k.trim().to_string())
        }
        _ => crate::chat::resolve_credential(&state.db)?,
    };

    let base_url: Option<String> = {
        let conn = state.db.conn();
        crate::db::repo::settings::get_or(&conn, sk::BASE_URL, None)
    };

    let provider = crate::provider::anthropic::AnthropicProvider::new(credential, base_url)?;
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

// --- browser sign-in via the Anthropic CLI ---------------------------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthOptions {
    /// Present state of the API-key credential.
    pub credentials: CredentialStatus,
    /// Whether the Anthropic CLI is installed, and whether it is signed in.
    pub cli: crate::oauth::CliStatus,
    pub mode: AuthMode,
    pub profile: Option<String>,
}

/// Everything the sign-in UI needs in one round trip.
#[tauri::command]
pub fn auth_options(app: AppHandle) -> Result<AuthOptions> {
    let state = app.state::<Arc<AppState>>();
    let (mode, profile): (AuthMode, Option<String>) = {
        let conn = state.db.conn();
        (
            crate::db::repo::settings::get_or(&conn, sk::AUTH_MODE, AuthMode::ApiKey),
            crate::db::repo::settings::get_or(&conn, sk::OAUTH_PROFILE, None),
        )
    };

    let mut credentials = secrets::status("anthropic");
    credentials.mode = mode;

    Ok(AuthOptions {
        credentials,
        cli: crate::oauth::status(profile.as_deref()),
        mode,
        profile,
    })
}

/// Open the browser sign-in. The CLI drives the flow; the UI polls
/// `auth_options` afterwards to see when it completed.
#[tauri::command]
pub fn oauth_begin_login(app: AppHandle, profile: Option<String>) -> Result<()> {
    let state = app.state::<Arc<AppState>>();
    let profile = profile.filter(|p| !p.trim().is_empty());
    crate::oauth::begin_login(profile.as_deref())?;
    if let Some(p) = &profile {
        let conn = state.db.conn();
        crate::db::repo::settings::set(&conn, sk::OAUTH_PROFILE, p)?;
    }
    Ok(())
}

/// Switch between an API key and browser sign-in.
#[tauri::command]
pub fn set_auth_mode(app: AppHandle, mode: AuthMode) -> Result<AuthOptions> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    if mode == AuthMode::Oauth && !crate::oauth::is_available() {
        return Err(AppError::invalid(
            "The Anthropic CLI (`ant`) is not installed, so browser sign-in is unavailable.",
        ));
    }
    {
        let conn = state.db.conn();
        crate::db::repo::settings::set(&conn, sk::AUTH_MODE, &mode)?;
    }
    // A different credential may expose a different set of models.
    state.models.lock().unwrap().fetched_at = None;
    auth_options(app)
}

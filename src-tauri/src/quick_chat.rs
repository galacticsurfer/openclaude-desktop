//! A desktop-wide hotkey that brings the window up ready to type.
//!
//! Off unless the user sets one. A global shortcut is claimed from the whole
//! session, so silently grabbing a combination the user's window manager or
//! another app already owns would be rude — and on Wayland the compositor
//! may refuse the grab entirely. Both cases are reported rather than
//! swallowed, so the setting can say what happened.

use crate::db::repo::settings as settings_repo;
use crate::error::{AppError, Result};
use crate::settings_defaults as sk;
use crate::state::AppState;
use std::str::FromStr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// The shortcut the user has configured, if any.
pub fn configured(state: &AppState) -> Option<String> {
    let raw: String =
        settings_repo::get_or(&state.db.conn(), sk::QUICK_CHAT_SHORTCUT, String::new());
    let trimmed = raw.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn parse(accelerator: &str) -> Result<Shortcut> {
    Shortcut::from_str(accelerator).map_err(|_| {
        AppError::invalid(format!(
            "\"{accelerator}\" is not a shortcut this desktop understands. \
             Try something like Ctrl+Alt+C."
        ))
    })
}

/// Drop any shortcut currently held.
pub fn unregister_all<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.global_shortcut().unregister_all();
}

/// Claim `accelerator`, replacing whatever was held before.
///
/// Errors if the combination is malformed or already taken by something
/// else on the desktop — the caller surfaces that to the user rather than
/// leaving them with a key that quietly does nothing.
pub fn register<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> Result<()> {
    let shortcut = parse(accelerator)?;
    unregister_all(app);

    let handle = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _sc, event| {
            // Fire on press only: the release would raise the window twice.
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if let Some(w) = handle.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
                let _ = handle.emit_to("main", "quick-chat:open", ());
            }
        })
        .map_err(|e| {
            AppError::invalid(format!(
                "{accelerator} could not be registered — another application \
                 or your desktop may already use it. ({e})"
            ))
        })
}

/// Claim the configured shortcut at startup, if there is one.
pub fn install<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    let Some(accelerator) = configured(state) else {
        return;
    };
    if let Err(e) = register(app, &accelerator) {
        // Never fatal: a shortcut that cannot be claimed must not stop the
        // app from starting.
        tracing::warn!(error = %e, accelerator, "quick-chat shortcut unavailable");
    }
}

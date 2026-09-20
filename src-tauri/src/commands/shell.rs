//! IPC for the embedded terminal.
//!
//! Every command here is reachable only from the window's own UI, driven by
//! a keystroke. Nothing in the provider or chat path can call them, and no
//! command returns shell output to anything but the terminal widget — see
//! the module docs in `shell.rs` for why that separation is structural.

use crate::error::Result;
use crate::shell::SharedShells;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Output from a terminal, addressed to the widget that owns it.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellOutput {
    pub id: String,
    pub data: String,
}

pub const EV_SHELL: &str = "shell:data";

#[tauri::command]
pub fn shell_open(
    app: AppHandle,
    shells: State<'_, SharedShells>,
    state: State<'_, Arc<AppState>>,
    id: String,
    project_id: Option<String>,
) -> Result<()> {
    if shells.is_open(&id) {
        return Ok(());
    }
    // A project's working folder if it has one, else the same empty scratch
    // directory conversations use — never wherever the app happened to be
    // launched from.
    let cwd = project_id
        .and_then(|pid| {
            crate::db::repo::projects::get(&state.db.conn(), &pid)
                .ok()
                .and_then(|p| p.working_dir)
        })
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| {
            let dir = crate::paths::session_dir();
            let _ = std::fs::create_dir_all(&dir);
            dir
        });

    let handle = app.clone();
    let key = id.clone();
    shells.open(&id, &cwd, move |data| {
        let _ = handle.emit(
            EV_SHELL,
            ShellOutput {
                id: key.clone(),
                data,
            },
        );
    })
}

#[tauri::command]
pub fn shell_write(shells: State<'_, SharedShells>, id: String, data: String) -> Result<()> {
    shells.write(&id, &data)
}

#[tauri::command]
pub fn shell_resize(
    shells: State<'_, SharedShells>,
    id: String,
    rows: u16,
    cols: u16,
) -> Result<()> {
    shells.resize(&id, rows, cols)
}

#[tauri::command]
pub fn shell_close(shells: State<'_, SharedShells>, id: String) {
    shells.close(&id);
}

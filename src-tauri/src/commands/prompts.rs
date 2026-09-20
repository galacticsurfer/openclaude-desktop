use crate::db::models::Prompt;
use crate::db::repo::prompts as repo;
use crate::error::{AppError, Result};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

fn check(title: &str, body: &str) -> Result<()> {
    if title.trim().is_empty() {
        return Err(AppError::invalid("A prompt needs a name."));
    }
    if body.trim().is_empty() {
        return Err(AppError::invalid("A prompt needs some text."));
    }
    Ok(())
}

#[tauri::command]
pub fn list_prompts(state: State<'_, Arc<AppState>>) -> Result<Vec<Prompt>> {
    repo::list(&state.db.conn())
}

#[tauri::command]
pub fn create_prompt(
    state: State<'_, Arc<AppState>>,
    title: String,
    body: String,
) -> Result<Prompt> {
    check(&title, &body)?;
    repo::create(&state.db.conn(), title.trim(), body.trim())
}

#[tauri::command]
pub fn update_prompt(
    state: State<'_, Arc<AppState>>,
    id: String,
    title: String,
    body: String,
) -> Result<Prompt> {
    check(&title, &body)?;
    repo::update(&state.db.conn(), &id, title.trim(), body.trim())
}

#[tauri::command]
pub fn delete_prompt(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::delete(&state.db.conn(), &id)
}

#[tauri::command]
pub fn mark_prompt_used(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::mark_used(&state.db.conn(), &id)
}

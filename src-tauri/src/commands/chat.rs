use crate::error::Result;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    conversation_id: String,
    text: String,
    attachment_ids: Vec<String>,
) -> Result<String> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    crate::chat::send(app.clone(), state, conversation_id, text, attachment_ids).await
}

#[tauri::command]
pub async fn retry_message(app: AppHandle, conversation_id: String) -> Result<String> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    crate::chat::retry(app.clone(), state, conversation_id).await
}

#[tauri::command]
pub async fn continue_message(app: AppHandle, conversation_id: String) -> Result<String> {
    let state = app.state::<Arc<AppState>>().inner().clone();
    crate::chat::continue_interrupted(app.clone(), state, conversation_id).await
}

#[tauri::command]
pub fn stop_generation(app: AppHandle, conversation_id: String) -> Result<()> {
    let state = app.state::<Arc<AppState>>();
    crate::chat::stop(state.inner(), &conversation_id)
}

#[tauri::command]
pub fn is_generating(app: AppHandle, conversation_id: String) -> bool {
    app.state::<Arc<AppState>>().is_streaming(&conversation_id)
}

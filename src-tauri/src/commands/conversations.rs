use crate::db::models::*;
use crate::db::repo::conversations as repo;
use crate::db::repo::{messages as msg_repo, settings as settings_repo};
use crate::error::{AppError, Result};
use crate::settings_defaults as sk;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn list_conversations(
    state: State<'_, Arc<AppState>>,
    scope: repo::ListScope,
    project_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ConversationSummary>> {
    let conn = state.db.conn();
    repo::list(
        &conn,
        scope,
        project_id.as_deref(),
        limit.unwrap_or(200).clamp(1, 1000),
        offset.unwrap_or(0).max(0),
    )
}

#[tauri::command]
pub fn get_conversation(state: State<'_, Arc<AppState>>, id: String) -> Result<Conversation> {
    repo::get(&state.db.conn(), &id)
}

#[tauri::command]
pub fn create_conversation(
    state: State<'_, Arc<AppState>>,
    input: repo::NewConversation,
) -> Result<Conversation> {
    let conn = state.db.conn();
    let mut input = input;
    if input.model.trim().is_empty() {
        // Fall back to the configured default so the UI can create a
        // conversation before the model list has loaded.
        input.model = settings_repo::get_or(&conn, sk::DEFAULT_MODEL, String::new());
        if input.model.is_empty() {
            // The dropdown discovers the real list asynchronously; this is
            // only the seed used before the user has picked anything.
            input.model = "sonnet".to_string();
        }
    }
    repo::create(&conn, input)
}

#[tauri::command]
pub fn get_messages(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> Result<Vec<Message>> {
    msg_repo::list(&state.db.conn(), &conversation_id)
}

#[tauri::command]
pub fn rename_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
    title: String,
) -> Result<()> {
    repo::rename(&state.db.conn(), &id, &title)
}

#[tauri::command]
pub fn set_conversation_model(
    state: State<'_, Arc<AppState>>,
    id: String,
    model: String,
) -> Result<()> {
    if model.trim().is_empty() {
        return Err(AppError::invalid("Choose a model."));
    }
    repo::set_model(&state.db.conn(), &id, &model)
}

#[tauri::command]
pub fn set_conversation_pinned(
    state: State<'_, Arc<AppState>>,
    id: String,
    pinned: bool,
) -> Result<()> {
    repo::set_pinned(&state.db.conn(), &id, pinned)
}

#[tauri::command]
pub fn set_conversation_archived(
    state: State<'_, Arc<AppState>>,
    id: String,
    archived: bool,
) -> Result<()> {
    repo::set_archived(&state.db.conn(), &id, archived)
}

#[tauri::command]
pub fn set_conversation_project(
    state: State<'_, Arc<AppState>>,
    id: String,
    project_id: Option<String>,
) -> Result<()> {
    repo::set_project(&state.db.conn(), &id, project_id.as_deref())
}

#[tauri::command]
pub fn set_conversation_system_prompt(
    state: State<'_, Arc<AppState>>,
    id: String,
    prompt: Option<String>,
) -> Result<()> {
    let p = prompt.filter(|s| !s.trim().is_empty());
    repo::set_system_prompt(&state.db.conn(), &id, p.as_deref())
}

/// Soft delete — recoverable from Trash.
#[tauri::command]
pub fn trash_conversation(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::trash(&state.db.conn(), &id)
}

#[tauri::command]
pub fn restore_conversation(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::restore(&state.db.conn(), &id)
}

/// Irreversible; the UI must confirm before calling this.
#[tauri::command]
pub fn delete_conversation_permanently(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::purge(&state.db.conn(), &id)
}

#[tauri::command]
pub fn empty_trash(state: State<'_, Arc<AppState>>) -> Result<usize> {
    repo::purge_trash_older_than(&state.db.conn(), now_ms())
}

#[tauri::command]
pub fn duplicate_conversation(state: State<'_, Arc<AppState>>, id: String) -> Result<Conversation> {
    let last_seq = {
        let conn = state.db.conn();
        msg_repo::last(&conn, &id)?.map(|m| m.seq)
    };
    match last_seq {
        // Branching from the final message is exactly "duplicate".
        Some(seq) => {
            let message_id = {
                let conn = state.db.conn();
                msg_repo::list_through_seq(&conn, &id, seq)?
                    .last()
                    .map(|m| m.id.clone())
                    .ok_or(AppError::NotFound("Message"))?
            };
            crate::chat::branch_from(&state.db, &message_id)
        }
        None => {
            let conn = state.db.conn();
            let source = repo::get(&conn, &id)?;
            repo::create(
                &conn,
                repo::NewConversation {
                    title: Some(format!("{} (copy)", source.title)),
                    model: source.model,
                    project_id: source.project_id,
                    system_prompt: source.system_prompt,
                },
            )
        }
    }
}

/// Fork the conversation at `message_id` into a new one.
#[tauri::command]
pub fn branch_conversation(
    state: State<'_, Arc<AppState>>,
    message_id: String,
) -> Result<Conversation> {
    crate::chat::branch_from(&state.db, &message_id)
}

#[tauri::command]
pub fn conversation_usage(state: State<'_, Arc<AppState>>, id: String) -> Result<UsageTotals> {
    repo::usage(&state.db.conn(), &id)
}

#[tauri::command]
pub fn export_conversation(
    state: State<'_, Arc<AppState>>,
    id: String,
    format: crate::export::ExportFormat,
    include_metadata: Option<bool>,
) -> Result<ExportResult> {
    let content = crate::export::export(&state.db, &id, format, include_metadata.unwrap_or(true))?;
    let title = repo::get(&state.db.conn(), &id)?.title;
    Ok(ExportResult {
        filename: crate::export::suggested_filename(&title, format),
        content,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub filename: String,
    pub content: String,
}

/// Write an export to a path the user chose in a native save dialog.
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<()> {
    let p = std::path::Path::new(&path);
    if !p.is_absolute() {
        return Err(AppError::invalid("Choose a location to save to."));
    }
    std::fs::write(p, contents)?;
    Ok(())
}

#[tauri::command]
pub fn delete_message(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    msg_repo::delete(&state.db.conn(), &id)
}

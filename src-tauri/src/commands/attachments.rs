use crate::attachments as svc;
use crate::db::models::Attachment;
use crate::db::repo::attachments as repo;
use crate::error::{AppError, Result};
use crate::state::AppState;
use base64::Engine;
use std::sync::Arc;
use tauri::State;

/// Ingest files the user picked or dropped.
///
/// Partial success is deliberate: dropping ten files where one is a binary
/// should attach nine and explain the tenth, not fail the whole gesture.
#[tauri::command]
pub fn add_attachments(
    state: State<'_, Arc<AppState>>,
    paths: Vec<String>,
    conversation_id: Option<String>,
    project_id: Option<String>,
) -> Result<AddResult> {
    let root = crate::paths::attachments_dir();
    std::fs::create_dir_all(&root)?;

    let mut added = Vec::new();
    let mut rejected = Vec::new();

    for p in paths {
        let path = std::path::Path::new(&p);
        match svc::ingest(path, &root) {
            Ok(v) => {
                let conn = state.db.conn();
                match repo::insert(
                    &conn,
                    repo::NewAttachment {
                        conversation_id: conversation_id.as_deref(),
                        project_id: project_id.as_deref(),
                        filename: &v.filename,
                        mime_type: &v.mime_type,
                        size_bytes: v.size_bytes,
                        kind: v.kind,
                        storage_path: &v.storage_path,
                        sha256: &v.sha256,
                        text_content: v.text_content.as_deref(),
                    },
                ) {
                    Ok(a) => added.push(a),
                    Err(e) => rejected.push(Rejection {
                        filename: v.filename,
                        reason: e.to_string(),
                    }),
                }
            }
            Err(e) => rejected.push(Rejection {
                filename: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&p)
                    .to_string(),
                reason: e.to_string(),
            }),
        }
    }

    Ok(AddResult { added, rejected })
}

/// Attach an image from the clipboard or a paste event.
#[tauri::command]
pub fn add_image_attachment(
    state: State<'_, Arc<AppState>>,
    filename: String,
    mime_type: String,
    data_base64: String,
    conversation_id: Option<String>,
) -> Result<Attachment> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|_| AppError::invalid("That image could not be decoded."))?;

    let root = crate::paths::attachments_dir();
    std::fs::create_dir_all(&root)?;
    let v = svc::ingest_bytes(&filename, &mime_type, &bytes, &root)?;

    let conn = state.db.conn();
    repo::insert(
        &conn,
        repo::NewAttachment {
            conversation_id: conversation_id.as_deref(),
            project_id: None,
            filename: &v.filename,
            mime_type: &v.mime_type,
            size_bytes: v.size_bytes,
            kind: v.kind,
            storage_path: &v.storage_path,
            sha256: &v.sha256,
            text_content: None,
        },
    )
}

#[tauri::command]
pub fn remove_attachment(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    let conn = state.db.conn();
    let a = repo::get(&conn, &id)?;
    repo::delete(&conn, &id)?;
    // Content-addressed: only unlink the blob when nothing else points at it.
    if repo::refcount(&conn, &a.sha256)? == 0 {
        let _ = std::fs::remove_file(crate::paths::attachments_dir().join(&a.storage_path));
    }
    Ok(())
}

#[tauri::command]
pub fn list_attachments(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
) -> Result<Vec<Attachment>> {
    repo::for_conversation(&state.db.conn(), &conversation_id)
}

/// Serve an attachment to the webview as a data URL.
///
/// Going through a command rather than opening the asset protocol to the
/// whole store means the renderer can only ever see blobs it names by id,
/// and the CSP stays closed.
#[tauri::command]
pub fn read_attachment_data_url(state: State<'_, Arc<AppState>>, id: String) -> Result<String> {
    let a = repo::get(&state.db.conn(), &id)?;
    let bytes = std::fs::read(crate::paths::attachments_dir().join(&a.storage_path))
        .map_err(|_| AppError::invalid("That attachment is no longer on disk."))?;
    Ok(format!(
        "data:{};base64,{}",
        a.mime_type,
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddResult {
    pub added: Vec<Attachment>,
    pub rejected: Vec<Rejection>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    pub filename: String,
    pub reason: String,
}

use crate::db::models::SearchHit;
use crate::db::repo::search as repo;
use crate::error::Result;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn search_all(
    state: State<'_, Arc<AppState>>,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<SearchHit>> {
    repo::search(&state.db.conn(), &query, limit.unwrap_or(50).clamp(1, 500))
}

#[tauri::command]
pub fn search_conversation(
    state: State<'_, Arc<AppState>>,
    conversation_id: String,
    query: String,
    limit: Option<i64>,
) -> Result<Vec<SearchHit>> {
    repo::search_in_conversation(
        &state.db.conn(),
        &conversation_id,
        &query,
        limit.unwrap_or(100).clamp(1, 500),
    )
}

#[tauri::command]
pub fn rebuild_search_index(state: State<'_, Arc<AppState>>) -> Result<()> {
    repo::rebuild_indexes(&state.db.conn())
}

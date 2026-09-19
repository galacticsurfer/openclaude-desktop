use crate::db::models::{Project, ProjectSummary};
use crate::db::repo::projects as repo;
use crate::error::{AppError, Result};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn list_projects(
    state: State<'_, Arc<AppState>>,
    include_archived: Option<bool>,
) -> Result<Vec<ProjectSummary>> {
    repo::list(&state.db.conn(), include_archived.unwrap_or(false))
}

#[tauri::command]
pub fn get_project(state: State<'_, Arc<AppState>>, id: String) -> Result<Project> {
    repo::get(&state.db.conn(), &id)
}

#[tauri::command]
pub fn create_project(
    state: State<'_, Arc<AppState>>,
    input: repo::ProjectInput,
) -> Result<Project> {
    validate(&input)?;
    repo::create(&state.db.conn(), input)
}

#[tauri::command]
pub fn update_project(
    state: State<'_, Arc<AppState>>,
    id: String,
    input: repo::ProjectInput,
) -> Result<Project> {
    validate(&input)?;
    repo::update(&state.db.conn(), &id, input)
}

#[tauri::command]
pub fn set_project_archived(
    state: State<'_, Arc<AppState>>,
    id: String,
    archived: bool,
) -> Result<()> {
    repo::set_archived(&state.db.conn(), &id, archived)
}

/// Conversations in the project survive and become ungrouped.
#[tauri::command]
pub fn delete_project(state: State<'_, Arc<AppState>>, id: String) -> Result<()> {
    repo::delete(&state.db.conn(), &id)
}

fn validate(input: &repo::ProjectInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::invalid("A project needs a name."));
    }
    if input.name.chars().count() > 120 {
        return Err(AppError::invalid("That project name is too long."));
    }
    // A working folder is a reference the user picks in a native dialog; we
    // only record it, never traverse it, but it must at least exist.
    if let Some(dir) = input
        .working_dir
        .as_deref()
        .filter(|d| !d.trim().is_empty())
    {
        let p = std::path::Path::new(dir);
        if !p.is_absolute() {
            return Err(AppError::invalid(
                "The working folder must be an absolute path.",
            ));
        }
        if !p.is_dir() {
            return Err(AppError::invalid(format!("{dir} is not a folder.")));
        }
    }
    Ok(())
}

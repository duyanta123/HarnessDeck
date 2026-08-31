//! The IPC surface behind the project switcher and project settings.

use std::path::PathBuf;

use super::Roster;
use crate::error::{Error, Result};
use crate::harness::commands::AppState;
use crate::harness::supervisor::Status;
use tauri::State;

#[tauri::command]
pub fn projects_list() -> Result<Roster> {
    Ok(super::roster())
}

#[tauri::command]
pub fn projects_add(
    path: PathBuf,
    name: Option<String>,
    profile: Option<String>,
) -> Result<Roster> {
    super::add(name, path, profile)
}

#[tauri::command]
pub fn projects_select(id: String) -> Result<Roster> {
    super::select(&id)
}

#[tauri::command]
pub fn projects_remove(id: String, state: State<'_, AppState>) -> Result<Roster> {
    let running = !matches!(
        state.supervisor.status(),
        Status::Stopped | Status::Failed { .. }
    );
    if running && super::is_active(&id) {
        return Err(Error::Project(
            "the active project cannot be removed while Harness is running; stop it first".into(),
        ));
    }
    super::remove(&id)
}

#[tauri::command]
pub fn projects_rename(id: String, name: String) -> Result<Roster> {
    super::rename(&id, name)
}

#[tauri::command]
pub fn projects_bind_profile(id: String, profile: String) -> Result<Roster> {
    super::bind_profile(&id, profile)
}

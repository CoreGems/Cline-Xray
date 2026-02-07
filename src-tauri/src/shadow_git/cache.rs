//! Disk-based JSON cache for shadow_git discovery results.
//!
//! Cache files are stored under `%APPDATA%/jira-dashboard/shadow_git_cache/`.
//! Each file is a simple JSON blob that gets loaded on startup and
//! written whenever discovery or refresh happens.

use std::path::PathBuf;

use super::types::{TasksResponse, WorkspacesResponse};

const CACHE_DIR: &str = "jira-dashboard/shadow_git_cache";
const WORKSPACES_FILE: &str = "workspaces.json";

/// Return the cache directory, creating it if needed.
fn cache_dir() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let dir = PathBuf::from(appdata).join(CACHE_DIR);
    if !dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!("Failed to create shadow_git cache dir {:?}: {}", dir, e);
            return None;
        }
    }
    Some(dir)
}

/// Tasks cache file name for a workspace: tasks_<workspace_id>.json
fn tasks_file(workspace_id: &str) -> String {
    format!("tasks_{}.json", workspace_id)
}

// ============ Workspaces ============

/// Load cached workspaces from disk
pub fn load_workspaces() -> Option<WorkspacesResponse> {
    let path = cache_dir()?.join(WORKSPACES_FILE);
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            match serde_json::from_str::<WorkspacesResponse>(&json) {
                Ok(data) => {
                    log::info!("Loaded {} workspaces from disk cache", data.workspaces.len());
                    Some(data)
                }
                Err(e) => {
                    log::warn!("Failed to parse workspaces cache: {}", e);
                    None
                }
            }
        }
        Err(_) => None,
    }
}

/// Save workspaces to disk cache
pub fn save_workspaces(data: &WorkspacesResponse) {
    if let Some(dir) = cache_dir() {
        let path = dir.join(WORKSPACES_FILE);
        match serde_json::to_string_pretty(data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::warn!("Failed to write workspaces cache: {}", e);
                } else {
                    log::info!("Saved {} workspaces to disk cache", data.workspaces.len());
                }
            }
            Err(e) => log::warn!("Failed to serialize workspaces cache: {}", e),
        }
    }
}

// ============ Tasks ============

/// Load cached tasks for a workspace from disk
pub fn load_tasks(workspace_id: &str) -> Option<TasksResponse> {
    let path = cache_dir()?.join(tasks_file(workspace_id));
    match std::fs::read_to_string(&path) {
        Ok(json) => {
            match serde_json::from_str::<TasksResponse>(&json) {
                Ok(data) => {
                    log::info!(
                        "Loaded {} tasks for workspace {} from disk cache",
                        data.tasks.len(),
                        workspace_id
                    );
                    Some(data)
                }
                Err(e) => {
                    log::warn!("Failed to parse tasks cache for {}: {}", workspace_id, e);
                    None
                }
            }
        }
        Err(_) => None,
    }
}

/// Save tasks for a workspace to disk cache
pub fn save_tasks(workspace_id: &str, data: &TasksResponse) {
    if let Some(dir) = cache_dir() {
        let path = dir.join(tasks_file(workspace_id));
        match serde_json::to_string_pretty(data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::warn!("Failed to write tasks cache for {}: {}", workspace_id, e);
                } else {
                    log::info!(
                        "Saved {} tasks for workspace {} to disk cache",
                        data.tasks.len(),
                        workspace_id
                    );
                }
            }
            Err(e) => log::warn!("Failed to serialize tasks cache for {}: {}", workspace_id, e),
        }
    }
}

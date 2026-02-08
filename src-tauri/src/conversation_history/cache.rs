//! Disk-based JSON cache for conversation_history task index.
//!
//! Cache file stored at `%APPDATA%/jira-dashboard/conversation_history_cache/tasks_index.json`.
//! Contains the TaskHistoryListResponse so we don't re-parse 180+ task dirs on every request.

use std::path::PathBuf;

use super::types::TaskHistoryListResponse;

const CACHE_DIR: &str = "jira-dashboard/conversation_history_cache";
const TASKS_INDEX_FILE: &str = "tasks_index.json";

/// Return the cache directory, creating it if needed.
fn cache_dir() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let dir = PathBuf::from(appdata).join(CACHE_DIR);
    if !dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!(
                "Failed to create conversation_history cache dir {:?}: {}",
                dir,
                e
            );
            return None;
        }
    }
    Some(dir)
}

/// Load cached task index from disk
pub fn load_tasks_index() -> Option<TaskHistoryListResponse> {
    let path = cache_dir()?.join(TASKS_INDEX_FILE);
    match std::fs::read_to_string(&path) {
        Ok(json) => match serde_json::from_str::<TaskHistoryListResponse>(&json) {
            Ok(data) => {
                log::info!(
                    "Loaded {} tasks from conversation_history disk cache",
                    data.total_tasks
                );
                Some(data)
            }
            Err(e) => {
                log::warn!("Failed to parse conversation_history cache: {}", e);
                None
            }
        },
        Err(_) => None,
    }
}

/// Save task index to disk cache
pub fn save_tasks_index(data: &TaskHistoryListResponse) {
    if let Some(dir) = cache_dir() {
        let path = dir.join(TASKS_INDEX_FILE);
        match serde_json::to_string_pretty(data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::warn!("Failed to write conversation_history cache: {}", e);
                } else {
                    log::info!(
                        "Saved {} tasks to conversation_history disk cache",
                        data.total_tasks
                    );
                }
            }
            Err(e) => log::warn!("Failed to serialize conversation_history cache: {}", e),
        }
    }
}

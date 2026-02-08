//! Disk-based JSON cache for conversation_history task index.
//!
//! Cache file stored at `%APPDATA%/jira-dashboard/conversation_history_cache/tasks_index.json`.
//! Contains the TaskHistoryListResponse so we don't re-parse 180+ task dirs on every request.
//!
//! ## Best-effort semantics
//!
//! All disk cache operations are **best-effort**:
//! - Failures are logged with structured context (path, error, size) but never panic.
//! - Callers (in `handlers/index.rs`) do not check return values — disk cache
//!   is an optimization, not a correctness requirement.
//! - If the cache directory doesn't exist and can't be created, save silently logs and returns.
//! - If the cache file is corrupted, load returns `None` and the next request triggers a full scan.

use std::path::PathBuf;

use super::types::TaskHistoryListResponse;

const CACHE_DIR: &str = "jira-dashboard/conversation_history_cache";
const TASKS_INDEX_FILE: &str = "tasks_index.json";

/// Return the cache directory, creating it if needed.
///
/// Returns `None` (with a warning log) if:
/// - `%APPDATA%` environment variable is not set
/// - The directory doesn't exist and can't be created
fn cache_dir() -> Option<PathBuf> {
    let appdata = match std::env::var("APPDATA") {
        Ok(val) => val,
        Err(_) => {
            log::warn!(
                "Disk cache: %APPDATA% not set — cannot resolve cache directory for '{}'",
                CACHE_DIR
            );
            return None;
        }
    };
    let dir = PathBuf::from(appdata).join(CACHE_DIR);
    if !dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::warn!(
                "Disk cache: failed to create directory {:?}: {} (cache will be unavailable)",
                dir,
                e
            );
            return None;
        }
        log::debug!("Disk cache: created directory {:?}", dir);
    }
    Some(dir)
}

/// Load cached task index from disk.
///
/// Returns `None` if the file doesn't exist, is unreadable, or fails to deserialize.
/// All failure cases are logged.
pub fn load_tasks_index() -> Option<TaskHistoryListResponse> {
    let dir = cache_dir()?;
    let path = dir.join(TASKS_INDEX_FILE);

    if !path.exists() {
        log::debug!("Disk cache: no cache file at {:?} (cold start)", path);
        return None;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(e) => {
            log::warn!(
                "Disk cache: failed to read {:?}: {} (will trigger fresh scan)",
                path,
                e
            );
            return None;
        }
    };

    let byte_len = content.len();

    match serde_json::from_str::<TaskHistoryListResponse>(&content) {
        Ok(data) => {
            log::info!(
                "Disk cache: loaded {} tasks from {:?} ({:.1} KB)",
                data.total_tasks,
                path,
                byte_len as f64 / 1024.0
            );
            Some(data)
        }
        Err(e) => {
            log::warn!(
                "Disk cache: failed to parse {:?} ({:.1} KB): {} (will trigger fresh scan)",
                path,
                byte_len as f64 / 1024.0,
                e
            );
            None
        }
    }
}

/// Save task index to disk cache (best-effort).
///
/// Logs on both success and failure. Never panics or returns errors to callers.
/// This is intentionally fire-and-forget — the in-memory cache is the source of truth.
pub fn save_tasks_index(data: &TaskHistoryListResponse) {
    let dir = match cache_dir() {
        Some(d) => d,
        None => {
            log::warn!(
                "Disk cache: cannot save {} tasks — cache directory unavailable",
                data.total_tasks
            );
            return;
        }
    };

    let path = dir.join(TASKS_INDEX_FILE);

    let json = match serde_json::to_string_pretty(data) {
        Ok(j) => j,
        Err(e) => {
            log::warn!(
                "Disk cache: failed to serialize {} tasks for {:?}: {}",
                data.total_tasks,
                path,
                e
            );
            return;
        }
    };

    let byte_len = json.len();

    match std::fs::write(&path, &json) {
        Ok(()) => {
            log::info!(
                "Disk cache: saved {} tasks to {:?} ({:.1} KB)",
                data.total_tasks,
                path,
                byte_len as f64 / 1024.0
            );
        }
        Err(e) => {
            log::warn!(
                "Disk cache: failed to write {:?} ({:.1} KB): {} (in-memory cache is still valid)",
                path,
                byte_len as f64 / 1024.0,
                e
            );
        }
    }
}

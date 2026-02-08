//! Files-in-context parsing.
//!
//! Contains:
//! - Files-in-context parsing from task_metadata.json
//! - Metadata-based file audit views
//!
//! Must not include API history parsing.

use super::detail::parse_task_metadata_detail;
use super::root::tasks_root;
use super::types::*;

/// Parse a task's files-in-context audit trail from task_metadata.json.
///
/// This is a focused parser for the `/files` endpoint. It reads:
/// - `task_metadata.json` — files_in_context array with read/edit timestamps
///
/// Supports filtering via:
/// - `source` — filter by record_source (e.g. "cline_edited", "read_tool")
/// - `state` — filter by record_state ("active" or "stale")
///
/// Returns None if the task directory doesn't exist or has no task_metadata.json.
pub fn parse_task_files(
    task_id: &str,
    source_filter: Option<&str>,
    state_filter: Option<&str>,
) -> Option<TaskFilesResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let metadata_path = dir.join("task_metadata.json");

    if !metadata_path.exists() {
        log::warn!("No task_metadata.json for task {}", task_id);
        return None;
    }

    // Parse task_metadata.json using the shared detail parser
    let (all_files, _, _, _, _, _) = parse_task_metadata_detail(&metadata_path);

    // Compute stats before filtering
    let total_files = all_files.len();
    let files_edited_count = all_files
        .iter()
        .filter(|f| f.record_source.as_deref() == Some("cline_edited"))
        .count();
    let files_read_count = all_files
        .iter()
        .filter(|f| f.record_source.as_deref() == Some("read_tool"))
        .count();
    let files_mentioned_count = all_files
        .iter()
        .filter(|f| f.record_source.as_deref() == Some("file_mentioned"))
        .count();
    let files_user_edited_count = all_files
        .iter()
        .filter(|f| f.record_source.as_deref() == Some("user_edited"))
        .count();

    // Apply filters
    let filtered: Vec<FileInContextDetail> = all_files
        .into_iter()
        .filter(|file| {
            // Source filter
            if let Some(source) = source_filter {
                if file.record_source.as_deref() != Some(source) {
                    return false;
                }
            }
            // State filter
            if let Some(state) = state_filter {
                if file.record_state.as_deref() != Some(state) {
                    return false;
                }
            }
            true
        })
        .collect();

    Some(TaskFilesResponse {
        task_id: task_id.to_string(),
        total_files,
        files_edited_count,
        files_read_count,
        files_mentioned_count,
        files_user_edited_count,
        files: filtered,
    })
}

//! Handler for the GET /latest composite endpoint.
//!
//! Orchestrates conversation_history (prompts, subtasks) and shadow_git
//! (diffs, changed files) into a single response.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use crate::state::AppState;
use super::types::{LatestErrorResponse, LatestQuery, LatestResponse, SubtaskSummaryItem};

/// Get the latest task/subtask prompt merged with its diff and changed files
///
/// Automatically resolves the most recent Cline task, finds its last subtask
/// (or initial prompt if single-prompt task), locates the corresponding
/// checkpoint workspace, and returns the diff + file list + prompt in a
/// single response.
///
/// **Designed for both UI rendering and LLM/agent tool-use consumption.**
///
/// - `scope=subtask` (default): Returns only the latest subtask's diff and prompt
/// - `scope=task`: Returns the full task diff (all subtasks merged) with the latest prompt
/// - `exclude`: Pathspec exclusion patterns (e.g. `?exclude=node_modules&exclude=target`)
#[utoipa::path(
    get,
    path = "/latest",
    params(LatestQuery),
    responses(
        (status = 200, description = "Latest task/subtask prompt + diff + changed files", body = LatestResponse),
        (status = 404, description = "No tasks found or no checkpoint data", body = LatestErrorResponse),
        (status = 500, description = "Internal server error", body = LatestErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "history", "tool"]
)]
pub async fn get_latest_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<LatestQuery>,
) -> Result<Json<LatestResponse>, (StatusCode, Json<LatestErrorResponse>)> {
    let scope = params.scope.clone();
    let explicit_excludes = params.exclude.clone();

    // Merge with .changesignore patterns (auto-load if no explicit excludes)
    let excludes = crate::changesignore::merge_excludes(&explicit_excludes);

    log::info!(
        "REST API: GET /latest — scope={}, excludes={:?} (explicit={:?})",
        scope, excludes, explicit_excludes
    );

    // Run the entire orchestration in a blocking context (filesystem + git CLI)
    let result = tokio::task::spawn_blocking(move || {
        resolve_latest(&scope, &excludes)
    })
    .await;

    match result {
        Ok(Ok(response)) => {
            log::info!(
                "REST API: GET /latest — task={}, subtask={:?}, {} files changed",
                response.task_id,
                response.subtask_index,
                response.diff.as_ref().map(|d| d.files.len()).unwrap_or(0)
            );
            Ok(Json(response))
        }
        Ok(Err(LatestError::NotFound(msg))) => {
            log::warn!("REST API: GET /latest — 404: {}", msg);
            Err((
                StatusCode::NOT_FOUND,
                Json(LatestErrorResponse {
                    error: msg,
                    code: 404,
                }),
            ))
        }
        Ok(Err(LatestError::Internal(msg))) => {
            log::error!("REST API: GET /latest — 500: {}", msg);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LatestErrorResponse {
                    error: msg,
                    code: 500,
                }),
            ))
        }
        Err(e) => {
            log::error!("REST API: GET /latest — spawn_blocking failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LatestErrorResponse {
                    error: format!("Internal error: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

// ============ Internal orchestration ============

enum LatestError {
    NotFound(String),
    Internal(String),
}

/// Synchronous orchestration: resolve the latest task/subtask + diff.
fn resolve_latest(scope: &str, excludes: &[String]) -> Result<LatestResponse, LatestError> {
    // 1. Get the most recent task from conversation history
    let task_list = crate::conversation_history::summary::scan_all_tasks();
    let latest_task = task_list
        .tasks
        .first()
        .ok_or_else(|| LatestError::NotFound("No Cline tasks found".to_string()))?;

    let task_id = &latest_task.task_id;
    let task_started_at = latest_task.started_at.clone();
    let task_ended_at = latest_task.ended_at.clone();

    log::info!("Latest task: {} (started {})", task_id, task_started_at);

    // 2. Get subtasks for this task
    let subtasks_opt = crate::conversation_history::subtasks::parse_task_subtasks(task_id);

    // 3. Determine which subtask/prompt to use
    let (prompt, prompt_timestamp, subtask_index, is_initial_task, total_subtasks,
         message_range_start, message_range_end, message_count, tool_call_count, tools_used) =
        if let Some(ref subtasks) = subtasks_opt {
            if !subtasks.subtasks.is_empty() {
                // Use the last subtask (most recent prompt)
                let last = subtasks.subtasks.last().unwrap();
                (
                    last.prompt.clone(),
                    last.timestamp.clone(),
                    Some(last.subtask_index),
                    Some(last.is_initial_task),
                    subtasks.total_subtasks,
                    Some(last.message_range_start),
                    last.message_range_end,
                    last.message_count,
                    last.tool_call_count,
                    last.tools_used.clone(),
                )
            } else {
                // No subtask entries — use task_prompt from summary
                (
                    latest_task.task_prompt.clone().unwrap_or_default(),
                    task_started_at.clone(),
                    None,
                    None,
                    0,
                    None,
                    None,
                    latest_task.message_count,
                    latest_task.tool_use_count,
                    latest_task.tool_breakdown.keys().cloned().collect(),
                )
            }
        } else {
            // No ui_messages.json — fall back to summary
            (
                latest_task.task_prompt.clone().unwrap_or_default(),
                task_started_at.clone(),
                None,
                None,
                0,
                None,
                None,
                latest_task.message_count,
                latest_task.tool_use_count,
                latest_task.tool_breakdown.keys().cloned().collect(),
            )
        };

    // 4. Resolve workspace for this task (shadow git)
    let workspace_result = crate::shadow_git::discovery::find_workspace_for_task(task_id);

    let (diff, no_diff_reason, workspace_id) = match workspace_result {
        Some((ws_id, git_dir)) => {
            // 5. Get the diff based on scope
            let diff_result = if scope == "task" {
                // Full task diff
                crate::shadow_git::discovery::get_task_diff(task_id, &git_dir, excludes)
            } else if let Some(si) = subtask_index {
                // Subtask diff
                crate::shadow_git::discovery::get_subtask_diff(
                    task_id, si, &ws_id, &git_dir, excludes,
                )
            } else {
                // No subtask info — full task diff as fallback
                crate::shadow_git::discovery::get_task_diff(task_id, &git_dir, excludes)
            };

            match diff_result {
                Ok(diff) => (Some(diff), None, Some(ws_id)),
                Err(e) => {
                    log::warn!("Diff computation failed: {}. Returning prompt without diff.", e);
                    (None, Some(e), Some(ws_id))
                }
            }
        }
        None => {
            log::warn!(
                "No checkpoint workspace found for task {}. Returning prompt without diff.",
                task_id
            );
            (
                None,
                Some("no_checkpoint_workspace".to_string()),
                None,
            )
        }
    };

    // 6. Build subtask summaries for UI tab rendering (diffs NOT included)
    let subtasks_summary: Vec<SubtaskSummaryItem> = if let Some(ref subtasks) = subtasks_opt {
        subtasks.subtasks.iter().map(|s| SubtaskSummaryItem {
            subtask_index: s.subtask_index,
            is_initial_task: s.is_initial_task,
            prompt: s.prompt.clone(),
            timestamp: s.timestamp.clone(),
            message_count: s.message_count,
            tool_call_count: s.tool_call_count,
            tools_used: s.tools_used.clone(),
        }).collect()
    } else {
        vec![]
    };

    Ok(LatestResponse {
        task_id: task_id.clone(),
        subtask_index,
        is_initial_task,
        total_subtasks,
        prompt,
        prompt_timestamp,
        diff,
        no_diff_reason,
        message_range_start,
        message_range_end,
        message_count,
        tool_call_count,
        tools_used,
        workspace_id,
        task_started_at,
        task_ended_at,
        scope: scope.to_string(),
        subtasks: subtasks_summary,
    })
}

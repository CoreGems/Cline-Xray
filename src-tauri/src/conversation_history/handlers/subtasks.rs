//! Subtask detection handler.
//!
//! Responsibility:
//! - Detect task/feedback subtask boundaries from ui_messages.json
//!
//! Owns: GET /history/tasks/{task_id}/subtasks

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use super::common::validate_task_id;
use crate::conversation_history::subtasks::parse_task_subtasks;
use crate::conversation_history::types::{HistoryErrorResponse, SubtasksResponse};
use crate::state::AppState;

/// Get detected subtasks for a single Cline task
///
/// Parses `ui_messages.json` to detect task/feedback boundaries within a conversation.
/// Tasks are often multi-phase: the user provides additional instructions via `<feedback>`
/// tags after seeing the initial result. Each feedback-driven prompt is a subtask.
///
/// Returns:
/// - The initial task prompt (subtask #0)
/// - Any feedback-driven subtasks (#1, #2, …)
/// - Message ranges in `api_conversation_history.json` for each subtask
/// - Tool call counts and tool names used within each subtask's range
///
/// This is an on-demand parse — files are read from disk each request.
/// Lightweight: only parses `ui_messages.json` (small) + scans `api_conversation_history.json`
/// for tool counts within ranges.
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}/subtasks",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)")
    ),
    responses(
        (status = 200, description = "Subtask detection timeline with message ranges and tool counts", body = SubtasksResponse),
        (status = 404, description = "Task not found", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_task_subtasks_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<SubtasksResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    log::info!("REST API: GET /history/tasks/{}/subtasks — detecting subtasks", task_id);

    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let subtasks = parse_task_subtasks(&tid);
        let elapsed = start.elapsed();
        log::info!(
            "Subtask detection for {} complete in {:.1}ms",
            tid,
            elapsed.as_secs_f64() * 1000.0
        );
        subtasks
    })
    .await;

    match result {
        Ok(Some(response)) => {
            log::info!(
                "REST API: Task {} subtasks: {} total, has_subtasks={}",
                task_id,
                response.total_subtasks,
                response.has_subtasks,
            );
            Ok(Json(response))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} not found for subtasks", task_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(HistoryErrorResponse {
                    error: format!("Task '{}' not found or has no ui_messages.json", task_id),
                    code: 404,
                }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to detect subtasks for task {}: {}", task_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to detect subtasks: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

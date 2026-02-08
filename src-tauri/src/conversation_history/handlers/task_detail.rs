//! Task detail handler.
//!
//! Responsibility:
//! - Full deep-dive into a single task
//!
//! Owns: GET /history/tasks/{task_id}

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use super::common::validate_task_id;
use crate::conversation_history::detail::parse_task_detail;
use crate::conversation_history::types::{HistoryErrorResponse, TaskDetailResponse};
use crate::state::AppState;

/// Get full detail for a single Cline task
///
/// Returns a comprehensive deep-dive into a single task, including:
/// - All conversation messages (text/thinking truncated, tool inputs/results summarized)
/// - Tool call timeline with input/result summaries
/// - Files tracked in context (read, edited, mentioned)
/// - Model usage history (may switch models mid-task)
/// - Environment snapshots (OS, VS Code version, Cline version)
/// - Focus chain / task progress checklist (markdown)
///
/// Timestamps for each message are joined from `ui_messages.json` via `conversationHistoryIndex`.
/// Content blocks are truncated for manageability (text/thinking: 500 chars, tool input: 300, tool result: 200).
///
/// This is an on-demand parse — the full task files are read from disk each time.
/// Typical parse time: 10–200ms depending on task size (up to ~4 MB).
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)")
    ),
    responses(
        (status = 200, description = "Full task detail with messages, tools, files, model info, environment, and focus chain", body = TaskDetailResponse),
        (status = 404, description = "Task not found", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_task_detail_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskDetailResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    log::info!("REST API: GET /history/tasks/{} — parsing task detail", task_id);

    // Run parse in blocking context (filesystem I/O — may read up to ~4 MB of JSON)
    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let detail = parse_task_detail(&tid);
        let elapsed = start.elapsed();
        log::info!(
            "Task detail parse for {} complete in {:.1}ms",
            tid,
            elapsed.as_secs_f64() * 1000.0
        );
        detail
    })
    .await;

    match result {
        Ok(Some(detail)) => {
            log::info!(
                "REST API: Task {} detail: {} messages, {} tool calls, {} files, {:.1} KB",
                task_id,
                detail.message_count,
                detail.tool_use_count,
                detail.files_in_context_count,
                detail.api_history_size_bytes as f64 / 1024.0
            );
            Ok(Json(detail))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} not found", task_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(HistoryErrorResponse {
                    error: format!("Task '{}' not found or has no conversation history", task_id),
                    code: 404,
                }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to parse task {}: {}", task_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to parse task detail: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

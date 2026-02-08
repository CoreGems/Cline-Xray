//! Thinking / reasoning handler.
//!
//! Responsibility:
//! - Extraction and analysis of thinking blocks
//! - Truncation and filtering controls
//!
//! Owns: GET /history/tasks/{task_id}/thinking

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use super::common::validate_task_id;
use crate::conversation_history::thinking::parse_task_thinking;
use crate::conversation_history::types::{HistoryErrorResponse, TaskThinkingQuery, ThinkingBlocksResponse};
use crate::state::AppState;

/// Get thinking blocks timeline for a single Cline task
///
/// Returns a timeline of all thinking blocks (extended thinking / chain-of-thought) from
/// assistant messages in the task's conversation history.
///
/// This extracts the raw agent reasoning that is hidden from the user in the Cline UI.
/// Each thinking block includes:
/// - Block index (sequential)
/// - Message index where it appears
/// - Timestamp (from ui_messages join)
/// - Thinking content (with optional truncation)
/// - Full length + truncation status
///
/// Supports optional filtering and truncation via query parameters:
/// - `?max_length=500` — truncate each block to 500 chars (default: 1000, set to 0 for no truncation)
/// - `?min_length=100` — only include blocks with at least 100 chars
///
/// Aggregate stats include total thinking blocks, total characters, and average block length.
///
/// This is an on-demand parse — files are read from disk each request.
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}/thinking",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)"),
        TaskThinkingQuery
    ),
    responses(
        (status = 200, description = "Thinking blocks timeline with agent reasoning chain", body = ThinkingBlocksResponse),
        (status = 404, description = "Task not found", body = HistoryErrorResponse),
        (status = 400, description = "Invalid parameters", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_task_thinking_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Query(params): Query<TaskThinkingQuery>,
) -> Result<Json<ThinkingBlocksResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    let max_length = params.max_length;
    let min_length = params.min_length;

    log::info!(
        "REST API: GET /history/tasks/{}/thinking — max_length={:?}, min_length={:?}",
        task_id, max_length, min_length
    );

    let tid = task_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parse_task_thinking(&tid, max_length, min_length);
        let elapsed = start.elapsed();
        log::info!(
            "Task thinking parse for {} complete in {:.1}ms",
            tid,
            elapsed.as_secs_f64() * 1000.0
        );
        response
    })
    .await;

    match result {
        Ok(Some(response)) => {
            log::info!(
                "REST API: Task {} thinking: {} blocks, {} total chars, {} avg chars/block",
                task_id,
                response.total_thinking_blocks,
                response.total_characters,
                response.avg_block_length,
            );
            Ok(Json(response))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} not found for thinking blocks", task_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(HistoryErrorResponse {
                    error: format!("Task '{}' not found or has no conversation history", task_id),
                    code: 404,
                }),
            ))
        }
        Err(e) => {
            log::error!(
                "REST API: Failed to parse thinking blocks for task {}: {}",
                task_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to parse task thinking blocks: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

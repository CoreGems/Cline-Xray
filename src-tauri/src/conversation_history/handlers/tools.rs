//! Tools timeline handler.
//!
//! Responsibility:
//! - Tool call timeline
//! - Success / failure classification
//! - Tool filtering
//!
//! Owns: GET /history/tasks/{task_id}/tools

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use super::common::validate_task_id;
use crate::conversation_history::tools::parse_task_tools;
use crate::conversation_history::types::{HistoryErrorResponse, TaskToolsQuery, ToolCallTimelineResponse};
use crate::state::AppState;

/// Get tool call timeline for a single Cline task
///
/// Returns a timeline of all tool calls (tool_use → tool_result pairs) for a task,
/// with success/fail status extracted from the `is_error` field on tool_result blocks.
///
/// Each entry includes:
/// - Tool name, tool_use_id, message indices
/// - Tool input (truncated, 300 chars)
/// - Tool result (truncated, 200 chars)
/// - Success status: `true` (is_error absent/false), `false` (is_error=true), or `null` (no result found)
/// - Error text (truncated, 300 chars) when is_error=true
///
/// Supports filtering via:
/// - `?tool_name=execute_command` — partial match, case-insensitive
/// - `?failed_only=true` — show only failed calls (is_error=true)
///
/// Aggregate stats include success/failure/no-result counts and tool breakdown.
///
/// This is an on-demand parse — files are read from disk each request.
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}/tools",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)"),
        TaskToolsQuery
    ),
    responses(
        (status = 200, description = "Tool call timeline with success/fail status and input/result summaries", body = ToolCallTimelineResponse),
        (status = 404, description = "Task not found", body = HistoryErrorResponse),
        (status = 400, description = "Invalid parameters", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_task_tools_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Query(params): Query<TaskToolsQuery>,
) -> Result<Json<ToolCallTimelineResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    let tool_name_filter = params.tool_name.as_deref();
    let failed_only = params.failed_only.unwrap_or(false);

    log::info!(
        "REST API: GET /history/tasks/{}/tools — tool_name={:?}, failed_only={}",
        task_id, tool_name_filter, failed_only
    );

    let tid = task_id.clone();
    let filter_name = tool_name_filter.map(|s| s.to_string());

    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parse_task_tools(&tid, filter_name.as_deref(), failed_only);
        let elapsed = start.elapsed();
        log::info!(
            "Task tools parse for {} complete in {:.1}ms",
            tid,
            elapsed.as_secs_f64() * 1000.0
        );
        response
    })
    .await;

    match result {
        Ok(Some(response)) => {
            log::info!(
                "REST API: Task {} tools: {} total, {} filtered ({} success, {} fail, {} no-result)",
                task_id,
                response.total_tool_calls,
                response.filtered_count,
                response.success_count,
                response.failure_count,
                response.no_result_count,
            );
            Ok(Json(response))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} not found for tools timeline", task_id);
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
                "REST API: Failed to parse tools for task {}: {}",
                task_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to parse task tools: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

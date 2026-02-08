//! Files-in-context handler.
//!
//! Responsibility:
//! - Files-in-context audit trail
//! - Filtering by source and state
//! - Aggregated stats
//!
//! Owns: GET /history/tasks/{task_id}/files

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use super::common::validate_task_id;
use crate::conversation_history::files::parse_task_files;
use crate::conversation_history::types::{HistoryErrorResponse, TaskFilesQuery, TaskFilesResponse};
use crate::state::AppState;

/// Get files-in-context audit trail for a single Cline task
///
/// Returns the files tracked by Cline during a task, extracted from `task_metadata.json`.
/// Each file entry includes:
/// - File path (relative)
/// - Record state: "active" or "stale"
/// - Record source: "cline_edited", "read_tool", "file_mentioned", "user_edited"
/// - Timestamps for when Cline read, edited, or user edited the file
///
/// Supports optional filtering via query parameters:
/// - `?source=cline_edited` — filter by record_source
/// - `?state=active` — filter by record_state
///
/// Aggregate stats include counts for edited, read, mentioned, and user-edited files.
///
/// This is an on-demand parse — the task_metadata.json file is read from disk each request.
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}/files",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)"),
        TaskFilesQuery
    ),
    responses(
        (status = 200, description = "Files-in-context audit trail with stats and timestamps", body = TaskFilesResponse),
        (status = 404, description = "Task not found", body = HistoryErrorResponse),
        (status = 400, description = "Invalid parameters", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_task_files_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Query(params): Query<TaskFilesQuery>,
) -> Result<Json<TaskFilesResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    let source_filter = params.source.as_deref();
    let state_filter = params.state.as_deref();

    log::info!(
        "REST API: GET /history/tasks/{}/files — source={:?}, state={:?}",
        task_id, source_filter, state_filter
    );

    let tid = task_id.clone();
    let source = source_filter.map(|s| s.to_string());
    let state = state_filter.map(|s| s.to_string());

    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parse_task_files(&tid, source.as_deref(), state.as_deref());
        let elapsed = start.elapsed();
        log::info!(
            "Task files parse for {} complete in {:.1}ms",
            tid,
            elapsed.as_secs_f64() * 1000.0
        );
        response
    })
    .await;

    match result {
        Ok(Some(response)) => {
            log::info!(
                "REST API: Task {} files: {} total ({} edited, {} read, {} mentioned, {} user-edited), {} filtered",
                task_id,
                response.total_files,
                response.files_edited_count,
                response.files_read_count,
                response.files_mentioned_count,
                response.files_user_edited_count,
                response.files.len(),
            );
            Ok(Json(response))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} not found for files", task_id);
            Err((
                StatusCode::NOT_FOUND,
                Json(HistoryErrorResponse {
                    error: format!("Task '{}' not found or has no task_metadata.json", task_id),
                    code: 404,
                }),
            ))
        }
        Err(e) => {
            log::error!(
                "REST API: Failed to parse files for task {}: {}",
                task_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to parse task files: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

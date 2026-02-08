//! Messages handlers.
//!
//! Responsibility:
//! - Message-level access
//! - Pagination and filtering
//! - Single-message expansion
//!
//! Owns:
//! - GET /history/tasks/{task_id}/messages
//! - GET /history/tasks/{task_id}/messages/{index}

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use super::common::validate_task_id;
use crate::conversation_history::messages::{parse_task_messages, parse_single_message};
use crate::conversation_history::types::{
    FullMessageResponse, HistoryErrorResponse, PaginatedMessagesResponse, TaskMessagesQuery,
};
use crate::state::AppState;

/// Get paginated messages for a single Cline task
///
/// Returns a paginated list of conversation messages from `api_conversation_history.json`,
/// with timestamps joined from `ui_messages.json` via `conversationHistoryIndex`.
///
/// Each message includes:
/// - Role (user/assistant)
/// - Timestamp (from ui_messages join)
/// - Content blocks: text (truncated), thinking (truncated), tool_use (name + input summary),
///   tool_result (result summary)
///
/// Supports pagination via `?offset=` and `?limit=` (default: 20, max: 100).
/// Supports role filtering via `?role=user` or `?role=assistant`.
///
/// This is an on-demand parse — files are read from disk each request.
/// Lighter than the full task detail endpoint since it skips metadata/files/focus_chain.
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}/messages",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)"),
        TaskMessagesQuery
    ),
    responses(
        (status = 200, description = "Paginated message list with timestamps and content summaries", body = PaginatedMessagesResponse),
        (status = 404, description = "Task not found", body = HistoryErrorResponse),
        (status = 400, description = "Invalid parameters", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_task_messages_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Query(params): Query<TaskMessagesQuery>,
) -> Result<Json<PaginatedMessagesResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    // Validate role filter
    if let Some(ref role) = params.role {
        if role != "user" && role != "assistant" {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(HistoryErrorResponse {
                    error: format!("Invalid role '{}': must be 'user' or 'assistant'", role),
                    code: 400,
                }),
            ));
        }
    }

    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(20).min(100); // default 20, max 100
    let role_filter = params.role.clone();

    log::info!(
        "REST API: GET /history/tasks/{}/messages — offset={}, limit={}, role={:?}",
        task_id, offset, limit, role_filter
    );

    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parse_task_messages(&tid, offset, limit, role_filter.as_deref());
        let elapsed = start.elapsed();
        log::info!(
            "Task messages parse for {} complete in {:.1}ms",
            tid,
            elapsed.as_secs_f64() * 1000.0
        );
        response
    })
    .await;

    match result {
        Ok(Some(response)) => {
            log::info!(
                "REST API: Task {} messages: returning {} of {} (filtered {}, offset {}, has_more {})",
                task_id,
                response.messages.len(),
                response.total_messages,
                response.filtered_count,
                response.offset,
                response.has_more,
            );
            Ok(Json(response))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} not found for messages", task_id);
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
                "REST API: Failed to parse messages for task {}: {}",
                task_id, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to parse task messages: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Get a single message with full untruncated content
///
/// Returns a single conversation message by its array index, with **full untruncated content**
/// including complete thinking blocks, full tool inputs (pretty-printed JSON), and full
/// tool result outputs.
///
/// This is the "expand" endpoint — the paginated message list truncates content for
/// performance, but this endpoint returns everything for a single message.
///
/// Timestamps are joined from `ui_messages.json` via `conversationHistoryIndex`.
///
/// Use case: user clicks "expand" on a message in the UI to see full thinking,
/// full tool input/result, or full text content.
#[utoipa::path(
    get,
    path = "/history/tasks/{task_id}/messages/{index}",
    params(
        ("task_id" = String, Path, description = "Task ID (epoch milliseconds directory name)"),
        ("index" = usize, Path, description = "Message index in the conversation history array (0-based)")
    ),
    responses(
        (status = 200, description = "Single message with full untruncated content", body = FullMessageResponse),
        (status = 404, description = "Task or message not found", body = HistoryErrorResponse),
        (status = 400, description = "Invalid parameters", body = HistoryErrorResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_single_message_handler(
    State(_state): State<Arc<AppState>>,
    Path((task_id, msg_index)): Path<(String, usize)>,
) -> Result<Json<FullMessageResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    validate_task_id(&task_id)?;

    log::info!(
        "REST API: GET /history/tasks/{}/messages/{} — fetching single message with full content",
        task_id, msg_index
    );

    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parse_single_message(&tid, msg_index);
        let elapsed = start.elapsed();
        log::info!(
            "Single message parse for {}[{}] complete in {:.1}ms",
            tid,
            msg_index,
            elapsed.as_secs_f64() * 1000.0
        );
        response
    })
    .await;

    match result {
        Ok(Some(response)) => {
            let total_content_chars: usize = response
                .content
                .iter()
                .map(|b| {
                    b.text_length.unwrap_or(0)
                        + b.tool_input_length.unwrap_or(0)
                        + b.tool_result_length.unwrap_or(0)
                })
                .sum();

            // Warn if payload is unusually large (soft guardrail for observability)
            const LARGE_PAYLOAD_THRESHOLD: usize = 1_000_000; // 1 MB
            if total_content_chars > LARGE_PAYLOAD_THRESHOLD {
                log::warn!(
                    "REST API: Large message payload for task {} message #{}: {} chars ({:.1} MB) - consider pagination or streaming for UI",
                    task_id, msg_index, total_content_chars, total_content_chars as f64 / 1_000_000.0
                );
            }

            log::info!(
                "REST API: Task {} message #{}: role={}, {} blocks, ~{} chars total",
                task_id,
                msg_index,
                response.role,
                response.content.len(),
                total_content_chars
            );
            Ok(Json(response))
        }
        Ok(None) => {
            log::warn!("REST API: Task {} message #{} not found", task_id, msg_index);
            Err((
                StatusCode::NOT_FOUND,
                Json(HistoryErrorResponse {
                    error: format!(
                        "Message index {} not found in task '{}' (task may not exist or index is out of bounds)",
                        msg_index, task_id
                    ),
                    code: 404,
                }),
            ))
        }
        Err(e) => {
            log::error!(
                "REST API: Failed to parse message {}[{}]: {}",
                task_id, msg_index, e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to parse single message: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

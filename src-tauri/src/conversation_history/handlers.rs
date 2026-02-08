//! Axum route handlers for the Conversation History API.
//!
//! Follows the same pattern as `shadow_git/handlers.rs`:
//! - Memory cache (RwLock) + disk cache
//! - Lazy loading from disk on first access
//! - spawn_blocking for heavy filesystem I/O

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::state::AppState;
use super::{cache, parser};
use super::types::{FullMessageResponse, HistoryErrorResponse, HistoryTasksQuery, PaginatedMessagesResponse, TaskDetailResponse, TaskHistoryListResponse, TaskMessagesQuery, TaskThinkingQuery, TaskToolsQuery, ThinkingBlocksResponse, ToolCallTimelineResponse};

// ============ In-memory cache ============

/// Cached task index (populated from disk or after scan)
static TASKS_INDEX_CACHE: once_cell::sync::Lazy<RwLock<Option<TaskHistoryListResponse>>> =
    once_cell::sync::Lazy::new(|| {
        // On first access, try loading from disk cache
        let disk = cache::load_tasks_index();
        RwLock::new(disk)
    });

// ============ Handlers ============

/// List all Cline task conversation histories
///
/// Scans the Cline task storage directory and returns a summary for each task,
/// including message counts, tool usage breakdown, model info, and file audit data.
///
/// Each task is parsed from its `api_conversation_history.json`, `task_metadata.json`,
/// and `ui_messages.json` files under `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/`.
///
/// Results are cached in memory and persisted to disk.
/// Pass `?refresh=true` to force a full re-scan from disk.
/// Supports optional `?model=`, `?limit=`, `?offset=` query parameters.
#[utoipa::path(
    get,
    path = "/history/tasks",
    params(HistoryTasksQuery),
    responses(
        (status = 200, description = "List of Cline task conversation history summaries", body = TaskHistoryListResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn list_history_tasks_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<HistoryTasksQuery>,
) -> Result<Json<TaskHistoryListResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    let force_refresh = params.refresh.unwrap_or(false);

    // Return cached data if available and not refreshing
    if !force_refresh {
        let cache = TASKS_INDEX_CACHE.read();
        if let Some(ref cached) = *cache {
            log::info!(
                "REST API: GET /history/tasks — returning {} cached tasks",
                cached.total_tasks
            );
            return Ok(Json(apply_filters(cached.clone(), &params)));
        }
    }

    log::info!(
        "REST API: GET /history/tasks — scanning all tasks (refresh={})",
        force_refresh
    );

    // Run scan in blocking context (heavy filesystem I/O — reads ~180 MB of JSON)
    let result = tokio::task::spawn_blocking(|| {
        let start = std::time::Instant::now();
        let response = parser::scan_all_tasks();
        let elapsed = start.elapsed();
        log::info!(
            "Conversation history scan complete: {} tasks in {:.1}s",
            response.total_tasks,
            elapsed.as_secs_f64()
        );
        response
    })
    .await;

    match result {
        Ok(response) => {
            log::info!(
                "REST API: Scanned {} tasks ({} messages, {} tool calls, {:.1} MB) — caching",
                response.total_tasks,
                response.total_messages,
                response.total_tool_calls,
                response.total_api_history_bytes as f64 / 1024.0 / 1024.0
            );

            // Update memory cache
            *TASKS_INDEX_CACHE.write() = Some(response.clone());
            // Persist to disk
            cache::save_tasks_index(&response);

            Ok(Json(apply_filters(response, &params)))
        }
        Err(e) => {
            log::error!("REST API: Failed to scan task histories: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to scan task histories: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

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
    // Validate task_id format
    if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HistoryErrorResponse {
                error: format!("Invalid task_id '{}': must be a numeric epoch milliseconds value", task_id),
                code: 400,
            }),
        ));
    }

    let max_length = params.max_length;
    let min_length = params.min_length;

    log::info!(
        "REST API: GET /history/tasks/{}/thinking — max_length={:?}, min_length={:?}",
        task_id, max_length, min_length
    );

    let tid = task_id.clone();
    
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parser::parse_task_thinking(&tid, max_length, min_length);
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
            log::error!("REST API: Failed to parse thinking blocks for task {}: {}", task_id, e);
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
    // Validate task_id format
    if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HistoryErrorResponse {
                error: format!("Invalid task_id '{}': must be a numeric epoch milliseconds value", task_id),
                code: 400,
            }),
        ));
    }

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
        let response = parser::parse_task_tools(&tid, filter_name.as_deref(), failed_only);
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
            log::error!("REST API: Failed to parse tools for task {}: {}", task_id, e);
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

/// Apply optional query filters (model, limit, offset) to the response
fn apply_filters(
    mut response: TaskHistoryListResponse,
    params: &HistoryTasksQuery,
) -> TaskHistoryListResponse {
    // Filter by model if specified
    if let Some(ref model) = params.model {
        response.tasks.retain(|t| {
            t.model_id
                .as_deref()
                .map(|m| m.contains(model.as_str()))
                .unwrap_or(false)
        });
        response.total_tasks = response.tasks.len();
    }

    // Apply offset
    let offset = params.offset.unwrap_or(0);
    if offset > 0 && offset < response.tasks.len() {
        response.tasks = response.tasks.split_off(offset);
    } else if offset >= response.tasks.len() {
        response.tasks.clear();
    }

    // Apply limit
    if let Some(limit) = params.limit {
        response.tasks.truncate(limit);
    }

    response
}

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
    log::info!("REST API: GET /history/tasks/{} — parsing task detail", task_id);

    // Validate task_id format (should be numeric epoch ms)
    if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HistoryErrorResponse {
                error: format!("Invalid task_id '{}': must be a numeric epoch milliseconds value", task_id),
                code: 400,
            }),
        ));
    }

    // Run parse in blocking context (filesystem I/O — may read up to ~4 MB of JSON)
    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let detail = parser::parse_task_detail(&tid);
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
    // Validate task_id format
    if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HistoryErrorResponse {
                error: format!("Invalid task_id '{}': must be a numeric epoch milliseconds value", task_id),
                code: 400,
            }),
        ));
    }

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
        let response = parser::parse_task_messages(
            &tid,
            offset,
            limit,
            role_filter.as_deref(),
        );
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
            log::error!("REST API: Failed to parse messages for task {}: {}", task_id, e);
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
    Path((task_id, index)): Path<(String, String)>,
) -> Result<Json<FullMessageResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    // Validate task_id format
    if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HistoryErrorResponse {
                error: format!("Invalid task_id '{}': must be a numeric epoch milliseconds value", task_id),
                code: 400,
            }),
        ));
    }

    // Validate and parse index
    let msg_index: usize = match index.parse() {
        Ok(i) => i,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(HistoryErrorResponse {
                    error: format!("Invalid message index '{}': must be a non-negative integer", index),
                    code: 400,
                }),
            ));
        }
    };

    log::info!(
        "REST API: GET /history/tasks/{}/messages/{} — fetching single message with full content",
        task_id, msg_index
    );

    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let response = parser::parse_single_message(&tid, msg_index);
        let elapsed = start.elapsed();
        log::info!(
            "Single message parse for {}[{}] complete in {:.1}ms",
            tid, msg_index,
            elapsed.as_secs_f64() * 1000.0
        );
        response
    })
    .await;

    match result {
        Ok(Some(response)) => {
            let total_content_chars: usize = response.content.iter().map(|b| {
                b.text_length.unwrap_or(0) + b.tool_input_length.unwrap_or(0) + b.tool_result_length.unwrap_or(0)
            }).sum();
            log::info!(
                "REST API: Task {} message #{}: role={}, {} blocks, ~{} chars total",
                task_id, msg_index, response.role,
                response.content.len(), total_content_chars
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
            log::error!("REST API: Failed to parse message {}[{}]: {}", task_id, msg_index, e);
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

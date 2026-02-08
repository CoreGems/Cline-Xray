//! Aggregate stats handler.
//!
//! Responsibility:
//! - Compute aggregate statistics across all tasks
//! - Reuse cached task index from the index handler
//!
//! Owns: GET /history/stats
//!
//! ## Correctness Notes
//!
//! - **Division by zero**: All averages are guarded by `if total_tasks > 0`.
//!   When there are zero tasks, averages return `0.0` (not NaN or Inf).
//!
//! - **Integer overflow**: `usize` sums (messages, tool calls, files) are computed
//!   with `Iterator::sum()`. With ~200 tasks × ~50 messages each = ~10,000 total,
//!   this is well within `usize` range (even on 32-bit). If the task corpus grows
//!   to millions, `checked_add` would be warranted, but that's far beyond current scale.
//!
//! - **Size fields**: `total_api_history_bytes`, `avg_task_size_bytes`,
//!   `min_task_size_bytes`, and `max_task_size_bytes` all refer to
//!   `api_conversation_history.json` file sizes only (NOT `ui_messages.json`).
//!   `total_ui_messages_bytes` is a separate field for UI message sizes.
//!
//! - **Sorting independence**: `earliest_task` and `latest_task` are computed with
//!   explicit `Iterator::min_by` / `Iterator::max_by` on `started_at`, NOT by
//!   relying on positional access (first/last). This makes the function correct
//!   regardless of the input ordering.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::conversation_history::types::{
    HistoryErrorResponse, HistoryStatsQuery, HistoryStatsResponse, TaskHistoryListResponse,
};
use crate::state::AppState;

use super::index::get_or_refresh_task_index;

/// Compute aggregate stats from a TaskHistoryListResponse.
///
/// This function is **order-independent** — it does not assume tasks are sorted.
/// All positional invariants (earliest/latest) use explicit min/max comparisons.
fn compute_stats(task_list: &TaskHistoryListResponse) -> HistoryStatsResponse {
    let total_tasks = task_list.total_tasks;
    let tasks = &task_list.tasks;

    // ---- Top-level counts ----
    // These are pre-aggregated in TaskHistoryListResponse by scan_all_tasks().
    let total_messages = task_list.total_messages;
    let total_tool_calls = task_list.total_tool_calls;
    // Thinking blocks are NOT pre-aggregated — sum from individual task summaries.
    let total_thinking_blocks: usize = tasks.iter().map(|t| t.thinking_count).sum();

    // ---- Size stats ----
    // Note: these refer to api_conversation_history.json sizes ONLY.
    // ui_messages.json sizes are tracked separately in total_ui_messages_bytes.
    let total_api_history_bytes = task_list.total_api_history_bytes;
    let total_ui_messages_bytes: u64 = tasks.iter().map(|t| t.ui_messages_size_bytes).sum();

    let (min_task_size_bytes, max_task_size_bytes) = if tasks.is_empty() {
        (0u64, 0u64)
    } else {
        // min/max of api_conversation_history.json sizes (NOT ui_messages)
        let min = tasks
            .iter()
            .map(|t| t.api_history_size_bytes)
            .min()
            .unwrap_or(0);
        let max = tasks
            .iter()
            .map(|t| t.api_history_size_bytes)
            .max()
            .unwrap_or(0);
        (min, max)
    };

    // Division by zero: guarded — returns 0.0 when total_tasks == 0.
    let avg_task_size_bytes = if total_tasks > 0 {
        total_api_history_bytes as f64 / total_tasks as f64
    } else {
        0.0
    };

    // ---- Averages ----
    // All guarded against division by zero (total_tasks == 0 → 0.0).
    let avg_messages_per_task = if total_tasks > 0 {
        total_messages as f64 / total_tasks as f64
    } else {
        0.0
    };

    let avg_tool_calls_per_task = if total_tasks > 0 {
        total_tool_calls as f64 / total_tasks as f64
    } else {
        0.0
    };

    let avg_thinking_blocks_per_task = if total_tasks > 0 {
        total_thinking_blocks as f64 / total_tasks as f64
    } else {
        0.0
    };

    let total_files_in_context: usize = tasks.iter().map(|t| t.files_in_context).sum();
    let avg_files_in_context = if total_tasks > 0 {
        total_files_in_context as f64 / total_tasks as f64
    } else {
        0.0
    };

    // ---- Tool breakdown + percentages ----
    let tool_breakdown = task_list.aggregate_tool_breakdown.clone();
    // Division by zero: guarded — empty map when total_tool_calls == 0.
    let tool_percentages: HashMap<String, f64> = if total_tool_calls > 0 {
        tool_breakdown
            .iter()
            .map(|(name, count)| {
                let pct = (*count as f64 / total_tool_calls as f64) * 100.0;
                // Round to 1 decimal place for readability
                (name.clone(), (pct * 10.0).round() / 10.0)
            })
            .collect()
    } else {
        HashMap::new()
    };

    // ---- Model usage ----
    // Count of tasks per model_id and model_provider.
    // Tasks with no model info (model_id=None) are excluded from the counts.
    let mut model_usage: HashMap<String, usize> = HashMap::new();
    let mut model_provider_usage: HashMap<String, usize> = HashMap::new();
    for task in tasks {
        if let Some(ref model_id) = task.model_id {
            *model_usage.entry(model_id.clone()).or_insert(0) += 1;
        }
        if let Some(ref provider) = task.model_provider {
            *model_provider_usage.entry(provider.clone()).or_insert(0) += 1;
        }
    }

    // ---- Cline version distribution ----
    // Tasks with no cline_version (cline_version=None) are excluded.
    let mut cline_version_usage: HashMap<String, usize> = HashMap::new();
    for task in tasks {
        if let Some(ref version) = task.cline_version {
            *cline_version_usage.entry(version.clone()).or_insert(0) += 1;
        }
    }

    // ---- File stats ----
    let total_files_edited: usize = tasks.iter().map(|t| t.files_edited).sum();
    let total_files_read: usize = tasks.iter().map(|t| t.files_read).sum();
    let tasks_with_focus_chain = tasks.iter().filter(|t| t.has_focus_chain).count();

    // ---- Time range ----
    // Use explicit min/max on started_at (ISO 8601 string — lexicographic order
    // matches chronological order for ISO 8601 with timezone offset).
    // This is ORDER-INDEPENDENT — does not rely on the task list being sorted.
    let earliest_task = tasks
        .iter()
        .map(|t| &t.started_at)
        .min()
        .cloned();
    let latest_task = tasks
        .iter()
        .map(|t| &t.started_at)
        .max()
        .cloned();

    HistoryStatsResponse {
        total_tasks,
        total_messages,
        total_tool_calls,
        total_thinking_blocks,
        total_api_history_bytes,
        total_ui_messages_bytes,
        avg_task_size_bytes,
        min_task_size_bytes,
        max_task_size_bytes,
        avg_messages_per_task,
        avg_tool_calls_per_task,
        avg_thinking_blocks_per_task,
        avg_files_in_context,
        tool_breakdown,
        tool_percentages,
        model_usage,
        model_provider_usage,
        cline_version_usage,
        total_files_in_context,
        total_files_edited,
        total_files_read,
        tasks_with_focus_chain,
        earliest_task,
        latest_task,
        tasks_root: task_list.tasks_root.clone(),
    }
}

// ============ Handler ============

/// Get aggregate statistics across all Cline task conversation histories
///
/// Computes totals, averages, breakdowns (tools, models, versions),
/// file stats, and time range across all tasks. Reuses the same cached
/// task index as GET /history/tasks for efficiency.
///
/// Pass `?refresh=true` to force a full re-scan from disk before computing stats.
#[utoipa::path(
    get,
    path = "/history/stats",
    params(HistoryStatsQuery),
    responses(
        (status = 200, description = "Aggregate statistics across all Cline task histories", body = HistoryStatsResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn get_history_stats_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<HistoryStatsQuery>,
) -> Result<Json<HistoryStatsResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    let force_refresh = params.refresh.unwrap_or(false);

    let task_list = get_or_refresh_task_index(force_refresh).await?;

    log::info!(
        "REST API: GET /history/stats — computed stats for {} tasks ({} messages, {} tool calls)",
        task_list.total_tasks,
        task_list.total_messages,
        task_list.total_tool_calls
    );

    let stats = compute_stats(&task_list);
    Ok(Json(stats))
}

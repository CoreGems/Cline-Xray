//! Types for the "Latest" composite endpoint
//!
//! Merges conversation history (prompt, subtask metadata) with shadow git
//! (diff, changed files) into a single response for zero-click consumption
//! by both the UI and external LLM/agent tool calls.

use serde::{Deserialize, Serialize};

use crate::shadow_git::types::DiffResult;

/// Summary of a subtask for the "Latest" response.
/// Provides enough metadata for the UI to render subtask tabs
/// without loading the full diff (which is done on-demand per subtask tab click).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskSummaryItem {
    /// Subtask index (0 = initial task, 1+ = feedback)
    pub subtask_index: usize,
    /// Whether this is the initial task prompt (true) or feedback (false)
    pub is_initial_task: bool,
    /// The prompt text (full, untruncated)
    pub prompt: String,
    /// ISO 8601 timestamp when this subtask was issued
    pub timestamp: String,
    /// Number of API messages in this subtask's range
    pub message_count: usize,
    /// Number of tool calls within this subtask
    pub tool_call_count: usize,
    /// Tool names used (deduplicated, sorted)
    pub tools_used: Vec<String>,
}

/// Composite response for GET /latest
///
/// Contains the latest task/subtask prompt, its diff (files + patch),
/// and contextual metadata — all in a single response. Designed for
/// both UI rendering and LLM/agent tool-use consumption.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LatestResponse {
    // ---- Identity ----
    /// Task ID (directory name, epoch milliseconds)
    pub task_id: String,
    /// Subtask index within the task (0 = initial, 1+ = feedback). Null if scope=task.
    pub subtask_index: Option<usize>,
    /// Whether the resolved subtask is the initial task prompt (true) or a feedback subtask (false)
    pub is_initial_task: Option<bool>,
    /// Total number of subtasks in this task (including initial)
    pub total_subtasks: usize,

    // ---- Prompt ----
    /// The prompt text (full, untruncated) of the latest subtask or initial task
    pub prompt: String,
    /// ISO 8601 timestamp when this prompt was issued
    pub prompt_timestamp: String,

    // ---- Diff ----
    /// The diff result (files changed + unified patch). Null if no checkpoint data.
    pub diff: Option<DiffResult>,
    /// Reason why diff is null (if applicable)
    pub no_diff_reason: Option<String>,

    // ---- Context (from conversation history) ----
    /// First message index in api_conversation_history for this subtask
    pub message_range_start: Option<usize>,
    /// Last message index (inclusive)
    pub message_range_end: Option<usize>,
    /// Number of API messages in this subtask's range
    pub message_count: usize,
    /// Number of tool calls within this subtask's message range
    pub tool_call_count: usize,
    /// Tool names used (deduplicated, sorted)
    pub tools_used: Vec<String>,

    // ---- Resolution metadata ----
    /// Workspace ID (checkpoint workspace that contains this task)
    pub workspace_id: Option<String>,
    /// ISO 8601 timestamp when the task started
    pub task_started_at: String,
    /// ISO 8601 timestamp when the task ended (last UI message)
    pub task_ended_at: Option<String>,
    /// Scope used for this response ("subtask" or "task")
    pub scope: String,

    // ---- Subtask summaries (for UI tab rendering) ----
    /// All subtasks in this task with metadata (prompt, tool counts, etc.)
    /// Diffs are NOT included — the UI fetches them on-demand per subtask tab click.
    pub subtasks: Vec<SubtaskSummaryItem>,
}

/// Query parameters for GET /latest
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LatestQuery {
    /// Scope: "subtask" (default) = latest subtask only, "task" = full task diff
    #[serde(default = "default_scope")]
    pub scope: String,
    /// Pathspec exclusion patterns (repeated), e.g. ?exclude=node_modules&exclude=target
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_scope() -> String {
    "task".to_string()
}

/// Error response for /latest
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LatestErrorResponse {
    pub error: String,
    pub code: u16,
}

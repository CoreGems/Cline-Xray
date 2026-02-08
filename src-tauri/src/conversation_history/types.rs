//! Types for the Conversation History API
//!
//! Represents Cline task data parsed from the physical JSON logs at:
//! `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/<task-id>/`

use serde::{Deserialize, Serialize};

// ============================================================================
// Response types (what our API returns)
// ============================================================================

/// Summary of a single Cline task (lightweight — no full conversation content)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistorySummary {
    /// Task ID (directory name, epoch milliseconds)
    pub task_id: String,
    /// ISO 8601 timestamp derived from task_id (task start time)
    pub started_at: String,
    /// ISO 8601 timestamp of the last UI message (task end time)
    pub ended_at: Option<String>,
    /// Total number of API messages (user + assistant turns)
    pub message_count: usize,
    /// Number of tool_use blocks across all assistant messages
    pub tool_use_count: usize,
    /// Number of thinking blocks
    pub thinking_count: usize,
    /// Tool usage breakdown: tool_name → count
    pub tool_breakdown: std::collections::HashMap<String, usize>,
    /// Model ID used (from task_metadata or first ui_message)
    pub model_id: Option<String>,
    /// Model provider (e.g. "anthropic")
    pub model_provider: Option<String>,
    /// Number of files in context (from task_metadata)
    pub files_in_context: usize,
    /// Files edited by Cline (from task_metadata)
    pub files_edited: usize,
    /// Files read by Cline (from task_metadata)
    pub files_read: usize,
    /// Cline version that created this task
    pub cline_version: Option<String>,
    /// Size of api_conversation_history.json in bytes
    pub api_history_size_bytes: u64,
    /// Size of ui_messages.json in bytes
    pub ui_messages_size_bytes: u64,
    /// Whether a focus_chain markdown file exists for this task
    pub has_focus_chain: bool,
    /// First user message text (truncated to 200 chars) — task description
    pub task_prompt: Option<String>,
}

/// Response for GET /history/tasks
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryListResponse {
    /// List of task summaries (sorted by started_at descending = newest first)
    pub tasks: Vec<TaskHistorySummary>,
    /// Total number of task directories found
    pub total_tasks: usize,
    /// Total size of all api_conversation_history.json files (bytes)
    pub total_api_history_bytes: u64,
    /// Root path that was scanned
    pub tasks_root: String,
    /// Aggregate tool usage across all tasks
    pub aggregate_tool_breakdown: std::collections::HashMap<String, usize>,
    /// Total tool calls across all tasks
    pub total_tool_calls: usize,
    /// Total messages across all tasks
    pub total_messages: usize,
}

// ============================================================================
// Task Detail response (P1: single-task deep-dive)
// ============================================================================

/// Full detail for a single Cline task — messages, tools, files, model info, env, focus chain
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetailResponse {
    /// Task ID (directory name, epoch milliseconds)
    pub task_id: String,
    /// ISO 8601 timestamp derived from task_id (task start time)
    pub started_at: String,
    /// ISO 8601 timestamp of the last UI message (task end time)
    pub ended_at: Option<String>,

    // ---- Summary stats (same as TaskHistorySummary) ----
    /// Total number of API messages (user + assistant turns)
    pub message_count: usize,
    /// Number of tool_use blocks across all assistant messages
    pub tool_use_count: usize,
    /// Number of thinking blocks
    pub thinking_count: usize,
    /// First user message text (truncated to 200 chars) — task description
    pub task_prompt: Option<String>,

    // ---- Conversation messages (truncated content) ----
    /// All conversation messages with content blocks (text/thinking truncated)
    pub messages: Vec<ConversationMessage>,

    // ---- Tool call timeline ----
    /// Extracted tool calls in order: tool name, inputs, result summary
    pub tool_calls: Vec<ToolCallDetail>,
    /// Tool usage breakdown: tool_name → count
    pub tool_breakdown: std::collections::HashMap<String, usize>,

    // ---- Files in context (from task_metadata) ----
    /// Files tracked by Cline during this task
    pub files: Vec<FileInContextDetail>,
    /// Number of files in context
    pub files_in_context_count: usize,
    /// Files edited by Cline
    pub files_edited_count: usize,
    /// Files read by Cline
    pub files_read_count: usize,

    // ---- Model info (from task_metadata) ----
    /// Model usage entries (may switch models mid-task)
    pub model_usage: Vec<ModelUsageDetail>,

    // ---- Environment info (from task_metadata) ----
    /// Environment snapshots captured during the task
    pub environment: Vec<EnvironmentDetail>,

    // ---- Focus chain ----
    /// Focus chain / task progress checklist (markdown content, if present)
    pub focus_chain: Option<String>,
    /// Whether focus_chain file exists
    pub has_focus_chain: bool,

    // ---- File sizes ----
    /// Size of api_conversation_history.json in bytes
    pub api_history_size_bytes: u64,
    /// Size of ui_messages.json in bytes
    pub ui_messages_size_bytes: u64,

    // ---- Local path ----
    /// Full local filesystem path to the task directory
    pub task_dir_path: String,
}

/// A single conversation message with its content blocks
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConversationMessage {
    /// Message index in the api_conversation_history array (0-based)
    pub index: usize,
    /// "user" or "assistant"
    pub role: String,
    /// ISO 8601 timestamp (from ui_messages join, if available)
    pub timestamp: Option<String>,
    /// Content blocks in this message
    pub content: Vec<ContentBlockSummary>,
}

/// A content block inside a message (truncated for list view)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContentBlockSummary {
    /// Block type: "text", "thinking", "tool_use", "tool_result", "unknown"
    #[serde(rename = "type")]
    pub block_type: String,
    /// Truncated text content (for text/thinking blocks — max 500 chars)
    pub text: Option<String>,
    /// Full text length in characters (so UI can show "... N more chars")
    pub full_text_length: Option<usize>,
    /// Tool use ID (for tool_use and tool_result blocks)
    pub tool_use_id: Option<String>,
    /// Tool name (for tool_use blocks)
    pub tool_name: Option<String>,
    /// Tool input summary (for tool_use blocks — first 300 chars of JSON)
    pub tool_input: Option<String>,
    /// Tool result summary (for tool_result blocks — first 200 chars)
    pub tool_result_text: Option<String>,
}

/// A tool call with associated result (extracted from messages)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallDetail {
    /// Sequential index of this tool call in the task (0-based)
    pub call_index: usize,
    /// Message index where the tool_use block appears
    pub message_index: usize,
    /// Tool name (e.g. "write_to_file", "execute_command")
    pub tool_name: String,
    /// Tool use ID (links tool_use to tool_result)
    pub tool_use_id: String,
    /// Tool input — first 300 chars of the JSON-serialized input
    pub input_summary: String,
    /// Full input length in chars
    pub input_full_length: usize,
    /// Tool result — first 200 chars of the result text (from the next tool_result block)
    pub result_summary: Option<String>,
    /// Full result length in chars
    pub result_full_length: Option<usize>,
}

/// A file tracked by Cline during the task (from task_metadata.json)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileInContextDetail {
    /// File path (relative)
    pub path: String,
    /// Record state: "active" or "stale"
    pub record_state: Option<String>,
    /// Record source: "file_mentioned", "read_tool", "cline_edited", "user_edited"
    pub record_source: Option<String>,
    /// ISO 8601 timestamp when Cline read this file
    pub cline_read_date: Option<String>,
    /// ISO 8601 timestamp when Cline edited this file
    pub cline_edit_date: Option<String>,
    /// ISO 8601 timestamp when user edited this file
    pub user_edit_date: Option<String>,
}

/// Model usage entry from task_metadata
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsageDetail {
    /// ISO 8601 timestamp
    pub timestamp: Option<String>,
    /// Model ID (e.g. "claude-sonnet-4-5-20250929")
    pub model_id: Option<String>,
    /// Provider ID (e.g. "anthropic")
    pub model_provider_id: Option<String>,
    /// Mode: "act" or "plan"
    pub mode: Option<String>,
}

/// Environment snapshot from task_metadata
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentDetail {
    /// ISO 8601 timestamp
    pub timestamp: Option<String>,
    /// OS name (e.g. "win32")
    pub os_name: Option<String>,
    /// OS version (e.g. "10.0.17763")
    pub os_version: Option<String>,
    /// Host application (e.g. "Visual Studio Code")
    pub host_name: Option<String>,
    /// Host version (e.g. "1.106.3")
    pub host_version: Option<String>,
    /// Cline extension version (e.g. "3.39.2")
    pub cline_version: Option<String>,
}

/// Error response for history endpoints
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HistoryErrorResponse {
    pub error: String,
    pub code: u16,
}

// ============================================================================
// Internal parsing types (Cline's JSON format)
// ============================================================================

/// A single message in api_conversation_history.json
#[derive(Debug, Deserialize)]
pub struct RawApiMessage {
    pub role: String,
    pub content: Vec<RawContentBlock>,
}

/// A content block inside a message
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum RawContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
        /// true if the tool call returned an error (Anthropic API convention)
        #[serde(default)]
        is_error: Option<bool>,
    },
    /// Catch-all for unknown block types (future Cline versions)
    #[serde(other)]
    Unknown,
}

/// A UI message from ui_messages.json (timestamps + subtask detection)
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawUiMessage {
    pub ts: u64,
    #[serde(rename = "type")]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub conversation_history_index: Option<i64>,
    /// The "say" sub-type: "task", "user_feedback", "api_req_started", etc.
    #[serde(default)]
    pub say: Option<String>,
    /// Text content (task prompt for say="task", feedback text for say="user_feedback")
    #[serde(default)]
    pub text: Option<String>,
}

/// task_metadata.json structure
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RawTaskMetadata {
    #[serde(default)]
    pub files_in_context: Vec<RawFileInContext>,
    #[serde(default)]
    pub model_usage: Vec<RawModelUsage>,
    #[serde(default)]
    pub environment_history: Vec<RawEnvironmentEntry>,
}

/// A file-in-context entry from task_metadata
#[derive(Debug, Deserialize)]
pub struct RawFileInContext {
    pub path: String,
    pub record_state: Option<String>,
    pub record_source: Option<String>,
    pub cline_read_date: Option<u64>,
    pub cline_edit_date: Option<u64>,
    pub user_edit_date: Option<u64>,
}

/// Model usage entry from task_metadata
#[derive(Debug, Deserialize)]
pub struct RawModelUsage {
    pub ts: Option<u64>,
    pub model_id: Option<String>,
    pub model_provider_id: Option<String>,
    pub mode: Option<String>,
}

/// Environment history entry from task_metadata
#[derive(Debug, Deserialize)]
pub struct RawEnvironmentEntry {
    pub ts: Option<u64>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub host_name: Option<String>,
    pub host_version: Option<String>,
    pub cline_version: Option<String>,
}

// ============================================================================
// Query parameters
// ============================================================================

/// Query parameters for GET /history/tasks
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct HistoryTasksQuery {
    /// Set to true to force re-scan from disk (bypass cache)
    #[serde(default)]
    pub refresh: Option<bool>,
    /// Filter by model_id (e.g. "claude-sonnet-4-5-20250929")
    #[serde(default)]
    pub model: Option<String>,
    /// Limit number of results (default: all)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: Option<usize>,
}

/// Query parameters for GET /history/tasks/:taskId/messages
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TaskMessagesQuery {
    /// Offset into the message list (default: 0)
    #[serde(default)]
    pub offset: Option<usize>,
    /// Maximum number of messages to return (default: 20, max: 100)
    #[serde(default)]
    pub limit: Option<usize>,
    /// Filter by role: "user" or "assistant" (default: all)
    #[serde(default)]
    pub role: Option<String>,
}

/// Query parameters for GET /history/tasks/:taskId/tools
#[derive(Debug, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TaskToolsQuery {
    /// Filter by tool name (e.g. "write_to_file", "execute_command"). Partial match supported.
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Filter to only show failed tool calls (is_error=true)
    #[serde(default)]
    pub failed_only: Option<bool>,
}

// ============================================================================
// Tool Call Timeline response (GET /history/tasks/:taskId/tools)
// ============================================================================

/// A single tool call in the timeline with its result and success/fail status
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallTimelineEntry {
    /// Sequential index of this tool call in the task (0-based)
    pub call_index: usize,
    /// Message index where the tool_use block appears in api_conversation_history
    pub message_index: usize,
    /// Message index where the tool_result block appears (if found)
    pub result_message_index: Option<usize>,
    /// ISO 8601 timestamp of the tool_use message (from ui_messages join)
    pub timestamp: Option<String>,
    /// Tool name (e.g. "write_to_file", "execute_command", "read_file")
    pub tool_name: String,
    /// Tool use ID (links tool_use → tool_result in the Anthropic API format)
    pub tool_use_id: String,
    /// Truncated tool input (first 300 chars of JSON-serialized input)
    pub input_summary: String,
    /// Full input length in chars (so UI can show "… N more chars")
    pub input_full_length: usize,
    /// Truncated tool result (first 200 chars of result text)
    pub result_summary: Option<String>,
    /// Full result length in chars
    pub result_full_length: Option<usize>,
    /// Whether the tool call succeeded (true) or failed (false).
    /// Determined by the `is_error` field on the tool_result block.
    /// If no tool_result was found (orphaned tool_use), this is `None`.
    pub success: Option<bool>,
    /// Error text extracted from the tool_result when is_error=true (truncated to 300 chars)
    pub error_text: Option<String>,
}

/// Response for GET /history/tasks/:taskId/tools — tool call timeline
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallTimelineResponse {
    /// Task ID
    pub task_id: String,
    /// Total number of tool calls in this task (before filtering)
    pub total_tool_calls: usize,
    /// Number of tool calls after filtering (by tool_name / failed_only)
    pub filtered_count: usize,
    /// Number of successful tool calls (is_error absent or false)
    pub success_count: usize,
    /// Number of failed tool calls (is_error=true)
    pub failure_count: usize,
    /// Number of tool calls with no result (orphaned — no matching tool_result block)
    pub no_result_count: usize,
    /// Tool usage breakdown: tool_name → count (across all calls, before filtering)
    pub tool_breakdown: std::collections::HashMap<String, usize>,
    /// The tool call timeline entries (filtered, in chronological order)
    pub tool_calls: Vec<ToolCallTimelineEntry>,
}

// ============================================================================
// Paginated Messages response (P1.5: GET /history/tasks/:taskId/messages)
// ============================================================================

// ============================================================================
// Single Message response (P1.6: GET /history/tasks/:taskId/messages/:index)
// ============================================================================

/// Full single message with untruncated content
/// Response for GET /history/tasks/:taskId/messages/:index
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FullMessageResponse {
    /// Task ID
    pub task_id: String,
    /// Message index in the api_conversation_history array (0-based)
    pub index: usize,
    /// Total number of messages in this task
    pub total_messages: usize,
    /// "user" or "assistant"
    pub role: String,
    /// ISO 8601 timestamp (from ui_messages join, if available)
    pub timestamp: Option<String>,
    /// Content blocks — full untruncated content
    pub content: Vec<FullContentBlock>,
}

/// A content block with FULL untruncated content (for single message view)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FullContentBlock {
    /// Block type: "text", "thinking", "tool_use", "tool_result", "unknown"
    #[serde(rename = "type")]
    pub block_type: String,
    /// Full text content (for text/thinking blocks — NOT truncated)
    pub text: Option<String>,
    /// Text length in characters
    pub text_length: Option<usize>,
    /// Tool use ID (for tool_use and tool_result blocks)
    pub tool_use_id: Option<String>,
    /// Tool name (for tool_use blocks)
    pub tool_name: Option<String>,
    /// Full tool input as JSON string (NOT truncated)
    pub tool_input: Option<String>,
    /// Full tool input length in chars
    pub tool_input_length: Option<usize>,
    /// Full tool result text (NOT truncated)
    pub tool_result_text: Option<String>,
    /// Full tool result length in chars
    pub tool_result_length: Option<usize>,
}

/// Response for GET /history/tasks/:taskId/messages — paginated message list
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedMessagesResponse {
    /// Task ID
    pub task_id: String,
    /// Total number of messages in this task (before filtering)
    pub total_messages: usize,
    /// Number of messages after role filtering (before pagination)
    pub filtered_count: usize,
    /// Current page offset
    pub offset: usize,
    /// Current page limit
    pub limit: usize,
    /// Whether there are more messages after this page
    pub has_more: bool,
    /// The messages in this page
    pub messages: Vec<ConversationMessage>,
}

// ============================================================================
// Thinking Blocks response (GET /history/tasks/:taskId/thinking)
// ============================================================================

/// A single thinking block entry in the timeline
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingBlockEntry {
    /// Sequential index of this thinking block in the task (0-based)
    pub block_index: usize,
    /// Message index where this thinking block appears in api_conversation_history
    pub message_index: usize,
    /// ISO 8601 timestamp of the message containing this thinking block (from ui_messages join)
    pub timestamp: Option<String>,
    /// The thinking content (truncated to max_length if specified, otherwise full)
    pub thinking: String,
    /// Full thinking text length in characters
    pub full_length: usize,
    /// Whether this thinking block was truncated
    pub is_truncated: bool,
}

/// Response for GET /history/tasks/:taskId/thinking — thinking blocks timeline
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingBlocksResponse {
    /// Task ID
    pub task_id: String,
    /// Total number of thinking blocks in this task
    pub total_thinking_blocks: usize,
    /// Total characters across all thinking blocks
    pub total_characters: usize,
    /// Average thinking block length in characters
    pub avg_block_length: usize,
    /// The thinking block entries (in chronological order)
    pub thinking_blocks: Vec<ThinkingBlockEntry>,
}

/// Query parameters for GET /history/tasks/:taskId/thinking
#[derive(Debug, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TaskThinkingQuery {
    /// Maximum characters per thinking block (default: 1000, set to 0 for no truncation)
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Minimum thinking block length to include (filter out short blocks)
    #[serde(default)]
    pub min_length: Option<usize>,
}

// ============================================================================
// Files in Context response (GET /history/tasks/:taskId/files)
// ============================================================================

/// Response for GET /history/tasks/:taskId/files — files-in-context audit trail
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilesResponse {
    /// Task ID
    pub task_id: String,
    /// Total number of files in context (before filtering)
    pub total_files: usize,
    /// Number of files edited by Cline (record_source = "cline_edited")
    pub files_edited_count: usize,
    /// Number of files read by Cline (record_source = "read_tool")
    pub files_read_count: usize,
    /// Number of files mentioned (record_source = "file_mentioned")
    pub files_mentioned_count: usize,
    /// Number of files edited by user (record_source = "user_edited")
    pub files_user_edited_count: usize,
    /// The files in context (filtered if query params provided)
    pub files: Vec<FileInContextDetail>,
}

/// Query parameters for GET /history/tasks/:taskId/files
#[derive(Debug, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct TaskFilesQuery {
    /// Filter by record state: "active" or "stale"
    #[serde(default)]
    pub state: Option<String>,
    /// Filter by record source: "cline_edited", "read_tool", "file_mentioned", "user_edited"
    #[serde(default)]
    pub source: Option<String>,
}

// ============================================================================
// Aggregate Stats response (GET /history/stats)
// ============================================================================

/// Aggregate statistics across all Cline task conversation histories
///
/// Computed by scanning all task summaries and aggregating counts, breakdowns,
/// and averages. Reuses the same cached task index as GET /history/tasks.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStatsResponse {
    // ---- Top-level counts ----
    /// Total number of task directories found
    pub total_tasks: usize,
    /// Total API messages across all tasks (user + assistant turns)
    pub total_messages: usize,
    /// Total tool calls across all tasks
    pub total_tool_calls: usize,
    /// Total thinking blocks across all tasks
    pub total_thinking_blocks: usize,

    // ---- Size stats ----
    // NOTE: All "task size" fields below refer to api_conversation_history.json ONLY,
    // not ui_messages.json. The two file types are tracked separately because they
    // have different sizes and parsing costs.
    /// Total size of all api_conversation_history.json files across all tasks (bytes)
    pub total_api_history_bytes: u64,
    /// Total size of all ui_messages.json files across all tasks (bytes, tracked separately)
    pub total_ui_messages_bytes: u64,
    /// Average api_conversation_history.json size per task (bytes). 0.0 if no tasks.
    pub avg_task_size_bytes: f64,
    /// Smallest api_conversation_history.json among all tasks (bytes). 0 if no tasks.
    pub min_task_size_bytes: u64,
    /// Largest api_conversation_history.json among all tasks (bytes). 0 if no tasks.
    pub max_task_size_bytes: u64,

    // ---- Averages ----
    /// Average messages per task
    pub avg_messages_per_task: f64,
    /// Average tool calls per task
    pub avg_tool_calls_per_task: f64,
    /// Average thinking blocks per task
    pub avg_thinking_blocks_per_task: f64,
    /// Average files in context per task
    pub avg_files_in_context: f64,

    // ---- Tool breakdown ----
    /// Aggregate tool usage breakdown: tool_name → total count across all tasks
    pub tool_breakdown: std::collections::HashMap<String, usize>,
    /// Tool usage as percentages: tool_name → percentage of all tool calls
    pub tool_percentages: std::collections::HashMap<String, f64>,

    // ---- Model usage ----
    /// Model usage breakdown: model_id → number of tasks using that model
    pub model_usage: std::collections::HashMap<String, usize>,
    /// Model provider breakdown: provider_id → number of tasks using that provider
    pub model_provider_usage: std::collections::HashMap<String, usize>,

    // ---- Cline version distribution ----
    /// Cline version breakdown: version → number of tasks
    pub cline_version_usage: std::collections::HashMap<String, usize>,

    // ---- File stats ----
    /// Total files in context across all tasks
    pub total_files_in_context: usize,
    /// Total files edited by Cline across all tasks
    pub total_files_edited: usize,
    /// Total files read by Cline across all tasks
    pub total_files_read: usize,
    /// Number of tasks with a focus chain file
    pub tasks_with_focus_chain: usize,

    // ---- Time range ----
    /// ISO 8601 timestamp of the earliest task
    pub earliest_task: Option<String>,
    /// ISO 8601 timestamp of the most recent task
    pub latest_task: Option<String>,

    /// Root path that was scanned
    pub tasks_root: String,
}

/// Query parameters for GET /history/stats
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct HistoryStatsQuery {
    /// Set to true to force re-scan from disk (bypass cache)
    #[serde(default)]
    pub refresh: Option<bool>,
}

// ============================================================================
// Subtask Detection response (GET /history/tasks/:taskId/subtasks)
// ============================================================================

/// A detected subtask within a Cline task conversation.
///
/// Tasks are often multi-phase: the user provides additional instructions via
/// `<feedback>` tags after seeing the initial result. Each feedback-driven prompt
/// is a subtask that maps to a contiguous range of api_conversation_history messages.
///
/// Detection source: `ui_messages.json` entries where `say = "task"` (initial)
/// or `say = "user_feedback"` (subsequent subtasks).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskEntry {
    /// Subtask index (0 = initial task, 1+ = feedback-driven subtasks)
    pub subtask_index: usize,
    /// The subtask prompt text
    pub prompt: String,
    /// ISO 8601 timestamp when this subtask was issued
    pub timestamp: String,
    /// Whether this is the initial task (true) or a feedback subtask (false)
    pub is_initial_task: bool,
    /// First message index in api_conversation_history for this subtask
    pub message_range_start: usize,
    /// Last message index (inclusive). None if extends to end of conversation.
    pub message_range_end: Option<usize>,
    /// Number of API messages in this subtask's range
    pub message_count: usize,
    /// Number of tool calls within this subtask's message range
    pub tool_call_count: usize,
    /// Tool names used in this subtask (deduplicated)
    pub tools_used: Vec<String>,
}

/// Response for GET /history/tasks/:taskId/subtasks — subtask detection timeline
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtasksResponse {
    /// Task ID
    pub task_id: String,
    /// Total number of subtasks (including initial task)
    pub total_subtasks: usize,
    /// Whether this task has any feedback-driven subtasks (total > 1)
    pub has_subtasks: bool,
    /// The detected subtasks in chronological order
    pub subtasks: Vec<SubtaskEntry>,
}

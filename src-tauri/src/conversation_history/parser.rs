//! Parser for Cline task data files.
//!
//! Scans `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/`
//! and produces `TaskHistorySummary` for each task directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::*;

/// Maximum characters to keep for task_prompt truncation
const PROMPT_TRUNCATE_LEN: usize = 200;
/// Maximum characters for text/thinking blocks in detail view
const TEXT_TRUNCATE_LEN: usize = 500;
/// Maximum characters for tool input summary in detail view
const TOOL_INPUT_TRUNCATE_LEN: usize = 300;
/// Maximum characters for tool result summary in detail view
const TOOL_RESULT_TRUNCATE_LEN: usize = 200;
/// Maximum characters for error text in tool timeline
const ERROR_TEXT_TRUNCATE_LEN: usize = 300;

/// Safely truncate a UTF-8 string to at most `max_chars` characters.
/// Avoids panicking on multi-byte character boundaries (e.g. `—`, `…`, emoji).
fn truncate_utf8(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}…", truncated)
    }
}

/// Return the Cline tasks root directory
pub fn tasks_root() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let root = PathBuf::from(appdata)
        .join("Code")
        .join("User")
        .join("globalStorage")
        .join("saoudrizwan.claude-dev")
        .join("tasks");
    if root.exists() {
        Some(root)
    } else {
        log::warn!("Cline tasks root not found: {:?}", root);
        None
    }
}

/// Convert epoch milliseconds to ISO 8601 string
fn epoch_ms_to_iso(epoch_ms: u64) -> String {
    let secs = (epoch_ms / 1000) as i64;
    let nanos = ((epoch_ms % 1000) * 1_000_000) as u32;
    match chrono::DateTime::from_timestamp(secs, nanos) {
        Some(dt) => dt.with_timezone(&chrono::Local).to_rfc3339(),
        None => format!("{}ms", epoch_ms),
    }
}

/// Scan all task directories and produce summaries.
///
/// This parses each task's files (api_conversation_history.json, task_metadata.json,
/// ui_messages.json) to extract summary statistics. Large files are parsed with
/// streaming where possible.
pub fn scan_all_tasks() -> TaskHistoryListResponse {
    let root = match tasks_root() {
        Some(r) => r,
        None => {
            return TaskHistoryListResponse {
                tasks: vec![],
                total_tasks: 0,
                total_api_history_bytes: 0,
                tasks_root: "NOT FOUND".to_string(),
                aggregate_tool_breakdown: HashMap::new(),
                total_tool_calls: 0,
                total_messages: 0,
            };
        }
    };

    let root_str = root.to_string_lossy().to_string();
    let mut tasks = Vec::new();
    let mut total_api_bytes: u64 = 0;
    let mut aggregate_tools: HashMap<String, usize> = HashMap::new();
    let mut total_tool_calls: usize = 0;
    let mut total_messages: usize = 0;

    // Read task directories
    let entries = match std::fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to read tasks directory {:?}: {}", root, e);
            return TaskHistoryListResponse {
                tasks: vec![],
                total_tasks: 0,
                total_api_history_bytes: 0,
                tasks_root: root_str,
                aggregate_tool_breakdown: HashMap::new(),
                total_tool_calls: 0,
                total_messages: 0,
            };
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let task_id = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        // Parse this task directory
        match parse_task_dir(&task_id, &path) {
            Some(summary) => {
                total_api_bytes += summary.api_history_size_bytes;
                total_messages += summary.message_count;
                total_tool_calls += summary.tool_use_count;

                // Aggregate tool breakdown
                for (tool, count) in &summary.tool_breakdown {
                    *aggregate_tools.entry(tool.clone()).or_insert(0) += count;
                }

                tasks.push(summary);
            }
            None => {
                log::debug!("Skipping task dir {:?} (no parseable data)", path);
            }
        }
    }

    let total_tasks = tasks.len();

    // Sort by started_at descending (newest first)
    tasks.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    TaskHistoryListResponse {
        tasks,
        total_tasks,
        total_api_history_bytes: total_api_bytes,
        tasks_root: root_str,
        aggregate_tool_breakdown: aggregate_tools,
        total_tool_calls,
        total_messages,
    }
}

/// Parse a single task directory into a TaskHistorySummary
fn parse_task_dir(task_id: &str, dir: &Path) -> Option<TaskHistorySummary> {
    let api_history_path = dir.join("api_conversation_history.json");
    let metadata_path = dir.join("task_metadata.json");
    let ui_messages_path = dir.join("ui_messages.json");

    // We need at least api_conversation_history.json to produce a summary
    if !api_history_path.exists() {
        return None;
    }

    // Get file sizes (cheap — no parsing needed)
    let api_size = std::fs::metadata(&api_history_path).map(|m| m.len()).unwrap_or(0);
    let ui_size = std::fs::metadata(&ui_messages_path).map(|m| m.len()).unwrap_or(0);

    // Derive start time from task_id (it's epoch ms)
    let started_at = match task_id.parse::<u64>() {
        Ok(epoch_ms) => epoch_ms_to_iso(epoch_ms),
        Err(_) => "unknown".to_string(),
    };

    // Check for focus_chain file
    let focus_chain_name = format!("focus_chain_taskid_{}.md", task_id);
    let has_focus_chain = dir.join(&focus_chain_name).exists();

    // Parse api_conversation_history.json
    let (message_count, tool_use_count, thinking_count, tool_breakdown, task_prompt) =
        parse_api_history(&api_history_path);

    // Parse task_metadata.json (lightweight)
    let (model_id, model_provider, cline_version, files_in_context, files_edited, files_read) =
        parse_task_metadata(&metadata_path);

    // Get end time from ui_messages.json (just the last timestamp)
    let ended_at = parse_ui_messages_end_time(&ui_messages_path);

    Some(TaskHistorySummary {
        task_id: task_id.to_string(),
        started_at,
        ended_at,
        message_count,
        tool_use_count,
        thinking_count,
        tool_breakdown,
        model_id,
        model_provider,
        files_in_context,
        files_edited,
        files_read,
        cline_version,
        api_history_size_bytes: api_size,
        ui_messages_size_bytes: ui_size,
        has_focus_chain,
        task_prompt,
    })
}

/// Parse api_conversation_history.json for summary stats.
///
/// Returns: (message_count, tool_use_count, thinking_count, tool_breakdown, task_prompt)
fn parse_api_history(
    path: &Path,
) -> (
    usize,
    usize,
    usize,
    HashMap<String, usize>,
    Option<String>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", path, e);
            return (0, 0, 0, HashMap::new(), None);
        }
    };

    let messages: Vec<RawApiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", path, e);
            return (0, 0, 0, HashMap::new(), None);
        }
    };

    let message_count = messages.len();
    let mut tool_use_count = 0usize;
    let mut thinking_count = 0usize;
    let mut tool_breakdown: HashMap<String, usize> = HashMap::new();
    let mut task_prompt: Option<String> = None;

    for msg in &messages {
        // Extract task prompt from first user message
        if task_prompt.is_none() && msg.role == "user" {
            for block in &msg.content {
                if let RawContentBlock::Text { text } = block {
                    let truncated = truncate_utf8(text, PROMPT_TRUNCATE_LEN);
                    task_prompt = Some(truncated);
                    break;
                }
            }
        }

        for block in &msg.content {
            match block {
                RawContentBlock::ToolUse { name, .. } => {
                    tool_use_count += 1;
                    *tool_breakdown.entry(name.clone()).or_insert(0) += 1;
                }
                RawContentBlock::Thinking { .. } => {
                    thinking_count += 1;
                }
                _ => {}
            }
        }
    }

    (
        message_count,
        tool_use_count,
        thinking_count,
        tool_breakdown,
        task_prompt,
    )
}

/// Parse task_metadata.json for model info, cline version, file counts.
///
/// Returns: (model_id, model_provider, cline_version, files_in_context, files_edited, files_read)
fn parse_task_metadata(
    path: &Path,
) -> (Option<String>, Option<String>, Option<String>, usize, usize, usize) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, None, None, 0, 0, 0),
    };

    let metadata: RawTaskMetadata = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse task_metadata {:?}: {}", path, e);
            return (None, None, None, 0, 0, 0);
        }
    };

    // Model info from first model_usage entry
    let model_id = metadata
        .model_usage
        .first()
        .and_then(|m| m.model_id.clone());
    let model_provider = metadata
        .model_usage
        .first()
        .and_then(|m| m.model_provider_id.clone());

    // Cline version from first environment_history entry
    let cline_version = metadata
        .environment_history
        .first()
        .and_then(|e| e.cline_version.clone());

    // File counts
    let files_in_context = metadata.files_in_context.len();
    let files_edited = metadata
        .files_in_context
        .iter()
        .filter(|f| {
            f.record_source
                .as_deref()
                .map(|s| s == "cline_edited")
                .unwrap_or(false)
        })
        .count();
    let files_read = metadata
        .files_in_context
        .iter()
        .filter(|f| {
            f.record_source
                .as_deref()
                .map(|s| s == "read_tool")
                .unwrap_or(false)
        })
        .count();

    (
        model_id,
        model_provider,
        cline_version,
        files_in_context,
        files_edited,
        files_read,
    )
}

/// Extract the last timestamp from ui_messages.json to get the task end time.
///
/// We do a lightweight parse — just looking for the last `ts` value.
fn parse_ui_messages_end_time(path: &Path) -> Option<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    // Parse as a Vec of RawUiMessage (we only need timestamps)
    let messages: Vec<RawUiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse ui_messages {:?}: {}", path, e);
            return None;
        }
    };

    // Get the last message's timestamp
    messages
        .last()
        .map(|m| epoch_ms_to_iso(m.ts))
}

// ============================================================================
// P1: Single-task detail parsing
// ============================================================================

/// Parse a single task directory into a full TaskDetailResponse.
///
/// This is the heavy parser — it reads all four files in the task directory:
/// - api_conversation_history.json → messages, tool calls, thinking blocks
/// - ui_messages.json → timestamps (joined by conversationHistoryIndex)
/// - task_metadata.json → files, model usage, environment
/// - focus_chain_taskid_<id>.md → task progress checklist
///
/// Returns None if the task directory doesn't exist or has no api_conversation_history.
pub fn parse_task_detail(task_id: &str) -> Option<TaskDetailResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let api_history_path = dir.join("api_conversation_history.json");
    let metadata_path = dir.join("task_metadata.json");
    let ui_messages_path = dir.join("ui_messages.json");
    let focus_chain_name = format!("focus_chain_taskid_{}.md", task_id);
    let focus_chain_path = dir.join(&focus_chain_name);

    // We need at least api_conversation_history.json
    if !api_history_path.exists() {
        log::warn!("No api_conversation_history.json for task {}", task_id);
        return None;
    }

    // File sizes
    let api_size = std::fs::metadata(&api_history_path).map(|m| m.len()).unwrap_or(0);
    let ui_size = std::fs::metadata(&ui_messages_path).map(|m| m.len()).unwrap_or(0);

    // Start time from task_id
    let started_at = match task_id.parse::<u64>() {
        Ok(epoch_ms) => epoch_ms_to_iso(epoch_ms),
        Err(_) => "unknown".to_string(),
    };

    // ---- Parse ui_messages.json for timestamp mapping ----
    let timestamp_map = build_timestamp_map(&ui_messages_path);
    let ended_at = parse_ui_messages_end_time(&ui_messages_path);

    // ---- Parse api_conversation_history.json (full detail) ----
    let (messages, tool_calls, tool_breakdown, message_count, tool_use_count, thinking_count, task_prompt) =
        parse_api_history_detail(&api_history_path, &timestamp_map);

    // ---- Parse task_metadata.json (full detail) ----
    let (files, files_in_context_count, files_edited_count, files_read_count, model_usage, environment) =
        parse_task_metadata_detail(&metadata_path);

    // ---- Read focus chain ----
    let has_focus_chain = focus_chain_path.exists();
    let focus_chain = if has_focus_chain {
        std::fs::read_to_string(&focus_chain_path).ok()
    } else {
        None
    };

    Some(TaskDetailResponse {
        task_id: task_id.to_string(),
        started_at,
        ended_at,
        message_count,
        tool_use_count,
        thinking_count,
        task_prompt,
        messages,
        tool_calls,
        tool_breakdown,
        files,
        files_in_context_count,
        files_edited_count,
        files_read_count,
        model_usage,
        environment,
        focus_chain,
        has_focus_chain,
        api_history_size_bytes: api_size,
        ui_messages_size_bytes: ui_size,
    })
}

/// Build a mapping from api_conversation_history index → ISO 8601 timestamp.
///
/// Uses `ui_messages.json` where `conversationHistoryIndex` links each UI message
/// back to the api_conversation_history array position.
fn build_timestamp_map(ui_messages_path: &Path) -> HashMap<i64, String> {
    let mut map = HashMap::new();

    let content = match std::fs::read_to_string(ui_messages_path) {
        Ok(c) => c,
        Err(_) => return map,
    };

    let messages: Vec<RawUiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return map,
    };

    for msg in &messages {
        if let Some(idx) = msg.conversation_history_index {
            if idx >= 0 {
                // Use the first (earliest) timestamp for each conversation index
                map.entry(idx).or_insert_with(|| epoch_ms_to_iso(msg.ts));
            }
        }
    }

    map
}

/// Parse api_conversation_history.json into full detail structures.
///
/// Returns: (messages, tool_calls, tool_breakdown, message_count, tool_use_count, thinking_count, task_prompt)
fn parse_api_history_detail(
    path: &Path,
    timestamp_map: &HashMap<i64, String>,
) -> (
    Vec<ConversationMessage>,
    Vec<ToolCallDetail>,
    HashMap<String, usize>,
    usize,
    usize,
    usize,
    Option<String>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", path, e);
            return (vec![], vec![], HashMap::new(), 0, 0, 0, None);
        }
    };

    let raw_messages: Vec<RawApiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", path, e);
            return (vec![], vec![], HashMap::new(), 0, 0, 0, None);
        }
    };

    let message_count = raw_messages.len();
    let mut tool_use_count = 0usize;
    let mut thinking_count = 0usize;
    let mut tool_breakdown: HashMap<String, usize> = HashMap::new();
    let mut task_prompt: Option<String> = None;
    let mut messages: Vec<ConversationMessage> = Vec::with_capacity(message_count);
    let mut tool_calls: Vec<ToolCallDetail> = Vec::new();

    // First pass: collect all tool_use IDs and their message indices so we can
    // look up tool_results by tool_use_id in the second pass.
    // Actually, we'll do it in one pass by collecting pending tool_use entries
    // and resolving results as we encounter tool_result blocks.
    let mut pending_tool_results: HashMap<String, usize> = HashMap::new(); // tool_use_id → tool_calls index

    for (msg_idx, raw_msg) in raw_messages.iter().enumerate() {
        // Extract task prompt from first user message
        if task_prompt.is_none() && raw_msg.role == "user" {
            for block in &raw_msg.content {
                if let RawContentBlock::Text { text } = block {
                    task_prompt = Some(truncate_utf8(text, PROMPT_TRUNCATE_LEN));
                    break;
                }
            }
        }

        // Build content block summaries
        let mut content_blocks: Vec<ContentBlockSummary> = Vec::new();

        for block in &raw_msg.content {
            match block {
                RawContentBlock::Text { text } => {
                    let full_len = text.chars().count();
                    content_blocks.push(ContentBlockSummary {
                        block_type: "text".to_string(),
                        text: Some(truncate_utf8(text, TEXT_TRUNCATE_LEN)),
                        full_text_length: Some(full_len),
                        tool_use_id: None,
                        tool_name: None,
                        tool_input: None,
                        tool_result_text: None,
                    });
                }
                RawContentBlock::Thinking { thinking } => {
                    thinking_count += 1;
                    let full_len = thinking.chars().count();
                    content_blocks.push(ContentBlockSummary {
                        block_type: "thinking".to_string(),
                        text: Some(truncate_utf8(thinking, TEXT_TRUNCATE_LEN)),
                        full_text_length: Some(full_len),
                        tool_use_id: None,
                        tool_name: None,
                        tool_input: None,
                        tool_result_text: None,
                    });
                }
                RawContentBlock::ToolUse { id, name, input } => {
                    tool_use_count += 1;
                    *tool_breakdown.entry(name.clone()).or_insert(0) += 1;

                    let input_json = serde_json::to_string(input).unwrap_or_default();
                    let input_full_length = input_json.chars().count();
                    let input_summary = truncate_utf8(&input_json, TOOL_INPUT_TRUNCATE_LEN);

                    content_blocks.push(ContentBlockSummary {
                        block_type: "tool_use".to_string(),
                        text: None,
                        full_text_length: None,
                        tool_use_id: Some(id.clone()),
                        tool_name: Some(name.clone()),
                        tool_input: Some(input_summary.clone()),
                        tool_result_text: None,
                    });

                    // Create a ToolCallDetail entry
                    let call_idx = tool_calls.len();
                    tool_calls.push(ToolCallDetail {
                        call_index: call_idx,
                        message_index: msg_idx,
                        tool_name: name.clone(),
                        tool_use_id: id.clone(),
                        input_summary,
                        input_full_length,
                        result_summary: None,
                        result_full_length: None,
                    });

                    // Track pending result
                    pending_tool_results.insert(id.clone(), call_idx);
                }
                RawContentBlock::ToolResult { tool_use_id, content: result_content, .. } => {
                    let result_text = extract_tool_result_text(result_content);
                    let result_full_length = result_text.chars().count();
                    let result_summary = truncate_utf8(&result_text, TOOL_RESULT_TRUNCATE_LEN);

                    content_blocks.push(ContentBlockSummary {
                        block_type: "tool_result".to_string(),
                        text: None,
                        full_text_length: None,
                        tool_use_id: Some(tool_use_id.clone()),
                        tool_name: None,
                        tool_input: None,
                        tool_result_text: Some(result_summary.clone()),
                    });

                    // Resolve the pending tool call
                    if let Some(&call_idx) = pending_tool_results.get(tool_use_id) {
                        if let Some(call) = tool_calls.get_mut(call_idx) {
                            call.result_summary = Some(result_summary);
                            call.result_full_length = Some(result_full_length);
                        }
                    }
                }
                RawContentBlock::Unknown => {
                    content_blocks.push(ContentBlockSummary {
                        block_type: "unknown".to_string(),
                        text: None,
                        full_text_length: None,
                        tool_use_id: None,
                        tool_name: None,
                        tool_input: None,
                        tool_result_text: None,
                    });
                }
            }
        }

        // Look up timestamp from ui_messages join
        let timestamp = timestamp_map.get(&(msg_idx as i64)).cloned();

        messages.push(ConversationMessage {
            index: msg_idx,
            role: raw_msg.role.clone(),
            timestamp,
            content: content_blocks,
        });
    }

    (messages, tool_calls, tool_breakdown, message_count, tool_use_count, thinking_count, task_prompt)
}

/// Extract readable text from a tool_result content value.
///
/// tool_result.content can be:
/// - A JSON array of `[{type: "text", text: "..."}]`
/// - A plain string
/// - null / other
fn extract_tool_result_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::Array(arr) => {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    parts.push(text.to_string());
                }
            }
            parts.join("\n")
        }
        serde_json::Value::String(s) => s.clone(),
        _ => content.to_string(),
    }
}

// ============================================================================
// P1.5: Paginated messages parsing
// ============================================================================

/// Parse a task's messages with pagination support.
///
/// This is optimized for the `/messages` endpoint — it only parses
/// `api_conversation_history.json` + `ui_messages.json` (for timestamps),
/// applies optional role filtering, then returns a page of messages.
///
/// Returns None if the task directory doesn't exist or has no api_conversation_history.
pub fn parse_task_messages(
    task_id: &str,
    offset: usize,
    limit: usize,
    role_filter: Option<&str>,
) -> Option<PaginatedMessagesResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let api_history_path = dir.join("api_conversation_history.json");
    let ui_messages_path = dir.join("ui_messages.json");

    if !api_history_path.exists() {
        log::warn!("No api_conversation_history.json for task {}", task_id);
        return None;
    }

    // Build timestamp map from ui_messages
    let timestamp_map = build_timestamp_map(&ui_messages_path);

    // Parse api_conversation_history.json
    let content = match std::fs::read_to_string(&api_history_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", api_history_path, e);
            return None;
        }
    };

    let raw_messages: Vec<RawApiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", api_history_path, e);
            return None;
        }
    };

    let total_messages = raw_messages.len();

    // Build all ConversationMessage entries (with content block summaries)
    let all_messages: Vec<ConversationMessage> = raw_messages
        .iter()
        .enumerate()
        .map(|(idx, raw_msg)| {
            let content_blocks = raw_msg
                .content
                .iter()
                .map(|block| match block {
                    RawContentBlock::Text { text } => {
                        let full_len = text.chars().count();
                        ContentBlockSummary {
                            block_type: "text".to_string(),
                            text: Some(truncate_utf8(text, TEXT_TRUNCATE_LEN)),
                            full_text_length: Some(full_len),
                            tool_use_id: None,
                            tool_name: None,
                            tool_input: None,
                            tool_result_text: None,
                        }
                    }
                    RawContentBlock::Thinking { thinking } => {
                        let full_len = thinking.chars().count();
                        ContentBlockSummary {
                            block_type: "thinking".to_string(),
                            text: Some(truncate_utf8(thinking, TEXT_TRUNCATE_LEN)),
                            full_text_length: Some(full_len),
                            tool_use_id: None,
                            tool_name: None,
                            tool_input: None,
                            tool_result_text: None,
                        }
                    }
                    RawContentBlock::ToolUse { id, name, input } => {
                        let input_json = serde_json::to_string(input).unwrap_or_default();
                        ContentBlockSummary {
                            block_type: "tool_use".to_string(),
                            text: None,
                            full_text_length: None,
                            tool_use_id: Some(id.clone()),
                            tool_name: Some(name.clone()),
                            tool_input: Some(truncate_utf8(&input_json, TOOL_INPUT_TRUNCATE_LEN)),
                            tool_result_text: None,
                        }
                    }
                    RawContentBlock::ToolResult { tool_use_id, content: result_content, .. } => {
                        let result_text = extract_tool_result_text(result_content);
                        ContentBlockSummary {
                            block_type: "tool_result".to_string(),
                            text: None,
                            full_text_length: None,
                            tool_use_id: Some(tool_use_id.clone()),
                            tool_name: None,
                            tool_input: None,
                            tool_result_text: Some(truncate_utf8(&result_text, TOOL_RESULT_TRUNCATE_LEN)),
                        }
                    }
                    RawContentBlock::Unknown => ContentBlockSummary {
                        block_type: "unknown".to_string(),
                        text: None,
                        full_text_length: None,
                        tool_use_id: None,
                        tool_name: None,
                        tool_input: None,
                        tool_result_text: None,
                    },
                })
                .collect();

            let timestamp = timestamp_map.get(&(idx as i64)).cloned();

            ConversationMessage {
                index: idx,
                role: raw_msg.role.clone(),
                timestamp,
                content: content_blocks,
            }
        })
        .collect();

    // Apply role filter
    let filtered: Vec<ConversationMessage> = if let Some(role) = role_filter {
        all_messages
            .into_iter()
            .filter(|m| m.role == role)
            .collect()
    } else {
        all_messages
    };

    let filtered_count = filtered.len();

    // Apply pagination
    let page: Vec<ConversationMessage> = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    let has_more = offset + page.len() < filtered_count;

    Some(PaginatedMessagesResponse {
        task_id: task_id.to_string(),
        total_messages,
        filtered_count,
        offset,
        limit,
        has_more,
        messages: page,
    })
}

// ============================================================================
// P1.6: Single message parsing (full untruncated content)
// ============================================================================

/// Parse a single message by index with FULL untruncated content.
///
/// This is for the `/messages/:index` endpoint — returns one message with
/// complete text, thinking, tool input, and tool result content.
///
/// Returns None if the task directory doesn't exist, has no api_conversation_history,
/// or the index is out of bounds.
pub fn parse_single_message(task_id: &str, index: usize) -> Option<FullMessageResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let api_history_path = dir.join("api_conversation_history.json");
    let ui_messages_path = dir.join("ui_messages.json");

    if !api_history_path.exists() {
        log::warn!("No api_conversation_history.json for task {}", task_id);
        return None;
    }

    // Parse api_conversation_history.json
    let content = match std::fs::read_to_string(&api_history_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", api_history_path, e);
            return None;
        }
    };

    let raw_messages: Vec<RawApiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", api_history_path, e);
            return None;
        }
    };

    let total_messages = raw_messages.len();

    // Bounds check
    if index >= total_messages {
        log::warn!("Message index {} out of bounds (total: {})", index, total_messages);
        return None;
    }

    // Build timestamp map
    let timestamp_map = build_timestamp_map(&ui_messages_path);
    let timestamp = timestamp_map.get(&(index as i64)).cloned();

    let raw_msg = &raw_messages[index];

    // Build full content blocks (NO truncation)
    let content_blocks: Vec<FullContentBlock> = raw_msg
        .content
        .iter()
        .map(|block| match block {
            RawContentBlock::Text { text } => {
                let text_length = text.chars().count();
                FullContentBlock {
                    block_type: "text".to_string(),
                    text: Some(text.clone()),
                    text_length: Some(text_length),
                    tool_use_id: None,
                    tool_name: None,
                    tool_input: None,
                    tool_input_length: None,
                    tool_result_text: None,
                    tool_result_length: None,
                }
            }
            RawContentBlock::Thinking { thinking } => {
                let text_length = thinking.chars().count();
                FullContentBlock {
                    block_type: "thinking".to_string(),
                    text: Some(thinking.clone()),
                    text_length: Some(text_length),
                    tool_use_id: None,
                    tool_name: None,
                    tool_input: None,
                    tool_input_length: None,
                    tool_result_text: None,
                    tool_result_length: None,
                }
            }
            RawContentBlock::ToolUse { id, name, input } => {
                let input_json = serde_json::to_string_pretty(input).unwrap_or_default();
                let input_length = input_json.chars().count();
                FullContentBlock {
                    block_type: "tool_use".to_string(),
                    text: None,
                    text_length: None,
                    tool_use_id: Some(id.clone()),
                    tool_name: Some(name.clone()),
                    tool_input: Some(input_json),
                    tool_input_length: Some(input_length),
                    tool_result_text: None,
                    tool_result_length: None,
                }
            }
            RawContentBlock::ToolResult { tool_use_id, content: result_content, .. } => {
                let result_text = extract_tool_result_text(result_content);
                let result_length = result_text.chars().count();
                FullContentBlock {
                    block_type: "tool_result".to_string(),
                    text: None,
                    text_length: None,
                    tool_use_id: Some(tool_use_id.clone()),
                    tool_name: None,
                    tool_input: None,
                    tool_input_length: None,
                    tool_result_text: Some(result_text),
                    tool_result_length: Some(result_length),
                }
            }
            RawContentBlock::Unknown => FullContentBlock {
                block_type: "unknown".to_string(),
                text: None,
                text_length: None,
                tool_use_id: None,
                tool_name: None,
                tool_input: None,
                tool_input_length: None,
                tool_result_text: None,
                tool_result_length: None,
            },
        })
        .collect();

    Some(FullMessageResponse {
        task_id: task_id.to_string(),
        index,
        total_messages,
        role: raw_msg.role.clone(),
        timestamp,
        content: content_blocks,
    })
}

/// Parse task_metadata.json into full detail structures.
///
/// Returns: (files, files_count, files_edited, files_read, model_usage, environment)
fn parse_task_metadata_detail(
    path: &Path,
) -> (
    Vec<FileInContextDetail>,
    usize,
    usize,
    usize,
    Vec<ModelUsageDetail>,
    Vec<EnvironmentDetail>,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (vec![], 0, 0, 0, vec![], vec![]),
    };

    let metadata: RawTaskMetadata = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse task_metadata {:?}: {}", path, e);
            return (vec![], 0, 0, 0, vec![], vec![]);
        }
    };

    // Files in context
    let files: Vec<FileInContextDetail> = metadata
        .files_in_context
        .iter()
        .map(|f| FileInContextDetail {
            path: f.path.clone(),
            record_state: f.record_state.clone(),
            record_source: f.record_source.clone(),
            cline_read_date: f.cline_read_date.map(epoch_ms_to_iso),
            cline_edit_date: f.cline_edit_date.map(epoch_ms_to_iso),
            user_edit_date: f.user_edit_date.map(epoch_ms_to_iso),
        })
        .collect();

    let files_in_context_count = files.len();
    let files_edited_count = metadata
        .files_in_context
        .iter()
        .filter(|f| f.record_source.as_deref() == Some("cline_edited"))
        .count();
    let files_read_count = metadata
        .files_in_context
        .iter()
        .filter(|f| f.record_source.as_deref() == Some("read_tool"))
        .count();

    // Model usage
    let model_usage: Vec<ModelUsageDetail> = metadata
        .model_usage
        .iter()
        .map(|m| ModelUsageDetail {
            timestamp: m.ts.map(epoch_ms_to_iso),
            model_id: m.model_id.clone(),
            model_provider_id: m.model_provider_id.clone(),
            mode: m.mode.clone(),
        })
        .collect();

    // Environment
    let environment: Vec<EnvironmentDetail> = metadata
        .environment_history
        .iter()
        .map(|e| EnvironmentDetail {
            timestamp: e.ts.map(epoch_ms_to_iso),
            os_name: e.os_name.clone(),
            os_version: e.os_version.clone(),
            host_name: e.host_name.clone(),
            host_version: e.host_version.clone(),
            cline_version: e.cline_version.clone(),
        })
        .collect();

    (files, files_in_context_count, files_edited_count, files_read_count, model_usage, environment)
}

// ============================================================================
// Tool call timeline parsing (GET /history/tasks/:taskId/tools)
// ============================================================================

/// Parse a task's tool call timeline — extracts tool_use + tool_result blocks,
/// pairs them by tool_use_id, and determines success/fail from is_error.
///
/// This is a focused parser for the `/tools` endpoint. It reads:
/// - `api_conversation_history.json` — tool_use (assistant) + tool_result (user) blocks
/// - `ui_messages.json` — timestamps (joined by conversationHistoryIndex)
///
/// Returns None if the task directory doesn't exist or has no api_conversation_history.
pub fn parse_task_tools(
    task_id: &str,
    tool_name_filter: Option<&str>,
    failed_only: bool,
) -> Option<ToolCallTimelineResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let api_history_path = dir.join("api_conversation_history.json");
    let ui_messages_path = dir.join("ui_messages.json");

    if !api_history_path.exists() {
        log::warn!("No api_conversation_history.json for task {}", task_id);
        return None;
    }

    // Build timestamp map from ui_messages
    let timestamp_map = build_timestamp_map(&ui_messages_path);

    // Parse api_conversation_history.json
    let content = match std::fs::read_to_string(&api_history_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", api_history_path, e);
            return None;
        }
    };

    let raw_messages: Vec<RawApiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", api_history_path, e);
            return None;
        }
    };

    // ---- Pass 1: Collect all tool_use entries ----
    // We store: (call_index, message_index, tool_name, tool_use_id, input_summary, input_full_length, timestamp)
    let mut tool_calls: Vec<ToolCallTimelineEntry> = Vec::new();
    // Map: tool_use_id → index into tool_calls vec (for resolving results)
    let mut pending: HashMap<String, usize> = HashMap::new();
    let mut tool_breakdown: HashMap<String, usize> = HashMap::new();
    let mut call_counter: usize = 0;

    for (msg_idx, raw_msg) in raw_messages.iter().enumerate() {
        let timestamp = timestamp_map.get(&(msg_idx as i64)).cloned();

        for block in &raw_msg.content {
            match block {
                RawContentBlock::ToolUse { id, name, input } => {
                    *tool_breakdown.entry(name.clone()).or_insert(0) += 1;

                    let input_json = serde_json::to_string(input).unwrap_or_default();
                    let input_full_length = input_json.chars().count();
                    let input_summary = truncate_utf8(&input_json, TOOL_INPUT_TRUNCATE_LEN);

                    let entry = ToolCallTimelineEntry {
                        call_index: call_counter,
                        message_index: msg_idx,
                        result_message_index: None,
                        timestamp: timestamp.clone(),
                        tool_name: name.clone(),
                        tool_use_id: id.clone(),
                        input_summary,
                        input_full_length,
                        result_summary: None,
                        result_full_length: None,
                        success: None, // will be resolved when we find tool_result
                        error_text: None,
                    };

                    let idx = tool_calls.len();
                    tool_calls.push(entry);
                    pending.insert(id.clone(), idx);
                    call_counter += 1;
                }
                RawContentBlock::ToolResult { tool_use_id, content: result_content, is_error } => {
                    let result_text = extract_tool_result_text(result_content);
                    let result_full_length = result_text.chars().count();
                    let result_summary = truncate_utf8(&result_text, TOOL_RESULT_TRUNCATE_LEN);

                    let is_err = is_error.unwrap_or(false);

                    // Resolve the pending tool call
                    if let Some(&call_idx) = pending.get(tool_use_id) {
                        if let Some(entry) = tool_calls.get_mut(call_idx) {
                            entry.result_message_index = Some(msg_idx);
                            entry.result_summary = Some(result_summary);
                            entry.result_full_length = Some(result_full_length);
                            entry.success = Some(!is_err);

                            if is_err {
                                entry.error_text = Some(truncate_utf8(&result_text, ERROR_TEXT_TRUNCATE_LEN));
                            }
                        }
                    }
                }
                _ => {} // skip text, thinking, unknown
            }
        }
    }

    // ---- Compute stats before filtering ----
    let total_tool_calls = tool_calls.len();
    let success_count = tool_calls.iter().filter(|c| c.success == Some(true)).count();
    let failure_count = tool_calls.iter().filter(|c| c.success == Some(false)).count();
    let no_result_count = tool_calls.iter().filter(|c| c.success.is_none()).count();

    // ---- Apply filters ----
    let filtered: Vec<ToolCallTimelineEntry> = tool_calls
        .into_iter()
        .filter(|entry| {
            // Tool name filter (partial match, case-insensitive)
            if let Some(filter) = tool_name_filter {
                let filter_lower = filter.to_lowercase();
                if !entry.tool_name.to_lowercase().contains(&filter_lower) {
                    return false;
                }
            }
            // Failed-only filter
            if failed_only && entry.success != Some(false) {
                return false;
            }
            true
        })
        .collect();

    let filtered_count = filtered.len();

    Some(ToolCallTimelineResponse {
        task_id: task_id.to_string(),
        total_tool_calls,
        filtered_count,
        success_count,
        failure_count,
        no_result_count,
        tool_breakdown,
        tool_calls: filtered,
    })
}

// ============================================================================
// Thinking blocks parsing (GET /history/tasks/:taskId/thinking)
// ============================================================================

/// Parse a task's thinking blocks — extracts all thinking blocks from assistant messages.
///
/// This is a focused parser for the `/thinking` endpoint. It reads:
/// - `api_conversation_history.json` — thinking blocks (assistant messages only)
/// - `ui_messages.json` — timestamps (joined by conversationHistoryIndex)
///
/// Supports optional truncation via `max_length` and filtering via `min_length`.
///
/// Returns None if the task directory doesn't exist or has no api_conversation_history.
pub fn parse_task_thinking(
    task_id: &str,
    max_length: Option<usize>,
    min_length: Option<usize>,
) -> Option<ThinkingBlocksResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let api_history_path = dir.join("api_conversation_history.json");
    let ui_messages_path = dir.join("ui_messages.json");

    if !api_history_path.exists() {
        log::warn!("No api_conversation_history.json for task {}", task_id);
        return None;
    }

    // Build timestamp map from ui_messages
    let timestamp_map = build_timestamp_map(&ui_messages_path);

    // Parse api_conversation_history.json
    let content = match std::fs::read_to_string(&api_history_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", api_history_path, e);
            return None;
        }
    };

    let raw_messages: Vec<RawApiMessage> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", api_history_path, e);
            return None;
        }
    };

    // Default truncation: 1000 chars (0 means no truncation)
    let max_len = max_length.unwrap_or(1000);
    let apply_truncation = max_len > 0;

    // ---- Extract all thinking blocks ----
    let mut thinking_blocks: Vec<ThinkingBlockEntry> = Vec::new();
    let mut block_counter: usize = 0;
    let mut total_characters: usize = 0;

    for (msg_idx, raw_msg) in raw_messages.iter().enumerate() {
        // Only consider assistant messages (thinking blocks only appear in assistant messages)
        if raw_msg.role != "assistant" {
            continue;
        }

        let timestamp = timestamp_map.get(&(msg_idx as i64)).cloned();

        for block in &raw_msg.content {
            if let RawContentBlock::Thinking { thinking } = block {
                let full_length = thinking.chars().count();
                total_characters += full_length;

                // Apply min_length filter if specified
                if let Some(min_len) = min_length {
                    if full_length < min_len {
                        continue; // Skip this block
                    }
                }

                let (thinking_text, is_truncated) = if apply_truncation && full_length > max_len {
                    (truncate_utf8(thinking, max_len), true)
                } else {
                    (thinking.clone(), false)
                };

                thinking_blocks.push(ThinkingBlockEntry {
                    block_index: block_counter,
                    message_index: msg_idx,
                    timestamp: timestamp.clone(),
                    thinking: thinking_text,
                    full_length,
                    is_truncated,
                });

                block_counter += 1;
            }
        }
    }

    let total_thinking_blocks = thinking_blocks.len();
    let avg_block_length = if total_thinking_blocks > 0 {
        total_characters / total_thinking_blocks
    } else {
        0
    };

    Some(ThinkingBlocksResponse {
        task_id: task_id.to_string(),
        total_thinking_blocks,
        total_characters,
        avg_block_length,
        thinking_blocks,
    })
}

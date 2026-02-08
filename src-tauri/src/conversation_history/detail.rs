//! Full task detail parsing.
//!
//! Contains:
//! - Heavy parsing across all task files
//! - Timestamp join logic
//! - Focus chain loading
//!
//! This module is agent-cold and can be larger.

use std::collections::HashMap;
use std::path::Path;

use super::root::tasks_root;
use super::summary::parse_ui_messages_end_time;
use super::types::*;
use super::util::{
    epoch_ms_to_iso, truncate_utf8, PROMPT_TRUNCATE_LEN, TEXT_TRUNCATE_LEN,
    TOOL_INPUT_TRUNCATE_LEN, TOOL_RESULT_TRUNCATE_LEN,
};

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
    let api_size = std::fs::metadata(&api_history_path)
        .map(|m| m.len())
        .unwrap_or(0);
    let ui_size = std::fs::metadata(&ui_messages_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // Start time from task_id
    let started_at = match task_id.parse::<u64>() {
        Ok(epoch_ms) => epoch_ms_to_iso(epoch_ms),
        Err(_) => "unknown".to_string(),
    };

    // ---- Parse ui_messages.json for timestamp mapping ----
    let timestamp_map = build_timestamp_map(&ui_messages_path);
    let ended_at = parse_ui_messages_end_time(&ui_messages_path);

    // ---- Parse api_conversation_history.json (full detail) ----
    let (
        messages,
        tool_calls,
        tool_breakdown,
        message_count,
        tool_use_count,
        thinking_count,
        task_prompt,
    ) = parse_api_history_detail(&api_history_path, &timestamp_map);

    // ---- Parse task_metadata.json (full detail) ----
    let (
        files,
        files_in_context_count,
        files_edited_count,
        files_read_count,
        model_usage,
        environment,
    ) = parse_task_metadata_detail(&metadata_path);

    // ---- Read focus chain ----
    let has_focus_chain = focus_chain_path.exists();
    let focus_chain = if has_focus_chain {
        std::fs::read_to_string(&focus_chain_path).ok()
    } else {
        None
    };

    // Full local path to the task directory
    let task_dir_path = dir.to_string_lossy().to_string();

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
        task_dir_path,
    })
}

/// Build a mapping from api_conversation_history index → ISO 8601 timestamp.
///
/// Uses `ui_messages.json` where `conversationHistoryIndex` links each UI message
/// back to the api_conversation_history array position.
pub(crate) fn build_timestamp_map(ui_messages_path: &Path) -> HashMap<i64, String> {
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
                map.entry(idx)
                    .or_insert_with(|| epoch_ms_to_iso(msg.ts));
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
                RawContentBlock::ToolResult {
                    tool_use_id,
                    content: result_content,
                    ..
                } => {
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

    (
        messages,
        tool_calls,
        tool_breakdown,
        message_count,
        tool_use_count,
        thinking_count,
        task_prompt,
    )
}

/// Extract readable text from a tool_result content value.
///
/// tool_result.content can be:
/// - A JSON array of `[{type: "text", text: "..."}]`
/// - A plain string
/// - null / other
pub(crate) fn extract_tool_result_text(content: &serde_json::Value) -> String {
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

/// Parse task_metadata.json into full detail structures.
///
/// Returns: (files, files_count, files_edited, files_read, model_usage, environment)
pub(crate) fn parse_task_metadata_detail(
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

    (
        files,
        files_in_context_count,
        files_edited_count,
        files_read_count,
        model_usage,
        environment,
    )
}

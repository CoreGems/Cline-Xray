//! Tool call timeline parsing.
//!
//! Contains:
//! - Tool call timeline parsing
//! - Tool/result pairing
//! - Success/failure derivation
//! - Tool-specific filtering
//!
//! Must be isolated from message pagination logic.

use std::collections::HashMap;

use super::detail::{build_timestamp_map, extract_tool_result_text};
use super::root::tasks_root;
use super::types::*;
use super::util::{truncate_utf8, ERROR_TEXT_TRUNCATE_LEN, TOOL_INPUT_TRUNCATE_LEN, TOOL_RESULT_TRUNCATE_LEN};

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

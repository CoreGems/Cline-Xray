//! Paginated and single-message parsing.
//!
//! Contains:
//! - Paginated message parsing
//! - Single-message full parsing
//! - Message-level filtering
//!
//! Must not contain directory scanning or aggregation.

use super::detail::{build_timestamp_map, extract_tool_result_text};
use super::root::tasks_root;
use super::types::*;
use super::util::{truncate_utf8, TEXT_TRUNCATE_LEN, TOOL_INPUT_TRUNCATE_LEN, TOOL_RESULT_TRUNCATE_LEN};

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

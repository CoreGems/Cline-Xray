//! Thinking block parsing.
//!
//! Contains:
//! - Thinking block extraction
//! - Length filtering
//! - Truncation controls
//! - Statistics (counts, averages)
//!
//! Must not include tool or message pagination logic.

use super::detail::build_timestamp_map;
use super::root::tasks_root;
use super::types::*;
use super::util::truncate_utf8;

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

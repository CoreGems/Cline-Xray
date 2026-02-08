//! Subtask detection from ui_messages.json.
//!
//! Parses `ui_messages.json` to detect task/feedback boundaries,
//! then cross-references `api_conversation_history.json` to compute
//! message ranges and tool call counts per subtask.
//!
//! Detection strategy:
//! - `say = "task"` → initial task prompt (subtask #0)
//! - `say = "user_feedback"` → feedback subtask (#1, #2, …)
//!
//! See SUBTASK_FI.md for full design rationale.

use std::collections::HashSet;

use super::root::tasks_root;
use super::types::*;
use super::util::epoch_ms_to_iso;

/// Internal marker extracted from ui_messages.json
struct SubtaskMarker {
    prompt: String,
    ts: u64,
    conversation_history_index: i64,
    is_initial: bool,
}

/// Parse subtasks for a single task.
///
/// Reads `ui_messages.json` to find task/feedback markers, then reads
/// `api_conversation_history.json` to compute tool usage per subtask range.
///
/// Returns None if the task directory doesn't exist or has no ui_messages.
pub fn parse_task_subtasks(task_id: &str) -> Option<SubtasksResponse> {
    let root = tasks_root()?;
    let dir = root.join(task_id);

    if !dir.is_dir() {
        log::warn!("Task directory not found: {:?}", dir);
        return None;
    }

    let ui_messages_path = dir.join("ui_messages.json");
    let api_history_path = dir.join("api_conversation_history.json");

    if !ui_messages_path.exists() {
        log::warn!("No ui_messages.json for task {}", task_id);
        return None;
    }

    // ---- Step 1: Parse ui_messages.json for subtask markers ----
    let ui_content = match std::fs::read_to_string(&ui_messages_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to read {:?}: {}", ui_messages_path, e);
            return None;
        }
    };

    let ui_messages: Vec<RawUiMessage> = match serde_json::from_str(&ui_content) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("Failed to parse {:?}: {}", ui_messages_path, e);
            return None;
        }
    };

    let markers = extract_subtask_markers(&ui_messages);

    if markers.is_empty() {
        // No task or feedback entries found — return single empty subtask
        return Some(SubtasksResponse {
            task_id: task_id.to_string(),
            total_subtasks: 0,
            has_subtasks: false,
            subtasks: vec![],
        });
    }

    // ---- Step 2: Parse api_conversation_history.json for tool counting ----
    let api_messages: Vec<RawApiMessage> = if api_history_path.exists() {
        match std::fs::read_to_string(&api_history_path) {
            Ok(c) => serde_json::from_str(&c).unwrap_or_default(),
            Err(_) => vec![],
        }
    } else {
        vec![]
    };

    let total_api_msgs = api_messages.len();

    // ---- Step 3: Build subtask entries with message ranges ----
    let mut subtasks: Vec<SubtaskEntry> = Vec::with_capacity(markers.len());

    for (i, marker) in markers.iter().enumerate() {
        // Compute range_start
        let range_start = if marker.is_initial {
            0usize
        } else {
            // Feedback appears after conversationHistoryIndex — next subtask starts at idx + 1
            let idx = marker.conversation_history_index;
            if idx < 0 { 0 } else { (idx as usize) + 1 }
        };

        // Compute range_end (inclusive)
        let range_end = if i + 1 < markers.len() {
            // Next marker's conversationHistoryIndex is the last message of THIS subtask
            let next_idx = markers[i + 1].conversation_history_index;
            if next_idx < 0 {
                // Shouldn't happen for non-initial markers, but handle gracefully
                if total_api_msgs > 0 { Some(total_api_msgs - 1) } else { None }
            } else {
                Some(next_idx as usize)
            }
        } else {
            // Last subtask extends to end of conversation
            if total_api_msgs > 0 { Some(total_api_msgs - 1) } else { None }
        };

        // Count messages in range
        let message_count = match range_end {
            Some(end) if end >= range_start => end - range_start + 1,
            _ => 0,
        };

        // Count tool calls in range
        let (tool_call_count, tools_used) = count_tools_in_range(
            &api_messages,
            range_start,
            range_end.unwrap_or(0),
        );

        subtasks.push(SubtaskEntry {
            subtask_index: i,
            prompt: marker.prompt.clone(),
            timestamp: epoch_ms_to_iso(marker.ts),
            is_initial_task: marker.is_initial,
            message_range_start: range_start,
            message_range_end: range_end,
            message_count,
            tool_call_count,
            tools_used,
        });
    }

    let total_subtasks = subtasks.len();
    let has_subtasks = total_subtasks > 1;

    Some(SubtasksResponse {
        task_id: task_id.to_string(),
        total_subtasks,
        has_subtasks,
        subtasks,
    })
}

/// Extract subtask markers from ui_messages.
///
/// Finds entries where `say = "task"` (initial task) or `say = "user_feedback"`
/// (feedback subtasks) and extracts their prompt text, timestamp, and
/// conversation history index.
fn extract_subtask_markers(ui_messages: &[RawUiMessage]) -> Vec<SubtaskMarker> {
    let mut markers = Vec::new();

    for msg in ui_messages {
        let say = match &msg.say {
            Some(s) => s.as_str(),
            None => continue,
        };

        match say {
            "task" => {
                let prompt = msg.text.clone().unwrap_or_default();
                markers.push(SubtaskMarker {
                    prompt,
                    ts: msg.ts,
                    conversation_history_index: msg.conversation_history_index.unwrap_or(-1),
                    is_initial: true,
                });
            }
            "user_feedback" => {
                let prompt = msg.text.clone().unwrap_or_default();
                // Skip empty feedback
                if prompt.trim().is_empty() {
                    continue;
                }
                markers.push(SubtaskMarker {
                    prompt,
                    ts: msg.ts,
                    conversation_history_index: msg.conversation_history_index.unwrap_or(-1),
                    is_initial: false,
                });
            }
            _ => {}
        }
    }

    markers
}

/// Count tool_use blocks within a message range of api_conversation_history.
///
/// Returns (total_tool_calls, deduplicated_tool_names).
fn count_tools_in_range(
    api_messages: &[RawApiMessage],
    range_start: usize,
    range_end: usize,
) -> (usize, Vec<String>) {
    let mut count = 0usize;
    let mut tool_names: HashSet<String> = HashSet::new();

    let end = range_end.min(api_messages.len().saturating_sub(1));

    for msg_idx in range_start..=end {
        if msg_idx >= api_messages.len() {
            break;
        }
        let msg = &api_messages[msg_idx];
        for block in &msg.content {
            if let RawContentBlock::ToolUse { name, .. } = block {
                count += 1;
                tool_names.insert(name.clone());
            }
        }
    }

    let mut names: Vec<String> = tool_names.into_iter().collect();
    names.sort();

    (count, names)
}

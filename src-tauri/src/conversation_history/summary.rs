//! Task list and summary-level parsing.
//!
//! Contains:
//! - Directory scanning
//! - Task summary parsing
//! - Lightweight parsing used by /history/tasks endpoint
//! - Aggregation logic
//!
//! This is a hot path module — keep it compact and focused.

use std::collections::HashMap;
use std::path::Path;

use super::root::tasks_root;
use super::types::*;
use super::util::{epoch_ms_to_iso, truncate_utf8, PROMPT_TRUNCATE_LEN};

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
pub(crate) fn parse_ui_messages_end_time(path: &Path) -> Option<String> {
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

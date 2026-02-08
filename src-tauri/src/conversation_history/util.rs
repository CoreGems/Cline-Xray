//! Shared utility helpers for conversation history parsing.
//!
//! Contains:
//! - Truncation constants
//! - UTF-8 safe string truncation
//! - Epoch → ISO timestamp conversion
//!
//! This module must contain no filesystem access and no parsing logic.

/// Maximum characters to keep for task_prompt truncation
pub const PROMPT_TRUNCATE_LEN: usize = 200;
/// Maximum characters for text/thinking blocks in detail view
pub const TEXT_TRUNCATE_LEN: usize = 500;
/// Maximum characters for tool input summary in detail view
pub const TOOL_INPUT_TRUNCATE_LEN: usize = 300;
/// Maximum characters for tool result summary in detail view
pub const TOOL_RESULT_TRUNCATE_LEN: usize = 200;
/// Maximum characters for error text in tool timeline
pub const ERROR_TEXT_TRUNCATE_LEN: usize = 300;

/// Safely truncate a UTF-8 string to at most `max_chars` characters.
/// Avoids panicking on multi-byte character boundaries (e.g. `—`, `…`, emoji).
pub fn truncate_utf8(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars).collect();
        format!("{}…", truncated)
    }
}

/// Convert epoch milliseconds to ISO 8601 string
pub fn epoch_ms_to_iso(epoch_ms: u64) -> String {
    let secs = (epoch_ms / 1000) as i64;
    let nanos = ((epoch_ms % 1000) * 1_000_000) as u32;
    match chrono::DateTime::from_timestamp(secs, nanos) {
        Some(dt) => dt.with_timezone(&chrono::Local).to_rfc3339(),
        None => format!("{}ms", epoch_ms),
    }
}

//! Shared validation helpers for conversation history handlers.
//!
//! Contains:
//! - Task ID validation
//! - Common error response helpers
//!
//! This module must remain small and focused.

use axum::http::StatusCode;
use axum::Json;

use crate::conversation_history::types::HistoryErrorResponse;

/// Validate that a task_id is a numeric epoch milliseconds value.
///
/// Returns `Ok(())` if valid, or an error tuple suitable for returning from handlers.
pub fn validate_task_id(task_id: &str) -> Result<(), (StatusCode, Json<HistoryErrorResponse>)> {
    if task_id.is_empty() || !task_id.chars().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(HistoryErrorResponse {
                error: format!(
                    "Invalid task_id '{}': must be a numeric epoch milliseconds value",
                    task_id
                ),
                code: 400,
            }),
        ));
    }
    Ok(())
}

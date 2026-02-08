//! Task root directory resolution for Cline task data.
//!
//! Contains:
//! - Environment-dependent path construction
//! - Task root directory detection
//!
//! This module must contain no parsing logic.

use std::path::PathBuf;

/// Return the Cline tasks root directory
///
/// Looks for: `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/`
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

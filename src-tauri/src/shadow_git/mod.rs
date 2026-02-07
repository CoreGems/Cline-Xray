//! Shadow Git — Discover Cline checkpoint repos and enumerate tasks/diffs
//!
//! Cline (VS Code extension) creates a hidden "shadow" Git repo under
//! `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/checkpoints/<workspace-id>/.git`
//!
//! This module discovers those repos, lists tasks, and produces diffs.

pub mod types;
pub mod discovery;
pub mod cache;
pub mod handlers;

pub use types::*;
pub use discovery::{list_tasks_for_workspace, list_steps_for_task};
pub use handlers::*;

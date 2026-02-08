//! Conversation History — Parse and expose Cline task conversation logs
//!
//! Cline (VS Code extension) stores task data under:
//! `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/<task-id>/`
//!
//! Each task directory contains:
//! - `api_conversation_history.json` — raw Anthropic API message log
//! - `ui_messages.json` — timestamped UI-level messages
//! - `task_metadata.json` — model, files, environment info
//! - `focus_chain_taskid_<id>.md` — task progress checklist
//!
//! This module parses those files and exposes them via REST API.

pub mod types;
pub mod parser;
pub mod cache;
pub mod handlers;

pub use types::*;
pub use handlers::*;

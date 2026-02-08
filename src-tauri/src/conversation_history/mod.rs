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
//!
//! ## Module Organization
//!
//! The parsing logic is split into focused modules by endpoint responsibility:
//!
//! - `util` — shared helpers (truncation, timestamp conversion)
//! - `root` — filesystem / task root resolution
//! - `summary` — task list & summary parsing (GET /history/tasks)
//! - `detail` — full task detail parsing (GET /history/tasks/:id)
//! - `messages` — paginated + single-message parsing (GET /history/tasks/:id/messages)
//! - `tools` — tool call timeline parsing (GET /history/tasks/:id/tools)
//! - `thinking` — thinking block parsing (GET /history/tasks/:id/thinking)
//! - `files` — files-in-context parsing (GET /history/tasks/:id/files)

pub mod types;
pub mod cache;
pub mod handlers;  // Now points to handlers/ directory with submodules

// Internal parsing modules (pub(crate) for handler access)
pub(crate) mod util;
pub(crate) mod root;
pub(crate) mod summary;
pub(crate) mod detail;
pub(crate) mod messages;
pub(crate) mod tools;
pub(crate) mod thinking;
pub(crate) mod files;
pub(crate) mod subtasks;

pub use types::*;
pub use handlers::*;

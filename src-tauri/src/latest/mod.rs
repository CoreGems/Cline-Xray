//! Latest — Composite endpoint merging conversation history + shadow git
//!
//! Provides `GET /latest` which auto-resolves the most recent Cline task,
//! finds its last subtask prompt, locates the checkpoint workspace, and
//! returns prompt + diff + changed files in a single response.
//!
//! Designed for both UI rendering and LLM/agent REST API tool-use.

pub mod types;
pub mod handler;

pub use types::*;
pub use handler::get_latest_handler;

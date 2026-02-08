//! Handler modules for conversation history API.
//!
//! This module re-exports all handler functions from their focused submodules.
//! Each submodule is responsible for one URL family.
//!
//! ## Module Structure
//!
//! - `common` — shared validation helpers (task_id validation)
//! - `index` — task list + cache (GET /history/tasks)
//! - `task_detail` — single task detail (GET /history/tasks/{task_id})
//! - `messages` — messages + expansion (GET /history/tasks/{task_id}/messages[/{index}])
//! - `tools` — tool timeline (GET /history/tasks/{task_id}/tools)
//! - `thinking` — thinking blocks (GET /history/tasks/{task_id}/thinking)
//! - `files` — files in context (GET /history/tasks/{task_id}/files)
//! - `stats` — aggregate stats across all tasks (GET /history/stats)

mod common;

// Public submodules - utoipa generates __path_* types that must be accessible
// from the handlers module for OpenAPI derive macro to find them
pub mod files;
pub mod index;
pub mod messages;
pub mod stats;
pub mod subtasks;
pub mod task_detail;
pub mod thinking;
pub mod tools;

// Re-export all handler functions for backward compatibility
pub use files::get_task_files_handler;
pub use index::list_history_tasks_handler;
pub use messages::{get_single_message_handler, get_task_messages_handler};
pub use stats::get_history_stats_handler;
pub use subtasks::get_task_subtasks_handler;
pub use task_detail::get_task_detail_handler;
pub use thinking::get_task_thinking_handler;
pub use tools::get_task_tools_handler;

// Re-export utoipa __path_* types for OpenAPI generation
pub use files::__path_get_task_files_handler;
pub use index::__path_list_history_tasks_handler;
pub use messages::{__path_get_single_message_handler, __path_get_task_messages_handler};
pub use stats::__path_get_history_stats_handler;
pub use subtasks::__path_get_task_subtasks_handler;
pub use task_detail::__path_get_task_detail_handler;
pub use thinking::__path_get_task_thinking_handler;
pub use tools::__path_get_task_tools_handler;

use serde::{Deserialize, Serialize};

/// A discovered checkpoint workspace (a directory under checkpoints/ with a .git)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    /// The workspace-id directory name (e.g. "4184916832")
    pub id: String,
    /// Absolute path to the .git or .git_disabled directory
    pub git_dir: String,
    /// Whether the git dir is .git (active) or .git_disabled (paused)
    pub active: bool,
    /// Number of tasks found in this workspace (distinct task-ids from commit subjects)
    pub task_count: usize,
    /// ISO 8601 timestamp of the most recent checkpoint commit in this workspace
    pub last_modified: String,
}

/// Response for GET /changes/workspaces
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacesResponse {
    /// List of discovered checkpoint workspaces
    pub workspaces: Vec<WorkspaceInfo>,
    /// The root path that was scanned
    pub checkpoints_root: String,
}

/// A task summary (group of checkpoint commits sharing a task-id)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClineTaskSummary {
    /// The task-id extracted from commit subjects
    pub task_id: String,
    /// The workspace-id this task belongs to
    pub workspace_id: String,
    /// Number of checkpoint commits (steps) in this task
    pub steps: usize,
    /// Number of distinct files changed across all steps
    pub files_changed: usize,
    /// ISO 8601 timestamp of the most recent checkpoint commit
    pub last_modified: String,
}

/// Response for GET /changes/tasks
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TasksResponse {
    /// The workspace-id these tasks belong to
    pub workspace_id: String,
    /// List of tasks
    pub tasks: Vec<ClineTaskSummary>,
}

/// A single checkpoint step (one commit in a task)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointStep {
    /// 40-char commit SHA
    pub hash: String,
    /// Commit subject line (e.g. "checkpoint-<wsId>-<taskId>")
    pub subject: String,
    /// ISO 8601 timestamp of the commit
    pub timestamp: String,
    /// Number of files changed in this step (vs parent commit)
    pub files_changed: usize,
    /// Step index (1-based, chronological order)
    pub index: usize,
}

/// Response for GET /changes/tasks/:taskId/steps
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StepsResponse {
    /// Task ID these steps belong to
    pub task_id: String,
    /// Workspace ID
    pub workspace_id: String,
    /// Ordered list of checkpoint steps (chronological, oldest first)
    pub steps: Vec<CheckpointStep>,
}

/// A file in a diff
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    /// File path relative to repo root
    pub path: String,
    /// Lines added
    pub lines_added: usize,
    /// Lines removed
    pub lines_removed: usize,
    /// File status
    pub status: String, // "added" | "modified" | "deleted" | "renamed"
}

/// Full diff result for a step or task
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    /// List of files changed
    pub files: Vec<DiffFile>,
    /// Unified diff patch text
    pub patch: String,
    /// The "from" commit reference
    pub from_ref: String,
    /// The "to" commit reference
    pub to_ref: String,
}

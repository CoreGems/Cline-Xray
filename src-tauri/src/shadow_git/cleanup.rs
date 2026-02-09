//! Workspace cleanup — nuke all checkpoint history by re-initializing the bare git repo.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of a nuke operation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NukeWorkspaceResponse {
    /// Workspace ID that was nuked
    pub workspace_id: String,
    /// Number of commits that were deleted
    pub deleted_commits: usize,
    /// Number of tasks that were deleted
    pub deleted_tasks: usize,
    /// The git command used to re-initialize the repo
    pub git_command: String,
    /// Whether the operation was successful
    pub success: bool,
}

/// Count the commits in a bare git repo before nuking it.
/// Returns (commit_count, task_count).
fn count_commits_and_tasks(git_dir: &str) -> (usize, usize) {
    let output = std::process::Command::new("git")
        .args(["--git-dir", git_dir, "log", "--all", "--pretty=format:%s"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            let commit_count = lines.len();

            // Count distinct task IDs from commit subjects like "checkpoint-<wsId>-<taskId>"
            let mut task_ids = std::collections::HashSet::new();
            for line in &lines {
                // Extract task ID from subject: "checkpoint-<wsId>-<taskId>"
                let parts: Vec<&str> = line.splitn(3, '-').collect();
                if parts.len() >= 3 {
                    task_ids.insert(parts[2].to_string());
                }
            }

            (commit_count, task_ids.len())
        }
        _ => (0, 0),
    }
}

/// Nuke a workspace's git history by deleting and re-initializing the bare repo.
///
/// # Safety checks
/// - Verifies the path ends with `.git` (not `.git_disabled` — Cline is active)
/// - Verifies the `.git` directory exists
///
/// # Steps
/// 1. Count existing commits/tasks (for the response)
/// 2. Delete the `.git` directory entirely
/// 3. Run `git init --bare <same path>` to recreate it empty
pub fn nuke_workspace(workspace_id: &str, git_dir: &str) -> Result<NukeWorkspaceResponse, String> {
    let git_path = Path::new(git_dir);

    // Safety: must be a .git directory (not .git_disabled)
    let dir_name = git_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if dir_name == ".git_disabled" {
        return Err(format!(
            "Cannot nuke workspace '{}': git dir is '.git_disabled' — Cline is actively running a task. \
             Wait for the task to finish before nuking.",
            workspace_id
        ));
    }

    if dir_name != ".git" {
        return Err(format!(
            "Cannot nuke workspace '{}': unexpected git dir name '{}' (expected '.git')",
            workspace_id, dir_name
        ));
    }

    if !git_path.exists() {
        return Err(format!(
            "Cannot nuke workspace '{}': git dir does not exist at '{}'",
            workspace_id, git_dir
        ));
    }

    // Count existing commits and tasks before nuking
    let (commit_count, task_count) = count_commits_and_tasks(git_dir);
    log::info!(
        "Nuke workspace '{}': found {} commits, {} tasks — deleting '{}'",
        workspace_id, commit_count, task_count, git_dir
    );

    // Step 1: Delete the .git directory entirely
    if let Err(e) = std::fs::remove_dir_all(git_path) {
        return Err(format!(
            "Failed to delete git dir '{}': {}",
            git_dir, e
        ));
    }
    log::info!("Nuke workspace '{}': deleted git dir", workspace_id);

    // Step 2: Re-initialize as bare repo
    let git_command = format!("git init --bare \"{}\"", git_dir);
    let init_result = std::process::Command::new("git")
        .args(["init", "--bare", git_dir])
        .output();

    match init_result {
        Ok(out) if out.status.success() => {
            log::info!(
                "Nuke workspace '{}': re-initialized bare repo — {} commits, {} tasks deleted",
                workspace_id, commit_count, task_count
            );
            Ok(NukeWorkspaceResponse {
                workspace_id: workspace_id.to_string(),
                deleted_commits: commit_count,
                deleted_tasks: task_count,
                git_command,
                success: true,
            })
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!(
                "git init --bare failed for '{}': {}",
                git_dir, stderr
            ))
        }
        Err(e) => Err(format!(
            "Failed to run git init --bare for '{}': {}",
            git_dir, e
        )),
    }
}

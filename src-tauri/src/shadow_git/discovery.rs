use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::types::{ClineTaskSummary, WorkspaceInfo};

/// Find the Cline globalStorage root directory.
/// On Windows: %APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev
pub fn cline_root() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let root = PathBuf::from(appdata)
        .join("Code")
        .join("User")
        .join("globalStorage")
        .join("saoudrizwan.claude-dev");
    if root.exists() {
        Some(root)
    } else {
        log::warn!("Cline root not found at {:?}", root);
        None
    }
}

/// Return the checkpoints root: <cline_root>/checkpoints
pub fn checkpoints_root() -> Option<PathBuf> {
    cline_root().map(|r| r.join("checkpoints"))
}

/// Discover all checkpoint repos (workspace dirs containing .git or .git_disabled).
/// For each workspace, count distinct task-ids by parsing commit subjects.
pub fn find_workspaces() -> Vec<WorkspaceInfo> {
    let cp_root = match checkpoints_root() {
        Some(r) if r.exists() => r,
        _ => {
            log::info!("Checkpoints root does not exist");
            return Vec::new();
        }
    };

    log::info!("Scanning checkpoints root: {:?}", cp_root);

    let mut workspaces = Vec::new();

    let entries = match std::fs::read_dir(&cp_root) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to read checkpoints dir: {}", e);
            return Vec::new();
        }
    };

    for entry in entries.flatten() {
        let ws_id = entry.file_name().to_string_lossy().to_string();
        let ws_path = entry.path();

        // Check for .git (active) or .git_disabled (paused)
        for (git_name, active) in &[(".git", true), (".git_disabled", false)] {
            let git_dir = ws_path.join(git_name);
            if git_dir.exists() {
                let (task_count, last_modified) = count_tasks_and_latest(&git_dir);
                workspaces.push(WorkspaceInfo {
                    id: ws_id.clone(),
                    git_dir: git_dir.to_string_lossy().to_string(),
                    active: *active,
                    task_count,
                    last_modified,
                });
                // Only count the first one found (.git takes precedence)
                break;
            }
        }
    }

    // Sort by last_modified descending (most recent first)
    workspaces.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    log::info!("Found {} checkpoint workspaces", workspaces.len());
    workspaces
}

/// Count distinct task-ids and find the most recent commit timestamp.
/// Returns (task_count, last_modified_iso).
fn count_tasks_and_latest(git_dir: &PathBuf) -> (usize, String) {
    let commits = parse_checkpoint_commits(git_dir);
    let mut task_ids = std::collections::HashSet::new();
    let mut latest = String::new();

    for (_, task_id, ts) in &commits {
        task_ids.insert(task_id.clone());
        // git log returns in reverse chronological order, so the first entry is the latest
        if latest.is_empty() {
            latest = ts.clone();
        }
    }

    (task_ids.len(), latest)
}

/// Parsed checkpoint commit: (hash, task_id, iso_timestamp)
type CheckpointCommit = (String, String, String);

/// Parse all checkpoint commits from a git repo.
/// Returns Vec of (commit_hash, task_id, iso_timestamp).
fn parse_checkpoint_commits(git_dir: &PathBuf) -> Vec<CheckpointCommit> {
    let git_dir_str = git_dir.to_string_lossy().to_string();

    // git --git-dir <path> log --all --pretty=format:%H|%s|%aI
    let output = Command::new("git")
        .args([
            "--git-dir",
            &git_dir_str,
            "log",
            "--all",
            "--pretty=format:%H|%s|%aI",
        ])
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                log::warn!("git log failed for {:?}: {}", git_dir, stderr.trim());
                return Vec::new();
            }

            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut commits = Vec::new();

            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(3, '|').collect();
                if parts.len() < 3 {
                    continue;
                }
                let hash = parts[0].to_string();
                let subject = parts[1];
                let timestamp = parts[2].to_string();

                // Parse: checkpoint-<wsId>-<taskId>
                if let Some(rest) = subject.strip_prefix("checkpoint-") {
                    if let Some(dash_pos) = rest.rfind('-') {
                        let task_id = &rest[dash_pos + 1..];
                        if !task_id.is_empty() {
                            commits.push((hash, task_id.to_string(), timestamp));
                        }
                    }
                }
            }

            commits
        }
        Err(e) => {
            log::error!("Failed to execute git for {:?}: {}", git_dir, e);
            Vec::new()
        }
    }
}

/// Count files changed in a single commit using git diff --name-only
fn count_files_in_commit(git_dir: &PathBuf, hash: &str) -> usize {
    let git_dir_str = git_dir.to_string_lossy().to_string();
    // diff this commit vs its parent: git --git-dir <path> diff --name-only <hash>^..<hash>
    let output = Command::new("git")
        .args([
            "--git-dir",
            &git_dir_str,
            "diff",
            "--name-only",
            &format!("{}^..{}", hash, hash),
        ])
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                // Might fail for root commit (no parent). Try diff-tree for root.
                return count_files_root_commit(git_dir, hash);
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().filter(|l| !l.is_empty()).count()
        }
        Err(_) => 0,
    }
}

/// Count files in a root commit (no parent) using diff-tree
fn count_files_root_commit(git_dir: &PathBuf, hash: &str) -> usize {
    let git_dir_str = git_dir.to_string_lossy().to_string();
    let output = Command::new("git")
        .args([
            "--git-dir",
            &git_dir_str,
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            hash,
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().filter(|l| !l.is_empty()).count()
        }
        Err(_) => 0,
    }
}

/// List all tasks for a specific workspace, grouped from checkpoint commits.
/// The `git_dir` should be the .git or .git_disabled path for the workspace.
pub fn list_tasks_for_workspace(workspace_id: &str, git_dir: &PathBuf) -> Vec<ClineTaskSummary> {
    let commits = parse_checkpoint_commits(git_dir);

    // Group commits by task_id
    let mut task_map: HashMap<String, Vec<CheckpointCommit>> = HashMap::new();
    for commit in commits {
        task_map.entry(commit.1.clone()).or_default().push(commit);
    }

    let mut tasks: Vec<ClineTaskSummary> = task_map
        .into_iter()
        .map(|(task_id, task_commits)| {
            let steps = task_commits.len();

            // Count total distinct files changed across all steps
            let mut all_files = std::collections::HashSet::new();
            for (hash, _, _) in &task_commits {
                let git_dir_str = git_dir.to_string_lossy().to_string();
                let output = Command::new("git")
                    .args([
                        "--git-dir",
                        &git_dir_str,
                        "diff",
                        "--name-only",
                        &format!("{}^..{}", hash, hash),
                    ])
                    .output();
                if let Ok(out) = output {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    for f in stdout.lines().filter(|l| !l.is_empty()) {
                        all_files.insert(f.to_string());
                    }
                }
            }

            // Most recent timestamp (commits are in reverse chronological order from git log)
            let last_modified = task_commits
                .first()
                .map(|(_, _, ts)| ts.clone())
                .unwrap_or_default();

            ClineTaskSummary {
                task_id,
                workspace_id: workspace_id.to_string(),
                steps,
                files_changed: all_files.len(),
                last_modified,
            }
        })
        .collect();

    // Sort by last_modified descending (most recent first)
    tasks.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    log::info!(
        "Found {} tasks for workspace {}",
        tasks.len(),
        workspace_id
    );
    tasks
}

/// List individual checkpoint steps for a specific task.
/// Returns steps in chronological order (oldest first), each with a 1-based index.
pub fn list_steps_for_task(
    task_id: &str,
    workspace_id: &str,
    git_dir: &PathBuf,
) -> Vec<super::types::CheckpointStep> {
    let commits = parse_checkpoint_commits(git_dir);

    // Filter to only commits for this task, they come in reverse chronological order
    let mut task_commits: Vec<CheckpointCommit> = commits
        .into_iter()
        .filter(|(_, tid, _)| tid == task_id)
        .collect();

    // Reverse to chronological order (oldest first)
    task_commits.reverse();

    let steps: Vec<super::types::CheckpointStep> = task_commits
        .iter()
        .enumerate()
        .map(|(i, (hash, _, timestamp))| {
            let files_changed = count_files_in_commit(git_dir, hash);
            super::types::CheckpointStep {
                hash: hash.clone(),
                subject: format!("checkpoint-{}-{}", workspace_id, task_id),
                timestamp: timestamp.clone(),
                files_changed,
                index: i + 1,
            }
        })
        .collect();

    log::info!(
        "Found {} steps for task {} in workspace {}",
        steps.len(),
        task_id,
        workspace_id
    );
    steps
}

/// Compute the diff for a single step (parent → commit).
/// `step_index` is 1-based. Returns a DiffResult with file list + unified patch.
pub fn get_step_diff(
    task_id: &str,
    step_index: usize,
    git_dir: &PathBuf,
) -> Result<super::types::DiffResult, String> {
    let commits = parse_checkpoint_commits(git_dir);

    // Filter to this task, reverse to chronological order
    let mut task_commits: Vec<CheckpointCommit> = commits
        .into_iter()
        .filter(|(_, tid, _)| tid == task_id)
        .collect();
    task_commits.reverse();

    if step_index == 0 || step_index > task_commits.len() {
        return Err(format!(
            "Step index {} out of range (task has {} steps)",
            step_index,
            task_commits.len()
        ));
    }

    let (to_hash, _, _) = &task_commits[step_index - 1];
    let to_ref = to_hash.clone();

    // from_ref = parent of the step commit (to_hash^), or empty tree for the first commit
    let from_ref = if step_index > 1 {
        task_commits[step_index - 2].0.clone()
    } else {
        // For the first step, use the parent of the commit (may not exist for root)
        format!("{}^", to_ref)
    };

    let git_dir_str = git_dir.to_string_lossy().to_string();

    // Get --numstat for file-level stats
    let numstat_output = Command::new("git")
        .args([
            "--git-dir", &git_dir_str,
            "diff", "--numstat",
            &from_ref, &to_ref,
        ])
        .output()
        .map_err(|e| format!("Failed to run git diff --numstat: {}", e))?;

    let files = if numstat_output.status.success() {
        parse_numstat(&String::from_utf8_lossy(&numstat_output.stdout))
    } else {
        // Might be root commit — try diff-tree
        let dt_out = Command::new("git")
            .args([
                "--git-dir", &git_dir_str,
                "diff-tree", "--numstat", "--no-commit-id", "-r", &to_ref,
            ])
            .output()
            .map_err(|e| format!("Failed to run git diff-tree: {}", e))?;
        parse_numstat(&String::from_utf8_lossy(&dt_out.stdout))
    };

    // Get unified diff patch text
    let patch_output = Command::new("git")
        .args([
            "--git-dir", &git_dir_str,
            "diff", &from_ref, &to_ref,
        ])
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    let patch = if patch_output.status.success() {
        String::from_utf8_lossy(&patch_output.stdout).to_string()
    } else {
        // Try diff-tree for root commits
        let dt_out = Command::new("git")
            .args([
                "--git-dir", &git_dir_str,
                "diff-tree", "-p", "--no-commit-id", "-r", &to_ref,
            ])
            .output()
            .unwrap_or(patch_output);
        String::from_utf8_lossy(&dt_out.stdout).to_string()
    };

    log::info!(
        "Step diff for task {} step {}: {} files, {} bytes patch",
        task_id, step_index, files.len(), patch.len()
    );

    Ok(super::types::DiffResult {
        files,
        patch,
        from_ref,
        to_ref,
    })
}

/// Parse git --numstat output into DiffFile vec.
/// Format: <added>\t<removed>\t<path>
fn parse_numstat(output: &str) -> Vec<super::types::DiffFile> {
    output
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                return None;
            }
            let added = parts[0].parse::<usize>().unwrap_or(0);
            let removed = parts[1].parse::<usize>().unwrap_or(0);
            let path = parts[2].to_string();

            let status = if added > 0 && removed == 0 && parts[0] != "-" {
                "added".to_string()
            } else if removed > 0 && added == 0 {
                "deleted".to_string()
            } else {
                "modified".to_string()
            };

            Some(super::types::DiffFile {
                path,
                lines_added: added,
                lines_removed: removed,
                status,
            })
        })
        .collect()
}

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
    let mut git_commands: Vec<String> = Vec::new();

    // Get --numstat for file-level stats
    let numstat_args = [
        "--git-dir", &git_dir_str,
        "diff", "--numstat",
        &from_ref, &to_ref,
    ];
    git_commands.push(format!("git {}", numstat_args.join(" ")));

    let numstat_output = Command::new("git")
        .args(&numstat_args)
        .output()
        .map_err(|e| format!("Failed to run git diff --numstat: {}", e))?;

    let files = if numstat_output.status.success() {
        parse_numstat(&String::from_utf8_lossy(&numstat_output.stdout))
    } else {
        // Might be root commit — try diff-tree
        let dt_args = [
            "--git-dir", &git_dir_str,
            "diff-tree", "--numstat", "--no-commit-id", "-r", &to_ref,
        ];
        git_commands.push(format!("git {} (fallback)", dt_args.join(" ")));
        let dt_out = Command::new("git")
            .args(&dt_args)
            .output()
            .map_err(|e| format!("Failed to run git diff-tree: {}", e))?;
        parse_numstat(&String::from_utf8_lossy(&dt_out.stdout))
    };

    // Get unified diff patch text
    let patch_args = [
        "--git-dir", &git_dir_str,
        "diff", &from_ref, &to_ref,
    ];
    git_commands.push(format!("git {}", patch_args.join(" ")));

    let patch_output = Command::new("git")
        .args(&patch_args)
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    let patch = if patch_output.status.success() {
        String::from_utf8_lossy(&patch_output.stdout).to_string()
    } else {
        // Try diff-tree for root commits
        let dt_patch_args = [
            "--git-dir", &git_dir_str,
            "diff-tree", "-p", "--no-commit-id", "-r", &to_ref,
        ];
        git_commands.push(format!("git {} (fallback)", dt_patch_args.join(" ")));
        let dt_out = Command::new("git")
            .args(&dt_patch_args)
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
        git_commands,
    })
}

/// Compute the full task diff (first checkpoint's parent → last checkpoint).
/// This gives the complete set of changes for the entire task.
/// Supports `exclude` patterns for pathspec exclusions.
pub fn get_task_diff(
    task_id: &str,
    git_dir: &PathBuf,
    excludes: &[String],
) -> Result<super::types::DiffResult, String> {
    // Verify git_dir exists on disk (Cline may rename .git ↔ .git_disabled during tasks)
    if !git_dir.exists() {
        return Err(format!(
            "Git directory does not exist (Cline may have disabled it): {}",
            git_dir.display()
        ));
    }

    let commits = parse_checkpoint_commits(git_dir);

    // Filter to this task, reverse to chronological order (oldest first)
    let mut task_commits: Vec<CheckpointCommit> = commits
        .into_iter()
        .filter(|(_, tid, _)| tid == task_id)
        .collect();
    task_commits.reverse();

    if task_commits.is_empty() {
        return Err(format!("No checkpoint commits found for task '{}'", task_id));
    }

    let first_hash = &task_commits[0].0;
    let last_hash = &task_commits[task_commits.len() - 1].0;

    // from_ref = parent of first checkpoint (first_hash^)
    let from_ref = format!("{}^", first_hash);
    let to_ref = last_hash.clone();

    let git_dir_str = git_dir.to_string_lossy().to_string();
    let mut git_commands: Vec<String> = Vec::new();

    log::debug!(
        "Task diff: git --git-dir {} diff --numstat {}  {} (excludes={:?})",
        git_dir_str, from_ref, to_ref, excludes
    );

    // Build numstat args with exclude patterns
    // Use ":/" (repo root) instead of "." (CWD-relative) to avoid pathspec issues
    let mut numstat_args = vec![
        "--git-dir".to_string(), git_dir_str.clone(),
        "diff".to_string(), "--numstat".to_string(),
        from_ref.clone(), to_ref.clone(),
    ];
    if !excludes.is_empty() {
        numstat_args.push("--".to_string());
        numstat_args.push(":/".to_string());
        for ex in excludes {
            numstat_args.push(format!(":(exclude){}", ex));
        }
    }

    git_commands.push(format!("git {}", numstat_args.join(" ")));

    let numstat_output = Command::new("git")
        .args(&numstat_args)
        .output()
        .map_err(|e| format!("Failed to run git diff --numstat: {}", e))?;

    let files = if numstat_output.status.success() {
        let stdout = String::from_utf8_lossy(&numstat_output.stdout);
        let stderr = String::from_utf8_lossy(&numstat_output.stderr);
        if !stderr.is_empty() {
            log::warn!("git diff --numstat stderr: {}", stderr.trim());
        }
        if stdout.trim().is_empty() {
            log::warn!(
                "git diff --numstat returned empty stdout for task {} ({} → {})",
                task_id, from_ref, to_ref
            );
        }
        parse_numstat(&stdout)
    } else {
        let stderr = String::from_utf8_lossy(&numstat_output.stderr);
        log::warn!(
            "git diff --numstat failed (exit={}): {}. Trying diff-tree fallback.",
            numstat_output.status, stderr.trim()
        );
        // Fallback: try without parent (root commit scenario)
        let mut fallback_args = vec![
            "--git-dir".to_string(), git_dir_str.clone(),
            "diff-tree".to_string(), "--numstat".to_string(),
            "--no-commit-id".to_string(), "-r".to_string(),
            to_ref.clone(),
        ];
        if !excludes.is_empty() {
            for ex in excludes {
                fallback_args.push(format!(":(exclude){}", ex));
            }
        }
        git_commands.push(format!("git {} (fallback)", fallback_args.join(" ")));
        let dt_out = Command::new("git")
            .args(&fallback_args)
            .output()
            .map_err(|e| format!("Failed to run git diff-tree: {}", e))?;
        if !dt_out.status.success() {
            let dt_stderr = String::from_utf8_lossy(&dt_out.stderr);
            log::error!("git diff-tree also failed: {}", dt_stderr.trim());
        }
        parse_numstat(&String::from_utf8_lossy(&dt_out.stdout))
    };

    // Build patch args with exclude patterns
    let mut patch_args = vec![
        "--git-dir".to_string(), git_dir_str.clone(),
        "diff".to_string(),
        from_ref.clone(), to_ref.clone(),
    ];
    if !excludes.is_empty() {
        patch_args.push("--".to_string());
        patch_args.push(":/".to_string());
        for ex in excludes {
            patch_args.push(format!(":(exclude){}", ex));
        }
    }

    git_commands.push(format!("git {}", patch_args.join(" ")));

    let patch_output = Command::new("git")
        .args(&patch_args)
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    let patch = if patch_output.status.success() {
        let stderr = String::from_utf8_lossy(&patch_output.stderr);
        if !stderr.is_empty() {
            log::warn!("git diff patch stderr: {}", stderr.trim());
        }
        String::from_utf8_lossy(&patch_output.stdout).to_string()
    } else {
        let stderr = String::from_utf8_lossy(&patch_output.stderr);
        log::warn!("git diff patch failed (exit={}): {}. Trying diff-tree fallback.", patch_output.status, stderr.trim());
        // Fallback for root commit
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
        "Task diff for task {}: {} → {} ({} files, {} bytes patch)",
        task_id, from_ref, to_ref, files.len(), patch.len()
    );

    Ok(super::types::DiffResult {
        files,
        patch,
        from_ref,
        to_ref,
        git_commands,
    })
}

/// Parse an ISO 8601 / RFC 3339 timestamp into epoch milliseconds for comparison.
/// Handles both chrono rfc3339 (with fractional seconds) and git %aI (without).
/// Falls back to string comparison if parsing fails.
fn parse_timestamp_ms(ts: &str) -> i64 {
    // Try chrono parsing (handles both formats)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        return dt.timestamp_millis();
    }
    // Fallback: try without fractional seconds
    if let Ok(dt) = chrono::DateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%:z") {
        return dt.timestamp_millis();
    }
    log::warn!("Failed to parse timestamp for comparison: {}", ts);
    0
}

/// Map subtask time boundaries to checkpoint step ranges.
///
/// Given subtask timestamps (from conversation_history) and step timestamps (from git),
/// returns Vec of (subtask_index, first_step_array_idx, last_step_array_idx).
/// Steps array must be in chronological order (oldest first).
///
/// Uses proper datetime parsing instead of lexicographic string comparison
/// to handle format differences (chrono rfc3339 with fractional seconds vs
/// git %aI without fractional seconds).
pub fn map_subtasks_to_steps(
    subtasks: &crate::conversation_history::types::SubtasksResponse,
    steps: &[super::types::CheckpointStep],
) -> Vec<(usize, usize, usize)> {
    let mut mappings = Vec::new();

    // Pre-parse step timestamps to epoch_ms for efficient comparison
    let step_times: Vec<i64> = steps.iter()
        .map(|s| parse_timestamp_ms(&s.timestamp))
        .collect();

    for (si, subtask) in subtasks.subtasks.iter().enumerate() {
        let subtask_start_ms = parse_timestamp_ms(&subtask.timestamp);
        let subtask_end_ms = subtasks.subtasks
            .get(si + 1)
            .map(|next| parse_timestamp_ms(&next.timestamp));

        let mut first_step: Option<usize> = None;
        let mut last_step: Option<usize> = None;

        for (i, step_ms) in step_times.iter().enumerate() {
            let in_range = *step_ms >= subtask_start_ms
                && subtask_end_ms.map_or(true, |end| *step_ms < end);
            if in_range {
                if first_step.is_none() {
                    first_step = Some(i);
                }
                last_step = Some(i);
            }
        }

        if let (Some(first), Some(last)) = (first_step, last_step) {
            log::debug!(
                "Subtask #{} mapped to steps {}..{} (step hashes: {}..{})",
                si, first, last,
                &steps[first].hash[..8], &steps[last].hash[..8]
            );
            mappings.push((si, first, last));
        } else {
            log::warn!(
                "Subtask #{} (start={}, end={:?}) has no matching checkpoint steps",
                si, subtask.timestamp,
                subtasks.subtasks.get(si + 1).map(|n| &n.timestamp)
            );
        }
    }

    mappings
}

/// Compute the diff for a single subtask phase.
///
/// Maps conversation history subtask boundaries to checkpoint step ranges,
/// then computes git diff between the boundary steps.
/// `subtask_index` is 0-based (0 = initial task, 1+ = feedback subtasks).
pub fn get_subtask_diff(
    task_id: &str,
    subtask_index: usize,
    workspace_id: &str,
    git_dir: &PathBuf,
    excludes: &[String],
) -> Result<super::types::DiffResult, String> {
    // Verify git_dir exists on disk (Cline may rename .git ↔ .git_disabled during tasks)
    if !git_dir.exists() {
        return Err(format!(
            "Git directory does not exist (Cline may have disabled it): {}",
            git_dir.display()
        ));
    }

    // 1. Get subtask boundaries from conversation_history
    let subtasks = crate::conversation_history::subtasks::parse_task_subtasks(task_id)
        .ok_or_else(|| format!("No subtask data for task '{}' (ui_messages.json not found or no task entry)", task_id))?;

    if subtask_index >= subtasks.total_subtasks {
        return Err(format!(
            "Subtask index {} out of range (task has {} subtasks)",
            subtask_index, subtasks.total_subtasks
        ));
    }

    // 2. Get checkpoint steps (chronological order)
    let steps = list_steps_for_task(task_id, workspace_id, git_dir);
    if steps.is_empty() {
        return Err(format!("No checkpoint steps for task '{}'", task_id));
    }

    // 3. Map subtasks to step ranges
    let mappings = map_subtasks_to_steps(&subtasks, &steps);
    let mapping = mappings.iter()
        .find(|(si, _, _)| *si == subtask_index)
        .ok_or_else(|| format!(
            "No checkpoint steps found within subtask #{}'s time window",
            subtask_index
        ))?;

    let (_, first_step_idx, last_step_idx) = *mapping;

    // 4. Compute diff boundaries
    let to_ref = steps[last_step_idx].hash.clone();
    let from_ref = if first_step_idx > 0 {
        // Previous step's commit = baseline for this subtask
        steps[first_step_idx - 1].hash.clone()
    } else {
        // First subtask — use parent of first commit
        format!("{}^", steps[first_step_idx].hash)
    };

    let git_dir_str = git_dir.to_string_lossy().to_string();
    let mut git_commands: Vec<String> = Vec::new();

    log::debug!(
        "Subtask diff: git --git-dir {} diff --numstat {} {} (subtask #{}, excludes={:?})",
        git_dir_str, from_ref, to_ref, subtask_index, excludes
    );

    // 5. Build numstat args with exclude patterns
    // Do NOT use "-- ." pathspec (CWD-relative) — omit pathspec unless excludes are needed
    let mut numstat_args = vec![
        "--git-dir".to_string(), git_dir_str.clone(),
        "diff".to_string(), "--numstat".to_string(),
        from_ref.clone(), to_ref.clone(),
    ];
    if !excludes.is_empty() {
        numstat_args.push("--".to_string());
        numstat_args.push(":/".to_string());
        for ex in excludes {
            numstat_args.push(format!(":(exclude){}", ex));
        }
    }

    git_commands.push(format!("git {}", numstat_args.join(" ")));

    let numstat_output = Command::new("git")
        .args(&numstat_args)
        .output()
        .map_err(|e| format!("Failed to run git diff --numstat: {}", e))?;

    let files = if numstat_output.status.success() {
        let stdout = String::from_utf8_lossy(&numstat_output.stdout);
        let stderr = String::from_utf8_lossy(&numstat_output.stderr);
        if !stderr.is_empty() {
            log::warn!("git diff --numstat stderr (subtask #{}): {}", subtask_index, stderr.trim());
        }
        if stdout.trim().is_empty() {
            log::warn!(
                "git diff --numstat returned empty for subtask #{} ({} → {})",
                subtask_index, from_ref, to_ref
            );
        }
        parse_numstat(&stdout)
    } else {
        let stderr = String::from_utf8_lossy(&numstat_output.stderr);
        log::warn!(
            "git diff --numstat failed for subtask #{} (exit={}): {}. Trying diff-tree fallback.",
            subtask_index, numstat_output.status, stderr.trim()
        );
        // Fallback for root commit
        let mut fallback_args = vec![
            "--git-dir".to_string(), git_dir_str.clone(),
            "diff-tree".to_string(), "--numstat".to_string(),
            "--no-commit-id".to_string(), "-r".to_string(),
            to_ref.clone(),
        ];
        if !excludes.is_empty() {
            for ex in excludes {
                fallback_args.push(format!(":(exclude){}", ex));
            }
        }
        let dt_out = Command::new("git")
            .args(&fallback_args)
            .output()
            .map_err(|e| format!("Failed to run git diff-tree: {}", e))?;
        if !dt_out.status.success() {
            let dt_stderr = String::from_utf8_lossy(&dt_out.stderr);
            log::error!("git diff-tree also failed for subtask #{}: {}", subtask_index, dt_stderr.trim());
        }
        parse_numstat(&String::from_utf8_lossy(&dt_out.stdout))
    };

    // 6. Build patch args with exclude patterns
    let mut patch_args = vec![
        "--git-dir".to_string(), git_dir_str.clone(),
        "diff".to_string(),
        from_ref.clone(), to_ref.clone(),
    ];
    if !excludes.is_empty() {
        patch_args.push("--".to_string());
        patch_args.push(":/".to_string());
        for ex in excludes {
            patch_args.push(format!(":(exclude){}", ex));
        }
    }

    git_commands.push(format!("git {}", patch_args.join(" ")));

    let patch_output = Command::new("git")
        .args(&patch_args)
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    let patch = if patch_output.status.success() {
        let stderr = String::from_utf8_lossy(&patch_output.stderr);
        if !stderr.is_empty() {
            log::warn!("git diff patch stderr (subtask #{}): {}", subtask_index, stderr.trim());
        }
        String::from_utf8_lossy(&patch_output.stdout).to_string()
    } else {
        let stderr = String::from_utf8_lossy(&patch_output.stderr);
        log::warn!(
            "git diff patch failed for subtask #{} (exit={}): {}. Trying diff-tree fallback.",
            subtask_index, patch_output.status, stderr.trim()
        );
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
        "Subtask diff for task {} subtask #{}: {} → {} ({} files, {} bytes patch)",
        task_id, subtask_index, from_ref, to_ref, files.len(), patch.len()
    );

    Ok(super::types::DiffResult {
        files,
        patch,
        from_ref,
        to_ref,
        git_commands,
    })
}

/// Find which workspace contains a given task_id by scanning all workspaces.
///
/// Returns (workspace_id, git_dir_path) on first match.
/// Iterates all checkpoint workspaces and checks their commit subjects for the task_id.
pub fn find_workspace_for_task(task_id: &str) -> Option<(String, PathBuf)> {
    let workspaces = find_workspaces();

    for ws in &workspaces {
        let git_dir = PathBuf::from(&ws.git_dir);
        let commits = parse_checkpoint_commits(&git_dir);
        let has_task = commits.iter().any(|(_, tid, _)| tid == task_id);
        if has_task {
            log::info!(
                "Resolved task {} → workspace {} (git_dir: {})",
                task_id, ws.id, ws.git_dir
            );
            return Some((ws.id.clone(), git_dir));
        }
    }

    log::warn!("No workspace found containing task {}", task_id);
    None
}

/// Get file contents at a specific git ref using `git show <ref>:<path>`.
///
/// For each path, runs `git --git-dir <git_dir> show <ref>:<path>` and
/// returns the file content. Deleted files (not present at `ref`) will
/// have `content: None` and an error message.
///
/// Binary files may return garbled content — callers should skip them.
pub fn get_file_contents(
    git_dir: &PathBuf,
    git_ref: &str,
    paths: &[String],
) -> Vec<super::types::FileContent> {
    let git_dir_str = git_dir.to_string_lossy().to_string();

    paths.iter().map(|path| {
        let ref_path = format!("{}:{}", git_ref, path);
        let output = Command::new("git")
            .args(["--git-dir", &git_dir_str, "show", &ref_path])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let content = String::from_utf8_lossy(&out.stdout).to_string();
                let size = content.len();
                super::types::FileContent {
                    path: path.clone(),
                    content: Some(content),
                    error: None,
                    size: Some(size),
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                log::debug!("git show {} failed for {}: {}", ref_path, path, stderr.trim());
                super::types::FileContent {
                    path: path.clone(),
                    content: None,
                    error: Some(stderr.trim().to_string()),
                    size: None,
                }
            }
            Err(e) => {
                super::types::FileContent {
                    path: path.clone(),
                    content: None,
                    error: Some(format!("Failed to execute git: {}", e)),
                    size: None,
                }
            }
        }
    }).collect()
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

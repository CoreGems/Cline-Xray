use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use super::{cache, cleanup, discovery};
use super::types::{DiffResult, FileContentsRequest, FileContentsResponse, StepsResponse, TasksResponse, WorkspacesResponse};
use super::cleanup::NukeWorkspaceResponse;

// ============ In-memory caches ============

/// Cached workspaces result (populated from disk or after discovery)
static WORKSPACES_CACHE: once_cell::sync::Lazy<RwLock<Option<WorkspacesResponse>>> =
    once_cell::sync::Lazy::new(|| {
        // On first access, try loading from disk cache
        let disk = cache::load_workspaces();
        RwLock::new(disk)
    });

/// Cached tasks per workspace: workspace_id → TasksResponse
/// Pre-populated from disk cache on first access.
static TASKS_CACHE: once_cell::sync::Lazy<RwLock<std::collections::HashMap<String, TasksResponse>>> =
    once_cell::sync::Lazy::new(|| {
        RwLock::new(std::collections::HashMap::new())
    });

/// Cached steps per task: "workspace_id:task_id" → StepsResponse
/// Loaded lazily from disk per-task.
static STEPS_CACHE: once_cell::sync::Lazy<RwLock<std::collections::HashMap<String, StepsResponse>>> =
    once_cell::sync::Lazy::new(|| {
        RwLock::new(std::collections::HashMap::new())
    });

// ============ Types ============

/// Error response for changes endpoints
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ChangesErrorResponse {
    pub error: String,
    pub code: u16,
}

/// Query parameters for /changes/workspaces
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct WorkspacesQuery {
    /// Set to true to force re-discovery (bypass cache)
    #[serde(default)]
    pub refresh: Option<bool>,
}

/// Query parameters for /changes/tasks
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TasksQuery {
    /// Workspace ID to list tasks for (required)
    pub workspace: String,
    /// Set to true to force re-enumeration (bypass cache)
    #[serde(default)]
    pub refresh: Option<bool>,
}

/// Query parameters for /changes/tasks/:taskId/steps
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct StepsQuery {
    /// Workspace ID (required to locate the git repo)
    pub workspace: String,
    /// Set to true to force re-enumeration (bypass cache)
    #[serde(default)]
    pub refresh: Option<bool>,
}

/// Query parameters for /changes/tasks/:taskId/steps/:index/diff
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct StepDiffQuery {
    /// Workspace ID (required to locate the git repo)
    pub workspace: String,
}

/// Query parameters for /changes/tasks/:taskId/diff
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TaskDiffQuery {
    /// Workspace ID (required to locate the git repo)
    pub workspace: String,
    /// Pathspec exclusion patterns (repeated), e.g. ?exclude=node_modules&exclude=target
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Query parameters for /changes/tasks/:taskId/subtasks/:subtaskIndex/diff
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SubtaskDiffQuery {
    /// Workspace ID (required to locate the git repo)
    pub workspace: String,
    /// Pathspec exclusion patterns (repeated)
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Path parameters for subtask diff endpoint
#[derive(Debug, Deserialize)]
pub struct SubtaskDiffPath {
    pub task_id: String,
    pub subtask_index: usize,
}

/// Path parameters for step diff endpoint
#[derive(Debug, Deserialize)]
pub struct StepDiffPath {
    pub task_id: String,
    pub index: usize,
}

// ============ Handlers ============

/// List discovered checkpoint workspaces
///
/// Scans the Cline globalStorage checkpoints directory and returns all
/// workspace directories that contain a shadow Git repo (.git or .git_disabled).
/// For each workspace, a task count is computed by parsing commit subjects.
///
/// Results are cached in memory and persisted to disk (`%APPDATA%/jira-dashboard/shadow_git_cache/`).
/// On cold start, the disk cache is loaded automatically.
/// Pass `?refresh=true` to force re-discovery from disk.
#[utoipa::path(
    get,
    path = "/changes/workspaces",
    params(WorkspacesQuery),
    responses(
        (status = 200, description = "List of discovered checkpoint workspaces", body = WorkspacesResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "tool"]
)]
pub async fn list_workspaces_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<WorkspacesQuery>,
) -> Result<Json<WorkspacesResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    let force_refresh = params.refresh.unwrap_or(false);

    // Return cached data if available and not refreshing
    if !force_refresh {
        let cache = WORKSPACES_CACHE.read();
        if let Some(ref cached) = *cache {
            log::info!(
                "REST API: GET /changes/workspaces — returning {} cached workspaces",
                cached.workspaces.len()
            );
            return Ok(Json(cached.clone()));
        }
    }

    log::info!(
        "REST API: GET /changes/workspaces — discovering (refresh={})",
        force_refresh
    );

    // Run discovery in blocking context (filesystem + git CLI calls)
    let result = tokio::task::spawn_blocking(|| {
        let workspaces = discovery::find_workspaces();
        let root = discovery::checkpoints_root()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "N/A".to_string());
        WorkspacesResponse {
            workspaces,
            checkpoints_root: root,
        }
    })
    .await;

    match result {
        Ok(response) => {
            log::info!(
                "REST API: Discovered {} checkpoint workspaces — caching (memory + disk)",
                response.workspaces.len()
            );
            // Update memory cache
            *WORKSPACES_CACHE.write() = Some(response.clone());
            // Persist to disk
            cache::save_workspaces(&response);
            Ok(Json(response))
        }
        Err(e) => {
            log::error!("REST API: Failed to discover workspaces: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to discover workspaces: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// List all tasks for a workspace
///
/// Enumerates all Cline tasks (groups of checkpoint commits) for the specified
/// workspace. Each task includes step count, files changed, and last modified
/// timestamp.
///
/// Results are cached in memory and persisted to disk.
/// On cold start, disk cache is loaded per-workspace on first request.
/// Pass `?refresh=true` to re-scan from git.
#[utoipa::path(
    get,
    path = "/changes/tasks",
    params(TasksQuery),
    responses(
        (status = 200, description = "List of tasks for the workspace", body = TasksResponse),
        (status = 400, description = "Missing or invalid workspace parameter", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "tool"]
)]
pub async fn list_tasks_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<TasksQuery>,
) -> Result<Json<TasksResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    let workspace_id = params.workspace.clone();
    let force_refresh = params.refresh.unwrap_or(false);

    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'workspace' query parameter".to_string(),
                code: 400,
            }),
        ));
    }

    // Return memory-cached data if available and not refreshing
    if !force_refresh {
        let mem_cache = TASKS_CACHE.read();
        if let Some(cached) = mem_cache.get(&workspace_id) {
            log::info!(
                "REST API: GET /changes/tasks — returning {} memory-cached tasks for workspace {}",
                cached.tasks.len(),
                workspace_id
            );
            return Ok(Json(cached.clone()));
        }
        drop(mem_cache);

        // Try disk cache (cold start scenario)
        if let Some(disk_cached) = cache::load_tasks(&workspace_id) {
            log::info!(
                "REST API: GET /changes/tasks — loaded {} tasks from disk cache for workspace {}",
                disk_cached.tasks.len(),
                workspace_id
            );
            // Promote to memory cache
            TASKS_CACHE.write().insert(workspace_id.clone(), disk_cached.clone());
            return Ok(Json(disk_cached));
        }
    }

    log::info!(
        "REST API: GET /changes/tasks — enumerating tasks for workspace {} (refresh={})",
        workspace_id,
        force_refresh
    );

    // Look up the git_dir for this workspace from the workspaces cache or re-discover
    let git_dir = {
        let ws_cache = WORKSPACES_CACHE.read();
        ws_cache
            .as_ref()
            .and_then(|r| {
                r.workspaces
                    .iter()
                    .find(|w| w.id == workspace_id)
                    .map(|w| w.git_dir.clone())
            })
    };

    let git_dir = match git_dir {
        Some(d) => d,
        None => {
            // Not in cache — try to discover it
            let found = tokio::task::spawn_blocking({
                let ws_id = workspace_id.clone();
                move || {
                    let workspaces = discovery::find_workspaces();
                    workspaces
                        .into_iter()
                        .find(|w| w.id == ws_id)
                        .map(|w| w.git_dir)
                }
            })
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ChangesErrorResponse {
                        error: format!("Discovery failed: {}", e),
                        code: 500,
                    }),
                )
            })?;

            match found {
                Some(d) => d,
                None => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ChangesErrorResponse {
                            error: format!(
                                "Workspace '{}' not found in checkpoint repositories",
                                workspace_id
                            ),
                            code: 400,
                        }),
                    ));
                }
            }
        }
    };

    // Run task enumeration in blocking context
    let ws_id = workspace_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let git_path = std::path::PathBuf::from(&git_dir);
        let tasks = discovery::list_tasks_for_workspace(&ws_id, &git_path);
        TasksResponse {
            workspace_id: ws_id,
            tasks,
        }
    })
    .await;

    match result {
        Ok(response) => {
            log::info!(
                "REST API: Found {} tasks for workspace {} — caching (memory + disk)",
                response.tasks.len(),
                workspace_id
            );
            // Update memory cache
            TASKS_CACHE.write().insert(workspace_id.clone(), response.clone());
            // Persist to disk
            cache::save_tasks(&workspace_id, &response);
            Ok(Json(response))
        }
        Err(e) => {
            log::error!("REST API: Failed to enumerate tasks: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to enumerate tasks: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Helper: resolve git_dir for a workspace from cache or discovery
async fn resolve_git_dir(workspace_id: &str) -> Result<String, (StatusCode, Json<ChangesErrorResponse>)> {
    // Try memory cache first
    let cached = {
        let ws_cache = WORKSPACES_CACHE.read();
        ws_cache.as_ref().and_then(|r| {
            r.workspaces.iter().find(|w| w.id == workspace_id).map(|w| w.git_dir.clone())
        })
    };
    if let Some(d) = cached {
        return Ok(d);
    }

    // Fall back to discovery
    let ws_id = workspace_id.to_string();
    let found = tokio::task::spawn_blocking(move || {
        discovery::find_workspaces().into_iter().find(|w| w.id == ws_id).map(|w| w.git_dir)
    })
    .await
    .map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ChangesErrorResponse { error: format!("Discovery failed: {}", e), code: 500 }),
    ))?;

    found.ok_or_else(|| (
        StatusCode::BAD_REQUEST,
        Json(ChangesErrorResponse {
            error: format!("Workspace '{}' not found in checkpoint repositories", workspace_id),
            code: 400,
        }),
    ))
}

/// List checkpoint steps for a task
///
/// Returns the individual checkpoint commits (steps) for a given task,
/// in chronological order (oldest first). Each step includes the commit hash,
/// timestamp, and number of files changed vs its parent commit.
///
/// The `workspace` query parameter is required to locate the git repo.
#[utoipa::path(
    get,
    path = "/changes/tasks/{task_id}/steps",
    params(
        ("task_id" = String, Path, description = "Task ID to list steps for"),
        StepsQuery
    ),
    responses(
        (status = 200, description = "List of checkpoint steps for the task", body = StepsResponse),
        (status = 400, description = "Missing workspace parameter or task not found", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "tool"]
)]
pub async fn list_steps_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Query(params): Query<StepsQuery>,
) -> Result<Json<StepsResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    let workspace_id = params.workspace.clone();
    let force_refresh = params.refresh.unwrap_or(false);

    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'workspace' query parameter".to_string(),
                code: 400,
            }),
        ));
    }

    let cache_key = cache::steps_cache_key(&workspace_id, &task_id);

    // 1. Return memory-cached data if available and not refreshing
    if !force_refresh {
        let mem = STEPS_CACHE.read();
        if let Some(cached) = mem.get(&cache_key) {
            log::info!(
                "REST API: GET /changes/tasks/{}/steps — returning {} memory-cached steps",
                task_id, cached.steps.len()
            );
            return Ok(Json(cached.clone()));
        }
        drop(mem);

        // 2. Try disk cache (cold start / restart scenario)
        if let Some(disk_cached) = cache::load_steps(&workspace_id, &task_id) {
            log::info!(
                "REST API: GET /changes/tasks/{}/steps — loaded {} steps from disk cache",
                task_id, disk_cached.steps.len()
            );
            STEPS_CACHE.write().insert(cache_key.clone(), disk_cached.clone());
            return Ok(Json(disk_cached));
        }
    }

    // 3. Compute from git
    log::info!(
        "REST API: GET /changes/tasks/{}/steps — enumerating (workspace={}, refresh={})",
        task_id, workspace_id, force_refresh
    );

    let git_dir = resolve_git_dir(&workspace_id).await?;

    let tid = task_id.clone();
    let ws_id = workspace_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let git_path = std::path::PathBuf::from(&git_dir);
        let steps = discovery::list_steps_for_task(&tid, &ws_id, &git_path);
        StepsResponse {
            task_id: tid,
            workspace_id: ws_id,
            steps,
        }
    })
    .await;

    match result {
        Ok(response) => {
            log::info!(
                "REST API: Found {} steps for task {} — caching (memory + disk)",
                response.steps.len(),
                task_id
            );
            // Update memory cache
            STEPS_CACHE.write().insert(cache_key, response.clone());
            // Persist to disk
            cache::save_steps(&workspace_id, &task_id, &response);
            Ok(Json(response))
        }
        Err(e) => {
            log::error!("REST API: Failed to enumerate steps: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to enumerate steps: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Get the full task diff (base→HEAD)
///
/// Returns the unified diff and file-level stats for the entire task,
/// computed from the first checkpoint's parent to the last checkpoint.
/// This shows the cumulative changes across all steps.
///
/// Supports `exclude` query params for pathspec exclusion patterns
/// (e.g. `?exclude=src-tauri/target&exclude=node_modules`).
#[utoipa::path(
    get,
    path = "/changes/tasks/{task_id}/diff",
    params(
        ("task_id" = String, Path, description = "Task ID"),
        TaskDiffQuery
    ),
    responses(
        (status = 200, description = "Full task diff result", body = DiffResult),
        (status = 400, description = "Invalid parameters", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "tool"]
)]
pub async fn task_diff_handler(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    Query(params): Query<TaskDiffQuery>,
) -> Result<Json<DiffResult>, (StatusCode, Json<ChangesErrorResponse>)> {
    let workspace_id = params.workspace.clone();
    let explicit_excludes = params.exclude.clone();

    // Merge with .changesignore patterns (auto-load if no explicit excludes)
    let excludes = crate::changesignore::merge_excludes(&explicit_excludes);

    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'workspace' query parameter".to_string(),
                code: 400,
            }),
        ));
    }

    log::info!(
        "REST API: GET /changes/tasks/{}/diff — workspace={}, excludes={:?}",
        task_id, workspace_id, excludes
    );

    let git_dir = resolve_git_dir(&workspace_id).await?;

    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let git_path = std::path::PathBuf::from(&git_dir);
        discovery::get_task_diff(&tid, &git_path, &excludes)
    })
    .await;

    match result {
        Ok(Ok(diff)) => {
            log::info!(
                "REST API: Task diff for {}: {} files, {} bytes patch",
                task_id, diff.files.len(), diff.patch.len()
            );
            Ok(Json(diff))
        }
        Ok(Err(e)) => {
            log::warn!("REST API: Task diff error: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ChangesErrorResponse { error: e, code: 400 }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to compute task diff: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to compute task diff: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Get the diff for a single checkpoint step
///
/// Returns the unified diff (patch) and file-level statistics for the specified
/// step (by 1-based index) within a task. The diff is computed between the
/// step's parent commit and the step commit itself.
///
/// The `workspace` query parameter is required to locate the git repo.
#[utoipa::path(
    get,
    path = "/changes/tasks/{task_id}/steps/{index}/diff",
    params(
        ("task_id" = String, Path, description = "Task ID"),
        ("index" = usize, Path, description = "Step index (1-based, chronological)"),
        StepDiffQuery
    ),
    responses(
        (status = 200, description = "Diff result for the step", body = DiffResult),
        (status = 400, description = "Invalid parameters", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "tool"]
)]
pub async fn step_diff_handler(
    State(_state): State<Arc<AppState>>,
    Path(path): Path<StepDiffPath>,
    Query(params): Query<StepDiffQuery>,
) -> Result<Json<DiffResult>, (StatusCode, Json<ChangesErrorResponse>)> {
    let workspace_id = params.workspace.clone();
    let task_id = path.task_id;
    let step_index = path.index;

    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'workspace' query parameter".to_string(),
                code: 400,
            }),
        ));
    }

    log::info!(
        "REST API: GET /changes/tasks/{}/steps/{}/diff — workspace={}",
        task_id, step_index, workspace_id
    );

    let git_dir = resolve_git_dir(&workspace_id).await?;

    let tid = task_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let git_path = std::path::PathBuf::from(&git_dir);
        discovery::get_step_diff(&tid, step_index, &git_path)
    })
    .await;

    match result {
        Ok(Ok(diff)) => {
            log::info!(
                "REST API: Step diff for task {} step {}: {} files",
                task_id, step_index, diff.files.len()
            );
            Ok(Json(diff))
        }
        Ok(Err(e)) => {
            log::warn!("REST API: Step diff error: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ChangesErrorResponse {
                    error: e,
                    code: 400,
                }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to compute step diff: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to compute step diff: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Get the diff for a single subtask phase
///
/// Computes the diff for a specific subtask within a task by mapping
/// conversation history feedback boundaries to checkpoint commit ranges.
/// Subtask #0 is the initial task, #1+ are feedback-driven subtasks.
///
/// This bridges the conversation_history module (subtask detection from
/// `ui_messages.json`) with the shadow_git module (checkpoint commits).
/// Each subtask's time window is mapped to the checkpoint steps that
/// fall within it, and the diff is computed across that step range.
#[utoipa::path(
    get,
    path = "/changes/tasks/{task_id}/subtasks/{subtask_index}/diff",
    params(
        ("task_id" = String, Path, description = "Task ID"),
        ("subtask_index" = usize, Path, description = "Subtask index (0-based: 0=initial task, 1+=feedback subtasks)"),
        SubtaskDiffQuery
    ),
    responses(
        (status = 200, description = "Diff result for the subtask phase", body = DiffResult),
        (status = 400, description = "Invalid parameters or no steps in subtask window", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes", "tool"]
)]
pub async fn subtask_diff_handler(
    State(_state): State<Arc<AppState>>,
    Path(path): Path<SubtaskDiffPath>,
    Query(params): Query<SubtaskDiffQuery>,
) -> Result<Json<DiffResult>, (StatusCode, Json<ChangesErrorResponse>)> {
    let workspace_id = params.workspace.clone();
    let explicit_excludes = params.exclude.clone();
    let task_id = path.task_id;

    // Merge with .changesignore patterns (auto-load if no explicit excludes)
    let excludes = crate::changesignore::merge_excludes(&explicit_excludes);
    let subtask_index = path.subtask_index;

    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'workspace' query parameter".to_string(),
                code: 400,
            }),
        ));
    }

    log::info!(
        "REST API: GET /changes/tasks/{}/subtasks/{}/diff — workspace={}, excludes={:?}",
        task_id, subtask_index, workspace_id, excludes
    );

    let git_dir = resolve_git_dir(&workspace_id).await?;

    let tid = task_id.clone();
    let ws_id = workspace_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let git_path = std::path::PathBuf::from(&git_dir);
        discovery::get_subtask_diff(&tid, subtask_index, &ws_id, &git_path, &excludes)
    })
    .await;

    match result {
        Ok(Ok(diff)) => {
            log::info!(
                "REST API: Subtask diff for task {} subtask #{}: {} files, {} bytes patch",
                task_id, subtask_index, diff.files.len(), diff.patch.len()
            );
            Ok(Json(diff))
        }
        Ok(Err(e)) => {
            log::warn!("REST API: Subtask diff error: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ChangesErrorResponse { error: e, code: 400 }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to compute subtask diff: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to compute subtask diff: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Nuke a workspace's checkpoint history
///
/// Deletes ALL checkpoint history for the specified workspace by removing the
/// `.git` directory and re-initializing it as an empty bare repo.
/// The workspace ID stays the same, but all task/step commits are gone.
/// Cline will recreate checkpoints when the next task runs.
///
/// **Safety:**
/// - Cannot nuke if `.git_disabled` (Cline is actively running a task)
/// - Returns the number of deleted commits and tasks
///
/// **This operation cannot be undone.**
#[utoipa::path(
    post,
    path = "/changes/workspaces/{id}/nuke",
    params(
        ("id" = String, Path, description = "Workspace ID to nuke")
    ),
    responses(
        (status = 200, description = "Workspace nuked successfully", body = NukeWorkspaceResponse),
        (status = 400, description = "Cannot nuke (e.g. active task)", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes"]
)]
pub async fn nuke_workspace_handler(
    State(_state): State<Arc<AppState>>,
    Path(workspace_id): Path<String>,
) -> Result<Json<NukeWorkspaceResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing workspace ID".to_string(),
                code: 400,
            }),
        ));
    }

    log::info!(
        "REST API: POST /changes/workspaces/{}/nuke — nuking workspace",
        workspace_id
    );

    // Resolve git_dir for this workspace
    let git_dir = resolve_git_dir(&workspace_id).await?;

    let ws_id = workspace_id.clone();
    let gd = git_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        cleanup::nuke_workspace(&ws_id, &gd)
    })
    .await;

    match result {
        Ok(Ok(response)) => {
            log::info!(
                "REST API: Nuked workspace {} — {} commits, {} tasks deleted",
                workspace_id, response.deleted_commits, response.deleted_tasks
            );

            // Invalidate caches for this workspace
            TASKS_CACHE.write().remove(&workspace_id);
            // Remove all steps cache entries for this workspace
            {
                let mut steps = STEPS_CACHE.write();
                let keys_to_remove: Vec<String> = steps
                    .keys()
                    .filter(|k| k.starts_with(&format!("{}:", workspace_id)))
                    .cloned()
                    .collect();
                for k in keys_to_remove {
                    steps.remove(&k);
                }
            }
            // Invalidate workspaces cache to force re-discovery
            *WORKSPACES_CACHE.write() = None;

            Ok(Json(response))
        }
        Ok(Err(e)) => {
            log::warn!("REST API: Nuke workspace error: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(ChangesErrorResponse { error: e, code: 400 }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to nuke workspace: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to nuke workspace: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Get file contents from a checkpoint workspace at a specific git ref
///
/// Reads the contents of specified files from the shadow git repo using
/// `git show <ref>:<path>`. This is used to provide actual file bodies
/// (not just diffs) as context for LLM chat sessions.
///
/// For each requested path, returns the file content at the given git ref.
/// Files that don't exist at that ref (e.g., deleted files) will have
/// `content: null` and an error message.
#[utoipa::path(
    post,
    path = "/changes/file-contents",
    request_body = FileContentsRequest,
    responses(
        (status = 200, description = "File contents retrieved", body = FileContentsResponse),
        (status = 400, description = "Invalid parameters", body = ChangesErrorResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes"]
)]
pub async fn file_contents_handler(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<FileContentsRequest>,
) -> Result<Json<FileContentsResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    let workspace_id = body.workspace.clone();
    let git_ref = body.git_ref.clone();
    let paths = body.paths.clone();

    if workspace_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'workspace' field".to_string(),
                code: 400,
            }),
        ));
    }

    if git_ref.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ChangesErrorResponse {
                error: "Missing required 'gitRef' field".to_string(),
                code: 400,
            }),
        ));
    }

    if paths.is_empty() {
        return Ok(Json(FileContentsResponse {
            files: Vec::new(),
            retrieved: 0,
            failed: 0,
            total_size: 0,
        }));
    }

    log::info!(
        "REST API: POST /changes/file-contents — workspace={}, ref={}, {} paths",
        workspace_id, &git_ref[..std::cmp::min(8, git_ref.len())], paths.len()
    );

    let git_dir = resolve_git_dir(&workspace_id).await?;

    let result = tokio::task::spawn_blocking(move || {
        let git_path = std::path::PathBuf::from(&git_dir);
        discovery::get_file_contents(&git_path, &git_ref, &paths)
    })
    .await;

    match result {
        Ok(files) => {
            let retrieved = files.iter().filter(|f| f.content.is_some()).count();
            let failed = files.iter().filter(|f| f.content.is_none()).count();
            let total_size: usize = files.iter().filter_map(|f| f.size).sum();

            log::info!(
                "REST API: File contents — {} retrieved, {} failed, {} bytes total",
                retrieved, failed, total_size
            );

            Ok(Json(FileContentsResponse {
                files,
                retrieved,
                failed,
                total_size,
            }))
        }
        Err(e) => {
            log::error!("REST API: Failed to get file contents: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to get file contents: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

// ============ .changesignore Handlers ============

/// Get the current .changesignore patterns
///
/// Returns the parsed exclude patterns from the `.changesignore` file in the
/// project root. If the file doesn't exist, returns built-in defaults.
///
/// The response includes both the parsed patterns (for programmatic use)
/// and the raw file content (for UI editing).
#[utoipa::path(
    get,
    path = "/changes/ignore",
    responses(
        (status = 200, description = "Current .changesignore patterns", body = crate::changesignore::ChangesIgnoreResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes"]
)]
pub async fn get_ignore_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<crate::changesignore::ChangesIgnoreResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    log::info!("REST API: GET /changes/ignore");

    let result = tokio::task::spawn_blocking(crate::changesignore::load_full).await;

    match result {
        Ok(response) => {
            log::info!(
                "REST API: GET /changes/ignore — {} patterns from {}",
                response.patterns.len(),
                response.source
            );
            Ok(Json(response))
        }
        Err(e) => {
            log::error!("REST API: Failed to load .changesignore: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to load .changesignore: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// Update the .changesignore file
///
/// Writes new content to the `.changesignore` file in the project root.
/// The raw content is preserved as-is (including comments and formatting).
/// Returns the updated parsed patterns.
#[utoipa::path(
    put,
    path = "/changes/ignore",
    request_body = crate::changesignore::ChangesIgnoreUpdateRequest,
    responses(
        (status = 200, description = ".changesignore updated successfully", body = crate::changesignore::ChangesIgnoreResponse),
        (status = 500, description = "Internal server error", body = ChangesErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["changes"]
)]
pub async fn update_ignore_handler(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<crate::changesignore::ChangesIgnoreUpdateRequest>,
) -> Result<Json<crate::changesignore::ChangesIgnoreResponse>, (StatusCode, Json<ChangesErrorResponse>)> {
    let raw_content = body.raw_content.clone();

    log::info!(
        "REST API: PUT /changes/ignore — {} bytes",
        raw_content.len()
    );

    let result = tokio::task::spawn_blocking(move || {
        crate::changesignore::save_content(&raw_content)
    })
    .await;

    match result {
        Ok(Ok(response)) => {
            log::info!(
                "REST API: PUT /changes/ignore — saved {} patterns",
                response.patterns.len()
            );
            Ok(Json(response))
        }
        Ok(Err(e)) => {
            log::error!("REST API: Failed to save .changesignore: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: e,
                    code: 500,
                }),
            ))
        }
        Err(e) => {
            log::error!("REST API: Failed to save .changesignore (spawn): {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ChangesErrorResponse {
                    error: format!("Failed to save .changesignore: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

//! Task index and caching handler.
//!
//! Responsibility:
//! - Task list endpoint (`GET /history/tasks`)
//! - In-memory + disk cache lifecycle
//! - Filtering, pagination, refresh logic
//! - Single-flight refresh coordination (stampede prevention)
//!
//! ## Concurrency Model
//!
//! The task index is cached at two levels:
//!
//! 1. **In-memory cache** (`TASKS_INDEX_CACHE`): A `parking_lot::RwLock<Option<TaskHistoryListResponse>>`.
//!    - Readers (`RwLock::read()`) are non-blocking and concurrent — multiple handlers
//!      can serve cached data simultaneously.
//!    - Writers (`RwLock::write()`) are exclusive but brief — only held to swap the
//!      `Option` after a scan completes. Never held during I/O.
//!    - Initialized lazily from disk cache on first access via `once_cell::sync::Lazy`.
//!
//! 2. **Disk cache** (`cache.rs`): Best-effort JSON persistence at
//!    `%APPDATA%/jira-dashboard/conversation_history_cache/tasks_index.json`.
//!    Failures are logged but never propagated to callers.
//!
//! ## Single-Flight Refresh
//!
//! The `REFRESH_LOCK` (`tokio::sync::Mutex<()>`) serializes refresh operations:
//!
//! - Only one disk scan can run at a time, regardless of how many concurrent
//!   `?refresh=true` requests arrive.
//! - Callers that arrive while a refresh is in-flight **await** the lock, then
//!   find the freshly-populated cache and return immediately (double-check pattern).
//! - This prevents N concurrent refreshes from each scanning ~180 MB of JSON.
//!
//! ## Ordering Invariant
//!
//! `scan_all_tasks()` returns tasks sorted by `started_at` descending (newest first).
//! This is the canonical ordering. Consumers (e.g. `compute_stats`) should NOT rely
//! on this ordering for correctness — they should use explicit min/max when needed.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::conversation_history::cache;
use crate::conversation_history::summary::scan_all_tasks;
use crate::conversation_history::types::{
    HistoryErrorResponse, HistoryTasksQuery, TaskHistoryListResponse,
};
use crate::state::AppState;

// ============ In-memory cache ============

/// Cached task index (populated from disk or after a full scan).
///
/// ## Why `RwLock<Option<_>>`?
///
/// - `Option`: The cache starts empty if no disk cache exists. `None` = cold cache.
/// - `RwLock`: Multiple readers can serve cached data concurrently (common case).
///   The write lock is only held briefly to swap the `Option` after a scan completes.
/// - `parking_lot::RwLock`: No poisoning (unlike std), predictable fairness.
///
/// ## When reads vs writes occur:
///
/// - **Read**: Every non-refresh request reads the cache. This is the hot path.
/// - **Write**: Only after a successful scan completes (inside `do_refresh()`).
///   The lock is NOT held during the scan itself — only during the pointer swap.
static TASKS_INDEX_CACHE: once_cell::sync::Lazy<RwLock<Option<TaskHistoryListResponse>>> =
    once_cell::sync::Lazy::new(|| {
        // On first access, try loading from disk cache.
        // This runs synchronously during Lazy initialization (before any async context).
        let disk = cache::load_tasks_index();
        RwLock::new(disk)
    });

/// Serializes refresh operations to prevent cache stampede.
///
/// When multiple concurrent requests arrive with `?refresh=true` (or on a cold cache),
/// only the first caller performs the disk scan. Subsequent callers await this lock,
/// then find the freshly-populated cache via the double-check pattern in `get_or_refresh_task_index`.
///
/// Uses `tokio::sync::Mutex` (not `parking_lot`) because it's held across an `.await`
/// boundary (`spawn_blocking`). Holding a sync mutex across `.await` would block the
/// async runtime — `tokio::sync::Mutex` yields the task instead.
static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Monotonically increasing generation counter for the cache.
///
/// Incremented each time `do_refresh()` completes successfully.
/// Used by the double-check pattern: if the generation changed while a caller
/// was waiting on `REFRESH_LOCK`, another refresh already ran and the caller
/// can return the now-fresh cache without scanning again.
///
/// Starts at 0. Wraps at u64::MAX (not a practical concern — would take
/// billions of years at 1 refresh/second).
static CACHE_GENERATION: AtomicU64 = AtomicU64::new(0);

// ============ Shared cache accessor ============

/// Get the cached task index, or refresh it from disk.
///
/// This is the shared entry point used by both `GET /history/tasks` and
/// `GET /history/stats`. It manages the in-memory + disk cache lifecycle
/// with single-flight refresh coordination.
///
/// ## Guarantees
///
/// - Callers always get a consistent `TaskHistoryListResponse` (either cached or freshly scanned).
/// - At most one disk scan runs at a time, regardless of concurrent callers.
/// - If `force_refresh` is false and the cache is populated, returns immediately (no lock contention).
/// - If `force_refresh` is true, the caller awaits the refresh lock. If another refresh
///   just completed, the double-check avoids a redundant scan (callers arriving within
///   the same refresh window get the same fresh data).
pub(crate) async fn get_or_refresh_task_index(
    force_refresh: bool,
) -> Result<TaskHistoryListResponse, (StatusCode, Json<HistoryErrorResponse>)> {
    // Fast path: return cached data if available and not refreshing.
    // This read lock is brief and non-blocking (parking_lot RwLock).
    if !force_refresh {
        let cache = TASKS_INDEX_CACHE.read();
        if let Some(ref cached) = *cache {
            log::debug!("Task index cache hit: {} tasks", cached.total_tasks);
            return Ok(cached.clone());
        }
    }

    // Snapshot the generation BEFORE acquiring the lock.
    // If it changes while we wait, another refresh completed and we can skip ours.
    let gen_before = CACHE_GENERATION.load(Ordering::Acquire);

    // Slow path: need to refresh. Serialize through REFRESH_LOCK.
    let _guard = REFRESH_LOCK.lock().await;

    // Double-check: another caller may have completed a refresh while we waited.
    let gen_after = CACHE_GENERATION.load(Ordering::Acquire);
    if gen_after != gen_before {
        // Generation changed → another refresh ran while we were waiting.
        // Return the now-fresh cache. This applies to BOTH cold-cache and
        // force_refresh callers — in a stampede of N refresh requests,
        // only the FIRST one scans, the remaining N-1 get its result.
        let cache = TASKS_INDEX_CACHE.read();
        if let Some(ref cached) = *cache {
            log::debug!(
                "Task index: piggy-backed on concurrent refresh (gen {} → {}, {} tasks)",
                gen_before,
                gen_after,
                cached.total_tasks
            );
            return Ok(cached.clone());
        }
        // Edge case: generation changed but cache is None.
        // This shouldn't happen (do_refresh sets Some before incrementing gen),
        // but if it does, fall through to scan.
        log::warn!("Task index: generation changed but cache is empty — will scan");
    }

    // For cold-cache (!force_refresh) with no generation change, check if cache
    // was populated by the Lazy initializer (disk load) while we waited.
    if !force_refresh {
        let cache = TASKS_INDEX_CACHE.read();
        if let Some(ref cached) = *cache {
            log::debug!(
                "Task index: cache populated while waiting ({} tasks)",
                cached.total_tasks
            );
            return Ok(cached.clone());
        }
    }

    // We are the designated scanner. Run the scan.
    do_refresh().await
}

/// Perform the actual disk scan, update caches, and return the result.
///
/// Called only while holding `REFRESH_LOCK` — guaranteed single-threaded entry.
async fn do_refresh(
) -> Result<TaskHistoryListResponse, (StatusCode, Json<HistoryErrorResponse>)> {
    log::info!("Task index: starting full disk scan");

    // Run scan in blocking context (heavy filesystem I/O — reads ~180 MB of JSON).
    // spawn_blocking moves this to the Tokio blocking thread pool, keeping the
    // async runtime responsive.
    let result = tokio::task::spawn_blocking(|| {
        let start = std::time::Instant::now();
        let response = scan_all_tasks();
        let elapsed = start.elapsed();
        log::info!(
            "Task index scan complete: {} tasks in {:.1}s",
            response.total_tasks,
            elapsed.as_secs_f64()
        );
        response
    })
    .await;

    match result {
        Ok(response) => {
            log::info!(
                "Task index: scanned {} tasks ({} messages, {} tool calls, {:.1} MB api_history)",
                response.total_tasks,
                response.total_messages,
                response.total_tool_calls,
                response.total_api_history_bytes as f64 / 1024.0 / 1024.0
            );

            // Update in-memory cache. The write lock is held only for the duration
            // of the pointer swap — not during I/O.
            *TASKS_INDEX_CACHE.write() = Some(response.clone());

            // Bump the generation counter AFTER updating the cache.
            // Ordering: Release ensures the cache write is visible to other threads
            // that subsequently read the generation with Acquire ordering.
            let new_gen = CACHE_GENERATION.fetch_add(1, Ordering::Release) + 1;
            log::debug!("Task index: cache generation bumped to {}", new_gen);

            // Persist to disk (best-effort — failures are logged, never propagated).
            cache::save_tasks_index(&response);

            Ok(response)
        }
        Err(e) => {
            log::error!(
                "Task index: spawn_blocking panicked during scan: {}",
                e
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(HistoryErrorResponse {
                    error: format!("Failed to scan task histories: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

// ============ Handler ============

/// List all Cline task conversation histories
///
/// Scans the Cline task storage directory and returns a summary for each task,
/// including message counts, tool usage breakdown, model info, and file audit data.
///
/// Each task is parsed from its `api_conversation_history.json`, `task_metadata.json`,
/// and `ui_messages.json` files under `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/`.
///
/// Results are cached in memory and persisted to disk.
/// Pass `?refresh=true` to force a full re-scan from disk.
/// Supports optional `?model=`, `?limit=`, `?offset=` query parameters.
#[utoipa::path(
    get,
    path = "/history/tasks",
    params(HistoryTasksQuery),
    responses(
        (status = 200, description = "List of Cline task conversation history summaries", body = TaskHistoryListResponse),
        (status = 500, description = "Internal server error", body = HistoryErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["history", "tool"]
)]
pub async fn list_history_tasks_handler(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<HistoryTasksQuery>,
) -> Result<Json<TaskHistoryListResponse>, (StatusCode, Json<HistoryErrorResponse>)> {
    let force_refresh = params.refresh.unwrap_or(false);

    let response = get_or_refresh_task_index(force_refresh).await?;

    log::info!(
        "REST API: GET /history/tasks — returning {} tasks (refresh={})",
        response.total_tasks,
        force_refresh
    );

    Ok(Json(apply_filters(response, &params)))
}

/// Apply optional query filters (model, limit, offset) to the response.
///
/// Operates on a clone of the cached data — does not mutate the cache.
fn apply_filters(
    mut response: TaskHistoryListResponse,
    params: &HistoryTasksQuery,
) -> TaskHistoryListResponse {
    // Filter by model if specified
    if let Some(ref model) = params.model {
        response.tasks.retain(|t| {
            t.model_id
                .as_deref()
                .map(|m| m.contains(model.as_str()))
                .unwrap_or(false)
        });
        response.total_tasks = response.tasks.len();
    }

    // Apply offset
    let offset = params.offset.unwrap_or(0);
    if offset > 0 && offset < response.tasks.len() {
        response.tasks = response.tasks.split_off(offset);
    } else if offset >= response.tasks.len() {
        response.tasks.clear();
    }

    // Apply limit
    if let Some(limit) = params.limit {
        response.tasks.truncate(limit);
    }

    response
}

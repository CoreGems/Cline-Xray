# SHADOW_GIT_SUBTASK_FIX — Subtask-Level Diffs in Changes Tab

## Problem Statement

The **Changes → Tasks** tab currently supports two diff granularities:

| Granularity | API | What it shows |
|-------------|-----|---------------|
| **Full Task Diff** | `GET /changes/tasks/:taskId/diff` | First checkpoint parent → last checkpoint (entire task) |
| **Step Diff** | `GET /changes/tasks/:taskId/steps/:index/diff` | One commit → its parent (single checkpoint) |

**Missing middle tier: Subtask Diff** — when a Cline task has multiple phases (initial task + feedback-driven subtasks), we want to see the diff for just ONE subtask phase, not the entire task and not individual micro-steps.

**Example from the Fibonacci task:**

```
Full Task Diff: created fibonacci.py + fibonacci.js (everything)
├── Subtask #0 Diff: created fibonacci.py only      ← NEW
├── Subtask #1 Diff: created fibonacci.js only       ← NEW
│
├── Step 1 Diff: initial snapshot
├── Step 2 Diff: fibonacci.py created
├── Step 3 Diff: completion checkpoint
├── Step 4 Diff: feedback received checkpoint
├── Step 5 Diff: fibonacci.js created
└── Step 6 Diff: completion checkpoint
```

---

## Can We Do This? — Feasibility Assessment

### ✅ YES — with timestamp-based step-to-subtask mapping

Both systems share the same `task_id` (epoch milliseconds) and have timestamps:

| System | Data | Timestamp Source |
|--------|------|------------------|
| **Subtask Detection** (conversation_history) | `SubtaskEntry.timestamp` | `ui_messages.json` → `ts` field (epoch ms → ISO 8601) |
| **Shadow Git Steps** | `CheckpointStep.timestamp` | Git commit author date (`%aI` format, ISO 8601) |

**The bridge:** Map each subtask's time window to a range of checkpoint steps, then compute `git diff` from the first step in that range to the last step.

### Mapping Algorithm

```
Given:
  subtasks = [S0(t=13:54:12), S1(t=13:55:22)]     ← from conversation_history
  steps    = [C1(t=13:54:13), C2(t=13:54:30),      ← from shadow git
              C3(t=13:54:43), C4(t=13:55:22),
              C5(t=13:55:30), C6(t=13:55:34)]

Mapping:
  S0 window: 13:54:12 → 13:55:22 (exclusive)
    → steps: C1, C2, C3  (timestamps within this window)
    → diff: C1^..C3  (parent of first step → last step)

  S1 window: 13:55:22 → ∞
    → steps: C4, C5, C6
    → diff: C3..C6   (last step of prev subtask → last step of this subtask)
```

**Key insight:** The subtask diff for subtask N is computed as:
- **from_ref:** last step of subtask N-1 (or first step's parent for subtask 0)
- **to_ref:** last step of subtask N

This gives a clean diff showing exactly what changed during that subtask phase.

---

## Architecture: Where Things Live

### Current Architecture

```
┌─────────────────────────────────────┐     ┌──────────────────────────────────┐
│ conversation_history/               │     │ shadow_git/                      │
│                                     │     │                                  │
│ • subtasks.rs → SubtasksResponse    │     │ • discovery.rs → steps, diffs    │
│   (parses ui_messages.json)         │     │   (parses git commits)           │
│                                     │     │                                  │
│ • handlers/subtasks.rs              │     │ • handlers.rs                    │
│   GET /history/tasks/:id/subtasks   │     │   GET /changes/tasks/:id/diff    │
│                                     │     │   GET /changes/.../steps/:i/diff │
└─────────────────────────────────────┘     └──────────────────────────────────┘
         ↓ subtask timestamps                      ↓ step commits
         └──────────── BRIDGE NEEDED ──────────────┘
```

### Proposed Architecture

The **subtask diff** feature sits at the intersection of both modules. Two design options:

#### Option A: New endpoint in `shadow_git` that calls into `conversation_history` (RECOMMENDED)

```
GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff?workspace=<id>
```

Flow:
1. Handler in `shadow_git/handlers.rs` receives request
2. Calls `conversation_history::subtasks::parse_task_subtasks(task_id)` to get subtask boundaries
3. Calls `shadow_git::discovery::list_steps_for_task(task_id, ...)` to get checkpoint steps
4. Maps subtask timestamp windows to step ranges
5. Computes `git diff` between the boundary steps
6. Returns `DiffResult` (same type as existing diffs)

**Why this is better:** The diff computation lives in `shadow_git` where all git operations are. It just imports subtask boundary data from `conversation_history`.

#### Option B: New endpoint in `conversation_history` that calls into `shadow_git`

This is less natural — conversation_history shouldn't know about git diffs.

---

## Proposed API

### New Endpoint

```
GET /changes/tasks/{task_id}/subtasks/{subtask_index}/diff?workspace=<id>&exclude=<pattern>
```

**Parameters:**
- `task_id` — The Cline task ID (epoch milliseconds)
- `subtask_index` — 0-based subtask index (0 = initial task, 1+ = feedback subtasks)
- `workspace` — Workspace ID (required, same as existing diff endpoints)
- `exclude` — Optional pathspec exclusion patterns (same as task diff)

**Response:** `DiffResult` (existing type — `files`, `patch`, `from_ref`, `to_ref`)

**Error cases:**
- 404 if task has no checkpoint commits
- 400 if subtask_index is out of range
- 400 if no steps fall within the subtask's time window

### Also: Subtask-annotated steps listing

Enhance the existing steps response to include subtask mapping:

```
GET /changes/tasks/{task_id}/steps?workspace=<id>&annotate_subtasks=true
```

This would add a `subtask_index: Option<usize>` field to each `CheckpointStep`, showing which subtask each step belongs to. Useful for UI grouping.

---

## Proposed Types

### New types needed

```rust
/// Query parameters for subtask diff
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SubtaskDiffQuery {
    /// Workspace ID (required)
    pub workspace: String,
    /// Pathspec exclusion patterns
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Path parameters for subtask diff
#[derive(Debug, Deserialize)]
pub struct SubtaskDiffPath {
    pub task_id: String,
    pub subtask_index: usize,
}
```

### Enhanced `CheckpointStep` (optional enhancement)

```rust
pub struct CheckpointStep {
    // ... existing fields ...
    /// Which subtask this step belongs to (if annotated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtask_index: Option<usize>,
}
```

---

## Implementation Plan

### Step-by-step (ordered by dependency)

#### 1. Bridge function: `map_subtasks_to_steps()`

New function in `shadow_git/discovery.rs` (or a new `shadow_git/subtask_bridge.rs`):

```rust
use crate::conversation_history::subtasks::parse_task_subtasks;
use crate::conversation_history::types::SubtasksResponse;

/// Map subtask time boundaries to step ranges
/// Returns Vec of (subtask_index, first_step_idx, last_step_idx) — 0-based into steps array
pub fn map_subtasks_to_steps(
    subtasks: &SubtasksResponse,
    steps: &[CheckpointStep],
) -> Vec<(usize, usize, usize)> {
    let mut mappings = Vec::new();

    for (si, subtask) in subtasks.subtasks.iter().enumerate() {
        let subtask_start = &subtask.timestamp;
        let subtask_end = subtasks.subtasks
            .get(si + 1)
            .map(|next| next.timestamp.as_str());

        // Find steps whose timestamps fall in [subtask_start, subtask_end)
        let mut first_step: Option<usize> = None;
        let mut last_step: Option<usize> = None;

        for (i, step) in steps.iter().enumerate() {
            let in_range = step.timestamp >= *subtask_start
                && subtask_end.map_or(true, |end| step.timestamp < end);
            if in_range {
                if first_step.is_none() {
                    first_step = Some(i);
                }
                last_step = Some(i);
            }
        }

        if let (Some(first), Some(last)) = (first_step, last_step) {
            mappings.push((si, first, last));
        }
    }

    mappings
}
```

#### 2. Subtask diff computation: `get_subtask_diff()`

New function in `shadow_git/discovery.rs`:

```rust
pub fn get_subtask_diff(
    task_id: &str,
    subtask_index: usize,
    git_dir: &PathBuf,
    excludes: &[String],
) -> Result<DiffResult, String> {
    // 1. Get subtask boundaries from conversation_history
    let subtasks = parse_task_subtasks(task_id)
        .ok_or_else(|| format!("No subtask data for task '{}'", task_id))?;

    if subtask_index >= subtasks.total_subtasks {
        return Err(format!(
            "Subtask index {} out of range (task has {} subtasks)",
            subtask_index, subtasks.total_subtasks
        ));
    }

    // 2. Get checkpoint steps
    let steps = list_steps_for_task(task_id, "unknown", git_dir);
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

    // 5. Run git diff (reuse existing diff machinery)
    // ... same pattern as get_task_diff() ...
}
```

#### 3. Handler: `subtask_diff_handler()`

New handler in `shadow_git/handlers.rs`:

```rust
/// Get the diff for a single subtask phase
///
/// Computes the diff for a specific subtask within a task by mapping
/// conversation history feedback boundaries to checkpoint commit ranges.
/// Subtask #0 is the initial task, #1+ are feedback-driven subtasks.
#[utoipa::path(
    get,
    path = "/changes/tasks/{task_id}/subtasks/{subtask_index}/diff",
    params(SubtaskDiffPath, SubtaskDiffQuery),
    responses(
        (status = 200, body = DiffResult),
        (status = 400, body = ChangesErrorResponse),
        (status = 404, body = ChangesErrorResponse),
    ),
    tags = ["changes", "tool"]
)]
pub async fn subtask_diff_handler(...) -> Result<Json<DiffResult>, ...> {
    // Similar pattern to task_diff_handler
}
```

#### 4. Route registration

In `server.rs`:
```rust
.route("/changes/tasks/:task_id/subtasks/:subtask_index/diff",
       get(shadow_git::subtask_diff_handler))
```

#### 5. Frontend API

In `src/lib/tabs/changes/api.ts`:
```typescript
export async function fetchSubtaskDiff(
  taskId: string,
  subtaskIndex: number,
  workspaceId: string,
  excludes: string[] = []
): Promise<DiffResult> {
  // GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff?workspace=<id>
}
```

#### 6. Frontend UI

In `TaskListSubtab.svelte`, add subtask diff buttons alongside the existing "Full Diff" and step "Diff" buttons:

```
┌─────────────────────────────────────────────────────────────────────┐
│ Task 1770576852118  │ 6 steps │ 2 files │ Latest  │ ▸ Full Diff   │
├─────────────────────────────────────────────────────────────────────┤
│ ┌─ 🔀 Subtask Diffs ──────────────────────────────────────────┐    │
│ │  #0 Initial Task: "write python script…"       ▸ Diff (2 files)│ │
│ │  #1 Feedback: "now write javascript…"          ▸ Diff (1 file) │ │
│ └──────────────────────────────────────────────────────────────┘    │
│                                                                     │
│ ┌─ Checkpoint Steps (6) ──────────────────────────────────────┐    │
│ │  Step 6 │ a1b2c3d4 │ 1 file  │ 13:55:34 │ ▸ Diff │ ST#1   │    │
│ │  Step 5 │ e5f6g7h8 │ 1 file  │ 13:55:30 │ ▸ Diff │ ST#1   │    │
│ │  Step 4 │ i9j0k1l2 │ 0 files │ 13:55:22 │ ▸ Diff │ ST#1   │    │
│ │  Step 3 │ m3n4o5p6 │ 0 files │ 13:54:43 │ ▸ Diff │ ST#0   │    │
│ │  Step 2 │ q7r8s9t0 │ 1 file  │ 13:54:30 │ ▸ Diff │ ST#0   │    │
│ │  Step 1 │ u1v2w3x4 │ 1 file  │ 13:54:13 │ ▸ Diff │ ST#0   │    │
│ └──────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow Diagram

```
User clicks "Subtask #1 Diff" in Changes tab
    │
    ▼
Frontend: fetchSubtaskDiff(taskId, 1, workspaceId)
    │
    ▼
GET /changes/tasks/1770576852118/subtasks/1/diff?workspace=4184916832
    │
    ▼
shadow_git::handlers::subtask_diff_handler
    │
    ├──► conversation_history::subtasks::parse_task_subtasks("1770576852118")
    │    │
    │    ├──► Reads ui_messages.json from Cline globalStorage
    │    └──► Returns SubtasksResponse { subtasks: [S0, S1] }
    │
    ├──► shadow_git::discovery::list_steps_for_task("1770576852118", ...)
    │    │
    │    └──► git --git-dir <path> log --all --pretty=format:%H|%s|%aI
    │         Returns [C1, C2, C3, C4, C5, C6] (chronological)
    │
    ├──► map_subtasks_to_steps(&subtasks, &steps)
    │    │
    │    └──► S1 window: 13:55:22 → ∞
    │         Steps in window: C4(idx=3), C5(idx=4), C6(idx=5)
    │         from_ref = C3.hash (last step of prev subtask)
    │         to_ref = C6.hash (last step of this subtask)
    │
    └──► git --git-dir <path> diff <C3.hash> <C6.hash>
         │
         └──► Returns DiffResult { files: [...], patch: "..." }
```

---

## Edge Cases & Challenges

### 1. Timestamp Precision & Ordering

**Challenge:** Git commit timestamps have second-level precision. Subtask timestamps from `ui_messages.json` have millisecond precision. Two events could have the same second-level timestamp.

**Mitigation:** Use `>=` for start boundary and `<` for end boundary. If a step timestamp exactly equals a subtask boundary, it belongs to the NEW subtask (which is correct — the feedback checkpoint is part of the feedback phase).

### 2. Clock Skew Between UI and Git

**Challenge:** The `ui_messages.json` timestamp comes from the Cline VS Code extension (JavaScript `Date.now()`). The git commit timestamp comes from `git commit` (system clock). These should be identical (same machine) but could differ by a few milliseconds.

**Mitigation:** Add a small tolerance window (e.g., ±2 seconds) when mapping timestamps. This is probably unnecessary for same-machine operations but is a safety net.

### 3. Empty Subtask Ranges

**Challenge:** A subtask might have no checkpoint steps if:
- The feedback was provided but no tool actions happened yet
- The checkpoints were created before/after the feedback window

**Response:** Return a 400 error with "No checkpoint steps found within subtask's time window". The UI should show "No changes" gracefully.

### 4. First Subtask's Parent Commit

**Challenge:** The first subtask's diff starts from the parent of the first checkpoint (`C1^`). This might fail if C1 is the root commit (no parent).

**Existing solution:** `get_task_diff()` already handles this with a `diff-tree` fallback for root commits. Reuse the same pattern.

### 5. Task ID Mismatch

**Challenge:** The `task_id` in conversation_history (epoch ms directory name) and the `task_id` in shadow git (extracted from commit subjects) must match. They should — both come from the same Cline task creation timestamp.

**Verification:** Log both IDs at debug level and compare. If they don't match, the subtask endpoint returns 404.

### 6. Multiple Workspaces for Same Task

**Challenge:** A task could theoretically exist in multiple workspaces (e.g., if the user switched VS Code workspaces mid-task). In practice this doesn't happen — Cline creates checkpoints in the workspace where the task started.

**Mitigation:** The `workspace` query param already scopes to one workspace.

### 7. No Conversation History Available

**Challenge:** The task exists in shadow git but the conversation history files (`ui_messages.json`) have been deleted or are inaccessible.

**Response:** Return 404 with "Subtask detection data not available for this task". The full task diff and step diffs remain functional.

---

## Performance Considerations

### What's expensive?
1. **`parse_task_subtasks()`** — reads and parses `ui_messages.json` + `api_conversation_history.json`. These are small files. ~1-5ms.
2. **`list_steps_for_task()`** — runs `git log` + counts files per step. ~50-200ms depending on number of steps.
3. **`git diff from_ref to_ref`** — runs git diff between two commits. ~10-100ms depending on diff size.

### Total expected latency: ~100-300ms (acceptable for on-demand computation)

### Caching Strategy

- Subtask-to-step mappings could be cached in-memory (same pattern as `STEPS_CACHE`), keyed by `workspace_id:task_id`.
- The mapping only changes if new checkpoints are created (which invalidates the steps cache anyway).
- Diff results are NOT cached (same as current step/task diffs) — they depend on `exclude` patterns.

---

## Files to Modify

### Backend (Rust)

| File | Change | Effort |
|------|--------|--------|
| `shadow_git/discovery.rs` | Add `map_subtasks_to_steps()` + `get_subtask_diff()` | Medium |
| `shadow_git/handlers.rs` | Add `subtask_diff_handler()` + query/path types | Medium |
| `shadow_git/types.rs` | Optional: Add `subtask_index` to `CheckpointStep` | Small |
| `shadow_git/mod.rs` | Re-export new handler | Trivial |
| `server.rs` | Add route for subtask diff | Trivial |
| `openapi.rs` | Register new endpoint | Trivial |

### Frontend (Svelte/TypeScript)

| File | Change | Effort |
|------|--------|--------|
| `src/lib/tabs/changes/api.ts` | Add `fetchSubtaskDiff()` | Small |
| `src/lib/tabs/changes/types.ts` | No new types needed (reuses `DiffResult`) | None |
| `src/lib/tabs/changes/TaskListSubtab.svelte` | Add subtask diff section in expanded task panel | Medium |

### Cross-module dependency

```
shadow_git/discovery.rs
  └──► use crate::conversation_history::subtasks::parse_task_subtasks;
  └──► use crate::conversation_history::types::SubtasksResponse;
```

This is the **only cross-module dependency**. It's clean — `shadow_git` imports from `conversation_history` but not vice versa.

---

## UI Design Detail

### Option A: Subtask Diff Panel (inline, below task row)

Add a new expandable section between the "Full Diff" panel and the "Steps" panel:

```
┌── Task Row ──────────────────────────────────────────────────────┐
│ 1770576852118  │ 6 steps │ 2 files │  [▸ Full Diff] [🔀 Subtasks]│
├──────────────────────────────────────────────────────────────────┤
│ 🔀 Subtask Diffs                                                 │
│                                                                   │
│ ┌─ #0 · Initial Task ─────────────────────────────┐              │
│ │ "write simple python script…"                    │              │
│ │ Steps: C1–C3 · 1 file · 13:54:12–13:55:22      │  [▸ Diff]   │
│ └──────────────────────────────────────────────────┘              │
│       ↓                                                           │
│ ┌─ #1 · Feedback ─────────────────────────────────┐              │
│ │ "now write java script doing the same…"          │              │
│ │ Steps: C4–C6 · 1 file · 13:55:22–13:55:34      │  [▸ Diff]   │
│ └──────────────────────────────────────────────────┘              │
├──────────────────────────────────────────────────────────────────┤
│ Checkpoint Steps (6)                                              │
│ ...                                                               │
└──────────────────────────────────────────────────────────────────┘
```

### Option B: Subtask badges on step rows

Instead of a separate section, annotate each step row with its subtask index. Add a filter to show only steps for a specific subtask.

```
Step 6 │ a1b2c3d4 │ 1 file │ 13:55:34 │ [ST#1] │ ▸ Diff
Step 5 │ e5f6g7h8 │ 1 file │ 13:55:30 │ [ST#1] │ ▸ Diff
Step 4 │ i9j0k1l2 │ 0 files│ 13:55:22 │ [ST#1] │ ▸ Diff
Step 3 │ m3n4o5p6 │ 0 files│ 13:54:43 │ [ST#0] │ ▸ Diff
Step 2 │ q7r8s9t0 │ 1 file │ 13:54:30 │ [ST#0] │ ▸ Diff
Step 1 │ u1v2w3x4 │ 1 file │ 13:54:13 │ [ST#0] │ ▸ Diff
```

**Recommendation:** Do both — Option A for the subtask-level aggregate diffs, and Option B as visual annotation on existing steps.

---

## Diff Hierarchy Summary

After implementation, the Changes tab will support **three levels** of diff granularity:

```
┌─────────────────────────────────────────────────────────┐
│                    FULL TASK DIFF                        │
│              (first commit^ → last commit)              │
│                                                         │
│  ┌──────────────────────┬──────────────────────┐        │
│  │   SUBTASK #0 DIFF    │   SUBTASK #1 DIFF    │  ← NEW│
│  │  (C1^ → C3)          │  (C3 → C6)           │       │
│  │                      │                       │       │
│  │  ┌────┬────┬────┐   │  ┌────┬────┬────┐    │       │
│  │  │ S1 │ S2 │ S3 │   │  │ S4 │ S5 │ S6 │    │       │
│  │  │diff│diff│diff│   │  │diff│diff│diff│    │       │
│  │  └────┴────┴────┘   │  └────┴────┴────┘    │       │
│  └──────────────────────┴──────────────────────┘        │
└─────────────────────────────────────────────────────────┘
```

This gives users exactly the right level of detail:
- **"What did this entire task change?"** → Full Task Diff
- **"What did the Python phase vs JavaScript phase each produce?"** → Subtask Diff
- **"What exactly happened in this one checkpoint?"** → Step Diff

---

## Implementation Priority & Effort Estimate

| Component | Effort | Priority |
|-----------|--------|----------|
| `map_subtasks_to_steps()` bridge function | 1-2 hours | P0 — core logic |
| `get_subtask_diff()` in discovery.rs | 1 hour | P0 — reuses existing diff patterns |
| `subtask_diff_handler()` in handlers.rs | 1 hour | P0 — follows existing handler patterns |
| Route + OpenAPI registration | 15 min | P0 — mechanical |
| Frontend `fetchSubtaskDiff()` API | 15 min | P0 — mechanical |
| UI: Subtask diff section in task panel | 2-3 hours | P1 — user-facing |
| UI: Subtask badges on step rows | 1 hour | P2 — nice to have |
| Enhanced steps endpoint with subtask annotation | 1 hour | P2 — optional |
| Caching for subtask-to-step mappings | 30 min | P3 — optimization |

**Total estimated effort: ~8-10 hours**

---

## Summary

| Question | Answer |
|----------|--------|
| Can we do subtask diffs? | **Yes** — timestamp-based mapping between subtask boundaries and checkpoint steps |
| Is it complex? | **Moderate** — the bridge logic is new but reuses existing diff infrastructure |
| Cross-module coupling? | **Minimal** — one import: `shadow_git` reads from `conversation_history::subtasks` |
| New API endpoint? | `GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff?workspace=<id>` |
| Same response type? | **Yes** — returns `DiffResult` (same as task/step diffs) |
| Breaking changes? | **None** — purely additive |
| Performance? | **Fine** — ~100-300ms per request, cacheable |

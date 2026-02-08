# SUBTASK_FI — Subtask Detection & Integration Reasoning

## Problem Statement

When reading Cline conversation history for tasks, we correctly identify the **first prompt** (the initial `<task>` message). However, within `ui_messages.json`, there are additional user prompts wrapped in `<feedback>` tags that represent **follow-up instructions** — effectively breaking a single task into multiple subtasks.

**Example from `task_sample/ui_messages.json`:**
```json
{
    "type": "text",
    "text": "<feedback>\nnow write java script doing the same . do not test\n</feedback>"
}
```

These `<feedback>` prompts are essentially sub-prompts that steer the task in a new direction, creating logical subtask boundaries within a single Cline task session.

## What Are Subtasks?

A Cline "task" is a single session identified by a timestamp-based task ID. Within that session:

- **Subtask #0** — The initial task prompt (e.g., "write simple python script to generate fibonacci number")
- **Subtask #1+** — Each `<feedback>` message from the user (e.g., "now write java script doing the same")

Each subtask represents a distinct phase of work with its own prompt, its own set of API calls, tool uses, and file changes.

## Detection Strategy

### Source: `ui_messages.json`

The `ui_messages.json` file in each task's Cline globalStorage directory contains the full conversation timeline. We scan for two patterns:

1. **Initial task prompt**: First message with `type: "say"` and `say: "task"` — this is subtask #0
2. **Feedback prompts**: Messages with `type: "say"` and `say: "user_feedback"` — these are subtask #1, #2, etc.

Additionally, we look inside `api_conversation_history.json` for content blocks containing `<feedback>...</feedback>` tags within `"text"` type content blocks of `"human"` role messages. This serves as a cross-reference.

### Timestamp Extraction

Each subtask gets a timestamp from the `ts` field of its `ui_messages.json` entry. This timestamp is critical for:
- Ordering subtasks chronologically
- Mapping subtasks to checkpoint commit ranges (for per-subtask diffs)

## Architecture

### Backend (Rust)

#### New Module: `conversation_history/subtasks.rs`
- `parse_task_subtasks(task_id) -> Option<SubtasksResponse>`
- Finds the task's `ui_messages.json` in Cline globalStorage
- Scans messages for task prompt and user_feedback entries
- Extracts prompt text and timestamps
- Returns structured `SubtasksResponse`

#### Types: `conversation_history/types.rs`
```rust
pub struct SubtaskEntry {
    pub index: usize,           // 0-based
    pub prompt: String,         // The user's instruction text
    pub timestamp: String,      // ISO timestamp from ui_messages.json
    pub is_initial: bool,       // true for subtask #0
}

pub struct SubtasksResponse {
    pub task_id: String,
    pub subtasks: Vec<SubtaskEntry>,
    pub total_subtasks: usize,
}
```

#### Handler: `conversation_history/handlers/subtasks.rs`
- `GET /history/tasks/:taskId/subtasks` — Returns all detected subtasks for a task

### Shadow Git Bridge

#### `shadow_git/discovery.rs` — New Functions
- `map_subtasks_to_steps(subtasks, steps)` — Maps subtask time windows to checkpoint step ranges
- `get_subtask_diff(task_id, subtask_index, workspace_id, git_dir, excludes)` — Computes git diff for a specific subtask phase

The bridge works by:
1. Getting subtask timestamps from `conversation_history::subtasks`
2. Getting checkpoint steps from `shadow_git::discovery`
3. For each subtask, finding which steps fall within its time window (from subtask start to next subtask start)
4. Computing `git diff` across that step range

#### `shadow_git/handlers.rs` — New Handler
- `GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff?workspace=<id>` — Returns the diff for a specific subtask phase

### Frontend (Svelte)

#### History Tab: `TaskDetailView.svelte`
- Displays subtask list with prompts, timestamps, and initial/feedback labels
- Shows subtask count in task detail header

#### Changes Tab: `TaskListSubtab.svelte`
- New "Subtasks" button on each task row (teal-colored, next to "Full Diff")
- Expandable panel showing all subtasks with:
  - Index badge (numbered circle)
  - 🎯 Initial Task / 💬 Feedback labels
  - Prompt text (truncated with line-clamp)
  - Timestamp
  - Per-subtask "Diff" button that loads the git diff for that subtask's time window

#### API Functions
- `history/api.ts`: `fetchTaskSubtasks(taskId)` — GET /history/tasks/:taskId/subtasks
- `changes/api.ts`: `fetchSubtaskDiff(taskId, subtaskIndex, workspaceId, excludes)` — GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff

## Data Flow

```
ui_messages.json
    ↓ parse
SubtasksResponse { subtasks: [#0 initial, #1 feedback, ...] }
    ↓ map timestamps to git commits
CheckpointStep[] (filtered by time window)
    ↓ git diff between boundary commits
DiffResult { files, patch, fromRef, toRef }
    ↓ render in UI
Subtask panel with per-subtask diffs
```

## UI Design

The subtask panel uses **teal** as its accent color (distinguishing it from purple=Full Diff, amber=Step Diff, blue=Steps):

```
┌─────────────────────────────────────────────┐
│ SUBTASKS (2) · Task: 1770576852118      [✕] │
├─────────────────────────────────────────────┤
│ [0] 🎯 Initial Task    2/8/26 1:54 PM      │
│     write simple python script to generate  │
│     fibonacci number (do not test)    [Diff] │
├─────────────────────────────────────────────┤
│ [1] 💬 Feedback #1      2/8/26 1:55 PM     │
│     now write java script doing the same.   │
│     do not test                       [Diff] │
│   ┌─ Diff ──────────────────────────────┐   │
│   │ a1b2c3d4 → e5f6g7h8 · 1 file       │   │
│   │ [dark diff patch content...]         │   │
│   │ ▸ Files changed (1)                 │   │
│   └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## Files Modified

### Backend (Rust)
| File | Change |
|------|--------|
| `src-tauri/src/conversation_history/types.rs` | Added `SubtaskEntry`, `SubtasksResponse` |
| `src-tauri/src/conversation_history/subtasks.rs` | **NEW** — Subtask parsing from ui_messages.json |
| `src-tauri/src/conversation_history/handlers/subtasks.rs` | **NEW** — GET /history/tasks/:taskId/subtasks |
| `src-tauri/src/conversation_history/mod.rs` | Added `pub mod subtasks;` + re-exports |
| `src-tauri/src/conversation_history/handlers/mod.rs` | Added `pub mod subtasks;` + re-exports |
| `src-tauri/src/shadow_git/discovery.rs` | Added `map_subtasks_to_steps()`, `get_subtask_diff()` |
| `src-tauri/src/shadow_git/handlers.rs` | Added `subtask_diff_handler`, `SubtaskDiffQuery`, `SubtaskDiffPath` |
| `src-tauri/src/shadow_git/mod.rs` | Re-exported `get_subtask_diff` |
| `src-tauri/src/server.rs` | Added route for subtask diff |
| `src-tauri/src/openapi.rs` | Registered subtask endpoints in OpenAPI spec |

### Frontend (Svelte/TypeScript)
| File | Change |
|------|--------|
| `src/lib/tabs/history/types.ts` | Added `SubtaskEntry`, `SubtasksResponse` types |
| `src/lib/tabs/history/api.ts` | Added `fetchTaskSubtasks()` |
| `src/lib/tabs/history/TaskDetailView.svelte` | Shows subtask list in task detail |
| `src/lib/tabs/changes/api.ts` | Added `fetchSubtaskDiff()` |
| `src/lib/tabs/changes/TaskListSubtab.svelte` | Subtasks button, panel, per-subtask diff UI |

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/history/tasks/:taskId/subtasks` | List subtasks (prompts + timestamps) |
| GET | `/changes/tasks/:taskId/subtasks/:index/diff?workspace=<id>` | Git diff for subtask phase |

## Edge Cases

1. **Single-prompt task** — Returns 1 subtask (index 0, the initial task). No feedback entries.
2. **No ui_messages.json** — Returns 404 / empty. Task may have been deleted.
3. **Subtask with no checkpoint steps** — The time window may not contain any git commits. Returns error explaining no steps in window.
4. **First subtask diff** — Uses `firstCommit^` as the from-ref (parent of first checkpoint).
5. **Last subtask** — Time window extends to infinity (no upper bound).

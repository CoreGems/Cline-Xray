# LATEST_HOWTO.md — "Latest" Unified View Assessment

> **Goal**: A single "Latest" tab/view that merges the **latest task or subtask's diff + changed files** with **its prompt text** — collapsing today's multi-step drill-down (workspace → task → subtasks → diff) into one auto-focused, zero-click view.

---

## 1. What Exists Today

### 1.1 Two Separate Module Pillars

| Pillar | Module | Root Path | What it knows |
|--------|--------|-----------|---------------|
| **Conversation History** | `src-tauri/src/conversation_history/` | `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/<taskId>/` | Task prompts, subtask prompts (`ui_messages.json`), message ranges, tool call counts, model info, files-in-context |
| **Shadow Git (Changes)** | `src-tauri/src/shadow_git/` | `%APPDATA%/Code/User/globalStorage/saoudrizwan.claude-dev/checkpoints/<workspaceId>/` | Checkpoint commits, step-level diffs, task-level diffs, subtask-level diffs (unified patches + file stats) |

### 1.2 Existing APIs That Would Feed "Latest"

| Endpoint | Returns | Notes |
|----------|---------|-------|
| `GET /history/tasks` | List of all tasks sorted newest-first | Has `task_prompt` (truncated 200 chars) |
| `GET /history/tasks/:taskId/subtasks` | All subtask entries for a task | Each has `prompt`, `timestamp`, `subtask_index`, `is_initial_task`, message range, tool counts |
| `GET /changes/workspaces` | All checkpoint workspaces | Need to find the workspace for the latest task |
| `GET /changes/tasks?workspace=<id>` | Tasks in a workspace sorted by `last_modified` | Gives `task_id`, `steps`, `files_changed` |
| `GET /changes/tasks/:taskId/subtasks/:idx/diff?workspace=<id>` | Unified diff + file list for one subtask | The subtask diff is already the cross-module bridge — it maps conversation subtask time windows to checkpoint commit ranges |
| `GET /changes/tasks/:taskId/diff?workspace=<id>` | Full task diff (first commit → last commit) | Cumulative diff across all subtasks |

### 1.3 Current UI Flow (Changes Tab)

```
ChangesTab
  └─ TaskListSubtab
       ├─ Workspace list → click → Task list
       │     └─ Per-task: [▸ Full Diff] [▸ Subtasks]
       │           ├─ Full Diff panel (purple) — task-level cumulative diff
       │           └─ Subtasks panel (teal) — per-subtask prompt + [▸ Diff] button
       └─ Steps expansion → per-step diff (amber)
```

**Problem**: Getting to "what did the latest subtask change + what was the prompt" requires **4–5 clicks**: select workspace → find latest task → click Subtasks → find last subtask → click Diff. The "Latest" view eliminates this entirely.

---

## 2. What "Latest" Would Show

A single view that auto-resolves and displays:

```
┌─────────────────────────────────────────────────────┐
│  ⚡ LATEST — auto-refreshes to most recent activity │
├─────────────────────────────────────────────────────┤
│                                                     │
│  📋 Task: 1738967631234                             │
│  🕐 2026-02-08 17:05:23                             │
│  💬 Subtask #3 (Feedback)                           │
│                                                     │
│  ┌─ PROMPT ──────────────────────────────────────┐  │
│  │ "fix the subtask diff handler to correctly    │  │
│  │  map time windows when..."                    │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌─ CHANGED FILES (7) ──────────────────────────┐  │
│  │ ~ src-tauri/src/shadow_git/discovery.rs  +42 -18│
│  │ + src/lib/tabs/changes/LatestView.svelte +120  │ │
│  │ ~ src-tauri/src/server.rs                +3 -1 │ │
│  │ ...                                            │ │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌─ DIFF ────────────────────────────────────────┐  │
│  │ (unified patch text, syntax-highlighted)      │  │
│  │ [📋 Copy Diff]                                │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  Context: workspace 4184916832 · 12 steps total     │
│  Message range: msgs 45-62 · 8 tool calls           │
│  Tools: write_to_file, read_file, execute_command   │
└─────────────────────────────────────────────────────┘
```

### 2.1 Definition of "Latest"

The "latest" item is determined by this resolution order:

1. **Find the most recent task** — sort all tasks from `GET /history/tasks` by `started_at` descending → take first.
2. **Find the last subtask** — call `GET /history/tasks/:taskId/subtasks` → take the subtask with the **highest `subtask_index`** (the most recent feedback prompt, or the initial task if there's only one).
3. **Get the diff for that subtask** — call `GET /changes/tasks/:taskId/subtasks/:lastIndex/diff?workspace=<resolved_workspace>`.
4. **Merge** the subtask's `prompt` + `timestamp` + `tools_used` with the diff's `files` + `patch` into a single response.

If the latest task has **no subtasks** (single-prompt task), fall back to the full task diff (`GET /changes/tasks/:taskId/diff`).

If the latest task has **no checkpoint commits** (conversation-only, no code changes), show the prompt with a "No code changes detected" message.

---

## 3. Feasibility Assessment

### 3.1 What Already Works (90% of the data pipeline exists)

| Requirement | Existing Support | Gap |
|-------------|-----------------|-----|
| Find latest task | `GET /history/tasks` returns newest-first | ✅ Just take `tasks[0]` |
| Get subtask prompts | `GET /history/tasks/:taskId/subtasks` | ✅ Returns all prompts with timestamps |
| Map task → workspace | Shadow git workspace discovery | ⚠️ **Gap**: No direct task-to-workspace lookup. Must scan all workspaces' task lists to find which workspace contains the task ID. |
| Get subtask diff | `GET /changes/tasks/:taskId/subtasks/:idx/diff` | ✅ Already implements the time-window → commit-range mapping |
| File list with stats | `DiffResult.files` has path, status, lines added/removed | ✅ Complete |
| Unified patch text | `DiffResult.patch` | ✅ Complete |
| Tool usage per subtask | `SubtaskEntry.tools_used`, `tool_call_count` | ✅ Complete |

### 3.2 The One Real Gap: Task-to-Workspace Resolution

Today, getting a subtask diff requires knowing the `workspace_id`. The conversation history module doesn't track which workspace a task belongs to. The shadow git module organizes by workspace → task, not task → workspace.

**Resolution strategies** (pick one):

| Strategy | Complexity | Performance |
|----------|-----------|-------------|
| **A. Brute-force scan**: For the latest task_id, iterate all workspaces' task lists until found | Low | O(workspaces × tasks). Fine for <10 workspaces. |
| **B. Reverse index cache**: Build a `HashMap<task_id, workspace_id>` during workspace/task discovery. Keep in memory. | Medium | O(1) lookup after initial build. Best for production. |
| **C. Combined discovery endpoint**: New `GET /latest` handler does workspace→task scan internally, never exposes workspace_id to the caller. | Medium | Cleanest API surface. Hides the workspace concept. |

**Recommended**: Strategy **C** — the "Latest" endpoint handles resolution internally. The caller never needs to know about workspaces.

### 3.3 Complexity Estimate

| Component | Effort | Notes |
|-----------|--------|-------|
| **New Rust composite type** `LatestResponse` | Small | Struct combining prompt + diff + metadata |
| **New Rust handler** `get_latest_handler` | Medium | Orchestrates: history tasks → subtasks → workspace resolution → subtask diff. ~100-150 lines. |
| **Wire into server.rs** | Trivial | One `.route("/latest", get(...))` line |
| **OpenAPI annotation** | Small | `#[utoipa::path]` on the handler |
| **New Svelte component** `LatestSubtab.svelte` or `LatestTab.svelte` | Medium | ~200 lines. Mostly copy-paste from existing subtask diff rendering in TaskListSubtab. |
| **Tab/subtab wiring** | Trivial | Add to `tabs/index.ts` or as a subtab in Changes/History |

**Total estimate**: ~400–500 lines of new code across 3–4 files. No architectural changes needed.

---

## 4. API Design

### 4.1 Option A: Single Composite Endpoint (Recommended)

```
GET /latest
GET /latest?scope=subtask    (default — latest subtask only)
GET /latest?scope=task       (full task diff, all subtasks merged)
GET /latest?exclude=target&exclude=node_modules
```

**Response** (`LatestResponse`):

```jsonc
{
  // ---- Identity ----
  "taskId": "1738967631234",
  "subtaskIndex": 3,           // null if scope=task
  "isInitialTask": false,
  "totalSubtasks": 4,

  // ---- Prompt ----
  "prompt": "fix the subtask diff handler to correctly map...",
  "promptTimestamp": "2026-02-08T17:05:23.000Z",

  // ---- Diff ----
  "diff": {
    "files": [
      { "path": "src-tauri/src/shadow_git/discovery.rs", "status": "modified", "linesAdded": 42, "linesRemoved": 18 },
      // ...
    ],
    "patch": "diff --git a/...\n...",
    "fromRef": "abc12345",
    "toRef": "def67890"
  },

  // ---- Context (from conversation history) ----
  "messageRangeStart": 45,
  "messageRangeEnd": 62,
  "messageCount": 18,
  "toolCallCount": 8,
  "toolsUsed": ["write_to_file", "read_file", "execute_command"],

  // ---- Resolution metadata ----
  "workspaceId": "4184916832",
  "taskStartedAt": "2026-02-08T14:30:00.000Z",
  "taskEndedAt": "2026-02-08T17:12:45.000Z",
  "resolvedAt": "2026-02-08T17:14:01.000Z"  // when this response was computed
}
```

**Error cases**:
- No tasks found → `404 { "error": "No Cline tasks found", "code": 404 }`
- No workspace for task → `404 { "error": "No checkpoint workspace found for task ...", "code": 404 }`
- No diff available → Return response with `"diff": null` and a `"noDiffReason": "no_checkpoints_in_window"` field

### 4.2 Option B: Keep Separate, Compose in Frontend

No new backend endpoint. The frontend calls:
1. `GET /history/tasks?limit=1` → get latest task
2. `GET /history/tasks/:id/subtasks` → get subtasks, pick last
3. `GET /changes/workspaces` → scan for workspace
4. `GET /changes/tasks?workspace=<ws>` → verify task exists
5. `GET /changes/tasks/:id/subtasks/:idx/diff?workspace=<ws>` → get diff

**Pro**: No backend changes.
**Con**: 5 sequential HTTP calls, ~500ms+ latency. Workspace resolution logic duplicated in frontend. Fragile.

**Verdict**: Option A is clearly better — one call, one response, server handles the plumbing.

---

## 5. UI Placement Options

### 5.1 Option A: New Top-Level Tab — `Latest`

```
[ My Jiras ] [ Activity ] [ API ] [ Agent ] [ Changes ] [ History ] [ ⚡ Latest ]
```

- **Pro**: Most discoverable. Zero navigation to see latest changes.
- **Con**: Adds a 7th tab. Might feel redundant if it only shows one thing.
- **Best for**: Users who primarily want "what did Cline just do?"

### 5.2 Option B: Subtab Inside Changes Tab

```
Changes Tab:
  [ Tasks ] [ Diff ] [ ⚡ Latest ] [ Export ]
```

- **Pro**: Logically grouped with other diff/change views. No new top-level tab.
- **Con**: Still requires navigating to Changes tab first.
- **Best for**: Keeping the top bar clean.

### 5.3 Option C: Subtab Inside History Tab

```
History Tab:
  [ Tasks ] [ Stats ] [ ⚡ Latest ]
```

- **Pro**: The prompt text lives in History, so "latest prompt + diff" bridges both.
- **Con**: History is conversation-focused, diff is changes-focused — slight conceptual mismatch.

### 5.4 Option D: Auto-Landing Page (Default View)

Make the app **default to the Latest view** on launch instead of "My Jiras". The Latest view is shown first, and users can navigate to other tabs as needed.

- **Pro**: Maximum zero-click value. "Open app → see what just happened."
- **Con**: Breaks existing UX expectations. Jira users expect Jira first.
- **Compromise**: Make default tab configurable in Settings.

### Recommended: **Option A or B** depending on user preference.

---

## 6. Implementation Plan (If Approved)

### Phase 1: API (`GET /latest`)

```
New/modified files:
  src-tauri/src/latest/              ← new module
    mod.rs                           ← module root
    types.rs                         ← LatestResponse, LatestQuery
    handler.rs                       ← get_latest_handler (orchestration logic)
  src-tauri/src/server.rs            ← add route
  src-tauri/src/openapi.rs           ← add to OpenAPI doc
  src-tauri/src/main.rs              ← mod latest
```

**Handler pseudo-logic**:
```
fn get_latest_handler(scope, excludes):
    1. tasks = conversation_history::list_tasks(limit=1)
    2. if tasks.empty → 404
    3. task = tasks[0]
    4. subtasks = conversation_history::parse_subtasks(task.id)
    5. latest_subtask = subtasks.last()
    6. workspace_id = shadow_git::find_workspace_for_task(task.id)  // new helper
    7. if scope == "subtask":
         diff = shadow_git::get_subtask_diff(task.id, latest_subtask.index, workspace_id, excludes)
       else:
         diff = shadow_git::get_task_diff(task.id, workspace_id, excludes)
    8. return LatestResponse { prompt, diff, metadata }
```

### Phase 2: Workspace Reverse Lookup

```
New helper in src-tauri/src/shadow_git/discovery.rs:
  pub fn find_workspace_for_task(task_id: &str) -> Option<(String, PathBuf)>
    - Iterates all workspaces
    - For each, runs `git log --oneline` grepping for task_id in commit subjects
    - Returns (workspace_id, git_dir) on first match
    - Cached in TASKS_CACHE for subsequent calls
```

### Phase 3: UI Component

```
New/modified files:
  src/lib/tabs/changes/LatestSubtab.svelte   ← new component (~200 lines)
  src/lib/tabs/changes/ChangesTab.svelte      ← add subtab
  src/lib/tabs/changes/api.ts                 ← add fetchLatest()
  src/lib/tabs/changes/types.ts               ← add LatestResponse type
```

---

## 7. Edge Cases & Considerations

| Edge Case | Handling |
|-----------|----------|
| **No tasks at all** | Show empty state: "No Cline tasks found. Run a task first." |
| **Task exists in history but not in checkpoints** | Show prompt + "No checkpoint data. Checkpoints may be disabled or deleted." |
| **Task has checkpoints but no subtask match** | Fall back to full task diff |
| **Multiple workspaces with same task ID** | Take the one with the most recent `last_modified` |
| **Workspace `.git_disabled`** | Still readable — `git log` works with `--git-dir` flag. Already handled by existing discovery code. |
| **Very large diff** | Same truncation/streaming as existing diff views. Add `?max_patch_size=` param if needed. |
| **Task is still in progress** (active Cline session) | Show whatever checkpoints exist so far. The diff represents "changes up to the last checkpoint." Add `"taskInProgress": true` flag if detectable. |
| **Cline directory not found** | Existing error handling returns appropriate 404/500 |

---

## 8. Implementation Status

> **Status: ✅ IMPLEMENTED (v2 — Enhanced with Subtask Tabs)** — Full task diff by default + on-demand subtask drill-down tabs. Both backend and frontend compile cleanly.

### Current Architecture (v2)

The "Latest" view now provides:

1. **Full task diff by default** — `scope=task` is the new default (was `subtask` in v1)
2. **Subtask tab bar** — All subtasks listed as clickable tabs (`📋 Full Task | 🎯 Initial | 💬 #1 | 💬 #2 | ...`)
3. **On-demand subtask diffs** — Clicking a subtask tab loads its diff via `GET /changes/tasks/:taskId/subtasks/:idx/diff` (cached after first load)
4. **Copy for each view** — Copy Diff button + Copy Prompt button on every tab
5. **Backend returns subtask summaries** — `LatestResponse.subtasks[]` contains all subtask metadata (prompt, timestamps, tool counts) without diffs

```
┌─────────────────────────────────────────────────────────────┐
│  ⚡ Latest Task                              [↻ Refresh]    │
├─────────────────────────────────────────────────────────────┤
│  📋 Task: 1738967631234  [LATEST]                           │
│  🕐 02/08/2026  ·  4 subtasks  ·  WS: 4184916832           │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────┬─────────────┬──────────┬──────────┐           │
│  │📋 Full   │ 🎯 Initial  │ 💬 #1    │ 💬 #2    │  ← tabs  │
│  │ Task ●   │   12🔧      │  8🔧     │  5🔧     │           │
│  └──────────┴─────────────┴──────────┴──────────┘           │
│                                                             │
│  ┌─ PROMPT ─────────────────────────────── [📋 Copy] ──┐   │
│  │ "fix the subtask diff handler to correctly map..."   │   │
│  └──────────────────────────────────────────────────────┘   │
│  🔧 write_to_file  🔧 read_file  🔧 execute_command        │
│                                                             │
│  📦 12.3 KB  +142  −38   abc12345 → def67890               │
│                                                             │
│  ┌─ CHANGED FILES (7) ─────────────────────────────────┐   │
│  │ ~ discovery.rs                         +42 -18      │   │
│  │ + LatestSubtab.svelte                  +280         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─ UNIFIED DIFF (Full Task) ──────── [📋 Copy Diff] ──┐   │
│  │ diff --git a/...                                     │   │
│  │ (syntax-highlighted unified patch)                   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                             │
│  View: Full Task · Messages: 0–62 · 02/08 → 02/08          │
└─────────────────────────────────────────────────────────────┘
```

**Clicking a subtask tab** (e.g., `💬 #1`) triggers:
- On-demand fetch of `GET /changes/tasks/:taskId/subtasks/1/diff?workspace=<ws>`
- Shows that subtask's prompt, tools, and diff
- Result is cached — clicking back and forth is instant

### Files Created

| File | Purpose |
|------|---------|
| `src-tauri/src/latest/mod.rs` | Module root — re-exports types + handler |
| `src-tauri/src/latest/types.rs` | `LatestResponse`, `SubtaskSummaryItem`, `LatestErrorResponse`, `LatestQuery` with utoipa derive |
| `src-tauri/src/latest/handler.rs` | `get_latest_handler` — composite orchestration (history → subtasks → workspace lookup → diff + subtask summaries) |
| `src/lib/tabs/changes/LatestSubtab.svelte` | UI component — tabbed view with full task diff default + on-demand subtask diff tabs + copy buttons |

### Files Modified

| File | Change |
|------|--------|
| `src-tauri/src/shadow_git/discovery.rs` | Added `find_workspace_for_task()` — reverse lookup scanning all workspaces |
| `src-tauri/src/shadow_git/mod.rs` | Exported `find_workspace_for_task` |
| `src-tauri/src/main.rs` | Added `mod latest` |
| `src-tauri/src/server.rs` | Added `use crate::latest`, route `/latest` with auth middleware |
| `src-tauri/src/openapi.rs` | Registered `get_latest_handler` path + `LatestResponse`/`LatestErrorResponse`/`SubtaskSummaryItem` schemas |
| `src/lib/tabs/changes/types.ts` | Added `LatestResponse`, `SubtaskSummaryItem` interfaces, `'Latest'` to `ChangesSubTab` union |
| `src/lib/tabs/changes/api.ts` | Added `fetchLatest()` + existing `fetchSubtaskDiff()` used for on-demand subtask diffs |
| `src/lib/tabs/changes/ChangesTab.svelte` | Added `⚡ Latest` subtab (default active), imports `LatestSubtab` |
| `src/lib/tabs/changes/index.ts` | Exported `LatestSubtab`, `SubtaskSummaryItem`, `fetchLatest` |

### API Endpoint

```
GET /latest                          → task scope (default — full task diff + subtask summaries)
GET /latest?scope=subtask            → latest subtask diff only
GET /latest?scope=task&exclude=target&exclude=node_modules
```

**Response now includes `subtasks[]`** — an array of `SubtaskSummaryItem` with metadata for all subtasks (prompt, timestamp, tool counts, message counts). Diffs are NOT included in this array — they are fetched on-demand via:

```
GET /changes/tasks/:taskId/subtasks/:idx/diff?workspace=<ws>
```

Registered in OpenAPI at `/openapi.json` with tags `["changes", "history", "tool"]`.

### Data Flow

```
                    ┌─────────────────┐
  User opens        │  GET /latest    │  ← default scope=task
  Latest tab   ──►  │  (1 HTTP call)  │
                    └────────┬────────┘
                             │
         ┌───────────────────┼────────────────────┐
         ▼                   ▼                    ▼
  Full task diff      Latest prompt       Subtask summaries[]
  (files + patch)     (text + timestamp)  (metadata only, no diffs)
         │                                        │
         └──── renders immediately ───────────────┘

  User clicks subtask tab (#1):
                    ┌─────────────────────────────────────────┐
                    │ GET /changes/tasks/:id/subtasks/1/diff  │  ← on demand
                    └────────────────┬────────────────────────┘
                                     ▼
                              Subtask #1 diff
                              (files + patch)
                                     │
                                     ▼
                              Cached in Map<>
                              (instant on re-click)
```

---

## 9. Summary

| Question | Answer |
|----------|--------|
| **Is it feasible?** | ✅ Yes — 90% of data pipeline exists. One new composite endpoint + one UI component. |
| **Is it hard?** | No — ~400-500 lines of new code. No architectural changes. |
| **What's the real gap?** | Task-to-workspace reverse lookup (small helper function + cache). |
| **Best API approach?** | Single `GET /latest` endpoint that handles all orchestration server-side. |
| **Best UI approach?** | New subtab in Changes tab, or new top-level tab for maximum visibility. |
| **Risk?** | Low. All underlying APIs are stable and tested. The "Latest" view is pure composition. |
| **Time estimate?** | 2-3 hours for API + UI + tests, assuming the developer is familiar with the codebase. |

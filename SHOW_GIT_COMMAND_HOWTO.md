# Show Git Commands Feature — Dev Plan

## Overview

Expose the **actual git commands** executed by the backend in every shadow-git API response, so the UI can display them in a collapsible panel. This is invaluable for debugging, learning, and verifying that the correct commit ranges are being diffed.

## Data Flow

```
discovery.rs (builds git args)
      │
      ▼
DiffResult.git_commands: Vec<String>   ← NEW field
      │
      ▼
JSON response  { "files":[…], "patch":"…", "gitCommands":["git --git-dir …","…"] }
      │
      ▼
TaskListSubtab / LatestSubtab  →  collapsible <pre> block
```

## Backend Changes

### 1. Add `git_commands` to `DiffResult` (types.rs)

```rust
// src-tauri/src/shadow_git/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub files: Vec<DiffFile>,
    pub patch: String,
    pub from_ref: String,
    pub to_ref: String,
    pub git_commands: Vec<String>,   // ← NEW
}
```

### 2. Collect commands in `get_task_diff()` / `get_subtask_diff()` / `get_step_diff()` (discovery.rs)

In each function, maintain a `let mut git_commands: Vec<String> = Vec::new();` and push formatted command strings each time a `Command::new("git")` is built:

```rust
let cmd_str = format!("git {}", numstat_args.join(" "));
git_commands.push(cmd_str);
```

Do this for:
| Function | Commands to capture |
|----------|-------------------|
| `get_task_diff` | `git diff --numstat`, `git diff` (patch), fallback `git diff-tree` if used |
| `get_subtask_diff` | Same as above |
| `get_step_diff` | Same as above |

Return `git_commands` in the `DiffResult`.

### 3. Also add to `StepsResponse` and `TaskListResponse` (optional, lower priority)

For `list_steps_for_task` and `list_tasks_for_workspace`, a single `git_command` field could show the `git log --all --pretty=…` used for discovery. This is lower priority since the diff commands are the most useful to expose.

### 4. Add to `LatestResponse` (latest/types.rs)

The `LatestResponse.diff` field already uses `DiffResult`, so the `gitCommands` field will automatically flow through. No extra changes needed.

## Frontend Changes

### 5. Update TypeScript types (changes/types.ts)

```typescript
export interface DiffData {
  files: DiffFile[];
  patch: string;
  fromRef: string;
  toRef: string;
  gitCommands?: string[];   // ← NEW
}
```

### 6. Collapsible Git Commands panel (TaskListSubtab.svelte)

Add a small toggle below the diff header in both:
- **Full Task Diff** section
- **Subtask Diff** section

```svelte
<!-- Git Commands Toggle -->
{#if diff?.gitCommands?.length}
  <button
    class="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1 mt-1"
    onclick={() => showGitCmds = !showGitCmds}
  >
    <span class="font-mono">{showGitCmds ? '▾' : '▸'}</span>
    <span>Git Commands ({diff.gitCommands.length})</span>
  </button>
  {#if showGitCmds}
    <pre class="text-xs bg-gray-900 text-green-400 p-3 rounded mt-1 overflow-x-auto font-mono">
{diff.gitCommands.join('\n')}
    </pre>
  {/if}
{/if}
```

### 7. Same toggle in LatestSubtab.svelte

Add the same collapsible panel in the diff section of LatestSubtab, using `data.diff.gitCommands`.

## UI Design

```
┌─────────────────────────────────────────────┐
│ Full Task Diff · 8605bd65 → 7bb86402        │
│ 9 files · 45.2KB                            │
│ ▸ Git Commands (2)           ← collapsed    │
│                                             │
│ ▾ Git Commands (2)           ← expanded     │
│ ┌─────────────────────────────────────────┐ │
│ │ $ git --git-dir C:\Users\...\checkpo... │ │
│ │     diff --numstat 8605bd65^  7bb86402  │ │
│ │ $ git --git-dir C:\Users\...\checkpo... │ │
│ │     diff 8605bd65^  7bb86402            │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ ▸ FILES CHANGED (9)                         │
└─────────────────────────────────────────────┘
```

**Styling:**
- Toggle text: `text-xs text-gray-400` (subtle, doesn't compete with main content)
- Commands block: dark terminal theme (`bg-gray-900 text-green-400 font-mono`)
- Commands are selectable/copyable (users can paste into terminal)
- Long paths may overflow horizontally → use `overflow-x-auto`

## REST APIs Affected

All endpoints that return `DiffResult` will gain the new `gitCommands` field:

| # | Method | Endpoint | Handler | What changes |
|---|--------|----------|---------|-------------|
| 1 | `GET` | `/changes/tasks/:task_id/diff?workspace=…` | `task_diff_handler` | Full task diff — adds `gitCommands` with `git diff --numstat` + `git diff` commands |
| 2 | `GET` | `/changes/tasks/:task_id/subtasks/:subtask_index/diff?workspace=…` | `subtask_diff_handler` | Per-subtask diff — adds `gitCommands` with same git commands scoped to subtask commit range |
| 3 | `GET` | `/changes/tasks/:task_id/steps/:index/diff?workspace=…` | `step_diff_handler` | Per-step diff — adds `gitCommands` for single commit diff |
| 4 | `GET` | `/latest` | `get_latest_handler` | Composite endpoint — `diff` field is a `DiffResult`, so `gitCommands` flows through automatically |

**Endpoints NOT affected** (they don't return `DiffResult`):

| Method | Endpoint | Why not affected |
|--------|----------|-----------------|
| `GET` | `/changes/workspaces` | Returns workspace list, no diffs |
| `GET` | `/changes/tasks?workspace=…` | Returns task summaries, no diffs |
| `GET` | `/changes/tasks/:task_id/steps?workspace=…` | Returns step list, no diffs |

### Example Response (before → after)

**Before:**
```json
{
  "files": [...],
  "patch": "diff --git a/...",
  "fromRef": "8605bd65^",
  "toRef": "7bb86402"
}
```

**After:**
```json
{
  "files": [...],
  "patch": "diff --git a/...",
  "fromRef": "8605bd65^",
  "toRef": "7bb86402",
  "gitCommands": [
    "git --git-dir C:\\Users\\...\\checkpoints\\1329051419\\.git diff --numstat 8605bd65^ 7bb86402",
    "git --git-dir C:\\Users\\...\\checkpoints\\1329051419\\.git diff 8605bd65^ 7bb86402"
  ]
}
```

## Files to Modify

| # | File | Change |
|---|------|--------|
| 1 | `src-tauri/src/shadow_git/types.rs` | Add `git_commands: Vec<String>` to `DiffResult` |
| 2 | `src-tauri/src/shadow_git/discovery.rs` | Collect command strings in `get_task_diff`, `get_subtask_diff`, `get_step_diff` |
| 3 | `src/lib/tabs/changes/types.ts` | Add `gitCommands?: string[]` to `DiffData` |
| 4 | `src/lib/tabs/changes/TaskListSubtab.svelte` | Collapsible toggle + `<pre>` block in task diff & subtask diff sections |
| 5 | `src/lib/tabs/changes/LatestSubtab.svelte` | Same toggle in the Latest diff section |

## Implementation Order

1. **Backend first** — modify `types.rs` then `discovery.rs`
2. **Frontend types** — update `types.ts`
3. **UI toggles** — add to `TaskListSubtab.svelte` and `LatestSubtab.svelte`
4. **Test** — rebuild backend, verify `gitCommands` appears in JSON, verify UI toggle works

## Estimated Effort

- Backend: ~30 min (straightforward string collection)
- Frontend: ~20 min (simple toggle component)
- Testing: ~10 min
- **Total: ~1 hour**

# Cline Shadow Git — Integration Into Jira Dashboard

> **Discovery doc:** `shadow_dif_discovery.md`  
> **Date:** 2026-02-07  
> **Status:** Design / Proposal

---

## 1. The Opportunity

Every Cline task produces **checkpoint commits** in a hidden ("shadow") Git repo.
These commits capture *exactly* which files changed per step and per task —
isolated from any other dirty state in your working tree.

We already have a tab-based Tauri+Svelte app.  
Adding a **"Changes"** top-level tab would let us:

- Browse Cline task history inside the app
- View per-task and per-step diffs (like Cline's "View Changes" but **ours**)
- Export clean patches (attach to Jira, archive, etc.)
- Track agent productivity over time

---

## 2. Where the Data Lives

```
%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\
├── tasks\
│   └── <task-id>\
│       └── api_conversation_history.json   ← has lastCheckpointHash
└── checkpoints\
    └── <workspace-id>\
        └── .git                            ← shadow Git repo
```

**Key facts:**
- Each workspace gets its own checkpoint `.git` directory
- Commit subjects follow pattern: `checkpoint-<workspace-id>-<task-id>`
- Task logs contain `lastCheckpointHash` (40-char SHA)
- Step diffs = parent commit → checkpoint commit
- Task diffs = first-checkpoint-parent → last checkpoint

---

## 3. How It Fits Our Architecture

### Current tab pattern

```
src/lib/tabs/
├── index.ts              ← TabId union, tabs[] array
├── my-jiras/             ← { MyJirasTab, ListSubtab, types, api, index }
├── activity/             ← { ActivityTab, RESTSubtab, InferenceSubtab, ... }
├── api/                  ← { ApiTab, RESTSubtab, ToolsSubtab, ToolsConsoleSubtab, ... }
└── agent/                ← { AgentTab, ChatSubtab, AgentChatSubtab, ... }
```

### New tab: `changes/`

```
src/lib/tabs/
└── changes/
    ├── index.ts              ← re-exports
    ├── types.ts              ← ClineTask, Checkpoint, DiffFile, etc.
    ├── api.ts                ← Tauri invoke wrappers
    ├── ChangesTab.svelte     ← main container + subtab nav
    ├── TaskListSubtab.svelte ← browse tasks
    ├── DiffViewSubtab.svelte ← view diffs for selected task/step
    └── ExportSubtab.svelte   ← export patches
```

### Rust backend additions

```
src-tauri/src/
└── shadow_git/
    ├── mod.rs                ← pub mod, discovery logic
    ├── types.rs              ← ClineTask, Checkpoint structs
    ├── discovery.rs          ← find checkpoint repos, enumerate tasks
    ├── diff.rs               ← produce diffs (task-level, step-level)
    └── handlers.rs           ← Tauri commands: list_tasks, get_task_diff, export_patch
```

### Registration touchpoints

| File | Change |
|------|--------|
| `src/lib/tabs/index.ts` | Add `'changes'` to `TabId`, add `ChangesTab` export, add to `tabs[]` |
| `src/App.svelte` | Add `{:else if activeTab === 'changes'}` block |
| `src-tauri/src/main.rs` | Add `mod shadow_git;`, register new Tauri commands |

---

## 4. ASCII Screens

### 4.1  Top Bar with "Changes" tab added

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Jira Viewer   [My Jiras] [Activity] [API] [Agent] [*Changes*]    ⟳  ⚙ │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.2  Changes Tab — Task List subtab (default view)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Jira Viewer   [My Jiras] [Activity] [API] [Agent] [*Changes*]    ⟳  ⚙ │
├──────────────────────────────────────────────────────────────────────────┤
│  ──Tasks──   ──Diff──   ──Export──                                      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Workspace: jira-dashboard (4184916832)          [Refresh]               │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │ #  │ Task ID          │ Steps │ Files │ Last Changed     │         │ │
│  ├────┼──────────────────┼───────┼───────┼──────────────────┼─────────┤ │
│  │  1 │ 1738900123456    │    12 │     7 │ 2026-02-07 14:01 │ [View]  │ │
│  │  2 │ 1738899000000    │     5 │     3 │ 2026-02-07 11:30 │ [View]  │ │
│  │  3 │ 1738880000000    │     8 │    11 │ 2026-02-06 18:45 │ [View]  │ │
│  │  4 │ 1738870000000    │     3 │     2 │ 2026-02-06 14:20 │ [View]  │ │
│  │  5 │ 1738860000000    │    20 │    15 │ 2026-02-06 09:10 │ [View]  │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│  Total: 5 tasks, 48 steps, 38 files touched                             │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.3  Changes Tab — Diff subtab (task-level diff)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Jira Viewer   [My Jiras] [Activity] [API] [Agent] [*Changes*]    ⟳  ⚙ │
├──────────────────────────────────────────────────────────────────────────┤
│  ──Tasks──   [*Diff*]   ──Export──                                      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Task: 1738900123456   ◂ Back to list                                    │
│                                                                          │
│  Scope: ( ) Step-by-step   (●) Full task                                │
│                                                                          │
│  ┌── Files changed (7) ─────────────────────────────────────────────┐   │
│  │  ● src/lib/tabs/changes/ChangesTab.svelte         +48  -0       │   │
│  │  ● src/lib/tabs/changes/types.ts                  +32  -0       │   │
│  │  ● src/lib/tabs/changes/api.ts                    +25  -0       │   │
│  │  ● src/lib/tabs/index.ts                           +3  -1       │   │
│  │  ● src/App.svelte                                  +2  -0       │   │
│  │  ● src-tauri/src/shadow_git/mod.rs                +85  -0       │   │
│  │  ● src-tauri/src/main.rs                           +5  -1       │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌── src/lib/tabs/changes/ChangesTab.svelte ────────────────────────┐   │
│  │  @@ -0,0 +1,48 @@                                               │   │
│  │  + <script lang="ts">                                            │   │
│  │  +   import TaskListSubtab from "./TaskListSubtab.svelte";       │   │
│  │  +   import DiffViewSubtab from "./DiffViewSubtab.svelte";       │   │
│  │  +   import ExportSubtab from "./ExportSubtab.svelte";           │   │
│  │  +   ...                                                         │   │
│  │  + </script>                                                     │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.4  Changes Tab — Diff subtab (step-by-step mode)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Jira Viewer   [My Jiras] [Activity] [API] [Agent] [*Changes*]    ⟳  ⚙ │
├──────────────────────────────────────────────────────────────────────────┤
│  ──Tasks──   [*Diff*]   ──Export──                                      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Task: 1738900123456   ◂ Back to list                                    │
│                                                                          │
│  Scope: (●) Step-by-step   ( ) Full task                                │
│                                                                          │
│  Step: [◂ Prev]  3 of 12  [Next ▸]                                      │
│  Commit: a1b2c3d4 → e5f6g7h8                                            │
│                                                                          │
│  ┌── Files changed (2) ─────────────────────────────────────────────┐   │
│  │  ● src/lib/tabs/changes/types.ts                  +12  -3       │   │
│  │  ● src-tauri/src/shadow_git/diff.rs               +20  -5       │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌── src/lib/tabs/changes/types.ts ─────────────────────────────────┐   │
│  │  @@ -5,8 +5,17 @@                                               │   │
│  │    export interface ClineTask {                                   │   │
│  │      taskId: string;                                             │   │
│  │  -   checkpoints: string[];                                      │   │
│  │  +   checkpoints: Checkpoint[];                                  │   │
│  │  +   filesChanged: number;                                       │   │
│  │  +   linesAdded: number;                                         │   │
│  │  +   linesRemoved: number;                                       │   │
│  │    }                                                             │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.5  Changes Tab — Export subtab

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Jira Viewer   [My Jiras] [Activity] [API] [Agent] [*Changes*]    ⟳  ⚙ │
├──────────────────────────────────────────────────────────────────────────┤
│  ──Tasks──   ──Diff──   [*Export*]                                      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Task: 1738900123456   ◂ Back to list                                    │
│                                                                          │
│  ┌── Export Options ────────────────────────────────────────────────┐   │
│  │                                                                  │   │
│  │  Format:  (●) Unified diff (.patch)                             │   │
│  │           ( ) Git format-patch (.mbox)                          │   │
│  │           ( ) File list only (.txt)                             │   │
│  │                                                                  │   │
│  │  Scope:   (●) Full task   ( ) Current step                     │   │
│  │                                                                  │   │
│  │  Exclude patterns:                                              │   │
│  │  ┌──────────────────────────────────────────────────────────┐   │   │
│  │  │ src-tauri/target                                         │   │   │
│  │  │ node_modules                                             │   │   │
│  │  │ package-lock.json                                        │   │   │
│  │  └──────────────────────────────────────────────────────────┘   │   │
│  │  [+ Add pattern]                                                │   │
│  │                                                                  │   │
│  │               [ Export to File ]   [ Copy to Clipboard ]        │   │
│  │                                                                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌── Preview ───────────────────────────────────────────────────────┐   │
│  │  diff --git a/src/lib/tabs/index.ts b/src/lib/tabs/index.ts    │   │
│  │  --- a/src/lib/tabs/index.ts                                    │   │
│  │  +++ b/src/lib/tabs/index.ts                                    │   │
│  │  @@ -6,7 +6,8 @@                                               │   │
│  │   export { ApiTab } from './api';                               │   │
│  │   export { AgentTab } from './agent';                           │   │
│  │  +export { ChangesTab } from './changes';                       │   │
│  │  ...                                                            │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 4.6  Empty State (no checkpoints found)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  Jira Viewer   [My Jiras] [Activity] [API] [Agent] [*Changes*]    ⟳  ⚙ │
├──────────────────────────────────────────────────────────────────────────┤
│  ──Tasks──   ──Diff──   ──Export──                                      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                                                                          │
│                         ┌─────────────────┐                              │
│                         │   📂  No Data    │                              │
│                         └─────────────────┘                              │
│                                                                          │
│            No Cline checkpoint repositories found.                       │
│                                                                          │
│            Expected location:                                            │
│            %APPDATA%\Code\User\globalStorage\                            │
│                saoudrizwan.claude-dev\checkpoints\                        │
│                                                                          │
│            Make sure Cline extension is installed and                     │
│            you have run at least one task.                                │
│                                                                          │
│                        [ Open Folder ]                                   │
│                                                                          │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Data Flow

```
                        ┌──────────────────────────────┐
                        │   Cline Extension (VS Code)  │
                        │                              │
                        │  tasks/<id>/*.json           │
                        │  checkpoints/<ws>/.git       │
                        └──────────────┬───────────────┘
                                       │ (filesystem)
                                       ▼
┌───────────────┐     invoke()    ┌────────────────────┐
│  Svelte UI    │ ◄──────────────►│  Tauri (Rust)      │
│               │                 │                    │
│  ChangesTab   │                 │  shadow_git/       │
│  ├ TaskList   │  list_tasks()   │  ├ discovery.rs    │
│  ├ DiffView   │  ───────────►   │  │  find repos     │
│  └ Export     │                 │  │  enumerate tasks │
│               │  get_task_diff()│  ├ diff.rs         │
│               │  ───────────►   │  │  git diff ops   │
│               │                 │  ├ types.rs        │
│               │  export_patch() │  │  ClineTask, etc │
│               │  ───────────►   │  └ handlers.rs     │
│               │                 │     Tauri commands  │
└───────────────┘                 └────────────────────┘
                                       │
                                       │ std::process::Command("git")
                                       ▼
                                  ┌──────────┐
                                  │   git    │
                                  │ (CLI)    │
                                  └──────────┘
```

---

## 6. Type Definitions (TypeScript side)

```typescript
// src/lib/tabs/changes/types.ts

/** A single Cline checkpoint */
export interface Checkpoint {
  hash: string;          // 40-char SHA
  subject: string;       // commit subject: "checkpoint-<wsId>-<taskId>"
  timestamp: string;     // ISO 8601
  filesChanged: number;
}

/** A Cline task (group of checkpoints) */
export interface ClineTask {
  taskId: string;
  workspaceId: string;
  checkpoints: Checkpoint[];
  totalFiles: number;
  totalAdded: number;
  totalRemoved: number;
  firstCheckpoint: string;   // SHA
  lastCheckpoint: string;    // SHA
  baseCommit: string;        // SHA (first checkpoint's parent)
  lastModified: string;      // ISO 8601
}

/** A file in a diff */
export interface DiffFile {
  path: string;
  linesAdded: number;
  linesRemoved: number;
  status: 'added' | 'modified' | 'deleted' | 'renamed';
}

/** Full diff result */
export interface DiffResult {
  files: DiffFile[];
  patch: string;            // unified diff text
  fromRef: string;
  toRef: string;
}

/** Available subtabs */
export type ChangesSubTab = 'Tasks' | 'Diff' | 'Export';

export interface SubTabDefinition {
  id: ChangesSubTab;
  label: string;
}
```

---

## 7. Rust Backend Design

### 7.1  Discovery (`shadow_git/discovery.rs`)

```rust
use std::path::PathBuf;
use std::process::Command;

/// Find the Cline globalStorage root
pub fn cline_root() -> PathBuf {
    let appdata = std::env::var("APPDATA")
        .expect("APPDATA not set");
    PathBuf::from(appdata)
        .join("Code")
        .join("User")
        .join("globalStorage")
        .join("saoudrizwan.claude-dev")
}

/// Discover all checkpoint repos (workspace dirs with .git or .git_disabled)
pub fn find_checkpoint_repos() -> Vec<(String, PathBuf)> {
    let cp_root = cline_root().join("checkpoints");
    let mut repos = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&cp_root) {
        for entry in entries.flatten() {
            let ws_id = entry.file_name().to_string_lossy().to_string();
            for git_name in &[".git", ".git_disabled"] {
                let git_dir = entry.path().join(git_name);
                if git_dir.exists() {
                    repos.push((ws_id.clone(), git_dir));
                }
            }
        }
    }
    repos
}

/// Enumerate tasks inside a checkpoint repo by parsing commit subjects
pub fn list_tasks(git_dir: &PathBuf) -> Vec<ClineTask> {
    // git --git-dir <path> log --pretty="%H %s" --all
    // parse subjects: checkpoint-<wsId>-<taskId>
    // group by taskId
    // ...
    todo!()
}
```

### 7.2  Diff (`shadow_git/diff.rs`)

```rust
/// Get task-level diff (base → last checkpoint)
pub fn task_diff(
    git_dir: &PathBuf,
    base: &str,
    end: &str,
    excludes: &[String],
) -> Result<DiffResult, String> {
    // git --git-dir <path> diff <base> <end> -- . ":(exclude)..." 
    let mut args = vec![
        "--git-dir", &git_dir.to_string_lossy(),
        "diff", base, end, "--",  ".",
    ];
    for ex in excludes {
        args.push(&format!(":(exclude){}", ex));
    }
    // execute and parse
    todo!()
}

/// Get step-level diff (prev checkpoint → current checkpoint)
pub fn step_diff(
    git_dir: &PathBuf,
    from: &str,
    to: &str,
) -> Result<DiffResult, String> {
    // git --git-dir <path> diff <from> <to>
    todo!()
}
```

### 7.3  Tauri Commands (`shadow_git/handlers.rs`)

```rust
#[tauri::command]
fn list_cline_tasks() -> Result<Vec<ClineTask>, String> { ... }

#[tauri::command]
fn get_task_diff(
    task_id: String,
    excludes: Vec<String>,
) -> Result<DiffResult, String> { ... }

#[tauri::command]
fn get_step_diff(
    task_id: String,
    step_index: usize,
) -> Result<DiffResult, String> { ... }

#[tauri::command]
fn export_patch(
    task_id: String,
    excludes: Vec<String>,
    format: String,       // "unified" | "format-patch" | "name-only"
) -> Result<String, String> { ... }
```

---

## 8. Implementation Plan (Step-by-Step)

### Phase 1 — Rust backend (shadow_git module)

```
Step  What                                         Touches
────  ──────────────────────────────────────────── ──────────────────────
1.1   Create src-tauri/src/shadow_git/mod.rs       new file
1.2   Create types.rs with ClineTask, Checkpoint   new file
1.3   Create discovery.rs — find repos, enum tasks new file
1.4   Create diff.rs — task_diff(), step_diff()    new file
1.5   Create handlers.rs — Tauri commands          new file
1.6   Register module & commands in main.rs        src-tauri/src/main.rs
1.7   Test with PowerShell (verify output matches  manual verification
      the discovery doc's approach)
```

### Phase 2 — Svelte frontend (changes tab)

```
Step  What                                         Touches
────  ──────────────────────────────────────────── ──────────────────────
2.1   Create src/lib/tabs/changes/ directory        new dir
2.2   Create types.ts                              new file
2.3   Create api.ts (invoke wrappers)              new file
2.4   Create TaskListSubtab.svelte                 new file
2.5   Create DiffViewSubtab.svelte                 new file
2.6   Create ExportSubtab.svelte                   new file
2.7   Create ChangesTab.svelte (container+nav)     new file
2.8   Create index.ts (re-exports)                 new file
```

### Phase 3 — Wire it up

```
Step  What                                         Touches
────  ──────────────────────────────────────────── ──────────────────────
3.1   Add 'changes' to TabId union                 src/lib/tabs/index.ts
3.2   Add ChangesTab export                        src/lib/tabs/index.ts
3.3   Add to tabs[] array                          src/lib/tabs/index.ts
3.4   Add {:else if} block in App.svelte           src/App.svelte
3.5   Verify tab appears and navigates             manual test
```

### Phase 4 — Polish

```
Step  What                                         Touches
────  ──────────────────────────────────────────── ──────────────────────
4.1   Syntax-highlighted diff rendering            DiffViewSubtab.svelte
4.2   File save dialog for export                  ExportSubtab.svelte
4.3   Clipboard support                            ExportSubtab.svelte
4.4   Loading states & error handling              all subtabs
4.5   Settings: configurable exclude patterns      SettingsModal.svelte
```

---

## 9. Key Design Decisions & Reasoning

### Why a Tauri command, not direct filesystem access from Svelte?

The shadow git repo requires **executing `git` CLI commands** with `--git-dir`.
Svelte (WebView) can't spawn processes directly.  
Tauri commands give us Rust's `std::process::Command` —
fast, safe, and consistent with how we already call `git` in the discovery doc.

### Why not use libgit2 / git2 crate?

We *could* use the `git2` Rust crate to avoid shelling out.  
**Pros:** No `git` CLI dependency, faster, more structured output.  
**Cons:** Heavier dependency, more code for diff formatting.  
**Recommendation:** Start with `std::process::Command("git")` (simpler, matches the discovery doc exactly), migrate to `git2` later if perf matters.

### Why three subtabs (Tasks / Diff / Export)?

Matches the natural workflow:
1. **Browse** → pick a task
2. **Inspect** → see what changed (step or task level)
3. **Act** → export a patch, copy to clipboard, attach to Jira

Could start with just Tasks + Diff and add Export later.

### How do we map workspace-id to our project?

On startup, we can compute our workspace's hash the same way VS Code does
(or just scan all checkpoint repos and check if our project root's files
appear in the commits). Pragmatic approach: let the user pick the workspace
from a dropdown if multiple exist, or auto-detect by matching the most
recent checkpoint commit's tree content against `$CWD`.

### Thread safety

Git operations can be slow on large repos. All Tauri commands should be
`async` and run git in a background thread (`tokio::task::spawn_blocking`).
The UI shows a spinner during loading.

---

## 10. Minimal Viable Slice

If we want the fastest path to "something useful", the order is:

1. **Rust: `list_cline_tasks()`** — just enumerate tasks + file counts
2. **Svelte: TaskListSubtab** — display the list
3. **Rust: `get_task_diff()`** — return unified diff text
4. **Svelte: DiffViewSubtab** — render the diff (even plain `<pre>` is useful)

That's **4 files** of new code + **3 small edits** to existing files.  
Export and step-by-step navigation can follow.

---

## 11. File Manifest (New Files)

```
NEW FILES:
  src-tauri/src/shadow_git/mod.rs
  src-tauri/src/shadow_git/types.rs
  src-tauri/src/shadow_git/discovery.rs
  src-tauri/src/shadow_git/diff.rs
  src-tauri/src/shadow_git/handlers.rs
  src/lib/tabs/changes/index.ts
  src/lib/tabs/changes/types.ts
  src/lib/tabs/changes/api.ts
  src/lib/tabs/changes/ChangesTab.svelte
  src/lib/tabs/changes/TaskListSubtab.svelte
  src/lib/tabs/changes/DiffViewSubtab.svelte
  src/lib/tabs/changes/ExportSubtab.svelte

MODIFIED FILES:
  src/lib/tabs/index.ts         (add ChangesTab to TabId, exports, tabs[])
  src/App.svelte                (add {:else if activeTab === 'changes'} block)
  src-tauri/src/main.rs         (add mod shadow_git, register commands)
```

---

## 12. Risk Assessment

| Risk | Mitigation |
|------|-----------|
| `.git` dir is hidden/missing | Use `-Force` (PS) / handle gracefully in Rust |
| Multiple workspace-id dirs | Let user pick, or auto-match by project path |
| Git not on PATH | Check at startup, show friendly error |
| Large diffs slow to render | Paginate / lazy-load file diffs |
| Cline changes checkpoint format | Version-check, fail gracefully |
| Checkpoint `.git` renamed to `.git_disabled` | Already handled in discovery (check both names) |

---

## 13. REST API Endpoints

We follow the same dual-spec pattern already in place: **public** endpoints in `openapi.json` (agent-facing) and **admin** endpoints in `openapi_admin.json` (UI diagnostics). See `src-tauri/src/openapi.rs` for how the existing specs are split.

All new routes live under the `/changes/` prefix, registered in `src-tauri/src/server.rs` as a new route group — similar to how `/tools/` routes are grouped today.

### Public API (agent-facing, in `openapi.json`)

These are the endpoints an AI agent or external script could call. Auth required (Bearer token), same as `/jira/list`.

| Method | Path | Purpose | Response Shape |
|--------|------|---------|---------------|
| `GET` | `/changes/workspaces` | List discovered checkpoint workspaces | `{ workspaces: [{ id, gitDir, taskCount }] }` |
| `GET` | `/changes/tasks` | List all tasks for a workspace | `{ tasks: [{ taskId, workspaceId, steps, filesChanged, lastModified }] }` |
| `GET` | `/changes/tasks/:taskId/diff` | Get full task diff (base→HEAD) | `{ files: [...], patch, fromRef, toRef }` |
| `GET` | `/changes/tasks/:taskId/steps` | List checkpoints (steps) in a task | `{ steps: [{ hash, timestamp, filesChanged }] }` |
| `GET` | `/changes/tasks/:taskId/steps/:index/diff` | Get single step diff | `{ files: [...], patch, fromRef, toRef }` |

Query params on diff endpoints:
- `exclude` (repeated) — pathspec exclusion patterns, e.g. `?exclude=src-tauri/target&exclude=node_modules`
- `name_only` (bool) — return just file list, no patch text

### Admin API (UI-facing, in `openapi_admin.json`)

These support the Export subtab and diagnostics. No auth required (same as `/access-logs`).

| Method | Path | Purpose | Response Shape |
|--------|------|---------|---------------|
| `POST` | `/changes/export` | Generate and return a patch file | `{ patch: "...", format, filesCount }` |
| `GET` | `/changes/config` | Get default exclude patterns, workspace prefs | `{ excludes: [...], defaultWorkspace }` |
| `PUT` | `/changes/config` | Update exclude patterns / prefs | `200 OK` |

### How this maps to existing patterns

- **Route registration** → add a `changes_routes` block in `server.rs`, merged into the main `Router` just like `tool_routes` today
- **Handler file** → `src-tauri/src/shadow_git/handlers.rs`, with `#[utoipa::path(...)]` annotations for OpenAPI generation — same pattern as `src-tauri/src/api/handlers.rs` and `src-tauri/src/tool_runtime/handlers.rs`
- **OpenAPI registration** → add handler paths to `PublicApiDoc` and `AdminApiDoc` in `src-tauri/src/openapi.rs`
- **Auth middleware** → public endpoints use `.layer(middleware::from_fn_with_state(state.clone(), auth_middleware))` — same as the `/jira/` and `/agent/` routes
- **State** → handlers take `State(state): State<Arc<AppState>>` to access the auth token and any cached data — identical to existing handlers

### Tauri Commands vs REST — which does the UI use?

Two options, both valid:

| Approach | UI calls | Backend file | Precedent in our app |
|----------|----------|-------------|---------------------|
| **Tauri invoke** | `invoke('list_cline_tasks')` | `shadow_git/handlers.rs` (#[tauri::command]) | `get_issue`, `list_issues` in `main.rs` |
| **REST fetch** | `fetch('/changes/tasks')` | `shadow_git/handlers.rs` (Axum handler) | Agent tab → `fetch('/agent/chat')` in `src/lib/tabs/agent/api.ts` |

**Recommendation:** Expose **both**. The Svelte UI uses Tauri invoke (faster, no network hop). The REST endpoints exist so CLI scripts and agents can also query task changes — e.g. `curl http://127.0.0.1:<port>/changes/tasks` with the Bearer token from `.env`, same workflow as `scripts/test_jira_list.ps1`.

### Example: CLI test script

Following the pattern of `scripts/test_jira_list.ps1`:

```
scripts/test_changes.ps1
  → reads REST_API_URL and REST_API_TOKEN from .env
  → calls GET /changes/workspaces
  → calls GET /changes/tasks?workspace=<id>
  → calls GET /changes/tasks/<taskId>/diff?exclude=src-tauri/target
  → prints file list + patch preview
```

---

## 14. References

- **Discovery doc:** `shadow_dif_discovery.md` (in this repo)
- **Cline source (checkpoints):** `saoudrizwan.claude-dev` extension storage
- **Existing tab pattern:** `src/lib/tabs/agent/` (good template to copy)
- **Tauri invoke docs:** https://v2.tauri.app/develop/calling-rust/

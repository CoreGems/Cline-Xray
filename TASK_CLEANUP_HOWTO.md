# Task Cleanup Feature — Dev Plan

## Overview

Simple workspace cleanup: **nuke all checkpoint history** for a workspace by re-initializing its git repo. The workspace ID stays the same, but all task/step commits are gone. New tasks will start fresh in the same workspace.

## Why Simple?

Selectively deleting individual task commits from interleaved git history requires `git filter-repo` or complex branch rewriting. Instead, just wipe the repo clean — Cline will recreate it when the next task runs.

## What Happens

```
Before:  checkpoints/1329051419/.git  →  142 commits, 7 tasks, ~50MB
After:   checkpoints/1329051419/.git  →  0 commits, 0 tasks, ~0MB
```

The workspace folder and `.git` directory structure remain — only the history is wiped.

## Backend

**Single function** in `cleanup.rs`:

```
nuke_workspace(git_dir):
  1. Verify .git exists (not .git_disabled — Cline is using it)
  2. Delete .git directory entirely
  3. Run: git init --bare <same path>
  4. Return { deletedCommits, gitCommand }
```

That's it. Three shell operations: check, delete, re-init.

**Endpoint**: `POST /changes/workspaces/:id/nuke`

## Frontend

**One button** on the workspace header bar (next to Refresh):

```
◂ Workspaces | Workspace: 1329051419  Active     [🗑 Nuke] [Refresh]
```

**Confirmation dialog** (since it's destructive):

```
⚠️ Nuke workspace 1329051419?

This will delete ALL checkpoint history:
• 7 tasks
• 142 commits

The workspace will be re-initialized empty.
Cline will create new checkpoints on the next task.

This cannot be undone.

[Cancel]  [Nuke It]
```

**After nuke**: Task list refreshes → shows "No Tasks Found".

## Safety

- **Cannot nuke if `.git_disabled`** — means Cline is actively running a task
- **Confirmation required** — dialog with commit/task counts
- **No partial delete** — all or nothing, avoids git history complexity

## Files to Modify

| # | File | Change |
|---|------|--------|
| 1 | `src-tauri/src/shadow_git/cleanup.rs` | **NEW** — `nuke_workspace()` function |
| 2 | `src-tauri/src/shadow_git/mod.rs` | Add `pub mod cleanup;` |
| 3 | `src-tauri/src/shadow_git/handlers.rs` | Add `nuke_workspace_handler` |
| 4 | `src-tauri/src/server.rs` | Add route |
| 5 | `src/lib/tabs/changes/api.ts` | Add `nukeWorkspace()` |
| 6 | `src/lib/tabs/changes/TaskListSubtab.svelte` | Add nuke button + confirmation |

## Estimated Effort

~1 hour total.

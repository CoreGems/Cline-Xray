
## The mystery

Cline’s **View Changes** feels like a normal diff UI. But it has one superpower Git doesn’t: it shows **only the files touched in the current task**, even if your working tree has other unrelated dirt.

At first I assumed it was just `git diff` with an allowlist.

Then I looked at the task logs and saw this:

- an event named `checkpoint_created`
- a field called `lastCheckpointHash`
- a 40-char SHA that looks exactly like a Git commit id

That was the “aha moment.”

Cline wasn’t diffing my repo.

It was diffing **its own hidden checkpoint repository**.

---

## Proof screenshots

![Cline panel showing “View Changes” / “Explain Changes”](https://dev-to-uploads.s3.amazonaws.com/uploads/articles/bxm6ua2hfllex4c9pphv.png)

![VS Code diff view showing “New changes (5 files)” (includes build noise like `target/` if you don’t exclude it)](https://dev-to-uploads.s3.amazonaws.com/uploads/articles/n9mhp27herydgkg398mg.png)

---

## What “View Changes” actually is

Cline creates **checkpoints** during a task. Each checkpoint is stored as a commit in a **shadow Git repository** under VS Code’s global storage.

So “View Changes” is effectively one of these:

- **Step diff:** previous checkpoint commit → current checkpoint commit
- **Task diff:** baseline (parent of first checkpoint in the task) → last checkpoint in the task

That’s why it’s task-scoped and stable even if your repo isn’t clean.

---

## Where the shadow repo lives (Windows + VS Code stable)

On Windows with VS Code stable, Cline’s storage is here:

- `%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\`

Checkpoints live under:

- `%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\checkpoints\<workspace-id>\.git`

**Important:** that `.git` directory is often **hidden**, so you must use `-Force` in PowerShell.

---

## Step 1 — Extract `lastCheckpointHash` from task logs

If you already have the hash from a log line, skip this.

Otherwise, search task logs for `lastCheckpointHash`:

```powershell
$clineRoot = Join-Path $env:APPDATA "Code\User\globalStorage\saoudrizwan.claude-dev"
$tasksRoot = Join-Path $clineRoot "tasks"

# Find occurrences of lastCheckpointHash in any JSON logs
Select-String -Path (Join-Path $tasksRoot "*.json") -Recurse -Pattern '"lastCheckpointHash"\s*:\s*"[0-9a-f]{40}"' |
  Select-Object -First 20
```

If you want to extract the actual hashes:

```powershell
$logFile = "PATH\TO\THE\TASK\LOG.json"

$hashes = Select-String -Path $logFile -Pattern '"lastCheckpointHash"\s*:\s*"([0-9a-f]{40})"' -AllMatches |
  ForEach-Object { $_.Matches } |
  ForEach-Object { $_.Groups[1].Value } |
  Select-Object -Unique

$hashes
```

Pick the one you care about (usually the last).

---

## Step 2 — Find which checkpoint repo contains that commit

There can be multiple `<workspace-id>` folders under `checkpoints\`.
The fastest method is: **scan them** and ask Git whether the hash exists.

```powershell
$clineRoot = Join-Path $env:APPDATA "Code\User\globalStorage\saoudrizwan.claude-dev"
$cpRoot    = Join-Path $clineRoot "checkpoints"

$hash = "bc36122624231f1e64d0768fabf3bfc7f3eecfef" # <-- your lastCheckpointHash

$hits = @()

foreach ($ws in Get-ChildItem $cpRoot -Directory -Force) {
  foreach ($gitName in @(".git", ".git_disabled")) {
    $gd = Join-Path $ws.FullName $gitName
    if (Test-Path $gd) {
      git --git-dir "$gd" cat-file -e "$hash^{commit}" 2>$null
      if ($LASTEXITCODE -eq 0) { $hits += $gd }
    }
  }
}

$hits
```

If this prints a path like:

- `...\checkpoints\4184916832\.git`

…that’s your shadow repo for this task.

---

## Step 3 — Reproduce the “Compare” (step diff)

Once you have the shadow `.git` directory:

```powershell
$shadowGit = $hits[0]
$cur  = $hash
$prev = (git --git-dir "$shadowGit" rev-parse "$cur^").Trim()

git --git-dir "$shadowGit" diff $prev $cur > cline-step.patch
git --git-dir "$shadowGit" diff --name-only $prev $cur
```

This gives you a real patch file (`cline-step.patch`) matching the checkpoint-to-checkpoint diff.

---

## Step 4 — Export the whole task as a clean patch (the “real win”)

This is the part that makes Cline’s UI reproducible in your own tooling:

1) derive the **workspace id** from the checkpoint repo path  
2) derive the **task id** from the checkpoint commit subject  
3) gather all commits belonging to the same task  
4) diff from `start^` → `HEAD`  
5) exclude build noise (e.g., `src-tauri/target`)

Here’s the exact PowerShell pattern that worked for me:

```powershell
$shadowGit = $hits[0]

# workspace id is the folder name above .git
$wsId = Split-Path (Split-Path $shadowGit -Parent) -Leaf

# commit subject typically looks like: checkpoint-<wsId>-<taskId>
$subject = (git --git-dir "$shadowGit" log -1 --pretty=%s).Trim()
$taskId  = ($subject -replace "^checkpoint-$wsId-","")

# all commits that belong to this task id
$commits = @(git --git-dir "$shadowGit" log --pretty=%H --grep "checkpoint-$wsId-$taskId")

# oldest checkpoint commit for this task
$start = $commits[-1].Trim()
$end   = (git --git-dir "$shadowGit" rev-parse HEAD).Trim()
$base  = (git --git-dir "$shadowGit" rev-parse "$start^").Trim()

# produce a clean patch (exclude build output)
git --git-dir "$shadowGit" diff $base $end -- . ":(exclude)src-tauri/target" > cline-task-clean.patch

# sanity: list exactly which files the task touched (minus excluded paths)
git --git-dir "$shadowGit" diff --name-only $base $end -- . ":(exclude)src-tauri/target"
```

That last command should print a tight, task-scoped file list (mine looked like):

- `package-lock.json`
- `package.json`
- `src-tauri/capabilities/default.json`
- `src-tauri/gen/schemas/capabilities.json`
- `src-tauri/src/api/handlers.rs`
- `src-tauri/tauri.conf.json`
- `src/modules/jira-details/JiraDetailsPanel.svelte`

---

## A real example patch (sanitized)

This is a real exported task patch. I redacted the Jira host to `example.atlassian.net`.

```diff
diff --git a/package-lock.json b/package-lock.json
index c93f6a4..9fef6c9 100644
--- a/package-lock.json
+++ b/package-lock.json
@@ -9,6 +9,7 @@
       "version": "0.0.1",
       "dependencies": {
         "@tauri-apps/api": "^2.0.0",
+        "@tauri-apps/plugin-shell": "^2.3.5",
         "@tauri-apps/plugin-store": "^2.0.0"
       },
       "devDependencies": {
@@ -1187,9 +1188,9 @@
       }
     },
     "node_modules/@tauri-apps/api": {
-      "version": "2.9.1",
-      "resolved": "https://registry.npmjs.org/@tauri-apps/api/-/api-2.9.1.tgz",
-      "integrity": "sha512-IGlhP6EivjXHepbBic618GOmiWe4URJiIeZFlB7x3czM0yDHHYviH1Xvoiv4FefdkQtn6v7TuwWCRfOGdnVUGw==",
+      "version": "2.10.1",
+      "resolved": "https://registry.npmjs.org/@tauri-apps/api/-/api-2.10.1.tgz",
+      "integrity": "sha512-hKL/jWf293UDSUN09rR69hrToyIXBb8CjGaWC7gfinvnQrBVvnLr08FeFi38gxtugAVyVcTa5/FD/Xnkb1siBw==",
       "license": "Apache-2.0 OR MIT",
       "funding": {
         "type": "opencollective",
@@ -1413,6 +1414,15 @@
         "node": ">= 10"
       }
     },
+    "node_modules/@tauri-apps/plugin-shell": {
+      "version": "2.3.5",
+      "resolved": "https://registry.npmjs.org/@tauri-apps/plugin-shell/-/plugin-shell-2.3.5.tgz",
+      "integrity": "sha512-jewtULhiQ7lI7+owCKAjc8tYLJr92U16bPOeAa472LHJdgaibLP83NcfAF2e+wkEcA53FxKQAZ7byDzs2eeizg==",
+      "license": "MIT OR Apache-2.0",
+      "dependencies": {
+        "@tauri-apps/api": "^2.10.1"
+      }
+    },
     "node_modules/@tauri-apps/plugin-store": {
       "version": "2.4.2",
       "resolved": "https://registry.npmjs.org/@tauri-apps/plugin-store/-/plugin-store-2.4.2.tgz",
diff --git a/package.json b/package.json
index cc9c202..612a085 100644
--- a/package.json
+++ b/package.json
@@ -20,6 +20,7 @@
   },
   "dependencies": {
     "@tauri-apps/api": "^2.0.0",
+    "@tauri-apps/plugin-shell": "^2.3.5",
     "@tauri-apps/plugin-store": "^2.0.0"
   }
 }
diff --git a/src-tauri/capabilities/default.json b/src-tauri/capabilities/default.json
new file mode 100644
index 0000000..cdb5351
--- /dev/null
+++ b/src-tauri/capabilities/default.json
@@ -0,0 +1,20 @@
+{
+  "$schema": "../gen/schemas/desktop-schema.json",
+  "identifier": "default",
+  "description": "Default capabilities for the app",
+  "windows": ["main"],
+  "permissions": [
+    "core:default",
+    "shell:allow-open",
+    {
+      "identifier": "shell:allow-execute",
+      "allow": [
+        {
+          "name": "open-chrome",
+          "cmd": "cmd",
+          "args": true
+        }
+      ]
+    }
+  ]
+}
diff --git a/src-tauri/gen/schemas/capabilities.json b/src-tauri/gen/schemas/capabilities.json
index 9e26dfe..b8be7a8 100644
--- a/src-tauri/gen/schemas/capabilities.json
+++ b/src-tauri/gen/schemas/capabilities.json
@@ -1 +1 @@
-{} \ No newline at end of file
+{"default":{"identifier":"default","description":"Default capabilities for the app","local":true,"windows":["main"],"permissions":["core:default","shell:allow-open",{"identifier":"shell:allow-execute","allow":[{"args":true,"cmd":"cmd","name":"open-chrome"}]}]}} \ No newline at end of file
diff --git a/src-tauri/src/api/handlers.rs b/src-tauri/src/api/handlers.rs
index f74fe7a..c2e2cb4 100644
--- a/src-tauri/src/api/handlers.rs
+++ b/src-tauri/src/api/handlers.rs
@@ -224,7 +224,7 @@ pub async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthRe
       (status = 500, description = "Internal server error", body = ErrorResponse)
     ),
     security(("bearerAuth" = [])),
-    tags = ["jira", "tool"]
+    tags = ["jira", "tool", "gpt"]
 )]
 pub async fn jira_list_handler(
     State(state): State<Arc<AppState>>,
diff --git a/src-tauri/tauri.conf.json b/src-tauri/tauri.conf.json
index 6bdd1d1..84bd72f 100644
--- a/src-tauri/tauri.conf.json
+++ b/src-tauri/tauri.conf.json
@@ -32,5 +32,16 @@
       "icon": [
         "icons/icon.ico"
       ]
+    },
+    "plugins": {
+      "shell": {
+        "scope": [
+          {
+            "name": "open-chrome",
+            "cmd": "cmd",
+            "args": ["/c", "start", "chrome", { "validator": "\\S+" }]
+          }
+        ]
+      }
     }
   }
 }
diff --git a/src/modules/jira-details/JiraDetailsPanel.svelte b/src/modules/jira-details/JiraDetailsPanel.svelte
index 69cb60c..32ddb90 100644
--- a/src/modules/jira-details/JiraDetailsPanel.svelte
+++ b/src/modules/jira-details/JiraDetailsPanel.svelte
@@ -1,4 +1,5 @@
 <script lang="ts">
+  import { Command } from "@tauri-apps/plugin-shell";
   import type { IssueDetails } from "../../types";
   import { getStatusClass } from "./utils";
   import JiraMetadataGrid from "./JiraMetadataGrid.svelte";
@@ -18,6 +19,12 @@
   type DetailTab = 'details';
   let activeDetailTab: DetailTab = $state('details');

+  // Open URL in Chrome
+  async function openInChrome(url: string) {
+    const command = Command.create("open-chrome", [url]);
+    await command.spawn();
+  }
 </script>
@@ -59,23 +66,39 @@
-  {#if onRefresh}
+  <div class="ml-auto flex items-center gap-1">
   <button
-    onclick={onRefresh}
-    disabled={loading}
-    class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ml-auto"
-    title="Refresh issue details"
+    onclick={() => openInChrome(`https://example.atlassian.net/browse/${issue.key}`)}
+    class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors"
+    title="Open in Chrome"
   >
     <svg
-      class="w-5 h-5 {loading ? 'animate-spin' : ''}"
+      class="w-5 h-5"
       fill="none"
       stroke="currentColor"
       viewBox="0 0 24 24"
     >
-      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
+      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
     </svg>
   </button>
-  {/if}
+  {#if onRefresh}
+    <button
+      onclick={onRefresh}
+      disabled={loading}
+      class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
+      title="Refresh issue details"
+    >
+      <svg class="w-5 h-5 {loading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
+        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
+      </svg>
+    </button>
+  {/if}
+  </div>
```

---

## Why your exported patch can be “noisy”

If you export diffs without exclusions, you’ll often pick up build artifacts (`target/`, `.fingerprint/`, generated schemas, lockfiles).

Two practical rules:

- **Always exclude build output** (`target/`, `dist/`, `.next/`, etc.)
- Decide whether generated files belong in the patch (sometimes yes, often no)

Example exclusion pattern (works well for Rust/Tauri):

```powershell
git --git-dir "$shadowGit" diff $base $end -- . ":(exclude)src-tauri/target" > cline-task-clean.patch
```

---

## Make it a one-command tool (optional)

Once you trust the workflow, wrap it into a function so you can type one command after a task:

```powershell
function Export-ClineTaskPatch {
  param(
    [Parameter(Mandatory=$true)][string]$Hash,
    [string]$ExcludePath = "src-tauri/target"
  )

  $clineRoot = Join-Path $env:APPDATA "Code\User\globalStorage\saoudrizwan.claude-dev"
  $cpRoot    = Join-Path $clineRoot "checkpoints"

  $shadowGit = $null
  foreach ($ws in Get-ChildItem $cpRoot -Directory -Force) {
    foreach ($gitName in @(".git", ".git_disabled")) {
      $gd = Join-Path $ws.FullName $gitName
      if (Test-Path $gd) {
        git --git-dir "$gd" cat-file -e "$Hash^{commit}" 2>$null
        if ($LASTEXITCODE -eq 0) { $shadowGit = $gd; break }
      }
    }
    if ($shadowGit) { break }
  }

  if (-not $shadowGit) { throw "Could not find checkpoint repo containing hash $Hash" }

  $wsId    = Split-Path (Split-Path $shadowGit -Parent) -Leaf
  $subject = (git --git-dir "$shadowGit" show -s --format=%s $Hash).Trim()
  $taskId  = ($subject -replace "^checkpoint-$wsId-","")

  $commits = @(git --git-dir "$shadowGit" log --pretty=%H --grep "checkpoint-$wsId-$taskId")
  $start   = $commits[-1].Trim()
  $end     = (git --git-dir "$shadowGit" rev-parse HEAD).Trim()
  $base    = (git --git-dir "$shadowGit" rev-parse "$start^").Trim()

  $out = "cline-task-clean.patch"
  git --git-dir "$shadowGit" diff $base $end -- . ":(exclude)$ExcludePath" > $out

  Write-Host "Wrote $out"
  Write-Host "Files touched:"
  git --git-dir "$shadowGit" diff --name-only $base $end -- . ":(exclude)$ExcludePath"
}
```

Usage:

```powershell
Export-ClineTaskPatch -Hash "bc36122624231f1e64d0768fabf3bfc7f3eecfef"
```

---

## Takeaways

- **“View Changes” is task-scoped** because it’s driven by **checkpoint commits**, not your repo’s state.
- You can export Cline’s exact task diff as a real patch by:
  - locating the checkpoint repo that contains `lastCheckpointHash`
  - diffing from the task chain’s start baseline (`start^`) to `HEAD`
  - excluding noisy paths like `target/`

Once you have this, you can build your own “Export Patch” button, attach patches to Jira, or keep an audit trail of agent runs—without needing Cline’s UI.

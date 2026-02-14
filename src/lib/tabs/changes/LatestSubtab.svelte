<script lang="ts">
  import { onMount } from "svelte";
  import { fetchLatest, fetchSubtaskDiff, fetchFileContents } from "./api";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import type { LatestResponse, DiffResult, DiffFile, SubtaskSummaryItem } from "./types";
  import type { ChatAttachment } from "../agent/types";

  // ---- State ----
  let loading = $state(true);
  let error: string | null = $state(null);
  let data: LatestResponse | null = $state(null);

  // Active view: 'task' = full task diff, or subtask index number
  let activeView: 'task' | number = $state('task');

  // Per-subtask diff cache: subtaskIndex -> { diff, loading, error }
  let subtaskDiffs: Map<number, { diff: DiffResult | null; loading: boolean; error: string | null }> = $state(new Map());

  let copyLabel = $state('📋 Copy Diff');
  let showGitCmds = $state(false);
  let gitCmdCopyLabel = $state('📋');
  let askLlmLoading = $state(false);

  onMount(() => {
    loadLatest();
  });

  async function loadLatest() {
    loading = true;
    error = null;
    data = null;
    activeView = 'task';
    subtaskDiffs = new Map();
    copyLabel = '📋 Copy Diff';
    try {
      // Default scope=task (full task diff)
      data = await fetchLatest('task');
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
    }
  }

  function switchToTask() {
    activeView = 'task';
    copyLabel = '📋 Copy Diff';
  }

  async function switchToSubtask(index: number) {
    activeView = index;
    copyLabel = '📋 Copy Diff';

    // If we already have this subtask's diff cached, don't refetch
    if (subtaskDiffs.has(index)) return;

    // Need taskId and workspaceId from the loaded data
    if (!data || !data.workspaceId) return;

    // Mark as loading
    subtaskDiffs.set(index, { diff: null, loading: true, error: null });
    subtaskDiffs = new Map(subtaskDiffs); // trigger reactivity

    try {
      const diff = await fetchSubtaskDiff(data.taskId, index, data.workspaceId);
      subtaskDiffs.set(index, { diff, loading: false, error: null });
      subtaskDiffs = new Map(subtaskDiffs);
    } catch (e: any) {
      subtaskDiffs.set(index, { diff: null, loading: false, error: e.message || String(e) });
      subtaskDiffs = new Map(subtaskDiffs);
    }
  }

  // Get the currently active diff
  function getActiveDiff(): DiffResult | null {
    if (activeView === 'task') {
      return data?.diff ?? null;
    }
    const cached = subtaskDiffs.get(activeView as number);
    return cached?.diff ?? null;
  }

  function getActiveLoading(): boolean {
    if (activeView === 'task') return false; // task diff loaded with initial fetch
    const cached = subtaskDiffs.get(activeView as number);
    return cached?.loading ?? false;
  }

  function getActiveError(): string | null {
    if (activeView === 'task') return null;
    const cached = subtaskDiffs.get(activeView as number);
    return cached?.error ?? null;
  }

  // Get prompt for active view
  function getActivePrompt(): string {
    if (activeView === 'task') {
      return data?.prompt ?? '';
    }
    const subtask = data?.subtasks?.find(s => s.subtaskIndex === activeView);
    return subtask?.prompt ?? data?.prompt ?? '';
  }

  function getActiveSubtask(): SubtaskSummaryItem | null {
    if (activeView === 'task' || !data?.subtasks) return null;
    return data.subtasks.find(s => s.subtaskIndex === activeView) ?? null;
  }

  function copyDiff() {
    const diff = getActiveDiff();
    if (diff?.patch) {
      navigator.clipboard.writeText(diff.patch);
      copyLabel = '✓ Copied!';
      setTimeout(() => copyLabel = '📋 Copy Diff', 1500);
    }
  }

  function copyPrompt() {
    if (activeView === 'task' && data?.subtasks && data.subtasks.length > 0) {
      // Copy all prompts in sequence for Full Task view
      const allPrompts = data.subtasks.map(s =>
        `${s.isInitialTask ? '🎯 Initial Task' : `💬 Feedback #${s.subtaskIndex}`} (${formatDate(s.timestamp)}):\n${s.prompt}`
      ).join('\n\n---\n\n');
      navigator.clipboard.writeText(allPrompts);
    } else {
      const prompt = getActivePrompt();
      if (prompt) {
        navigator.clipboard.writeText(prompt);
      }
    }
  }

  function formatDate(iso: string): string {
    try {
      const d = new Date(iso);
      return d.toLocaleString(undefined, {
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit', second: '2-digit'
      });
    } catch {
      return iso;
    }
  }

  function shortHash(ref: string): string {
    return ref.substring(0, 8);
  }

  function statusColor(status: string): string {
    switch (status) {
      case 'added': return 'text-green-600';
      case 'deleted': return 'text-red-600';
      case 'renamed': return 'text-purple-600';
      default: return 'text-yellow-600';
    }
  }

  function statusIcon(status: string): string {
    switch (status) {
      case 'added': return '+';
      case 'deleted': return '−';
      case 'renamed': return '→';
      default: return '~';
    }
  }

  function subtaskLabel(s: SubtaskSummaryItem): string {
    return s.isInitialTask ? '🎯 Initial' : `💬 #${s.subtaskIndex}`;
  }

  // ============== Diff Parsing Helpers ==============

  /**
   * Parse a unified diff patch into per-file sections.
   * Returns a formatted string with each file's path, status, and its diff body.
   */
  function extractPerFileDiffs(patch: string, files: DiffFile[]): string {
    // Split patch on "diff --git" boundaries
    const fileSections: string[] = [];
    const diffRegex = /^diff --git /m;
    const parts = patch.split(diffRegex);

    // Build a lookup: normalized path -> DiffFile for status info
    const fileMap = new Map<string, DiffFile>();
    for (const f of files) {
      fileMap.set(f.path, f);
    }

    for (const part of parts) {
      if (!part.trim()) continue;

      // Reconstruct the full section with the "diff --git" prefix
      const section = 'diff --git ' + part;

      // Extract file path from the first line: "diff --git a/path b/path"
      const headerMatch = section.match(/^diff --git a\/(.+?) b\/(.+)/m);
      const filePath = headerMatch ? headerMatch[2] : '(unknown)';

      // Find file metadata
      const fileMeta = fileMap.get(filePath);
      const status = fileMeta ? fileMeta.status.toUpperCase() : 'MODIFIED';
      const added = fileMeta ? fileMeta.linesAdded : 0;
      const removed = fileMeta ? fileMeta.linesRemoved : 0;

      fileSections.push(
        `${'='.repeat(60)}\n` +
        `FILE: ${filePath}\n` +
        `STATUS: ${status}  (+${added} -${removed})\n` +
        `${'='.repeat(60)}\n` +
        section.trimEnd()
      );
    }

    if (fileSections.length === 0) {
      // Fallback: couldn't parse, return the raw patch
      return patch;
    }

    return fileSections.join('\n\n');
  }

  // ============== Ask LLM ==============

  async function askLlm() {
    if (!data) return;

    askLlmLoading = true;
    try {
      const atts: ChatAttachment[] = [];

      // 1. All prompts
      if (data.subtasks && data.subtasks.length > 0) {
        const allPrompts = data.subtasks.map(s =>
          `${s.isInitialTask ? '🎯 Initial Task' : `💬 Feedback #${s.subtaskIndex}`} (${formatDate(s.timestamp)}):\n${s.prompt}`
        ).join('\n\n---\n\n');
        atts.push({
          id: `prompts-${Date.now()}`,
          label: `All Prompts (${data.subtasks.length})`,
          type: 'prompts',
          content: allPrompts,
          meta: { count: data.subtasks.length }
        });
      } else if (data.prompt) {
        atts.push({
          id: `prompt-${Date.now()}`,
          label: 'Task Prompt',
          type: 'prompts',
          content: data.prompt
        });
      }

      // 2. Actual file contents fetched from shadow git
      const diff = data.diff;
      if (diff && diff.files.length > 0 && data.workspaceId) {
        // Separate non-deleted files (fetch at toRef) and deleted files (note only)
        const nonDeletedPaths = diff.files
          .filter(f => f.status !== 'deleted')
          .map(f => f.path);
        const deletedFiles = diff.files.filter(f => f.status === 'deleted');

        let fileBodySections: string[] = [];

        // Fetch actual file bodies from shadow git at toRef (latest state)
        if (nonDeletedPaths.length > 0) {
          try {
            console.log(`[askLlm] Fetching ${nonDeletedPaths.length} file bodies from shadow git (ref=${diff.toRef.substring(0, 8)})...`);
            const fileContentsResp = await fetchFileContents(data.workspaceId, diff.toRef, nonDeletedPaths);

            // Build file metadata lookup
            const fileMeta = new Map(diff.files.map(f => [f.path, f]));

            for (const fc of fileContentsResp.files) {
              const meta = fileMeta.get(fc.path);
              const status = meta ? meta.status.toUpperCase() : 'MODIFIED';
              const added = meta ? meta.linesAdded : 0;
              const removed = meta ? meta.linesRemoved : 0;

              if (fc.content !== null) {
                fileBodySections.push(
                  `${'='.repeat(60)}\n` +
                  `FILE: ${fc.path}\n` +
                  `STATUS: ${status}  (+${added} -${removed})  SIZE: ${fc.size ?? fc.content.length} bytes\n` +
                  `${'='.repeat(60)}\n` +
                  fc.content
                );
              } else {
                fileBodySections.push(
                  `${'='.repeat(60)}\n` +
                  `FILE: ${fc.path}\n` +
                  `STATUS: ${status}  [content unavailable: ${fc.error || 'unknown error'}]\n` +
                  `${'='.repeat(60)}`
                );
              }
            }

            console.log(`[askLlm] Retrieved ${fileContentsResp.retrieved}/${nonDeletedPaths.length} files (${(fileContentsResp.totalSize / 1024).toFixed(1)}KB)`);
          } catch (e: any) {
            console.warn('[askLlm] Failed to fetch file contents, falling back to diff sections:', e.message);
            // Fallback: use per-file diff sections from patch
            if (diff.patch) {
              const perFileContent = extractPerFileDiffs(diff.patch, diff.files);
              fileBodySections = [perFileContent];
            }
          }
        }

        // Note deleted files
        for (const df of deletedFiles) {
          fileBodySections.push(
            `${'='.repeat(60)}\n` +
            `FILE: ${df.path}\n` +
            `STATUS: DELETED  (-${df.linesRemoved} lines)\n` +
            `${'='.repeat(60)}\n` +
            `[File was deleted in this task]`
          );
        }

        if (fileBodySections.length > 0) {
          const fullContent = fileBodySections.join('\n\n');
          atts.push({
            id: `files-${Date.now()}`,
            label: `File Contents (${diff.files.length} files, ${(fullContent.length / 1024).toFixed(1)}KB)`,
            type: 'files',
            content: fullContent,
            meta: { count: diff.files.length, sizeKB: Math.round(fullContent.length / 1024) }
          });
        }
      } else if (diff && diff.files.length > 0 && diff.patch) {
        // Fallback: no workspace ID, use per-file diff sections
        const perFileContent = extractPerFileDiffs(diff.patch, diff.files);
        const totalSize = perFileContent.length;
        atts.push({
          id: `files-${Date.now()}`,
          label: `Changed Files (${diff.files.length}, ${(totalSize / 1024).toFixed(1)}KB)`,
          type: 'files',
          content: perFileContent,
          meta: { count: diff.files.length, sizeKB: Math.round(totalSize / 1024) }
        });
      }

      // 3. Unified diff patch
      if (diff && diff.patch) {
        atts.push({
          id: `diff-${Date.now()}`,
          label: `Unified Diff (${(diff.patch.length / 1024).toFixed(1)}KB)`,
          type: 'diff',
          content: diff.patch,
          meta: { sizeKB: Math.round(diff.patch.length / 1024) }
        });
      }

      console.log('[askLlm] data.diff:', data.diff ? `${data.diff.files.length} files, ${data.diff.patch?.length ?? 0} bytes patch` : 'NULL (no checkpoint)');
      console.log('[askLlm] attachments to send:', atts.length, atts.map(a => a.label));

      // Navigate to Agent → Chat with payload
      navigationStore.navigateToChat({
        attachments: atts,
        timestamp: Date.now()
      });
    } finally {
      askLlmLoading = false;
    }
  }
</script>

<div class="flex-1 p-6 overflow-auto">
  <!-- Header -->
  <div class="flex items-center justify-between mb-4">
    <div>
      <h2 class="text-lg font-semibold text-gray-900 flex items-center gap-2">
        ⚡ Latest Task
        <span class="text-xs font-normal text-gray-500">
          Full task diff + subtask drill-down
        </span>
      </h2>
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={askLlm}
        disabled={loading || !data || askLlmLoading}
        class="px-4 py-1.5 text-sm font-medium text-white bg-purple-600 rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        title="Send task artifacts (with full file contents) to Chat"
      >
        {askLlmLoading ? '⏳ Loading files...' : '🤖 Ask LLM'}
      </button>
      <button
        onclick={loadLatest}
        disabled={loading}
        class="px-4 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {loading ? 'Loading...' : '↻ Refresh'}
      </button>
    </div>
  </div>

  <!-- Loading -->
  {#if loading}
    <div class="flex items-center justify-center py-20">
      <div class="text-center">
        <svg class="animate-spin h-8 w-8 text-blue-500 mx-auto mb-3" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <p class="text-gray-500 text-sm">Resolving latest task...</p>
      </div>
    </div>

  <!-- Error -->
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-lg p-4">
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-red-500 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
        </svg>
        <div>
          <p class="text-sm font-medium text-red-800">Failed to load latest</p>
          <p class="text-sm text-red-600 mt-1">{error}</p>
        </div>
      </div>
    </div>

  <!-- Data loaded -->
  {:else if data}
    <div class="space-y-4">

      <!-- Identity Card -->
      <div class="bg-white border border-gray-200 rounded-lg p-4 shadow-sm">
        <div class="flex items-start justify-between">
          <div>
            <div class="flex items-center gap-3 mb-1">
              <span class="text-sm font-bold text-gray-900">📋 Task:</span>
              <code class="text-sm font-mono bg-gray-100 px-2 py-0.5 rounded">{data.taskId}</code>
              <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
            </div>
            <div class="flex items-center gap-4 text-xs text-gray-500">
              <span>🕐 {formatDate(data.taskStartedAt)}</span>
              {#if data.totalSubtasks > 0}
                <span class="font-medium text-gray-700">{data.totalSubtasks} subtask{data.totalSubtasks !== 1 ? 's' : ''}</span>
              {/if}
              {#if data.workspaceId}
                <span>WS: <code class="font-mono">{data.workspaceId}</code></span>
              {/if}
            </div>
          </div>
          <div class="flex items-center gap-2 text-xs text-gray-500">
            <span>{data.messageCount} msgs</span>
            <span>{data.toolCallCount} tools</span>
          </div>
        </div>
      </div>

      <!-- ============ Subtask Tab Bar ============ -->
      {#if data.subtasks && data.subtasks.length > 0}
        <div class="bg-white border border-gray-200 rounded-lg shadow-sm overflow-hidden">
          <div class="flex items-center gap-0 border-b border-gray-200 overflow-x-auto">
            <!-- Full Task tab (always first) -->
            <button
              onclick={switchToTask}
              class="flex-shrink-0 px-4 py-2.5 text-xs font-semibold border-b-2 transition-colors whitespace-nowrap {activeView === 'task'
                ? 'border-purple-500 text-purple-700 bg-purple-50'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
            >
              📋 Full Task
              {#if data.diff}
                <span class="ml-1.5 text-[10px] font-normal text-gray-400">
                  {data.diff.files.length} files
                </span>
              {/if}
            </button>

            <!-- Divider -->
            <div class="w-px h-6 bg-gray-200 flex-shrink-0"></div>

            <!-- Subtask tabs -->
            {#each data.subtasks as subtask}
              <button
                onclick={() => switchToSubtask(subtask.subtaskIndex)}
                class="flex-shrink-0 px-3 py-2.5 text-xs font-medium border-b-2 transition-colors whitespace-nowrap {activeView === subtask.subtaskIndex
                  ? 'border-teal-500 text-teal-700 bg-teal-50'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:bg-gray-50'}"
              >
                {subtaskLabel(subtask)}
                <span class="ml-1 text-[10px] font-normal text-gray-400">
                  {subtask.toolCallCount}🔧
                </span>
              </button>
            {/each}
          </div>

          <!-- Active View Content -->
          <div class="p-0">
            {#if activeView === 'task'}
              <!-- Full Task View (already loaded) -->
              <!-- Nothing extra needed, diff renders below -->
            {:else}
              <!-- Subtask view: show loading / error / subtask-specific info -->
              {#if getActiveLoading()}
                <div class="flex items-center justify-center py-8">
                  <div class="text-center">
                    <svg class="animate-spin h-6 w-6 text-teal-500 mx-auto mb-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    <p class="text-gray-500 text-xs">Loading subtask #{activeView} diff...</p>
                  </div>
                </div>
              {:else if getActiveError()}
                <div class="m-4 bg-red-50 border border-red-200 rounded-lg p-3">
                  <p class="text-sm font-medium text-red-800">Subtask diff failed</p>
                  <p class="text-xs text-red-600 mt-1">{getActiveError()}</p>
                </div>
              {/if}
            {/if}
          </div>
        </div>
      {/if}

      <!-- Subtask metadata bar (when viewing a subtask) -->
      {#if activeView !== 'task' && getActiveSubtask()}
        {@const sub = getActiveSubtask()!}
        <div class="bg-teal-50 border border-teal-200 rounded-lg px-4 py-2.5 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-bold {sub.isInitialTask ? 'bg-teal-600 text-white' : 'bg-teal-100 text-teal-700'}">
              {sub.isInitialTask ? '🎯 Initial Task' : `💬 Feedback #${sub.subtaskIndex}`}
            </span>
            <span class="text-xs text-teal-600">🕐 {formatDate(sub.timestamp)}</span>
          </div>
          <div class="flex items-center gap-3 text-xs text-teal-600">
            <span>{sub.messageCount} msgs</span>
            <span>{sub.toolCallCount} tool calls</span>
          </div>
        </div>
      {/if}

      <!-- Prompt(s) -->
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
        <div class="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center justify-between">
          <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">
            {#if activeView === 'task' && data.subtasks && data.subtasks.length > 1}
              All Prompts ({data.subtasks.length})
            {:else if activeView === 'task'}
              Task Prompt
            {:else}
              Subtask Prompt
            {/if}
          </span>
          <button
            onclick={copyPrompt}
            class="text-[10px] font-medium px-2 py-0.5 rounded bg-gray-200 hover:bg-gray-300 text-gray-600 transition-colors"
          >
            📋 Copy{activeView === 'task' && data.subtasks && data.subtasks.length > 1 ? ' All' : ''}
          </button>
        </div>
        {#if activeView === 'task' && data.subtasks && data.subtasks.length > 0}
          <!-- Full Task view: show ALL subtask prompts in sequence -->
          <div class="divide-y divide-gray-100">
            {#each data.subtasks as subtask}
              <div class="p-4">
                <div class="flex items-center gap-2 mb-2">
                  <span class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold {subtask.isInitialTask ? 'bg-teal-600 text-white' : 'bg-gray-200 text-gray-600'}">
                    {subtask.isInitialTask ? '🎯 Initial' : `💬 #${subtask.subtaskIndex}`}
                  </span>
                  <span class="text-[10px] text-gray-400">{formatDate(subtask.timestamp)}</span>
                  <span class="text-[10px] text-gray-400">· {subtask.toolCallCount}🔧 · {subtask.messageCount} msgs</span>
                </div>
                <p class="text-sm text-gray-800 whitespace-pre-wrap break-words leading-relaxed">{subtask.prompt || '(empty prompt)'}</p>
              </div>
            {/each}
          </div>
        {:else}
          <!-- Single prompt (subtask view or no subtasks) -->
          <div class="p-4">
            <p class="text-sm text-gray-800 whitespace-pre-wrap break-words leading-relaxed">{getActivePrompt() || '(empty prompt)'}</p>
          </div>
        {/if}
      </div>

      <!-- Tools Used (for active view) -->
      {#if activeView === 'task' && data.toolsUsed.length > 0}
        <div class="flex flex-wrap gap-1.5">
          {#each data.toolsUsed as tool}
            <span class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium bg-gray-100 text-gray-600 border border-gray-200">
              🔧 {tool}
            </span>
          {/each}
        </div>
      {:else if activeView !== 'task' && getActiveSubtask()?.toolsUsed?.length}
        <div class="flex flex-wrap gap-1.5">
          {#each getActiveSubtask()!.toolsUsed as tool}
            <span class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium bg-gray-100 text-gray-600 border border-gray-200">
              🔧 {tool}
            </span>
          {/each}
        </div>
      {/if}

      <!-- Diff Content (shared renderer for task or subtask) -->
      {#if !getActiveLoading() && !getActiveError()}
        {@const activeDiff = getActiveDiff()}
        {#if activeDiff}
          <!-- Diff stats bar -->
          <div class="flex items-center gap-3 text-xs">
            <span class="inline-flex items-center px-2.5 py-1 rounded-lg font-bold bg-gray-800 text-white tabular-nums">
              📦 {(activeDiff.patch.length / 1024).toFixed(1)} KB
            </span>
            <span class="inline-flex items-center px-2 py-1 rounded-lg font-bold bg-green-50 text-green-700 border border-green-200 tabular-nums">
              +{activeDiff.files.reduce((s, f) => s + f.linesAdded, 0)}
            </span>
            <span class="inline-flex items-center px-2 py-1 rounded-lg font-bold bg-red-50 text-red-700 border border-red-200 tabular-nums">
              −{activeDiff.files.reduce((s, f) => s + f.linesRemoved, 0)}
            </span>
            <span class="text-gray-400 font-mono text-[10px]">
              {shortHash(activeDiff.fromRef)} → {shortHash(activeDiff.toRef)}
            </span>
          </div>

          <!-- Git Commands Toggle -->
          {#if activeDiff.gitCommands?.length}
            <div class="flex items-center gap-2">
              <button
                class="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1"
                onclick={() => showGitCmds = !showGitCmds}
              >
                <span class="font-mono">{showGitCmds ? '▾' : '▸'}</span>
                <span>Git Commands ({activeDiff.gitCommands.length})</span>
              </button>
              {#if showGitCmds}
                <button
                  onclick={() => { navigator.clipboard.writeText(activeDiff!.gitCommands!.join('\n')); gitCmdCopyLabel = '✓'; setTimeout(() => gitCmdCopyLabel = '📋', 1500); }}
                  class="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
                  title="Copy git commands"
                >{gitCmdCopyLabel}</button>
              {/if}
            </div>
            {#if showGitCmds}
              <pre class="text-xs bg-gray-900 text-green-400 p-3 rounded font-mono select-text whitespace-pre-wrap break-all">{activeDiff.gitCommands.join('\n')}</pre>
            {/if}
          {/if}

          <!-- Changed Files -->
          <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
            <div class="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center justify-between">
              <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">
                Changed Files ({activeDiff.files.length})
              </span>
              <span class="text-[10px] text-gray-400">
                {Math.round(activeDiff.patch.length / 1024)}KB patch
              </span>
            </div>
            <div class="p-3">
              {#each activeDiff.files as f}
                <div class="flex items-center gap-2 py-1 text-xs font-mono">
                  <span class="{statusColor(f.status)} font-bold w-3 text-center">{statusIcon(f.status)}</span>
                  <span class="text-gray-700 truncate flex-1">{f.path}</span>
                  <span class="text-green-600 tabular-nums">+{f.linesAdded}</span>
                  <span class="text-red-600 tabular-nums">-{f.linesRemoved}</span>
                </div>
              {/each}
            </div>
          </div>

          <!-- Unified Diff Patch -->
          <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
            <div class="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center justify-between sticky top-0 z-10">
              <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">
                Unified Diff {activeView === 'task' ? '(Full Task)' : `(Subtask #${activeView})`}
              </span>
              <div class="flex items-center gap-2">
                {#if activeDiff.patch}
                  <button
                    onclick={copyDiff}
                    class="text-xs font-medium px-3 py-1 rounded bg-blue-600 hover:bg-blue-700 text-white transition-colors shadow-sm"
                  >
                    {copyLabel}
                  </button>
                {/if}
              </div>
            </div>
            {#if activeDiff.patch}
              <pre class="diff-scroll" style="background: #111827; color: #e5e7eb; font-size: 10px; line-height: 16px; padding: 12px; font-family: ui-monospace, monospace; white-space: pre-wrap; word-break: break-all; overflow-wrap: anywhere; user-select: text; margin: 0; width: 100%; box-sizing: border-box;">{activeDiff.patch}</pre>
            {:else}
              <div class="p-4 text-xs text-gray-400 italic">No patch content (empty diff)</div>
            {/if}
          </div>

        {:else if activeView === 'task' && data.noDiffReason}
          <!-- No diff for task scope -->
          <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-4">
            <div class="flex items-start gap-3">
              <span class="text-xl">⚠️</span>
              <div>
                <p class="text-sm font-medium text-yellow-800">No diff available</p>
                <p class="text-sm text-yellow-600 mt-1">
                  {#if data.noDiffReason === 'no_checkpoint_workspace'}
                    No checkpoint workspace found for this task. Checkpoints may be disabled or the data was deleted.
                  {:else}
                    {data.noDiffReason}
                  {/if}
                </p>
              </div>
            </div>
          </div>
        {/if}
      {/if}

      <!-- Context Footer -->
      <div class="text-[10px] text-gray-400 flex items-center gap-3 pt-2">
        <span>View: {activeView === 'task' ? 'Full Task' : `Subtask #${activeView}`}</span>
        {#if data.messageRangeStart !== null && data.messageRangeEnd !== null}
          <span>Messages: {data.messageRangeStart}–{data.messageRangeEnd}</span>
        {/if}
        <span>Task: {formatDate(data.taskStartedAt)}{data.taskEndedAt ? ` → ${formatDate(data.taskEndedAt)}` : ' (in progress?)'}</span>
      </div>
    </div>
  {/if}
</div>

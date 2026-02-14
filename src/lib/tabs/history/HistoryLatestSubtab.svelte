<script lang="ts">
  // History Latest Subtab — shows the most recent task from conversation history
  import { onMount } from "svelte";
  import { fetchHistoryTasks, fetchTaskDetail, fetchTaskSubtasks } from "./api";
  import type { TaskHistorySummary, TaskDetailResponse, SubtaskEntry } from "./types";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import type { ChatAttachment } from "../agent/types";

  interface Props {
    onViewDetail?: (taskId: string) => void;
  }

  let { onViewDetail }: Props = $props();

  // ---- State ----
  let loading = $state(true);
  let error: string | null = $state(null);
  let task: TaskHistorySummary | null = $state(null);
  let detail: TaskDetailResponse | null = $state(null);
  let subtasks: SubtaskEntry[] = $state([]);
  let detailLoading = $state(false);
  let subtasksLoading = $state(false);

  onMount(() => {
    loadLatest();
  });

  async function loadLatest() {
    loading = true;
    error = null;
    task = null;
    detail = null;
    subtasks = [];
    try {
      const resp = await fetchHistoryTasks(false, undefined, 1, 0);
      if (resp.tasks.length === 0) {
        error = 'No tasks found in conversation history.';
        return;
      }
      task = resp.tasks[0];

      // Fetch detail and subtasks in parallel
      detailLoading = true;
      subtasksLoading = true;

      const [detailResp, subtasksResp] = await Promise.allSettled([
        fetchTaskDetail(task.taskId),
        fetchTaskSubtasks(task.taskId),
      ]);

      if (detailResp.status === 'fulfilled') {
        detail = detailResp.value;
      }
      if (subtasksResp.status === 'fulfilled') {
        subtasks = subtasksResp.value.subtasks;
      }
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
      detailLoading = false;
      subtasksLoading = false;
    }
  }

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
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

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function timeSince(iso: string): string {
    try {
      const d = new Date(iso);
      const now = new Date();
      const diffMs = now.getTime() - d.getTime();
      const mins = Math.floor(diffMs / 60000);
      if (mins < 1) return 'just now';
      if (mins < 60) return `${mins}m ago`;
      const hours = Math.floor(mins / 60);
      if (hours < 24) return `${hours}h ago`;
      const days = Math.floor(hours / 24);
      return `${days}d ago`;
    } catch {
      return '';
    }
  }

  function topTools(breakdown: Record<string, number>, max: number = 8): [string, number][] {
    return Object.entries(breakdown)
      .sort(([, a], [, b]) => b - a)
      .slice(0, max);
  }

  function copyPrompt() {
    const text = task?.taskPrompt || detail?.taskPrompt || '';
    if (text) navigator.clipboard.writeText(text);
  }

  // ---- Ask LLM ----
  let askLlmLoading = $state(false);

  async function askLlm() {
    if (!task && !detail) return;
    askLlmLoading = true;
    try {
      const atts: ChatAttachment[] = [];

      // 1. Prompts (subtasks or single task prompt)
      if (subtasks.length > 0) {
        const allPrompts = subtasks.map((s) =>
          `${s.isInitialTask ? '🎯 Initial Task' : `💬 Feedback #${s.subtaskIndex}`} (${formatDate(s.timestamp)}):\n${s.prompt}`
        ).join('\n\n---\n\n');
        atts.push({ id: `prompts-${Date.now()}`, label: `All Prompts (${subtasks.length})`, type: 'prompts', content: allPrompts, meta: { count: subtasks.length } });
      } else {
        const promptText = task?.taskPrompt || detail?.taskPrompt || '';
        if (promptText) {
          atts.push({ id: `prompts-${Date.now()}`, label: 'Task Prompt', type: 'prompts', content: promptText, meta: { count: 1 } });
        }
      }

      // 2. Task summary
      const d = detail;
      const t = task;
      const summaryLines: string[] = [];
      if (t) {
        summaryLines.push(
          `Task ID: ${t.taskId}`,
          `Started: ${formatDate(t.startedAt)}`,
          `Ended: ${formatDate(t.endedAt)}`,
          `Messages: ${t.messageCount}`,
          `Tool Calls: ${t.toolUseCount}`,
          `Thinking Blocks: ${t.thinkingCount}`,
          `Files Edited: ${t.filesEdited}`,
          `Files Read: ${t.filesRead}`,
          `API History Size: ${formatBytes(t.apiHistorySizeBytes)}`,
        );
        if (Object.keys(t.toolBreakdown).length > 0) {
          summaryLines.push('', '--- Tool Breakdown ---');
          for (const [name, count] of Object.entries(t.toolBreakdown).sort((a, b) => b[1] - a[1])) {
            summaryLines.push(`  ${name}: ${count}`);
          }
        }
      }
      if (d) {
        if (d.modelUsage.length > 0) {
          summaryLines.push('', '--- Model Usage ---');
          for (const mu of d.modelUsage) {
            summaryLines.push(`  ${mu.modelId ?? '?'} via ${mu.modelProviderId ?? '?'} (${mu.mode ?? '?'})`);
          }
        }
        if (d.environment.length > 0) {
          const env = d.environment[0];
          summaryLines.push('', '--- Environment ---');
          summaryLines.push(`  OS: ${env.osName ?? '?'} ${env.osVersion ?? ''}`);
          summaryLines.push(`  Host: ${env.hostName ?? '?'} ${env.hostVersion ?? ''}`);
          summaryLines.push(`  Cline: ${env.clineVersion ?? '?'}`);
        }
      }
      if (summaryLines.length > 0) {
        atts.push({ id: `summary-${Date.now()}`, label: 'Task Summary', type: 'text', content: summaryLines.join('\n'), meta: {} });
      }

      // 3. Files in context
      if (d && d.files.length > 0) {
        const fileLines = d.files.map(f => {
          const src = f.recordSource ?? 'unknown';
          const state = f.recordState ?? '';
          return `${f.path}  [${src}${state ? ', ' + state : ''}]`;
        });
        atts.push({ id: `files-${Date.now()}`, label: `Files in Context (${d.files.length})`, type: 'files', content: `Files in Context (${d.files.length}):\n` + fileLines.join('\n'), meta: { count: d.files.length } });
      }

      // 4. Tool call timeline
      if (d && d.toolCalls.length > 0) {
        const toolLines = d.toolCalls.map(tc =>
          `#${tc.callIndex} [msg#${tc.messageIndex}] ${tc.toolName}: ${tc.inputSummary.slice(0, 120)}${tc.inputSummary.length > 120 ? '…' : ''}`
        );
        atts.push({ id: `tools-${Date.now()}`, label: `Tool Calls (${d.toolCalls.length})`, type: 'text', content: `Tool Call Timeline (${d.toolCalls.length} calls):\n` + toolLines.join('\n'), meta: { count: d.toolCalls.length } });
      }

      // 5. Focus chain
      if (d?.focusChain) {
        atts.push({ id: `focus-${Date.now()}`, label: 'Focus Chain', type: 'text', content: d.focusChain, meta: {} });
      }

      console.log('[askLlm] History latest task attachments:', atts.length, atts.map(a => a.label));
      navigationStore.navigateToChat({ attachments: atts, timestamp: Date.now() });
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
          Most recent conversation history
        </span>
      </h2>
    </div>
    <div class="flex items-center gap-2">
      {#if task}
        <button
          onclick={askLlm}
          disabled={askLlmLoading || loading}
          class="px-4 py-1.5 text-sm font-medium rounded-lg transition-colors {askLlmLoading ? 'bg-gray-200 text-gray-400 cursor-wait' : 'bg-indigo-500 text-white hover:bg-indigo-600'} disabled:opacity-50 disabled:cursor-not-allowed"
          title="Send latest task details to Agent Chat"
        >
          {askLlmLoading ? '⏳ Loading…' : '🤖 Ask LLM'}
        </button>
      {/if}
      {#if task && onViewDetail}
        <button
          onclick={() => onViewDetail?.(task!.taskId)}
          class="px-4 py-1.5 text-sm font-medium text-white bg-purple-600 rounded-lg hover:bg-purple-700 transition-colors"
        >
          📋 Full Detail
        </button>
      {/if}
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
        <p class="text-gray-500 text-sm">Loading latest task...</p>
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
          <p class="text-sm font-medium text-red-800">Failed to load latest task</p>
          <p class="text-sm text-red-600 mt-1">{error}</p>
        </div>
      </div>
    </div>

  <!-- Data loaded -->
  {:else if task}
    <div class="space-y-4">

      <!-- Identity Card -->
      <div class="bg-white border border-gray-200 rounded-lg p-4 shadow-sm">
        <div class="flex items-start justify-between">
          <div>
            <div class="flex items-center gap-3 mb-1">
              <span class="text-sm font-bold text-gray-900">📋 Task:</span>
              <code class="text-sm font-mono bg-gray-100 px-2 py-0.5 rounded">{task.taskId}</code>
              <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
              {#if task.startedAt}
                <span class="text-xs text-gray-400">{timeSince(task.startedAt)}</span>
              {/if}
            </div>
            <div class="flex items-center gap-4 text-xs text-gray-500 mt-1">
              <span>🕐 {formatDate(task.startedAt)}</span>
              {#if task.endedAt}
                <span>→ {formatDate(task.endedAt)}</span>
              {:else}
                <span class="text-amber-600 font-medium">⏳ In Progress</span>
              {/if}
            </div>
          </div>
          <div class="flex items-center gap-3 text-xs text-gray-500">
            {#if task.modelId}
              <span class="inline-flex items-center px-2 py-0.5 rounded-full bg-indigo-50 text-indigo-700 border border-indigo-200 font-medium">
                🤖 {task.modelId}
              </span>
            {/if}
          </div>
        </div>
      </div>

      <!-- Stats Row -->
      <div class="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-6 gap-3">
        <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm text-center">
          <div class="text-2xl font-bold text-gray-900">{task.messageCount}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-0.5">Messages</div>
        </div>
        <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm text-center">
          <div class="text-2xl font-bold text-gray-900">{task.toolUseCount}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-0.5">Tool Calls</div>
        </div>
        <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm text-center">
          <div class="text-2xl font-bold text-gray-900">{task.thinkingCount}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-0.5">Thinking</div>
        </div>
        <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm text-center">
          <div class="text-2xl font-bold text-gray-900">{task.filesEdited}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-0.5">Files Edited</div>
        </div>
        <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm text-center">
          <div class="text-2xl font-bold text-gray-900">{task.filesRead}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-0.5">Files Read</div>
        </div>
        <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm text-center">
          <div class="text-2xl font-bold text-gray-900">{formatBytes(task.apiHistorySizeBytes)}</div>
          <div class="text-[10px] text-gray-500 uppercase tracking-wide mt-0.5">API Size</div>
        </div>
      </div>

      <!-- Task Prompt -->
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
        <div class="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center justify-between">
          <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">
            {subtasks.length > 1 ? `All Prompts (${subtasks.length})` : 'Task Prompt'}
          </span>
          <button
            onclick={copyPrompt}
            class="text-[10px] font-medium px-2 py-0.5 rounded bg-gray-200 hover:bg-gray-300 text-gray-600 transition-colors"
          >
            📋 Copy
          </button>
        </div>
        {#if subtasks.length > 1}
          <!-- Multiple subtask prompts -->
          <div class="divide-y divide-gray-100">
            {#each subtasks as subtask}
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
          <!-- Single prompt -->
          <div class="p-4">
            <p class="text-sm text-gray-800 whitespace-pre-wrap break-words leading-relaxed">
              {task.taskPrompt || detail?.taskPrompt || '(no prompt available)'}
            </p>
          </div>
        {/if}
      </div>

      <!-- Tools Used -->
      {#if Object.keys(task.toolBreakdown).length > 0}
        <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
          <div class="px-4 py-2 bg-gray-50 border-b border-gray-200">
            <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">
              Tools Used ({Object.keys(task.toolBreakdown).length})
            </span>
          </div>
          <div class="p-3 flex flex-wrap gap-1.5">
            {#each topTools(task.toolBreakdown) as [tool, count]}
              <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-gray-100 text-gray-600 border border-gray-200">
                🔧 {tool}
                <span class="text-gray-400">×{count}</span>
              </span>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Files in Context -->
      {#if detail && detail.files.length > 0}
        <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
          <div class="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center justify-between">
            <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">
              Files in Context ({detail.files.length})
            </span>
            <div class="flex items-center gap-3 text-[10px] text-gray-400">
              <span class="text-green-600">{detail.filesEditedCount} edited</span>
              <span class="text-blue-600">{detail.filesReadCount} read</span>
            </div>
          </div>
          <div class="p-3 max-h-60 overflow-y-auto">
            {#each detail.files.slice(0, 30) as file}
              <div class="flex items-center gap-2 py-1 text-xs font-mono">
                <span class="flex-shrink-0 w-4 text-center {file.clineEditDate ? 'text-green-600 font-bold' : file.clineReadDate ? 'text-blue-600' : 'text-gray-400'}">
                  {file.clineEditDate ? '✎' : file.clineReadDate ? '👁' : '·'}
                </span>
                <span class="text-gray-700 truncate flex-1">{file.path}</span>
              </div>
            {/each}
            {#if detail.files.length > 30}
              <div class="text-[10px] text-gray-400 mt-2 text-center">
                ...and {detail.files.length - 30} more files
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Model & Environment Info -->
      {#if detail}
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {#if detail.modelUsage.length > 0}
            <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm">
              <div class="text-xs font-bold text-gray-600 uppercase tracking-wide mb-2">Model</div>
              {#each detail.modelUsage.slice(0, 3) as mu}
                <div class="flex items-center gap-2 text-xs text-gray-700 py-0.5">
                  <span class="font-medium">{mu.modelId || '—'}</span>
                  <span class="text-gray-400">{mu.modelProviderId || ''}</span>
                  {#if mu.mode}
                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 text-gray-500">{mu.mode}</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
          {#if detail.environment.length > 0}
            {@const env = detail.environment[0]}
            <div class="bg-white border border-gray-200 rounded-lg p-3 shadow-sm">
              <div class="text-xs font-bold text-gray-600 uppercase tracking-wide mb-2">Environment</div>
              <div class="space-y-0.5 text-xs text-gray-700">
                {#if env.hostName}<div>🖥️ {env.hostName} {env.hostVersion || ''}</div>{/if}
                {#if env.osName}<div>💻 {env.osName} {env.osVersion || ''}</div>{/if}
                {#if env.clineVersion}<div>🔧 Cline v{env.clineVersion}</div>{/if}
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Focus Chain -->
      {#if detail?.focusChain}
        <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
          <div class="px-4 py-2 bg-gray-50 border-b border-gray-200">
            <span class="text-xs font-bold text-gray-600 uppercase tracking-wide">🔗 Focus Chain</span>
          </div>
          <div class="p-4">
            <pre class="text-xs text-gray-700 whitespace-pre-wrap break-words leading-relaxed">{detail.focusChain}</pre>
          </div>
        </div>
      {/if}

      <!-- Context Footer -->
      <div class="text-[10px] text-gray-400 flex items-center gap-3 pt-2">
        <span>Task: {formatDate(task.startedAt)}{task.endedAt ? ` → ${formatDate(task.endedAt)}` : ' (in progress)'}</span>
        <span>API: {formatBytes(task.apiHistorySizeBytes)}</span>
        <span>UI: {formatBytes(task.uiMessagesSizeBytes)}</span>
        {#if task.clineVersion}
          <span>v{task.clineVersion}</span>
        {/if}
      </div>
    </div>
  {/if}
</div>

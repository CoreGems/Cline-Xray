<script lang="ts">
  import { onMount } from "svelte";
  import { fetchHistoryTasks } from "./api";
  import type { TaskHistorySummary, TaskHistoryListResponse } from "./types";

  // ---- Props ----
  let { onViewDetail }: { onViewDetail: (taskId: string) => void } = $props();

  // ---- State ----
  let loading = $state(true);
  let error: string | null = $state(null);
  let response: TaskHistoryListResponse | null = $state(null);
  let tasks: TaskHistorySummary[] = $state([]);
  let expandedTaskId: string | null = $state(null);
  let elapsed = $state(0);
  let promptCopyLabel = $state('📋 Copy');
  let searchQuery = $state('');

  // Derived filtered tasks — searches through all subtask prompts (not just initial)
  let filteredTasks = $derived.by(() => {
    if (!searchQuery.trim()) return tasks;
    const q = searchQuery.toLowerCase().trim();
    return tasks.filter(t =>
      t.taskId.toLowerCase().includes(q) ||
      (t.taskPrompt && t.taskPrompt.toLowerCase().includes(q)) ||
      (t.modelId && t.modelId.toLowerCase().includes(q)) ||
      (t.subtaskPrompts && t.subtaskPrompts.some(p => p.toLowerCase().includes(q)))
    );
  });

  onMount(() => {
    loadTasks(true);
  });

  async function loadTasks(refresh: boolean) {
    loading = true;
    error = null;
    const start = performance.now();
    try {
      response = await fetchHistoryTasks(refresh);
      tasks = response.tasks;
      elapsed = Math.round(performance.now() - start);
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
    }
  }

  function toggleTask(task: TaskHistorySummary) {
    expandedTaskId = expandedTaskId === task.taskId ? null : task.taskId;
  }

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    try {
      const d = new Date(iso);
      return d.toLocaleString(undefined, {
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit'
      });
    } catch {
      return iso;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatDuration(startIso: string, endIso: string | null): string {
    if (!endIso) return '—';
    try {
      const start = new Date(startIso).getTime();
      const end = new Date(endIso).getTime();
      const diffMs = end - start;
      if (diffMs < 0) return '—';
      const mins = Math.floor(diffMs / 60000);
      const secs = Math.floor((diffMs % 60000) / 1000);
      if (mins < 1) return `${secs}s`;
      if (mins < 60) return `${mins}m ${secs}s`;
      const hrs = Math.floor(mins / 60);
      return `${hrs}h ${mins % 60}m`;
    } catch {
      return '—';
    }
  }

  function shortModel(modelId: string | null): string {
    if (!modelId) return '—';
    // Shorten common model names
    return modelId
      .replace('claude-sonnet-4-5-20250929', 'sonnet-4.5')
      .replace('claude-3-5-sonnet-20241022', 'sonnet-3.5')
      .replace('claude-3-opus', 'opus-3')
      .replace('claude-', '')
      .replace('-20250', '-25')
      .replace('-20241', '-24');
  }

  function topTools(breakdown: Record<string, number>, n: number = 3): Array<[string, number]> {
    return Object.entries(breakdown)
      .sort((a, b) => b[1] - a[1])
      .slice(0, n);
  }

  function toolColor(name: string): string {
    switch (name) {
      case 'write_to_file': return 'bg-green-100 text-green-700';
      case 'replace_in_file': return 'bg-yellow-100 text-yellow-700';
      case 'execute_command': return 'bg-blue-100 text-blue-700';
      case 'read_file': return 'bg-purple-100 text-purple-700';
      case 'attempt_completion': return 'bg-indigo-100 text-indigo-700';
      case 'search_files': return 'bg-cyan-100 text-cyan-700';
      case 'list_files': return 'bg-teal-100 text-teal-700';
      case 'ask_followup_question': return 'bg-orange-100 text-orange-700';
      case 'browser_action': return 'bg-pink-100 text-pink-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }
</script>

<div class="flex-1 p-6 overflow-auto">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-lg font-semibold text-gray-900">Conversation History</h2>
      <p class="text-sm text-gray-500 mt-1">
        {#if response}
          <code class="bg-gray-100 px-1.5 py-0.5 rounded text-xs font-mono">{response.tasksRoot}</code>
          <span class="ml-2 text-gray-400">({elapsed}ms)</span>
        {:else}
          Scanning Cline task directories...
        {/if}
      </p>
    </div>
    <div class="flex items-center gap-3">
      <!-- Compact Search Bar -->
      <div class="relative">
        <div class="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none">
          <svg class="h-3.5 w-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
          </svg>
        </div>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search prompts..."
          class="w-56 pl-8 pr-7 py-1.5 text-xs border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-white placeholder-gray-400"
        />
        {#if searchQuery}
          <button
            onclick={() => searchQuery = ''}
            class="absolute inset-y-0 right-0 pr-2 flex items-center text-gray-400 hover:text-gray-600"
          >
            <svg class="h-3.5 w-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        {/if}
      </div>
      {#if searchQuery && filteredTasks.length !== tasks.length}
        <span class="text-xs text-gray-500 whitespace-nowrap">{filteredTasks.length}/{tasks.length}</span>
      {/if}
      <button
        onclick={() => loadTasks(true)}
        disabled={loading}
        class="px-4 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {loading ? 'Scanning...' : 'Refresh'}
      </button>
    </div>
  </div>

  <!-- Loading State -->
  {#if loading}
    <div class="flex items-center justify-center py-20">
      <div class="text-center">
        <svg class="animate-spin h-8 w-8 text-blue-500 mx-auto mb-3" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <p class="text-gray-500 text-sm">Parsing conversation histories...</p>
        <p class="text-gray-400 text-xs mt-1">This may take a few seconds on first load (~84 MB of JSON)</p>
      </div>
    </div>

  <!-- Error State -->
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
      <div class="flex items-start gap-3">
        <svg class="w-5 h-5 text-red-500 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
        </svg>
        <div>
          <p class="text-sm font-medium text-red-800">Failed to load conversation histories</p>
          <p class="text-sm text-red-600 mt-1">{error}</p>
        </div>
      </div>
    </div>

  <!-- No Tasks -->
  {:else if tasks.length === 0}
    <div class="flex items-center justify-center py-20">
      <div class="text-center max-w-md">
        <div class="text-4xl mb-4">💬</div>
        <h3 class="text-lg font-semibold text-gray-900 mb-2">No Task Histories Found</h3>
        <p class="text-sm text-gray-500 mb-4">
          No Cline task conversation histories were found.
        </p>
        <div class="bg-gray-50 border border-gray-200 rounded-lg p-3 text-left">
          <p class="text-xs text-gray-600 mb-1 font-medium">Expected location:</p>
          <code class="text-xs font-mono text-gray-700 break-all">
            %APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\tasks\
          </code>
          <p class="text-xs text-gray-500 mt-2">
            Make sure the Cline extension is installed and you have run at least one task.
          </p>
        </div>
      </div>
    </div>

  <!-- Data Loaded -->
  {:else if response}
    <!-- Aggregate Stats Bar -->
    <div class="grid grid-cols-5 gap-3 mb-6">
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-gray-900">{response.totalTasks}</div>
        <div class="text-xs text-gray-500 mt-0.5">Tasks</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-blue-600">{response.totalMessages.toLocaleString()}</div>
        <div class="text-xs text-gray-500 mt-0.5">Messages</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-green-600">{response.totalToolCalls.toLocaleString()}</div>
        <div class="text-xs text-gray-500 mt-0.5">Tool Calls</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-purple-600">{formatBytes(response.totalApiHistoryBytes)}</div>
        <div class="text-xs text-gray-500 mt-0.5">API History</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-lg font-bold text-gray-700 leading-tight">
          {#each topTools(response.aggregateToolBreakdown, 3) as [name, count]}
            <div class="text-xs">
              <span class="font-mono">{name}</span>
              <span class="text-gray-400 ml-1">{count}</span>
            </div>
          {/each}
        </div>
        <div class="text-xs text-gray-500 mt-0.5">Top Tools</div>
      </div>
    </div>

    <!-- Task Table -->
    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 border-b border-gray-200">
          <tr>
            <th class="w-8 px-3 py-3"></th>
            <th class="text-left px-3 py-3 font-medium text-gray-600">#</th>
            <th class="text-left px-3 py-3 font-medium text-gray-600">Task</th>
            <th class="text-right px-3 py-3 font-medium text-gray-600">Msgs</th>
            <th class="text-right px-3 py-3 font-medium text-gray-600">Tools</th>
            <th class="text-left px-3 py-3 font-medium text-gray-600">Model</th>
            <th class="text-left px-3 py-3 font-medium text-gray-600">Duration</th>
            <th class="text-right px-3 py-3 font-medium text-gray-600">Size</th>
            <th class="text-left px-3 py-3 font-medium text-gray-600">Started</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredTasks as task, i}
            <tr
              class="border-b border-gray-100 hover:bg-blue-50 transition-colors cursor-pointer {i === 0 && expandedTaskId !== task.taskId ? 'bg-blue-50/60 ring-1 ring-inset ring-blue-200' : ''} {expandedTaskId === task.taskId ? 'bg-indigo-50' : ''}"
              onclick={() => toggleTask(task)}
            >
              <td class="px-3 py-3 text-gray-400 text-xs text-center">
                <span class="inline-block transition-transform {expandedTaskId === task.taskId ? 'rotate-90' : ''}">▸</span>
              </td>
              <td class="px-3 py-3 text-gray-400 font-mono text-xs">{i + 1}</td>
              <td class="px-3 py-3 max-w-xs">
                <div class="font-mono font-medium text-gray-900 text-xs">
                  {task.taskId}
                  {#if i === 0}
                    <span class="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
                  {/if}
                </div>
                {#if task.taskPrompt}
                  <div class="text-xs text-gray-500 truncate mt-0.5 max-w-[300px]" title={task.taskPrompt}>
                    {task.taskPrompt}
                  </div>
                {/if}
              </td>
              <td class="px-3 py-3 text-right font-mono text-gray-700">{task.messageCount}</td>
              <td class="px-3 py-3 text-right font-mono text-gray-700">{task.toolUseCount}</td>
              <td class="px-3 py-3 text-gray-600 text-xs font-mono">{shortModel(task.modelId)}</td>
              <td class="px-3 py-3 text-gray-600 text-xs">{formatDuration(task.startedAt, task.endedAt)}</td>
              <td class="px-3 py-3 text-right text-gray-500 text-xs">{formatBytes(task.apiHistorySizeBytes)}</td>
              <td class="px-3 py-3 text-gray-600 text-xs">{formatDate(task.startedAt)}</td>
            </tr>

            <!-- Expanded Detail Panel -->
            {#if expandedTaskId === task.taskId}
              <tr>
                <td colspan="9" class="p-0">
                  <div class="bg-gray-50 border-t border-b border-gray-200 px-6 py-4">
                    <div class="grid grid-cols-3 gap-6">
                      <!-- Left: Task Info -->
                      <div>
                        <div class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">Task Info</div>
                        <dl class="space-y-1.5 text-xs">
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Started</dt>
                            <dd class="text-gray-900 font-mono">{formatDate(task.startedAt)}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Ended</dt>
                            <dd class="text-gray-900 font-mono">{formatDate(task.endedAt)}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Duration</dt>
                            <dd class="text-gray-900">{formatDuration(task.startedAt, task.endedAt)}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Model</dt>
                            <dd class="text-gray-900 font-mono">{task.modelId ?? '—'}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Provider</dt>
                            <dd class="text-gray-900">{task.modelProvider ?? '—'}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Cline Version</dt>
                            <dd class="text-gray-900">{task.clineVersion ?? '—'}</dd>
                          </div>
                        </dl>
                      </div>

                      <!-- Middle: Counts -->
                      <div>
                        <div class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">Counts</div>
                        <dl class="space-y-1.5 text-xs">
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Messages</dt>
                            <dd class="text-gray-900 font-bold">{task.messageCount}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Tool Calls</dt>
                            <dd class="text-gray-900 font-bold">{task.toolUseCount}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Thinking Blocks</dt>
                            <dd class="text-gray-900">{task.thinkingCount}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Files in Context</dt>
                            <dd class="text-gray-900">{task.filesInContext}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Files Edited</dt>
                            <dd class="text-green-700 font-medium">{task.filesEdited}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Files Read</dt>
                            <dd class="text-purple-700">{task.filesRead}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">API History</dt>
                            <dd class="text-gray-900">{formatBytes(task.apiHistorySizeBytes)}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">UI Messages</dt>
                            <dd class="text-gray-900">{formatBytes(task.uiMessagesSizeBytes)}</dd>
                          </div>
                          <div class="flex justify-between">
                            <dt class="text-gray-500">Focus Chain</dt>
                            <dd>{task.hasFocusChain ? '✅' : '—'}</dd>
                          </div>
                        </dl>
                      </div>

                      <!-- Right: Tool Breakdown -->
                      <div>
                        <div class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">Tool Breakdown</div>
                        {#if Object.keys(task.toolBreakdown).length === 0}
                          <p class="text-xs text-gray-400 italic">No tool calls</p>
                        {:else}
                          <div class="space-y-1">
                            {#each Object.entries(task.toolBreakdown).sort((a, b) => b[1] - a[1]) as [name, count]}
                              <div class="flex items-center gap-2">
                                <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {toolColor(name)}">
                                  {name}
                                </span>
                                <div class="flex-1 bg-gray-200 rounded-full h-1.5">
                                  <div
                                    class="bg-blue-500 h-1.5 rounded-full"
                                    style="width: {Math.round((count / task.toolUseCount) * 100)}%"
                                  ></div>
                                </div>
                                <span class="text-xs font-mono text-gray-600 w-6 text-right">{count}</span>
                              </div>
                            {/each}
                          </div>
                        {/if}
                      </div>
                    </div>

                    <!-- Task Prompt (full) -->
                    {#if task.taskPrompt}
                      <div class="mt-4 pt-3 border-t border-gray-200">
                        <div class="flex items-center justify-between mb-1">
                          <div class="text-xs font-medium text-gray-500 uppercase tracking-wide">Task Prompt</div>
                          <button
                            onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(task.taskPrompt ?? ''); promptCopyLabel = '✓ Copied!'; setTimeout(() => promptCopyLabel = '📋 Copy', 1500); }}
                            class="text-xs font-medium px-2.5 py-1 rounded bg-blue-600 hover:bg-blue-700 text-white transition-colors shadow-sm"
                          >
                            {promptCopyLabel}
                          </button>
                        </div>
                        <div class="bg-white border border-gray-200 rounded p-3 text-xs text-gray-700 font-mono whitespace-pre-wrap">
                          {task.taskPrompt}
                        </div>
                      </div>
                    {/if}

                    <!-- View Full Detail Button -->
                    <div class="mt-4 pt-3 border-t border-gray-200 flex justify-end">
                      <button
                        onclick={(e) => { e.stopPropagation(); onViewDetail(task.taskId); }}
                        class="px-4 py-2 text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg transition-colors shadow-sm flex items-center gap-2"
                      >
                        View Full Detail →
                      </button>
                    </div>
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Summary -->
    <div class="mt-4 text-sm text-gray-500">
      Showing {filteredTasks.length} of {response.totalTasks} tasks ·
      {response.totalMessages.toLocaleString()} messages ·
      {response.totalToolCalls.toLocaleString()} tool calls ·
      {formatBytes(response.totalApiHistoryBytes)} total
    </div>
  {/if}
</div>

<script lang="ts">
  import { onMount } from "svelte";
  import { fetchHistoryStats } from "./api";
  import type { HistoryStatsResponse } from "./types";

  // ---- State ----
  let loading = $state(true);
  let error: string | null = $state(null);
  let stats: HistoryStatsResponse | null = $state(null);
  let elapsed = $state(0);

  onMount(() => { loadStats(false); });

  async function loadStats(refresh: boolean) {
    loading = true;
    error = null;
    const start = performance.now();
    try {
      stats = await fetchHistoryStats(refresh);
      elapsed = Math.round(performance.now() - start);
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    try {
      return new Date(iso).toLocaleString(undefined, {
        year: 'numeric', month: 'short', day: 'numeric',
        hour: '2-digit', minute: '2-digit'
      });
    } catch { return iso; }
  }

  function sortedEntries(record: Record<string, number>): [string, number][] {
    return Object.entries(record).sort((a, b) => b[1] - a[1]);
  }

  function toolColor(name: string): string {
    const colors: Record<string, string> = {
      'write_to_file': 'bg-green-100 text-green-700',
      'replace_in_file': 'bg-yellow-100 text-yellow-700',
      'execute_command': 'bg-blue-100 text-blue-700',
      'read_file': 'bg-purple-100 text-purple-700',
      'attempt_completion': 'bg-indigo-100 text-indigo-700',
      'search_files': 'bg-cyan-100 text-cyan-700',
      'list_files': 'bg-teal-100 text-teal-700',
      'ask_followup_question': 'bg-orange-100 text-orange-700',
      'browser_action': 'bg-pink-100 text-pink-700',
    };
    return colors[name] || 'bg-gray-100 text-gray-700';
  }
</script>

<div class="flex-1 p-6 overflow-auto">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-lg font-semibold text-gray-900">Aggregate Statistics</h2>
      <p class="text-sm text-gray-500 mt-1">
        {#if stats}
          <code class="bg-gray-100 px-1.5 py-0.5 rounded text-xs font-mono">{stats.tasksRoot}</code>
          <span class="ml-2 text-gray-400">({elapsed}ms)</span>
        {:else}
          Loading stats...
        {/if}
      </p>
    </div>
    <button
      onclick={() => loadStats(true)}
      disabled={loading}
      class="px-4 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
    >
      {loading ? 'Loading...' : 'Refresh'}
    </button>
  </div>

  <!-- Loading -->
  {#if loading}
    <div class="flex items-center justify-center py-20">
      <div class="text-center">
        <svg class="animate-spin h-8 w-8 text-blue-500 mx-auto mb-3" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <p class="text-gray-500 text-sm">Computing aggregate statistics...</p>
      </div>
    </div>

  <!-- Error -->
  {:else if error}
    <div class="bg-red-50 border border-red-200 rounded-lg p-4">
      <p class="text-sm font-medium text-red-800">Failed to load stats</p>
      <p class="text-sm text-red-600 mt-1">{error}</p>
    </div>

  <!-- Stats Dashboard -->
  {:else if stats}
    <!-- Row 1: Key Metrics -->
    <div class="grid grid-cols-5 gap-3 mb-6">
      <div class="bg-white border border-gray-200 rounded-lg p-4 text-center">
        <div class="text-3xl font-bold text-gray-900">{stats.totalTasks}</div>
        <div class="text-xs text-gray-500 mt-1">Total Tasks</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-4 text-center">
        <div class="text-3xl font-bold text-blue-600">{stats.totalMessages.toLocaleString()}</div>
        <div class="text-xs text-gray-500 mt-1">Total Messages</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-4 text-center">
        <div class="text-3xl font-bold text-green-600">{stats.totalToolCalls.toLocaleString()}</div>
        <div class="text-xs text-gray-500 mt-1">Total Tool Calls</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-4 text-center">
        <div class="text-3xl font-bold text-purple-600">{stats.totalThinkingBlocks.toLocaleString()}</div>
        <div class="text-xs text-gray-500 mt-1">Thinking Blocks</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-4 text-center">
        <div class="text-3xl font-bold text-orange-600">{formatBytes(stats.totalApiHistoryBytes)}</div>
        <div class="text-xs text-gray-500 mt-1">API History Size</div>
      </div>
    </div>

    <!-- Row 2: Averages -->
    <div class="grid grid-cols-5 gap-3 mb-6">
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-xl font-bold text-gray-700">{stats.avgMessagesPerTask.toFixed(1)}</div>
        <div class="text-xs text-gray-500 mt-0.5">Avg Messages/Task</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-xl font-bold text-gray-700">{stats.avgToolCallsPerTask.toFixed(1)}</div>
        <div class="text-xs text-gray-500 mt-0.5">Avg Tools/Task</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-xl font-bold text-gray-700">{stats.avgThinkingBlocksPerTask.toFixed(1)}</div>
        <div class="text-xs text-gray-500 mt-0.5">Avg Thinking/Task</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-xl font-bold text-gray-700">{stats.avgFilesInContext.toFixed(1)}</div>
        <div class="text-xs text-gray-500 mt-0.5">Avg Files/Task</div>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg p-3 text-center">
        <div class="text-xl font-bold text-gray-700">{formatBytes(stats.avgTaskSizeBytes)}</div>
        <div class="text-xs text-gray-500 mt-0.5">Avg Task Size</div>
      </div>
    </div>

    <!-- Row 3: Two-column layout -->
    <div class="grid grid-cols-2 gap-6 mb-6">
      <!-- Tool Breakdown -->
      <div class="bg-white border border-gray-200 rounded-lg p-4">
        <h3 class="text-sm font-semibold text-gray-700 mb-3">🔧 Tool Usage Breakdown</h3>
        {#if Object.keys(stats.toolBreakdown).length === 0}
          <p class="text-xs text-gray-400 italic">No tool calls</p>
        {:else}
          <div class="space-y-2">
            {#each sortedEntries(stats.toolBreakdown) as [name, count]}
              <div class="flex items-center gap-2">
                <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium whitespace-nowrap {toolColor(name)}">
                  {name}
                </span>
                <div class="flex-1 bg-gray-200 rounded-full h-2">
                  <div
                    class="bg-blue-500 h-2 rounded-full transition-all"
                    style="width: {Math.round((count / stats.totalToolCalls) * 100)}%"
                  ></div>
                </div>
                <span class="text-xs font-mono text-gray-600 w-12 text-right">{count.toLocaleString()}</span>
                <span class="text-xs text-gray-400 w-12 text-right">{stats.toolPercentages[name]?.toFixed(1) ?? '0'}%</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Model Usage -->
      <div class="bg-white border border-gray-200 rounded-lg p-4">
        <h3 class="text-sm font-semibold text-gray-700 mb-3">🤖 Model Usage</h3>
        {#if Object.keys(stats.modelUsage).length === 0}
          <p class="text-xs text-gray-400 italic">No model data</p>
        {:else}
          <div class="space-y-2">
            {#each sortedEntries(stats.modelUsage) as [model, count]}
              <div class="flex items-center gap-2">
                <span class="text-xs font-mono text-gray-700 flex-1 truncate" title={model}>{model}</span>
                <div class="w-24 bg-gray-200 rounded-full h-2">
                  <div
                    class="bg-purple-500 h-2 rounded-full transition-all"
                    style="width: {Math.round((count / stats.totalTasks) * 100)}%"
                  ></div>
                </div>
                <span class="text-xs font-mono text-gray-600 w-8 text-right">{count}</span>
                <span class="text-xs text-gray-400 w-10 text-right">{((count / stats.totalTasks) * 100).toFixed(0)}%</span>
              </div>
            {/each}
          </div>

          <!-- Provider sub-section -->
          {#if Object.keys(stats.modelProviderUsage).length > 0}
            <div class="mt-4 pt-3 border-t border-gray-100">
              <h4 class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">By Provider</h4>
              <div class="flex flex-wrap gap-2">
                {#each sortedEntries(stats.modelProviderUsage) as [provider, count]}
                  <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-indigo-50 text-indigo-700">
                    {provider}
                    <span class="bg-indigo-200 text-indigo-800 rounded-full px-1.5 py-0.5 text-[10px] font-bold">{count}</span>
                  </span>
                {/each}
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Row 4: Three-column detail cards -->
    <div class="grid grid-cols-3 gap-4 mb-6">
      <!-- File Stats -->
      <div class="bg-white border border-gray-200 rounded-lg p-4">
        <h3 class="text-sm font-semibold text-gray-700 mb-3">📁 File Statistics</h3>
        <dl class="space-y-2 text-xs">
          <div class="flex justify-between">
            <dt class="text-gray-500">Total Files in Context</dt>
            <dd class="font-mono font-medium text-gray-900">{stats.totalFilesInContext.toLocaleString()}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Files Edited (by Cline)</dt>
            <dd class="font-mono font-medium text-green-700">{stats.totalFilesEdited.toLocaleString()}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Files Read (by Cline)</dt>
            <dd class="font-mono font-medium text-purple-700">{stats.totalFilesRead.toLocaleString()}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Tasks with Focus Chain</dt>
            <dd class="font-mono text-gray-900">{stats.tasksWithFocusChain} / {stats.totalTasks}</dd>
          </div>
        </dl>
      </div>

      <!-- Size Stats -->
      <div class="bg-white border border-gray-200 rounded-lg p-4">
        <h3 class="text-sm font-semibold text-gray-700 mb-3">💾 Size Statistics</h3>
        <dl class="space-y-2 text-xs">
          <div class="flex justify-between">
            <dt class="text-gray-500">API History (total)</dt>
            <dd class="font-mono font-medium text-gray-900">{formatBytes(stats.totalApiHistoryBytes)}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">UI Messages (total)</dt>
            <dd class="font-mono font-medium text-gray-900">{formatBytes(stats.totalUiMessagesBytes)}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Smallest Task</dt>
            <dd class="font-mono text-gray-700">{formatBytes(stats.minTaskSizeBytes)}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Largest Task</dt>
            <dd class="font-mono text-gray-700">{formatBytes(stats.maxTaskSizeBytes)}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Average Task</dt>
            <dd class="font-mono text-gray-700">{formatBytes(stats.avgTaskSizeBytes)}</dd>
          </div>
        </dl>
      </div>

      <!-- Time Range & Cline Versions -->
      <div class="bg-white border border-gray-200 rounded-lg p-4">
        <h3 class="text-sm font-semibold text-gray-700 mb-3">📅 Time Range</h3>
        <dl class="space-y-2 text-xs">
          <div class="flex justify-between">
            <dt class="text-gray-500">Earliest Task</dt>
            <dd class="font-mono text-gray-900">{formatDate(stats.earliestTask)}</dd>
          </div>
          <div class="flex justify-between">
            <dt class="text-gray-500">Latest Task</dt>
            <dd class="font-mono text-gray-900">{formatDate(stats.latestTask)}</dd>
          </div>
        </dl>

        {#if Object.keys(stats.clineVersionUsage).length > 0}
          <div class="mt-3 pt-3 border-t border-gray-100">
            <h4 class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">Cline Versions</h4>
            <div class="space-y-1">
              {#each sortedEntries(stats.clineVersionUsage) as [version, count]}
                <div class="flex justify-between text-xs">
                  <span class="font-mono text-gray-700">v{version}</span>
                  <span class="text-gray-500">{count} tasks</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

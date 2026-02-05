<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import type { AccessLogEntry } from '../../types';

  // Subtab state
  type ActivitySubTab = 'REST';
  let activeSubTab: ActivitySubTab = 'REST';
  const subTabs: { id: ActivitySubTab; label: string }[] = [
    { id: 'REST', label: 'REST' }
  ];

  let logs: AccessLogEntry[] = [];
  let loading = false;
  let error: string | null = null;
  let autoRefresh = true;
  let refreshInterval: ReturnType<typeof setInterval> | null = null;

  // Format timestamp for display
  function formatTimestamp(timestamp: string): string {
    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString('en-US', { 
        hour12: false, 
        hour: '2-digit', 
        minute: '2-digit', 
        second: '2-digit' 
      });
    } catch {
      return timestamp;
    }
  }

  // Get status color class
  function getStatusColor(statusCode: number): string {
    if (statusCode >= 200 && statusCode < 300) return 'text-green-600';
    if (statusCode >= 400 && statusCode < 500) return 'text-yellow-600';
    if (statusCode >= 500) return 'text-red-600';
    return 'text-gray-600';
  }

  // Get method color class
  function getMethodColor(method: string): string {
    switch (method.toUpperCase()) {
      case 'GET': return 'bg-blue-100 text-blue-800';
      case 'POST': return 'bg-green-100 text-green-800';
      case 'PUT': return 'bg-yellow-100 text-yellow-800';
      case 'DELETE': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  }

  async function fetchLogs() {
    try {
      loading = true;
      error = null;
      logs = await invoke<AccessLogEntry[]>('get_access_logs');
      // Reverse to show newest first
      logs = logs.reverse();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function clearLogs() {
    try {
      await invoke('clear_access_logs');
      logs = [];
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    if (autoRefresh) {
      startAutoRefresh();
    } else {
      stopAutoRefresh();
    }
  }

  function startAutoRefresh() {
    if (refreshInterval) return;
    refreshInterval = setInterval(fetchLogs, 2000); // Refresh every 2 seconds
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  onMount(() => {
    fetchLogs();
    if (autoRefresh) {
      startAutoRefresh();
    }
  });

  onDestroy(() => {
    stopAutoRefresh();
  });
</script>

<div class="flex-1 flex flex-col h-full bg-gray-50">
  <!-- Subtab Navigation -->
  <div class="bg-white border-b border-gray-200 px-4">
    <div class="flex gap-1">
      {#each subTabs as tab}
        <button
          onclick={() => activeSubTab = tab.id}
          class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {activeSubTab === tab.id
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- REST Subtab Content -->
  {#if activeSubTab === 'REST'}
    <div class="flex-1 flex flex-col">
      <!-- Header -->
      <div class="bg-white border-b border-gray-200 px-4 py-3 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <svg class="w-5 h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
          </svg>
          <h2 class="text-lg font-semibold text-gray-800">REST API Access Log</h2>
          <span class="text-sm text-gray-500">({logs.length} entries)</span>
        </div>
        
        <div class="flex items-center gap-2">
          <!-- Auto-refresh toggle -->
          <button
            onclick={toggleAutoRefresh}
            class="px-3 py-1.5 text-sm rounded-md flex items-center gap-1.5 {autoRefresh ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-600'}"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
            </svg>
            {autoRefresh ? 'Auto' : 'Manual'}
          </button>
          
          <!-- Refresh button -->
          <button
            onclick={fetchLogs}
            disabled={loading}
            class="px-3 py-1.5 text-sm bg-blue-500 text-white rounded-md hover:bg-blue-600 disabled:opacity-50 flex items-center gap-1.5"
          >
            <svg class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
            </svg>
            Refresh
          </button>
          
          <!-- Clear button -->
          <button
            onclick={clearLogs}
            class="px-3 py-1.5 text-sm bg-gray-200 text-gray-700 rounded-md hover:bg-gray-300 flex items-center gap-1.5"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
            </svg>
            Clear
          </button>
        </div>
      </div>

      <!-- Error display -->
      {#if error}
        <div class="mx-4 mt-3 p-3 bg-red-50 border border-red-200 rounded-md text-red-700 text-sm">
          {error}
        </div>
      {/if}

      <!-- Log table -->
      <div class="flex-1 overflow-auto p-4">
        {#if logs.length === 0 && !loading}
          <div class="flex flex-col items-center justify-center h-full text-gray-400">
            <svg class="w-16 h-16 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
            </svg>
            <p class="text-lg font-medium">No access logs yet</p>
            <p class="text-sm mt-1">Run a CLI test script to see HTTP requests here</p>
            <p class="text-xs mt-3 font-mono bg-gray-100 px-3 py-1 rounded">.\scripts\test_health.ps1</p>
          </div>
        {:else}
          <table class="w-full bg-white rounded-lg shadow-sm overflow-hidden">
            <thead class="bg-gray-50">
              <tr>
                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Time</th>
                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Method</th>
                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Path</th>
                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Duration</th>
                <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Client</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200">
              {#each logs as log (log.id)}
                <tr class="hover:bg-gray-50">
                  <td class="px-4 py-2 text-sm text-gray-600 font-mono whitespace-nowrap">
                    {formatTimestamp(log.timestamp)}
                  </td>
                  <td class="px-4 py-2">
                    <span class="px-2 py-0.5 text-xs font-semibold rounded {getMethodColor(log.method)}">
                      {log.method}
                    </span>
                  </td>
                  <td class="px-4 py-2 text-sm text-gray-800 font-mono">
                    {log.path}
                  </td>
                  <td class="px-4 py-2 text-sm font-semibold {getStatusColor(log.statusCode)}">
                    {log.statusCode}
                  </td>
                  <td class="px-4 py-2 text-sm text-gray-600">
                    {log.durationMs}ms
                  </td>
                  <td class="px-4 py-2 text-sm text-gray-500 font-mono">
                    {log.clientIp}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    </div>
  {/if}
</div>

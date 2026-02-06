<script lang="ts">
  // Inference Subtab - Displays AI inference activity logs (Gemini API calls)
  import { onMount, onDestroy } from 'svelte';
  import type { InferenceLogEntry } from './types';
  import { fetchInferenceLogs, clearInferenceLogs } from './api';
  import { formatTimestamp } from './utils';

  let logs: InferenceLogEntry[] = $state([]);
  let loading = $state(false);
  let error: string | null = $state(null);
  let autoRefresh = $state(true);
  let refreshInterval: ReturnType<typeof setInterval> | null = null;
  
  // Modal state
  let selectedLog: InferenceLogEntry | null = $state(null);
  let showModal = $state(false);
  let copySuccess = $state(false);

  async function loadLogs() {
    try {
      loading = true;
      error = null;
      logs = await fetchInferenceLogs();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function handleClearLogs() {
    try {
      await clearInferenceLogs();
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
    refreshInterval = setInterval(loadLogs, 2000); // Refresh every 2 seconds
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  function getProviderColor(provider: string): string {
    switch (provider.toLowerCase()) {
      case 'gemini':
        return 'bg-blue-100 text-blue-800';
      case 'openai':
        return 'bg-green-100 text-green-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  }

  function getSuccessColor(success: boolean): string {
    return success ? 'text-green-600' : 'text-red-600';
  }

  function handleRowDoubleClick(log: InferenceLogEntry) {
    selectedLog = log;
    showModal = true;
  }

  function closeModal() {
    showModal = false;
    selectedLog = null;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && showModal) {
      closeModal();
    }
  }

  async function copyMetadataToClipboard() {
    if (!selectedLog?.metadata) return;
    
    try {
      const text = JSON.stringify(selectedLog.metadata, null, 2);
      await navigator.clipboard.writeText(text);
      copySuccess = true;
      setTimeout(() => {
        copySuccess = false;
      }, 2000);
    } catch (e) {
      console.error('Failed to copy to clipboard:', e);
    }
  }

  onMount(() => {
    loadLogs();
    if (autoRefresh) {
      startAutoRefresh();
    }
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    stopAutoRefresh();
    window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="flex-1 flex flex-col">
  <!-- Header -->
  <div class="bg-white border-b border-gray-200 px-4 py-3 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <svg class="w-5 h-5 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
      </svg>
      <h2 class="text-lg font-semibold text-gray-800">Inference Activity Log</h2>
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
        onclick={loadLogs}
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
        onclick={handleClearLogs}
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
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
        </svg>
        <p class="text-lg font-medium">No inference logs yet</p>
        <p class="text-sm mt-1">Use the Agent chat to generate AI inference activity</p>
      </div>
    {:else}
      <table class="w-full bg-white rounded-lg shadow-sm overflow-hidden">
        <thead class="bg-gray-50">
          <tr>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Time</th>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Provider</th>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Model</th>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Type</th>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Duration</th>
            <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Message Preview</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-200">
          {#each logs as log (log.id)}
            <tr 
              class="hover:bg-blue-50 cursor-pointer transition-colors"
              ondblclick={() => handleRowDoubleClick(log)}
              title="Double-click to view details"
            >
              <td class="px-4 py-2 text-sm text-gray-600 font-mono whitespace-nowrap">
                {formatTimestamp(log.timestamp)}
              </td>
              <td class="px-4 py-2">
                <span class="px-2 py-0.5 text-xs font-semibold rounded {getProviderColor(log.provider)}">
                  {log.provider}
                </span>
              </td>
              <td class="px-4 py-2 text-sm text-gray-800 font-mono">
                {log.model}
              </td>
              <td class="px-4 py-2 text-sm text-gray-600">
                {log.requestType}
              </td>
              <td class="px-4 py-2">
                <span class="flex items-center gap-1 text-sm font-semibold {getSuccessColor(log.success)}">
                  {#if log.success}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                    </svg>
                    OK
                  {:else}
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                    {log.errorMessage ? log.errorMessage.substring(0, 30) + '...' : 'Error'}
                  {/if}
                </span>
              </td>
              <td class="px-4 py-2 text-sm text-gray-600">
                {log.durationMs}ms
              </td>
              <td class="px-4 py-2 text-sm text-gray-500 max-w-xs truncate" title={log.userMessagePreview || ''}>
                {log.userMessagePreview || '-'}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="text-xs text-gray-400 mt-2 text-center">Double-click a row to view full details</p>
    {/if}
  </div>
</div>

<!-- Details Modal -->
{#if showModal && selectedLog}
  <div 
    class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
    onclick={(e) => e.target === e.currentTarget && closeModal()}
  >
    <div class="bg-white rounded-lg shadow-xl max-w-4xl w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col">
      <!-- Modal Header -->
      <div class="bg-gray-50 px-6 py-4 border-b border-gray-200 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <svg class="w-6 h-6 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
          </svg>
          <h3 class="text-lg font-semibold text-gray-800">Inference Details</h3>
          <span class="px-2 py-0.5 text-xs font-semibold rounded {getProviderColor(selectedLog.provider)}">
            {selectedLog.provider}
          </span>
        </div>
        <button
          onclick={closeModal}
          class="p-1 hover:bg-gray-200 rounded-full transition-colors"
        >
          <svg class="w-6 h-6 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>

      <!-- Modal Content -->
      <div class="flex-1 overflow-y-auto p-6">
        <!-- Basic Info Grid -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <div class="bg-gray-50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase tracking-wider">Model</p>
            <p class="font-mono text-sm font-semibold text-gray-800 mt-1">{selectedLog.model}</p>
          </div>
          <div class="bg-gray-50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase tracking-wider">Status</p>
            <p class="text-sm font-semibold mt-1 {getSuccessColor(selectedLog.success)}">
              {selectedLog.success ? 'Success' : 'Failed'}
            </p>
          </div>
          <div class="bg-gray-50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase tracking-wider">Duration</p>
            <p class="font-mono text-sm font-semibold text-gray-800 mt-1">{selectedLog.durationMs}ms</p>
          </div>
          <div class="bg-gray-50 rounded-lg p-3">
            <p class="text-xs text-gray-500 uppercase tracking-wider">Timestamp</p>
            <p class="font-mono text-xs text-gray-800 mt-1">{selectedLog.timestamp}</p>
          </div>
        </div>

        <!-- System Prompt Section -->
        {#if selectedLog.systemPrompt}
          <div class="mb-6">
            <h4 class="text-sm font-semibold text-gray-700 mb-2 flex items-center gap-2">
              <svg class="w-4 h-4 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
              </svg>
              System Instruction
            </h4>
            <div class="bg-purple-50 border border-purple-200 rounded-lg p-4 max-h-40 overflow-y-auto">
              <p class="text-sm text-gray-800 whitespace-pre-wrap">{selectedLog.systemPrompt}</p>
            </div>
          </div>
        {/if}

        <!-- Error Message Section -->
        {#if selectedLog.errorMessage}
          <div class="mb-6">
            <h4 class="text-sm font-semibold text-gray-700 mb-2 flex items-center gap-2">
              <svg class="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              Error Message
            </h4>
            <div class="bg-red-50 border border-red-200 rounded-lg p-4">
              <p class="text-sm text-red-800 whitespace-pre-wrap">{selectedLog.errorMessage}</p>
            </div>
          </div>
        {/if}

        <!-- Conversation History -->
        {#if selectedLog.metadata?.history && Array.isArray(selectedLog.metadata.history) && selectedLog.metadata.history.length > 0}
          {@const history = selectedLog.metadata.history as Array<{role: string, content: string}>}
          <div class="mb-6">
            <h4 class="text-sm font-semibold text-gray-700 mb-2 flex items-center gap-2">
              <svg class="w-4 h-4 text-orange-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
              </svg>
              Conversation History ({history.length} messages)
            </h4>
            <div class="bg-orange-50 border border-orange-200 rounded-lg p-4 max-h-48 overflow-y-auto">
              {#each history as msg, i}
                <div class="mb-2 pb-2 {i < history.length - 1 ? 'border-b border-orange-200' : ''}">
                  <span class="text-xs font-semibold uppercase {msg.role === 'user' ? 'text-blue-600' : 'text-green-600'}">
                    {msg.role}:
                  </span>
                  <p class="text-sm text-gray-700 mt-1">{msg.content}</p>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Raw Metadata -->
        <details class="text-sm" open>
          <summary class="cursor-pointer text-gray-500 hover:text-gray-700">Raw Metadata</summary>
          <div class="mt-2 relative">
            <button
              onclick={copyMetadataToClipboard}
              class="absolute top-2 right-2 px-2 py-1 text-xs rounded flex items-center gap-1 transition-colors {copySuccess ? 'bg-green-500 text-white' : 'bg-gray-200 text-gray-600 hover:bg-gray-300'}"
              title="Copy to clipboard"
            >
              {#if copySuccess}
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                </svg>
                Copied!
              {:else}
                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
                </svg>
                Copy
              {/if}
            </button>
            <pre class="p-4 pr-20 bg-gray-100 rounded-lg overflow-x-auto text-xs font-mono">{JSON.stringify(selectedLog.metadata, null, 2)}</pre>
          </div>
        </details>
      </div>

      <!-- Modal Footer -->
      <div class="bg-gray-50 px-6 py-4 border-t border-gray-200 flex justify-end">
        <button
          onclick={closeModal}
          class="px-4 py-2 bg-gray-200 text-gray-700 rounded-md hover:bg-gray-300 transition-colors"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}

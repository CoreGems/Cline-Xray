<script lang="ts">
  import { fetchTaskDetail, fetchTaskMessages, fetchSingleMessage, fetchTaskTools, fetchTaskThinking } from "./api";
  import type { TaskDetailResponse, ConversationMessage, PaginatedMessagesResponse, FullMessageResponse, ToolCallTimelineResponse, ToolCallTimelineEntry, ThinkingBlocksResponse } from "./types";

  // ---- Props ----
  let { taskId, onBack }: { taskId: string; onBack: () => void } = $props();

  // ---- State ----
  let loading = $state(true);
  let error: string | null = $state(null);
  let detail: TaskDetailResponse | null = $state(null);
  let elapsed = $state(0);
  let activeSection: 'messages' | 'tools' | 'thinking' | 'files' | 'env' | 'focus' = $state('messages');

  // ---- Full message expand state ----
  let expandedMsg: FullMessageResponse | null = $state(null);
  let expandLoading = $state(false);
  let expandError: string | null = $state(null);

  async function expandMessage(index: number) {
    expandLoading = true;
    expandError = null;
    try {
      expandedMsg = await fetchSingleMessage(taskId, index);
    } catch (e: any) {
      expandError = e.message || String(e);
    } finally {
      expandLoading = false;
    }
  }

  function closeExpand() {
    expandedMsg = null;
    expandError = null;
  }

  // ---- Paginated messages state ----
  let msgOffset = $state(0);
  let msgLimit = $state(20);
  let msgRole: string | undefined = $state(undefined);
  let msgLoading = $state(false);
  let msgError: string | null = $state(null);
  let paginatedMessages: ConversationMessage[] = $state([]);
  let msgTotalMessages = $state(0);
  let msgFilteredCount = $state(0);
  let msgHasMore = $state(false);
  let msgElapsed = $state(0);

  // Load on mount
  $effect(() => {
    loadDetail(taskId);
  });

  // ---- Tool timeline state ----
  let toolsData: ToolCallTimelineResponse | null = $state(null);
  let toolsLoading = $state(false);
  let toolsError: string | null = $state(null);
  let toolNameFilter: string = $state('');
  let failedOnlyFilter: boolean = $state(false);
  let toolsElapsed = $state(0);

  // ---- Thinking blocks state ----
  let thinkingData: ThinkingBlocksResponse | null = $state(null);
  let thinkingLoading = $state(false);
  let thinkingError: string | null = $state(null);
  let thinkingMaxLength: number = $state(1000);
  let thinkingMinLength: number = $state(0);
  let thinkingElapsed = $state(0);

  // Load paginated messages when tab is active or params change
  $effect(() => {
    if (activeSection === 'messages' && detail) {
      loadMessages(taskId, msgOffset, msgLimit, msgRole);
    }
  });

  // Load tool timeline when tools tab is active
  $effect(() => {
    if (activeSection === 'tools' && detail) {
      loadTools(taskId, toolNameFilter || undefined, failedOnlyFilter);
    }
  });

  // Load thinking blocks when thinking tab is active
  $effect(() => {
    if (activeSection === 'thinking' && detail) {
      loadThinking(taskId, thinkingMaxLength, thinkingMinLength);
    }
  });

  async function loadThinking(id: string, maxLength?: number, minLength?: number) {
    thinkingLoading = true;
    thinkingError = null;
    const start = performance.now();
    try {
      thinkingData = await fetchTaskThinking(id, maxLength, minLength);
      thinkingElapsed = Math.round(performance.now() - start);
    } catch (e: any) {
      thinkingError = e.message || String(e);
    } finally {
      thinkingLoading = false;
    }
  }

  function applyThinkingFilters() {
    if (detail) {
      loadThinking(taskId, thinkingMaxLength, thinkingMinLength);
    }
  }

  function clearThinkingFilters() {
    thinkingMaxLength = 1000;
    thinkingMinLength = 0;
    if (detail) {
      loadThinking(taskId, 1000, 0);
    }
  }

  async function loadTools(id: string, toolName?: string, failedOnly?: boolean) {
    toolsLoading = true;
    toolsError = null;
    const start = performance.now();
    try {
      toolsData = await fetchTaskTools(id, toolName, failedOnly);
      toolsElapsed = Math.round(performance.now() - start);
    } catch (e: any) {
      toolsError = e.message || String(e);
    } finally {
      toolsLoading = false;
    }
  }

  function applyToolFilters() {
    if (detail) {
      loadTools(taskId, toolNameFilter || undefined, failedOnlyFilter);
    }
  }

  function clearToolFilters() {
    toolNameFilter = '';
    failedOnlyFilter = false;
    if (detail) {
      loadTools(taskId, undefined, false);
    }
  }

  async function loadDetail(id: string) {
    loading = true;
    error = null;
    const start = performance.now();
    try {
      detail = await fetchTaskDetail(id);
      elapsed = Math.round(performance.now() - start);
    } catch (e: any) {
      error = e.message || String(e);
    } finally {
      loading = false;
    }
  }

  async function loadMessages(id: string, offset: number, limit: number, role?: string) {
    msgLoading = true;
    msgError = null;
    const start = performance.now();
    try {
      const resp: PaginatedMessagesResponse = await fetchTaskMessages(id, offset, limit, role);
      paginatedMessages = resp.messages;
      msgTotalMessages = resp.totalMessages;
      msgFilteredCount = resp.filteredCount;
      msgHasMore = resp.hasMore;
      msgElapsed = Math.round(performance.now() - start);
    } catch (e: any) {
      msgError = e.message || String(e);
    } finally {
      msgLoading = false;
    }
  }

  function msgPrev() {
    msgOffset = Math.max(0, msgOffset - msgLimit);
  }
  function msgNext() {
    if (msgHasMore) msgOffset = msgOffset + msgLimit;
  }
  function setRoleFilter(role: string | undefined) {
    msgRole = role;
    msgOffset = 0;
  }

  // Computed for current page display
  $effect(() => {
    // Just for reactivity tracking — values are used in template
    void msgOffset; void msgFilteredCount; void msgLimit;
  });

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    try {
      return new Date(iso).toLocaleString(undefined, {
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit', second: '2-digit'
      });
    } catch { return iso; }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatDuration(startIso: string, endIso: string | null): string {
    if (!endIso) return '—';
    try {
      const diffMs = new Date(endIso).getTime() - new Date(startIso).getTime();
      if (diffMs < 0) return '—';
      const mins = Math.floor(diffMs / 60000);
      const secs = Math.floor((diffMs % 60000) / 1000);
      if (mins < 1) return `${secs}s`;
      if (mins < 60) return `${mins}m ${secs}s`;
      const hrs = Math.floor(mins / 60);
      return `${hrs}h ${mins % 60}m`;
    } catch { return '—'; }
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

  function roleColor(role: string): string {
    return role === 'assistant' ? 'border-blue-300 bg-blue-50' : 'border-gray-300 bg-white';
  }

  function roleLabel(role: string): string {
    return role === 'assistant' ? '🤖 Assistant' : '👤 User';
  }

  function blockIcon(type: string): string {
    switch (type) {
      case 'text': return '📝';
      case 'thinking': return '💭';
      case 'tool_use': return '🔧';
      case 'tool_result': return '📤';
      default: return '❓';
    }
  }

  function sourceColor(source: string | null): string {
    switch (source) {
      case 'cline_edited': return 'text-green-700 bg-green-50';
      case 'read_tool': return 'text-purple-700 bg-purple-50';
      case 'user_edited': return 'text-blue-700 bg-blue-50';
      case 'file_mentioned': return 'text-gray-600 bg-gray-50';
      default: return 'text-gray-500 bg-gray-50';
    }
  }

  const sections = [
    { id: 'messages' as const, label: 'Messages', icon: '💬' },
    { id: 'tools' as const, label: 'Tools', icon: '🔧' },
    { id: 'thinking' as const, label: 'Thinking', icon: '💭' },
    { id: 'files' as const, label: 'Files', icon: '📁' },
    { id: 'env' as const, label: 'Environment', icon: '⚙️' },
    { id: 'focus' as const, label: 'Focus Chain', icon: '✅' },
  ];
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden">
  <!-- Loading -->
  {#if loading}
    <div class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <svg class="animate-spin h-8 w-8 text-blue-500 mx-auto mb-3" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        <p class="text-gray-500 text-sm">Loading task {taskId}...</p>
      </div>
</div>

  <!-- Error -->
  {:else if error}
    <div class="p-6">
      <button onclick={onBack} class="mb-4 text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1">
        ← Back to Tasks
      </button>
      <div class="bg-red-50 border border-red-200 rounded-lg p-4">
        <p class="text-sm font-medium text-red-800">Failed to load task detail</p>
        <p class="text-sm text-red-600 mt-1">{error}</p>
      </div>
    </div>

  <!-- Detail View -->
  {:else if detail}
    <!-- Header -->
    <div class="bg-white border-b border-gray-200 px-6 py-4 flex-shrink-0">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <button onclick={onBack} class="text-sm text-blue-600 hover:text-blue-800 flex items-center gap-1 font-medium">
            ← Back
          </button>
          <div>
            <h2 class="text-lg font-semibold text-gray-900 flex items-center gap-2">
              Task <code class="bg-gray-100 px-2 py-0.5 rounded text-sm font-mono">{detail.taskId}</code>
            </h2>
            <p class="text-xs text-gray-500 mt-0.5">
              {formatDate(detail.startedAt)} — {formatDate(detail.endedAt)} · {formatDuration(detail.startedAt, detail.endedAt)} · {formatBytes(detail.apiHistorySizeBytes)} · {elapsed}ms
            </p>
          </div>
        </div>
        <!-- Summary chips -->
        <div class="flex items-center gap-3 text-xs">
          <span class="bg-blue-100 text-blue-700 px-2 py-1 rounded font-medium">{detail.messageCount} msgs</span>
          <span class="bg-green-100 text-green-700 px-2 py-1 rounded font-medium">{detail.toolUseCount} tools</span>
          <span class="bg-purple-100 text-purple-700 px-2 py-1 rounded font-medium">{detail.thinkingCount} thinking</span>
          <span class="bg-gray-100 text-gray-700 px-2 py-1 rounded font-medium">{detail.filesInContextCount} files</span>
        </div>
      </div>

      {#if detail.taskPrompt}
        <div class="mt-2 text-xs text-gray-600 bg-gray-50 rounded p-2 font-mono truncate" title={detail.taskPrompt}>
          {detail.taskPrompt}
        </div>
      {/if}
    </div>

    <!-- Section Tabs -->
    <div class="bg-white border-b border-gray-200 px-6 flex-shrink-0">
      <div class="flex gap-1">
        {#each sections as sec}
          <button
            onclick={() => activeSection = sec.id}
            class="px-3 py-2 text-xs font-medium border-b-2 transition-colors flex items-center gap-1 {activeSection === sec.id
              ? 'border-blue-500 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            <span>{sec.icon}</span>
            {sec.label}
            {#if sec.id === 'messages'}<span class="text-gray-400 ml-0.5">({detail.messageCount})</span>{/if}
            {#if sec.id === 'tools'}<span class="text-gray-400 ml-0.5">({detail.toolCalls.length})</span>{/if}
            {#if sec.id === 'files'}<span class="text-gray-400 ml-0.5">({detail.files.length})</span>{/if}
          </button>
        {/each}
      </div>
    </div>

    <!-- Section Content -->
    <div class="flex-1 overflow-y-scroll bg-gray-50 detail-scroll">

      <!-- ============ MESSAGES (paginated) ============ -->
      {#if activeSection === 'messages'}
        <div class="p-4 max-w-5xl mx-auto">
          <!-- Controls bar: role filter + pagination -->
          <div class="bg-white border border-gray-200 rounded-lg px-4 py-2.5 mb-4 flex items-center justify-between flex-wrap gap-2">
            <!-- Role filter -->
            <div class="flex items-center gap-1.5">
              <span class="text-[10px] text-gray-500 uppercase font-medium tracking-wide mr-1">Role:</span>
              <button onclick={() => setRoleFilter(undefined)}
                class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {!msgRole ? 'bg-blue-100 text-blue-700' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'}">
                All
              </button>
              <button onclick={() => setRoleFilter('user')}
                class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {msgRole === 'user' ? 'bg-blue-100 text-blue-700' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'}">
                👤 User
              </button>
              <button onclick={() => setRoleFilter('assistant')}
                class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {msgRole === 'assistant' ? 'bg-blue-100 text-blue-700' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'}">
                🤖 Assistant
              </button>
            </div>

            <!-- Page info + navigation -->
            <div class="flex items-center gap-2">
              {#if msgLoading}
                <span class="text-[10px] text-gray-400 animate-pulse">loading…</span>
              {:else}
                <span class="text-[10px] text-gray-500">
                  {msgOffset + 1}–{Math.min(msgOffset + paginatedMessages.length, msgFilteredCount)} of {msgFilteredCount}
                  {#if msgRole}({msgTotalMessages} total){/if}
                  · {msgElapsed}ms
                </span>
              {/if}
              <button onclick={msgPrev} disabled={msgOffset === 0 || msgLoading}
                class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {msgOffset === 0 ? 'bg-gray-100 text-gray-300 cursor-not-allowed' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}">
                ← Prev
              </button>
              <button onclick={msgNext} disabled={!msgHasMore || msgLoading}
                class="px-2 py-0.5 rounded text-[10px] font-medium transition-colors {!msgHasMore ? 'bg-gray-100 text-gray-300 cursor-not-allowed' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}">
                Next →
              </button>
              <!-- Page size selector -->
              <select onchange={(e) => { msgLimit = Number((e.target as HTMLSelectElement).value); msgOffset = 0; }}
                class="text-[10px] border border-gray-200 rounded px-1 py-0.5 text-gray-600 bg-white">
                <option value="10" selected={msgLimit === 10}>10/page</option>
                <option value="20" selected={msgLimit === 20}>20/page</option>
                <option value="50" selected={msgLimit === 50}>50/page</option>
                <option value="100" selected={msgLimit === 100}>100/page</option>
              </select>
            </div>
          </div>

          <!-- Error -->
          {#if msgError}
            <div class="bg-red-50 border border-red-200 rounded-lg p-3 mb-4">
              <p class="text-xs text-red-700">{msgError}</p>
            </div>
          {/if}

          <!-- Messages list -->
          <div class="space-y-3">
            {#each paginatedMessages as msg}
              <div class="border rounded-lg {roleColor(msg.role)} p-3">
                <div class="flex items-center justify-between mb-2">
                  <span class="text-xs font-semibold text-gray-700">{roleLabel(msg.role)} <span class="text-gray-400 font-normal">#{msg.index}</span></span>
                  <div class="flex items-center gap-2">
                    <button onclick={() => expandMessage(msg.index)}
                      class="px-1.5 py-0.5 rounded text-[10px] font-medium bg-indigo-50 text-indigo-600 hover:bg-indigo-100 transition-colors"
                      title="View full untruncated content">
                      🔍 Full
                    </button>
                    {#if msg.timestamp}
                      <span class="text-[10px] text-gray-400 font-mono">{formatDate(msg.timestamp)}</span>
                    {/if}
                  </div>
                </div>
                <div class="space-y-2">
                  {#each msg.content as block}
                    <div class="text-xs">
                      {#if block.type === 'text'}
                        <div class="flex items-start gap-1.5">
                          <span class="text-gray-400 flex-shrink-0">{blockIcon(block.type)}</span>
                          <div class="text-gray-700 whitespace-pre-wrap break-words font-mono bg-white/60 rounded px-2 py-1 w-full">
                            {block.text}
                            {#if block.fullTextLength && block.text && block.fullTextLength > block.text.length}
                              <span class="text-gray-400 italic ml-1">…{block.fullTextLength - block.text.length} more chars</span>
                            {/if}
                          </div>
                        </div>
                      {:else if block.type === 'thinking'}
                        <details class="group">
                          <summary class="cursor-pointer flex items-center gap-1.5 text-gray-500 hover:text-gray-700">
                            <span>{blockIcon(block.type)}</span>
                            <span class="italic">Thinking</span>
                            <span class="text-gray-400">({block.fullTextLength?.toLocaleString()} chars)</span>
                          </summary>
                          <div class="mt-1 ml-5 text-gray-600 whitespace-pre-wrap break-words font-mono bg-amber-50 rounded px-2 py-1 border border-amber-200">
                            {block.text}
                            {#if block.fullTextLength && block.text && block.fullTextLength > block.text.length}
                              <span class="text-gray-400 italic">…{block.fullTextLength - block.text.length} more chars</span>
                            {/if}
                          </div>
                        </details>
                      {:else if block.type === 'tool_use'}
                        <div class="flex items-start gap-1.5">
                          <span class="flex-shrink-0">{blockIcon(block.type)}</span>
                          <div class="w-full">
                            <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {toolColor(block.toolName ?? '')}">
                              {block.toolName}
                            </span>
                            <span class="text-gray-400 ml-1 font-mono text-[10px]">{block.toolUseId}</span>
                            {#if block.toolInput}
                              <details class="mt-1">
                                <summary class="cursor-pointer text-gray-500 hover:text-gray-700 text-[10px]">Input</summary>
                                <pre class="mt-0.5 text-[10px] text-gray-600 bg-white rounded px-2 py-1 border overflow-x-auto">{block.toolInput}</pre>
                              </details>
                            {/if}
                          </div>
                        </div>
                      {:else if block.type === 'tool_result'}
                        <div class="flex items-start gap-1.5">
                          <span class="flex-shrink-0">{blockIcon(block.type)}</span>
                          <div class="w-full">
                            <span class="text-gray-500 text-[10px]">Result for <span class="font-mono">{block.toolUseId}</span></span>
                            {#if block.toolResultText}
                              <details>
                                <summary class="cursor-pointer text-gray-500 hover:text-gray-700 text-[10px]">Output ({block.toolResultText.length} chars)</summary>
                                <pre class="mt-0.5 text-[10px] text-gray-600 bg-green-50 rounded px-2 py-1 border border-green-200 overflow-x-auto whitespace-pre-wrap">{block.toolResultText}</pre>
                              </details>
                            {/if}
                          </div>
                        </div>
                      {:else}
                        <div class="text-gray-400 italic">Unknown block type: {block.type}</div>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            {/each}

            {#if !msgLoading && paginatedMessages.length === 0 && !msgError}
              <div class="text-center py-10 text-gray-400 text-sm">No messages{msgRole ? ` for role "${msgRole}"` : ''}</div>
            {/if}
          </div>

          <!-- Bottom pagination -->
          {#if paginatedMessages.length > 0}
            <div class="flex items-center justify-center gap-3 mt-4 pb-2">
              <button onclick={msgPrev} disabled={msgOffset === 0 || msgLoading}
                class="px-3 py-1 rounded text-xs font-medium transition-colors {msgOffset === 0 ? 'bg-gray-100 text-gray-300 cursor-not-allowed' : 'bg-white border border-gray-300 text-gray-600 hover:bg-gray-50'}">
                ← Previous
              </button>
              <span class="text-xs text-gray-500">
                Page {Math.floor(msgOffset / msgLimit) + 1} of {Math.ceil(msgFilteredCount / msgLimit)}
              </span>
              <button onclick={msgNext} disabled={!msgHasMore || msgLoading}
                class="px-3 py-1 rounded text-xs font-medium transition-colors {!msgHasMore ? 'bg-gray-100 text-gray-300 cursor-not-allowed' : 'bg-white border border-gray-300 text-gray-600 hover:bg-gray-50'}">
                Next →
              </button>
            </div>
          {/if}
        </div>

      <!-- ============ TOOLS (Enhanced with filters and success/fail) ============ -->
      {:else if activeSection === 'tools'}
        <div class="p-4 max-w-5xl mx-auto">
          <!-- Stats bar -->
          {#if toolsData}
            <div class="flex gap-3 mb-4">
              <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-gray-900">{toolsData.totalToolCalls}</div>
                <div class="text-[10px] text-gray-500">Total Calls</div>
              </div>
              <div class="bg-white border border-green-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-green-600">{toolsData.successCount}</div>
                <div class="text-[10px] text-gray-500">✓ Success</div>
              </div>
              <div class="bg-white border border-red-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-red-600">{toolsData.failureCount}</div>
                <div class="text-[10px] text-gray-500">✗ Failed</div>
              </div>
              <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-gray-400">{toolsData.noResultCount}</div>
                <div class="text-[10px] text-gray-500">⊗ No Result</div>
              </div>
              <div class="flex-1"></div>
              <div class="bg-gray-50 border border-gray-200 rounded-lg px-3 py-1 text-right">
                <div class="text-[10px] text-gray-500">Filtered: {toolsData.filteredCount} of {toolsData.totalToolCalls}</div>
                <div class="text-[10px] text-gray-400">{toolsElapsed}ms</div>
              </div>
            </div>
          {/if}

          <!-- Filters -->
          <div class="bg-white border border-gray-200 rounded-lg px-4 py-3 mb-4">
            <div class="flex items-center gap-3 flex-wrap">
              <span class="text-[10px] text-gray-500 uppercase font-medium tracking-wide">Filters:</span>
              <input
                type="text"
                bind:value={toolNameFilter}
                placeholder="Tool name..."
                class="px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                onkeydown={(e) => { if (e.key === 'Enter') applyToolFilters(); }}
              />
              <label class="flex items-center gap-1.5 cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={failedOnlyFilter}
                  class="w-3.5 h-3.5 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                />
                <span class="text-xs text-gray-600">Failed only</span>
              </label>
              <button
                onclick={applyToolFilters}
                class="px-2.5 py-1 rounded text-[10px] font-medium bg-blue-500 text-white hover:bg-blue-600 transition-colors"
              >
                Apply
              </button>
              {#if toolNameFilter || failedOnlyFilter}
                <button
                  onclick={clearToolFilters}
                  class="px-2.5 py-1 rounded text-[10px] font-medium bg-gray-200 text-gray-700 hover:bg-gray-300 transition-colors"
                >
                  Clear
                </button>
              {/if}
              {#if toolsLoading}
                <span class="text-[10px] text-gray-400 animate-pulse ml-2">loading…</span>
              {/if}
            </div>
          </div>

          <!-- Tool breakdown -->
          {#if toolsData}
            <div class="bg-white border border-gray-200 rounded-lg p-4 mb-4">
              <h3 class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-3">Tool Breakdown</h3>
              <div class="flex flex-wrap gap-2">
                {#each Object.entries(toolsData.toolBreakdown).sort((a, b) => b[1] - a[1]) as [name, count]}
                  <button
                    onclick={() => { toolNameFilter = name; applyToolFilters(); }}
                    class="flex items-center gap-1.5 hover:ring-2 hover:ring-blue-300 rounded transition-all"
                    title="Filter by {name}"
                  >
                    <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {toolColor(name)}">
                      {name}
                    </span>
                    <span class="text-xs font-mono text-gray-600">{count}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Error -->
          {#if toolsError}
            <div class="bg-red-50 border border-red-200 rounded-lg p-3 mb-4">
              <p class="text-xs text-red-700">{toolsError}</p>
            </div>
          {/if}

          <!-- Tool call timeline with success/fail -->
          {#if toolsData && toolsData.toolCalls.length > 0}
            <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
              <table class="w-full text-xs">
                <thead class="bg-gray-50 border-b border-gray-200">
                  <tr>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">#</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Status</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Tool</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Input</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Result</th>
                    <th class="text-right px-3 py-2 font-medium text-gray-600">Msg#</th>
                  </tr>
                </thead>
                <tbody>
                  {#each toolsData.toolCalls as tc}
                    <tr class="border-b border-gray-100 hover:bg-gray-50">
                      <td class="px-3 py-2 text-gray-400 font-mono">{tc.callIndex}</td>
                      <td class="px-3 py-2">
                        {#if tc.success === true}
                          <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-green-100 text-green-700" title="Success">✓</span>
                        {:else if tc.success === false}
                          <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-red-100 text-red-700" title={tc.errorText || 'Failed'}>✗</span>
                        {:else}
                          <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-gray-100 text-gray-500" title="No result">⊗</span>
                        {/if}
                      </td>
                      <td class="px-3 py-2">
                        <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {toolColor(tc.toolName)}">
                          {tc.toolName}
                        </span>
                      </td>
                      <td class="px-3 py-2 max-w-xs">
                        <div class="text-gray-600 font-mono truncate" title={tc.inputSummary}>
                          {tc.inputSummary.slice(0, 80)}{tc.inputSummary.length > 80 ? '…' : ''}
                        </div>
                        <div class="text-gray-400 text-[10px]">{tc.inputFullLength.toLocaleString()} chars</div>
                      </td>
                      <td class="px-3 py-2 max-w-xs">
                        {#if tc.success === false && tc.errorText}
                          <div class="text-red-600 font-mono truncate" title={tc.errorText}>
                            ⚠ {tc.errorText.slice(0, 60)}{tc.errorText.length > 60 ? '…' : ''}
                          </div>
                          <div class="text-gray-400 text-[10px]">{tc.resultFullLength?.toLocaleString() ?? 0} chars</div>
                        {:else if tc.resultSummary}
                          <div class="text-gray-600 font-mono truncate" title={tc.resultSummary}>
                            {tc.resultSummary.slice(0, 60)}{tc.resultSummary.length > 60 ? '…' : ''}
                          </div>
                          <div class="text-gray-400 text-[10px]">{tc.resultFullLength?.toLocaleString()} chars</div>
                        {:else}
                          <span class="text-gray-400 italic">no result</span>
                        {/if}
                      </td>
                      <td class="px-3 py-2 text-right text-gray-400 font-mono">
                        {tc.messageIndex}
                        {#if tc.resultMessageIndex}
                          <span class="text-gray-300">→{tc.resultMessageIndex}</span>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {:else if toolsData}
            <div class="text-center py-10 text-gray-400 text-sm">
              No tool calls match the current filters
            </div>
          {/if}
        </div>

      <!-- ============ THINKING (Agent reasoning chain) ============ -->
      {:else if activeSection === 'thinking'}
        <div class="p-4 max-w-5xl mx-auto">
          <!-- Stats bar -->
          {#if thinkingData}
            <div class="flex gap-3 mb-4">
              <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-gray-900">{thinkingData.totalThinkingBlocks}</div>
                <div class="text-[10px] text-gray-500">Total Blocks</div>
              </div>
              <div class="bg-white border border-purple-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-purple-600">{(thinkingData.totalCharacters / 1000).toFixed(1)}K</div>
                <div class="text-[10px] text-gray-500">Total Chars</div>
              </div>
              <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
                <div class="text-lg font-bold text-gray-600">{thinkingData.avgBlockLength.toFixed(0)}</div>
                <div class="text-[10px] text-gray-500">Avg Length</div>
              </div>
              <div class="flex-1"></div>
              <div class="bg-gray-50 border border-gray-200 rounded-lg px-3 py-1 text-right">
                <div class="text-[10px] text-gray-400">{thinkingElapsed}ms</div>
              </div>
            </div>
          {/if}

          <!-- Filters -->
          <div class="bg-white border border-gray-200 rounded-lg px-4 py-3 mb-4">
            <div class="flex items-center gap-3 flex-wrap">
              <span class="text-[10px] text-gray-500 uppercase font-medium tracking-wide">Filters:</span>
              <label class="flex items-center gap-1.5">
                <span class="text-xs text-gray-600">Max length:</span>
                <input
                  type="number"
                  bind:value={thinkingMaxLength}
                  placeholder="1000"
                  class="px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent w-20"
                  min="0"
                />
                <span class="text-[10px] text-gray-400">(0 = no truncation)</span>
              </label>
              <label class="flex items-center gap-1.5">
                <span class="text-xs text-gray-600">Min length:</span>
                <input
                  type="number"
                  bind:value={thinkingMinLength}
                  placeholder="0"
                  class="px-2 py-1 text-xs border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent w-20"
                  min="0"
                />
              </label>
              <button
                onclick={applyThinkingFilters}
                class="px-2.5 py-1 rounded text-[10px] font-medium bg-blue-500 text-white hover:bg-blue-600 transition-colors"
              >
                Apply
              </button>
              {#if thinkingMaxLength !== 1000 || thinkingMinLength !== 0}
                <button
                  onclick={clearThinkingFilters}
                  class="px-2.5 py-1 rounded text-[10px] font-medium bg-gray-200 text-gray-700 hover:bg-gray-300 transition-colors"
                >
                  Clear
                </button>
              {/if}
              {#if thinkingLoading}
                <span class="text-[10px] text-gray-400 animate-pulse ml-2">loading…</span>
              {/if}
            </div>
          </div>

          <!-- Error -->
          {#if thinkingError}
            <div class="bg-red-50 border border-red-200 rounded-lg p-3 mb-4">
              <p class="text-xs text-red-700">{thinkingError}</p>
            </div>
          {/if}

          <!-- Thinking blocks list -->
          {#if thinkingData && thinkingData.thinkingBlocks.length > 0}
            <div class="space-y-3">
              {#each thinkingData.thinkingBlocks as block, i}
                <div class="bg-white border border-amber-200 rounded-lg p-3">
                  <div class="flex items-center justify-between mb-2">
                    <div class="flex items-center gap-2">
                      <span class="text-xs font-semibold text-amber-700">💭 Block #{block.blockIndex}</span>
                      <span class="text-[10px] text-gray-400">msg #{block.messageIndex}</span>
                      {#if block.timestamp}
                        <span class="text-[10px] text-gray-400 font-mono">{formatDate(block.timestamp)}</span>
                      {/if}
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="text-[10px] text-gray-500">{block.fullLength.toLocaleString()} chars</span>
                      {#if block.isTruncated}
                        <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-100 text-amber-700">
                          truncated
                        </span>
                      {/if}
                    </div>
                  </div>
                  <details class="group" open={i < 3}>
                    <summary class="cursor-pointer text-xs text-amber-600 hover:text-amber-800 font-medium mb-1">
                      {block.isTruncated ? 'Show truncated content' : 'Show full content'}
                    </summary>
                    <pre class="text-xs text-gray-700 whitespace-pre-wrap break-words font-mono bg-amber-50 rounded px-3 py-2 border border-amber-200 max-h-96 overflow-y-auto">{block.thinking}</pre>
                  </details>
                </div>
              {/each}
            </div>
          {:else if thinkingData}
            <div class="text-center py-10 text-gray-400 text-sm">
              No thinking blocks match the current filters
            </div>
          {/if}
        </div>

      <!-- ============ FILES ============ -->
      {:else if activeSection === 'files'}
        <div class="p-4 max-w-5xl mx-auto">
          <!-- File stats -->
          <div class="flex gap-3 mb-4">
            <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
              <div class="text-lg font-bold text-gray-900">{detail.filesInContextCount}</div>
              <div class="text-[10px] text-gray-500">In Context</div>
            </div>
            <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
              <div class="text-lg font-bold text-green-600">{detail.filesEditedCount}</div>
              <div class="text-[10px] text-gray-500">Edited</div>
            </div>
            <div class="bg-white border border-gray-200 rounded-lg px-4 py-2 text-center">
              <div class="text-lg font-bold text-purple-600">{detail.filesReadCount}</div>
              <div class="text-[10px] text-gray-500">Read</div>
            </div>
          </div>

          {#if detail.files.length === 0}
            <div class="text-sm text-gray-400 italic text-center py-10">No files tracked in task_metadata.json</div>
          {:else}
            <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
              <table class="w-full text-xs">
                <thead class="bg-gray-50 border-b border-gray-200">
                  <tr>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">File Path</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Source</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">State</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Read</th>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Edited</th>
                  </tr>
                </thead>
                <tbody>
                  {#each detail.files as f}
                    <tr class="border-b border-gray-100 hover:bg-gray-50">
                      <td class="px-3 py-2 font-mono text-gray-700 max-w-xs truncate" title={f.path}>{f.path}</td>
                      <td class="px-3 py-2">
                        {#if f.recordSource}
                          <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {sourceColor(f.recordSource)}">
                            {f.recordSource}
                          </span>
                        {:else}
                          <span class="text-gray-400">—</span>
                        {/if}
                      </td>
                      <td class="px-3 py-2 text-gray-500">{f.recordState ?? '—'}</td>
                      <td class="px-3 py-2 text-gray-500 font-mono text-[10px]">{f.clineReadDate ? formatDate(f.clineReadDate) : '—'}</td>
                      <td class="px-3 py-2 text-gray-500 font-mono text-[10px]">{f.clineEditDate ? formatDate(f.clineEditDate) : '—'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>

      <!-- ============ ENVIRONMENT ============ -->
      {:else if activeSection === 'env'}
        <div class="p-4 max-w-3xl mx-auto space-y-4">
          <!-- Model Usage -->
          <div class="bg-white border border-gray-200 rounded-lg p-4">
            <h3 class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-3">Model Usage</h3>
            {#if detail.modelUsage.length === 0}
              <p class="text-sm text-gray-400 italic">No model usage data</p>
            {:else}
              <div class="space-y-2">
                {#each detail.modelUsage as mu}
                  <div class="flex items-center gap-3 text-xs bg-gray-50 rounded px-3 py-2">
                    {#if mu.mode}
                      <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {mu.mode === 'act' ? 'bg-green-100 text-green-700' : 'bg-blue-100 text-blue-700'}">
                        {mu.mode}
                      </span>
                    {/if}
                    <span class="font-mono text-gray-700">{mu.modelId ?? '—'}</span>
                    <span class="text-gray-400">via</span>
                    <span class="text-gray-600">{mu.modelProviderId ?? '—'}</span>
                    {#if mu.timestamp}
                      <span class="text-gray-400 ml-auto font-mono text-[10px]">{formatDate(mu.timestamp)}</span>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Environment -->
          <div class="bg-white border border-gray-200 rounded-lg p-4">
            <h3 class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-3">Environment</h3>
            {#if detail.environment.length === 0}
              <p class="text-sm text-gray-400 italic">No environment data</p>
            {:else}
              {#each detail.environment as env}
                <dl class="space-y-1.5 text-xs">
                  <div class="flex justify-between">
                    <dt class="text-gray-500">OS</dt>
                    <dd class="text-gray-900 font-mono">{env.osName ?? '—'} {env.osVersion ?? ''}</dd>
                  </div>
                  <div class="flex justify-between">
                    <dt class="text-gray-500">Host</dt>
                    <dd class="text-gray-900">{env.hostName ?? '—'} {env.hostVersion ?? ''}</dd>
                  </div>
                  <div class="flex justify-between">
                    <dt class="text-gray-500">Cline Version</dt>
                    <dd class="text-gray-900 font-mono">{env.clineVersion ?? '—'}</dd>
                  </div>
                  {#if env.timestamp}
                    <div class="flex justify-between">
                      <dt class="text-gray-500">Timestamp</dt>
                      <dd class="text-gray-600 font-mono text-[10px]">{formatDate(env.timestamp)}</dd>
                    </div>
                  {/if}
                </dl>
              {/each}
            {/if}
          </div>
        </div>

      <!-- ============ FOCUS CHAIN ============ -->
      {:else if activeSection === 'focus'}
        <div class="p-4 max-w-4xl mx-auto">
          {#if detail.focusChain}
            <div class="bg-white border border-gray-200 rounded-lg p-4">
              <h3 class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-3">
                Task Progress Checklist
              </h3>
              <pre class="text-xs text-gray-700 whitespace-pre-wrap font-mono leading-relaxed">{detail.focusChain}</pre>
            </div>
          {:else}
            <div class="flex items-center justify-center py-20 text-gray-400 text-sm">
              No focus chain file for this task
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- ============ FULL MESSAGE MODAL ============ -->
  {#if expandLoading || expandedMsg || expandError}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
         onkeydown={(e) => { if (e.key === 'Escape') closeExpand(); }}>
      <div class="bg-white rounded-xl shadow-2xl max-w-4xl w-full max-h-[90vh] flex flex-col overflow-hidden">
        <!-- Modal header -->
        <div class="flex items-center justify-between px-5 py-3 border-b border-gray-200 bg-gray-50 flex-shrink-0">
          {#if expandedMsg}
            <div class="flex items-center gap-2">
              <span class="text-sm font-semibold text-gray-800">{roleLabel(expandedMsg.role)}</span>
              <span class="text-xs text-gray-400">#{expandedMsg.index} of {expandedMsg.totalMessages}</span>
              {#if expandedMsg.timestamp}
                <span class="text-[10px] text-gray-400 font-mono ml-2">{formatDate(expandedMsg.timestamp)}</span>
              {/if}
              <span class="text-[10px] text-gray-400 ml-2">({expandedMsg.content.length} blocks)</span>
            </div>
          {:else if expandLoading}
            <span class="text-sm text-gray-500 animate-pulse">Loading full message…</span>
          {:else}
            <span class="text-sm text-red-600">Error</span>
          {/if}
          <button onclick={closeExpand}
            class="p-1 rounded hover:bg-gray-200 text-gray-500 hover:text-gray-700 transition-colors text-lg leading-none" title="Close (Esc)">
            ✕
          </button>
        </div>

        <!-- Modal body -->
        <div class="flex-1 overflow-y-auto p-5">
          {#if expandLoading}
            <div class="flex items-center justify-center py-16">
              <svg class="animate-spin h-6 w-6 text-blue-500 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span class="text-sm text-gray-500">Loading full content…</span>
            </div>
          {:else if expandError}
            <div class="bg-red-50 border border-red-200 rounded-lg p-4">
              <p class="text-sm text-red-700">{expandError}</p>
            </div>
          {:else if expandedMsg}
            <div class="space-y-4">
              {#each expandedMsg.content as block}
                <div class="text-xs">
                  <!-- Block header -->
                  <div class="flex items-center gap-2 mb-1">
                    <span class="text-sm">{blockIcon(block.type)}</span>
                    <span class="text-xs font-semibold text-gray-600 uppercase">{block.type}</span>
                    {#if block.textLength}
                      <span class="text-[10px] text-gray-400">{block.textLength.toLocaleString()} chars</span>
                    {/if}
                    {#if block.toolName}
                      <span class="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {toolColor(block.toolName)}">
                        {block.toolName}
                      </span>
                    {/if}
                    {#if block.toolUseId}
                      <span class="text-[10px] text-gray-400 font-mono">{block.toolUseId}</span>
                    {/if}
                    {#if block.toolInputLength}
                      <span class="text-[10px] text-gray-400">input: {block.toolInputLength.toLocaleString()} chars</span>
                    {/if}
                    {#if block.toolResultLength}
                      <span class="text-[10px] text-gray-400">result: {block.toolResultLength.toLocaleString()} chars</span>
                    {/if}
                  </div>

                  <!-- Block content -->
                  {#if block.type === 'text' && block.text}
                    <pre class="text-xs text-gray-700 whitespace-pre-wrap break-words font-mono bg-gray-50 rounded-lg px-3 py-2 border border-gray-200 max-h-96 overflow-y-auto">{block.text}</pre>
                  {:else if block.type === 'thinking' && block.text}
                    <pre class="text-xs text-gray-600 whitespace-pre-wrap break-words font-mono bg-amber-50 rounded-lg px-3 py-2 border border-amber-200 max-h-96 overflow-y-auto">{block.text}</pre>
                  {:else if block.type === 'tool_use' && block.toolInput}
                    <pre class="text-xs text-gray-600 whitespace-pre-wrap break-words font-mono bg-blue-50 rounded-lg px-3 py-2 border border-blue-200 max-h-96 overflow-y-auto">{block.toolInput}</pre>
                  {:else if block.type === 'tool_result' && block.toolResultText}
                    <pre class="text-xs text-gray-600 whitespace-pre-wrap break-words font-mono bg-green-50 rounded-lg px-3 py-2 border border-green-200 max-h-96 overflow-y-auto">{block.toolResultText}</pre>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Modal footer with nav -->
        {#if expandedMsg}
          <div class="flex items-center justify-between px-5 py-3 border-t border-gray-200 bg-gray-50 flex-shrink-0">
            <button onclick={() => { if (expandedMsg && expandedMsg.index > 0) expandMessage(expandedMsg.index - 1); }}
              disabled={!expandedMsg || expandedMsg.index === 0}
              class="px-3 py-1 rounded text-xs font-medium transition-colors {!expandedMsg || expandedMsg.index === 0 ? 'bg-gray-100 text-gray-300 cursor-not-allowed' : 'bg-white border border-gray-300 text-gray-600 hover:bg-gray-50'}">
              ← Prev Message
            </button>
            <span class="text-[10px] text-gray-400">Message {(expandedMsg?.index ?? 0) + 1} of {expandedMsg?.totalMessages ?? '?'}</span>
            <button onclick={() => { if (expandedMsg && expandedMsg.index < expandedMsg.totalMessages - 1) expandMessage(expandedMsg.index + 1); }}
              disabled={!expandedMsg || expandedMsg.index >= expandedMsg.totalMessages - 1}
              class="px-3 py-1 rounded text-xs font-medium transition-colors {!expandedMsg || expandedMsg.index >= expandedMsg.totalMessages - 1 ? 'bg-gray-100 text-gray-300 cursor-not-allowed' : 'bg-white border border-gray-300 text-gray-600 hover:bg-gray-50'}">
      Next Message →
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

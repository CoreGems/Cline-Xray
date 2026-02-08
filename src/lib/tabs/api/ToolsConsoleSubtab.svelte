<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  /** API connection info from Tauri backend */
  interface ApiInfo {
    base_url: string;
    token: string;
  }

  /** Tool info from the runtime */
  interface ToolInfo {
    operationId: string;
    method: string;
    path: string;
    description: string;
    /** OpenAPI tags for grouping/categorization */
    tags: string[];
    config: ToolConfig;
  }

  /** Tool configuration */
  interface ToolConfig {
    enabled: boolean;
    dryRun: boolean;
    useFixtures: boolean;
    recordFixtures: boolean;
  }

  /** Tool invocation result */
  interface ToolInvokeResponse {
    operationId: string;
    success: boolean;
    data: any;
    error: string | null;
    durationMs: number;
    dryRun: boolean;
    fromFixture: boolean;
    validation: ValidationResult | null;
  }

  /** Validation result */
  interface ValidationResult {
    valid: boolean;
    errors: string[];
    warnings: string[];
  }

  /** Execution log entry */
  interface ToolExecutionLog {
    id: number;
    timestamp: string;
    operationId: string;
    source: string;
    args: any;
    success: boolean;
    response: any;
    error: string | null;
    durationMs: number;
    dryRun: boolean;
    fromFixture: boolean;
    validationResult: ValidationResult | null;
  }

  let tools: ToolInfo[] = $state([]);
  let logs: ToolExecutionLog[] = $state([]);
  let loading = $state(true);
  let error = $state('');
  
  // Tool invocation state
  let selectedTool = $state<string | null>(null);
  let argsInput = $state('{}');
  let invokeResult = $state<ToolInvokeResponse | null>(null);
  let invoking = $state(false);
  let dryRunOverride = $state(false);
  
  // Tab state
  let activeTab = $state<'invoke' | 'logs' | 'config'>('invoke');
  
  // Tag filtering state
  let selectedTag = $state<string | null>(null);

  let apiInfo: ApiInfo | null = null;
  
  // Computed: unique tags from all tools
  const allTags = $derived(() => {
    const tagSet = new Set<string>();
    tools.forEach(t => t.tags?.forEach(tag => tagSet.add(tag)));
    return Array.from(tagSet).sort();
  });
  
  // Computed: filtered tools based on selected tag
  const filteredTools = $derived(() => {
    if (!selectedTag) return tools;
    const tag = selectedTag; // TypeScript needs this for narrowing
    return tools.filter(t => t.tags?.includes(tag));
  });
  
  // Note: toolsByTag could be used for grouped display in the future
  // const toolsByTag = $derived(() => { ... });

  onMount(async () => {
    try {
      apiInfo = await invoke<ApiInfo>('get_api_info');
      await loadTools();
      await loadLogs();
    } catch (e) {
      error = `Failed to initialize: ${e}`;
    } finally {
      loading = false;
    }
  });

  async function loadTools() {
    if (!apiInfo) return;
    try {
      const resp = await fetch(`${apiInfo.base_url}/tools`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = await resp.json();
      tools = data.tools || [];
    } catch (e) {
      console.error('Failed to load tools:', e);
    }
  }

  async function loadLogs() {
    if (!apiInfo) return;
    try {
      const resp = await fetch(`${apiInfo.base_url}/tools/logs`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = await resp.json();
      logs = (data.logs || []).reverse(); // Most recent first
    } catch (e) {
      console.error('Failed to load logs:', e);
    }
  }

  async function invokeTool() {
    if (!selectedTool || !apiInfo) return;
    
    invoking = true;
    invokeResult = null;
    
    try {
      const args = JSON.parse(argsInput);
      const resp = await fetch(`${apiInfo.base_url}/tools/invoke`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          operationId: selectedTool,
          args,
          source: 'ui_console',
          dryRun: dryRunOverride || undefined,
        }),
      });
      
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      invokeResult = await resp.json();
      await loadLogs(); // Refresh logs
    } catch (e: any) {
      invokeResult = {
        operationId: selectedTool,
        success: false,
        data: null,
        error: e.message || String(e),
        durationMs: 0,
        dryRun: false,
        fromFixture: false,
        validation: null,
      };
    } finally {
      invoking = false;
    }
  }

  async function clearLogs() {
    if (!apiInfo) return;
    try {
      await fetch(`${apiInfo.base_url}/tools/logs`, { method: 'DELETE' });
      logs = [];
    } catch (e) {
      console.error('Failed to clear logs:', e);
    }
  }

  function selectTool(operationId: string) {
    selectedTool = operationId;
    argsInput = '{}';
    invokeResult = null;
  }

  function getMethodColor(method: string): string {
    switch (method.toUpperCase()) {
      case 'GET': return 'bg-green-100 text-green-800';
      case 'POST': return 'bg-blue-100 text-blue-800';
      case 'PUT': return 'bg-yellow-100 text-yellow-800';
      case 'DELETE': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  }

  function formatTimestamp(ts: string): string {
    try {
      return new Date(ts).toLocaleTimeString();
    } catch {
      return ts;
    }
  }
</script>

<div class="flex-1 overflow-auto p-6">
  <div class="max-w-6xl mx-auto w-full">
    <!-- Header -->
    <div class="mb-6">
      <h2 class="text-2xl font-bold text-gray-800 mb-2">Tools Console</h2>
      <p class="text-gray-600">
        Invoke tools through the <code class="bg-gray-200 px-2 py-1 rounded text-sm">ToolRuntime</code> choke-point.
        All calls (agent, UI, tests) go through this unified interface.
      </p>
    </div>

    {#if loading}
      <div class="flex items-center gap-3 p-8 text-gray-500">
        <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        Loading Tools Console...
      </div>
    {:else if error}
      <div class="p-4 bg-red-50 rounded-lg border border-red-200">
        <p class="text-red-600">{error}</p>
      </div>
    {:else}
      <!-- Tabs -->
      <div class="flex border-b border-gray-200 mb-6">
        <button
          class="px-4 py-2 font-medium text-sm {activeTab === 'invoke' ? 'text-indigo-600 border-b-2 border-indigo-600' : 'text-gray-500 hover:text-gray-700'}"
          onclick={() => { activeTab = 'invoke'; }}
        >
          Invoke Tool
        </button>
        <button
          class="px-4 py-2 font-medium text-sm {activeTab === 'logs' ? 'text-indigo-600 border-b-2 border-indigo-600' : 'text-gray-500 hover:text-gray-700'}"
          onclick={() => { activeTab = 'logs'; }}
        >
          Execution Logs ({logs.length})
        </button>
        <button
          class="px-4 py-2 font-medium text-sm {activeTab === 'config' ? 'text-indigo-600 border-b-2 border-indigo-600' : 'text-gray-500 hover:text-gray-700'}"
          onclick={() => { activeTab = 'config'; }}
        >
          Configuration
        </button>
      </div>

      <!-- Invoke Tab -->
      {#if activeTab === 'invoke'}
        <div class="grid grid-cols-2 gap-6">
          <!-- Tool Selection -->
          <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
            <h3 class="font-semibold text-gray-700 mb-3">Select Tool</h3>
            
            <!-- Tag Filter Tabs -->
            {#if allTags().length > 0}
              <div class="flex flex-wrap gap-2 mb-3 pb-3 border-b border-gray-200">
                <button
                  class="px-2 py-1 text-xs rounded-full transition-colors {selectedTag === null ? 'bg-indigo-600 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
                  onclick={() => { selectedTag = null; }}
                >
                  All ({tools.length})
                </button>
                {#each allTags() as tag}
                  <button
                    class="px-2 py-1 text-xs rounded-full transition-colors {selectedTag === tag ? 'bg-indigo-600 text-white' : 'bg-gray-100 text-gray-600 hover:bg-gray-200'}"
                    onclick={() => { selectedTag = tag; }}
                  >
                    {tag} ({tools.filter(t => t.tags?.includes(tag)).length})
                  </button>
                {/each}
              </div>
            {/if}
            
            <div class="space-y-2 max-h-80 overflow-y-auto">
              {#each filteredTools() as tool}
                <button
                  class="w-full text-left p-3 rounded-lg border transition-colors {selectedTool === tool.operationId ? 'border-indigo-500 bg-indigo-50' : 'border-gray-200 hover:border-gray-300 hover:bg-gray-50'}"
                  onclick={() => selectTool(tool.operationId)}
                >
                  <div class="flex items-center justify-between mb-1">
                    <span class="text-sm font-semibold text-indigo-700">{tool.operationId}</span>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="px-2 py-0.5 rounded text-xs font-bold uppercase {getMethodColor(tool.method)}">
                      {tool.method}
                    </span>
                    <code class="text-sm font-mono text-gray-700">{tool.path}</code>
                  </div>
                  <p class="text-xs text-gray-500 mt-1 truncate">{tool.description}</p>
                  <div class="flex items-center gap-2 mt-2">
                    {#if !tool.config.enabled}
                      <span class="px-1.5 py-0.5 rounded text-xs bg-red-100 text-red-700">Disabled</span>
                    {/if}
                    {#if tool.config.dryRun}
                      <span class="px-1.5 py-0.5 rounded text-xs bg-yellow-100 text-yellow-700">Dry-run</span>
                    {/if}
                    {#if tool.config.useFixtures}
                      <span class="px-1.5 py-0.5 rounded text-xs bg-purple-100 text-purple-700">Fixture</span>
                    {/if}
                  </div>
                </button>
              {/each}

              {#if tools.length === 0}
                <div class="text-center text-gray-500 py-4">No tools available</div>
              {/if}
            </div>
          </div>

          <!-- Invocation Form -->
          <div class="space-y-4">
            <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
              <h3 class="font-semibold text-gray-700 mb-3">
                {selectedTool ? `Invoke: ${selectedTool}` : 'Select a tool'}
              </h3>

              {#if selectedTool}
                <div class="space-y-3">
                  <div>
                    <label class="block text-sm font-medium text-gray-600 mb-1">Arguments (JSON)</label>
                    <textarea
                      bind:value={argsInput}
                      class="w-full h-32 font-mono text-sm p-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"
                      placeholder="&#123;&#125;"
                    ></textarea>
                  </div>
                  
                  <div class="flex items-center gap-4">
                    <label class="flex items-center gap-2 text-sm text-gray-600">
                      <input type="checkbox" bind:checked={dryRunOverride} class="rounded border-gray-300" />
                      Dry-run (mock response)
                    </label>
                  </div>

                  <button
                    onclick={invokeTool}
                    disabled={invoking}
                    class="w-full py-2 px-4 bg-indigo-600 text-white font-medium rounded-lg hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                  >
                    {#if invoking}
                      <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                      </svg>
                      Invoking...
                    {:else}
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"></path>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                      </svg>
                      Invoke Tool
                    {/if}
                  </button>
                </div>
              {:else}
                <p class="text-gray-500 text-sm">Select a tool from the list to invoke it.</p>
              {/if}
            </div>

            <!-- Result -->
            {#if invokeResult}
              <div class="bg-white rounded-lg shadow-sm border {invokeResult.success ? 'border-green-200' : 'border-red-200'} p-4">
                <div class="flex items-center justify-between mb-3">
                  <h3 class="font-semibold {invokeResult.success ? 'text-green-700' : 'text-red-700'}">
                    {invokeResult.success ? '✓ Success' : '✗ Failed'}
                  </h3>
                  <div class="flex items-center gap-2 text-xs text-gray-500">
                    <span>{invokeResult.durationMs}ms</span>
                    {#if invokeResult.dryRun}
                      <span class="px-1.5 py-0.5 rounded bg-yellow-100 text-yellow-700">Dry-run</span>
                    {/if}
                    {#if invokeResult.fromFixture}
                      <span class="px-1.5 py-0.5 rounded bg-purple-100 text-purple-700">From Fixture</span>
                    {/if}
                  </div>
                </div>

                {#if invokeResult.validation && !invokeResult.validation.valid}
                  <div class="mb-3 p-2 bg-orange-50 rounded border border-orange-200">
                    <p class="text-xs font-medium text-orange-700 mb-1">Validation Issues:</p>
                    {#each invokeResult.validation.errors as err}
                      <p class="text-xs text-orange-600">• {err}</p>
                    {/each}
                  </div>
                {/if}

                {#if invokeResult.error}
                  <div class="p-3 bg-red-50 rounded">
                    <p class="text-sm text-red-600 font-mono">{invokeResult.error}</p>
                  </div>
                {:else}
                  <div class="p-3 bg-gray-50 rounded max-h-64 overflow-auto">
                    <pre class="text-xs font-mono text-gray-700 whitespace-pre-wrap">{JSON.stringify(invokeResult.data, null, 2)}</pre>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Logs Tab -->
      {#if activeTab === 'logs'}
        <div class="bg-white rounded-lg shadow-sm border border-gray-200">
          <div class="flex items-center justify-between p-4 border-b border-gray-200">
            <h3 class="font-semibold text-gray-700">Execution History</h3>
            <button
              onclick={clearLogs}
              class="px-3 py-1 text-sm text-red-600 hover:text-red-700 hover:bg-red-50 rounded"
            >
              Clear Logs
            </button>
          </div>
          
          <div class="divide-y divide-gray-100 max-h-[600px] overflow-auto">
            {#each logs as log}
              <div class="p-4 hover:bg-gray-50">
                <div class="flex items-center justify-between mb-2">
                  <div class="flex items-center gap-2">
                    <span class="w-2 h-2 rounded-full {log.success ? 'bg-green-500' : 'bg-red-500'}"></span>
                    <code class="text-sm font-semibold text-indigo-700">{log.operationId}</code>
                  </div>
                  <div class="flex items-center gap-2 text-xs text-gray-500">
                    <span>{formatTimestamp(log.timestamp)}</span>
                    <span class="text-gray-400">|</span>
                    <span>{log.durationMs}ms</span>
                    <span class="px-1.5 py-0.5 rounded bg-gray-100 text-gray-600">{log.source}</span>
                    {#if log.dryRun}
                      <span class="px-1.5 py-0.5 rounded bg-yellow-100 text-yellow-700">dry-run</span>
                    {/if}
                    {#if log.fromFixture}
                      <span class="px-1.5 py-0.5 rounded bg-purple-100 text-purple-700">fixture</span>
                    {/if}
                  </div>
                </div>
                
                <details class="text-xs">
                  <summary class="cursor-pointer text-gray-500 hover:text-gray-700">
                    {log.success ? 'View response' : `Error: ${log.error}`}
                  </summary>
                  <div class="mt-2 p-2 bg-gray-50 rounded">
                    <p class="font-medium text-gray-600 mb-1">Args:</p>
                    <pre class="text-gray-700 mb-2">{JSON.stringify(log.args, null, 2)}</pre>
                    {#if log.success && log.response}
                      <p class="font-medium text-gray-600 mb-1">Response:</p>
                      <pre class="text-gray-700">{JSON.stringify(log.response, null, 2)}</pre>
                    {/if}
                    {#if log.validationResult && !log.validationResult.valid}
                      <p class="font-medium text-orange-600 mb-1">Validation Errors:</p>
                      {#each log.validationResult.errors as err}
                        <p class="text-orange-600">• {err}</p>
                      {/each}
                    {/if}
                  </div>
                </details>
              </div>
            {/each}

            {#if logs.length === 0}
              <div class="p-8 text-center text-gray-500">
                No execution logs yet. Invoke a tool to see logs here.
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Config Tab -->
      {#if activeTab === 'config'}
        <div class="space-y-4">
          <div class="bg-blue-50 rounded-lg border border-blue-200 p-4">
            <div class="flex items-start gap-3">
              <svg class="w-5 h-5 text-blue-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
              </svg>
              <div class="text-sm text-blue-800">
                <p class="font-medium mb-1">ToolRuntime Configuration</p>
                <p>Configure enable/disable states, dry-run mode, fixtures, and circuit breakers via the API endpoints:</p>
                <ul class="mt-2 list-disc list-inside text-xs space-y-1">
                  <li><code class="bg-blue-100 px-1 rounded">PUT /tools/config</code> - Update global config</li>
                  <li><code class="bg-blue-100 px-1 rounded">PUT /tools/:operation_id/config</code> - Configure specific tool</li>
                  <li><code class="bg-blue-100 px-1 rounded">GET /tools/circuit-breakers</code> - View circuit breaker status</li>
                  <li><code class="bg-blue-100 px-1 rounded">POST /tools/fixtures</code> - Import fixtures</li>
                </ul>
              </div>
            </div>
          </div>

          <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
            <h3 class="font-semibold text-gray-700 mb-4">Tool Configurations</h3>
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead class="bg-gray-50">
                  <tr>
                    <th class="text-left px-3 py-2 font-medium text-gray-600">Operation ID</th>
                    <th class="text-center px-3 py-2 font-medium text-gray-600">Enabled</th>
                    <th class="text-center px-3 py-2 font-medium text-gray-600">Dry-run</th>
                    <th class="text-center px-3 py-2 font-medium text-gray-600">Fixtures</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-100">
                  {#each tools as tool}
                    <tr>
                      <td class="px-3 py-2">
                        <code class="text-indigo-600">{tool.operationId}</code>
                      </td>
                      <td class="px-3 py-2 text-center">
                        <span class="px-2 py-0.5 rounded text-xs {tool.config.enabled ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'}">
                          {tool.config.enabled ? 'Yes' : 'No'}
                        </span>
                      </td>
                      <td class="px-3 py-2 text-center">
                        <span class="px-2 py-0.5 rounded text-xs {tool.config.dryRun ? 'bg-yellow-100 text-yellow-700' : 'bg-gray-100 text-gray-600'}">
                          {tool.config.dryRun ? 'Yes' : 'No'}
                        </span>
                      </td>
                      <td class="px-3 py-2 text-center">
                        <span class="px-2 py-0.5 rounded text-xs {tool.config.useFixtures ? 'bg-purple-100 text-purple-700' : 'bg-gray-100 text-gray-600'}">
                          {tool.config.useFixtures ? 'Yes' : 'No'}
                        </span>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      {/if}

      <!-- Info Footer -->
      <div class="mt-6 p-4 bg-gray-50 rounded-lg border border-gray-200">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-gray-500 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
          </svg>
          <div class="text-sm text-gray-600">
            <p class="font-medium mb-1">How ToolRuntime Works</p>
            <p>The ToolRuntime is a <strong>choke-point</strong> that all tool invocations pass through. It provides:</p>
            <ul class="mt-2 list-disc list-inside text-xs space-y-1">
              <li><strong>Enable/Disable</strong> - Toggle tools on/off without code changes</li>
              <li><strong>Arg Clamps</strong> - Enforce min/max values on parameters</li>
              <li><strong>Dry-run Mode</strong> - Return mock responses without execution</li>
              <li><strong>Contract Validation</strong> - Validate against OpenAPI schema</li>
              <li><strong>Fixtures</strong> - Record/replay responses for testing</li>
              <li><strong>Circuit Breaker</strong> - Protect against cascading failures</li>
            </ul>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

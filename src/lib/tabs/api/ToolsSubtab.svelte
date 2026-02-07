<script lang="ts">
  // Tools Subtab - Auto-generated agent tools from OpenAPI spec
  import { onMount } from 'svelte';
  import { fetchToolsFromOpenApi, type AgentTool, getMethodColor } from './utils';

  let tools: AgentTool[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      tools = await fetchToolsFromOpenApi();
    } catch (e) {
      error = `Failed to load tools: ${e}`;
    } finally {
      loading = false;
    }
  });
</script>

<div class="flex-1 overflow-auto p-6">
  <div class="max-w-4xl mx-auto w-full">
    <!-- Header -->
    <div class="mb-6">
      <h2 class="text-2xl font-bold text-gray-800 mb-2">Agent Tools</h2>
      <p class="text-gray-600">
        Auto-generated from <code class="bg-gray-200 px-2 py-1 rounded text-sm">/openapi.json</code>
        — tools available to the AI agent for function calling
      </p>
    </div>

    {#if loading}
      <div class="flex items-center gap-3 p-8 text-gray-500">
        <svg class="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        Loading tools from OpenAPI spec...
      </div>
    {:else if error}
      <div class="p-4 bg-red-50 rounded-lg border border-red-200">
        <p class="text-red-600">{error}</p>
      </div>
    {:else}
      <!-- Tools Count -->
      <div class="mb-4 text-sm text-gray-500">
        {tools.length} tool{tools.length !== 1 ? 's' : ''} available
      </div>

      <!-- Tools List -->
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 divide-y divide-gray-200">
        {#each tools as tool}
          <div class="p-4 hover:bg-gray-50 transition-colors">
            <div class="flex items-start gap-3">
              <!-- Method Badge -->
              <span class="px-2 py-1 rounded text-xs font-bold uppercase min-w-[60px] text-center {getMethodColor(tool.method)}">
                {tool.method}
              </span>
              
              <!-- Path and Description -->
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <code class="text-sm font-mono text-gray-800">{tool.path}</code>
                  {#if tool.auth}
                    <span class="px-2 py-0.5 rounded text-xs bg-orange-100 text-orange-700 flex items-center gap-1">
                      <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clip-rule="evenodd"></path>
                      </svg>
                      Auth
                    </span>
                  {/if}
                </div>
                <p class="text-sm text-gray-600 mb-1.5">{tool.description}</p>
                <div class="inline-flex items-center gap-1.5 px-2.5 py-1 bg-slate-100 border border-slate-200 rounded-md">
                  <svg class="w-3.5 h-3.5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"></path>
                  </svg>
                  <span class="text-xs font-medium text-slate-500">Tool:</span>
                  <code class="text-sm font-semibold font-mono text-indigo-700">{tool.name}</code>
                </div>
                {#if tool.parameters.length > 0}
                  <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
                    <span class="text-xs font-medium text-gray-500">Parameters:</span>
                    {#each tool.parameters as param}
                      <span class="text-xs font-mono">
                        <span class="text-blue-600">{param.name}</span><span class="text-gray-400">: {param.type}</span>
                        {#if param.required}<span class="text-red-500 ml-0.5">*</span>{/if}
                      </span>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>
        {/each}

        {#if tools.length === 0}
          <div class="p-8 text-center text-gray-500">
            No tools found in the OpenAPI spec.
          </div>
        {/if}
      </div>

      <!-- Info Footer -->
      <div class="mt-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-blue-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
          </svg>
          <div class="text-sm text-blue-800">
            <p class="font-medium mb-1">How Tools Work</p>
            <p>Tools are auto-generated from the OpenAPI spec. When the agent needs data, it calls these tools which map directly to REST API endpoints. Add a new endpoint to <code class="bg-blue-100 px-1 rounded">openapi.rs</code> and it automatically appears here.</p>
          </div>
        </div>
      </div>

      <!-- Excluded Endpoints Note -->
      <div class="mt-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-gray-500 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M13.477 14.89A6 6 0 015.11 6.524l8.367 8.368zm1.414-1.414L6.524 5.11a6 6 0 018.367 8.367zM18 10a8 8 0 11-16 0 8 8 0 0116 0z" clip-rule="evenodd"></path>
          </svg>
          <div class="text-sm text-gray-600">
            <p class="font-medium mb-1">Excluded Endpoints</p>
            <p><code class="bg-gray-200 px-1 rounded">POST /agent/chat</code> (the agent itself) and <code class="bg-gray-200 px-1 rounded">GET /openapi.json</code> (meta endpoint) are excluded from tools to prevent recursion.</p>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

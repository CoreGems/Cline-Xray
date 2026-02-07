<script lang="ts">
  // REST Subtab - Displays list of available REST API endpoints with filtering
  // Fetches endpoints from OpenAPI specs at runtime (single source of truth)
  import { onMount } from 'svelte';
  import { fetchEndpointsFromOpenApi, clearEndpointCache, getMethodColor, getTagColor } from './utils';
  import type { ApiEndpoint, ApiType } from './types';

  // ============ State ============
  
  /** All endpoints fetched from OpenAPI specs */
  let endpoints: ApiEndpoint[] = [];
  
  /** Loading state while fetching */
  let isLoading = true;
  
  /** Error message if fetch fails */
  let error: string | null = null;
  
  /** Filter state - 'all' shows everything, or filter by apiType */
  let selectedFilter: 'all' | ApiType = 'all';

  // ============ Computed Values ============

  // Filtered endpoints based on selection
  $: filteredEndpoints = selectedFilter === 'all' 
    ? endpoints 
    : endpoints.filter(ep => ep.apiType === selectedFilter);

  // Count endpoints by type for display
  $: publicCount = endpoints.filter(ep => ep.apiType === 'public').length;
  $: adminCount = endpoints.filter(ep => ep.apiType === 'admin').length;

  // ============ Lifecycle ============

  onMount(() => {
    loadEndpoints();
  });

  // ============ Functions ============

  /**
   * Load endpoints from OpenAPI specs
   * Uses cached data if available
   */
  async function loadEndpoints(forceRefresh = false) {
    isLoading = true;
    error = null;
    
    try {
      endpoints = await fetchEndpointsFromOpenApi(forceRefresh);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to fetch endpoints';
      console.error('Failed to fetch endpoints:', e);
    } finally {
      isLoading = false;
    }
  }

  /**
   * Refresh endpoints (bypass cache)
   */
  async function handleRefresh() {
    clearEndpointCache();
    await loadEndpoints(true);
  }
</script>

<div class="flex-1 overflow-auto p-6">
  <div class="max-w-4xl mx-auto w-full">
    <!-- Header -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-2">
        <h2 class="text-2xl font-bold text-gray-800">REST API Endpoints</h2>
        <!-- Refresh Button -->
        <button
          class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded-lg transition-colors disabled:opacity-50"
          on:click={handleRefresh}
          disabled={isLoading}
          title="Refresh endpoints from backend"
        >
          <svg class="w-5 h-5 {isLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
        </button>
      </div>
      <div class="flex items-center justify-between">
        <p class="text-gray-600">Available API endpoints on <code class="bg-gray-200 px-2 py-1 rounded text-sm">http://localhost:3030</code></p>
        
        <!-- API Type Filter Toggle -->
        {#if !isLoading && !error}
          <div class="flex items-center gap-1 bg-gray-100 rounded-lg p-1">
            <button
              class="px-3 py-1.5 text-sm font-medium rounded-md transition-colors {selectedFilter === 'all' ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-600 hover:text-gray-900'}"
              on:click={() => selectedFilter = 'all'}
            >
              All ({endpoints.length})
            </button>
            <button
              class="px-3 py-1.5 text-sm font-medium rounded-md transition-colors {selectedFilter === 'public' ? 'bg-white text-green-700 shadow-sm' : 'text-gray-600 hover:text-gray-900'}"
              on:click={() => selectedFilter = 'public'}
            >
              <span class="flex items-center gap-1">
                <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM4.332 8.027a6.012 6.012 0 011.912-2.706C6.512 5.73 6.974 6 7.5 6A1.5 1.5 0 019 7.5V8a2 2 0 004 0 2 2 0 011.523-1.943A5.977 5.977 0 0116 10c0 .34-.028.675-.083 1H15a2 2 0 00-2 2v2.197A5.973 5.973 0 0110 16v-2a2 2 0 00-2-2 2 2 0 01-2-2 2 2 0 00-1.668-1.973z" clip-rule="evenodd"></path>
                </svg>
                Public ({publicCount})
              </span>
            </button>
            <button
              class="px-3 py-1.5 text-sm font-medium rounded-md transition-colors {selectedFilter === 'admin' ? 'bg-white text-purple-700 shadow-sm' : 'text-gray-600 hover:text-gray-900'}"
              on:click={() => selectedFilter = 'admin'}
            >
              <span class="flex items-center gap-1">
                <svg class="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd"></path>
                </svg>
                Admin ({adminCount})
              </span>
            </button>
          </div>
        {/if}
      </div>
    </div>

    <!-- Loading State -->
    {#if isLoading}
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 p-12">
        <div class="flex flex-col items-center justify-center text-gray-500">
          <svg class="w-8 h-8 animate-spin mb-4 text-blue-500" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <p class="text-lg font-medium">Loading API endpoints...</p>
          <p class="text-sm mt-1">Fetching from OpenAPI specs</p>
        </div>
      </div>

    <!-- Error State -->
    {:else if error}
      <div class="bg-red-50 rounded-lg border border-red-200 p-6">
        <div class="flex items-start gap-3">
          <svg class="w-6 h-6 text-red-500 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path>
          </svg>
          <div class="flex-1">
            <h3 class="text-lg font-medium text-red-800">Failed to load endpoints</h3>
            <p class="text-red-700 mt-1">{error}</p>
            <p class="text-red-600 text-sm mt-2">Make sure the backend server is running on port 3030.</p>
            <button
              class="mt-4 px-4 py-2 bg-red-100 text-red-800 rounded-lg hover:bg-red-200 transition-colors font-medium text-sm"
              on:click={() => loadEndpoints()}
            >
              Try Again
            </button>
          </div>
        </div>
      </div>

    <!-- API List -->
    {:else}
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 divide-y divide-gray-200">
        {#each filteredEndpoints as endpoint}
          <div class="p-4 hover:bg-gray-50 transition-colors">
            <div class="flex items-start gap-3">
              <!-- Method Badge -->
              <span class="px-2 py-1 rounded text-xs font-bold uppercase {getMethodColor(endpoint.method)} min-w-[60px] text-center">
                {endpoint.method}
              </span>
              
              <!-- Path and Description -->
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <code class="text-sm font-mono text-gray-800">{endpoint.path}</code>
                  {#if endpoint.auth}
                    <span class="px-2 py-0.5 rounded text-xs bg-orange-100 text-orange-700 flex items-center gap-1">
                      <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z" clip-rule="evenodd"></path>
                      </svg>
                      Auth
                    </span>
                  {/if}
                  {#if endpoint.apiType === 'admin'}
                    <span class="px-2 py-0.5 rounded text-xs bg-purple-100 text-purple-700">
                      internal
                    </span>
                  {/if}
                </div>
                <p class="text-sm text-gray-600">{endpoint.description}</p>
              </div>

              <!-- Tag Badges -->
              <div class="flex gap-1 flex-wrap">
                {#each endpoint.tags as tag}
                  <span class="px-2 py-1 rounded text-xs {getTagColor(tag)}">
                    {tag}
                  </span>
                {/each}
              </div>
            </div>
          </div>
        {/each}

        {#if filteredEndpoints.length === 0}
          <div class="p-8 text-center text-gray-500">
            No endpoints found for the selected filter.
          </div>
        {/if}
      </div>

      <!-- Source Info Banner -->
      <div class="mt-6 p-4 bg-green-50 rounded-lg border border-green-200">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-green-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
          </svg>
          <div class="text-sm text-green-800">
            <p class="font-medium">Live from OpenAPI</p>
            <p>Endpoints are fetched from <code class="bg-green-100 px-1 rounded">/openapi.json</code> and <code class="bg-green-100 px-1 rounded">/openapi_admin.json</code> - always in sync with the backend.</p>
          </div>
        </div>
      </div>

      <!-- Footer Info -->
      <div class="mt-4 p-4 bg-blue-50 rounded-lg border border-blue-200">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-blue-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
          </svg>
          <div class="text-sm text-blue-800">
            <p class="font-medium mb-1">Authentication</p>
            <p>Protected endpoints require a Bearer token in the Authorization header. The token should be your Jira API token.</p>
          </div>
        </div>
      </div>

      <!-- Admin API Note (shown when admin filter is active) -->
      {#if selectedFilter === 'admin'}
        <div class="mt-4 p-4 bg-purple-50 rounded-lg border border-purple-200">
          <div class="flex items-start gap-3">
            <svg class="w-5 h-5 text-purple-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd"></path>
            </svg>
            <div class="text-sm text-purple-800">
              <p class="font-medium mb-1">Admin/Internal Endpoints</p>
              <p>These endpoints are for internal use only. They are not included in the public OpenAPI spec (<code class="bg-purple-100 px-1 rounded">/openapi.json</code>) but are documented in <code class="bg-purple-100 px-1 rounded">/openapi_admin.json</code>.</p>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

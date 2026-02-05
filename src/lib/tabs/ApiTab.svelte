<script lang="ts">
  // API Tab - displays list of available REST API endpoints
  
  // Subtab state
  type ApiSubTab = 'REST';
  let activeSubTab: ApiSubTab = 'REST';
  const subTabs: { id: ApiSubTab; label: string }[] = [
    { id: 'REST', label: 'REST' }
  ];

  interface ApiEndpoint {
    method: 'GET' | 'POST' | 'PUT' | 'DELETE';
    path: string;
    description: string;
    tag: string;
    auth: boolean;
  }

  const endpoints: ApiEndpoint[] = [
    {
      method: 'GET',
      path: '/health',
      description: 'Health check endpoint - returns service status and uptime',
      tag: 'system',
      auth: false
    },
    {
      method: 'GET',
      path: '/openapi.json',
      description: 'OpenAPI specification in JSON format',
      tag: 'system',
      auth: false
    },
    {
      method: 'GET',
      path: '/access-logs',
      description: 'Get all HTTP access log entries',
      tag: 'system',
      auth: false
    },
    {
      method: 'DELETE',
      path: '/access-logs',
      description: 'Clear all HTTP access log entries',
      tag: 'system',
      auth: false
    },
    {
      method: 'GET',
      path: '/jira/list',
      description: 'List Jira issues based on JQL query',
      tag: 'jira',
      auth: true
    },
    {
      method: 'POST',
      path: '/agent/chat',
      description: 'Chat with Google Gemini AI - supports conversation history',
      tag: 'agent',
      auth: true
    }
  ];

  function getMethodColor(method: string): string {
    switch (method) {
      case 'GET': return 'bg-green-100 text-green-800';
      case 'POST': return 'bg-blue-100 text-blue-800';
      case 'PUT': return 'bg-yellow-100 text-yellow-800';
      case 'DELETE': return 'bg-red-100 text-red-800';
      default: return 'bg-gray-100 text-gray-800';
    }
  }

  function getTagColor(tag: string): string {
    switch (tag) {
      case 'system': return 'bg-purple-100 text-purple-700';
      case 'jira': return 'bg-indigo-100 text-indigo-700';
      case 'agent': return 'bg-emerald-100 text-emerald-700';
      default: return 'bg-gray-100 text-gray-700';
    }
  }
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
    <div class="flex-1 overflow-auto p-6">
      <div class="max-w-4xl mx-auto w-full">
        <!-- Header -->
        <div class="mb-6">
          <h2 class="text-2xl font-bold text-gray-800 mb-2">REST API Endpoints</h2>
          <p class="text-gray-600">Available API endpoints on <code class="bg-gray-200 px-2 py-1 rounded text-sm">http://localhost:3030</code></p>
        </div>

        <!-- API List -->
        <div class="bg-white rounded-lg shadow-sm border border-gray-200 divide-y divide-gray-200">
          {#each endpoints as endpoint}
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
                  </div>
                  <p class="text-sm text-gray-600">{endpoint.description}</p>
                </div>

                <!-- Tag Badge -->
                <span class="px-2 py-1 rounded text-xs {getTagColor(endpoint.tag)}">
                  {endpoint.tag}
                </span>
              </div>
            </div>
          {/each}
        </div>

        <!-- Footer Info -->
        <div class="mt-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
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
      </div>
    </div>
  {/if}
</div>

<script lang="ts">
  import type { JiraSettings, AgentSettings } from "../types";
  import { DEFAULT_AGENT_SETTINGS } from "../types";

  const AGENT_SETTINGS_KEY = 'agent-settings';

  interface Props {
    settings: JiraSettings;
    apiToken: string;
    onSave: (settings: JiraSettings, apiToken: string) => void;
    onClose: () => void;
  }

  let { settings, apiToken, onSave, onClose }: Props = $props();

  // Jira settings state
  let baseUrl = $state("");
  let email = $state("");
  let token = $state("");
  let defaultJql = $state("");
  
  // Agent settings state
  let filterTextGenerationOnly = $state(true);
  let requiredMethods = $state<string[]>(["generateContent"]);
  let excludeKeywords = $state<string[]>([]);
  let newKeyword = $state("");
  
  // Tab state
  let activeTab = $state<"jira" | "agent">("jira");

  // Available generation methods for filtering
  const availableMethods = [
    { id: "generateContent", label: "Generate Content (Text)", description: "Standard text generation" },
    { id: "countTokens", label: "Count Tokens", description: "Token counting capability" },
    { id: "createCachedContent", label: "Cached Content", description: "Context caching support" },
    { id: "batchGenerateContent", label: "Batch Generate", description: "Batch processing support" },
  ];

  $effect(() => {
    // Load Jira settings
    baseUrl = settings.baseUrl;
    email = settings.email;
    defaultJql = settings.defaultJql;
    token = apiToken;
    
    // Load Agent settings from localStorage
    loadAgentSettings();
  });

  function loadAgentSettings() {
    try {
      const stored = localStorage.getItem(AGENT_SETTINGS_KEY);
      if (stored) {
        const agentSettings = JSON.parse(stored) as AgentSettings;
        filterTextGenerationOnly = agentSettings.filterTextGenerationOnly;
        requiredMethods = agentSettings.requiredMethods || [...DEFAULT_AGENT_SETTINGS.requiredMethods];
        excludeKeywords = agentSettings.excludeKeywords || [...DEFAULT_AGENT_SETTINGS.excludeKeywords];
      } else {
        filterTextGenerationOnly = DEFAULT_AGENT_SETTINGS.filterTextGenerationOnly;
        requiredMethods = [...DEFAULT_AGENT_SETTINGS.requiredMethods];
        excludeKeywords = [...DEFAULT_AGENT_SETTINGS.excludeKeywords];
      }
    } catch (e) {
      console.error('Failed to load agent settings:', e);
      filterTextGenerationOnly = DEFAULT_AGENT_SETTINGS.filterTextGenerationOnly;
      requiredMethods = [...DEFAULT_AGENT_SETTINGS.requiredMethods];
      excludeKeywords = [...DEFAULT_AGENT_SETTINGS.excludeKeywords];
    }
  }

  function saveAgentSettings() {
    const agentSettings: AgentSettings = {
      filterTextGenerationOnly,
      requiredMethods,
      excludeKeywords
    };
    localStorage.setItem(AGENT_SETTINGS_KEY, JSON.stringify(agentSettings));
  }

  function toggleMethod(methodId: string) {
    if (requiredMethods.includes(methodId)) {
      requiredMethods = requiredMethods.filter(m => m !== methodId);
    } else {
      requiredMethods = [...requiredMethods, methodId];
    }
  }

  function addKeyword() {
    const keyword = newKeyword.trim();
    if (keyword && !excludeKeywords.some(k => k.toLowerCase() === keyword.toLowerCase())) {
      excludeKeywords = [...excludeKeywords, keyword];
      newKeyword = "";
    }
  }

  function removeKeyword(keyword: string) {
    excludeKeywords = excludeKeywords.filter(k => k !== keyword);
  }

  function handleKeywordKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addKeyword();
    }
  }

  function resetToDefaults() {
    excludeKeywords = [...DEFAULT_AGENT_SETTINGS.excludeKeywords];
  }

  function handleSave() {
    // Save agent settings to localStorage
    saveAgentSettings();
    
    // Save Jira settings via callback
    onSave(
      {
        baseUrl: baseUrl.trim(),
        email: email.trim(),
        defaultJql: defaultJql.trim(),
      },
      token.trim()
    );
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onClose();
    }
  }

  const tabs = [
    { id: "jira" as const, label: "Jira" },
    { id: "agent" as const, label: "Agent" },
  ];
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white rounded-lg shadow-xl w-full max-w-md mx-4">
    <div class="flex items-center justify-between p-4 border-b border-gray-200">
      <h2 class="text-lg font-semibold text-gray-900">Settings</h2>
      <button
        onclick={onClose}
        class="p-1 text-gray-400 hover:text-gray-600 rounded-md transition-colors"
        aria-label="Close settings"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
        </svg>
      </button>
    </div>

    <!-- Tab Navigation -->
    <div class="border-b border-gray-200">
      <nav class="flex -mb-px" aria-label="Settings tabs">
        {#each tabs as tab}
          <button
            onclick={() => activeTab = tab.id}
            class="px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeTab === tab.id 
              ? 'border-blue-500 text-blue-600' 
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            {tab.label}
          </button>
        {/each}
      </nav>
    </div>

    <!-- Tab Content -->
    <div class="p-4 space-y-4 max-h-[60vh] overflow-y-auto">
      {#if activeTab === "jira"}
        <!-- Jira Settings Tab -->
        <div>
          <label for="baseUrl" class="block text-sm font-medium text-gray-700 mb-1">
            Jira Base URL
          </label>
          <input
            id="baseUrl"
            type="url"
            bind:value={baseUrl}
            placeholder="https://your-domain.atlassian.net"
            class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>

        <div>
          <label for="email" class="block text-sm font-medium text-gray-700 mb-1">
            Email Address
          </label>
          <input
            id="email"
            type="email"
            bind:value={email}
            placeholder="your.email@company.com"
            class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
        </div>

        <div>
          <label for="token" class="block text-sm font-medium text-gray-700 mb-1">
            API Token
          </label>
          <input
            id="token"
            type="password"
            bind:value={token}
            placeholder="Your Jira API token"
            class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          <p class="mt-1 text-xs text-gray-500">
            Generate at <a href="https://id.atlassian.com/manage-profile/security/api-tokens" target="_blank" class="text-blue-600 hover:underline">Atlassian Account Settings</a>
          </p>
        </div>

        <div>
          <label for="defaultJql" class="block text-sm font-medium text-gray-700 mb-1">
            Default JQL Query
          </label>
          <textarea
            id="defaultJql"
            bind:value={defaultJql}
            placeholder="assignee = currentUser() ORDER BY updated DESC"
            rows="2"
            class="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
          ></textarea>
        </div>
      {:else if activeTab === "agent"}
        <!-- Agent Settings Tab -->
        <div>
          <h3 class="text-sm font-medium text-gray-900 mb-3">Model Filter Settings</h3>
          <p class="text-xs text-gray-500 mb-4">
            Configure which AI models appear in the agent model selection dropdown.
          </p>
          
          <!-- Filter Toggle -->
          <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg mb-4">
            <div>
              <p class="text-sm font-medium text-gray-700">Filter models</p>
              <p class="text-xs text-gray-500">Enable filtering by methods and keywords</p>
            </div>
            <label class="relative inline-flex items-center cursor-pointer">
              <input 
                type="checkbox" 
                bind:checked={filterTextGenerationOnly}
                class="sr-only peer"
              />
              <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
            </label>
          </div>

          {#if filterTextGenerationOnly}
            <!-- Exclude Keywords Section -->
            <div class="mb-4">
              <div class="flex items-center justify-between mb-2">
                <label class="block text-sm font-medium text-gray-700">
                  Exclude Keywords
                </label>
                <button
                  onclick={resetToDefaults}
                  class="text-xs text-blue-600 hover:text-blue-700 hover:underline"
                >
                  Reset to defaults
                </button>
              </div>
              <p class="text-xs text-gray-500 mb-2">
                Models with these keywords in name/description will be hidden.
              </p>
              
              <!-- Add keyword input -->
              <div class="flex gap-2 mb-2">
                <input
                  type="text"
                  bind:value={newKeyword}
                  onkeydown={handleKeywordKeyDown}
                  placeholder="Add keyword..."
                  class="flex-1 px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
                <button
                  onclick={addKeyword}
                  disabled={!newKeyword.trim()}
                  class="px-3 py-1.5 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
                >
                  Add
                </button>
              </div>
              
              <!-- Keywords list -->
              <div class="flex flex-wrap gap-1.5 p-2 bg-gray-50 rounded-lg min-h-[60px] max-h-[120px] overflow-y-auto">
                {#if excludeKeywords.length === 0}
                  <span class="text-xs text-gray-400 italic">No keywords configured</span>
                {:else}
                  {#each excludeKeywords as keyword}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-red-100 text-red-700 rounded-full">
                      {keyword}
                      <button
                        onclick={() => removeKeyword(keyword)}
                        class="p-0.5 hover:bg-red-200 rounded-full transition-colors"
                        title="Remove keyword"
                      >
                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                      </button>
                    </span>
                  {/each}
                {/if}
              </div>
            </div>

            <!-- Required Methods (collapsed by default) -->
            <details class="mb-4">
              <summary class="text-sm font-medium text-gray-700 cursor-pointer hover:text-gray-900">
                Advanced: Required Methods
              </summary>
              <div class="mt-2 pl-2 border-l-2 border-gray-200">
                <p class="text-xs text-gray-500 mb-2">
                  Models must support ALL selected methods.
                </p>
                <div class="space-y-1.5">
                  {#each availableMethods as method}
                    <label class="flex items-center gap-2 p-1.5 rounded hover:bg-gray-50 cursor-pointer">
                      <input
                        type="checkbox"
                        checked={requiredMethods.includes(method.id)}
                        onchange={() => toggleMethod(method.id)}
                        class="h-3.5 w-3.5 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                      />
                      <span class="text-xs text-gray-700">{method.label}</span>
                    </label>
                  {/each}
                </div>
              </div>
            </details>
          {/if}

          <!-- Info Box -->
          <div class="p-3 bg-blue-50 border border-blue-200 rounded-lg">
            <div class="flex gap-2">
              <svg class="w-5 h-5 text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              <div>
                <p class="text-xs text-blue-700">
                  <strong>Keyword filtering</strong> excludes models like Imagen, Veo, Nano Banana, and other non-text models that share the same API methods as text models.
                </p>
              </div>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <div class="flex justify-end gap-3 p-4 border-t border-gray-200">
      <button
        onclick={onClose}
        class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
      >
        Save
      </button>
    </div>
  </div>
</div>

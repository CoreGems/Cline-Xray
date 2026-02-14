<script lang="ts">
  import type { JiraSettings, AgentSettings, VendorFilterSettings } from "../types";
  import { DEFAULT_VENDOR_FILTER_SETTINGS, DEFAULT_GEMINI_FILTER_SETTINGS, DEFAULT_OPENAI_FILTER_SETTINGS } from "../types";
  import { vendorRegistry } from "./tabs/agent/vendors";
  import { fetchChangesIgnore, updateChangesIgnore } from "./tabs/changes/api";

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
  
  // Agent settings state (per-vendor)
  let vendorSettings = $state<Record<string, VendorFilterSettings>>({});
  let activeVendorTab = $state('');
  let newKeyword = $state("");
  
  // Changes ignore state
  let ignoreContent = $state("");
  let ignorePatterns = $state<string[]>([]);
  let ignoreSource = $state("");
  let ignoreFilePath = $state("");
  let ignoreLoading = $state(false);
  let ignoreSaving = $state(false);
  let ignoreError = $state("");
  let ignoreSaveSuccess = $state(false);
  let ignoreLoaded = $state(false);

  // Tab state
  let activeTab = $state<"jira" | "agent" | "changes">("jira");

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
    // Build settings from scratch (avoid reading reactive state to prevent $effect loops)
    let loaded: Record<string, VendorFilterSettings> = {};
    try {
      const stored = localStorage.getItem(AGENT_SETTINGS_KEY);
      if (stored) {
        const parsed = JSON.parse(stored) as AgentSettings;
        // Migrate: if old flat format, convert to per-vendor
        if (parsed.vendors) {
          loaded = { ...parsed.vendors };
        } else if (parsed.excludeKeywords) {
          // Legacy migration
          loaded = {
            gemini: {
              filterEnabled: parsed.filterTextGenerationOnly ?? true,
              requiredMethods: parsed.requiredMethods ?? ['generateContent'],
              excludeKeywords: parsed.excludeKeywords ?? [],
            },
          };
        }
      }
    } catch (e) {
      console.error('Failed to load agent settings:', e);
    }
    // Ensure all registered vendors have entries
    for (const vendor of vendorRegistry.getAllVendors()) {
      if (!loaded[vendor.id]) {
        loaded[vendor.id] = { ...getVendorDefaults(vendor.id) };
      }
    }
    // Single state write (no reads of vendorSettings inside this fn)
    vendorSettings = loaded;
    activeVendorTab = vendorRegistry.getAllVendors()[0]?.id ?? '';
  }

  function saveAgentSettings() {
    const agentSettings: AgentSettings = { vendors: { ...vendorSettings } };
    localStorage.setItem(AGENT_SETTINGS_KEY, JSON.stringify(agentSettings));
  }

  function toggleFilter(vendorId: string) {
    if (vendorSettings[vendorId]) {
      vendorSettings[vendorId] = { ...vendorSettings[vendorId], filterEnabled: !vendorSettings[vendorId].filterEnabled };
      vendorSettings = { ...vendorSettings };
    }
  }

  function addKeyword() {
    const keyword = newKeyword.trim();
    const vs = vendorSettings[activeVendorTab];
    if (keyword && vs && !vs.excludeKeywords.some((k: string) => k.toLowerCase() === keyword.toLowerCase())) {
      vs.excludeKeywords = [...vs.excludeKeywords, keyword];
      vendorSettings = { ...vendorSettings };
      newKeyword = "";
    }
  }

  function removeKeyword(keyword: string) {
    const vs = vendorSettings[activeVendorTab];
    if (vs) {
      vs.excludeKeywords = vs.excludeKeywords.filter((k: string) => k !== keyword);
      vendorSettings = { ...vendorSettings };
    }
  }

  function handleKeywordKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addKeyword();
    }
  }

  /** Get the default filter settings for a vendor ID */
  function getVendorDefaults(vendorId: string): VendorFilterSettings {
    if (vendorId === 'gemini') return DEFAULT_GEMINI_FILTER_SETTINGS;
    if (vendorId === 'openai') return DEFAULT_OPENAI_FILTER_SETTINGS;
    return DEFAULT_VENDOR_FILTER_SETTINGS;
  }

  function resetToDefaults() {
    vendorSettings[activeVendorTab] = { ...getVendorDefaults(activeVendorTab) };
    vendorSettings = { ...vendorSettings };
  }

  function toggleMethod(vendorId: string, methodId: string) {
    const vs = vendorSettings[vendorId];
    if (!vs) return;
    if (vs.requiredMethods.includes(methodId)) {
      vs.requiredMethods = vs.requiredMethods.filter((m: string) => m !== methodId);
    } else {
      vs.requiredMethods = [...vs.requiredMethods, methodId];
    }
    vendorSettings = { ...vendorSettings };
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

  async function loadIgnorePatterns() {
    if (ignoreLoaded) return;
    ignoreLoading = true;
    ignoreError = "";
    try {
      const result = await fetchChangesIgnore();
      ignoreContent = result.rawContent;
      ignorePatterns = result.patterns;
      ignoreSource = result.source;
      ignoreFilePath = result.filePath;
      ignoreLoaded = true;
    } catch (e: any) {
      ignoreError = e.message || "Failed to load .changesignore";
    } finally {
      ignoreLoading = false;
    }
  }

  async function saveIgnorePatterns() {
    ignoreSaving = true;
    ignoreError = "";
    ignoreSaveSuccess = false;
    try {
      const result = await updateChangesIgnore(ignoreContent);
      ignorePatterns = result.patterns;
      ignoreSource = result.source;
      ignoreSaveSuccess = true;
      setTimeout(() => { ignoreSaveSuccess = false; }, 2000);
    } catch (e: any) {
      ignoreError = e.message || "Failed to save .changesignore";
    } finally {
      ignoreSaving = false;
    }
  }

  const tabs = [
    { id: "jira" as const, label: "Jira" },
    { id: "agent" as const, label: "Agent" },
    { id: "changes" as const, label: "Changes" },
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
            onclick={() => { activeTab = tab.id; if (tab.id === 'changes') loadIgnorePatterns(); }}
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
        <!-- Agent Settings Tab (per-vendor) -->
        <div>
          <h3 class="text-sm font-medium text-gray-900 mb-1">Model Filter Settings</h3>
          <p class="text-xs text-gray-500 mb-3">
            Configure per-vendor which AI models appear in the model selector.
          </p>

          <!-- Vendor Tabs -->
          <div class="flex border-b border-gray-200 mb-4">
            {#each vendorRegistry.getAllVendors() as vendor (vendor.id)}
              <button
                onclick={() => { activeVendorTab = vendor.id; newKeyword = ''; }}
                class="flex items-center gap-1.5 px-3 py-2 text-xs font-medium border-b-2 transition-colors relative
                  {activeVendorTab === vendor.id
                    ? 'text-gray-900 border-current'
                    : 'text-gray-500 border-transparent hover:text-gray-700 hover:border-gray-300'}"
                style={activeVendorTab === vendor.id ? `color: ${vendor.branding.primaryColor}` : ''}
              >
                <span class="text-sm">{vendor.branding.icon}</span>
                <span>{vendor.name}</span>
              </button>
            {/each}
          </div>

          <!-- Active vendor filter settings -->
          {#if vendorSettings[activeVendorTab]}
            {@const vs = vendorSettings[activeVendorTab]}

            <!-- Filter Toggle -->
            <div class="flex items-center justify-between p-3 bg-gray-50 rounded-lg mb-4">
              <div>
                <p class="text-sm font-medium text-gray-700">Filter models</p>
                <p class="text-xs text-gray-500">Enable keyword & method filtering</p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input 
                  type="checkbox" 
                  checked={vs.filterEnabled}
                  onchange={() => toggleFilter(activeVendorTab)}
                  class="sr-only peer"
                />
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
              </label>
            </div>

            {#if vs.filterEnabled}
              <!-- Exclude Keywords Section -->
              <div class="mb-4">
                <div class="flex items-center justify-between mb-2">
                  <span class="block text-sm font-medium text-gray-700">Exclude Keywords</span>
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
                  {#if vs.excludeKeywords.length === 0}
                    <span class="text-xs text-gray-400 italic">No keywords configured</span>
                  {:else}
                    {#each vs.excludeKeywords as keyword}
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
                          checked={vs.requiredMethods.includes(method.id)}
                          onchange={() => toggleMethod(activeVendorTab, method.id)}
                          class="h-3.5 w-3.5 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
                        />
                        <span class="text-xs text-gray-700">{method.label}</span>
                      </label>
                    {/each}
                  </div>
                </div>
              </details>
            {/if}
          {/if}

          <!-- Info Box -->
          <div class="p-3 bg-blue-50 border border-blue-200 rounded-lg">
            <div class="flex gap-2">
              <svg class="w-5 h-5 text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              <div>
                <p class="text-xs text-blue-700">
                  <strong>Keyword filtering</strong> excludes models like Imagen, Veo, Nano Banana, and other non-text models. Each vendor has its own filter settings.
                </p>
              </div>
            </div>
          </div>
        </div>
      {:else if activeTab === "changes"}
        <!-- Changes Ignore Patterns Tab -->
        <div>
          <h3 class="text-sm font-medium text-gray-900 mb-2">.changesignore Patterns</h3>
          <p class="text-xs text-gray-500 mb-3">
            Directories and files matching these patterns are excluded from the
            Changes → Latest diff view. One pattern per line. Lines starting with
            <code class="bg-gray-100 px-1 rounded">#</code> are comments.
          </p>

          {#if ignoreLoading}
            <div class="flex items-center justify-center py-8">
              <svg class="animate-spin h-5 w-5 text-blue-500 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
              </svg>
              <span class="text-sm text-gray-500">Loading patterns…</span>
            </div>
          {:else}
            <!-- Editor textarea -->
            <textarea
              bind:value={ignoreContent}
              rows="12"
              spellcheck="false"
              class="w-full px-3 py-2 text-xs font-mono border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-y bg-gray-50"
              placeholder="# Add patterns to exclude from diffs&#10;src-tauri/target&#10;node_modules&#10;*.lock"
            ></textarea>

            <!-- Active patterns summary -->
            <div class="mt-2 flex items-center justify-between">
              <span class="text-xs text-gray-500">
                {ignorePatterns.length} active pattern{ignorePatterns.length !== 1 ? 's' : ''}
                {#if ignoreSource}
                  · <span class="text-gray-400">{ignoreSource}</span>
                {/if}
              </span>
              {#if ignoreFilePath}
                <span class="text-xs text-gray-400 truncate max-w-[200px]" title={ignoreFilePath}>
                  {ignoreFilePath}
                </span>
              {/if}
            </div>

            <!-- Error display -->
            {#if ignoreError}
              <div class="mt-2 p-2 bg-red-50 border border-red-200 rounded text-xs text-red-700">
                {ignoreError}
              </div>
            {/if}

            <!-- Success display -->
            {#if ignoreSaveSuccess}
              <div class="mt-2 p-2 bg-green-50 border border-green-200 rounded text-xs text-green-700">
                ✓ Saved successfully
              </div>
            {/if}

            <!-- Save button for this tab -->
            <div class="mt-3 flex justify-end">
              <button
                onclick={saveIgnorePatterns}
                disabled={ignoreSaving}
                class="px-4 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
              >
                {ignoreSaving ? 'Saving…' : 'Save Patterns'}
              </button>
            </div>
          {/if}

          <!-- Info Box -->
          <div class="mt-3 p-3 bg-blue-50 border border-blue-200 rounded-lg">
            <div class="flex gap-2">
              <svg class="w-5 h-5 text-blue-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
              </svg>
              <div>
                <p class="text-xs text-blue-700">
                  <strong>Patterns</strong> are applied as git pathspec exclusions
                  (<code class="bg-blue-100 px-0.5 rounded">:(exclude)&lt;pattern&gt;</code>).
                  Common patterns: <code class="bg-blue-100 px-0.5 rounded">src-tauri/target</code>,
                  <code class="bg-blue-100 px-0.5 rounded">node_modules</code>,
                  <code class="bg-blue-100 px-0.5 rounded">*.lock</code>.
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

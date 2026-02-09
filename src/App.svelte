<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "./lib/TopBar.svelte";
  import SettingsModal from "./lib/SettingsModal.svelte";
  import { jiraStore } from "./lib/stores/jiraStore.svelte";
  import { navigationStore } from "./lib/stores/navigationStore.svelte";
  import { MyJirasTab, ActivityTab, ApiTab, AgentTab, ChangesTab, HistoryTab, type TabId } from "./lib/tabs";
  import type { JiraSettings } from "./types";

  // Local UI state (not related to Jira data)
  let showSettings = $state(false);
  let error: string | null = $state(null);
  let apiToken = $state("");

  onMount(async () => {
    await jiraStore.initialize();
  });

  function handleTabChange(tabId: TabId) {
    navigationStore.activeTab = tabId;
  }

  function handleRefresh() {
    jiraStore.refresh().catch((e) => showError(`${e}`));
  }

  async function handleSaveSettings(newSettings: JiraSettings, newToken: string) {
    try {
      await jiraStore.saveSettings(newSettings, newToken);
      apiToken = newToken;
      showSettings = false;
      await jiraStore.loadIssues(newSettings.defaultJql);
    } catch (e) {
      showError(`${e}`);
    }
  }

  function showError(message: string) {
    error = message;
    setTimeout(() => {
      if (error === message) {
        error = null;
      }
    }, 5000);
  }
</script>

<div class="h-screen flex flex-col bg-gray-100">
  {#if !jiraStore.isConfigured}
    <!-- Welcome/Setup Screen -->
    <div class="flex-1 flex items-center justify-center">
      <div class="text-center max-w-md p-8">
        <svg class="w-20 h-20 mx-auto mb-6 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"></path>
        </svg>
        <h1 class="text-2xl font-bold text-gray-900 mb-2">Welcome to Cline X-Ray</h1>
        <p class="text-gray-600 mb-6">
          Configure your Jira credentials to get started. You'll need your Atlassian account email and an API token.
        </p>
        <button
          onclick={() => showSettings = true}
          class="px-6 py-3 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
        >
          Configure Settings
        </button>
      </div>
    </div>
  {:else}
    <!-- Main App Layout -->
    <TopBar
      activeTab={navigationStore.activeTab}
      onTabChange={handleTabChange}
      onRefresh={handleRefresh}
      onSettingsClick={() => showSettings = true}
    />

    <!-- Tab Content Area -->
    <div class="flex-1 flex overflow-hidden">
      {#if navigationStore.activeTab === 'my-jiras'}
        <MyJirasTab />
      {:else if navigationStore.activeTab === 'activity'}
        <ActivityTab />
      {:else if navigationStore.activeTab === 'api'}
        <ApiTab />
      {:else if navigationStore.activeTab === 'agent'}
        <AgentTab />
      {:else if navigationStore.activeTab === 'changes'}
        <ChangesTab />
      {:else if navigationStore.activeTab === 'history'}
        <HistoryTab />
      {/if}
    </div>
  {/if}

  <!-- Settings Modal -->
  {#if showSettings}
    <SettingsModal
      settings={jiraStore.settings}
      {apiToken}
      onSave={handleSaveSettings}
      onClose={() => showSettings = false}
    />
  {/if}

  <!-- Error Toast -->
  {#if error}
    <div class="fixed bottom-4 right-4 max-w-md bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded-lg shadow-lg">
      <div class="flex items-center gap-2">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
        </svg>
        <span class="text-sm">{error}</span>
        <button
          onclick={() => error = null}
          class="ml-auto p-1 hover:bg-red-200 rounded"
          aria-label="Dismiss error"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
    </div>
  {/if}
</div>

<script lang="ts">
  import { tabs, type TabId } from "./tabs";

  interface Props {
    activeTab: TabId;
    onTabChange: (tabId: TabId) => void;
    onRefresh: () => void;
    onSettingsClick: () => void;
  }

  let { activeTab, onTabChange, onRefresh, onSettingsClick }: Props = $props();
</script>

<div class="flex items-center gap-3 px-4 py-2 bg-white border-b border-gray-200 shadow-sm">
  <!-- App Title -->
  <div class="flex items-center gap-4">
    <h1 class="text-lg font-semibold text-gray-800">Cline X-Ray</h1>
    
    <!-- Tab Navigation -->
    <nav class="flex items-center gap-1 ml-4">
      {#each tabs as tab (tab.id)}
        <button
          onclick={() => onTabChange(tab.id)}
          class="px-4 py-1.5 text-sm font-medium rounded-md transition-colors {activeTab === tab.id 
            ? 'bg-blue-100 text-blue-700' 
            : 'text-gray-600 hover:text-gray-800 hover:bg-gray-100'}"
        >
          {tab.label}
        </button>
      {/each}
    </nav>
  </div>

  <!-- Spacer -->
  <div class="flex-1"></div>

  <!-- Action Buttons -->
  <button
    onclick={onRefresh}
    class="p-2 text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-md transition-colors"
    title="Refresh"
  >
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
    </svg>
  </button>
  <button
    onclick={onSettingsClick}
    class="p-2 text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded-md transition-colors"
    title="Settings"
  >
    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
    </svg>
  </button>
</div>

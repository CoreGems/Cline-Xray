<script lang="ts">
  // API Tab - Main container with subtab navigation
  import RESTSubtab from "./RESTSubtab.svelte";
  import ToolsSubtab from "./ToolsSubtab.svelte";
  import ToolsConsoleSubtab from "./ToolsConsoleSubtab.svelte";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  
  const subTabs: { id: 'REST' | 'Tools' | 'Console', label: string }[] = [
    { id: 'REST', label: 'REST' },
    { id: 'Tools', label: 'Tools' },
    { id: 'Console', label: 'Tools Console' }
  ];
</script>

<div class="flex-1 flex flex-col h-full bg-gray-50">
  <!-- Subtab Navigation -->
  <div class="bg-white border-b border-gray-200 px-4">
    <div class="flex gap-1">
      {#each subTabs as tab}
        <button
          onclick={() => navigationStore.activeApiSubTab = tab.id}
          class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {navigationStore.activeApiSubTab === tab.id
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Subtab Content -->
  {#if navigationStore.activeApiSubTab === 'REST'}
    <RESTSubtab />
  {:else if navigationStore.activeApiSubTab === 'Tools'}
    <ToolsSubtab />
  {:else if navigationStore.activeApiSubTab === 'Console'}
    <ToolsConsoleSubtab />
  {/if}
</div>

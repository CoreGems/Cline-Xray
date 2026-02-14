<script lang="ts">
  // Agent Tab - Main container with subtab navigation
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import { stashStore } from "../../stores/stashStore.svelte";
  import ChatSubtab from "./ChatSubtab.svelte";
  import StashedSubtab from "./StashedSubtab.svelte";
  import type { AgentSubTab } from "./types";

  const subtabs: AgentSubTab[] = ['Chat', 'Stashed'];

  let activeSubTab = $derived(navigationStore.activeAgentSubTab);

  function setSubTab(tab: AgentSubTab) {
    navigationStore.activeAgentSubTab = tab;
  }
</script>

<div class="flex-1 flex flex-col h-full bg-gray-50">
  <!-- Subtab bar -->
  <div class="flex border-b border-gray-200 bg-white">
    {#each subtabs as tab}
      <button
        onclick={() => setSubTab(tab)}
        class="px-4 py-1.5 text-sm font-medium border-b-2 transition-colors flex items-center gap-1.5
          {activeSubTab === tab
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
      >
        {#if tab === 'Stashed'}
          <svg class="w-3.5 h-3.5 {activeSubTab === tab ? 'text-amber-500' : 'text-gray-400'}" fill="currentColor" viewBox="0 0 24 24"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"></path></svg>
          {tab}
          {#if stashStore.count > 0}
            <span class="text-[10px] min-w-[18px] h-[18px] flex items-center justify-center rounded-full {activeSubTab === tab ? 'bg-amber-100 text-amber-700' : 'bg-gray-100 text-gray-500'}">{stashStore.count}</span>
          {/if}
        {:else}
          {tab}
        {/if}
      </button>
    {/each}
  </div>

  <!-- Subtab content -->
  <div class="flex-1 flex flex-col overflow-hidden">
    {#if activeSubTab === 'Chat'}
      <ChatSubtab />
    {:else if activeSubTab === 'Stashed'}
      <StashedSubtab />
    {/if}
  </div>
</div>

<script lang="ts">
  // Agent Tab - Main container with subtab navigation
  import ChatSubtab from "./ChatSubtab.svelte";
  import AgentChatSubtab from "./AgentChatSubtab.svelte";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import type { SubTabDefinition } from "./types";
  
  const subTabs: SubTabDefinition[] = [
    { id: 'Agent Chat', label: 'Agent Chat' },
    { id: 'Chat', label: 'Chat' }
  ];
</script>

<div class="flex-1 flex flex-col h-full bg-gray-50">
  <!-- Subtab Navigation -->
  <div class="bg-white border-b border-gray-200 px-4">
    <div class="flex gap-1">
      {#each subTabs as tab}
        <button
          onclick={() => navigationStore.activeAgentSubTab = tab.id}
          class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {navigationStore.activeAgentSubTab === tab.id
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Subtab Content -->
  {#if navigationStore.activeAgentSubTab === 'Chat'}
    <ChatSubtab />
  {:else if navigationStore.activeAgentSubTab === 'Agent Chat'}
    <AgentChatSubtab />
  {/if}
</div>

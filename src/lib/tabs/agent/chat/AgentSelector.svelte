<!-- Agent Selector Header Bar -->
<script lang="ts">
  import { chatState } from "./chatState.svelte";
</script>

<div class="flex items-center gap-3 px-3 py-1.5 border-b border-gray-200 bg-white">
  <div class="flex items-center gap-2 flex-1">
    <span class="text-sm text-gray-600">Agent:</span>
    {#if chatState.agents.length === 0}
      <span class="text-sm text-gray-400 italic">No agents configured</span>
    {:else}
      <div class="flex items-center gap-2 flex-wrap">
        {#each chatState.agents as agent (agent.id)}
          <div class="relative group">
            <button
              onclick={() => chatState.selectAgent(agent.id)}
              class="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-sm transition-all {chatState.selectedAgentId === agent.id ? 'ring-2 ring-offset-1' : 'hover:bg-gray-100'}"
              style="background-color: {chatState.selectedAgentId === agent.id ? agent.color + '20' : 'transparent'}; color: {agent.color}; {chatState.selectedAgentId === agent.id ? `ring-color: ${agent.color}` : ''}"
              title={agent.description || agent.name}
            >
              <span class="w-2 h-2 rounded-full" style="background-color: {agent.color}"></span>
              <span class="font-medium">{agent.name}</span>
            </button>
            <button
              onclick={(e) => { e.stopPropagation(); chatState.openEditAgentModal(agent); }}
              class="absolute -top-1 -right-1 p-0.5 bg-white border border-gray-200 rounded-full shadow-sm opacity-0 group-hover:opacity-100 transition-opacity hover:bg-gray-100"
              title="Edit agent"
            >
              <svg class="w-3 h-3 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path></svg>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
  <button
    onclick={() => chatState.openAddAgentModal()}
    class="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
    title="Add a new agent"
  >
    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
    Add Agent
  </button>
</div>

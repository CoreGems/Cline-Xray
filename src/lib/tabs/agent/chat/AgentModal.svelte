<!-- Add/Edit Agent Modal -->
<script lang="ts">
  import { chatState, agentColors } from "./chatState.svelte";
</script>

{#if chatState.showAddAgentModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <div class="absolute inset-0 bg-black/50" onclick={() => chatState.closeAddAgentModal()} onkeydown={(e) => e.key === 'Escape' && chatState.closeAddAgentModal()} role="button" tabindex="0"></div>
    
    <div class="relative bg-white rounded-xl shadow-2xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
      <div class="flex items-center justify-between p-4 border-b border-gray-200">
        <h2 class="text-lg font-semibold text-gray-800">{chatState.editingAgent ? 'Edit Agent' : 'Add New Agent'}</h2>
        <button onclick={() => chatState.closeAddAgentModal()} class="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
        </button>
      </div>
      
      <!-- Tab bar -->
      <div class="border-b border-gray-200 bg-gray-50">
        <nav class="flex px-4" aria-label="Tabs">
          <button onclick={() => chatState.activeModalTab = 'basic'} class="px-4 py-3 text-sm font-medium border-b-2 transition-colors {chatState.activeModalTab === 'basic' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}">Basic Settings</button>
          <button onclick={() => chatState.activeModalTab = 'advanced'} class="px-4 py-3 text-sm font-medium border-b-2 transition-colors {chatState.activeModalTab === 'advanced' ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}">Advanced</button>
        </nav>
      </div>

      <div class="p-4 space-y-4">
        {#if chatState.activeModalTab === 'basic'}
          <div>
            <label for="agent-name" class="block text-sm font-medium text-gray-700 mb-1">Name <span class="text-red-500">*</span></label>
            <input id="agent-name" type="text" value={chatState.newAgentName} oninput={(e) => chatState.newAgentName = (e.target as HTMLInputElement).value} placeholder="e.g., Code Assistant, Task Manager" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent" />
          </div>
          <div>
            <label for="agent-description" class="block text-sm font-medium text-gray-700 mb-1">Description</label>
            <input id="agent-description" type="text" value={chatState.newAgentDescription} oninput={(e) => chatState.newAgentDescription = (e.target as HTMLInputElement).value} placeholder="Brief description of what this agent does" class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent" />
          </div>
          <div>
            <label for="agent-model" class="block text-sm font-medium text-gray-700 mb-1">Default Agent Model</label>
            {#if chatState.isLoadingAgentModels}
              <div class="flex items-center gap-2 px-3 py-2 border border-gray-300 rounded-lg bg-gray-50">
                <div class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
                <span class="text-sm text-gray-500">Loading models...</span>
              </div>
            {:else if chatState.agentModelsError}
              <div class="px-3 py-2 border border-red-300 rounded-lg bg-red-50">
                <p class="text-sm text-red-600">{chatState.agentModelsError}</p>
                <button onclick={() => chatState.loadAgentModels(true)} class="text-xs text-red-700 hover:underline mt-1">Retry</button>
              </div>
            {:else}
              <select id="agent-model" value={chatState.newAgentModel} onchange={(e) => chatState.newAgentModel = (e.target as HTMLSelectElement).value} class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white">
                <option value="">Select a model (optional)</option>
                {#each chatState.agentAvailableModels as model (model.name)}
                  <option value={model.name}>{model.displayName || model.name}</option>
                {/each}
              </select>
              {#if chatState.newAgentModel}
                {@const selectedAgentModel = chatState.agentAvailableModels.find(m => m.name === chatState.newAgentModel)}
                {#if selectedAgentModel?.description}
                  <p class="mt-1 text-xs text-gray-500">{selectedAgentModel.description}</p>
                {/if}
              {/if}
            {/if}
          </div>
          <div>
            <label for="agent-prompt" class="block text-sm font-medium text-gray-700 mb-1">System Prompt</label>
            <textarea id="agent-prompt" value={chatState.newAgentSystemPrompt} oninput={(e) => chatState.newAgentSystemPrompt = (e.target as HTMLTextAreaElement).value} placeholder="Instructions for the agent's behavior and capabilities..." rows="4" class="w-full px-3 py-2 border border-gray-300 rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"></textarea>
            <p class="mt-1 text-xs text-gray-500">Define the agent's personality, capabilities, and constraints.</p>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">Color</label>
            <div class="flex items-center gap-2 flex-wrap">
              {#each agentColors as color}
                <button onclick={() => chatState.newAgentColor = color} class="w-8 h-8 rounded-full transition-all {chatState.newAgentColor === color ? 'ring-2 ring-offset-2 ring-gray-400 scale-110' : 'hover:scale-105'}" style="background-color: {color}" title={color}></button>
              {/each}
            </div>
          </div>
          <div class="bg-gray-50 rounded-lg p-3">
            <p class="text-xs text-gray-500 mb-2">Preview:</p>
            <div class="flex items-center gap-2">
              <span class="w-3 h-3 rounded-full" style="background-color: {chatState.newAgentColor}"></span>
              <span class="font-medium" style="color: {chatState.newAgentColor}">{chatState.newAgentName || 'Agent Name'}</span>
            </div>
            {#if chatState.newAgentDescription}
              <p class="text-sm text-gray-600 mt-1 ml-5">{chatState.newAgentDescription}</p>
            {/if}
          </div>
        {:else}
          <div class="text-center py-8 text-gray-500">
            <svg class="w-12 h-12 mx-auto mb-3 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path></svg>
            <p class="text-sm font-medium">Advanced Settings</p>
            <p class="text-xs text-gray-400 mt-1">Coming soon - configure tools, APIs, and more.</p>
          </div>
        {/if}
      </div>
      
      <div class="flex items-center justify-between p-4 border-t border-gray-200 bg-gray-50">
        {#if chatState.editingAgent}
          <button onclick={(e) => chatState.deleteAgent(chatState.editingAgent!.id, e)} class="px-3 py-2 text-sm font-medium text-red-600 hover:text-red-700 hover:bg-red-50 rounded-lg transition-colors">Delete Agent</button>
        {:else}
          <div></div>
        {/if}
        <div class="flex items-center gap-2">
          <button onclick={() => chatState.closeAddAgentModal()} class="px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-100 rounded-lg transition-colors">Cancel</button>
          <button onclick={() => chatState.saveAgent()} disabled={!chatState.newAgentName.trim()} class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed">{chatState.editingAgent ? 'Save Changes' : 'Create Agent'}</button>
        </div>
      </div>
    </div>
  </div>
{/if}

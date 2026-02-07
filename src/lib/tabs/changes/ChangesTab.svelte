<script lang="ts">
  // Changes Tab - Main container with subtab navigation
  import TaskListSubtab from "./TaskListSubtab.svelte";
  import type { ChangesSubTab, SubTabDefinition } from "./types";

  // Subtab state
  let activeSubTab: ChangesSubTab = $state('Tasks');

  const subTabs: SubTabDefinition[] = [
    { id: 'Tasks', label: 'Tasks' },
    { id: 'Diff', label: 'Diff' },
    { id: 'Export', label: 'Export' },
  ];
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

  <!-- Subtab Content -->
  {#if activeSubTab === 'Tasks'}
    <TaskListSubtab />
  {:else if activeSubTab === 'Diff'}
    <div class="flex-1 flex items-center justify-center text-gray-400 text-sm">
      Diff view — coming soon
    </div>
  {:else if activeSubTab === 'Export'}
    <div class="flex-1 flex items-center justify-center text-gray-400 text-sm">
      Export — coming soon
    </div>
  {/if}
</div>

<script lang="ts">
  // Changes Tab - Main container with subtab navigation
  import TaskListSubtab from "./TaskListSubtab.svelte";
  import LatestSubtab from "./LatestSubtab.svelte";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import type { SubTabDefinition } from "./types";

  const subTabs: SubTabDefinition[] = [
    { id: 'Latest', label: '⚡ Latest' },
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
          onclick={() => navigationStore.activeChangesSubTab = tab.id}
          class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {navigationStore.activeChangesSubTab === tab.id
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Subtab Content -->
  {#if navigationStore.activeChangesSubTab === 'Latest'}
    <LatestSubtab />
  {:else if navigationStore.activeChangesSubTab === 'Tasks'}
    <TaskListSubtab />
  {:else if navigationStore.activeChangesSubTab === 'Diff'}
    <div class="flex-1 flex items-center justify-center text-gray-400 text-sm">
      Diff view — coming soon
    </div>
  {:else if navigationStore.activeChangesSubTab === 'Export'}
    <div class="flex-1 flex items-center justify-center text-gray-400 text-sm">
      Export — coming soon
    </div>
  {/if}
</div>

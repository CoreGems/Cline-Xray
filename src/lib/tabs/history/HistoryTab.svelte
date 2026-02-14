<script lang="ts">
  // History Tab - Main container with subtab navigation + task detail view
  import HistoryLatestSubtab from "./HistoryLatestSubtab.svelte";
  import HistoryTaskListSubtab from "./HistoryTaskListSubtab.svelte";
  import HistoryStatsSubtab from "./HistoryStatsSubtab.svelte";
  import TaskDetailView from "./TaskDetailView.svelte";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import type { SubTabDefinition } from "./types";

  // Task detail navigation
  let selectedTaskId: string | null = $state(null);

  const subTabs: SubTabDefinition[] = [
    { id: 'Latest', label: '⚡ Latest' },
    { id: 'Tasks', label: 'Tasks' },
    { id: 'Stats', label: 'Stats' },
  ];

  function openTaskDetail(taskId: string) {
    selectedTaskId = taskId;
  }

  function closeTaskDetail() {
    selectedTaskId = null;
  }
</script>

<div class="flex-1 flex flex-col h-full bg-gray-50">
  {#if selectedTaskId}
    <!-- Task Detail View (replaces everything) -->
    <TaskDetailView taskId={selectedTaskId} onBack={closeTaskDetail} />
  {:else}
    <!-- Subtab Navigation -->
    <div class="bg-white border-b border-gray-200 px-4">
      <div class="flex gap-1">
        {#each subTabs as tab}
          <button
            onclick={() => navigationStore.activeHistorySubTab = tab.id}
            class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {navigationStore.activeHistorySubTab === tab.id
              ? 'border-blue-500 text-blue-600'
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            {tab.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Subtab Content -->
    {#if navigationStore.activeHistorySubTab === 'Latest'}
      <HistoryLatestSubtab onViewDetail={openTaskDetail} />
    {:else if navigationStore.activeHistorySubTab === 'Tasks'}
      <HistoryTaskListSubtab onViewDetail={openTaskDetail} />
    {:else if navigationStore.activeHistorySubTab === 'Stats'}
      <HistoryStatsSubtab />
    {/if}
  {/if}
</div>

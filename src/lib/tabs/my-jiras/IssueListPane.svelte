<script lang="ts">
  // Issue List Pane - Left sidebar showing list of Jira issues
  import { jiraStore } from "../../stores/jiraStore.svelte";
  import { getStatusClass, formatDate, truncate } from "../../../modules/jira-details";
  import type { IssueSummary, IssueDetails } from "./types";

  // Props
  interface Props {
    selectedIssue: IssueDetails | null;
    onSelectIssue: (issue: IssueSummary) => void;
  }
  
  let { selectedIssue, onSelectIssue }: Props = $props();
  
  // Search filter state
  let searchQuery = $state("");
  
  // Filtered issues based on search query
  let filteredIssues = $derived(() => {
    if (!searchQuery.trim()) {
      return jiraStore.issues;
    }
    const query = searchQuery.toLowerCase().trim();
    return jiraStore.issues.filter((issue) => 
      issue.key.toLowerCase().includes(query) ||
      issue.summary.toLowerCase().includes(query) ||
      issue.status.toLowerCase().includes(query) ||
      (issue.assignee && issue.assignee.toLowerCase().includes(query)) ||
      issue.priority.toLowerCase().includes(query)
    );
  });

  // Refresh handler
  async function handleRefresh() {
    try {
      await jiraStore.refresh();
    } catch (e) {
      console.error("Failed to refresh:", e);
    }
  }
</script>

<div class="w-80 flex-shrink-0 h-full flex flex-col bg-gray-50 border-r border-gray-200">
  <div class="p-3 border-b border-gray-200 bg-white">
    <div class="flex items-center justify-between mb-2">
      <h2 class="text-sm font-semibold text-gray-700">Issues ({filteredIssues().length}{searchQuery ? ` / ${jiraStore.issueCount}` : ''})</h2>
      <button
        onclick={handleRefresh}
        disabled={jiraStore.listLoading}
        class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        title="Refresh issues"
      >
        <svg 
          class="w-5 h-5 {jiraStore.listLoading ? 'animate-spin' : ''}" 
          fill="none" 
          stroke="currentColor" 
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
        </svg>
      </button>
    </div>
    <!-- Search Bar -->
    <div class="relative">
      <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
      </svg>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter issues..."
        class="w-full pl-8 pr-8 py-1.5 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500"
      />
      {#if searchQuery}
        <button
          onclick={() => searchQuery = ""}
          class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
          title="Clear search"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      {/if}
    </div>
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if jiraStore.listLoading}
      <div class="flex items-center justify-center h-32">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
      </div>
    {:else if jiraStore.listError}
      <div class="flex items-center justify-center h-32 text-red-500 text-sm p-4 text-center">
        {jiraStore.listError}
      </div>
    {:else if !jiraStore.hasIssues}
      <div class="flex items-center justify-center h-32 text-gray-500 text-sm">
        No issues found
      </div>
    {:else if filteredIssues().length === 0}
      <div class="flex items-center justify-center h-32 text-gray-500 text-sm">
        No matching issues
      </div>
    {:else}
      {#each filteredIssues() as issue (issue.key)}
        <button
          onclick={() => onSelectIssue(issue)}
          class="w-full text-left p-3 border-b border-gray-100 hover:bg-gray-100 cursor-pointer transition-colors {selectedIssue?.key === issue.key ? 'bg-blue-50 border-l-2 border-l-blue-500' : ''}"
        >
          <div class="flex items-center gap-2 mb-1">
            <span class="text-blue-600 font-medium text-sm">{issue.key}</span>
            <span class="px-2 py-0.5 text-xs rounded-full {getStatusClass(issue.statusCategory)}">
              {issue.status}
            </span>
          </div>
          <div class="text-sm text-gray-800 mb-1">
            {truncate(issue.summary, 60)}
          </div>
          <div class="flex items-center gap-3 text-xs text-gray-500">
            {#if issue.assignee}
              <span>{issue.assignee}</span>
            {:else}
              <span class="italic">Unassigned</span>
            {/if}
            <span>{formatDate(issue.updated)}</span>
            <span class="text-gray-400">{issue.priority}</span>
          </div>
        </button>
      {/each}
    {/if}
  </div>
</div>

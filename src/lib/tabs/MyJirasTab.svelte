<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { jiraStore } from "../stores/jiraStore.svelte";
  import type { IssueDetails, IssueSummary } from "../../types";

  // Selected issue state
  let selectedIssue: IssueDetails | null = $state(null);
  let detailsLoading = $state(false);
  let detailsError: string | null = $state(null);
  
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
  
  // Details pane tab state
  type DetailTab = 'details';
  let activeDetailTab: DetailTab = $state('details');

  // Refresh handler
  async function handleRefresh() {
    try {
      await jiraStore.refresh();
    } catch (e) {
      console.error("Failed to refresh:", e);
    }
  }

  // Select issue handler - fetches full details from Jira API
  async function handleSelectIssue(issue: IssueSummary) {
    // Skip if already selected
    if (selectedIssue?.key === issue.key) return;
    
    await fetchIssueDetails(issue.key, issue);
  }

  // Fetch issue details from API
  async function fetchIssueDetails(key: string, basicInfo?: IssueSummary) {
    detailsLoading = true;
    detailsError = null;
    
    // Show basic info immediately while loading full details (if available)
    if (basicInfo) {
      selectedIssue = {
        key: basicInfo.key,
        summary: basicInfo.summary,
        status: basicInfo.status,
        statusCategory: basicInfo.statusCategory,
        priority: basicInfo.priority,
        assignee: basicInfo.assignee,
        reporter: null,
        issueType: "Loading...",
        created: basicInfo.updated,
        updated: basicInfo.updated,
        description: null,
        labels: [],
        components: [],
        resolution: null,
      };
    }
    
    try {
      // Fetch full issue details from Jira API
      const details = await invoke<IssueDetails>("get_issue", { key });
      selectedIssue = details;
    } catch (e) {
      console.error("Failed to fetch issue details:", e);
      detailsError = e instanceof Error ? e.message : String(e);
    } finally {
      detailsLoading = false;
    }
  }

  // Refresh current issue details
  async function handleRefreshDetails() {
    if (selectedIssue) {
      await fetchIssueDetails(selectedIssue.key);
    }
  }

  // Helper functions for issue list
  function getStatusClass(statusCategory: string): string {
    switch (statusCategory.toLowerCase()) {
      case "done":
        return "status-done";
      case "in progress":
      case "indeterminate":
        return "status-in-progress";
      case "blocked":
        return "status-blocked";
      default:
        return "status-todo";
    }
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleDateString();
  }

  function formatDateTime(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString();
  }

  function truncate(text: string, maxLength: number): string {
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength) + "...";
  }

  function renderDescription(description: string | null): string {
    if (!description) return "<p class='text-gray-400 italic'>No description</p>";
    // Basic Jira markup to HTML conversion
    // Note: whitespace-pre-wrap CSS handles newlines, so no need for <br> replacement
    return description
      .replace(/\*(\w+)\*/g, "<strong>$1</strong>")
      .replace(/_(\w+)_/g, "<em>$1</em>");
  }
</script>

<div class="flex-1 flex overflow-hidden">
  <!-- Left Pane - Issue List -->
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
            onclick={() => handleSelectIssue(issue)}
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

  <!-- Main Pane - Issue Details -->
  <div class="flex-1 h-full flex flex-col bg-white min-w-0">
    {#if !selectedIssue}
      <div class="flex items-center justify-center h-full text-gray-400">
        <div class="text-center">
          <svg class="w-16 h-16 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path>
          </svg>
          <p>Select an issue to view details</p>
        </div>
      </div>
    {:else}
      <div class="flex-1 flex flex-col overflow-hidden relative">
        <!-- Loading Overlay -->
        {#if detailsLoading}
          <div class="absolute inset-0 bg-white/70 flex items-center justify-center z-10">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
          </div>
        {/if}
        
        <!-- Fixed Header -->
        <div class="px-6 pt-6 pb-4 border-b border-gray-200 bg-white">
          <!-- Error Banner -->
          {#if detailsError}
            <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
              <strong>Error loading details:</strong> {detailsError}
            </div>
          {/if}
          
          <!-- Issue Header -->
          <div class="flex items-center gap-3 mb-2">
            <a href="{selectedIssue.key}" class="text-blue-600 font-semibold text-lg hover:underline">{selectedIssue.key}</a>
            <span class="px-3 py-1 text-sm rounded-full {getStatusClass(selectedIssue.statusCategory)}">
              {selectedIssue.status}
            </span>
            {#if selectedIssue.resolution}
              <span class="px-3 py-1 text-sm rounded-full bg-green-100 text-green-800">
                {selectedIssue.resolution}
              </span>
            {/if}
            <button
              onclick={handleRefreshDetails}
              disabled={detailsLoading}
              class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed ml-auto"
              title="Refresh issue details"
            >
              <svg 
                class="w-5 h-5 {detailsLoading ? 'animate-spin' : ''}" 
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
              </svg>
            </button>
          </div>
          <h1 class="text-2xl font-bold text-gray-900 mb-4">{selectedIssue.summary}</h1>
          
          <!-- Tab Bar -->
          <div class="flex gap-1 -mb-4">
            <button
              onclick={() => activeDetailTab = 'details'}
              class="px-4 py-2 text-sm font-medium rounded-t-lg transition-colors {activeDetailTab === 'details' ? 'bg-gray-50 text-blue-600 border-t border-l border-r border-gray-200' : 'text-gray-600 hover:text-gray-900 hover:bg-gray-100'}"
            >
              Details
            </button>
            <!-- Future tabs can be added here -->
          </div>
        </div>

        <!-- Tab Content -->
        <div class="flex-1 overflow-y-auto overflow-x-hidden">
          {#if activeDetailTab === 'details'}
            <div class="p-6 bg-gray-50">
              <!-- Metadata Grid -->
              <div class="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6 p-4 bg-white rounded-lg border border-gray-200">
                <div>
                  <span class="text-xs text-gray-500 uppercase">Type</span>
                  <p class="text-sm font-medium">{selectedIssue.issueType}</p>
                </div>
                <div>
                  <span class="text-xs text-gray-500 uppercase">Priority</span>
                  <p class="text-sm font-medium">{selectedIssue.priority}</p>
                </div>
                <div>
                  <span class="text-xs text-gray-500 uppercase">Assignee</span>
                  <p class="text-sm font-medium">{selectedIssue.assignee || "Unassigned"}</p>
                </div>
                <div>
                  <span class="text-xs text-gray-500 uppercase">Reporter</span>
                  <p class="text-sm font-medium">{selectedIssue.reporter || "Unknown"}</p>
                </div>
                <div>
                  <span class="text-xs text-gray-500 uppercase">Created</span>
                  <p class="text-sm font-medium">{formatDateTime(selectedIssue.created)}</p>
                </div>
                <div>
                  <span class="text-xs text-gray-500 uppercase">Updated</span>
                  <p class="text-sm font-medium">{formatDateTime(selectedIssue.updated)}</p>
                </div>
              </div>

              <!-- Labels -->
              {#if selectedIssue.labels.length > 0}
                <div class="mb-4">
                  <span class="text-xs text-gray-500 uppercase">Labels</span>
                  <div class="flex flex-wrap gap-2 mt-1">
                    {#each selectedIssue.labels as label}
                      <span class="px-2 py-1 text-xs bg-blue-100 text-blue-800 rounded">{label}</span>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Components -->
              {#if selectedIssue.components.length > 0}
                <div class="mb-4">
                  <span class="text-xs text-gray-500 uppercase">Components</span>
                  <div class="flex flex-wrap gap-2 mt-1">
                    {#each selectedIssue.components as component}
                      <span class="px-2 py-1 text-xs bg-purple-100 text-purple-800 rounded">{component}</span>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Description -->
              <div class="mt-6 w-full">
                <h3 class="text-sm font-semibold text-gray-700 mb-2 uppercase">Description</h3>
                <div class="text-sm text-gray-700 p-4 bg-white rounded-lg border border-gray-200 w-full description-content">
                  {@html renderDescription(selectedIssue.description)}
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

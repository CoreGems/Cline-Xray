<script lang="ts">
  import { open } from "@tauri-apps/plugin-shell";
  import type { IssueDetails } from "../../types";
  import { getStatusClass } from "./utils";
  import JiraMetadataGrid from "./JiraMetadataGrid.svelte";
  import JiraLabels from "./JiraLabels.svelte";
  import JiraComponents from "./JiraComponents.svelte";
  import JiraDescription from "./JiraDescription.svelte";

  interface Props {
    issue: IssueDetails | null;
    loading?: boolean;
    error?: string | null;
    onRefresh?: () => void;
  }

  let { issue, loading = false, error = null, onRefresh }: Props = $props();
  
  // Details pane tab state
  type DetailTab = 'details';
  let activeDetailTab: DetailTab = $state('details');

  // Open URL in browser
  async function openInBrowser(url: string) {
    await open(url);
  }
</script>

<div class="flex-1 h-full flex flex-col bg-white min-w-0">
  {#if !issue}
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
      {#if loading}
        <div class="absolute inset-0 bg-white/70 flex items-center justify-center z-10">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
        </div>
      {/if}
      
      <!-- Fixed Header -->
      <div class="px-6 pt-6 pb-4 border-b border-gray-200 bg-white">
        <!-- Error Banner -->
        {#if error}
          <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm">
            <strong>Error loading details:</strong> {error}
          </div>
        {/if}
        
        <!-- Issue Header -->
        <div class="flex items-center gap-3 mb-2">
          <a href="{issue.key}" class="text-blue-600 font-semibold text-lg hover:underline">{issue.key}</a>
          <span class="px-3 py-1 text-sm rounded-full {getStatusClass(issue.statusCategory)}">
            {issue.status}
          </span>
          {#if issue.resolution}
            <span class="px-3 py-1 text-sm rounded-full bg-green-100 text-green-800">
              {issue.resolution}
            </span>
          {/if}
          <div class="ml-auto flex items-center gap-1">
            <button
              onclick={() => openInBrowser(`https://sonymusicpub.atlassian.net/browse/${issue.key}`)}
              class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors"
              title="Open in browser"
            >
              <svg 
                class="w-5 h-5" 
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
              </svg>
            </button>
            {#if onRefresh}
              <button
                onclick={onRefresh}
                disabled={loading}
                class="p-1.5 text-blue-600 hover:text-blue-800 hover:bg-blue-50 rounded-md transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                title="Refresh issue details"
              >
                <svg 
                  class="w-5 h-5 {loading ? 'animate-spin' : ''}" 
                  fill="none" 
                  stroke="currentColor" 
                  viewBox="0 0 24 24"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
                </svg>
              </button>
            {/if}
          </div>
        </div>
        <h1 class="text-2xl font-bold text-gray-900 mb-4">{issue.summary}</h1>
        
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
            <JiraMetadataGrid {issue} />

            <!-- Labels -->
            <JiraLabels labels={issue.labels} />

            <!-- Components -->
            <JiraComponents components={issue.components} />

            <!-- Description -->
            <JiraDescription description={issue.description} />
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

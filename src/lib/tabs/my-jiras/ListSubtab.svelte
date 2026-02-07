<script lang="ts">
  // List Subtab - Issue list with details panel
  import IssueListPane from "./IssueListPane.svelte";
  import { JiraDetailsPanel } from "../../../modules/jira-details";
  import { fetchIssueDetails } from "./api";
  import type { IssueDetails, IssueSummary } from "./types";

  // Selected issue state
  let selectedIssue: IssueDetails | null = $state(null);
  let detailsLoading = $state(false);
  let detailsError: string | null = $state(null);

  // Select issue handler - fetches full details from Jira API
  async function handleSelectIssue(issue: IssueSummary) {
    // Skip if already selected
    if (selectedIssue?.key === issue.key) return;
    
    await loadIssueDetails(issue.key, issue);
  }

  // Fetch issue details from API
  async function loadIssueDetails(key: string, basicInfo?: IssueSummary) {
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
      const details = await fetchIssueDetails(key);
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
      await loadIssueDetails(selectedIssue.key);
    }
  }
</script>

<div class="flex-1 flex overflow-hidden">
  <!-- Left Pane - Issue List -->
  <IssueListPane 
    {selectedIssue} 
    onSelectIssue={handleSelectIssue} 
  />

  <!-- Right Pane - Issue Details (using extracted module) -->
  <JiraDetailsPanel 
    issue={selectedIssue} 
    loading={detailsLoading} 
    error={detailsError} 
    onRefresh={handleRefreshDetails} 
  />
</div>

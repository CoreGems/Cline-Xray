<script lang="ts">
  import type { IssueDetails } from "../../types";
  import { formatDateTime } from "./utils";

  interface Props {
    issue: IssueDetails;
  }

  let { issue }: Props = $props();
</script>

<div class="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6 p-4 bg-white rounded-lg border border-gray-200">
  <div>
    <span class="text-xs text-gray-500 uppercase">Type</span>
    <p class="text-sm font-medium">{issue.issueType}</p>
  </div>
  <div>
    <span class="text-xs text-gray-500 uppercase">Priority</span>
    <p class="text-sm font-medium">{issue.priority}</p>
  </div>
  <div>
    <span class="text-xs text-gray-500 uppercase">Assignee</span>
    <p class="text-sm font-medium">{issue.assignee || "Unassigned"}</p>
  </div>
  <div>
    <span class="text-xs text-gray-500 uppercase">Reporter</span>
    <p class="text-sm font-medium">{issue.reporter || "Unknown"}</p>
  </div>
  <div>
    <span class="text-xs text-gray-500 uppercase">Created</span>
    <p class="text-sm font-medium">{formatDateTime(issue.created)}</p>
  </div>
  <div>
    <span class="text-xs text-gray-500 uppercase">Updated</span>
    <p class="text-sm font-medium">{formatDateTime(issue.updated)}</p>
  </div>
</div>

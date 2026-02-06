// My Jiras Tab API Functions

import { invoke } from "@tauri-apps/api/core";
import type { IssueDetails } from "./types";

/**
 * Fetch full issue details from Jira API
 * @param key - The Jira issue key (e.g., "PROJ-123")
 * @returns The full issue details
 */
export async function fetchIssueDetails(key: string): Promise<IssueDetails> {
  return await invoke<IssueDetails>("get_issue", { key });
}

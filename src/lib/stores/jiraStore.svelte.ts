/**
 * Jira Store - Handles all Jira issue listing state and logic
 * This module provides reactive state management for Jira issues
 */

import { invoke } from "@tauri-apps/api/core";
import type { IssueSummary, JiraSettings, SearchResult } from "../../types";

// ============ Store State ============

class JiraStore {
  // Issue list state
  issues = $state<IssueSummary[]>([]);
  listLoading = $state(false);
  listError = $state<string | null>(null);
  currentJql = $state("assignee = currentUser() ORDER BY updated DESC");
  
  // Configuration state
  isConfigured = $state(false);
  settings = $state<JiraSettings>({
    baseUrl: "",
    email: "",
    defaultJql: "assignee = currentUser() ORDER BY updated DESC",
  });
  
  // ============ Issue List Methods ============
  
  /**
   * Load issues from cache (for startup - no API call)
   */
  async loadCachedIssues(): Promise<boolean> {
    try {
      const result = await invoke<SearchResult | null>("get_cached_issues");
      if (result && result.issues.length > 0) {
        this.issues = result.issues;
        console.log(`Loaded ${result.issues.length} issues from cache`);
        return true;
      }
      return false;
    } catch (e) {
      console.warn("Failed to load cached issues:", e);
      return false;
    }
  }
  
  /**
   * Load issues from Jira API using the specified JQL query
   * This is called when user explicitly refreshes
   */
  async loadIssues(jql?: string): Promise<void> {
    const queryJql = jql || this.currentJql;
    
    this.listLoading = true;
    this.listError = null;
    
    try {
      const result = await invoke<SearchResult>("list_issues", { jql: queryJql });
      this.issues = result.issues;
      this.currentJql = queryJql;
      console.log(`Fetched ${result.issues.length} issues from Jira API`);
    } catch (e) {
      this.listError = `Failed to load issues: ${e}`;
      this.issues = [];
      throw e;
    } finally {
      this.listLoading = false;
    }
  }
  
  /**
   * Refresh the current issue list (calls Jira API)
   */
  async refresh(): Promise<void> {
    await this.loadIssues(this.currentJql);
  }
  
  /**
   * Clear all issues
   */
  clearIssues(): void {
    this.issues = [];
    this.listError = null;
  }
  
  // ============ Configuration Methods ============
  
  /**
   * Check if the app is configured with Jira credentials
   */
  async checkConfiguration(): Promise<boolean> {
    try {
      this.isConfigured = await invoke<boolean>("is_configured");
      
      if (this.isConfigured) {
        this.settings = await invoke<JiraSettings>("get_settings");
        this.currentJql = this.settings.defaultJql || this.currentJql;
      }
      
      return this.isConfigured;
    } catch (e) {
      this.listError = `Configuration check failed: ${e}`;
      return false;
    }
  }
  
  /**
   * Save new settings and API token
   */
  async saveSettings(newSettings: JiraSettings, apiToken: string): Promise<void> {
    try {
      await invoke("save_settings", { settings: newSettings, apiToken });
      this.settings = newSettings;
      this.isConfigured = true;
      this.currentJql = newSettings.defaultJql || this.currentJql;
    } catch (e) {
      throw new Error(`Failed to save settings: ${e}`);
    }
  }
  
  /**
   * Initialize the store - check configuration and load cached issues
   * Does NOT call Jira API - only loads from cache for fast startup
   */
  async initialize(): Promise<void> {
    const configured = await this.checkConfiguration();
    if (configured) {
      // Load from cache only - don't call API on startup
      const hasCached = await this.loadCachedIssues();
      if (!hasCached) {
        console.log("No cached issues found. Click refresh to fetch from Jira.");
      }
    }
  }
  
  // ============ Computed Properties ============
  
  /**
   * Get the count of issues
   */
  get issueCount(): number {
    return this.issues.length;
  }
  
  /**
   * Check if there are any issues
   */
  get hasIssues(): boolean {
    return this.issues.length > 0;
  }
  
  /**
   * Check if currently loading
   */
  get isLoading(): boolean {
    return this.listLoading;
  }
}

// Export a singleton instance
export const jiraStore = new JiraStore();

// Also export the type for external use
export type { JiraStore };

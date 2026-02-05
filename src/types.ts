// Issue summary for list view
export interface IssueSummary {
  key: string;
  summary: string;
  status: string;
  statusCategory: string;
  assignee: string | null;
  priority: string;
  updated: string;
}

// Full issue details for detail view
export interface IssueDetails {
  key: string;
  summary: string;
  description: string | null;
  status: string;
  statusCategory: string;
  resolution: string | null;
  issueType: string;
  priority: string;
  assignee: string | null;
  reporter: string | null;
  created: string;
  updated: string;
  labels: string[];
  components: string[];
}

// Configuration settings
export interface JiraSettings {
  baseUrl: string;
  email: string;
  defaultJql: string;
}

// Search result wrapper
export interface SearchResult {
  issues: IssueSummary[];
  total: number;
}

// Access log entry for REST API activity tracking
export interface AccessLogEntry {
  id: number;
  timestamp: string;
  method: string;
  path: string;
  statusCode: number;
  durationMs: number;
  clientIp: string;
}

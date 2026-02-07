// Issue summary for list view
export interface IssueSummary {
  key: string;
  summary: string;
  status: string;
  statusCategory: string;
  assignee: string | null;
  priority: string;
  issueType: string;
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

// Inference log entry for AI model activity tracking
export interface InferenceLogEntry {
  id: number;
  timestamp: string;
  provider: string;           // e.g., "gemini", "openai"
  model: string;              // e.g., "gemini-2.0-flash"
  requestType: string;        // e.g., "chat", "completion"
  success: boolean;
  statusCode: number | null;
  durationMs: number;
  promptTokens: number | null;
  completionTokens: number | null;
  totalTokens: number | null;
  errorMessage: string | null;
  systemPrompt: string | null;
  userMessagePreview: string | null;
  metadata: Record<string, unknown> | null;
}

// Agent settings configuration
export interface AgentSettings {
  /** Required generation methods for filtering models (e.g., ["generateContent"]) */
  requiredMethods: string[];
  /** Whether to only show text generation models */
  filterTextGenerationOnly: boolean;
  /** Keywords to exclude models by name/description (case-insensitive) */
  excludeKeywords: string[];
}

// Default agent settings - exclude common image/video/audio/embedding model keywords
export const DEFAULT_AGENT_SETTINGS: AgentSettings = {
  requiredMethods: ["generateContent"],
  filterTextGenerationOnly: true,
  excludeKeywords: [
    "imagen",      // Image generation models
    "veo",         // Video generation models
    "banana",      // Nano Banana (image preview)
    "audio",       // Audio models
    "embedding",   // Embedding models (not for generation)
    "robotics",    // Robotics models
    "aqa",         // Attributed Question Answering
    "image preview", // Image preview models
  ]
};

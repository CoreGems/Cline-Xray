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

// Per-vendor filter settings
export interface VendorFilterSettings {
  /** Whether filtering is enabled for this vendor */
  filterEnabled: boolean;
  /** Required generation methods for filtering models (e.g., ["generateContent"]) */
  requiredMethods: string[];
  /** Keywords to exclude models by name/description (case-insensitive) */
  excludeKeywords: string[];
}

// Agent settings configuration (per-vendor keyed)
export interface AgentSettings {
  /** Per-vendor filter settings, keyed by vendor ID (e.g., "gemini", "openai") */
  vendors: Record<string, VendorFilterSettings>;

  // --- Legacy fields (for backward-compat migration) ---
  /** @deprecated Use vendors[vendorId].requiredMethods */
  requiredMethods?: string[];
  /** @deprecated Use vendors[vendorId].filterEnabled */
  filterTextGenerationOnly?: boolean;
  /** @deprecated Use vendors[vendorId].excludeKeywords */
  excludeKeywords?: string[];
}

/** Helper: get vendor filter settings with fallback to defaults */
export function getVendorSettings(
  settings: AgentSettings,
  vendorId: string,
): VendorFilterSettings {
  if (settings.vendors?.[vendorId]) {
    return settings.vendors[vendorId];
  }
  // Fallback: check if legacy flat fields exist (pre-migration data for gemini)
  if (vendorId === 'gemini' && settings.excludeKeywords) {
    return {
      filterEnabled: settings.filterTextGenerationOnly ?? true,
      requiredMethods: settings.requiredMethods ?? ["generateContent"],
      excludeKeywords: settings.excludeKeywords ?? [],
    };
  }
  return DEFAULT_VENDOR_FILTER_SETTINGS;
}

// Default per-vendor filter settings (generic baseline)
export const DEFAULT_VENDOR_FILTER_SETTINGS: VendorFilterSettings = {
  filterEnabled: false,
  requiredMethods: [],
  excludeKeywords: [],
};

// Default OpenAI-specific filter settings
export const DEFAULT_OPENAI_FILTER_SETTINGS: VendorFilterSettings = {
  filterEnabled: true,
  requiredMethods: [],
  excludeKeywords: [
    'instruct',        // Instruct-tuned variants (not for chat)
    'realtime',        // Realtime API models
    'audio',           // Audio models
    'tts',             // Text-to-speech models
    'whisper',         // Speech-to-text models
    'dall-e',          // Image generation models
    'embedding',       // Embedding models
    'babbage',         // Legacy models
    'davinci',         // Legacy models
    'search',          // Search models
    'similarity',      // Similarity models
    'code-',           // Legacy code models
    'text-',           // Legacy text models
  ],
};

// Default Gemini-specific filter settings
export const DEFAULT_GEMINI_FILTER_SETTINGS: VendorFilterSettings = {
  filterEnabled: true,
  requiredMethods: ["generateContent"],
  excludeKeywords: [
    "imagen",        // Image generation models
    "veo",           // Video generation models
    "banana",        // Nano Banana (image preview)
    "audio",         // Audio models
    "embedding",     // Embedding models (not for generation)
    "robotics",      // Robotics models
    "aqa",           // Attributed Question Answering
    "image preview", // Image preview models
  ]
};

// Default agent settings (ships with Gemini + OpenAI defaults)
export const DEFAULT_AGENT_SETTINGS: AgentSettings = {
  vendors: {
    gemini: { ...DEFAULT_GEMINI_FILTER_SETTINGS },
    openai: { ...DEFAULT_OPENAI_FILTER_SETTINGS },
  },
};

// History Tab Types
// Matches the Rust types in src-tauri/src/conversation_history/types.rs

/** Summary of a single Cline task conversation history */
export interface TaskHistorySummary {
  /** Task ID (directory name, epoch milliseconds) */
  taskId: string;
  /** ISO 8601 timestamp derived from task_id (task start time) */
  startedAt: string;
  /** ISO 8601 timestamp of the last UI message (task end time) */
  endedAt: string | null;
  /** Total number of API messages (user + assistant turns) */
  messageCount: number;
  /** Number of tool_use blocks across all assistant messages */
  toolUseCount: number;
  /** Number of thinking blocks */
  thinkingCount: number;
  /** Tool usage breakdown: tool_name → count */
  toolBreakdown: Record<string, number>;
  /** Model ID used (from task_metadata or first ui_message) */
  modelId: string | null;
  /** Model provider (e.g. "anthropic") */
  modelProvider: string | null;
  /** Number of files in context (from task_metadata) */
  filesInContext: number;
  /** Files edited by Cline */
  filesEdited: number;
  /** Files read by Cline */
  filesRead: number;
  /** Cline version that created this task */
  clineVersion: string | null;
  /** Size of api_conversation_history.json in bytes */
  apiHistorySizeBytes: number;
  /** Size of ui_messages.json in bytes */
  uiMessagesSizeBytes: number;
  /** Whether a focus_chain markdown file exists */
  hasFocusChain: boolean;
  /** First user message text (truncated to 200 chars) — task description */
  taskPrompt: string | null;
}

/** Response from GET /history/tasks */
export interface TaskHistoryListResponse {
  /** List of task summaries (newest first) */
  tasks: TaskHistorySummary[];
  /** Total number of task directories found */
  totalTasks: number;
  /** Total size of all api_conversation_history.json files (bytes) */
  totalApiHistoryBytes: number;
  /** Root path that was scanned */
  tasksRoot: string;
  /** Aggregate tool usage across all tasks */
  aggregateToolBreakdown: Record<string, number>;
  /** Total tool calls across all tasks */
  totalToolCalls: number;
  /** Total messages across all tasks */
  totalMessages: number;
}

// ============================================================================
// Task Detail types (P1: single-task deep-dive)
// ============================================================================

/** Full detail for a single Cline task */
export interface TaskDetailResponse {
  taskId: string;
  startedAt: string;
  endedAt: string | null;
  messageCount: number;
  toolUseCount: number;
  thinkingCount: number;
  taskPrompt: string | null;
  messages: ConversationMessage[];
  toolCalls: ToolCallDetail[];
  toolBreakdown: Record<string, number>;
  files: FileInContextDetail[];
  filesInContextCount: number;
  filesEditedCount: number;
  filesReadCount: number;
  modelUsage: ModelUsageDetail[];
  environment: EnvironmentDetail[];
  focusChain: string | null;
  hasFocusChain: boolean;
  apiHistorySizeBytes: number;
  uiMessagesSizeBytes: number;
  /** Full local filesystem path to the task directory */
  taskDirPath: string;
}

/** A single conversation message */
export interface ConversationMessage {
  index: number;
  role: string;
  timestamp: string | null;
  content: ContentBlockSummary[];
}

/** A content block inside a message */
export interface ContentBlockSummary {
  type: string;
  text: string | null;
  fullTextLength: number | null;
  toolUseId: string | null;
  toolName: string | null;
  toolInput: string | null;
  toolResultText: string | null;
}

/** A tool call with result */
export interface ToolCallDetail {
  callIndex: number;
  messageIndex: number;
  toolName: string;
  toolUseId: string;
  inputSummary: string;
  inputFullLength: number;
  resultSummary: string | null;
  resultFullLength: number | null;
}

/** A file tracked in context */
export interface FileInContextDetail {
  path: string;
  recordState: string | null;
  recordSource: string | null;
  clineReadDate: string | null;
  clineEditDate: string | null;
  userEditDate: string | null;
}

/** Model usage entry */
export interface ModelUsageDetail {
  timestamp: string | null;
  modelId: string | null;
  modelProviderId: string | null;
  mode: string | null;
}

/** Environment snapshot */
export interface EnvironmentDetail {
  timestamp: string | null;
  osName: string | null;
  osVersion: string | null;
  hostName: string | null;
  hostVersion: string | null;
  clineVersion: string | null;
}

/** API connection info */
export interface ApiInfo {
  base_url: string;
  token: string;
}

/** Available subtabs in the History tab */
/** Full single message with untruncated content */
export interface FullMessageResponse {
  taskId: string;
  index: number;
  totalMessages: number;
  role: string;
  timestamp: string | null;
  content: FullContentBlock[];
}

/** A content block with full untruncated content */
export interface FullContentBlock {
  type: string;
  text: string | null;
  textLength: number | null;
  toolUseId: string | null;
  toolName: string | null;
  toolInput: string | null;
  toolInputLength: number | null;
  toolResultText: string | null;
  toolResultLength: number | null;
}

/** Response for GET /history/tasks/:taskId/messages — paginated message list */
export interface PaginatedMessagesResponse {
  taskId: string;
  totalMessages: number;
  filteredCount: number;
  offset: number;
  limit: number;
  hasMore: boolean;
  messages: ConversationMessage[];
}

/** Tool call timeline entry with success/fail status */
export interface ToolCallTimelineEntry {
  callIndex: number;
  messageIndex: number;
  resultMessageIndex: number | null;
  timestamp: string | null;
  toolName: string;
  toolUseId: string;
  inputSummary: string;
  inputFullLength: number;
  resultSummary: string | null;
  resultFullLength: number | null;
  success: boolean | null;
  errorText: string | null;
}

/** Response for GET /history/tasks/:taskId/tools */
export interface ToolCallTimelineResponse {
  taskId: string;
  totalToolCalls: number;
  filteredCount: number;
  successCount: number;
  failureCount: number;
  noResultCount: number;
  toolBreakdown: Record<string, number>;
  toolCalls: ToolCallTimelineEntry[];
}

/** Thinking block entry */
export interface ThinkingBlockEntry {
  blockIndex: number;
  messageIndex: number;
  timestamp: string | null;
  thinking: string;
  fullLength: number;
  isTruncated: boolean;
}

/** Response for GET /history/tasks/:taskId/thinking */
export interface ThinkingBlocksResponse {
  taskId: string;
  totalThinkingBlocks: number;
  totalCharacters: number;
  avgBlockLength: number;
  thinkingBlocks: ThinkingBlockEntry[];
}

/** Response for GET /history/tasks/:taskId/files — files-in-context audit trail */
export interface TaskFilesResponse {
  taskId: string;
  totalFiles: number;
  filesEditedCount: number;
  filesReadCount: number;
  filesMentionedCount: number;
  filesUserEditedCount: number;
  files: FileInContextDetail[];
}

/** Aggregate statistics across all task histories (GET /history/stats) */
export interface HistoryStatsResponse {
  totalTasks: number;
  totalMessages: number;
  totalToolCalls: number;
  totalThinkingBlocks: number;
  totalApiHistoryBytes: number;
  totalUiMessagesBytes: number;
  avgTaskSizeBytes: number;
  minTaskSizeBytes: number;
  maxTaskSizeBytes: number;
  avgMessagesPerTask: number;
  avgToolCallsPerTask: number;
  avgThinkingBlocksPerTask: number;
  avgFilesInContext: number;
  toolBreakdown: Record<string, number>;
  toolPercentages: Record<string, number>;
  modelUsage: Record<string, number>;
  modelProviderUsage: Record<string, number>;
  clineVersionUsage: Record<string, number>;
  totalFilesInContext: number;
  totalFilesEdited: number;
  totalFilesRead: number;
  tasksWithFocusChain: number;
  earliestTask: string | null;
  latestTask: string | null;
  tasksRoot: string;
}

/** A detected subtask within a task conversation */
export interface SubtaskEntry {
  subtaskIndex: number;
  prompt: string;
  timestamp: string;
  isInitialTask: boolean;
  messageRangeStart: number;
  messageRangeEnd: number | null;
  messageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
}

/** Response for GET /history/tasks/:taskId/subtasks */
export interface SubtasksResponse {
  taskId: string;
  totalSubtasks: number;
  hasSubtasks: boolean;
  subtasks: SubtaskEntry[];
}

export type HistorySubTab = 'Tasks' | 'Stats';

/** Subtab definition */
export interface SubTabDefinition {
  id: HistorySubTab;
  label: string;
}

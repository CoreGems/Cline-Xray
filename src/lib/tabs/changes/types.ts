// Changes Tab Types

/** A discovered checkpoint workspace */
export interface WorkspaceInfo {
  /** Workspace-id directory name (e.g. "4184916832") */
  id: string;
  /** Absolute path to the .git or .git_disabled directory */
  gitDir: string;
  /** Whether the git dir is active (.git) or paused (.git_disabled) */
  active: boolean;
  /** Number of distinct tasks in this workspace */
  taskCount: number;
  /** ISO 8601 timestamp of the most recent checkpoint commit */
  lastModified: string;
}

/** Response from GET /changes/workspaces */
export interface WorkspacesResponse {
  /** List of discovered checkpoint workspaces */
  workspaces: WorkspaceInfo[];
  /** The root path that was scanned */
  checkpointsRoot: string;
}

/** A task summary (group of checkpoint commits) */
export interface ClineTaskSummary {
  /** Task ID extracted from commit subjects */
  taskId: string;
  /** Workspace ID this task belongs to */
  workspaceId: string;
  /** Number of checkpoint commits (steps) */
  steps: number;
  /** Number of distinct files changed */
  filesChanged: number;
  /** ISO 8601 timestamp of the most recent step */
  lastModified: string;
}

/** Response from GET /changes/tasks */
export interface TasksResponse {
  /** Workspace ID */
  workspaceId: string;
  /** List of tasks */
  tasks: ClineTaskSummary[];
}

/** A single checkpoint step (one commit in a task) */
export interface CheckpointStep {
  /** 40-char commit SHA */
  hash: string;
  /** Commit subject line */
  subject: string;
  /** ISO 8601 timestamp */
  timestamp: string;
  /** Files changed in this step vs parent */
  filesChanged: number;
  /** 1-based step index (chronological order) */
  index: number;
}

/** Response from GET /changes/tasks/:taskId/steps */
export interface StepsResponse {
  /** Task ID */
  taskId: string;
  /** Workspace ID */
  workspaceId: string;
  /** Ordered list of steps (oldest first) */
  steps: CheckpointStep[];
}

/** A file in a diff */
export interface DiffFile {
  /** File path relative to repo root */
  path: string;
  /** Lines added */
  linesAdded: number;
  /** Lines removed */
  linesRemoved: number;
  /** File status: added | modified | deleted | renamed */
  status: string;
}

/** Full diff result */
export interface DiffResult {
  /** List of files changed */
  files: DiffFile[];
  /** Unified diff patch text */
  patch: string;
  /** The "from" commit reference */
  fromRef: string;
  /** The "to" commit reference */
  toRef: string;
  /** The actual git commands that were executed to produce this diff */
  gitCommands?: string[];
}

/** API connection info (reused from agent) */
export interface ApiInfo {
  base_url: string;
  token: string;
}

/** Summary of a subtask (no diff — loaded on demand) */
export interface SubtaskSummaryItem {
  /** Subtask index (0 = initial, 1+ = feedback) */
  subtaskIndex: number;
  /** Whether this is the initial task prompt */
  isInitialTask: boolean;
  /** The prompt text */
  prompt: string;
  /** ISO 8601 timestamp */
  timestamp: string;
  /** Number of API messages */
  messageCount: number;
  /** Number of tool calls */
  toolCallCount: number;
  /** Tool names used */
  toolsUsed: string[];
}

/** Response from GET /latest */
export interface LatestResponse {
  /** Task ID */
  taskId: string;
  /** Subtask index (0 = initial, 1+ = feedback). Null if scope=task */
  subtaskIndex: number | null;
  /** Whether this is the initial task prompt */
  isInitialTask: boolean | null;
  /** Total subtasks in this task */
  totalSubtasks: number;
  /** The prompt text */
  prompt: string;
  /** ISO 8601 timestamp of the prompt */
  promptTimestamp: string;
  /** Diff result (files + patch). Null if no checkpoint data */
  diff: DiffResult | null;
  /** Reason why diff is null */
  noDiffReason: string | null;
  /** First message index in api_conversation_history */
  messageRangeStart: number | null;
  /** Last message index (inclusive) */
  messageRangeEnd: number | null;
  /** Number of API messages */
  messageCount: number;
  /** Number of tool calls */
  toolCallCount: number;
  /** Tool names used (deduplicated) */
  toolsUsed: string[];
  /** Workspace ID */
  workspaceId: string | null;
  /** ISO 8601 task start time */
  taskStartedAt: string;
  /** ISO 8601 task end time */
  taskEndedAt: string | null;
  /** Scope used: "subtask" or "task" */
  scope: string;
  /** All subtasks in this task (metadata only, no diffs — loaded on demand) */
  subtasks: SubtaskSummaryItem[];
}

/** Response from POST /changes/workspaces/:id/nuke */
export interface NukeWorkspaceResponse {
  /** Workspace ID that was nuked */
  workspaceId: string;
  /** Number of commits that were deleted */
  deletedCommits: number;
  /** Number of tasks that were deleted */
  deletedTasks: number;
  /** The git command used to re-initialize the repo */
  gitCommand: string;
  /** Whether the operation was successful */
  success: boolean;
}

/** A file with its content retrieved from shadow git */
export interface FileContentItem {
  /** File path relative to repo root */
  path: string;
  /** File content (null if not available at given ref) */
  content: string | null;
  /** Error message if retrieval failed */
  error: string | null;
  /** Size in bytes */
  size: number | null;
}

/** Response from POST /changes/file-contents */
export interface FileContentsResponse {
  /** Files with their contents */
  files: FileContentItem[];
  /** Number of files successfully retrieved */
  retrieved: number;
  /** Number of files that failed */
  failed: number;
  /** Total content size in bytes */
  totalSize: number;
}

/** Response from GET /changes/ignore */
export interface ChangesIgnoreResponse {
  /** Parsed exclude patterns (comments and blanks stripped) */
  patterns: string[];
  /** Raw file content (for UI editing) */
  rawContent: string;
  /** Source: "file", "defaults (no file)", etc. */
  source: string;
  /** Absolute path to the .changesignore file */
  filePath: string;
}

/** Available subtabs in the Changes tab */
export type ChangesSubTab = 'Tasks' | 'Diff' | 'Latest' | 'Export';

/** Subtab definition */
export interface SubTabDefinition {
  id: ChangesSubTab;
  label: string;
}

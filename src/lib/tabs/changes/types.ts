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
}

/** API connection info (reused from agent) */
export interface ApiInfo {
  base_url: string;
  token: string;
}

/** Available subtabs in the Changes tab */
export type ChangesSubTab = 'Tasks' | 'Diff' | 'Export';

/** Subtab definition */
export interface SubTabDefinition {
  id: ChangesSubTab;
  label: string;
}

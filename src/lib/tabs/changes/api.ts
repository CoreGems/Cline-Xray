// Changes Tab API Functions

import { invoke } from "@tauri-apps/api/core";
import type { ApiInfo, WorkspacesResponse, TasksResponse, StepsResponse, DiffResult, LatestResponse } from "./types";

/**
 * Get API connection info from the Tauri backend
 */
async function getApiInfo(): Promise<ApiInfo> {
  return await invoke('get_api_info');
}

/**
 * Fetch discovered checkpoint workspaces from the REST API
 * GET /changes/workspaces
 * @param refresh - if true, forces re-discovery (bypasses server cache)
 */
export async function fetchWorkspaces(refresh: boolean = false): Promise<WorkspacesResponse> {
  const apiInfo = await getApiInfo();
  const url = refresh
    ? `${apiInfo.base_url}/changes/workspaces?refresh=true`
    : `${apiInfo.base_url}/changes/workspaces`;

  const response = await fetch(url, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    }
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Fetch steps (checkpoints) for a specific task from the REST API
 * GET /changes/tasks/:taskId/steps?workspace=<id>
 * @param taskId - the task ID to list steps for
 * @param workspaceId - the workspace ID (required to locate the git repo)
 */
export async function fetchSteps(taskId: string, workspaceId: string): Promise<StepsResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams({ workspace: workspaceId });

  const response = await fetch(`${apiInfo.base_url}/changes/tasks/${taskId}/steps?${params}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    }
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Fetch the diff for a single checkpoint step
 * GET /changes/tasks/:taskId/steps/:index/diff?workspace=<id>
 * @param taskId - the task ID
 * @param stepIndex - 1-based step index (chronological)
 * @param workspaceId - the workspace ID
 */
export async function fetchStepDiff(taskId: string, stepIndex: number, workspaceId: string): Promise<DiffResult> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams({ workspace: workspaceId });

  const response = await fetch(
    `${apiInfo.base_url}/changes/tasks/${taskId}/steps/${stepIndex}/diff?${params}`,
    {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiInfo.token}`
      }
    }
  );

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Fetch the full task diff (base→HEAD) for an entire task
 * GET /changes/tasks/:taskId/diff?workspace=<id>&exclude=...
 * @param taskId - the task ID
 * @param workspaceId - the workspace ID
 * @param excludes - optional pathspec exclusion patterns
 */
export async function fetchTaskDiff(
  taskId: string,
  workspaceId: string,
  excludes: string[] = []
): Promise<DiffResult> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams({ workspace: workspaceId });
  for (const ex of excludes) {
    params.append('exclude', ex);
  }

  const response = await fetch(
    `${apiInfo.base_url}/changes/tasks/${taskId}/diff?${params}`,
    {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiInfo.token}`
      }
    }
  );

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Fetch the subtask diff for a specific subtask phase within a task
 * GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff?workspace=<id>&exclude=...
 * @param taskId - the task ID
 * @param subtaskIndex - 0-based subtask index (0=initial task, 1+=feedback)
 * @param workspaceId - the workspace ID
 * @param excludes - optional pathspec exclusion patterns
 */
export async function fetchSubtaskDiff(
  taskId: string,
  subtaskIndex: number,
  workspaceId: string,
  excludes: string[] = []
): Promise<DiffResult> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams({ workspace: workspaceId });
  for (const ex of excludes) {
    params.append('exclude', ex);
  }

  const response = await fetch(
    `${apiInfo.base_url}/changes/tasks/${taskId}/subtasks/${subtaskIndex}/diff?${params}`,
    {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiInfo.token}`
      }
    }
  );

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Fetch the latest task/subtask prompt + diff in a single call
 * GET /latest?scope=<scope>&exclude=...
 * @param scope - "subtask" (default) or "task" (full task diff)
 * @param excludes - optional pathspec exclusion patterns
 */
export async function fetchLatest(
  scope: string = 'subtask',
  excludes: string[] = []
): Promise<LatestResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (scope !== 'subtask') params.set('scope', scope);
  for (const ex of excludes) {
    params.append('exclude', ex);
  }
  const qs = params.toString();
  const url = qs ? `${apiInfo.base_url}/latest?${qs}` : `${apiInfo.base_url}/latest`;

  const response = await fetch(url, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    }
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

/**
 * Fetch tasks for a specific workspace from the REST API
 * GET /changes/tasks?workspace=<id>
 * @param workspaceId - the workspace ID to list tasks for
 * @param refresh - if true, forces re-enumeration (bypasses server cache)
 */
export async function fetchTasks(workspaceId: string, refresh: boolean = false): Promise<TasksResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams({ workspace: workspaceId });
  if (refresh) params.set('refresh', 'true');

  const response = await fetch(`${apiInfo.base_url}/changes/tasks?${params}`, {
    method: 'GET',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiInfo.token}`
    }
  });

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
    throw new Error(errorData.error || `HTTP error ${response.status}`);
  }

  return await response.json();
}

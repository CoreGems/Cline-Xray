// History Tab API Functions

import { invoke } from "@tauri-apps/api/core";
import type { ApiInfo, FullMessageResponse, HistoryStatsResponse, TaskDetailResponse, TaskHistoryListResponse } from "./types";

/**
 * Get API connection info from the Tauri backend
 */
async function getApiInfo(): Promise<ApiInfo> {
  return await invoke('get_api_info');
}

/**
 * Fetch conversation history task list from the REST API
 * GET /history/tasks
 * @param refresh - if true, forces re-scan from disk (bypass cache)
 * @param model - optional filter by model name substring
 * @param limit - optional limit on results
 * @param offset - optional offset for pagination
 */
export async function fetchHistoryTasks(
  refresh: boolean = false,
  model?: string,
  limit?: number,
  offset?: number
): Promise<TaskHistoryListResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (refresh) params.set('refresh', 'true');
  if (model) params.set('model', model);
  if (limit !== undefined) params.set('limit', String(limit));
  if (offset !== undefined) params.set('offset', String(offset));

  const qs = params.toString();
  const url = qs ? `${apiInfo.base_url}/history/tasks?${qs}` : `${apiInfo.base_url}/history/tasks`;

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
 * Fetch thinking blocks timeline for a specific task
 * GET /history/tasks/:taskId/thinking?max_length=&min_length=
 */
export async function fetchTaskThinking(
  taskId: string,
  maxLength?: number,
  minLength?: number
): Promise<import('./types').ThinkingBlocksResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (maxLength !== undefined) params.set('max_length', String(maxLength));
  if (minLength !== undefined) params.set('min_length', String(minLength));
  const qs = params.toString();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}/thinking${qs ? '?' + qs : ''}`;

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
 * Fetch tool call timeline for a specific task
 * GET /history/tasks/:taskId/tools?tool_name=&failed_only=
 */
export async function fetchTaskTools(
  taskId: string,
  toolName?: string,
  failedOnly?: boolean
): Promise<import('./types').ToolCallTimelineResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (toolName) params.set('tool_name', toolName);
  if (failedOnly) params.set('failed_only', 'true');
  const qs = params.toString();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}/tools${qs ? '?' + qs : ''}`;

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
 * Fetch a single message with full untruncated content
 * GET /history/tasks/:taskId/messages/:index
 */
export async function fetchSingleMessage(taskId: string, index: number): Promise<FullMessageResponse> {
  const apiInfo = await getApiInfo();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}/messages/${index}`;

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
 * Fetch paginated messages for a specific task
 * GET /history/tasks/:taskId/messages?offset=&limit=&role=
 */
export async function fetchTaskMessages(
  taskId: string,
  offset: number = 0,
  limit: number = 20,
  role?: string
): Promise<import('./types').PaginatedMessagesResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (offset > 0) params.set('offset', String(offset));
  if (limit !== 20) params.set('limit', String(limit));
  if (role) params.set('role', role);
  const qs = params.toString();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}/messages${qs ? '?' + qs : ''}`;

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
 * Fetch files-in-context audit trail for a specific task
 * GET /history/tasks/:taskId/files?source=&state=
 */
export async function fetchTaskFiles(
  taskId: string,
  source?: string,
  state?: string
): Promise<import('./types').TaskFilesResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (source) params.set('source', source);
  if (state) params.set('state', state);
  const qs = params.toString();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}/files${qs ? '?' + qs : ''}`;

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
 * Fetch aggregate stats across all task histories
 * GET /history/stats
 * @param refresh - if true, forces re-scan from disk (bypass cache)
 */
export async function fetchHistoryStats(refresh: boolean = false): Promise<HistoryStatsResponse> {
  const apiInfo = await getApiInfo();
  const params = new URLSearchParams();
  if (refresh) params.set('refresh', 'true');
  const qs = params.toString();
  const url = qs ? `${apiInfo.base_url}/history/stats?${qs}` : `${apiInfo.base_url}/history/stats`;

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
 * Fetch subtask detection timeline for a specific task
 * GET /history/tasks/:taskId/subtasks
 */
export async function fetchTaskSubtasks(taskId: string): Promise<import('./types').SubtasksResponse> {
  const apiInfo = await getApiInfo();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}/subtasks`;

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
 * Fetch full detail for a single task
 * GET /history/tasks/:taskId
 */
export async function fetchTaskDetail(taskId: string): Promise<TaskDetailResponse> {
  const apiInfo = await getApiInfo();
  const url = `${apiInfo.base_url}/history/tasks/${taskId}`;

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

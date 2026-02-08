<script lang="ts">
  import { onMount } from "svelte";
  import { fetchWorkspaces, fetchTasks, fetchSteps, fetchStepDiff, fetchTaskDiff, fetchSubtaskDiff } from "./api";
  import { fetchTaskSubtasks } from "../history/api";
  import type { SubtasksResponse } from "../history/types";
  import type { WorkspaceInfo, WorkspacesResponse, ClineTaskSummary, TasksResponse, CheckpointStep, DiffResult } from "./types";

  // ---- Workspace state ----
  let wsLoading = $state(true);
  let wsError: string | null = $state(null);
  let workspaces: WorkspaceInfo[] = $state([]);
  let checkpointsRoot: string = $state('');

  // ---- Task state ----
  let selectedWorkspace: WorkspaceInfo | null = $state(null);
  let taskLoading = $state(false);
  let taskError: string | null = $state(null);
  let tasks: ClineTaskSummary[] = $state([]);

  // ---- Steps state ----
  let expandedTaskId: string | null = $state(null);
  let stepsLoading = $state(false);
  let stepsError: string | null = $state(null);
  let steps: CheckpointStep[] = $state([]);

  // ---- Step Diff state ----
  let diffStepIndex: number | null = $state(null);
  let diffLoading = $state(false);
  let diffError: string | null = $state(null);
  let diffResult: DiffResult | null = $state(null);
  let copyLabel = $state('📋 Copy');

  // ---- Task Diff state ----
  let taskDiffId: string | null = $state(null);
  let taskDiffLoading = $state(false);
  let taskDiffError: string | null = $state(null);
  let taskDiffResult: DiffResult | null = $state(null);
  let taskDiffCopyLabel = $state('📋 Copy');

  // ---- Subtask Diff state ----
  let subtaskTaskId: string | null = $state(null);
  let subtasksData: SubtasksResponse | null = $state(null);
  let subtasksLoading = $state(false);
  let subtasksError: string | null = $state(null);
  let subtaskDiffIndex: number | null = $state(null);
  let subtaskDiffLoading = $state(false);
  let subtaskDiffError: string | null = $state(null);
  let subtaskDiffResult: DiffResult | null = $state(null);
  let subtaskDiffCopyLabel = $state('📋 Copy');

  onMount(() => {
    loadWorkspaces(false);
  });

  async function loadWorkspaces(refresh: boolean) {
    wsLoading = true;
    wsError = null;
    try {
      const resp: WorkspacesResponse = await fetchWorkspaces(refresh);
      workspaces = resp.workspaces;
      checkpointsRoot = resp.checkpointsRoot;
    } catch (e: any) {
      wsError = e.message || String(e);
    } finally {
      wsLoading = false;
    }
  }

  async function selectWorkspace(ws: WorkspaceInfo) {
    selectedWorkspace = ws;
    taskLoading = true;
    taskError = null;
    tasks = [];
    try {
      const resp: TasksResponse = await fetchTasks(ws.id, false);
      tasks = resp.tasks;
    } catch (e: any) {
      taskError = e.message || String(e);
    } finally {
      taskLoading = false;
    }
  }

  async function refreshTasks() {
    if (!selectedWorkspace) return;
    taskLoading = true;
    taskError = null;
    try {
      const resp: TasksResponse = await fetchTasks(selectedWorkspace.id, true);
      tasks = resp.tasks;
    } catch (e: any) {
      taskError = e.message || String(e);
    } finally {
      taskLoading = false;
    }
  }

  function backToWorkspaces() {
    selectedWorkspace = null;
    tasks = [];
    taskError = null;
    expandedTaskId = null;
    steps = [];
  }

  async function toggleSteps(task: ClineTaskSummary) {
    if (expandedTaskId === task.taskId) {
      // Collapse
      expandedTaskId = null;
      steps = [];
      stepsError = null;
      return;
    }
    // Expand
    expandedTaskId = task.taskId;
    stepsLoading = true;
    stepsError = null;
    steps = [];
    try {
      const resp = await fetchSteps(task.taskId, task.workspaceId);
      steps = resp.steps.slice().reverse(); // latest step on top
    } catch (e: any) {
      stepsError = e.message || String(e);
    } finally {
      stepsLoading = false;
    }
  }

  async function loadTaskDiff(task: ClineTaskSummary, e: MouseEvent) {
    e.stopPropagation(); // Don't toggle steps expansion
    if (!selectedWorkspace) return;

    if (taskDiffId === task.taskId) {
      // Collapse
      taskDiffId = null;
      taskDiffResult = null;
      taskDiffError = null;
      return;
    }

    taskDiffId = task.taskId;
    taskDiffLoading = true;
    taskDiffError = null;
    taskDiffResult = null;
    taskDiffCopyLabel = '📋 Copy';
    try {
      taskDiffResult = await fetchTaskDiff(task.taskId, selectedWorkspace.id);
    } catch (e: any) {
      taskDiffError = e.message || String(e);
    } finally {
      taskDiffLoading = false;
    }
  }

  async function loadStepDiff(step: CheckpointStep, e: MouseEvent) {
    e.stopPropagation(); // Don't toggle task expansion
    if (!expandedTaskId || !selectedWorkspace) return;

    if (diffStepIndex === step.index) {
      // Collapse diff
      diffStepIndex = null;
      diffResult = null;
      diffError = null;
      return;
    }

    diffStepIndex = step.index;
    diffLoading = true;
    diffError = null;
    diffResult = null;
    try {
      diffResult = await fetchStepDiff(expandedTaskId, step.index, selectedWorkspace.id);
    } catch (e: any) {
      diffError = e.message || String(e);
    } finally {
      diffLoading = false;
    }
  }

  async function loadSubtasks(task: ClineTaskSummary, e: MouseEvent) {
    e.stopPropagation();
    if (!selectedWorkspace) return;

    if (subtaskTaskId === task.taskId) {
      subtaskTaskId = null;
      subtasksData = null;
      subtasksError = null;
      subtaskDiffIndex = null;
      subtaskDiffResult = null;
      return;
    }

    subtaskTaskId = task.taskId;
    subtasksLoading = true;
    subtasksError = null;
    subtasksData = null;
    subtaskDiffIndex = null;
    subtaskDiffResult = null;
    try {
      subtasksData = await fetchTaskSubtasks(task.taskId);
    } catch (e: any) {
      subtasksError = e.message || String(e);
    } finally {
      subtasksLoading = false;
    }
  }

  async function loadSubtaskDiff(index: number, e: MouseEvent) {
    e.stopPropagation();
    if (!subtaskTaskId || !selectedWorkspace) return;

    if (subtaskDiffIndex === index) {
      subtaskDiffIndex = null;
      subtaskDiffResult = null;
      subtaskDiffError = null;
      return;
    }

    subtaskDiffIndex = index;
    subtaskDiffLoading = true;
    subtaskDiffError = null;
    subtaskDiffResult = null;
    subtaskDiffCopyLabel = '📋 Copy';
    try {
      subtaskDiffResult = await fetchSubtaskDiff(subtaskTaskId, index, selectedWorkspace.id);
    } catch (e: any) {
      subtaskDiffError = e.message || String(e);
    } finally {
      subtaskDiffLoading = false;
    }
  }

  function shortHash(hash: string): string {
    return hash.substring(0, 8);
  }

  function statusColor(status: string): string {
    switch (status) {
      case 'added': return 'text-green-600';
      case 'deleted': return 'text-red-600';
      case 'renamed': return 'text-purple-600';
      default: return 'text-yellow-600';
    }
  }

  function statusIcon(status: string): string {
    switch (status) {
      case 'added': return '+';
      case 'deleted': return '−';
      case 'renamed': return '→';
      default: return '~';
    }
  }

  function formatDate(iso: string): string {
    try {
      const d = new Date(iso);
      return d.toLocaleString(undefined, {
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit'
      });
    } catch {
      return iso;
    }
  }
</script>

<div class="flex-1 p-6 overflow-auto">

  <!-- ============ TASK LIST VIEW (drilled into a workspace) ============ -->
  {#if selectedWorkspace}
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <div class="flex items-center gap-3 mb-1">
          <button
            onclick={backToWorkspaces}
            class="text-blue-600 hover:text-blue-800 text-sm font-medium flex items-center gap-1 transition-colors"
          >
            ◂ Workspaces
          </button>
          <span class="text-gray-300">|</span>
          <h2 class="text-lg font-semibold text-gray-900">
            Workspace: <span class="font-mono">{selectedWorkspace.id}</span>
          </h2>
          {#if selectedWorkspace.active}
            <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">
              Active
            </span>
          {:else}
            <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-700">
              Paused
            </span>
          {/if}
        </div>
        <p class="text-sm text-gray-500">
          Git dir: <code class="bg-gray-100 px-1.5 py-0.5 rounded text-xs font-mono">{selectedWorkspace.gitDir}</code>
        </p>
      </div>
      <button
        onclick={refreshTasks}
        disabled={taskLoading}
        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {taskLoading ? 'Loading...' : 'Refresh'}
      </button>
    </div>

    <!-- Task Loading -->
    {#if taskLoading}
      <div class="flex items-center justify-center py-20">
        <div class="text-center">
          <svg class="animate-spin h-8 w-8 text-blue-500 mx-auto mb-3" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <p class="text-gray-500 text-sm">Enumerating tasks...</p>
        </div>
      </div>

    <!-- Task Error -->
    {:else if taskError}
      <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-red-500 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
          </svg>
          <div>
            <p class="text-sm font-medium text-red-800">Failed to list tasks</p>
            <p class="text-sm text-red-600 mt-1">{taskError}</p>
          </div>
        </div>
      </div>

    <!-- No Tasks -->
    {:else if tasks.length === 0}
      <div class="flex items-center justify-center py-20">
        <div class="text-center">
          <div class="text-4xl mb-4">📋</div>
          <h3 class="text-lg font-semibold text-gray-900 mb-2">No Tasks Found</h3>
          <p class="text-sm text-gray-500">
            No checkpoint commits found in this workspace.
          </p>
        </div>
      </div>

    <!-- Task Table -->
    {:else}
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
        <table class="w-full text-sm">
          <thead class="bg-gray-50 border-b border-gray-200">
            <tr>
              <th class="w-8 px-4 py-3"></th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">#</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Task ID</th>
              <th class="text-right px-4 py-3 font-medium text-gray-600">Steps</th>
              <th class="text-right px-4 py-3 font-medium text-gray-600">Files</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Last Changed</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600"></th>
            </tr>
          </thead>
          <tbody>
            {#each tasks as task, i}
              <tr
                class="border-b border-gray-100 hover:bg-blue-50 transition-colors cursor-pointer {i === 0 && expandedTaskId !== task.taskId ? 'bg-blue-50/60 ring-1 ring-inset ring-blue-200' : ''} {expandedTaskId === task.taskId ? 'bg-indigo-50' : ''}"
                onclick={() => toggleSteps(task)}
              >
                <td class="px-4 py-3 text-gray-400 text-xs text-center">
                  <span class="inline-block transition-transform {expandedTaskId === task.taskId ? 'rotate-90' : ''}">▸</span>
                </td>
                <td class="px-4 py-3 text-gray-400 font-mono text-xs">{i + 1}</td>
                <td class="px-4 py-3 font-mono font-medium text-gray-900">
                  {task.taskId}
                  {#if i === 0}
                    <span class="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
                  {/if}
                </td>
                <td class="px-4 py-3 text-right font-mono text-gray-700">{task.steps}</td>
                <td class="px-4 py-3 text-right font-mono text-gray-700">{task.filesChanged}</td>
                <td class="px-4 py-3 text-gray-600 text-xs">{formatDate(task.lastModified)}</td>
                <td class="px-4 py-3">
                  <div class="flex items-center gap-1.5">
                    <button
                      onclick={(e) => loadTaskDiff(task, e)}
                      class="text-xs font-medium px-2.5 py-1 rounded transition-colors {taskDiffId === task.taskId ? 'bg-purple-200 text-purple-800' : 'bg-purple-50 text-purple-700 hover:bg-purple-100'}"
                    >
                      {taskDiffId === task.taskId ? '▾ Hide Full Diff' : '▸ Full Diff'}
                    </button>
                    <button
                      onclick={(e) => loadSubtasks(task, e)}
                      class="text-xs font-medium px-2.5 py-1 rounded transition-colors {subtaskTaskId === task.taskId ? 'bg-teal-200 text-teal-800' : 'bg-teal-50 text-teal-700 hover:bg-teal-100'}"
                    >
                      {subtaskTaskId === task.taskId ? '▾ Subtasks' : '▸ Subtasks'}
                    </button>
                  </div>
                </td>
              </tr>
              <!-- Task-level Full Diff Panel -->
              {#if taskDiffId === task.taskId}
                <tr>
                  <td colspan="7" class="p-0">
                    <div class="bg-purple-50/50 border-t border-b border-purple-200 px-6 py-4">
                      {#if taskDiffLoading}
                        <div class="flex items-center gap-2 py-3">
                          <svg class="animate-spin h-4 w-4 text-purple-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          <span class="text-sm text-gray-500">Computing full task diff...</span>
                        </div>
                      {:else if taskDiffError}
                        <div class="text-sm text-red-600 py-2">Error: {taskDiffError}</div>
                      {:else if taskDiffResult}
                        <!-- Sticky header bar with copy button -->
                        <div class="flex items-center justify-between mb-2 sticky top-0 bg-purple-50/90 backdrop-blur-sm py-1 z-10">
                          <div class="text-[10px] text-gray-500 font-mono">
                            <span class="font-semibold text-purple-700">Full Task Diff</span> · {shortHash(taskDiffResult.fromRef)} → {shortHash(taskDiffResult.toRef)} · {taskDiffResult.files.length} file{taskDiffResult.files.length !== 1 ? 's' : ''} · {Math.round(taskDiffResult.patch.length / 1024)}KB
                          </div>
                          <div class="flex items-center gap-2">
                            {#if taskDiffResult.patch}
                              <button
                                onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(taskDiffResult!.patch); taskDiffCopyLabel = '✓ Copied!'; setTimeout(() => taskDiffCopyLabel = '📋 Copy Diff', 1500); }}
                                class="text-xs font-medium px-3 py-1 rounded bg-purple-600 hover:bg-purple-700 text-white transition-colors shadow-sm"
                              >
                                {taskDiffCopyLabel}
                              </button>
                            {/if}
                            <button
                              onclick={(e) => { e.stopPropagation(); taskDiffId = null; taskDiffResult = null; taskDiffError = null; }}
                              class="text-xs font-bold px-2 py-1 rounded bg-gray-200 hover:bg-red-100 text-gray-600 hover:text-red-700 transition-colors"
                              title="Close diff"
                            >✕</button>
                          </div>
                        </div>
                        <!-- Unified diff (shown immediately, scrollable with visible scrollbar) -->
                        {#if taskDiffResult.patch}
                          <pre class="diff-scroll" style="background: #111827; color: #e5e7eb; font-size: 10px; line-height: 16px; padding: 12px; border-radius: 6px; font-family: ui-monospace, monospace; white-space: pre-wrap; word-break: break-all; overflow-wrap: anywhere; user-select: text; margin: 0 0 12px 0; width: 100%; box-sizing: border-box;">{taskDiffResult.patch}</pre>
                        {:else}
                          <div class="text-xs text-gray-400 italic mb-3">No patch content (empty diff)</div>
                        {/if}
                        <!-- File list (collapsible) -->
                        <details class="group">
                          <summary class="text-[10px] font-medium text-gray-500 uppercase tracking-wide cursor-pointer hover:text-gray-700 select-none">
                            Files changed ({taskDiffResult.files.length})
                          </summary>
                          <div class="mt-2 bg-white rounded border border-gray-200 p-2">
                            {#each taskDiffResult.files as f}
                              <div class="flex items-center gap-2 py-0.5 text-xs font-mono">
                                <span class="{statusColor(f.status)} font-bold w-3 text-center">{statusIcon(f.status)}</span>
                                <span class="text-gray-700 truncate">{f.path}</span>
                                <span class="ml-auto text-green-600">+{f.linesAdded}</span>
                                <span class="text-red-600">-{f.linesRemoved}</span>
                              </div>
                            {/each}
                          </div>
                        </details>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/if}
              <!-- Subtask Panel -->
              {#if subtaskTaskId === task.taskId}
                <tr>
                  <td colspan="7" class="p-0">
                    <div class="bg-teal-50/50 border-t border-b border-teal-200 px-6 py-4">
                      {#if subtasksLoading}
                        <div class="flex items-center gap-2 py-3">
                          <svg class="animate-spin h-4 w-4 text-teal-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          <span class="text-sm text-gray-500">Loading subtasks...</span>
                        </div>
                      {:else if subtasksError}
                        <div class="text-sm text-red-600 py-2">Error: {subtasksError}</div>
                      {:else if subtasksData}
                        <div class="flex items-center justify-between mb-3">
                          <div class="text-xs font-medium text-teal-700 uppercase tracking-wide">
                            Subtasks ({subtasksData.totalSubtasks}) · Task: {subtasksData.taskId}
                          </div>
                          <button
                            onclick={(e) => { e.stopPropagation(); subtaskTaskId = null; subtasksData = null; subtasksError = null; subtaskDiffIndex = null; subtaskDiffResult = null; }}
                            class="text-xs font-bold px-2 py-1 rounded bg-gray-200 hover:bg-red-100 text-gray-600 hover:text-red-700 transition-colors"
                            title="Close subtasks"
                          >✕</button>
                        </div>
                        {#if subtasksData.subtasks.length === 0}
                          <div class="text-sm text-gray-500 py-2">No subtasks detected (single-prompt task).</div>
                        {:else}
                          <div class="space-y-2">
                            {#each subtasksData.subtasks as subtask, si}
                              <div class="bg-white rounded-lg border {subtaskDiffIndex === si ? 'border-teal-400 ring-1 ring-teal-200' : 'border-gray-200'} overflow-hidden">
                                <!-- Subtask header -->
                                <div class="flex items-start gap-3 px-4 py-3">
                                  <div class="flex-shrink-0 mt-0.5">
                                    <span class="inline-flex items-center justify-center w-6 h-6 rounded-full text-[10px] font-bold {si === 0 ? 'bg-teal-600 text-white' : 'bg-teal-100 text-teal-700'}">
                                      {si}
                                    </span>
                                  </div>
                                  <div class="flex-1 min-w-0">
                                    <div class="flex items-center gap-2 mb-1">
                                      <span class="text-xs font-semibold {si === 0 ? 'text-teal-700' : 'text-gray-700'}">
                                        {si === 0 ? '🎯 Initial Task' : `💬 Feedback #${si}`}
                                      </span>
                                      <span class="text-[10px] text-gray-400 font-mono">{formatDate(subtask.timestamp)}</span>
                                    </div>
                                    <p class="text-xs text-gray-600 whitespace-pre-wrap break-words line-clamp-3">{subtask.prompt}</p>
                                  </div>
                                  <button
                                    onclick={(e) => loadSubtaskDiff(si, e)}
                                    class="flex-shrink-0 text-xs font-medium px-2.5 py-1 rounded transition-colors {subtaskDiffIndex === si ? 'bg-teal-600 text-white' : 'bg-teal-50 text-teal-700 hover:bg-teal-100'}"
                                  >
                                    {subtaskDiffIndex === si ? '▾ Hide Diff' : '▸ Diff'}
                                  </button>
                                </div>
                                <!-- Subtask diff -->
                                {#if subtaskDiffIndex === si}
                                  <div class="border-t border-gray-200 px-4 py-3 bg-gray-50">
                                    {#if subtaskDiffLoading}
                                      <div class="flex items-center gap-2 py-2">
                                        <svg class="animate-spin h-4 w-4 text-teal-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                        </svg>
                                        <span class="text-xs text-gray-500">Computing subtask diff...</span>
                                      </div>
                                    {:else if subtaskDiffError}
                                      <div class="text-xs text-red-600 py-1">Error: {subtaskDiffError}</div>
                                    {:else if subtaskDiffResult}
                                      <div class="flex items-center justify-between mb-2">
                                        <div class="text-[10px] text-gray-400 font-mono">
                                          <span class="font-semibold text-teal-700">Subtask #{subtaskDiffIndex} Diff</span> · {shortHash(subtaskDiffResult.fromRef)} → {shortHash(subtaskDiffResult.toRef)} · {subtaskDiffResult.files.length} file{subtaskDiffResult.files.length !== 1 ? 's' : ''} · {Math.round(subtaskDiffResult.patch.length / 1024)}KB
                                        </div>
                                        {#if subtaskDiffResult.patch}
                                          <button
                                            onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(subtaskDiffResult!.patch); subtaskDiffCopyLabel = '✓ Copied'; setTimeout(() => subtaskDiffCopyLabel = '📋 Copy', 1500); }}
                                            class="text-[10px] font-medium px-2 py-0.5 rounded bg-teal-600 hover:bg-teal-700 text-white transition-colors"
                                          >
                                            {subtaskDiffCopyLabel}
                                          </button>
                                        {/if}
                                      </div>
                                      {#if subtaskDiffResult.patch}
                                        <pre class="diff-scroll" style="background: #111827; color: #e5e7eb; font-size: 10px; line-height: 16px; padding: 12px; border-radius: 6px; font-family: ui-monospace, monospace; white-space: pre-wrap; word-break: break-all; overflow-wrap: anywhere; user-select: text; margin: 0 0 12px 0; max-height: 384px !important; width: 100%; box-sizing: border-box;">{subtaskDiffResult.patch}</pre>
                                      {:else}
                                        <div class="text-xs text-gray-400 italic mb-3">No patch content (empty diff or no checkpoint steps in this subtask's time window)</div>
                                      {/if}
                                      <details class="group">
                                        <summary class="text-[10px] font-medium text-gray-500 uppercase tracking-wide cursor-pointer hover:text-gray-700 select-none">
                                          Files changed ({subtaskDiffResult.files.length})
                                        </summary>
                                        <div class="mt-2 bg-white rounded border border-gray-200 p-2">
                                          {#each subtaskDiffResult.files as f}
                                            <div class="flex items-center gap-2 py-0.5 text-xs font-mono">
                                              <span class="{statusColor(f.status)} font-bold w-3 text-center">{statusIcon(f.status)}</span>
                                              <span class="text-gray-700 truncate">{f.path}</span>
                                              <span class="ml-auto text-green-600">+{f.linesAdded}</span>
                                              <span class="text-red-600">-{f.linesRemoved}</span>
                                            </div>
                                          {/each}
                                        </div>
                                      </details>
                                    {/if}
                                  </div>
                                {/if}
                              </div>
                            {/each}
                          </div>
                        {/if}
                      {/if}
                    </div>
                  </td>
                </tr>
              {/if}
              <!-- Expanded Steps Panel -->
              {#if expandedTaskId === task.taskId}
                <tr>
                  <td colspan="6" class="p-0">
                    <div class="bg-gray-50 border-t border-b border-gray-200 px-6 py-4">
                      {#if stepsLoading}
                        <div class="flex items-center gap-2 py-3">
                          <svg class="animate-spin h-4 w-4 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                          </svg>
                          <span class="text-sm text-gray-500">Loading steps...</span>
                        </div>
                      {:else if stepsError}
                        <div class="text-sm text-red-600 py-2">
                          Error: {stepsError}
                        </div>
                      {:else if steps.length === 0}
                        <div class="text-sm text-gray-500 py-2">No steps found for this task.</div>
                      {:else}
                        <div class="text-xs font-medium text-gray-500 uppercase tracking-wide mb-2">
                          Checkpoint Steps ({steps.length})
                        </div>
                        <table class="w-full text-xs">
                          <thead>
                            <tr class="text-gray-500">
                              <th class="text-left px-3 py-1.5 font-medium">Step</th>
                              <th class="text-left px-3 py-1.5 font-medium">Commit</th>
                              <th class="text-right px-3 py-1.5 font-medium">Files</th>
                              <th class="text-left px-3 py-1.5 font-medium">Timestamp</th>
                              <th class="text-left px-3 py-1.5 font-medium"></th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each steps as step, si}
                              <tr class="border-t border-gray-200 {si === 0 ? 'bg-indigo-50/50' : 'hover:bg-white'} {diffStepIndex === step.index ? 'bg-amber-50' : ''}">
                                <td class="px-3 py-2 font-mono text-gray-600">
                                  {step.index}
                                  {#if si === 0}
                                    <span class="ml-1 text-[9px] font-bold text-indigo-600 uppercase">latest</span>
                                  {/if}
                                </td>
                                <td class="px-3 py-2 font-mono text-gray-800" title={step.hash}>
                                  {shortHash(step.hash)}
                                </td>
                                <td class="px-3 py-2 text-right font-mono text-gray-600">{step.filesChanged}</td>
                                <td class="px-3 py-2 text-gray-500">{formatDate(step.timestamp)}</td>
                                <td class="px-3 py-2">
                                  <button
                                    onclick={(e) => loadStepDiff(step, e)}
                                    class="text-xs font-medium px-2 py-0.5 rounded transition-colors {diffStepIndex === step.index ? 'bg-amber-200 text-amber-800' : 'text-blue-600 hover:bg-blue-100'}"
                                  >
                                    {diffStepIndex === step.index ? '▾ Hide Diff' : '▸ Diff'}
                                  </button>
                                </td>
                              </tr>
                              <!-- Inline step diff -->
                              {#if diffStepIndex === step.index}
                                <tr>
                                  <td colspan="5" class="p-0">
                                    <div class="bg-white border-t border-gray-200 px-4 py-3">
                                      {#if diffLoading}
                                        <div class="flex items-center gap-2 py-2">
                                          <svg class="animate-spin h-4 w-4 text-amber-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                          </svg>
                                          <span class="text-xs text-gray-500">Computing diff...</span>
                                        </div>
                                      {:else if diffError}
                                        <div class="text-xs text-red-600 py-1">Error: {diffError}</div>
                                      {:else if diffResult}
                                        <div class="flex items-center justify-between mb-2">
                                          <div class="text-[10px] text-gray-400 font-mono">
                                            {shortHash(diffResult.fromRef)} → {shortHash(diffResult.toRef)} · {diffResult.files.length} file{diffResult.files.length !== 1 ? 's' : ''}
                                          </div>
                                          {#if diffResult.patch}
                                            <button
                                              onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(diffResult!.patch); copyLabel = '✓ Copied'; setTimeout(() => copyLabel = '📋 Copy', 1500); }}
                                              class="text-[10px] font-medium px-2 py-0.5 rounded bg-gray-100 hover:bg-gray-200 text-gray-700 transition-colors"
                                            >
                                              {copyLabel}
                                            </button>
                                          {/if}
                                        </div>
                                        <!-- Unified diff (shown immediately) -->
                                        {#if diffResult.patch}
                                          <pre class="diff-scroll" style="background: #111827; color: #e5e7eb; font-size: 10px; line-height: 16px; padding: 12px; border-radius: 6px; font-family: ui-monospace, monospace; white-space: pre-wrap; word-break: break-all; overflow-wrap: anywhere; user-select: text; margin: 0 0 12px 0; max-height: 384px !important; width: 100%; box-sizing: border-box;">{diffResult.patch}</pre>
                                        {:else}
                                          <div class="text-xs text-gray-400 italic mb-3">No patch content (empty diff)</div>
                                        {/if}
                                        <!-- File list (collapsible) -->
                                        <details class="group">
                                          <summary class="text-[10px] font-medium text-gray-500 uppercase tracking-wide cursor-pointer hover:text-gray-700 select-none">
                                            Files changed ({diffResult.files.length})
                                          </summary>
                                          <div class="mt-2 bg-gray-50 rounded border border-gray-200 p-2">
                                            {#each diffResult.files as f}
                                              <div class="flex items-center gap-2 py-0.5 text-xs font-mono">
                                                <span class="{statusColor(f.status)} font-bold w-3 text-center">{statusIcon(f.status)}</span>
                                                <span class="text-gray-700 truncate">{f.path}</span>
                                                <span class="ml-auto text-green-600">+{f.linesAdded}</span>
                                                <span class="text-red-600">-{f.linesRemoved}</span>
                                              </div>
                                            {/each}
                                          </div>
                                        </details>
                                      {/if}
                                    </div>
                                  </td>
                                </tr>
                              {/if}
                            {/each}
                          </tbody>
                        </table>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Summary -->
      <div class="mt-4 text-sm text-gray-500">
        Total: {tasks.length} task{tasks.length !== 1 ? 's' : ''},
        {tasks.reduce((s, t) => s + t.steps, 0)} steps,
        {tasks.reduce((s, t) => s + t.filesChanged, 0)} files touched
      </div>
    {/if}

  <!-- ============ WORKSPACE LIST VIEW (default) ============ -->
  {:else}
    <!-- Header -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h2 class="text-lg font-semibold text-gray-900">Checkpoint Workspaces</h2>
        <p class="text-sm text-gray-500 mt-1">
          Scanning: <code class="bg-gray-100 px-1.5 py-0.5 rounded text-xs font-mono">{checkpointsRoot || '...'}</code>
        </p>
      </div>
      <button
        onclick={() => loadWorkspaces(true)}
        disabled={wsLoading}
        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {wsLoading ? 'Scanning...' : 'Refresh'}
      </button>
    </div>

    <!-- Loading State -->
    {#if wsLoading}
      <div class="flex items-center justify-center py-20">
        <div class="text-center">
          <svg class="animate-spin h-8 w-8 text-blue-500 mx-auto mb-3" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <p class="text-gray-500 text-sm">Discovering checkpoint repositories...</p>
        </div>
      </div>

    <!-- Error State -->
    {:else if wsError}
      <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-red-500 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
          </svg>
          <div>
            <p class="text-sm font-medium text-red-800">Failed to discover workspaces</p>
            <p class="text-sm text-red-600 mt-1">{wsError}</p>
          </div>
        </div>
      </div>

    <!-- Empty State -->
    {:else if workspaces.length === 0}
      <div class="flex items-center justify-center py-20">
        <div class="text-center max-w-md">
          <div class="text-4xl mb-4">📂</div>
          <h3 class="text-lg font-semibold text-gray-900 mb-2">No Checkpoints Found</h3>
          <p class="text-sm text-gray-500 mb-4">
            No Cline checkpoint repositories were found.
          </p>
          <div class="bg-gray-50 border border-gray-200 rounded-lg p-3 text-left">
            <p class="text-xs text-gray-600 mb-1 font-medium">Expected location:</p>
            <code class="text-xs font-mono text-gray-700 break-all">
              %APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\checkpoints\
            </code>
            <p class="text-xs text-gray-500 mt-2">
              Make sure the Cline extension is installed and you have run at least one task.
            </p>
          </div>
        </div>
      </div>

    <!-- Workspace Table -->
    {:else}
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
        <table class="w-full text-sm">
          <thead class="bg-gray-50 border-b border-gray-200">
            <tr>
              <th class="text-left px-4 py-3 font-medium text-gray-600">#</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Workspace ID</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Status</th>
              <th class="text-right px-4 py-3 font-medium text-gray-600">Tasks</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Last Changed</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Git Dir</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600"></th>
            </tr>
          </thead>
          <tbody>
            {#each workspaces as ws, i}
              <tr
                class="border-b border-gray-100 hover:bg-blue-50 transition-colors cursor-pointer {i === 0 ? 'bg-blue-50/60 ring-1 ring-inset ring-blue-200' : ''}"
                onclick={() => selectWorkspace(ws)}
              >
                <td class="px-4 py-3 text-gray-400 font-mono text-xs">{i + 1}</td>
                <td class="px-4 py-3 font-mono font-medium text-gray-900">
                  {ws.id}
                  {#if i === 0}
                    <span class="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
                  {/if}
                </td>
                <td class="px-4 py-3">
                  {#if ws.active}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">
                      <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
                      Active
                    </span>
                  {:else}
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-700">
                      <span class="w-1.5 h-1.5 rounded-full bg-yellow-500"></span>
                      Paused
                    </span>
                  {/if}
                </td>
                <td class="px-4 py-3 text-right font-mono text-gray-700">{ws.taskCount}</td>
                <td class="px-4 py-3 text-gray-600 text-xs">
                  {ws.lastModified ? formatDate(ws.lastModified) : '—'}
                </td>
                <td class="px-4 py-3 text-gray-500 font-mono text-xs truncate max-w-xs" title={ws.gitDir}>
                  {ws.gitDir}
                </td>
                <td class="px-4 py-3">
                  <span class="text-blue-600 text-xs font-medium">View Tasks ▸</span>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Summary -->
      <div class="mt-4 text-sm text-gray-500">
        Total: {workspaces.length} workspace{workspaces.length !== 1 ? 's' : ''},
        {workspaces.reduce((sum, ws) => sum + ws.taskCount, 0)} tasks
      </div>
    {/if}
  {/if}
</div>

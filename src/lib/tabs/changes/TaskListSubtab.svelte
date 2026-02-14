<script lang="ts">
  import { onMount } from "svelte";
  import { fetchWorkspaces, fetchTasks, fetchSteps, fetchStepDiff, fetchTaskDiff, fetchSubtaskDiff, nukeWorkspace, fetchFileContents } from "./api";
  import { fetchTaskSubtasks, fetchHistoryTasks } from "../history/api";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import { pinStore } from "../../stores/pinStore.svelte";
  import type { SubtasksResponse } from "../history/types";
  import type { ChatAttachment } from "../agent/types";
  import type { WorkspaceInfo, WorkspacesResponse, ClineTaskSummary, TasksResponse, CheckpointStep, DiffResult, DiffFile } from "./types";

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
  let showTaskGitCmds = $state(false);
  let showSubtaskGitCmds = $state(false);
  let showStepGitCmds = $state(false);
  let showDiscoveryGitCmd = $state(false);
  let gitCmdCopyLabel = $state('📋');

  // ---- Ask LLM state ----
  let askLlmLoading = $state(false);
  let askLlmSubtaskLoading = $state(false);

  // ---- Task prompt enrichment (from History) ----
  let taskPromptMap: Map<string, string> = $state(new Map());
  let promptsLoading = $state(false);

  // ---- Workspace prompt enrichment (from History) ----
  let workspacePromptMap: Map<string, string> = $state(new Map());
  let wsPromptsLoading = $state(false);

  // ---- Search state (shared across workspace & task views) ----
  let searchQuery = $state('');
  /** All subtask prompts per workspace (workspaceId → string[]) for search filtering */
  let workspaceAllPromptsMap: Map<string, string[]> = $state(new Map());
  /** All subtask prompts per task (taskId → string[]) for search filtering */
  let taskSubtaskPromptsMap: Map<string, string[]> = $state(new Map());

  // Pin filter state is persisted in pinStore (survives tab switches & app restarts)

  // ---- Nuke state ----
  let showNukeConfirm = $state(false);
  let nukeLoading = $state(false);
  let nukeError: string | null = $state(null);

  // ---- Derived filtered lists (search across both views) ----
  /** Count of pinned workspaces in current list (reactive) */
  let pinnedWsCount = $derived.by(() => {
    const pins = pinStore.pinnedWorkspaceSet;
    return workspaces.filter(ws => pins.has(ws.id)).length;
  });

  let filteredWorkspaces = $derived.by(() => {
    let result = workspaces;

    // Apply search filter
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      result = result.filter(ws => {
        if (ws.id.toLowerCase().includes(q)) return true;
        const latestPrompt = workspacePromptMap.get(ws.id);
        if (latestPrompt && latestPrompt.toLowerCase().includes(q)) return true;
        const allPrompts = workspaceAllPromptsMap.get(ws.id);
        if (allPrompts && allPrompts.some(p => p.toLowerCase().includes(q))) return true;
        return false;
      });
    }

    // Apply workspace pin filter
    if (pinStore.wsFilter === 'pinned') {
      result = result.filter(ws => pinStore.isWorkspacePinned(ws.id));
    } else {
      // In "all" view, sort pinned workspaces to the top
      const pinned = result.filter(ws => pinStore.isWorkspacePinned(ws.id));
      const unpinned = result.filter(ws => !pinStore.isWorkspacePinned(ws.id));
      result = [...pinned, ...unpinned];
    }

    return result;
  });

  /** Count of pinned tasks in the current workspace (reactive) */
  let pinnedTaskCount = $derived.by(() => {
    // Access pinStore.pinnedSet to create reactivity
    const pins = pinStore.pinnedSet;
    return tasks.filter(t => pins.has(t.taskId)).length;
  });

  let filteredTasks = $derived.by(() => {
    let result = tasks;

    // Apply search filter
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim();
      result = result.filter(t => {
        if (t.taskId.toLowerCase().includes(q)) return true;
        const prompt = taskPromptMap.get(t.taskId);
        if (prompt && prompt.toLowerCase().includes(q)) return true;
        const subtaskPrompts = taskSubtaskPromptsMap.get(t.taskId);
        if (subtaskPrompts && subtaskPrompts.some(p => p.toLowerCase().includes(q))) return true;
        return false;
      });
    }

    // Apply pin filter
    if (pinStore.taskFilter === 'pinned') {
      result = result.filter(t => pinStore.isPinned(t.taskId));
    } else {
      // In "all" view, sort pinned tasks to the top (preserving relative order within groups)
      const pinned = result.filter(t => pinStore.isPinned(t.taskId));
      const unpinned = result.filter(t => !pinStore.isPinned(t.taskId));
      result = [...pinned, ...unpinned];
    }

    return result;
  });

  // ── Expansion state persistence ───────────────────────────────────────────

  /** Save current expansion state to navigation store (persisted to localStorage) */
  function saveExpansionState() {
    navigationStore.updateChangesTasksState({
      selectedWorkspaceId: selectedWorkspace?.id ?? null,
      expandedTaskId,
      taskDiffId,
      subtaskTaskId,
      subtaskDiffIndex,
    });
  }

  /** Core: expand task diff (no toggle, no event) — used by restore & UI */
  async function expandTaskDiff(tid: string) {
    if (!selectedWorkspace) return;
    taskDiffId = tid;
    taskDiffLoading = true;
    taskDiffError = null;
    taskDiffResult = null;
    taskDiffCopyLabel = '📋 Copy';
    try {
      taskDiffResult = await fetchTaskDiff(tid, selectedWorkspace.id);
    } catch (e: any) {
      taskDiffError = e.message || String(e);
    } finally {
      taskDiffLoading = false;
    }
  }

  /** Core: expand subtasks panel (no toggle, no event) — used by restore & UI */
  async function expandSubtasks(tid: string) {
    if (!selectedWorkspace) return;
    subtaskTaskId = tid;
    subtasksLoading = true;
    subtasksError = null;
    subtasksData = null;
    subtaskDiffIndex = null;
    subtaskDiffResult = null;
    try {
      subtasksData = await fetchTaskSubtasks(tid);
    } catch (e: any) {
      subtasksError = e.message || String(e);
    } finally {
      subtasksLoading = false;
    }
  }

  /** Core: expand subtask diff (no toggle, no event) — used by restore & UI */
  async function expandSubtaskDiff(index: number) {
    if (!subtaskTaskId || !selectedWorkspace) return;
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

  /** Core: expand steps for a task (no toggle, no event) — used by restore */
  async function expandSteps(task: ClineTaskSummary) {
    expandedTaskId = task.taskId;
    stepsLoading = true;
    stepsError = null;
    steps = [];
    try {
      const resp = await fetchSteps(task.taskId, task.workspaceId);
      steps = resp.steps.slice().reverse();
    } catch (e: any) {
      stepsError = e.message || String(e);
    } finally {
      stepsLoading = false;
    }
  }

  /**
   * Restore previously saved expansion state after workspaces load.
   * Chains: workspace → tasks → (task diff | subtasks | steps) in parallel.
   */
  async function restoreSavedState() {
    const saved = navigationStore.changesTasksState;
    if (!saved.selectedWorkspaceId) return;

    const ws = workspaces.find(w => w.id === saved.selectedWorkspaceId);
    if (!ws) return;

    // Restore workspace selection (loads tasks)
    await selectWorkspace(ws);
    if (tasks.length === 0) return;

    // Restore task-level expansions in parallel
    const promises: Promise<void>[] = [];

    if (saved.taskDiffId && tasks.find(t => t.taskId === saved.taskDiffId)) {
      promises.push(expandTaskDiff(saved.taskDiffId));
    }

    if (saved.subtaskTaskId && tasks.find(t => t.taskId === saved.subtaskTaskId)) {
      promises.push(
        expandSubtasks(saved.subtaskTaskId).then(async () => {
          // After subtasks loaded, restore subtask diff if saved
          if (saved.subtaskDiffIndex !== null && subtasksData && saved.subtaskDiffIndex < subtasksData.subtasks.length) {
            await expandSubtaskDiff(saved.subtaskDiffIndex);
          }
        })
      );
    }

    if (saved.expandedTaskId && tasks.find(t => t.taskId === saved.expandedTaskId)) {
      const task = tasks.find(t => t.taskId === saved.expandedTaskId)!;
      promises.push(expandSteps(task));
    }

    await Promise.all(promises);
  }

  onMount(async () => {
    await loadWorkspaces(false);
    // Restore saved expansion state (fire-and-forget — non-blocking for workspace prompt enrichment)
    restoreSavedState();
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
    // Fire-and-forget: enrich workspaces with latest task prompt from History
    if (workspaces.length > 0) {
      enrichWorkspacePrompts(workspaces);
    }
  }

  /**
   * For each workspace, fetch its task list, find the latest task,
   * and look up that task's prompt from the History tab.
   * Runs in the background (non-blocking) so the workspace table shows immediately.
   */
  async function enrichWorkspacePrompts(wsList: WorkspaceInfo[]) {
    wsPromptsLoading = true;
    try {
      // 1. Fetch history tasks once (contains taskId → prompt mapping)
      const historyResp = await fetchHistoryTasks(false);
      const historyPromptMap = new Map<string, string>();
      for (const ht of historyResp.tasks) {
        if (ht.taskPrompt) {
          historyPromptMap.set(ht.taskId, ht.taskPrompt);
        }
      }

      // Also build a map of taskId → subtaskPrompts for search
      const historySubtaskPromptsMap = new Map<string, string[]>();
      for (const ht of historyResp.tasks) {
        if (ht.subtaskPrompts && ht.subtaskPrompts.length > 0) {
          historySubtaskPromptsMap.set(ht.taskId, ht.subtaskPrompts);
        }
      }

      // 2. For each workspace, fetch its tasks and find the latest task's prompt
      const newMap = new Map<string, string>();
      const newAllPromptsMap = new Map<string, string[]>();
      const fetches = wsList.map(async (ws) => {
        try {
          const tasksResp = await fetchTasks(ws.id, false);
          if (tasksResp.tasks.length > 0) {
            // Tasks are sorted newest-first; take the latest
            const latestTask = tasksResp.tasks[0];
            const prompt = historyPromptMap.get(latestTask.taskId);
            if (prompt) {
              newMap.set(ws.id, prompt);
            }
            // Collect all subtask prompts across all tasks in this workspace
            const allPrompts: string[] = [];
            for (const t of tasksResp.tasks) {
              const sp = historySubtaskPromptsMap.get(t.taskId);
              if (sp) allPrompts.push(...sp);
            }
            if (allPrompts.length > 0) {
              newAllPromptsMap.set(ws.id, allPrompts);
            }
          }
        } catch (e: any) {
          console.warn(`[enrichWorkspacePrompts] Failed to fetch tasks for workspace ${ws.id}:`, e.message);
        }
      });
      await Promise.all(fetches);
      workspacePromptMap = newMap;
      workspaceAllPromptsMap = newAllPromptsMap;
    } catch (e: any) {
      console.warn('[enrichWorkspacePrompts] Failed to fetch history tasks:', e.message);
    } finally {
      wsPromptsLoading = false;
    }
  }

  async function selectWorkspace(ws: WorkspaceInfo) {
    selectedWorkspace = ws;
    taskLoading = true;
    taskError = null;
    tasks = [];
    taskPromptMap = new Map();
    try {
      const resp: TasksResponse = await fetchTasks(ws.id, false);
      tasks = resp.tasks;
      // Fire-and-forget: enrich tasks with prompts from History
      enrichTaskPrompts(resp.tasks.map(t => t.taskId));
    } catch (e: any) {
      taskError = e.message || String(e);
    } finally {
      taskLoading = false;
      saveExpansionState();
    }
  }

  async function refreshTasks() {
    if (!selectedWorkspace) return;
    taskLoading = true;
    taskError = null;
    taskPromptMap = new Map();
    try {
      const resp: TasksResponse = await fetchTasks(selectedWorkspace.id, true);
      tasks = resp.tasks;
      // Fire-and-forget: enrich tasks with prompts from History
      enrichTaskPrompts(resp.tasks.map(t => t.taskId));
    } catch (e: any) {
      taskError = e.message || String(e);
    } finally {
      taskLoading = false;
    }
  }

  /**
   * Fetch conversation history task list and build a taskId → prompt map.
   * Runs in the background (non-blocking) so the task table shows immediately.
   */
  async function enrichTaskPrompts(taskIds: string[]) {
    if (taskIds.length === 0) return;
    promptsLoading = true;
    try {
      const historyResp = await fetchHistoryTasks(false);
      const newMap = new Map<string, string>();
      const newSubtaskMap = new Map<string, string[]>();
      for (const ht of historyResp.tasks) {
        if (taskIds.includes(ht.taskId)) {
          if (ht.taskPrompt) {
            newMap.set(ht.taskId, ht.taskPrompt);
          }
          if (ht.subtaskPrompts && ht.subtaskPrompts.length > 0) {
            newSubtaskMap.set(ht.taskId, ht.subtaskPrompts);
          }
        }
      }
      taskPromptMap = newMap;
      taskSubtaskPromptsMap = newSubtaskMap;
    } catch (e: any) {
      console.warn('[enrichTaskPrompts] Failed to fetch history tasks for prompt enrichment:', e.message);
    } finally {
      promptsLoading = false;
    }
  }

  function backToWorkspaces() {
    selectedWorkspace = null;
    tasks = [];
    taskError = null;
    expandedTaskId = null;
    taskDiffId = null;
    taskDiffResult = null;
    subtaskTaskId = null;
    subtasksData = null;
    subtaskDiffIndex = null;
    subtaskDiffResult = null;
    steps = [];
    taskPromptMap = new Map();
    saveExpansionState();
  }

  async function toggleSteps(task: ClineTaskSummary) {
    if (expandedTaskId === task.taskId) {
      expandedTaskId = null;
      steps = [];
      stepsError = null;
      saveExpansionState();
      return;
    }
    await expandSteps(task);
    saveExpansionState();
  }

  async function loadTaskDiff(task: ClineTaskSummary, e: MouseEvent) {
    e.stopPropagation();
    if (!selectedWorkspace) return;

    if (taskDiffId === task.taskId) {
      taskDiffId = null;
      taskDiffResult = null;
      taskDiffError = null;
      saveExpansionState();
      return;
    }

    await expandTaskDiff(task.taskId);
    saveExpansionState();
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
      saveExpansionState();
      return;
    }

    await expandSubtasks(task.taskId);
    saveExpansionState();
  }

  async function loadSubtaskDiff(index: number, e: MouseEvent) {
    e.stopPropagation();
    if (!subtaskTaskId || !selectedWorkspace) return;

    if (subtaskDiffIndex === index) {
      subtaskDiffIndex = null;
      subtaskDiffResult = null;
      subtaskDiffError = null;
      saveExpansionState();
      return;
    }

    await expandSubtaskDiff(index);
    saveExpansionState();
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

  async function confirmNuke() {
    if (!selectedWorkspace) return;
    nukeLoading = true;
    nukeError = null;
    try {
      await nukeWorkspace(selectedWorkspace.id);
      // After nuke: close dialog and refresh tasks (should show empty)
      showNukeConfirm = false;
      tasks = [];
      expandedTaskId = null;
      steps = [];
      taskDiffId = null;
      taskDiffResult = null;
      subtaskTaskId = null;
      subtasksData = null;
      // Refresh tasks to confirm empty
      await refreshTasks();
      saveExpansionState();
    } catch (e: any) {
      nukeError = e.message || String(e);
    } finally {
      nukeLoading = false;
    }
  }

  function extractPerFileDiffs(patch: string, files: DiffFile[]): string {
    const fileSections: string[] = [];
    const parts = patch.split(/^diff --git /m);
    const fileMap = new Map<string, DiffFile>();
    for (const f of files) fileMap.set(f.path, f);
    for (const part of parts) {
      if (!part.trim()) continue;
      const section = 'diff --git ' + part;
      const headerMatch = section.match(/^diff --git a\/(.+?) b\/(.+)/m);
      const filePath = headerMatch ? headerMatch[2] : '(unknown)';
      const fileMeta = fileMap.get(filePath);
      const status = fileMeta ? fileMeta.status.toUpperCase() : 'MODIFIED';
      const added = fileMeta ? fileMeta.linesAdded : 0;
      const removed = fileMeta ? fileMeta.linesRemoved : 0;
      fileSections.push(
        `${'='.repeat(60)}\nFILE: ${filePath}\nSTATUS: ${status}  (+${added} -${removed})\n${'='.repeat(60)}\n` + section.trimEnd()
      );
    }
    return fileSections.length > 0 ? fileSections.join('\n\n') : patch;
  }

  async function askLlmForTask(task: ClineTaskSummary, ev: MouseEvent) {
    ev.stopPropagation();
    if (!selectedWorkspace || !taskDiffResult) return;
    askLlmLoading = true;
    try {
      const atts: ChatAttachment[] = [];
      const diff = taskDiffResult;
      const workspaceId = selectedWorkspace.id;

      // 1. Fetch subtask prompts
      try {
        const subtasksResp = await fetchTaskSubtasks(task.taskId);
        if (subtasksResp.subtasks && subtasksResp.subtasks.length > 0) {
          const allPrompts = subtasksResp.subtasks.map((s: any) =>
            `${s.isInitialTask ? '🎯 Initial Task' : `💬 Feedback #${s.subtaskIndex}`} (${formatDate(s.timestamp)}):\n${s.prompt}`
          ).join('\n\n---\n\n');
          atts.push({ id: `prompts-${Date.now()}`, label: `All Prompts (${subtasksResp.subtasks.length})`, type: 'prompts', content: allPrompts, meta: { count: subtasksResp.subtasks.length } });
        }
      } catch (e: any) {
        console.warn('[askLlmForTask] Failed to fetch subtasks/prompts:', e.message);
      }

      // 2. Fetch actual file contents from shadow git
      if (diff.files.length > 0) {
        const nonDeletedPaths = diff.files.filter((f: any) => f.status !== 'deleted').map((f: any) => f.path);
        const deletedFiles = diff.files.filter((f: any) => f.status === 'deleted');
        let fileBodySections: string[] = [];
        if (nonDeletedPaths.length > 0) {
          try {
            const fileContentsResp = await fetchFileContents(workspaceId, diff.toRef, nonDeletedPaths);
            const fileMeta = new Map(diff.files.map((f: any) => [f.path, f]));
            for (const fc of fileContentsResp.files) {
              const meta = fileMeta.get(fc.path);
              const status = meta ? (meta as any).status.toUpperCase() : 'MODIFIED';
              const added = meta ? (meta as any).linesAdded : 0;
              const removed = meta ? (meta as any).linesRemoved : 0;
              if (fc.content !== null) {
                fileBodySections.push(`${'='.repeat(60)}\nFILE: ${fc.path}\nSTATUS: ${status}  (+${added} -${removed})  SIZE: ${fc.size ?? fc.content.length} bytes\n${'='.repeat(60)}\n` + fc.content);
              } else {
                fileBodySections.push(`${'='.repeat(60)}\nFILE: ${fc.path}\nSTATUS: ${status}  [content unavailable: ${fc.error || 'unknown error'}]\n${'='.repeat(60)}`);
              }
            }
          } catch (e: any) {
            console.warn('[askLlmForTask] Failed to fetch file contents, falling back to diff:', e.message);
            if (diff.patch) fileBodySections = [extractPerFileDiffs(diff.patch, diff.files)];
          }
        }
        for (const df of deletedFiles) {
          fileBodySections.push(`${'='.repeat(60)}\nFILE: ${df.path}\nSTATUS: DELETED  (-${df.linesRemoved} lines)\n${'='.repeat(60)}\n[File was deleted in this task]`);
        }
        if (fileBodySections.length > 0) {
          const fullContent = fileBodySections.join('\n\n');
          atts.push({ id: `files-${Date.now()}`, label: `File Contents (${diff.files.length} files, ${(fullContent.length / 1024).toFixed(1)}KB)`, type: 'files', content: fullContent, meta: { count: diff.files.length, sizeKB: Math.round(fullContent.length / 1024) } });
        }
      }

      // 3. Unified diff patch
      if (diff.patch) {
        atts.push({ id: `diff-${Date.now()}`, label: `Unified Diff (${(diff.patch.length / 1024).toFixed(1)}KB)`, type: 'diff', content: diff.patch, meta: { sizeKB: Math.round(diff.patch.length / 1024) } });
      }

      console.log('[askLlmForTask] attachments:', atts.length, atts.map(a => a.label));
      navigationStore.navigateToChat({ attachments: atts, timestamp: Date.now() });
    } finally {
      askLlmLoading = false;
    }
  }

  async function askLlmForSubtask(subtaskIndex: number, ev: MouseEvent) {
    ev.stopPropagation();
    if (!selectedWorkspace || !subtaskDiffResult || !subtasksData) return;
    askLlmSubtaskLoading = true;
    try {
      const atts: ChatAttachment[] = [];
      const diff = subtaskDiffResult;
      const workspaceId = selectedWorkspace.id;
      const subtask = subtasksData.subtasks[subtaskIndex];

      // 1. Subtask prompt
      if (subtask) {
        const promptLabel = subtask.isInitialTask ? '🎯 Initial Task' : `💬 Feedback #${subtask.subtaskIndex}`;
        const promptContent = `${promptLabel} (${formatDate(subtask.timestamp)}):\n${subtask.prompt}`;
        atts.push({ id: `prompt-${Date.now()}`, label: `Subtask #${subtaskIndex} Prompt`, type: 'prompts', content: promptContent, meta: { count: 1 } });
      }

      // 2. Fetch actual file contents from shadow git
      if (diff.files.length > 0) {
        const nonDeletedPaths = diff.files.filter((f: any) => f.status !== 'deleted').map((f: any) => f.path);
        const deletedFiles = diff.files.filter((f: any) => f.status === 'deleted');
        let fileBodySections: string[] = [];
        if (nonDeletedPaths.length > 0) {
          try {
            const fileContentsResp = await fetchFileContents(workspaceId, diff.toRef, nonDeletedPaths);
            const fileMeta = new Map(diff.files.map((f: any) => [f.path, f]));
            for (const fc of fileContentsResp.files) {
              const meta = fileMeta.get(fc.path);
              const status = meta ? (meta as any).status.toUpperCase() : 'MODIFIED';
              const added = meta ? (meta as any).linesAdded : 0;
              const removed = meta ? (meta as any).linesRemoved : 0;
              if (fc.content !== null) {
                fileBodySections.push(`${'='.repeat(60)}\nFILE: ${fc.path}\nSTATUS: ${status}  (+${added} -${removed})  SIZE: ${fc.size ?? fc.content.length} bytes\n${'='.repeat(60)}\n` + fc.content);
              } else {
                fileBodySections.push(`${'='.repeat(60)}\nFILE: ${fc.path}\nSTATUS: ${status}  [content unavailable: ${fc.error || 'unknown error'}]\n${'='.repeat(60)}`);
              }
            }
          } catch (e: any) {
            console.warn('[askLlmForSubtask] Failed to fetch file contents, falling back to diff:', e.message);
            if (diff.patch) fileBodySections = [extractPerFileDiffs(diff.patch, diff.files)];
          }
        }
        for (const df of deletedFiles) {
          fileBodySections.push(`${'='.repeat(60)}\nFILE: ${df.path}\nSTATUS: DELETED  (-${df.linesRemoved} lines)\n${'='.repeat(60)}\n[File was deleted in this subtask]`);
        }
        if (fileBodySections.length > 0) {
          const fullContent = fileBodySections.join('\n\n');
          atts.push({ id: `files-${Date.now()}`, label: `File Contents (${diff.files.length} files, ${(fullContent.length / 1024).toFixed(1)}KB)`, type: 'files', content: fullContent, meta: { count: diff.files.length, sizeKB: Math.round(fullContent.length / 1024) } });
        }
      }

      // 3. Unified diff patch
      if (diff.patch) {
        atts.push({ id: `diff-${Date.now()}`, label: `Subtask #${subtaskIndex} Diff (${(diff.patch.length / 1024).toFixed(1)}KB)`, type: 'diff', content: diff.patch, meta: { sizeKB: Math.round(diff.patch.length / 1024) } });
      }

      console.log('[askLlmForSubtask] attachments:', atts.length, atts.map(a => a.label));
      navigationStore.navigateToChat({ attachments: atts, timestamp: Date.now() });
    } finally {
      askLlmSubtaskLoading = false;
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
      <div class="flex items-center gap-2">
        <!-- Search bar (shared state, persists across views) -->
        <div class="relative">
          <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400 text-sm">🔍</span>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search prompts..."
            class="pl-8 pr-3 py-1.5 text-sm border-2 border-yellow-400 rounded-lg bg-yellow-50 focus:bg-white focus:border-yellow-500 focus:outline-none transition-colors w-48"
          />
        </div>
        <button
          onclick={() => { nukeError = null; showNukeConfirm = true; }}
          disabled={taskLoading || !selectedWorkspace?.active}
          class="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          title={selectedWorkspace?.active ? 'Delete all checkpoint history for this workspace' : 'Cannot nuke: Cline is actively running (.git_disabled)'}
        >
          🗑 Nuke
        </button>
        <button
          onclick={refreshTasks}
          disabled={taskLoading}
          class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {taskLoading ? 'Loading...' : 'Refresh'}
        </button>
      </div>
    </div>

    <!-- Nuke Confirmation Dialog -->
    {#if showNukeConfirm && selectedWorkspace}
      <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onclick={() => { if (!nukeLoading) { showNukeConfirm = false; nukeError = null; } }}>
        <div class="bg-white rounded-xl shadow-2xl max-w-md w-full mx-4 p-6" onclick={(e) => e.stopPropagation()}>
          <div class="flex items-start gap-3 mb-4">
            <div class="flex-shrink-0 w-10 h-10 rounded-full bg-red-100 flex items-center justify-center">
              <span class="text-xl">⚠️</span>
            </div>
            <div>
              <h3 class="text-lg font-semibold text-gray-900">Nuke workspace {selectedWorkspace.id}?</h3>
              <p class="text-sm text-gray-500 mt-1">This will delete <strong>ALL</strong> checkpoint history:</p>
            </div>
          </div>

          <div class="bg-red-50 border border-red-200 rounded-lg p-4 mb-4">
            <ul class="text-sm text-red-800 space-y-1">
              <li>• <strong>{tasks.length}</strong> task{tasks.length !== 1 ? 's' : ''}</li>
              <li>• <strong>{tasks.reduce((s, t) => s + t.steps, 0)}</strong> commits</li>
            </ul>
            <p class="text-sm text-red-700 mt-3">
              The workspace will be re-initialized empty.<br>
              Cline will create new checkpoints on the next task.
            </p>
            <p class="text-xs font-bold text-red-900 mt-3 uppercase tracking-wide">This cannot be undone.</p>
          </div>

          {#if nukeError}
            <div class="bg-red-50 border border-red-300 rounded-lg p-3 mb-4">
              <p class="text-sm text-red-700 font-medium">Error: {nukeError}</p>
            </div>
          {/if}

          <div class="flex items-center justify-end gap-3">
            <button
              onclick={() => { showNukeConfirm = false; nukeError = null; }}
              disabled={nukeLoading}
              class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 disabled:opacity-50 transition-colors"
            >
              Cancel
            </button>
            <button
              onclick={confirmNuke}
              disabled={nukeLoading}
              class="px-4 py-2 text-sm font-bold text-white bg-red-600 rounded-lg hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {nukeLoading ? 'Nuking...' : 'Nuke It'}
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- Git Discovery Command Toggle -->
    {#if selectedWorkspace}
      <div class="flex items-center gap-2 mb-3">
        <button
          class="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1"
          onclick={() => showDiscoveryGitCmd = !showDiscoveryGitCmd}
        >
          <span class="font-mono">{showDiscoveryGitCmd ? '▾' : '▸'}</span>
          <span>Git Command (1)</span>
        </button>
        {#if showDiscoveryGitCmd}
          <button
            onclick={() => { navigator.clipboard.writeText(`git --git-dir "${selectedWorkspace!.gitDir}" log --all "--pretty=format:%H|%s|%aI"`); gitCmdCopyLabel = '✓'; setTimeout(() => gitCmdCopyLabel = '📋', 1500); }}
            class="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
            title="Copy git command"
          >{gitCmdCopyLabel}</button>
        {/if}
      </div>
      {#if showDiscoveryGitCmd}
        <pre class="text-xs bg-gray-900 text-green-400 p-3 rounded mb-3 font-mono select-text whitespace-pre-wrap break-all">git --git-dir "{selectedWorkspace.gitDir}" log --all "--pretty=format:%H|%s|%aI"</pre>
      {/if}
    {/if}

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
      <!-- Pin Filter Toggle -->
      <div class="flex items-center gap-1 mb-3">
        <div class="inline-flex rounded-lg border border-gray-200 bg-white p-0.5 shadow-sm">
          <button
            onclick={() => pinStore.taskFilter = 'all'}
            class="px-3 py-1 text-xs font-medium rounded-md transition-colors {pinStore.taskFilter === 'all'
              ? 'bg-blue-600 text-white shadow-sm'
              : 'text-gray-600 hover:text-gray-800 hover:bg-gray-50'}"
          >
            All ({tasks.length})
          </button>
          <button
            onclick={() => pinStore.taskFilter = 'pinned'}
            class="px-3 py-1 text-xs font-medium rounded-md transition-colors {pinStore.taskFilter === 'pinned'
              ? 'bg-amber-500 text-white shadow-sm'
              : 'text-gray-600 hover:text-gray-800 hover:bg-gray-50'}"
          >
            📌 Pinned ({pinnedTaskCount})
          </button>
        </div>
        {#if pinStore.taskFilter === 'pinned' && pinnedTaskCount === 0}
          <span class="text-xs text-gray-400 ml-2 italic">No pinned tasks — click 📌 on any task to pin it</span>
        {/if}
      </div>

      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
        <table class="w-full text-sm">
          <thead class="bg-gray-50 border-b border-gray-200">
            <tr>
              <th class="w-8 px-4 py-3"></th>
              <th class="w-8 px-1 py-3"></th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">#</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Task ID</th>
              <th class="text-right px-4 py-3 font-medium text-gray-600">Steps</th>
              <th class="text-right px-4 py-3 font-medium text-gray-600">Files</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600">Last Changed</th>
              <th class="text-left px-4 py-3 font-medium text-gray-600"></th>
            </tr>
          </thead>
          <tbody>
            {#each filteredTasks as task, i}
              <tr
                class="border-b border-gray-100 hover:bg-blue-50 transition-colors cursor-pointer {i === 0 && expandedTaskId !== task.taskId ? 'bg-blue-50/60 ring-1 ring-inset ring-blue-200' : ''} {expandedTaskId === task.taskId ? 'bg-indigo-50' : ''}"
                onclick={() => toggleSteps(task)}
              >
                <td class="px-4 py-3 text-gray-400 text-xs text-center">
                  <span class="inline-block transition-transform {expandedTaskId === task.taskId ? 'rotate-90' : ''}">▸</span>
                </td>
                <td class="px-1 py-3 text-center">
                  <button
                    onclick={(e) => { e.stopPropagation(); pinStore.togglePin(task.taskId); }}
                    class="text-sm leading-none transition-all hover:scale-125 {pinStore.isPinned(task.taskId) ? 'opacity-100 grayscale-0' : 'opacity-30 grayscale hover:opacity-60'}"
                    title={pinStore.isPinned(task.taskId) ? 'Unpin this task' : 'Pin this task'}
                  >📌</button>
                </td>
                <td class="px-4 py-3 text-gray-400 font-mono text-xs">{i + 1}</td>
                <td class="px-4 py-3">
                  <div class="font-mono font-medium text-gray-900 text-xs">
                    {task.taskId}
                    {#if i === 0}
                      <span class="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
                    {/if}
                  </div>
                  {#if taskPromptMap.has(task.taskId)}
                    <div class="text-xs text-gray-500 truncate mt-0.5 max-w-[400px]" title={taskPromptMap.get(task.taskId)}>
                      {taskPromptMap.get(task.taskId)}
                    </div>
                  {:else if promptsLoading}
                    <div class="text-[10px] text-gray-400 mt-0.5 italic">loading prompt...</div>
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
                  <td colspan="8" class="p-0">
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
                            <button
                              onclick={(e) => askLlmForTask(task, e)}
                              disabled={askLlmLoading}
                              class="text-xs font-medium px-3 py-1 rounded bg-purple-600 hover:bg-purple-700 text-white transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
                              title="Send task artifacts (with full file contents) to Chat"
                            >
                              {askLlmLoading ? '⏳ Loading...' : '🤖 Ask LLM'}
                            </button>
                            {#if taskDiffResult.patch}
                              <button
                                onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(taskDiffResult!.patch); taskDiffCopyLabel = '✓ Copied!'; setTimeout(() => taskDiffCopyLabel = '📋 Copy Diff', 1500); }}
                                class="text-xs font-medium px-3 py-1 rounded bg-purple-600 hover:bg-purple-700 text-white transition-colors shadow-sm"
                              >
                                {taskDiffCopyLabel}
                              </button>
                            {/if}
                            <button
                              onclick={(e) => { e.stopPropagation(); taskDiffId = null; taskDiffResult = null; taskDiffError = null; saveExpansionState(); }}
                              class="text-xs font-bold px-2 py-1 rounded bg-gray-200 hover:bg-red-100 text-gray-600 hover:text-red-700 transition-colors"
                              title="Close diff"
                            >✕</button>
                          </div>
                        </div>
                        <!-- Git Commands Toggle -->
                        {#if taskDiffResult.gitCommands?.length}
                          <div class="flex items-center gap-2 mb-2">
                            <button
                              class="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1"
                              onclick={(e) => { e.stopPropagation(); showTaskGitCmds = !showTaskGitCmds; }}
                            >
                              <span class="font-mono">{showTaskGitCmds ? '▾' : '▸'}</span>
                              <span>Git Commands ({taskDiffResult.gitCommands.length})</span>
                            </button>
                            {#if showTaskGitCmds}
                              <button
                                onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(taskDiffResult!.gitCommands!.join('\n')); gitCmdCopyLabel = '✓'; setTimeout(() => gitCmdCopyLabel = '📋', 1500); }}
                                class="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
                                title="Copy git commands"
                              >{gitCmdCopyLabel}</button>
                            {/if}
                          </div>
                          {#if showTaskGitCmds}
                            <pre class="text-xs bg-gray-900 text-green-400 p-3 rounded mb-3 font-mono select-text whitespace-pre-wrap break-all">{taskDiffResult.gitCommands.join('\n')}</pre>
                          {/if}
                        {/if}
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
                  <td colspan="8" class="p-0">
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
                            onclick={(e) => { e.stopPropagation(); subtaskTaskId = null; subtasksData = null; subtasksError = null; subtaskDiffIndex = null; subtaskDiffResult = null; saveExpansionState(); }}
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
                                        <div class="flex items-center gap-1.5">
                                          <button
                                            onclick={(e) => askLlmForSubtask(si, e)}
                                            disabled={askLlmSubtaskLoading}
                                            class="text-[10px] font-medium px-2 py-0.5 rounded bg-teal-600 hover:bg-teal-700 text-white transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
                                            title="Send subtask artifacts (prompt + file contents + diff) to Chat"
                                          >
                                            {askLlmSubtaskLoading ? '⏳ Loading...' : '🤖 Ask LLM'}
                                          </button>
                                          {#if subtaskDiffResult.patch}
                                            <button
                                              onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(subtaskDiffResult!.patch); subtaskDiffCopyLabel = '✓ Copied'; setTimeout(() => subtaskDiffCopyLabel = '📋 Copy', 1500); }}
                                              class="text-[10px] font-medium px-2 py-0.5 rounded bg-teal-600 hover:bg-teal-700 text-white transition-colors"
                                            >
                                              {subtaskDiffCopyLabel}
                                            </button>
                                          {/if}
                                        </div>
                                      </div>
                                      <!-- Git Commands Toggle -->
                                      {#if subtaskDiffResult.gitCommands?.length}
                                        <div class="flex items-center gap-2 mb-2">
                                          <button
                                            class="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1"
                                            onclick={(e) => { e.stopPropagation(); showSubtaskGitCmds = !showSubtaskGitCmds; }}
                                          >
                                            <span class="font-mono">{showSubtaskGitCmds ? '▾' : '▸'}</span>
                                            <span>Git Commands ({subtaskDiffResult.gitCommands.length})</span>
                                          </button>
                                          {#if showSubtaskGitCmds}
                                            <button
                                              onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(subtaskDiffResult!.gitCommands!.join('\n')); gitCmdCopyLabel = '✓'; setTimeout(() => gitCmdCopyLabel = '📋', 1500); }}
                                              class="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
                                              title="Copy git commands"
                                            >{gitCmdCopyLabel}</button>
                                          {/if}
                                        </div>
                                        {#if showSubtaskGitCmds}
                                          <pre class="text-xs bg-gray-900 text-green-400 p-3 rounded mb-3 font-mono select-text whitespace-pre-wrap break-all">{subtaskDiffResult.gitCommands.join('\n')}</pre>
                                        {/if}
                                      {/if}
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
                  <td colspan="8" class="p-0">
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
                                        <!-- Git Commands Toggle -->
                                        {#if diffResult.gitCommands?.length}
                                          <div class="flex items-center gap-2 mb-2">
                                            <button
                                              class="text-xs text-gray-400 hover:text-gray-600 flex items-center gap-1"
                                              onclick={(e) => { e.stopPropagation(); showStepGitCmds = !showStepGitCmds; }}
                                            >
                                              <span class="font-mono">{showStepGitCmds ? '▾' : '▸'}</span>
                                              <span>Git Commands ({diffResult.gitCommands.length})</span>
                                            </button>
                                            {#if showStepGitCmds}
                                              <button
                                                onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(diffResult!.gitCommands!.join('\n')); gitCmdCopyLabel = '✓'; setTimeout(() => gitCmdCopyLabel = '📋', 1500); }}
                                                class="text-[10px] px-1.5 py-0.5 rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
                                                title="Copy git commands"
                                              >{gitCmdCopyLabel}</button>
                                            {/if}
                                          </div>
                                          {#if showStepGitCmds}
                                            <pre class="text-xs bg-gray-900 text-green-400 p-3 rounded mb-3 font-mono select-text whitespace-pre-wrap break-all">{diffResult.gitCommands.join('\n')}</pre>
                                          {/if}
                                        {/if}
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
        {#if pinStore.taskFilter === 'pinned'}
          📌 {filteredTasks.length} pinned task{filteredTasks.length !== 1 ? 's' : ''}
        {:else if searchQuery.trim()}
          Showing {filteredTasks.length} of {tasks.length} task{tasks.length !== 1 ? 's' : ''}
          {#if pinnedTaskCount > 0}· 📌 {pinnedTaskCount} pinned{/if}
        {:else}
          Total: {tasks.length} task{tasks.length !== 1 ? 's' : ''},
          {tasks.reduce((s, t) => s + t.steps, 0)} steps,
          {tasks.reduce((s, t) => s + t.filesChanged, 0)} files touched
          {#if pinnedTaskCount > 0}· 📌 {pinnedTaskCount} pinned{/if}
        {/if}
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
      <div class="flex items-center gap-2">
        <!-- Search bar (shared state, persists across views) -->
        <div class="relative">
          <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400 text-sm">🔍</span>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search prompts..."
            class="pl-8 pr-3 py-1.5 text-sm border-2 border-yellow-400 rounded-lg bg-yellow-50 focus:bg-white focus:border-yellow-500 focus:outline-none transition-colors w-48"
          />
        </div>
        <button
          onclick={() => loadWorkspaces(true)}
          disabled={wsLoading}
          class="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {wsLoading ? 'Scanning...' : 'Refresh'}
        </button>
      </div>
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
      <!-- Workspace Pin Filter Toggle -->
      <div class="flex items-center gap-1 mb-3">
        <div class="inline-flex rounded-lg border border-gray-200 bg-white p-0.5 shadow-sm">
          <button
            onclick={() => pinStore.wsFilter = 'all'}
            class="px-3 py-1 text-xs font-medium rounded-md transition-colors {pinStore.wsFilter === 'all'
              ? 'bg-blue-600 text-white shadow-sm'
              : 'text-gray-600 hover:text-gray-800 hover:bg-gray-50'}"
          >
            All ({workspaces.length})
          </button>
          <button
            onclick={() => pinStore.wsFilter = 'pinned'}
            class="px-3 py-1 text-xs font-medium rounded-md transition-colors {pinStore.wsFilter === 'pinned'
              ? 'bg-amber-500 text-white shadow-sm'
              : 'text-gray-600 hover:text-gray-800 hover:bg-gray-50'}"
          >
            📌 Pinned ({pinnedWsCount})
          </button>
        </div>
        {#if pinStore.wsFilter === 'pinned' && pinnedWsCount === 0}
          <span class="text-xs text-gray-400 ml-2 italic">No pinned workspaces — click 📌 on any workspace to pin it</span>
        {/if}
      </div>

      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden shadow-sm">
        <table class="w-full text-sm">
          <thead class="bg-gray-50 border-b border-gray-200">
            <tr>
              <th class="w-8 px-1 py-3"></th>
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
            {#each filteredWorkspaces as ws, i}
              <tr
                class="border-b border-gray-100 hover:bg-blue-50 transition-colors cursor-pointer {i === 0 ? 'bg-blue-50/60 ring-1 ring-inset ring-blue-200' : ''}"
                onclick={() => selectWorkspace(ws)}
              >
                <td class="px-1 py-3 text-center">
                  <button
                    onclick={(e) => { e.stopPropagation(); pinStore.toggleWorkspacePin(ws.id); }}
                    class="text-sm leading-none transition-all hover:scale-125 {pinStore.isWorkspacePinned(ws.id) ? 'opacity-100 grayscale-0' : 'opacity-30 grayscale hover:opacity-60'}"
                    title={pinStore.isWorkspacePinned(ws.id) ? 'Unpin this workspace' : 'Pin this workspace'}
                  >📌</button>
                </td>
                <td class="px-4 py-3 text-gray-400 font-mono text-xs">{i + 1}</td>
                <td class="px-4 py-3">
                  <div class="font-mono font-medium text-gray-900">
                    {ws.id}
                    {#if i === 0}
                      <span class="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-600 text-white uppercase tracking-wide">Latest</span>
                    {/if}
                  </div>
                  {#if workspacePromptMap.has(ws.id)}
                    <div class="text-xs text-gray-500 truncate mt-0.5 max-w-[350px]" title={workspacePromptMap.get(ws.id)}>
                      📝 {workspacePromptMap.get(ws.id)}
                    </div>
                  {:else if wsPromptsLoading}
                    <div class="text-[10px] text-gray-400 mt-0.5 italic">loading latest task...</div>
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
        {#if pinStore.wsFilter === 'pinned'}
          📌 {filteredWorkspaces.length} pinned workspace{filteredWorkspaces.length !== 1 ? 's' : ''},
          {filteredWorkspaces.reduce((sum, ws) => sum + ws.taskCount, 0)} tasks
        {:else if searchQuery.trim()}
          Showing {filteredWorkspaces.length} of {workspaces.length} workspace{workspaces.length !== 1 ? 's' : ''},
          {filteredWorkspaces.reduce((sum, ws) => sum + ws.taskCount, 0)} tasks
          {#if pinnedWsCount > 0}· 📌 {pinnedWsCount} pinned{/if}
        {:else}
          Total: {workspaces.length} workspace{workspaces.length !== 1 ? 's' : ''},
          {workspaces.reduce((sum, ws) => sum + ws.taskCount, 0)} tasks
          {#if pinnedWsCount > 0}· 📌 {pinnedWsCount} pinned{/if}
        {/if}
      </div>
    {/if}
  {/if}
</div>

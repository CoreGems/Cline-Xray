/**
 * Pin Store — persists pinned task and workspace IDs to localStorage
 *
 * Tasks are identified by their taskId (unique across workspaces).
 * Workspaces are identified by their workspace id string.
 * Provides reactive state via Svelte 5 runes.
 */

const TASK_STORAGE_KEY = 'xray-pinned-tasks';
const WS_STORAGE_KEY = 'xray-pinned-workspaces';
const CHAT_STORAGE_KEY = 'xray-pinned-chats';
const FILTER_STORAGE_KEY = 'xray-pin-filters';

function loadSet(key: string): Set<string> {
  try {
    const raw = localStorage.getItem(key);
    if (raw) {
      const arr = JSON.parse(raw);
      if (Array.isArray(arr)) return new Set(arr);
    }
  } catch {
    // corrupt data — ignore
  }
  return new Set();
}

function saveSet(key: string, pins: Set<string>) {
  try {
    localStorage.setItem(key, JSON.stringify([...pins]));
  } catch {
    // storage full or unavailable — silently ignore
  }
}

let _pinnedTasks = $state<Set<string>>(loadSet(TASK_STORAGE_KEY));
let _pinnedWorkspaces = $state<Set<string>>(loadSet(WS_STORAGE_KEY));
let _pinnedChats = $state<Set<string>>(loadSet(CHAT_STORAGE_KEY));

// ── Filter state (persisted) ────────────────────────────────────────────────
type FilterMode = 'all' | 'pinned';

function loadFilters(): { task: FilterMode; ws: FilterMode } {
  try {
    const raw = localStorage.getItem(FILTER_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        task: parsed.task === 'pinned' ? 'pinned' : 'all',
        ws: parsed.ws === 'pinned' ? 'pinned' : 'all',
      };
    }
  } catch { /* ignore */ }
  return { task: 'all', ws: 'all' };
}

function saveFilters(task: FilterMode, ws: FilterMode) {
  try {
    localStorage.setItem(FILTER_STORAGE_KEY, JSON.stringify({ task, ws }));
  } catch { /* ignore */ }
}

const initialFilters = loadFilters();
let _taskFilter = $state<FilterMode>(initialFilters.task);
let _wsFilter = $state<FilterMode>(initialFilters.ws);

export const pinStore = {
  // ── Task pins ─────────────────────────────────────────────────────────────

  /** Check if a task is pinned */
  isPinned(taskId: string): boolean {
    return _pinnedTasks.has(taskId);
  },

  /** Toggle pin state for a task */
  togglePin(taskId: string) {
    const newSet = new Set(_pinnedTasks);
    if (newSet.has(taskId)) {
      newSet.delete(taskId);
    } else {
      newSet.add(taskId);
    }
    _pinnedTasks = newSet;
    saveSet(TASK_STORAGE_KEY, _pinnedTasks);
  },

  /** Pin a task (no-op if already pinned) */
  pin(taskId: string) {
    if (!_pinnedTasks.has(taskId)) {
      const newSet = new Set(_pinnedTasks);
      newSet.add(taskId);
      _pinnedTasks = newSet;
      saveSet(TASK_STORAGE_KEY, _pinnedTasks);
    }
  },

  /** Unpin a task (no-op if not pinned) */
  unpin(taskId: string) {
    if (_pinnedTasks.has(taskId)) {
      const newSet = new Set(_pinnedTasks);
      newSet.delete(taskId);
      _pinnedTasks = newSet;
      saveSet(TASK_STORAGE_KEY, _pinnedTasks);
    }
  },

  /** Number of pinned tasks */
  get pinnedCount(): number {
    return _pinnedTasks.size;
  },

  /** The underlying set (for derived computations) */
  get pinnedSet(): Set<string> {
    return _pinnedTasks;
  },

  // ── Workspace pins ────────────────────────────────────────────────────────

  /** Check if a workspace is pinned */
  isWorkspacePinned(workspaceId: string): boolean {
    return _pinnedWorkspaces.has(workspaceId);
  },

  /** Toggle pin state for a workspace */
  toggleWorkspacePin(workspaceId: string) {
    const newSet = new Set(_pinnedWorkspaces);
    if (newSet.has(workspaceId)) {
      newSet.delete(workspaceId);
    } else {
      newSet.add(workspaceId);
    }
    _pinnedWorkspaces = newSet;
    saveSet(WS_STORAGE_KEY, _pinnedWorkspaces);
  },

  /** Number of pinned workspaces */
  get pinnedWorkspaceCount(): number {
    return _pinnedWorkspaces.size;
  },

  /** The underlying workspace pin set (for derived computations) */
  get pinnedWorkspaceSet(): Set<string> {
    return _pinnedWorkspaces;
  },

  // ── Chat pins ─────────────────────────────────────────────────────────────

  /** Check if a chat session is pinned */
  isChatPinned(chatId: string): boolean {
    return _pinnedChats.has(chatId);
  },

  /** Toggle pin state for a chat session */
  toggleChatPin(chatId: string) {
    const newSet = new Set(_pinnedChats);
    if (newSet.has(chatId)) {
      newSet.delete(chatId);
    } else {
      newSet.add(chatId);
    }
    _pinnedChats = newSet;
    saveSet(CHAT_STORAGE_KEY, _pinnedChats);
  },

  /** Number of pinned chats */
  get pinnedChatCount(): number {
    return _pinnedChats.size;
  },

  /** The underlying chat pin set */
  get pinnedChatSet(): Set<string> {
    return _pinnedChats;
  },

  // ── Filter state (persisted across tabs & restarts) ───────────────────────

  get taskFilter(): FilterMode { return _taskFilter; },
  set taskFilter(v: FilterMode) { _taskFilter = v; saveFilters(_taskFilter, _wsFilter); },

  get wsFilter(): FilterMode { return _wsFilter; },
  set wsFilter(v: FilterMode) { _wsFilter = v; saveFilters(_taskFilter, _wsFilter); },
};

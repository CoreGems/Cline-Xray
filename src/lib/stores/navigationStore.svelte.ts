/**
 * Navigation Store — global tab navigation + cross-tab payload passing
 * 
 * Provides:
 * 1. Programmatic tab switching from any component
 * 2. Pending chat payload delivery (e.g., from LatestSubtab → ChatSubtab)
 * 3. Persistent subtab state across app restarts (localStorage)
 */

import type { TabId } from '../tabs';
import type { AgentSubTab, ChatAttachment } from '../tabs/agent/types';
import type { ChangesSubTab } from '../tabs/changes/types';
import type { HistorySubTab } from '../tabs/history/types';
import type { ActivitySubTab } from '../tabs/activity/types';
import type { ApiSubTab } from '../tabs/api/types';
import type { MyJirasSubTab } from '../tabs/my-jiras/types';

/** Persisted expansion state for the Changes → Tasks subtab */
export interface ChangesTasksState {
  /** Which workspace is drilled into (null = workspace list view) */
  selectedWorkspaceId: string | null;
  /** Which task row is expanded (steps visible) */
  expandedTaskId: string | null;
  /** Which task has Full Diff open */
  taskDiffId: string | null;
  /** Which task has Subtasks panel open */
  subtaskTaskId: string | null;
  /** Which subtask's diff is shown inside the Subtasks panel */
  subtaskDiffIndex: number | null;
}

const DEFAULT_CHANGES_TASKS: ChangesTasksState = {
  selectedWorkspaceId: null,
  expandedTaskId: null,
  taskDiffId: null,
  subtaskTaskId: null,
  subtaskDiffIndex: null,
};

/** Payload delivered to ChatSubtab when navigating via "Ask LLM" */
export interface PendingChatPayload {
  attachments: ChatAttachment[];
  initialMessage?: string;
  timestamp: number;
}

// ── localStorage persistence ────────────────────────────────────────────────

const STORAGE_KEY = 'xray-nav-state';

interface PersistedNavState {
  activeTab: TabId;
  subtabs: {
    agent: AgentSubTab;
    changes: ChangesSubTab;
    history: HistorySubTab;
    activity: ActivitySubTab;
    api: ApiSubTab | 'Console';
    'my-jiras': MyJirasSubTab;
  };
  /** Expansion state for Changes → Tasks subtab */
  changesTasksState?: ChangesTasksState;
}

/** Default subtab values (used when nothing is persisted) */
const DEFAULTS: PersistedNavState = {
  activeTab: 'my-jiras',
  subtabs: {
    agent: 'Chat',
    changes: 'Latest',
    history: 'Latest',
    activity: 'REST',
    api: 'REST',
    'my-jiras': 'List',
  },
};

function loadState(): PersistedNavState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<PersistedNavState>;
      return {
        activeTab: parsed.activeTab ?? DEFAULTS.activeTab,
        subtabs: { ...DEFAULTS.subtabs, ...parsed.subtabs },
        changesTasksState: parsed.changesTasksState
          ? { ...DEFAULT_CHANGES_TASKS, ...parsed.changesTasksState }
          : { ...DEFAULT_CHANGES_TASKS },
      };
    }
  } catch {
    // corrupt data — ignore
  }
  return { ...DEFAULTS, subtabs: { ...DEFAULTS.subtabs }, changesTasksState: { ...DEFAULT_CHANGES_TASKS } };
}

function saveState(state: PersistedNavState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // storage full or unavailable — silently ignore
  }
}

// ── Reactive state (Svelte 5 runes — module-level singleton) ────────────────

const initial = loadState();

let _activeTab = $state<TabId>(initial.activeTab);
let _activeAgentSubTab = $state<AgentSubTab>(initial.subtabs.agent);
let _activeChangesSubTab = $state<ChangesSubTab>(initial.subtabs.changes);
let _activeHistorySubTab = $state<HistorySubTab>(initial.subtabs.history);
let _activeActivitySubTab = $state<ActivitySubTab>(initial.subtabs.activity);
let _activeApiSubTab = $state<ApiSubTab | 'Console'>(initial.subtabs.api);
let _activeMyJirasSubTab = $state<MyJirasSubTab>(initial.subtabs['my-jiras']);
let _pendingPayload = $state<PendingChatPayload | null>(null);
let _changesTasksState = $state<ChangesTasksState>(initial.changesTasksState ?? { ...DEFAULT_CHANGES_TASKS });

/** Persist current state to localStorage */
function persist() {
  saveState({
    activeTab: _activeTab,
    subtabs: {
      agent: _activeAgentSubTab,
      changes: _activeChangesSubTab,
      history: _activeHistorySubTab,
      activity: _activeActivitySubTab,
      api: _activeApiSubTab,
      'my-jiras': _activeMyJirasSubTab,
    },
    changesTasksState: _changesTasksState,
  });
}

export const navigationStore = {
  // ── Main tab ──────────────────────────────────────────────────────────────
  get activeTab() { return _activeTab; },
  set activeTab(v: TabId) { _activeTab = v; persist(); },

  // ── Agent subtab ──────────────────────────────────────────────────────────
  get activeAgentSubTab() { return _activeAgentSubTab; },
  set activeAgentSubTab(v: AgentSubTab) { _activeAgentSubTab = v; persist(); },

  // ── Changes subtab ────────────────────────────────────────────────────────
  get activeChangesSubTab() { return _activeChangesSubTab; },
  set activeChangesSubTab(v: ChangesSubTab) { _activeChangesSubTab = v; persist(); },

  // ── History subtab ────────────────────────────────────────────────────────
  get activeHistorySubTab() { return _activeHistorySubTab; },
  set activeHistorySubTab(v: HistorySubTab) { _activeHistorySubTab = v; persist(); },

  // ── Activity subtab ───────────────────────────────────────────────────────
  get activeActivitySubTab() { return _activeActivitySubTab; },
  set activeActivitySubTab(v: ActivitySubTab) { _activeActivitySubTab = v; persist(); },

  // ── API subtab ────────────────────────────────────────────────────────────
  get activeApiSubTab() { return _activeApiSubTab; },
  set activeApiSubTab(v: ApiSubTab | 'Console') { _activeApiSubTab = v; persist(); },

  // ── My Jiras subtab ───────────────────────────────────────────────────────
  get activeMyJirasSubTab() { return _activeMyJirasSubTab; },
  set activeMyJirasSubTab(v: MyJirasSubTab) { _activeMyJirasSubTab = v; persist(); },

  // ── Chat payload (transient — not persisted) ─────────────────────────────

  /** Navigate to Agent tab and deliver a payload */
  navigateToChat(payload: PendingChatPayload) {
    _pendingPayload = payload;
    _activeTab = 'agent';
    persist();
  },

  /** Called by ChatSubtab to pick up the payload (consumes it — one-shot) */
  consumeChatPayload(): PendingChatPayload | null {
    const p = _pendingPayload;
    _pendingPayload = null;
    return p;
  },

  /** Check if there's a pending payload without consuming */
  get hasPendingPayload() { return _pendingPayload !== null; },

  // ── Changes → Tasks expansion state (persisted) ──────────────────────────

  /** Get the current expansion state snapshot */
  get changesTasksState(): ChangesTasksState { return _changesTasksState; },

  /** Replace the entire expansion state and persist */
  set changesTasksState(v: ChangesTasksState) { _changesTasksState = v; persist(); },

  /** Merge a partial update into the expansion state and persist */
  updateChangesTasksState(patch: Partial<ChangesTasksState>) {
    _changesTasksState = { ..._changesTasksState, ...patch };
    persist();
  },
};

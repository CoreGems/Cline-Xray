// Changes Tab Module
// Re-exports for the Changes tab functionality

// Main component
export { default as ChangesTab } from './ChangesTab.svelte';

// Subtab components
export { default as TaskListSubtab } from './TaskListSubtab.svelte';
export { default as LatestSubtab } from './LatestSubtab.svelte';

// Types
export type {
  WorkspaceInfo,
  WorkspacesResponse,
  LatestResponse,
  SubtaskSummaryItem,
  ChangesSubTab,
  SubTabDefinition
} from './types';

// API functions
export { fetchWorkspaces, fetchLatest } from './api';

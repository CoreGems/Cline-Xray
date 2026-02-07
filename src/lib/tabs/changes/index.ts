// Changes Tab Module
// Re-exports for the Changes tab functionality

// Main component
export { default as ChangesTab } from './ChangesTab.svelte';

// Subtab components
export { default as TaskListSubtab } from './TaskListSubtab.svelte';

// Types
export type {
  WorkspaceInfo,
  WorkspacesResponse,
  ChangesSubTab,
  SubTabDefinition
} from './types';

// API functions
export { fetchWorkspaces } from './api';

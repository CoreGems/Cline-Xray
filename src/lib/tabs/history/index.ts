// History Tab Module
// Re-exports for the History tab functionality

// Main component
export { default as HistoryTab } from './HistoryTab.svelte';

// Subtab components
export { default as HistoryTaskListSubtab } from './HistoryTaskListSubtab.svelte';
export { default as TaskDetailView } from './TaskDetailView.svelte';

// Types
export type {
  TaskHistorySummary,
  TaskHistoryListResponse,
  TaskDetailResponse,
  ConversationMessage,
  ContentBlockSummary,
  ToolCallDetail,
  FileInContextDetail,
  ModelUsageDetail,
  EnvironmentDetail,
  HistorySubTab,
  SubTabDefinition
} from './types';

// API functions
export { fetchHistoryTasks, fetchTaskDetail } from './api';

// API Tab Module
// Re-exports for the API tab functionality

// Main component
export { default as ApiTab } from './ApiTab.svelte';

// Subtab components
export { default as RESTSubtab } from './RESTSubtab.svelte';

// Types
export type {
  HttpMethod,
  ApiEndpoint,
  ApiSubTab,
  SubTabDefinition
} from './types';

// Endpoint data
export { endpoints } from './endpoints';

// Utility functions
export { getMethodColor, getTagColor } from './utils';

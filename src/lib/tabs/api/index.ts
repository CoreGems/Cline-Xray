// API Tab Module
// Re-exports for the API tab functionality

// Main component
export { default as ApiTab } from './ApiTab.svelte';

// Subtab components
export { default as RESTSubtab } from './RESTSubtab.svelte';
export { default as ToolsSubtab } from './ToolsSubtab.svelte';
export { default as ToolsConsoleSubtab } from './ToolsConsoleSubtab.svelte';

// Types
export type {
  HttpMethod,
  ApiType,
  ApiEndpoint,
  ApiSubTab,
  SubTabDefinition
} from './types';

// Utility functions - Endpoint fetching (single source of truth from OpenAPI)
export {
  fetchEndpointsFromOpenApi,
  clearEndpointCache,
  getMethodColor,
  getTagColor,
  fetchToolsFromOpenApi
} from './utils';

// Agent tool types
export type { AgentTool } from './utils';

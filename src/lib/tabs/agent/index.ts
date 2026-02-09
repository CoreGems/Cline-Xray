// Agent Tab Module
// Re-exports for the Agent tab functionality

// Main component
export { default as AgentTab } from './AgentTab.svelte';

// Subtab components
export { default as ChatSubtab } from './ChatSubtab.svelte';
export { default as AgentChatSubtab } from './AgentChatSubtab.svelte';

// Types
export type {
  ChatAttachment,
  ChatMessage,
  ChatSession,
  AgentChatMessage,
  AgentChatSession,
  ChatRequest,
  ChatResponse,
  ApiInfo,
  AgentSubTab,
  SubTabDefinition
} from './types';

// API functions
export { getApiInfo, sendChatMessage } from './api';

// Tab components index
// Add new tab exports here as they are created

export { default as MyJirasTab } from './MyJirasTab.svelte';
export { default as ActivityTab } from './ActivityTab.svelte';
export { default as ApiTab } from './ApiTab.svelte';
export { default as AgentTab } from './AgentTab.svelte';

// Tab definitions for navigation
export type TabId = 'my-jiras' | 'activity' | 'api' | 'agent';

export interface TabDefinition {
  id: TabId;
  label: string;
  icon?: string;
}

export const tabs: TabDefinition[] = [
  { id: 'my-jiras', label: 'My Jiras' },
  { id: 'activity', label: 'Activity' },
  { id: 'api', label: 'API' },
  { id: 'agent', label: 'Agent' },
];

// Tab components index
// Add new tab exports here as they are created

// My Jiras tab is now a modular directory
export { MyJirasTab } from './my-jiras';

// Activity tab is now a modular directory
export { ActivityTab } from './activity';

// API tab is now a modular directory
export { ApiTab } from './api';

// Agent tab is now a modular directory
export { AgentTab } from './agent';

// Changes tab is now a modular directory
export { ChangesTab } from './changes';

// Tab definitions for navigation
export type TabId = 'my-jiras' | 'activity' | 'api' | 'agent' | 'changes';

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
  { id: 'changes', label: 'Changes' },
];

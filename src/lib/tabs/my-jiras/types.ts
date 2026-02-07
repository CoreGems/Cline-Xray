// My Jiras Tab Types

// Re-export types from main types
export type { IssueDetails, IssueSummary } from '../../../types';

/** Available subtabs in the My Jiras tab */
export type MyJirasSubTab = 'List';

/** Subtab definition */
export interface SubTabDefinition {
  id: MyJirasSubTab;
  label: string;
}

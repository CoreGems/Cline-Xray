// Activity Tab Types

// Re-export AccessLogEntry and InferenceLogEntry from main types
export type { AccessLogEntry, InferenceLogEntry } from '../../../types';

/** Available subtabs in the Activity tab */
export type ActivitySubTab = 'REST' | 'Inference';

/** Subtab definition */
export interface SubTabDefinition {
  id: ActivitySubTab;
  label: string;
}

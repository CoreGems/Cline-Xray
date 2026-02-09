/**
 * Navigation Store — global tab navigation + cross-tab payload passing
 * 
 * Provides:
 * 1. Programmatic tab switching from any component
 * 2. Pending chat payload delivery (e.g., from LatestSubtab → ChatSubtab)
 */

import type { TabId } from '../tabs';
import type { AgentSubTab, ChatAttachment } from '../tabs/agent/types';

/** Payload delivered to ChatSubtab when navigating via "Ask LLM" */
export interface PendingChatPayload {
  attachments: ChatAttachment[];
  initialMessage?: string;
  timestamp: number;
}

// Reactive state (Svelte 5 runes — module-level singleton)
let _activeTab = $state<TabId>('my-jiras');
let _activeAgentSubTab = $state<AgentSubTab>('Agent Chat');
let _pendingPayload = $state<PendingChatPayload | null>(null);

export const navigationStore = {
  get activeTab() { return _activeTab; },
  set activeTab(v: TabId) { _activeTab = v; },

  get activeAgentSubTab() { return _activeAgentSubTab; },
  set activeAgentSubTab(v: AgentSubTab) { _activeAgentSubTab = v; },

  /** Navigate to Agent → Chat and deliver a payload */
  navigateToChat(payload: PendingChatPayload) {
    _pendingPayload = payload;
    _activeAgentSubTab = 'Chat';
    _activeTab = 'agent';
  },

  /** Called by ChatSubtab to pick up the payload (consumes it — one-shot) */
  consumeChatPayload(): PendingChatPayload | null {
    const p = _pendingPayload;
    _pendingPayload = null;
    return p;
  },

  /** Check if there's a pending payload without consuming */
  get hasPendingPayload() { return _pendingPayload !== null; }
};

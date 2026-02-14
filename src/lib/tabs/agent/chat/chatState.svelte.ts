/**
 * Chat State — module-level singleton (Svelte 5 runes)
 * 
 * Shared reactive state for the Chat subtab:
 * - Chat sessions & messages
 * - Agent definitions
 * - Attachments, drafts, UI preferences
 */

import { untrack } from "svelte";
import { marked } from "marked";
import { getApiInfo, getAgentModels } from "../api";
import { vendorRegistry } from "../vendors";
import { navigationStore } from "../../../stores/navigationStore.svelte";
import { pinStore } from "../../../stores/pinStore.svelte";
import type { ChatMessage, ChatSession, ChatAttachment, AgentDefinition, AgentModel } from "../types";
import type { AgentSettings } from "../../../../types";
import { DEFAULT_AGENT_SETTINGS } from "../../../../types";

// Configure marked
marked.setOptions({ breaks: true, gfm: true });

/** Render markdown to HTML (synchronous) */
export function renderMarkdown(content: string): string {
  return marked.parse(content) as string;
}

// ── Storage Keys ────────────────────────────────────────────────────────────

const STORAGE_KEY = 'agent-chat-sessions';
const MODEL_STORAGE_KEY = 'agent-chat-selected-model';
const HISTORY_ENABLED_KEY = 'agent-chat-history-enabled';
const FONT_SIZE_KEY = 'agent-chat-font-size';
const AGENTS_STORAGE_KEY = 'agent-chat-agents';
const AGENT_SETTINGS_KEY = 'agent-settings';
const DRAFTS_STORAGE_KEY = 'agent-chat-drafts';

export { MODEL_STORAGE_KEY };

// ── Font Size ───────────────────────────────────────────────────────────────

export const FONT_SIZE_STEPS = [12, 13, 14, 15, 16, 18, 20];
const FONT_SIZE_DEFAULT = 14;

// ── Agent Colors ────────────────────────────────────────────────────────────

export const agentColors = [
  '#10B981', '#3B82F6', '#8B5CF6', '#F59E0B',
  '#EF4444', '#EC4899', '#06B6D4', '#6366F1',
];

// ── Draft persistence ───────────────────────────────────────────────────────

function loadDrafts(): Record<string, string> {
  try {
    const stored = localStorage.getItem(DRAFTS_STORAGE_KEY);
    return stored ? JSON.parse(stored) : {};
  } catch { return {}; }
}

function saveDraftToStorage(sessionId: string, text: string) {
  const drafts = loadDrafts();
  if (text.trim()) {
    drafts[sessionId] = text;
  } else {
    delete drafts[sessionId];
  }
  try { localStorage.setItem(DRAFTS_STORAGE_KEY, JSON.stringify(drafts)); } catch {}
}

function getDraftFromStorage(sessionId: string): string {
  return loadDrafts()[sessionId] ?? '';
}

function deleteDraftFromStorage(sessionId: string) {
  const drafts = loadDrafts();
  delete drafts[sessionId];
  try { localStorage.setItem(DRAFTS_STORAGE_KEY, JSON.stringify(drafts)); } catch {}
}

// ── Reactive State ──────────────────────────────────────────────────────────

// Chat sessions
let sessions = $state<ChatSession[]>([]);
let activeSessionId = $state<string | null>(null);
let sidebarCollapsed = $state(false);
let initialized = $state(false);

// Current chat
let messages = $state<ChatMessage[]>([]);
let inputMessage = $state('');
let isLoading = $state(false);
let error = $state<string | null>(null);

// Preferences
let chatHistoryEnabled = $state(localStorage.getItem(HISTORY_ENABLED_KEY) !== 'false');
let chatFontSize = $state((() => {
  const stored = localStorage.getItem(FONT_SIZE_KEY);
  if (stored) { const n = parseInt(stored, 10); if (FONT_SIZE_STEPS.includes(n)) return n; }
  return FONT_SIZE_DEFAULT;
})());

// Model
let selectedModel = $state(localStorage.getItem(MODEL_STORAGE_KEY) || vendorRegistry.getDefaultModel());

// Attachments
let attachments = $state<ChatAttachment[]>([]);
let previewAttachmentId = $state<string | null>(null);

// Agents
let agents = $state<AgentDefinition[]>([]);
let selectedAgentId = $state<string | null>(null);
let showAddAgentModal = $state(false);
let editingAgent = $state<AgentDefinition | null>(null);

// Agent form
let newAgentName = $state('');
let newAgentDescription = $state('');
let newAgentSystemPrompt = $state('');
let newAgentColor = $state('#10B981');
let newAgentModel = $state('');
let activeModalTab = $state<'basic' | 'advanced'>('basic');

// Agent models (for modal)
let agentAvailableModels = $state<AgentModel[]>([]);
let isLoadingAgentModels = $state(false);
let agentModelsError = $state<string | null>(null);

// UI state
let expandedUserMsgs = $state<Set<number>>(new Set());
let copiedId = $state<string | null>(null);
let copiedAll = $state(false);
let copiedMsgIdx = $state<number | null>(null);

// ── Derived State ───────────────────────────────────────────────────────────

let activeVendor = $derived(vendorRegistry.getVendorForModel(selectedModel));
let activeBranding = $derived(activeVendor?.branding ?? {
  primaryColor: '#4285F4', icon: '✦', poweredBy: 'Powered by AI', emptyStateTitle: 'Chat with AI',
});
let pinnedSessions = $derived(sessions.filter(s => pinStore.isChatPinned(s.id)));
let unpinnedSessions = $derived(sessions.filter(s => !pinStore.isChatPinned(s.id)));

// ── Helpers ─────────────────────────────────────────────────────────────────

function generateId(): string {
  return `chat-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

function saveSessions() {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions)); }
  catch (e) { console.error('Failed to save chat sessions:', e); }
}

function saveAgents() {
  try { localStorage.setItem(AGENTS_STORAGE_KEY, JSON.stringify(agents)); }
  catch (e) { console.error('Failed to save agents:', e); }
}

function updateCurrentSession() {
  if (!activeSessionId) return;
  const sessionIndex = sessions.findIndex(s => s.id === activeSessionId);
  if (sessionIndex >= 0) {
    const session = sessions[sessionIndex];
    session.messages = [...messages];
    session.attachments = attachments.length > 0 ? [...attachments] : undefined;
    session.updatedAt = Date.now();
    if (session.title === 'New Chat' && messages.length > 0) {
      const firstUserMsg = messages.find(m => m.role === 'user');
      if (firstUserMsg) {
        const titleText = firstUserMsg.displayContent ?? firstUserMsg.content;
        session.title = titleText.substring(0, 40) + (titleText.length > 40 ? '...' : '');
      }
    }
    sessions = [session, ...sessions.filter(s => s.id !== activeSessionId)];
    saveSessions();
  }
}

function updateCurrentSessionAttachments() {
  if (!activeSessionId) return;
  const session = sessions.find(s => s.id === activeSessionId);
  if (session) {
    session.attachments = attachments.length > 0 ? [...attachments] : undefined;
    session.updatedAt = Date.now();
    saveSessions();
  }
}

function buildAttachmentContext(): string {
  if (attachments.length === 0) return '';
  return attachments.map(a => `--- ${a.label} ---\n${a.content}`).join('\n\n') + '\n\n---\nUser message:\n';
}

function getAgentSettings(): AgentSettings {
  try {
    const stored = localStorage.getItem(AGENT_SETTINGS_KEY);
    if (stored) return JSON.parse(stored) as AgentSettings;
  } catch {}
  return DEFAULT_AGENT_SETTINGS;
}

function filterModels(models: AgentModel[]): AgentModel[] {
  const settings = getAgentSettings();
  if (!settings.filterTextGenerationOnly) return models;
  return models.filter(model => {
    if (settings.requiredMethods && settings.requiredMethods.length > 0) {
      const supported = model.supportedGenerationMethods || [];
      if (!settings.requiredMethods.every(m => supported.includes(m))) return false;
    }
    if (settings.excludeKeywords?.length) {
      const text = `${model.name || ''} ${model.displayName || ''} ${model.description || ''}`.toLowerCase();
      if (settings.excludeKeywords.some(kw => text.includes(kw.toLowerCase()))) return false;
    }
    return true;
  });
}

// ── Utility Functions ───────────────────────────────────────────────────────

export function formatTime(timestamp: number): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));
  if (diffDays === 0) return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  if (diffDays === 1) return 'Yesterday';
  if (diffDays < 7) return date.toLocaleDateString([], { weekday: 'short' });
  return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
}

export function attachmentIcon(type: ChatAttachment['type']): string {
  switch (type) {
    case 'prompts': return '📝'; case 'files': return '📄';
    case 'diff': return '📦'; default: return '📎';
  }
}

export function attachmentBg(type: ChatAttachment['type']): string {
  switch (type) {
    case 'prompts': return 'bg-blue-50 border-blue-200 text-blue-700';
    case 'files': return 'bg-green-50 border-green-200 text-green-700';
    case 'diff': return 'bg-amber-50 border-amber-200 text-amber-700';
    default: return 'bg-gray-50 border-gray-200 text-gray-700';
  }
}

// ── Exported Store ──────────────────────────────────────────────────────────

export const chatState = {
  // ── Getters / Setters ────────────────────
  get sessions() { return sessions; },
  get activeSessionId() { return activeSessionId; },
  get sidebarCollapsed() { return sidebarCollapsed; },
  set sidebarCollapsed(v: boolean) { sidebarCollapsed = v; },
  get initialized() { return initialized; },

  get messages() { return messages; },
  get inputMessage() { return inputMessage; },
  set inputMessage(v: string) { inputMessage = v; },
  get isLoading() { return isLoading; },
  get error() { return error; },
  set error(v: string | null) { error = v; },

  get chatHistoryEnabled() { return chatHistoryEnabled; },
  get chatFontSize() { return chatFontSize; },
  get selectedModel() { return selectedModel; },
  set selectedModel(v: string) { selectedModel = v; },

  get attachments() { return attachments; },
  get previewAttachmentId() { return previewAttachmentId; },
  set previewAttachmentId(v: string | null) { previewAttachmentId = v; },

  get activeVendor() { return activeVendor; },
  get activeBranding() { return activeBranding; },
  get pinnedSessions() { return pinnedSessions; },
  get unpinnedSessions() { return unpinnedSessions; },

  // Agents
  get agents() { return agents; },
  get selectedAgentId() { return selectedAgentId; },
  get showAddAgentModal() { return showAddAgentModal; },
  get editingAgent() { return editingAgent; },
  get newAgentName() { return newAgentName; },
  set newAgentName(v: string) { newAgentName = v; },
  get newAgentDescription() { return newAgentDescription; },
  set newAgentDescription(v: string) { newAgentDescription = v; },
  get newAgentSystemPrompt() { return newAgentSystemPrompt; },
  set newAgentSystemPrompt(v: string) { newAgentSystemPrompt = v; },
  get newAgentColor() { return newAgentColor; },
  set newAgentColor(v: string) { newAgentColor = v; },
  get newAgentModel() { return newAgentModel; },
  set newAgentModel(v: string) { newAgentModel = v; },
  get activeModalTab() { return activeModalTab; },
  set activeModalTab(v: 'basic' | 'advanced') { activeModalTab = v; },
  get agentAvailableModels() { return agentAvailableModels; },
  get isLoadingAgentModels() { return isLoadingAgentModels; },
  get agentModelsError() { return agentModelsError; },

  // UI
  get expandedUserMsgs() { return expandedUserMsgs; },
  get copiedId() { return copiedId; },
  get copiedAll() { return copiedAll; },
  get copiedMsgIdx() { return copiedMsgIdx; },

  // ── Actions ──────────────────────────────

  initialize() {
    if (initialized) return;
    // Load sessions
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored) as ChatSession[];
        sessions = parsed.sort((a, b) => b.updatedAt - a.updatedAt);
        if (sessions.length > 0 && !activeSessionId) {
          this.selectSession(sessions[0].id);
        }
      }
    } catch (e) { console.error('Failed to load chat sessions:', e); }

    // Load agents
    try {
      const stored = localStorage.getItem(AGENTS_STORAGE_KEY);
      if (stored) agents = JSON.parse(stored) as AgentDefinition[];
    } catch (e) { console.error('Failed to load agents:', e); }

    initialized = true;
  },

  /** Save current draft before component destruction */
  saveDraftOnDestroy() {
    if (activeSessionId) saveDraftToStorage(activeSessionId, inputMessage);
  },

  /** Check and consume pending "Ask LLM" payload */
  checkPendingPayload() {
    if (!initialized || !navigationStore.hasPendingPayload) return;
    const payload = navigationStore.consumeChatPayload();
    if (payload && payload.attachments.length > 0) {
      this.createNewSession();
      attachments = payload.attachments;
      previewAttachmentId = null;
      if (payload.initialMessage) inputMessage = payload.initialMessage;
      updateCurrentSessionAttachments();
    }
  },

  createNewSession() {
    if (activeSessionId) saveDraftToStorage(activeSessionId, inputMessage);
    const newSession: ChatSession = {
      id: generateId(), title: 'New Chat', messages: [],
      createdAt: Date.now(), updatedAt: Date.now()
    };
    sessions = [newSession, ...sessions];
    activeSessionId = newSession.id;
    messages = []; inputMessage = ''; error = null;
    saveSessions();
  },

  selectSession(sessionId: string) {
    if (sessionId === activeSessionId) return;
    const session = sessions.find(s => s.id === sessionId);
    if (session) {
      if (activeSessionId) saveDraftToStorage(activeSessionId, inputMessage);
      activeSessionId = sessionId;
      messages = [...session.messages];
      attachments = session.attachments ? [...session.attachments] : [];
      previewAttachmentId = null; error = null;
      inputMessage = getDraftFromStorage(sessionId);
    }
  },

  deleteSession(sessionId: string, event: MouseEvent) {
    event.stopPropagation();
    deleteDraftFromStorage(sessionId);
    if (pinStore.isChatPinned(sessionId)) pinStore.toggleChatPin(sessionId);
    sessions = sessions.filter(s => s.id !== sessionId);
    if (activeSessionId === sessionId) {
      if (sessions.length > 0) { this.selectSession(sessions[0].id); }
      else { activeSessionId = null; messages = []; }
    }
    saveSessions();
  },

  toggleChatHistory() {
    chatHistoryEnabled = !chatHistoryEnabled;
    localStorage.setItem(HISTORY_ENABLED_KEY, String(chatHistoryEnabled));
  },

  clearChatHistory() {
    messages = []; attachments = []; previewAttachmentId = null; error = null;
    if (activeSessionId) {
      const session = sessions.find(s => s.id === activeSessionId);
      if (session) {
        session.messages = []; session.attachments = undefined;
        session.title = 'New Chat'; session.updatedAt = Date.now();
        saveSessions();
      }
    }
  },

  async sendMessage() {
    if (!inputMessage.trim() || isLoading) return;
    if (!activeSessionId) this.createNewSession();

    const userMessage = inputMessage.trim();
    inputMessage = '';
    if (activeSessionId) deleteDraftFromStorage(activeSessionId);
    error = null;

    const isFirstMessage = messages.filter(m => m.role === 'user').length === 0;
    const contextPrefix = (isFirstMessage && attachments.length > 0) ? buildAttachmentContext() : '';
    const messageToSend = contextPrefix + userMessage;
    const userMsg: ChatMessage = contextPrefix
      ? { role: 'user', content: messageToSend, displayContent: userMessage }
      : { role: 'user', content: userMessage };

    messages = [...messages, userMsg];
    updateCurrentSession();
    isLoading = true;

    try {
      const historyForApi: ChatMessage[] = chatHistoryEnabled
        ? messages.slice(0, -1).map(m => ({ role: m.role, content: m.content }))
        : [];
      const apiInfo = await getApiInfo();
      const vendor = vendorRegistry.getVendorForModel(selectedModel);
      if (!vendor) throw new Error(`No vendor found for model: ${selectedModel}`);
      const data = await vendor.sendChatMessage(apiInfo, messageToSend, historyForApi, selectedModel);
      messages = [...messages, { role: 'model', content: data.response }];
      updateCurrentSession();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to send message';
      messages = messages.slice(0, -1);
      updateCurrentSession();
    } finally {
      isLoading = false;
    }
  },

  handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      this.sendMessage();
    }
  },

  // Font size
  increaseFontSize() {
    const idx = FONT_SIZE_STEPS.indexOf(chatFontSize);
    if (idx < FONT_SIZE_STEPS.length - 1) {
      chatFontSize = FONT_SIZE_STEPS[idx + 1];
      localStorage.setItem(FONT_SIZE_KEY, String(chatFontSize));
    }
  },
  decreaseFontSize() {
    const idx = FONT_SIZE_STEPS.indexOf(chatFontSize);
    if (idx > 0) {
      chatFontSize = FONT_SIZE_STEPS[idx - 1];
      localStorage.setItem(FONT_SIZE_KEY, String(chatFontSize));
    }
  },

  // Attachments
  removeAttachment(id: string) {
    attachments = attachments.filter(a => a.id !== id);
    if (previewAttachmentId === id) previewAttachmentId = null;
    updateCurrentSessionAttachments();
  },
  togglePreview(id: string) {
    previewAttachmentId = previewAttachmentId === id ? null : id;
  },

  // Message expand/collapse
  toggleUserMsgExpand(idx: number) {
    const newSet = new Set(expandedUserMsgs);
    if (newSet.has(idx)) newSet.delete(idx); else newSet.add(idx);
    expandedUserMsgs = newSet;
  },

  // Copy
  async copyModelResponse(content: string, idx: number) {
    try {
      await navigator.clipboard.writeText(content);
      copiedMsgIdx = idx;
      setTimeout(() => { copiedMsgIdx = null; }, 1500);
    } catch (e) { console.error('Failed to copy:', e); }
  },
  async copyAttachment(att: ChatAttachment) {
    try {
      await navigator.clipboard.writeText(att.content);
      copiedId = att.id;
      setTimeout(() => { copiedId = null; }, 1500);
    } catch (e) { console.error('Failed to copy:', e); }
  },
  async copyAllAttachments() {
    if (attachments.length === 0) return;
    try {
      const combined = attachments.map(a => `--- ${a.label} ---\n${a.content}`).join('\n\n');
      await navigator.clipboard.writeText(combined);
      copiedAll = true;
      setTimeout(() => { copiedAll = false; }, 1500);
    } catch (e) { console.error('Failed to copy:', e); }
  },

  // Pin
  togglePinSession(sessionId: string, event: MouseEvent) {
    event.stopPropagation();
    pinStore.toggleChatPin(sessionId);
  },

  // ── Agent Actions ────────────────────────

  selectAgent(agentId: string) {
    selectedAgentId = selectedAgentId === agentId ? null : agentId;
  },

  openAddAgentModal() {
    editingAgent = null;
    newAgentName = ''; newAgentDescription = ''; newAgentSystemPrompt = '';
    newAgentColor = '#10B981'; newAgentModel = ''; activeModalTab = 'basic';
    showAddAgentModal = true;
    this.loadAgentModels(true);
  },

  openEditAgentModal(agent: AgentDefinition) {
    editingAgent = agent;
    newAgentName = agent.name; newAgentDescription = agent.description;
    newAgentSystemPrompt = agent.systemPrompt; newAgentColor = agent.color;
    newAgentModel = agent.defaultModel || ''; activeModalTab = 'basic';
    showAddAgentModal = true;
    this.loadAgentModels(true);
  },

  closeAddAgentModal() {
    showAddAgentModal = false; editingAgent = null;
    newAgentName = ''; newAgentDescription = ''; newAgentSystemPrompt = '';
    newAgentColor = '#10B981'; newAgentModel = '';
  },

  saveAgent() {
    if (!newAgentName.trim()) return;
    const now = Date.now();
    if (editingAgent) {
      const index = agents.findIndex(a => a.id === editingAgent!.id);
      if (index >= 0) {
        agents[index] = { ...agents[index], name: newAgentName.trim(), description: newAgentDescription.trim(),
          systemPrompt: newAgentSystemPrompt.trim(), color: newAgentColor,
          defaultModel: newAgentModel || undefined, updatedAt: now };
        agents = [...agents];
      }
    } else {
      agents = [...agents, {
        id: `agent-${now}-${Math.random().toString(36).substring(2, 9)}`,
        name: newAgentName.trim(), description: newAgentDescription.trim(),
        systemPrompt: newAgentSystemPrompt.trim(), color: newAgentColor,
        defaultModel: newAgentModel || undefined, isBuiltIn: false,
        createdAt: now, updatedAt: now
      }];
    }
    saveAgents();
    this.closeAddAgentModal();
  },

  deleteAgent(agentId: string, event?: MouseEvent) {
    event?.stopPropagation();
    agents = agents.filter(a => a.id !== agentId);
    if (selectedAgentId === agentId) selectedAgentId = null;
    saveAgents();
    if (showAddAgentModal && editingAgent?.id === agentId) this.closeAddAgentModal();
  },

  async loadAgentModels(forceReload = false) {
    if (!forceReload && agentAvailableModels.length > 0) return;
    isLoadingAgentModels = true; agentModelsError = null;
    try {
      const response = await getAgentModels();
      agentAvailableModels = filterModels(response.models);
    } catch (e) {
      agentModelsError = e instanceof Error ? e.message : 'Failed to load models';
    } finally { isLoadingAgentModels = false; }
  },
};

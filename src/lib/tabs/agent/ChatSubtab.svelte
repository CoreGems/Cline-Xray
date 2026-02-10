<script lang="ts">
  import { untrack } from "svelte";
  import { marked } from "marked";
  import { sendChatMessage, getAgentModels } from "./api";
  import type { AgentSettings } from "../../../types";
  import { DEFAULT_AGENT_SETTINGS } from "../../../types";

  // Configure marked for safe, sane defaults
  marked.setOptions({
    breaks: true,    // Convert \n to <br>
    gfm: true,       // GitHub-flavored markdown
  });

  /** Render markdown to HTML (synchronous) */
  function renderMarkdown(content: string): string {
    return marked.parse(content) as string;
  }
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import type { ChatMessage, ChatSession, ChatAttachment } from "./types";

  const STORAGE_KEY = 'agent-chat-sessions';
  const MODEL_STORAGE_KEY = 'agent-chat-selected-model';
  const AGENT_SETTINGS_KEY = 'agent-settings';
  const DEFAULT_MODEL = 'gemini-2.0-flash';

  // Chat sessions state
  let sessions: ChatSession[] = $state([]);
  let activeSessionId: string | null = $state(null);
  let sidebarCollapsed = $state(false);
  let initialized = $state(false);
  
  // Current chat state
  let messages: ChatMessage[] = $state([]);
  let inputMessage = $state('');
  let isLoading = $state(false);
  let error: string | null = $state(null);

  // Model selector state
  interface ModelOption {
    id: string;        // e.g. "gemini-2.0-flash" (without "models/" prefix)
    displayName: string;
    description?: string;
  }
  let availableModels: ModelOption[] = $state([]);
  let selectedModel: string = $state(localStorage.getItem(MODEL_STORAGE_KEY) || DEFAULT_MODEL);
  let modelsLoading = $state(false);
  let modelsError: string | null = $state(null);
  let showModelDropdown = $state(false);

  // Attachment state (in-memory context artifacts from "Ask LLM")
  let attachments: ChatAttachment[] = $state([]);
  let previewAttachmentId: string | null = $state(null);

  // Load sessions from localStorage on mount (once)
  $effect(() => {
    if (!initialized) {
      untrack(() => {
        loadSessions();
        loadModels();
        initialized = true;
      });
    }
  });

  // Check for incoming "Ask LLM" payload from navigation store
  $effect(() => {
    if (initialized && navigationStore.hasPendingPayload) {
      untrack(() => {
        const payload = navigationStore.consumeChatPayload();
        if (payload && payload.attachments.length > 0) {
          createNewSession();
          attachments = payload.attachments;
          previewAttachmentId = null;
          if (payload.initialMessage) {
            inputMessage = payload.initialMessage;
          }
          // Persist attachments with the new session
          updateCurrentSessionAttachments();
        }
      });
    }
  });

  // Generate unique ID
  function generateId(): string {
    return `chat-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
  }

  // ============== Model Helpers ==============

  // Load agent filter settings from localStorage
  function getAgentSettings(): AgentSettings {
    try {
      const stored = localStorage.getItem(AGENT_SETTINGS_KEY);
      if (stored) {
        return JSON.parse(stored) as AgentSettings;
      }
    } catch (e) {
      console.error('Failed to load agent settings:', e);
    }
    return DEFAULT_AGENT_SETTINGS;
  }

  // Load available models from the API
  async function loadModels() {
    modelsLoading = true;
    modelsError = null;
    try {
      const data = await getAgentModels();
      // Filter to models that support generateContent and map to ModelOption
      let models: ModelOption[] = (data.models || [])
        .filter((m: any) => 
          m.supportedGenerationMethods?.includes('generateContent')
        )
        .map((m: any) => ({
          id: m.name.replace(/^models\//, ''),
          displayName: m.displayName || m.name.replace(/^models\//, ''),
          description: m.description,
        }))
        .sort((a: ModelOption, b: ModelOption) => a.displayName.localeCompare(b.displayName));
      
      // Apply keyword filtering from agent settings
      const settings = getAgentSettings();
      if (settings.filterTextGenerationOnly && settings.excludeKeywords?.length > 0) {
        models = models.filter(model => {
          const searchText = `${model.id} ${model.displayName} ${model.description || ''}`.toLowerCase();
          return !settings.excludeKeywords.some(keyword => 
            searchText.includes(keyword.toLowerCase())
          );
        });
      }
      
      availableModels = models;
      
      // If selected model isn't in the list, fall back to default
      if (availableModels.length > 0 && !availableModels.find(m => m.id === selectedModel)) {
        // Try to find gemini-2.0-flash or use first available
        const defaultMatch = availableModels.find(m => m.id === DEFAULT_MODEL);
        selectedModel = defaultMatch ? defaultMatch.id : availableModels[0].id;
        localStorage.setItem(MODEL_STORAGE_KEY, selectedModel);
      }
    } catch (e) {
      console.error('Failed to load models:', e);
      modelsError = e instanceof Error ? e.message : 'Failed to load models';
      // Keep default model as fallback
      if (availableModels.length === 0) {
        availableModels = [{ id: DEFAULT_MODEL, displayName: 'Gemini 2.0 Flash' }];
      }
    } finally {
      modelsLoading = false;
    }
  }

  // Select a model
  function selectModel(modelId: string) {
    selectedModel = modelId;
    showModelDropdown = false;
    localStorage.setItem(MODEL_STORAGE_KEY, modelId);
  }

  // Get display name for current model
  function currentModelDisplay(): string {
    const model = availableModels.find(m => m.id === selectedModel);
    return model?.displayName || selectedModel;
  }

  // Load sessions from localStorage
  function loadSessions() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored) as ChatSession[];
        sessions = parsed.sort((a, b) => b.updatedAt - a.updatedAt);
        // Load the most recent session if exists
        if (sessions.length > 0 && !activeSessionId) {
          selectSession(sessions[0].id);
        }
      }
    } catch (e) {
      console.error('Failed to load chat sessions:', e);
    }
  }

  // Save sessions to localStorage
  function saveSessions() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions));
    } catch (e) {
      console.error('Failed to save chat sessions:', e);
    }
  }

  // Update current session with messages
  function updateCurrentSession() {
    if (!activeSessionId) return;
    
    const sessionIndex = sessions.findIndex(s => s.id === activeSessionId);
    if (sessionIndex >= 0) {
      const session = sessions[sessionIndex];
      session.messages = [...messages];
      session.attachments = attachments.length > 0 ? [...attachments] : undefined;
      session.updatedAt = Date.now();
      
      // Update title from first user message if still default
      if (session.title === 'New Chat' && messages.length > 0) {
        const firstUserMsg = messages.find(m => m.role === 'user');
        if (firstUserMsg) {
          session.title = firstUserMsg.content.substring(0, 40) + (firstUserMsg.content.length > 40 ? '...' : '');
        }
      }
      
      // Move to top of list
      sessions = [session, ...sessions.filter(s => s.id !== activeSessionId)];
      saveSessions();
    }
  }

  // Update only attachments on current session (used after Ask LLM payload)
  function updateCurrentSessionAttachments() {
    if (!activeSessionId) return;
    const session = sessions.find(s => s.id === activeSessionId);
    if (session) {
      session.attachments = attachments.length > 0 ? [...attachments] : undefined;
      session.updatedAt = Date.now();
      saveSessions();
    }
  }

  // Create a new chat session
  function createNewSession() {
    const newSession: ChatSession = {
      id: generateId(),
      title: 'New Chat',
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    };
    sessions = [newSession, ...sessions];
    activeSessionId = newSession.id;
    messages = [];
    error = null;
    saveSessions();
  }

  // Select a session
  function selectSession(sessionId: string) {
    const session = sessions.find(s => s.id === sessionId);
    if (session) {
      activeSessionId = sessionId;
      messages = [...session.messages];
      attachments = session.attachments ? [...session.attachments] : [];
      previewAttachmentId = null;
      error = null;
    }
  }

  // Delete a session
  function deleteSession(sessionId: string, event: MouseEvent) {
    event.stopPropagation();
    sessions = sessions.filter(s => s.id !== sessionId);
    
    if (activeSessionId === sessionId) {
      if (sessions.length > 0) {
        selectSession(sessions[0].id);
      } else {
        activeSessionId = null;
        messages = [];
      }
    }
    saveSessions();
  }

  // Send message to Gemini via backend API
  async function sendMessage() {
    if (!inputMessage.trim() || isLoading) return;

    // Create session if none exists
    if (!activeSessionId) {
      createNewSession();
    }

    const userMessage = inputMessage.trim();
    inputMessage = '';
    error = null;

    // Add user message to chat
    messages = [...messages, { role: 'user', content: userMessage }];
    updateCurrentSession();

    isLoading = true;
    try {
      // On the first message of a session, prepend attachment context
      const isFirstMessage = messages.length === 1;
      const contextPrefix = (isFirstMessage && attachments.length > 0) ? buildAttachmentContext() : '';
      const messageToSend = contextPrefix + userMessage;

      const historyToSend = messages.slice(0, -1); // Send history without the message we just added
      const data = await sendChatMessage(messageToSend, historyToSend, selectedModel);
      
      // Add AI response to chat
      messages = [...messages, { role: 'model', content: data.response }];
      updateCurrentSession();
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : 'Failed to send message';
      error = errorMessage;
      // Remove the user message if the request failed
      messages = messages.slice(0, -1);
      updateCurrentSession();
    } finally {
      isLoading = false;
    }
  }

  // Handle Enter key press
  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      sendMessage();
    }
  }

  // Clear current chat (but keep session)
  function clearChat() {
    messages = [];
    attachments = [];
    previewAttachmentId = null;
    error = null;
    if (activeSessionId) {
      const session = sessions.find(s => s.id === activeSessionId);
      if (session) {
        session.messages = [];
        session.attachments = undefined;
        session.title = 'New Chat';
        session.updatedAt = Date.now();
        saveSessions();
      }
    }
  }

  // ============== Attachment Helpers ==============

  function attachmentIcon(type: ChatAttachment['type']): string {
    switch (type) {
      case 'prompts': return '📝';
      case 'files': return '📄';
      case 'diff': return '📦';
      default: return '📎';
    }
  }

  function attachmentBg(type: ChatAttachment['type']): string {
    switch (type) {
      case 'prompts': return 'bg-blue-50 border-blue-200 text-blue-700';
      case 'files': return 'bg-green-50 border-green-200 text-green-700';
      case 'diff': return 'bg-amber-50 border-amber-200 text-amber-700';
      default: return 'bg-gray-50 border-gray-200 text-gray-700';
    }
  }

  function removeAttachment(id: string) {
    attachments = attachments.filter(a => a.id !== id);
    if (previewAttachmentId === id) previewAttachmentId = null;
    updateCurrentSessionAttachments();
  }

  function togglePreview(id: string) {
    previewAttachmentId = previewAttachmentId === id ? null : id;
  }

  /** Build context prefix from attachments for the first message */
  function buildAttachmentContext(): string {
    if (attachments.length === 0) return '';
    return attachments.map(a =>
      `--- ${a.label} ---\n${a.content}`
    ).join('\n\n') + '\n\n---\nUser message:\n';
  }

  // Format timestamp
  function formatTime(timestamp: number): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));
    
    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } else if (diffDays === 1) {
      return 'Yesterday';
    } else if (diffDays < 7) {
      return date.toLocaleDateString([], { weekday: 'short' });
    } else {
      return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
    }
  }
</script>

<div class="flex-1 flex h-full overflow-hidden">
  <!-- Left Panel - Chat History -->
  <div class="flex flex-col bg-gray-100 border-r border-gray-200 transition-all duration-200 {sidebarCollapsed ? 'w-12' : 'w-64'}">
    <!-- Sidebar Header -->
    <div class="flex items-center justify-between p-3 border-b border-gray-200 bg-white">
      {#if !sidebarCollapsed}
        <h3 class="font-medium text-gray-700 text-sm">Chat History</h3>
      {/if}
      <div class="flex items-center gap-1 {sidebarCollapsed ? 'mx-auto' : ''}">
        {#if !sidebarCollapsed}
          <button
            onclick={createNewSession}
            class="p-1.5 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors"
            title="New Chat"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
            </svg>
          </button>
        {/if}
        <button
          onclick={() => sidebarCollapsed = !sidebarCollapsed}
          class="p-1.5 text-gray-600 hover:text-gray-800 hover:bg-gray-200 rounded transition-colors"
          title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          <svg class="w-5 h-5 transition-transform {sidebarCollapsed ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7"></path>
          </svg>
        </button>
      </div>
    </div>

    <!-- Session List -->
    {#if !sidebarCollapsed}
      <div class="flex-1 overflow-y-auto">
        {#if sessions.length === 0}
          <div class="p-4 text-center text-gray-500 text-sm">
            <p>No chat history yet.</p>
            <button 
              onclick={createNewSession}
              class="mt-2 text-blue-600 hover:text-blue-700 hover:underline"
            >
              Start a new chat
            </button>
          </div>
        {:else}
          <div class="py-2">
            {#each sessions as session (session.id)}
              <div
                role="button"
                tabindex="0"
                onclick={() => selectSession(session.id)}
                onkeydown={(e) => e.key === 'Enter' && selectSession(session.id)}
                class="w-full px-3 py-2 text-left hover:bg-gray-200 group transition-colors flex items-start gap-2 cursor-pointer {activeSessionId === session.id ? 'bg-blue-50 border-r-2 border-blue-500' : ''}"
              >
                <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
                </svg>
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium text-gray-800 truncate">
                    {session.title}
                  </div>
                  <div class="text-xs text-gray-500">
                    {formatTime(session.updatedAt)} · {session.messages.length} messages
                  </div>
                </div>
                <button
                  onclick={(e) => deleteSession(session.id, e)}
                  class="p-1 opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded transition-all"
                  title="Delete chat"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <!-- Collapsed state - just show new chat button -->
      <div class="flex-1 flex flex-col items-center pt-2">
        <button
          onclick={createNewSession}
          class="p-2 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors"
          title="New Chat"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <!-- Right Panel - Chat Area -->
  <div class="flex-1 flex flex-col overflow-hidden bg-gray-50">
    <!-- Chat Messages Area -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      {#if messages.length === 0}
        <div class="flex items-center justify-center h-full">
          <div class="text-center text-gray-500">
            <svg class="w-16 h-16 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
            </svg>
            <div class="flex items-center justify-center gap-2 mb-2">
              <span class="text-lg font-semibold" style="color: #4285F4">G</span><span class="text-lg font-semibold" style="color: #EA4335">o</span><span class="text-lg font-semibold" style="color: #FBBC04">o</span><span class="text-lg font-semibold" style="color: #4285F4">g</span><span class="text-lg font-semibold" style="color: #34A853">l</span><span class="text-lg font-semibold" style="color: #EA4335">e</span>
              <span class="text-lg font-semibold bg-gradient-to-r from-blue-500 via-purple-500 to-pink-500 bg-clip-text text-transparent">Gemini</span>
            </div>
            <h3 class="text-lg font-medium text-gray-700 mb-2">Chat with Gemini</h3>
            <p class="text-sm">Powered by Google Gemini AI</p>
            {#if !activeSessionId}
              <button 
                onclick={createNewSession}
                class="mt-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm"
              >
                Start New Chat
              </button>
            {/if}
          </div>
        </div>
      {:else}
        {#each messages as message}
          <div class="flex {message.role === 'user' ? 'justify-end' : 'justify-start'}">
            <div class="max-w-[80%] {message.role === 'user' 
              ? 'bg-blue-600 text-white rounded-l-lg rounded-tr-lg' 
              : 'bg-white border border-gray-200 text-gray-800 rounded-r-lg rounded-tl-lg shadow-sm'} px-4 py-3">
              <div class="flex items-center gap-2 mb-1">
                {#if message.role === 'user'}
                  <span class="text-xs font-medium text-blue-200">You</span>
                {:else}
                  <span class="text-xs font-bold bg-gradient-to-r from-blue-500 via-purple-500 to-pink-500 bg-clip-text text-transparent">✦ Gemini</span>
                {/if}
              </div>
              {#if message.role === 'model'}
                <div class="markdown-body text-sm">{@html renderMarkdown(message.content)}</div>
              {:else}
                <div class="whitespace-pre-wrap text-sm">{message.content}</div>
              {/if}
            </div>
          </div>
        {/each}
        
        {#if isLoading}
          <div class="flex justify-start">
            <div class="bg-white border border-gray-200 rounded-r-lg rounded-tl-lg px-4 py-3 shadow-sm">
              <div class="flex items-center gap-2">
                <div class="flex gap-1">
                  <span class="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style="animation-delay: 0ms"></span>
                  <span class="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style="animation-delay: 150ms"></span>
                  <span class="w-2 h-2 bg-gray-400 rounded-full animate-bounce" style="animation-delay: 300ms"></span>
                </div>
                <span class="text-sm text-gray-500">Thinking...</span>
              </div>
            </div>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Error Message -->
    {#if error}
      <div class="mx-4 mb-2 p-3 bg-red-50 border border-red-200 rounded-lg">
        <div class="flex items-center gap-2 text-red-700">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
          </svg>
          <span class="text-sm">{error}</span>
          <button onclick={() => error = null} class="ml-auto p-1 hover:bg-red-100 rounded">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
          </button>
        </div>
      </div>
    {/if}

    <!-- Input Area -->
    <div class="border-t border-gray-200 bg-white p-4">
      <!-- Attachment Bar (shown when attachments exist) -->
      {#if attachments.length > 0}
        <div class="mb-3">
          <div class="flex items-center gap-2 flex-wrap">
            {#each attachments as att (att.id)}
              <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-xs font-medium transition-colors {attachmentBg(att.type)} {previewAttachmentId === att.id ? 'ring-2 ring-offset-1 ring-blue-400' : ''}">
                <button
                  onclick={() => togglePreview(att.id)}
                  class="flex items-center gap-1.5 hover:opacity-80"
                  title="Click to preview"
                >
                  <span>{attachmentIcon(att.type)}</span>
                  <span>{att.label}</span>
                </button>
                <button
                  onclick={() => removeAttachment(att.id)}
                  class="ml-0.5 p-0.5 rounded hover:bg-red-100 hover:text-red-600 transition-colors"
                  title="Remove attachment"
                >
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                  </svg>
                </button>
              </div>
            {/each}
          </div>

          <!-- Preview Panel (expandable) -->
          {#if previewAttachmentId}
            {@const previewAtt = attachments.find(a => a.id === previewAttachmentId)}
            {#if previewAtt}
              <div class="mt-2 border border-gray-200 rounded-lg overflow-hidden bg-gray-50">
                <div class="px-3 py-1.5 bg-gray-100 border-b border-gray-200 flex items-center justify-between">
                  <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">
                    {attachmentIcon(previewAtt.type)} {previewAtt.label}
                  </span>
                  <span class="text-[10px] text-gray-400">
                    {(previewAtt.content.length / 1024).toFixed(1)} KB
                  </span>
                </div>
                <pre class="max-h-48 overflow-auto p-3 text-[10px] leading-relaxed font-mono text-gray-700 whitespace-pre-wrap break-all m-0">{previewAtt.content.length > 50000 ? previewAtt.content.substring(0, 50000) + '\n\n... [truncated for preview]' : previewAtt.content}</pre>
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      <div class="flex gap-2">
        <div class="flex-1 relative">
          <textarea
            bind:value={inputMessage}
            onkeydown={handleKeyDown}
            placeholder="Type your message..."
            rows="3"
            class="w-full px-4 py-3 border border-gray-300 rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
            disabled={isLoading}
          ></textarea>
        </div>
        <div class="flex flex-col gap-2">
          <button
            onclick={sendMessage}
            disabled={isLoading || !inputMessage.trim()}
            class="px-4 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
            title="Send message"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
            </svg>
          </button>
          {#if messages.length > 0}
            <button
              onclick={clearChat}
              disabled={isLoading}
              class="px-4 py-3 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2 disabled:cursor-not-allowed transition-colors"
              title="Clear chat"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
              </svg>
            </button>
          {/if}
        </div>
      </div>
      <!-- Model Selector + Hint Row -->
      <div class="mt-1 flex items-center gap-2">
        <div class="relative">
          <button
            onclick={() => showModelDropdown = !showModelDropdown}
            disabled={modelsLoading}
            class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-md border border-gray-300 bg-white text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title={modelsError ? `Error: ${modelsError}` : `Model: ${selectedModel}`}
          >
            <!-- Model icon -->
            <svg class="w-3.5 h-3.5 text-purple-500 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path>
            </svg>
            {#if modelsLoading}
              <span class="text-gray-400">Loading...</span>
            {:else}
              <span class="max-w-[180px] truncate">{currentModelDisplay()}</span>
            {/if}
            <!-- Chevron -->
            <svg class="w-3 h-3 text-gray-400 flex-shrink-0 transition-transform {showModelDropdown ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
            </svg>
          </button>

          <!-- Dropdown menu (opens upward) -->
          {#if showModelDropdown && availableModels.length > 0}
            <!-- Backdrop to close dropdown -->
            <div
              class="fixed inset-0 z-10"
              onclick={() => showModelDropdown = false}
              onkeydown={(e) => e.key === 'Escape' && (showModelDropdown = false)}
              role="button"
              tabindex="-1"
            ></div>
            <div class="absolute bottom-full left-0 mb-1 w-72 max-h-64 overflow-y-auto bg-white border border-gray-200 rounded-lg shadow-lg z-20">
              {#each availableModels as model (model.id)}
                <button
                  onclick={() => selectModel(model.id)}
                  class="w-full text-left px-3 py-2 text-xs hover:bg-blue-50 transition-colors flex items-center gap-2 {selectedModel === model.id ? 'bg-blue-50 text-blue-700' : 'text-gray-700'}"
                  title={model.description || model.id}
                >
                  <span class="w-4 flex-shrink-0 text-blue-500">
                    {#if selectedModel === model.id}
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                      </svg>
                    {/if}
                  </span>
                  <div class="flex-1 min-w-0">
                    <div class="font-medium truncate">{model.displayName}</div>
                    <div class="text-[10px] text-gray-400 truncate">{model.id}</div>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        {#if modelsError}
          <span class="text-[10px] text-amber-600" title={modelsError}>⚠</span>
        {/if}

        <!-- Refresh models button -->
        <button
          onclick={loadModels}
          disabled={modelsLoading}
          class="p-1 text-gray-400 hover:text-gray-600 rounded transition-colors disabled:opacity-50"
          title="Refresh models list"
        >
          <svg class="w-3.5 h-3.5 {modelsLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
        </button>

        <!-- Spacer + hint -->
        <span class="flex-1 text-xs text-gray-500 text-right">
        Press Enter to send, Shift+Enter for new line
        </span>
      </div>
    </div>
  </div>
</div>

<style>
  :global(.markdown-body) {
    line-height: 1.6;
    word-wrap: break-word;
  }
  :global(.markdown-body h1),
  :global(.markdown-body h2),
  :global(.markdown-body h3),
  :global(.markdown-body h4) {
    margin-top: 0.75em;
    margin-bottom: 0.35em;
    font-weight: 600;
    line-height: 1.3;
  }
  :global(.markdown-body h1) { font-size: 1.25em; }
  :global(.markdown-body h2) { font-size: 1.15em; }
  :global(.markdown-body h3) { font-size: 1.05em; }
  :global(.markdown-body p) {
    margin-top: 0.4em;
    margin-bottom: 0.4em;
  }
  :global(.markdown-body ul),
  :global(.markdown-body ol) {
    padding-left: 1.5em;
    margin-top: 0.3em;
    margin-bottom: 0.3em;
  }
  :global(.markdown-body li) { margin-bottom: 0.15em; }
  :global(.markdown-body ul) { list-style-type: disc; }
  :global(.markdown-body ol) { list-style-type: decimal; }
  :global(.markdown-body code) {
    background-color: rgba(0, 0, 0, 0.06);
    padding: 0.15em 0.35em;
    border-radius: 3px;
    font-size: 0.9em;
    font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  }
  :global(.markdown-body pre) {
    background-color: #f6f8fa;
    border: 1px solid #e1e4e8;
    border-radius: 6px;
    padding: 0.75em 1em;
    overflow-x: auto;
    margin: 0.5em 0;
  }
  :global(.markdown-body pre code) {
    background: none;
    padding: 0;
    font-size: 0.85em;
    line-height: 1.5;
  }
  :global(.markdown-body blockquote) {
    border-left: 3px solid #d1d5db;
    padding-left: 0.75em;
    margin: 0.5em 0;
    color: #6b7280;
  }
  :global(.markdown-body strong) { font-weight: 600; }
  :global(.markdown-body a) { color: #2563eb; text-decoration: underline; }
  :global(.markdown-body a:hover) { color: #1d4ed8; }
  :global(.markdown-body hr) {
    border: none;
    border-top: 1px solid #e5e7eb;
    margin: 0.75em 0;
  }
  :global(.markdown-body table) {
    border-collapse: collapse;
    width: 100%;
    margin: 0.5em 0;
    font-size: 0.9em;
  }
  :global(.markdown-body th),
  :global(.markdown-body td) {
    border: 1px solid #e5e7eb;
    padding: 0.35em 0.65em;
    text-align: left;
  }
  :global(.markdown-body th) { background-color: #f9fafb; font-weight: 600; }
  :global(.markdown-body > *:first-child) { margin-top: 0; }
  :global(.markdown-body > *:last-child) { margin-bottom: 0; }
</style>

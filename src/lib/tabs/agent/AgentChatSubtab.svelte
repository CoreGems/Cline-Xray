<script lang="ts">
  import { untrack } from "svelte";
  import type { AgentChatMessage, AgentChatSession, AgentDefinition, AgentModel } from "./types";
  import { getAgentModels } from "./api";
  import type { AgentSettings } from "../../../types";
  import { DEFAULT_AGENT_SETTINGS } from "../../../types";

  const STORAGE_KEY = 'agent-chat-agent-sessions';
  const AGENTS_STORAGE_KEY = 'agent-chat-agents';
  const AGENT_SETTINGS_KEY = 'agent-settings';

  // Agent chat sessions state
  let sessions: AgentChatSession[] = $state([]);
  let activeSessionId: string | null = $state(null);
  let sidebarCollapsed = $state(false);
  let initialized = $state(false);
  
  // Current chat state
  let messages: AgentChatMessage[] = $state([]);
  let inputMessage = $state('');
  let isLoading = $state(false);
  let error: string | null = $state(null);

  // Agent management state
  let agents: AgentDefinition[] = $state([]);
  let selectedAgentId: string | null = $state(null);
  let showAddAgentModal = $state(false);
  let editingAgent: AgentDefinition | null = $state(null);
  
  // New agent form state
  let newAgentName = $state('');
  let newAgentDescription = $state('');
  let newAgentSystemPrompt = $state('');
  let newAgentColor = $state('#10B981');
  let newAgentModel = $state('');
  let activeModalTab = $state<'basic' | 'advanced'>('basic');

  // Available models state
  let availableModels: AgentModel[] = $state([]);
  let isLoadingModels = $state(false);
  let modelsError: string | null = $state(null);

  // Predefined colors for agents
  const agentColors = [
    '#10B981', // green
    '#3B82F6', // blue
    '#8B5CF6', // purple
    '#F59E0B', // amber
    '#EF4444', // red
    '#EC4899', // pink
    '#06B6D4', // cyan
    '#6366F1', // indigo
  ];

  // Load sessions and agents from localStorage on mount (once)
  $effect(() => {
    if (!initialized) {
      untrack(() => {
        loadSessions();
        loadAgents();
        initialized = true;
      });
    }
  });

  // Generate unique ID
  function generateId(): string {
    return `agent-chat-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
  }

  // Load sessions from localStorage
  function loadSessions() {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored) as AgentChatSession[];
        sessions = parsed.sort((a, b) => b.updatedAt - a.updatedAt);
        // Load the most recent session if exists
        if (sessions.length > 0 && !activeSessionId) {
          selectSession(sessions[0].id);
        }
      }
    } catch (e) {
      console.error('Failed to load agent chat sessions:', e);
    }
  }

  // Save sessions to localStorage
  function saveSessions() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(sessions));
    } catch (e) {
      console.error('Failed to save agent chat sessions:', e);
    }
  }

  // Update current session with messages
  function updateCurrentSession() {
    if (!activeSessionId) return;
    
    const sessionIndex = sessions.findIndex(s => s.id === activeSessionId);
    if (sessionIndex >= 0) {
      const session = sessions[sessionIndex];
      session.messages = [...messages];
      session.updatedAt = Date.now();
      
      // Update title from first user message if still default
      if (session.title === 'New Agent Chat' && messages.length > 0) {
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

  // Create a new agent chat session
  function createNewSession() {
    const newSession: AgentChatSession = {
      id: generateId(),
      title: 'New Agent Chat',
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

  // Send message to Agent (placeholder - to be connected to real agent API)
  async function sendMessage() {
    if (!inputMessage.trim() || isLoading) return;

    // Create session if none exists
    if (!activeSessionId) {
      createNewSession();
    }

    const userMessage = inputMessage.trim();
    inputMessage = '';
    error = null;

    // Get the currently selected agent
    const currentAgent = getSelectedAgent();

    // Add user message to chat
    messages = [...messages, { role: 'user', content: userMessage, timestamp: Date.now() }];
    updateCurrentSession();

    isLoading = true;
    try {
      // TODO: Connect to actual agent API
      // For now, simulate a response
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      // Placeholder agent response - include agent info if one is selected
      const agentName = currentAgent?.name || 'Agent';
      const agentResponse = `[${agentName} Response Placeholder]\n\nReceived your message: "${userMessage}"\n\nThis is a placeholder response. The Agent Chat will be connected to the actual agent system.`;
      
      // Create agent message with agent info
      const agentMessage: AgentChatMessage = {
        role: 'agent',
        content: agentResponse,
        timestamp: Date.now(),
        agentId: currentAgent?.id,
        agentName: currentAgent?.name,
        agentColor: currentAgent?.color
      };
      
      messages = [...messages, agentMessage];
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
    error = null;
    if (activeSessionId) {
      const session = sessions.find(s => s.id === activeSessionId);
      if (session) {
        session.messages = [];
        session.title = 'New Agent Chat';
        session.updatedAt = Date.now();
        saveSessions();
      }
    }
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

  // ============== Agent Management Functions ==============

  // Load agents from localStorage
  function loadAgents() {
    try {
      const stored = localStorage.getItem(AGENTS_STORAGE_KEY);
      if (stored) {
        agents = JSON.parse(stored) as AgentDefinition[];
      }
    } catch (e) {
      console.error('Failed to load agents:', e);
    }
  }

  // Save agents to localStorage
  function saveAgents() {
    try {
      localStorage.setItem(AGENTS_STORAGE_KEY, JSON.stringify(agents));
    } catch (e) {
      console.error('Failed to save agents:', e);
    }
  }

  // Load agent settings from localStorage
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

  // Filter models based on agent settings
  function filterModels(models: AgentModel[]): AgentModel[] {
    const settings = getAgentSettings();
    
    if (!settings.filterTextGenerationOnly) {
      return models;
    }
    
    return models.filter(model => {
      // Check required methods
      if (settings.requiredMethods.length > 0) {
        const supportedMethods = model.supportedGenerationMethods || [];
        const hasAllMethods = settings.requiredMethods.every(method => 
          supportedMethods.includes(method)
        );
        if (!hasAllMethods) return false;
      }
      
      // Check exclude keywords (case-insensitive)
      if (settings.excludeKeywords && settings.excludeKeywords.length > 0) {
        const modelName = (model.name || '').toLowerCase();
        const displayName = (model.displayName || '').toLowerCase();
        const description = (model.description || '').toLowerCase();
        const searchText = `${modelName} ${displayName} ${description}`;
        
        const hasExcludedKeyword = settings.excludeKeywords.some(keyword => 
          searchText.includes(keyword.toLowerCase())
        );
        if (hasExcludedKeyword) return false;
      }
      
      return true;
    });
  }

  // Load available models from API
  async function loadModels(forceReload = false) {
    if (!forceReload && availableModels.length > 0) return; // Already loaded
    
    isLoadingModels = true;
    modelsError = null;
    
    try {
      const response = await getAgentModels();
      // Apply filter based on agent settings
      availableModels = filterModels(response.models);
    } catch (e) {
      modelsError = e instanceof Error ? e.message : 'Failed to load models';
      console.error('Failed to load models:', e);
    } finally {
      isLoadingModels = false;
    }
  }

  // Open Add Agent modal
  function openAddAgentModal() {
    editingAgent = null;
    newAgentName = '';
    newAgentDescription = '';
    newAgentSystemPrompt = '';
    newAgentColor = '#10B981';
    newAgentModel = '';
    activeModalTab = 'basic';
    showAddAgentModal = true;
    // Force reload to apply any settings changes
    loadModels(true);
  }

  // Open Edit Agent modal
  function openEditAgentModal(agent: AgentDefinition) {
    editingAgent = agent;
    newAgentName = agent.name;
    newAgentDescription = agent.description;
    newAgentSystemPrompt = agent.systemPrompt;
    newAgentColor = agent.color;
    newAgentModel = agent.defaultModel || '';
    activeModalTab = 'basic';
    showAddAgentModal = true;
    // Force reload to apply any settings changes
    loadModels(true);
  }

  // Close Add Agent modal
  function closeAddAgentModal() {
    showAddAgentModal = false;
    editingAgent = null;
    newAgentName = '';
    newAgentDescription = '';
    newAgentSystemPrompt = '';
    newAgentColor = '#10B981';
    newAgentModel = '';
  }

  // Save agent (create or update)
  function saveAgent() {
    if (!newAgentName.trim()) return;

    const now = Date.now();
    
    if (editingAgent) {
      // Update existing agent
      const index = agents.findIndex(a => a.id === editingAgent!.id);
      if (index >= 0) {
        agents[index] = {
          ...agents[index],
          name: newAgentName.trim(),
          description: newAgentDescription.trim(),
          systemPrompt: newAgentSystemPrompt.trim(),
          color: newAgentColor,
          defaultModel: newAgentModel || undefined,
          updatedAt: now
        };
        agents = [...agents];
      }
    } else {
      // Create new agent
      const newAgent: AgentDefinition = {
        id: `agent-${now}-${Math.random().toString(36).substring(2, 9)}`,
        name: newAgentName.trim(),
        description: newAgentDescription.trim(),
        systemPrompt: newAgentSystemPrompt.trim(),
        color: newAgentColor,
        defaultModel: newAgentModel || undefined,
        isBuiltIn: false,
        createdAt: now,
        updatedAt: now
      };
      agents = [...agents, newAgent];
    }
    
    saveAgents();
    closeAddAgentModal();
  }

  // Delete an agent
  function deleteAgent(agentId: string, event?: MouseEvent) {
    event?.stopPropagation();
    agents = agents.filter(a => a.id !== agentId);
    if (selectedAgentId === agentId) {
      selectedAgentId = null;
    }
    saveAgents();
    // Close modal if we're deleting from within the edit modal
    if (showAddAgentModal && editingAgent?.id === agentId) {
      closeAddAgentModal();
    }
  }

  // Select an agent
  function selectAgent(agentId: string) {
    selectedAgentId = selectedAgentId === agentId ? null : agentId;
  }

  // Get selected agent
  function getSelectedAgent(): AgentDefinition | null {
    return agents.find(a => a.id === selectedAgentId) || null;
  }
</script>

<div class="flex-1 flex h-full overflow-hidden">
  <!-- Left Panel - Agent Chat History -->
  <div class="flex flex-col bg-gray-100 border-r border-gray-200 transition-all duration-200 {sidebarCollapsed ? 'w-12' : 'w-64'}">
    <!-- Sidebar Header -->
    <div class="flex items-center justify-between p-3 border-b border-gray-200 bg-white">
      {#if !sidebarCollapsed}
        <h3 class="font-medium text-gray-700 text-sm">Agent Chat History</h3>
      {/if}
      <div class="flex items-center gap-1 {sidebarCollapsed ? 'mx-auto' : ''}">
        {#if !sidebarCollapsed}
          <button
            onclick={createNewSession}
            class="p-1.5 text-gray-600 hover:text-green-600 hover:bg-green-50 rounded transition-colors"
            title="New Agent Chat"
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
            <p>No agent chat history yet.</p>
            <button 
              onclick={createNewSession}
              class="mt-2 text-green-600 hover:text-green-700 hover:underline"
            >
              Start a new agent chat
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
                class="w-full px-3 py-2 text-left hover:bg-gray-200 group transition-colors flex items-start gap-2 cursor-pointer {activeSessionId === session.id ? 'bg-green-50 border-r-2 border-green-500' : ''}"
              >
                <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path>
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
                  title="Delete agent chat"
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
          class="p-2 text-gray-600 hover:text-green-600 hover:bg-green-50 rounded transition-colors"
          title="New Agent Chat"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
          </svg>
        </button>
      </div>
    {/if}
  </div>

  <!-- Right Panel - Agent Chat Area -->
  <div class="flex-1 flex flex-col overflow-hidden bg-gray-50">
    <!-- Agent Selection Header -->
    <div class="flex items-center gap-3 p-3 border-b border-gray-200 bg-white">
      <!-- Agent Selector -->
      <div class="flex items-center gap-2 flex-1">
        <span class="text-sm text-gray-600">Agent:</span>
        {#if agents.length === 0}
          <span class="text-sm text-gray-400 italic">No agents configured</span>
        {:else}
          <div class="flex items-center gap-2 flex-wrap">
            {#each agents as agent (agent.id)}
              <div class="relative group">
                <button
                  onclick={() => selectAgent(agent.id)}
                  class="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-sm transition-all {selectedAgentId === agent.id 
                    ? 'ring-2 ring-offset-1' 
                    : 'hover:bg-gray-100'}"
                  style="background-color: {selectedAgentId === agent.id ? agent.color + '20' : 'transparent'}; 
                         color: {agent.color}; 
                         {selectedAgentId === agent.id ? `ring-color: ${agent.color}` : ''}"
                  title={agent.description || agent.name}
                >
                  <span class="w-2 h-2 rounded-full" style="background-color: {agent.color}"></span>
                  <span class="font-medium">{agent.name}</span>
                </button>
                <!-- Edit button on hover -->
                <button
                  onclick={(e) => { e.stopPropagation(); openEditAgentModal(agent); }}
                  class="absolute -top-1 -right-1 p-0.5 bg-white border border-gray-200 rounded-full shadow-sm opacity-0 group-hover:opacity-100 transition-opacity hover:bg-gray-100"
                  title="Edit agent"
                >
                  <svg class="w-3 h-3 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"></path>
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      
      <!-- Add Agent Button -->
      <button
        onclick={openAddAgentModal}
        class="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
        title="Add a new agent"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
        </svg>
        Add Agent
      </button>
    </div>

    <!-- Chat Messages Area -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      {#if messages.length === 0}
        <div class="flex items-center justify-center h-full">
          <div class="text-center text-gray-500">
            <svg class="w-16 h-16 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path>
            </svg>
            <h3 class="text-lg font-medium text-gray-700 mb-2">Agent Chat</h3>
            <p class="text-sm">Start a conversation with the AI Agent.</p>
            <p class="text-xs text-gray-400 mt-1">The agent can perform tasks and access tools.</p>
            {#if !activeSessionId}
              <button 
                onclick={createNewSession}
                class="mt-4 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors text-sm"
              >
                Start New Agent Chat
              </button>
            {/if}
          </div>
        </div>
      {:else}
        {#each messages as message}
          <div class="flex {message.role === 'user' ? 'justify-end' : 'justify-start'}">
            <div class="max-w-[80%] {message.role === 'user' 
              ? 'bg-green-600 text-white rounded-l-lg rounded-tr-lg' 
              : 'bg-white border border-gray-200 text-gray-800 rounded-r-lg rounded-tl-lg shadow-sm'} px-4 py-3"
              style="{message.role === 'agent' && message.agentColor ? `border-left: 3px solid ${message.agentColor}` : ''}">
              <div class="flex items-center gap-2 mb-1">
                {#if message.role === 'user'}
                  <span class="text-xs font-medium text-green-200">You</span>
                {:else}
                  {#if message.agentColor}
                    <span class="w-3 h-3 rounded-full flex-shrink-0" style="background-color: {message.agentColor}"></span>
                  {:else}
                    <svg class="w-4 h-4 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path>
                    </svg>
                  {/if}
                  <span class="text-xs font-medium" style="color: {message.agentColor || '#10B981'}">
                    {message.agentName || 'Agent'}
                  </span>
                {/if}
              </div>
              <div class="whitespace-pre-wrap text-sm">{message.content}</div>
            </div>
          </div>
        {/each}
        
        {#if isLoading}
          <div class="flex justify-start">
            <div class="bg-white border border-gray-200 rounded-r-lg rounded-tl-lg px-4 py-3 shadow-sm">
              <div class="flex items-center gap-2">
                <div class="flex gap-1">
                  <span class="w-2 h-2 bg-green-400 rounded-full animate-bounce" style="animation-delay: 0ms"></span>
                  <span class="w-2 h-2 bg-green-400 rounded-full animate-bounce" style="animation-delay: 150ms"></span>
                  <span class="w-2 h-2 bg-green-400 rounded-full animate-bounce" style="animation-delay: 300ms"></span>
                </div>
                <span class="text-sm text-gray-500">Agent is working...</span>
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
      <div class="flex gap-2">
        <div class="flex-1 relative">
          <textarea
            bind:value={inputMessage}
            onkeydown={handleKeyDown}
            placeholder="Ask the agent to do something..."
            rows="2"
            class="w-full px-4 py-3 border border-gray-300 rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-green-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
            disabled={isLoading}
          ></textarea>
        </div>
        <div class="flex flex-col gap-2">
          <button
            onclick={sendMessage}
            disabled={isLoading || !inputMessage.trim()}
            class="px-4 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
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
      <p class="mt-2 text-xs text-gray-500 text-center">
        Press Enter to send, Shift+Enter for new line
      </p>
    </div>
  </div>
</div>

<!-- Add/Edit Agent Modal -->
{#if showAddAgentModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center">
    <!-- Backdrop -->
    <div 
      class="absolute inset-0 bg-black/50" 
      onclick={closeAddAgentModal}
      onkeydown={(e) => e.key === 'Escape' && closeAddAgentModal()}
      role="button"
      tabindex="0"
    ></div>
    
    <!-- Modal Content -->
    <div class="relative bg-white rounded-xl shadow-2xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
      <!-- Modal Header -->
      <div class="flex items-center justify-between p-4 border-b border-gray-200">
        <h2 class="text-lg font-semibold text-gray-800">
          {editingAgent ? 'Edit Agent' : 'Add New Agent'}
        </h2>
        <button
          onclick={closeAddAgentModal}
          class="p-1.5 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
      
      <!-- Tab Navigation -->
      <div class="border-b border-gray-200 bg-gray-50">
        <nav class="flex px-4" aria-label="Tabs">
          <button
            onclick={() => activeModalTab = 'basic'}
            class="px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeModalTab === 'basic' 
              ? 'border-blue-500 text-blue-600' 
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            Basic Settings
          </button>
          <button
            onclick={() => activeModalTab = 'advanced'}
            class="px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeModalTab === 'advanced' 
              ? 'border-blue-500 text-blue-600' 
              : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
          >
            Advanced
          </button>
        </nav>
      </div>

      <!-- Modal Body -->
      <div class="p-4 space-y-4">
        {#if activeModalTab === 'basic'}
          <!-- Basic Settings Tab -->
          <!-- Agent Name -->
          <div>
            <label for="agent-name" class="block text-sm font-medium text-gray-700 mb-1">
              Name <span class="text-red-500">*</span>
            </label>
            <input
              id="agent-name"
              type="text"
              bind:value={newAgentName}
              placeholder="e.g., Code Assistant, Task Manager"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          
          <!-- Agent Description -->
          <div>
            <label for="agent-description" class="block text-sm font-medium text-gray-700 mb-1">
              Description
            </label>
            <input
              id="agent-description"
              type="text"
              bind:value={newAgentDescription}
              placeholder="Brief description of what this agent does"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          
          <!-- Default Agent Model -->
          <div>
            <label for="agent-model" class="block text-sm font-medium text-gray-700 mb-1">
              Default Agent Model
            </label>
            {#if isLoadingModels}
              <div class="flex items-center gap-2 px-3 py-2 border border-gray-300 rounded-lg bg-gray-50">
                <div class="w-4 h-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
                <span class="text-sm text-gray-500">Loading models...</span>
              </div>
            {:else if modelsError}
              <div class="px-3 py-2 border border-red-300 rounded-lg bg-red-50">
                <p class="text-sm text-red-600">{modelsError}</p>
                <button 
                  onclick={() => { availableModels = []; loadModels(); }}
                  class="text-xs text-red-700 hover:underline mt-1"
                >
                  Retry
                </button>
              </div>
            {:else}
              <select
                id="agent-model"
                bind:value={newAgentModel}
                class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white"
              >
                <option value="">Select a model (optional)</option>
                {#each availableModels as model (model.name)}
                  <option value={model.name}>
                    {model.displayName || model.name}
                  </option>
                {/each}
              </select>
              {#if newAgentModel}
                {@const selectedModel = availableModels.find(m => m.name === newAgentModel)}
                {#if selectedModel?.description}
                  <p class="mt-1 text-xs text-gray-500">{selectedModel.description}</p>
                {/if}
              {/if}
            {/if}
          </div>
          
          <!-- System Prompt -->
          <div>
            <label for="agent-prompt" class="block text-sm font-medium text-gray-700 mb-1">
              System Prompt
            </label>
            <textarea
              id="agent-prompt"
              bind:value={newAgentSystemPrompt}
              placeholder="Instructions for the agent's behavior and capabilities..."
              rows="4"
              class="w-full px-3 py-2 border border-gray-300 rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            ></textarea>
            <p class="mt-1 text-xs text-gray-500">
              Define the agent's personality, capabilities, and constraints.
            </p>
          </div>
          
          <!-- Agent Color -->
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Color
            </label>
            <div class="flex items-center gap-2 flex-wrap">
              {#each agentColors as color}
                <button
                  onclick={() => newAgentColor = color}
                  class="w-8 h-8 rounded-full transition-all {newAgentColor === color ? 'ring-2 ring-offset-2 ring-gray-400 scale-110' : 'hover:scale-105'}"
                  style="background-color: {color}"
                  title={color}
                ></button>
              {/each}
            </div>
          </div>
          
          <!-- Preview -->
          <div class="bg-gray-50 rounded-lg p-3">
            <p class="text-xs text-gray-500 mb-2">Preview:</p>
            <div class="flex items-center gap-2">
              <span class="w-3 h-3 rounded-full" style="background-color: {newAgentColor}"></span>
              <span class="font-medium" style="color: {newAgentColor}">
                {newAgentName || 'Agent Name'}
              </span>
            </div>
            {#if newAgentDescription}
              <p class="text-sm text-gray-600 mt-1 ml-5">{newAgentDescription}</p>
            {/if}
          </div>
        {:else}
          <!-- Advanced Tab -->
          <div class="text-center py-8 text-gray-500">
            <svg class="w-12 h-12 mx-auto mb-3 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
            </svg>
            <p class="text-sm font-medium">Advanced Settings</p>
            <p class="text-xs text-gray-400 mt-1">Coming soon - configure tools, APIs, and more.</p>
          </div>
        {/if}
      </div>
      
      <!-- Modal Footer -->
      <div class="flex items-center justify-between p-4 border-t border-gray-200 bg-gray-50">
        {#if editingAgent}
          <button
            onclick={(e) => deleteAgent(editingAgent!.id, e)}
            class="px-3 py-2 text-sm font-medium text-red-600 hover:text-red-700 hover:bg-red-50 rounded-lg transition-colors"
          >
            Delete Agent
          </button>
        {:else}
          <div></div>
        {/if}
        
        <div class="flex items-center gap-2">
          <button
            onclick={closeAddAgentModal}
            class="px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
          >
            Cancel
          </button>
          <button
            onclick={saveAgent}
            disabled={!newAgentName.trim()}
            class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed"
          >
            {editingAgent ? 'Save Changes' : 'Create Agent'}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

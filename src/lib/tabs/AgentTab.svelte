<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  
  // Agent Tab - AI agent interactions
  
  // Subtab state
  type AgentSubTab = 'Chat';
  let activeSubTab: AgentSubTab = $state('Chat');
  const subTabs: { id: AgentSubTab; label: string }[] = [
    { id: 'Chat', label: 'Chat' }
  ];

  // Chat state
  interface ChatMessage {
    role: 'user' | 'model';
    content: string;
  }

  let messages: ChatMessage[] = $state([]);
  let inputMessage = $state('');
  let isLoading = $state(false);
  let error: string | null = $state(null);

  // Get API info from backend
  async function getApiInfo(): Promise<{ base_url: string; token: string }> {
    return await invoke('get_api_info');
  }

  // Send message to Gemini via backend API
  async function sendMessage() {
    if (!inputMessage.trim() || isLoading) return;

    const userMessage = inputMessage.trim();
    inputMessage = '';
    error = null;

    // Add user message to chat
    messages = [...messages, { role: 'user', content: userMessage }];

    isLoading = true;
    try {
      const apiInfo = await getApiInfo();
      
      const response = await fetch(`${apiInfo.base_url}/agent/chat`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiInfo.token}`
        },
        body: JSON.stringify({
          message: userMessage,
          history: messages.slice(0, -1) // Send history without the message we just added
        })
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || `HTTP error ${response.status}`);
      }

      const data = await response.json();
      
      // Add AI response to chat
      messages = [...messages, { role: 'model', content: data.response }];
    } catch (e) {
      const errorMessage = e instanceof Error ? e.message : 'Failed to send message';
      error = errorMessage;
      // Remove the user message if the request failed
      messages = messages.slice(0, -1);
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

  // Clear chat history
  function clearChat() {
    messages = [];
    error = null;
  }
</script>

<div class="flex-1 flex flex-col h-full bg-gray-50">
  <!-- Subtab Navigation -->
  <div class="bg-white border-b border-gray-200 px-4">
    <div class="flex gap-1">
      {#each subTabs as tab}
        <button
          onclick={() => activeSubTab = tab.id}
          class="px-4 py-2 text-sm font-medium border-b-2 transition-colors {activeSubTab === tab.id
            ? 'border-blue-500 text-blue-600'
            : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </div>

  <!-- Chat Subtab Content -->
  {#if activeSubTab === 'Chat'}
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Chat Messages Area -->
      <div class="flex-1 overflow-y-auto p-4 space-y-4">
        {#if messages.length === 0}
          <div class="flex items-center justify-center h-full">
            <div class="text-center text-gray-500">
              <svg class="w-16 h-16 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path>
              </svg>
              <h3 class="text-lg font-medium text-gray-700 mb-2">Chat with Gemini</h3>
              <p class="text-sm">Start a conversation with the AI assistant.</p>
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
                    <svg class="w-4 h-4 text-purple-500" fill="currentColor" viewBox="0 0 24 24">
                      <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
                    </svg>
                    <span class="text-xs font-medium text-purple-600">Gemini</span>
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
        <div class="flex gap-2">
          <div class="flex-1 relative">
            <textarea
              bind:value={inputMessage}
              onkeydown={handleKeyDown}
              placeholder="Type your message..."
              rows="2"
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
        <p class="mt-2 text-xs text-gray-500 text-center">
          Press Enter to send, Shift+Enter for new line
        </p>
      </div>
    </div>
  {/if}
</div>

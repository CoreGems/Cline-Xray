<!-- Chat History Sidebar -->
<script lang="ts">
  import { chatState, formatTime } from "./chatState.svelte";
</script>

<div class="flex flex-col bg-gray-100 border-r border-gray-200 transition-all duration-200 {chatState.sidebarCollapsed ? 'w-12' : 'w-64'}">
  <!-- Sidebar Header -->
  <div class="flex items-center justify-between px-3 py-1.5 border-b border-gray-200 bg-white">
    {#if !chatState.sidebarCollapsed}
      <h3 class="font-medium text-gray-700 text-sm">Chat History</h3>
    {/if}
    <div class="flex items-center gap-1 {chatState.sidebarCollapsed ? 'mx-auto' : ''}">
      {#if !chatState.sidebarCollapsed}
        <button onclick={() => chatState.createNewSession()} class="p-1.5 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors" title="New Chat">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
        </button>
      {/if}
      <button onclick={() => chatState.sidebarCollapsed = !chatState.sidebarCollapsed} class="p-1.5 text-gray-600 hover:text-gray-800 hover:bg-gray-200 rounded transition-colors" title={chatState.sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}>
        <svg class="w-5 h-5 transition-transform {chatState.sidebarCollapsed ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7"></path></svg>
      </button>
    </div>
  </div>

  {#if !chatState.sidebarCollapsed}
    <div class="flex-1 overflow-y-auto">
      {#if chatState.sessions.length === 0}
        <div class="p-4 text-center text-gray-500 text-sm">
          <p>No chat history yet.</p>
          <button onclick={() => chatState.createNewSession()} class="mt-2 text-blue-600 hover:text-blue-700 hover:underline">Start a new chat</button>
        </div>
      {:else}
        <!-- Pinned -->
        {#if chatState.pinnedSessions.length > 0}
          <div class="pt-2">
            <div class="px-3 py-1.5 flex items-center gap-1.5">
              <svg class="w-3.5 h-3.5 text-amber-500" fill="currentColor" viewBox="0 0 24 24"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z"></path></svg>
              <span class="text-xs font-semibold text-gray-500 uppercase tracking-wider">Pinned</span>
              <span class="text-xs text-gray-400">({chatState.pinnedSessions.length})</span>
            </div>
            {#each chatState.pinnedSessions as session (session.id)}
              <div role="button" tabindex="0" onclick={() => chatState.selectSession(session.id)} onkeydown={(e) => e.key === 'Enter' && chatState.selectSession(session.id)}
                class="w-full px-3 py-2 text-left hover:bg-amber-50 group transition-colors flex items-start gap-2 cursor-pointer {chatState.activeSessionId === session.id ? 'bg-blue-50 border-r-2 border-blue-500' : ''}">
                <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-amber-400" fill="currentColor" viewBox="0 0 24 24"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z"></path></svg>
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium text-gray-800 truncate">{session.title}</div>
                  <div class="text-xs text-gray-500">{formatTime(session.updatedAt)} · {session.messages.length} messages</div>
                </div>
                <div class="flex items-center gap-0.5">
                  <button onclick={(e) => chatState.togglePinSession(session.id, e)} class="p-1 text-amber-500 hover:text-amber-600 hover:bg-amber-100 rounded transition-all" title="Unpin chat">
                    <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z"></path></svg>
                  </button>
                  <button onclick={(e) => chatState.deleteSession(session.id, e)} class="p-1 opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded transition-all" title="Delete chat">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                  </button>
                </div>
              </div>
            {/each}
          </div>
          {#if chatState.unpinnedSessions.length > 0}
            <div class="mx-3 my-1 border-t border-gray-300"></div>
          {/if}
        {/if}

        <!-- Unpinned -->
        {#if chatState.unpinnedSessions.length > 0}
          <div class="py-2">
            {#if chatState.pinnedSessions.length > 0}
              <div class="px-3 py-1.5"><span class="text-xs font-semibold text-gray-500 uppercase tracking-wider">Recent</span></div>
            {/if}
            {#each chatState.unpinnedSessions as session (session.id)}
              <div role="button" tabindex="0" onclick={() => chatState.selectSession(session.id)} onkeydown={(e) => e.key === 'Enter' && chatState.selectSession(session.id)}
                class="w-full px-3 py-2 text-left hover:bg-gray-200 group transition-colors flex items-start gap-2 cursor-pointer {chatState.activeSessionId === session.id ? 'bg-blue-50 border-r-2 border-blue-500' : ''}">
                <svg class="w-4 h-4 mt-0.5 flex-shrink-0 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium text-gray-800 truncate">{session.title}</div>
                  <div class="text-xs text-gray-500">{formatTime(session.updatedAt)} · {session.messages.length} messages</div>
                </div>
                <div class="flex items-center gap-0.5">
                  <button onclick={(e) => chatState.togglePinSession(session.id, e)} class="p-1 opacity-0 group-hover:opacity-100 text-gray-400 hover:text-amber-500 hover:bg-amber-50 rounded transition-all" title="Pin chat">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 12V4h1V2H7v2h1v8l-2 2v2h5.2v6h1.6v-6H18v-2l-2-2z"></path></svg>
                  </button>
                  <button onclick={(e) => chatState.deleteSession(session.id, e)} class="p-1 opacity-0 group-hover:opacity-100 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded transition-all" title="Delete chat">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  {:else}
    <div class="flex-1 flex flex-col items-center pt-2">
      <button onclick={() => chatState.createNewSession()} class="p-2 text-gray-600 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors" title="New Chat">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path></svg>
      </button>
    </div>
  {/if}
</div>

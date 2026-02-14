<!-- Chat Messages Display Area -->
<script lang="ts">
  import { chatState, renderMarkdown } from "./chatState.svelte";
  import { stashStore } from "../../../stores/stashStore.svelte";

  interface Props { modelSelectorRef?: { getModelDisplayName(m: string): string } | undefined; }
  let { modelSelectorRef }: Props = $props();

  function getSessionTitle(): string {
    if (!chatState.activeSessionId) return 'Chat';
    const session = chatState.sessions.find(s => s.id === chatState.activeSessionId);
    return session?.title ?? 'Chat';
  }

  function toggleStash(content: string, msgIdx: number) {
    const model = modelSelectorRef?.getModelDisplayName(chatState.selectedModel) ?? chatState.selectedModel;
    // Find the preceding user question for this model answer
    let userQuestion: string | undefined;
    for (let i = msgIdx - 1; i >= 0; i--) {
      if (chatState.messages[i].role === 'user') {
        userQuestion = chatState.messages[i].displayContent ?? chatState.messages[i].content;
        break;
      }
    }
    stashStore.toggle(content, model, getSessionTitle(), userQuestion);
  }
</script>

<div class="flex-1 overflow-y-auto p-4 space-y-4" style="font-size: {chatState.chatFontSize}px;">
  {#if chatState.messages.length === 0}
    <div class="flex items-center justify-center h-full">
      <div class="text-center text-gray-500">
        <svg class="w-16 h-16 mx-auto mb-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>
        {#if chatState.activeBranding.emptyStateLogoHTML}
          <div class="flex items-center justify-center gap-2 mb-2">{@html chatState.activeBranding.emptyStateLogoHTML}</div>
        {:else}
          <div class="flex items-center justify-center gap-2 mb-2">
            <span class="text-lg font-semibold" style="color: {chatState.activeBranding.primaryColor}">{chatState.activeBranding.icon} {chatState.activeVendor?.name ?? 'AI'}</span>
          </div>
        {/if}
        <h3 class="text-lg font-medium text-gray-700 mb-2">{chatState.activeBranding.emptyStateTitle}</h3>
        <p class="text-sm">{chatState.activeBranding.poweredBy}</p>
        {#if !chatState.activeSessionId}
          <button onclick={() => chatState.createNewSession()} class="mt-4 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors text-sm">Start New Chat</button>
        {/if}
      </div>
    </div>
  {:else}
    {#each chatState.messages as message, idx}
      <div class="flex {message.role === 'user' ? 'justify-end' : 'justify-start'}">
        <div class="group/msg relative max-w-[80%] {message.role === 'user' ? 'bg-blue-600 text-white rounded-l-lg rounded-tr-lg' : 'bg-white border border-gray-200 text-gray-800 rounded-r-lg rounded-tl-lg shadow-sm'} px-4 py-3">
          <div class="flex items-center gap-2 mb-1">
            {#if message.role === 'user'}
              <span class="text-xs font-medium text-blue-200">You</span>
            {:else}
              {#if chatState.activeBranding.gradientCSS}
                <span class="text-xs font-bold bg-clip-text text-transparent" style="background-image: {chatState.activeBranding.gradientCSS}">{chatState.activeBranding.icon} {modelSelectorRef?.getModelDisplayName(chatState.selectedModel) ?? chatState.selectedModel}</span>
              {:else}
                <span class="text-xs font-bold" style="color: {chatState.activeBranding.primaryColor}">{chatState.activeBranding.icon} {modelSelectorRef?.getModelDisplayName(chatState.selectedModel) ?? chatState.selectedModel}</span>
              {/if}
            {/if}
          </div>
          {#if message.role === 'model'}
            <div class="markdown-body">{@html renderMarkdown(message.content)}</div>
            <div class="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover/msg:opacity-100 transition-opacity">
              <!-- Stash / bookmark button -->
              <button onclick={() => toggleStash(message.content, idx)} class="p-1.5 rounded-md bg-white border border-gray-200 shadow-sm hover:bg-amber-50 transition-colors" title={stashStore.isStashed(message.content) ? 'Remove from stash' : 'Stash this answer'}>
                {#if stashStore.isStashed(message.content)}
                  <svg class="w-4 h-4 text-amber-500" fill="currentColor" viewBox="0 0 24 24"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"></path></svg>
                {:else}
                  <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"></path></svg>
                {/if}
              </button>
              <!-- Copy button -->
              <button onclick={() => chatState.copyModelResponse(message.content, idx)} class="p-1.5 rounded-md bg-white border border-gray-200 shadow-sm hover:bg-gray-100 transition-colors" title={chatState.copiedMsgIdx === idx ? 'Copied!' : 'Copy response'}>
                {#if chatState.copiedMsgIdx === idx}
                  <svg class="w-4 h-4 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>
                {:else}
                  <svg class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg>
                {/if}
              </button>
            </div>
          {:else}
            {@const userText = message.displayContent ?? message.content}
            {@const isLong = userText.length > 150 || userText.split('\n').length > 3}
            {@const isExpanded = chatState.expandedUserMsgs.has(idx)}
            <div class="whitespace-pre-wrap {isLong && !isExpanded ? 'line-clamp-3' : ''}">{userText}</div>
            <div class="flex items-center gap-1.5 mt-1.5">
              {#if isLong}
                <button onclick={() => chatState.toggleUserMsgExpand(idx)} class="text-[11px] text-blue-200 hover:text-white underline underline-offset-2 transition-colors">{isExpanded ? 'Show less' : 'Show more'}</button>
              {/if}
              <button onclick={() => chatState.copyModelResponse(userText, idx)} class="ml-auto p-1 rounded-md bg-blue-500/30 hover:bg-blue-500/50 transition-colors" title={chatState.copiedMsgIdx === idx ? 'Copied!' : 'Copy message'}>
                {#if chatState.copiedMsgIdx === idx}
                  <svg class="w-3.5 h-3.5 text-green-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>
                {:else}
                  <svg class="w-3.5 h-3.5 text-blue-200" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg>
                {/if}
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/each}
    {#if chatState.isLoading}
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

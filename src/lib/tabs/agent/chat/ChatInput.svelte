<!-- Chat Input Area with Attachments -->
<script lang="ts">
  import { chatState, attachmentIcon, attachmentBg, FONT_SIZE_STEPS, MODEL_STORAGE_KEY } from "./chatState.svelte";
  import ModelSelector from "../ModelSelector.svelte";

  interface Props { modelSelectorRef?: ReturnType<typeof ModelSelector> | undefined; }
  let { modelSelectorRef = $bindable() }: Props = $props();
</script>

<!-- Error Message -->
{#if chatState.error}
  <div class="mx-4 mb-2 p-3 bg-red-50 border border-red-200 rounded-lg">
    <div class="flex items-center gap-2 text-red-700">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
      <span class="text-sm">{chatState.error}</span>
      <button onclick={() => chatState.error = null} class="ml-auto p-1 hover:bg-red-100 rounded">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
      </button>
    </div>
  </div>
{/if}

<div class="border-t border-gray-200 bg-white px-4 py-2">
  <!-- Attachment Bar -->
  {#if chatState.attachments.length > 0}
    <div class="mb-2">
      <div class="flex items-center gap-2 flex-wrap">
        {#each chatState.attachments as att (att.id)}
          <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-xs font-medium transition-colors {attachmentBg(att.type)} {chatState.previewAttachmentId === att.id ? 'ring-2 ring-offset-1 ring-blue-400' : ''}">
            <button onclick={() => chatState.togglePreview(att.id)} class="flex items-center gap-1.5 hover:opacity-80" title="Click to preview">
              <span>{attachmentIcon(att.type)}</span><span>{att.label}</span>
            </button>
            <button onclick={() => chatState.copyAttachment(att)} class="ml-0.5 p-0.5 rounded hover:bg-blue-100 hover:text-blue-600 transition-colors" title={chatState.copiedId === att.id ? 'Copied!' : 'Copy to clipboard'}>
              {#if chatState.copiedId === att.id}
                <svg class="w-3 h-3 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>
              {:else}
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg>
              {/if}
            </button>
            <button onclick={() => chatState.removeAttachment(att.id)} class="ml-0.5 p-0.5 rounded hover:bg-red-100 hover:text-red-600 transition-colors" title="Remove attachment">
              <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path></svg>
            </button>
          </div>
        {/each}
        {#if chatState.attachments.length >= 2}
          <button onclick={() => chatState.copyAllAttachments()} class="inline-flex items-center gap-1 px-2 py-1 rounded-lg border text-xs font-medium transition-colors {chatState.copiedAll ? 'bg-green-50 border-green-300 text-green-700' : 'bg-gray-50 border-gray-300 text-gray-600 hover:bg-gray-100 hover:border-gray-400'}" title={chatState.copiedAll ? 'All copied!' : 'Copy all attachments to clipboard'}>
            {#if chatState.copiedAll}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg><span>Copied!</span>
            {:else}
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg><span>Copy All</span>
            {/if}
          </button>
        {/if}
      </div>
      <!-- Preview Panel -->
      {#if chatState.previewAttachmentId}
        {@const previewAtt = chatState.attachments.find(a => a.id === chatState.previewAttachmentId)}
        {#if previewAtt}
          <div class="mt-2 border border-gray-200 rounded-lg overflow-hidden bg-gray-50">
            <div class="px-3 py-1.5 bg-gray-100 border-b border-gray-200 flex items-center justify-between">
              <span class="text-[10px] font-bold text-gray-500 uppercase tracking-wide">{attachmentIcon(previewAtt.type)} {previewAtt.label}</span>
              <span class="text-[10px] text-gray-400">{(previewAtt.content.length / 1024).toFixed(1)} KB</span>
            </div>
            <pre class="max-h-48 overflow-auto p-3 text-[10px] leading-relaxed font-mono text-gray-700 whitespace-pre-wrap break-all m-0">{previewAtt.content.length > 50000 ? previewAtt.content.substring(0, 50000) + '\n\n... [truncated for preview]' : previewAtt.content}</pre>
          </div>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="flex gap-2 items-stretch">
    <div class="flex-1 relative">
      <textarea bind:value={chatState.inputMessage} onkeydown={(e) => chatState.handleKeyDown(e)} placeholder="Type your message..." rows="2" class="w-full h-full px-3 py-2 border border-gray-300 rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed text-sm" disabled={chatState.isLoading}></textarea>
    </div>
    <div class="flex flex-col gap-2">
      <button onclick={() => chatState.sendMessage()} disabled={chatState.isLoading || !chatState.inputMessage.trim()} class="px-3 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors" title="Send message">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path></svg>
      </button>
      {#if chatState.messages.length > 0}
        <button onclick={() => chatState.clearChatHistory()} disabled={chatState.isLoading} class="px-3 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2 disabled:cursor-not-allowed transition-colors" title="Clear chat">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
        </button>
      {/if}
    </div>
  </div>
  <!-- Model Selector + Controls Row -->
  <div class="mt-1 flex items-center gap-2">
    <ModelSelector bind:selectedModel={chatState.selectedModel} bind:this={modelSelectorRef} storageKey={MODEL_STORAGE_KEY} />
    <button onclick={() => chatState.toggleChatHistory()} class="inline-flex items-center gap-1.5 px-2 py-1 rounded-md border text-xs font-medium transition-all {chatState.chatHistoryEnabled ? 'bg-green-50 border-green-300 text-green-700 hover:bg-green-100' : 'bg-gray-50 border-gray-300 text-gray-500 hover:bg-gray-100'}" title={chatState.chatHistoryEnabled ? 'Chat history ON — AI sees previous messages. Click to disable.' : 'Chat history OFF — each message is standalone. Click to enable.'}>
      <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
      <span>History {chatState.chatHistoryEnabled ? 'ON' : 'OFF'}</span>
    </button>
    {#if chatState.chatHistoryEnabled && chatState.messages.length > 0}
      <button onclick={() => chatState.clearChatHistory()} disabled={chatState.isLoading} class="inline-flex items-center gap-1 px-2 py-1 rounded-md border border-red-200 text-xs font-medium text-red-600 bg-red-50 hover:bg-red-100 transition-colors disabled:opacity-50 disabled:cursor-not-allowed" title="Clear conversation history">
        <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
        <span>Clear</span>
      </button>
    {/if}
    <div class="inline-flex items-center rounded-md border border-gray-300 bg-gray-50 overflow-hidden" title="Adjust chat font size ({chatState.chatFontSize}px)">
      <button onclick={() => chatState.decreaseFontSize()} disabled={chatState.chatFontSize <= FONT_SIZE_STEPS[0]} class="px-1.5 py-1 text-xs font-bold text-gray-600 hover:bg-gray-200 disabled:opacity-30 disabled:cursor-not-allowed transition-colors" title="Decrease font size">A−</button>
      <span class="px-1 py-1 text-[10px] text-gray-500 border-x border-gray-300 bg-white min-w-[28px] text-center">{chatState.chatFontSize}</span>
      <button onclick={() => chatState.increaseFontSize()} disabled={chatState.chatFontSize >= FONT_SIZE_STEPS[FONT_SIZE_STEPS.length - 1]} class="px-1.5 py-1 text-xs font-bold text-gray-600 hover:bg-gray-200 disabled:opacity-30 disabled:cursor-not-allowed transition-colors" title="Increase font size">A+</button>
    </div>
    <span class="flex-1 text-xs text-gray-500 text-right">Press Enter to send, Shift+Enter for new line</span>
  </div>
</div>

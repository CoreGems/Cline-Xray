<script lang="ts">
  import { untrack } from "svelte";
  import { chatState } from "./chat/chatState.svelte";
  import { navigationStore } from "../../stores/navigationStore.svelte";
  import ChatSidebar from "./chat/ChatSidebar.svelte";
  import AgentSelector from "./chat/AgentSelector.svelte";
  import AgentModal from "./chat/AgentModal.svelte";
  import ChatMessages from "./chat/ChatMessages.svelte";
  import ChatInput from "./chat/ChatInput.svelte";
  import ModelSelector from "./ModelSelector.svelte";

  let modelSelectorRef: ReturnType<typeof ModelSelector> | undefined = $state(undefined);

  // Initialize on first render
  $effect(() => {
    if (!chatState.initialized) {
      untrack(() => {
        chatState.initialize();
        setTimeout(() => modelSelectorRef?.loadModels(), 0);
      });
    }
  });

  // Save draft on destroy
  $effect(() => {
    return () => { chatState.saveDraftOnDestroy(); };
  });

  // Check for incoming "Ask LLM" payload
  $effect(() => {
    if (chatState.initialized && navigationStore.hasPendingPayload) {
      untrack(() => { chatState.checkPendingPayload(); });
    }
  });
</script>

<div class="flex-1 flex h-full overflow-hidden">
  <ChatSidebar />

  <!-- Right Panel - Chat Area -->
  <div class="flex-1 flex flex-col overflow-hidden bg-gray-50">
    <AgentSelector />
    <ChatMessages {modelSelectorRef} />
    <ChatInput bind:modelSelectorRef />
  </div>
</div>

<AgentModal />

<style>
  :global(.markdown-body) { line-height: 1.6; word-wrap: break-word; }
  :global(.markdown-body h1), :global(.markdown-body h2), :global(.markdown-body h3), :global(.markdown-body h4) { margin-top: 0.75em; margin-bottom: 0.35em; font-weight: 600; line-height: 1.3; }
  :global(.markdown-body h1) { font-size: 1.25em; }
  :global(.markdown-body h2) { font-size: 1.15em; }
  :global(.markdown-body h3) { font-size: 1.05em; }
  :global(.markdown-body p) { margin-top: 0.4em; margin-bottom: 0.4em; }
  :global(.markdown-body ul), :global(.markdown-body ol) { padding-left: 1.5em; margin-top: 0.3em; margin-bottom: 0.3em; }
  :global(.markdown-body li) { margin-bottom: 0.15em; }
  :global(.markdown-body ul) { list-style-type: disc; }
  :global(.markdown-body ol) { list-style-type: decimal; }
  :global(.markdown-body code) { background-color: rgba(0, 0, 0, 0.06); padding: 0.15em 0.35em; border-radius: 3px; font-size: 0.9em; font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace; }
  :global(.markdown-body pre) { background-color: #f6f8fa; border: 1px solid #e1e4e8; border-radius: 6px; padding: 0.75em 1em; overflow-x: auto; margin: 0.5em 0; }
  :global(.markdown-body pre code) { background: none; padding: 0; font-size: 0.85em; line-height: 1.5; }
  :global(.markdown-body blockquote) { border-left: 3px solid #d1d5db; padding-left: 0.75em; margin: 0.5em 0; color: #6b7280; }
  :global(.markdown-body strong) { font-weight: 600; }
  :global(.markdown-body a) { color: #2563eb; text-decoration: underline; }
  :global(.markdown-body a:hover) { color: #1d4ed8; }
  :global(.markdown-body hr) { border: none; border-top: 1px solid #e5e7eb; margin: 0.75em 0; }
  :global(.markdown-body table) { border-collapse: collapse; width: 100%; margin: 0.5em 0; font-size: 0.9em; }
  :global(.markdown-body th), :global(.markdown-body td) { border: 1px solid #e5e7eb; padding: 0.35em 0.65em; text-align: left; }
  :global(.markdown-body th) { background-color: #f9fafb; font-weight: 600; }
  :global(.markdown-body > *:first-child) { margin-top: 0; }
  :global(.markdown-body > *:last-child) { margin-bottom: 0; }
</style>

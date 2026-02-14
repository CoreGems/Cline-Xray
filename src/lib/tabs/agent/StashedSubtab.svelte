<!-- Stashed Answers — "Best Answers" viewer -->
<script lang="ts">
  import { stashStore } from "../../stores/stashStore.svelte";
  import { marked } from "marked";

  let expandedId = $state<string | null>(null);
  let questionExpandedIds = $state<Set<string>>(new Set());
  let editingNoteId = $state<string | null>(null);
  let noteText = $state('');
  let copiedId = $state<string | null>(null);
  let confirmClearAll = $state(false);
  let addingTagId = $state<string | null>(null);
  let newTagText = $state('');

  function renderMarkdown(content: string): string {
    return marked.parse(content) as string;
  }

  function formatDate(ts: number): string {
    const d = new Date(ts);
    const now = new Date();
    const diffDays = Math.floor((now.getTime() - d.getTime()) / (1000 * 60 * 60 * 24));
    if (diffDays === 0) return 'Today ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    if (diffDays === 1) return 'Yesterday ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    if (diffDays < 7) return d.toLocaleDateString([], { weekday: 'short' }) + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    return d.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function toggleExpand(id: string) {
    expandedId = expandedId === id ? null : id;
  }

  function toggleQuestion(id: string) {
    const next = new Set(questionExpandedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    questionExpandedIds = next;
  }

  function startEditNote(id: string, currentNote: string) {
    editingNoteId = id;
    noteText = currentNote;
  }

  function saveNote(id: string) {
    stashStore.updateNote(id, noteText.trim());
    editingNoteId = null;
    noteText = '';
  }

  function cancelEditNote() {
    editingNoteId = null;
    noteText = '';
  }

  async function copyContent(content: string, id: string) {
    try {
      await navigator.clipboard.writeText(content);
      copiedId = id;
      setTimeout(() => { copiedId = null; }, 1500);
    } catch (e) { console.error('Failed to copy:', e); }
  }

  function startAddTag(id: string) {
    addingTagId = id;
    newTagText = '';
  }

  function commitTag(id: string) {
    const tag = newTagText.trim();
    if (tag) {
      stashStore.addTag(id, tag);
    }
    newTagText = '';
    addingTagId = null;
  }

  function cancelAddTag() {
    addingTagId = null;
    newTagText = '';
  }

  function handleClearAll() {
    if (confirmClearAll) {
      stashStore.clearAll();
      confirmClearAll = false;
    } else {
      confirmClearAll = true;
      setTimeout(() => { confirmClearAll = false; }, 3000);
    }
  }
</script>

<div class="flex-1 flex flex-col h-full overflow-hidden bg-gray-50">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 bg-white border-b border-gray-200">
    <div class="flex items-center gap-2">
      <svg class="w-5 h-5 text-amber-500" fill="currentColor" viewBox="0 0 24 24"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"></path></svg>
      <h2 class="text-sm font-semibold text-gray-800">Stashed Answers</h2>
      <span class="text-xs text-gray-500 bg-gray-100 px-2 py-0.5 rounded-full">{stashStore.count}</span>
    </div>
    {#if stashStore.count > 0}
      <button onclick={handleClearAll} class="text-xs px-2 py-1 rounded transition-colors {confirmClearAll ? 'bg-red-100 text-red-700 hover:bg-red-200' : 'text-gray-500 hover:text-red-600 hover:bg-red-50'}">
        {confirmClearAll ? 'Click again to confirm' : 'Clear all'}
      </button>
    {/if}
  </div>

  <!-- Search -->
  {#if stashStore.count > 2}
    <div class="px-4 py-2 bg-white border-b border-gray-100">
      <div class="relative">
        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path></svg>
        <input
          type="text"
          placeholder="Search stashed answers..."
          class="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-amber-300 focus:border-amber-300"
          value={stashStore.searchQuery}
          oninput={(e) => { stashStore.searchQuery = (e.target as HTMLInputElement).value; }}
        />
      </div>
    </div>
  {/if}

  <!-- Tag filter bar -->
  {#if stashStore.allTags.length > 0}
    <div class="px-4 py-2 bg-white border-b border-gray-100 flex items-center gap-1.5 flex-wrap">
      <span class="text-xs text-gray-500 mr-1">Tags:</span>
      <button onclick={() => { stashStore.activeTagFilter = null; }} class="text-[11px] px-2 py-0.5 rounded-full border transition-colors {stashStore.activeTagFilter === null ? 'bg-amber-100 border-amber-300 text-amber-800 font-medium' : 'bg-gray-50 border-gray-200 text-gray-500 hover:bg-gray-100'}">All</button>
      {#each stashStore.allTags as tag}
        <button onclick={() => { stashStore.activeTagFilter = stashStore.activeTagFilter === tag ? null : tag; }} class="text-[11px] px-2 py-0.5 rounded-full border transition-colors {stashStore.activeTagFilter === tag ? 'bg-purple-100 border-purple-300 text-purple-800 font-medium' : 'bg-gray-50 border-gray-200 text-gray-600 hover:bg-purple-50 hover:border-purple-200'}">{tag}</button>
      {/each}
    </div>
  {/if}

  <!-- Content -->
  <div class="flex-1 overflow-y-auto">
    {#if stashStore.count === 0}
      <div class="flex items-center justify-center h-full">
        <div class="text-center text-gray-500 px-8">
          <svg class="w-16 h-16 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"></path></svg>
          <h3 class="text-lg font-medium text-gray-600 mb-2">No stashed answers yet</h3>
          <p class="text-sm text-gray-400">Hover over any model answer in Chat and click the ⭐ star button to stash it here for later review.</p>
        </div>
      </div>
    {:else if stashStore.filteredItems.length === 0}
      <div class="flex items-center justify-center h-48">
        <p class="text-sm text-gray-400">No matches for "{stashStore.searchQuery}"</p>
      </div>
    {:else}
      <div class="p-4 space-y-5">
        {#each stashStore.groupedBySession as group (group.sessionTitle)}
          <!-- Session group -->
          <div>
            <!-- Session header -->
            <div class="flex items-center gap-2 mb-2 px-1">
              <svg class="w-4 h-4 text-gray-400 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"></path></svg>
              <span class="text-xs font-semibold text-gray-600 uppercase tracking-wide">{group.sessionTitle}</span>
              <span class="text-[10px] text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded-full">{group.items.length}</span>
              <div class="flex-1 border-t border-gray-200 ml-2"></div>
            </div>
            <!-- Cards in this session -->
            <div class="space-y-2 pl-2 border-l-2 border-gray-200 ml-2">
              {#each group.items as item (item.id)}
                <div class="bg-white rounded-lg border border-gray-200 shadow-sm hover:shadow transition-shadow overflow-hidden">
                  <!-- Card Header -->
                  <div class="flex items-start gap-3 px-4 py-3">
                    <button onclick={() => toggleExpand(item.id)} class="flex-1 text-left min-w-0">
                      <div class="flex items-center gap-2 mb-1">
                        <span class="text-xs font-medium text-amber-600 bg-amber-50 px-1.5 py-0.5 rounded">{item.model}</span>
                        <span class="text-xs text-gray-400 ml-auto flex-shrink-0">{formatDate(item.stashedAt)}</span>
                      </div>
                      {#if item.note}
                        <div class="text-xs font-medium text-blue-600 mb-1">📌 {item.note}</div>
                      {/if}
                      <p class="text-sm text-gray-600 {expandedId === item.id ? '' : 'line-clamp-2'}">{item.preview}</p>
                    </button>
                    <div class="flex items-center gap-1 flex-shrink-0 pt-0.5">
                      <button onclick={() => startEditNote(item.id, item.note)} class="p-1.5 rounded-md text-gray-400 hover:text-blue-600 hover:bg-blue-50 transition-colors" title="Add/edit note">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path></svg>
                      </button>
                      <button onclick={() => copyContent(item.content, item.id)} class="p-1.5 rounded-md text-gray-400 hover:text-gray-700 hover:bg-gray-100 transition-colors" title={copiedId === item.id ? 'Copied!' : 'Copy markdown'}>
                        {#if copiedId === item.id}
                          <svg class="w-4 h-4 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path></svg>
                        {:else}
                          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg>
                        {/if}
                      </button>
                      <button onclick={() => toggleExpand(item.id)} class="p-1.5 rounded-md text-gray-400 hover:text-gray-700 hover:bg-gray-100 transition-colors" title={expandedId === item.id ? 'Collapse' : 'Expand'}>
                        <svg class="w-4 h-4 transition-transform {expandedId === item.id ? 'rotate-180' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
                      </button>
                      <button onclick={() => stashStore.unstash(item.id)} class="p-1.5 rounded-md text-gray-400 hover:text-red-600 hover:bg-red-50 transition-colors" title="Remove from stash">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                      </button>
                    </div>
                  </div>

                  <!-- Inline note editor -->
                  {#if editingNoteId === item.id}
                    <div class="px-4 pb-3 flex items-center gap-2">
                      <input type="text" placeholder="Add a note (e.g., 'Great summary of auth flow')" class="flex-1 text-sm border border-blue-300 rounded-md px-2 py-1 focus:outline-none focus:ring-2 focus:ring-blue-300" value={noteText} oninput={(e) => { noteText = (e.target as HTMLInputElement).value; }} onkeydown={(e) => { if (e.key === 'Enter') saveNote(item.id); if (e.key === 'Escape') cancelEditNote(); }} />
                      <button onclick={() => saveNote(item.id)} class="text-xs px-2 py-1 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors">Save</button>
                      <button onclick={cancelEditNote} class="text-xs px-2 py-1 text-gray-500 hover:text-gray-700 transition-colors">Cancel</button>
                    </div>
                  {/if}

                  <!-- Tags + Show question (single row) -->
                  <div class="px-4 pb-1.5 flex items-center gap-1 flex-wrap">
                    {#if item.tags && item.tags.length > 0}
                      {#each item.tags as tag}
                        <span class="inline-flex items-center gap-0.5 text-[11px] px-1.5 py-0.5 rounded-full bg-purple-50 border border-purple-200 text-purple-700">
                          {tag}
                          <button onclick={() => stashStore.removeTag(item.id, tag)} class="ml-0.5 hover:text-red-600 transition-colors" title="Remove tag">×</button>
                        </span>
                      {/each}
                    {/if}
                    {#if addingTagId === item.id}
                      <input type="text" placeholder="tag name" class="text-[11px] w-20 px-1.5 py-0.5 border border-purple-300 rounded-full focus:outline-none focus:ring-1 focus:ring-purple-300" value={newTagText} oninput={(e) => { newTagText = (e.target as HTMLInputElement).value; }} onkeydown={(e) => { if (e.key === 'Enter') commitTag(item.id); if (e.key === 'Escape') cancelAddTag(); }} />
                    {:else}
                      <button onclick={() => startAddTag(item.id)} class="text-[11px] px-1.5 py-0.5 rounded-full border border-dashed border-gray-300 text-gray-400 hover:text-purple-600 hover:border-purple-300 transition-colors" title="Add tag">+ tag</button>
                    {/if}
                    {#if item.userQuestion}
                      <button onclick={() => toggleQuestion(item.id)} class="ml-auto inline-flex items-center gap-1 text-[11px] text-blue-600 hover:text-blue-800 transition-colors">
                        <svg class="w-3 h-3 transition-transform {questionExpandedIds.has(item.id) ? 'rotate-90' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"></path></svg>
                        {questionExpandedIds.has(item.id) ? 'Hide question' : 'Show question'}
                      </button>
                    {/if}
                  </div>
                  {#if item.userQuestion && questionExpandedIds.has(item.id)}
                    <div class="px-4 pb-1.5">
                      <div class="px-3 py-2 bg-blue-50 border border-blue-200 rounded-md text-sm text-blue-900 whitespace-pre-wrap">{item.userQuestion}</div>
                    </div>
                  {/if}

                  <!-- Expanded markdown content -->
                  {#if expandedId === item.id}
                    <div class="border-t border-gray-100 px-4 py-3 bg-gray-50/50">
                      <div class="stash-markdown-body text-sm">{@html renderMarkdown(item.content)}</div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  :global(.stash-markdown-body) { line-height: 1.6; word-wrap: break-word; color: #374151; }
  :global(.stash-markdown-body h1), :global(.stash-markdown-body h2), :global(.stash-markdown-body h3), :global(.stash-markdown-body h4) { margin-top: 0.75em; margin-bottom: 0.35em; font-weight: 600; line-height: 1.3; }
  :global(.stash-markdown-body h1) { font-size: 1.25em; }
  :global(.stash-markdown-body h2) { font-size: 1.15em; }
  :global(.stash-markdown-body h3) { font-size: 1.05em; }
  :global(.stash-markdown-body p) { margin-top: 0.4em; margin-bottom: 0.4em; }
  :global(.stash-markdown-body ul), :global(.stash-markdown-body ol) { padding-left: 1.5em; margin-top: 0.3em; margin-bottom: 0.3em; }
  :global(.stash-markdown-body li) { margin-bottom: 0.15em; }
  :global(.stash-markdown-body ul) { list-style-type: disc; }
  :global(.stash-markdown-body ol) { list-style-type: decimal; }
  :global(.stash-markdown-body code) { background-color: rgba(0, 0, 0, 0.06); padding: 0.15em 0.35em; border-radius: 3px; font-size: 0.9em; font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace; }
  :global(.stash-markdown-body pre) { background-color: #f6f8fa; border: 1px solid #e1e4e8; border-radius: 6px; padding: 0.75em 1em; overflow-x: auto; margin: 0.5em 0; }
  :global(.stash-markdown-body pre code) { background: none; padding: 0; font-size: 0.85em; line-height: 1.5; }
  :global(.stash-markdown-body blockquote) { border-left: 3px solid #d1d5db; padding-left: 0.75em; margin: 0.5em 0; color: #6b7280; }
  :global(.stash-markdown-body strong) { font-weight: 600; }
  :global(.stash-markdown-body a) { color: #2563eb; text-decoration: underline; }
  :global(.stash-markdown-body a:hover) { color: #1d4ed8; }
  :global(.stash-markdown-body hr) { border: none; border-top: 1px solid #e5e7eb; margin: 0.75em 0; }
  :global(.stash-markdown-body table) { border-collapse: collapse; width: 100%; margin: 0.5em 0; font-size: 0.9em; }
  :global(.stash-markdown-body th), :global(.stash-markdown-body td) { border: 1px solid #e5e7eb; padding: 0.35em 0.65em; text-align: left; }
  :global(.stash-markdown-body th) { background-color: #f9fafb; font-weight: 600; }
  :global(.stash-markdown-body > *:first-child) { margin-top: 0; }
  :global(.stash-markdown-body > *:last-child) { margin-bottom: 0; }
</style>

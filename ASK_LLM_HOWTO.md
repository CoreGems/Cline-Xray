# Ask LLM Feature — Implementation Guide

## Overview

The "Ask LLM" button in **Changes → Latest** sends all task artifacts (prompts, file list, unified diff) to the **Agent → Chat** subtab as pre-attached context.

## Data Flow

```
LatestSubtab.svelte  →  navigationStore  →  App.svelte (tab switch)  →  AgentTab (subtab switch)  →  ChatSubtab ($effect consumes payload)
     [askLlm()]         [navigateToChat()]     [reads activeTab]         [reads activeAgentSubTab]      [creates session + attaches]
```

## Files Modified

| File | Role |
|------|------|
| `src/lib/stores/navigationStore.svelte.ts` | Svelte 5 runes singleton — holds `activeTab`, `activeAgentSubTab`, chat payload |
| `src/lib/stores/index.ts` | Re-exports `navigationStore` |
| `src/lib/tabs/agent/types.ts` | `ChatAttachment` interface, `ChatSession.attachments?` field |
| `src/App.svelte` | Tab switching reads from `navigationStore.activeTab` |
| `src/lib/tabs/agent/AgentTab.svelte` | Subtab switching reads from `navigationStore.activeAgentSubTab` |
| `src/lib/tabs/agent/ChatSubtab.svelte` | Attachment bar UI, `$effect` to consume payload, context injection on send |
| `src/lib/tabs/changes/LatestSubtab.svelte` | "🤖 Ask LLM" button + `askLlm()` function |

## What Gets Attached

The `askLlm()` function in LatestSubtab builds up to **3 attachments** (only when data exists):

| # | Type | Label Example | Condition |
|---|------|--------------|-----------|
| 1 | `prompts` | "All Prompts (3)" | Always (from `data.subtasks` or `data.prompt`) |
| 2 | `files` | "Changed Files (12)" | Only when `data.diff` is non-null and has files |
| 3 | `diff` | "Unified Diff (45.2KB)" | Only when `data.diff` has a `patch` string |

**Important:** If `data.diff` is `null` (e.g. `noDiffReason: 'no_checkpoint_workspace'`), only prompts will be attached. Check the browser console for `[askLlm]` logs to diagnose.

## ChatAttachment Type

```typescript
export interface ChatAttachment {
  id: string;           // unique key, e.g. "prompts-1707494084000"
  label: string;        // display label in chip
  type: 'prompts' | 'files' | 'diff' | 'generic';
  content: string;      // the actual text content
  meta?: Record<string, any>;  // optional metadata (count, sizeKB, etc.)
}
```

## Navigation Store

```typescript
// Producer (LatestSubtab):
navigationStore.navigateToChat({
  attachments: [...],
  timestamp: Date.now()
});

// Consumer (ChatSubtab):
$effect(() => {
  if (initialized && navigationStore.hasPendingPayload) {
    const payload = navigationStore.consumeChatPayload();
    // creates new session, sets attachments
  }
});
```

## Chat UI — Attachment Bar

When attachments are present, a horizontal bar of color-coded chips appears above the text input:

- 📝 **Prompts** — blue chip (`bg-blue-50`)
- 📄 **Files** — green chip (`bg-green-50`)
- 📦 **Diff** — amber chip (`bg-amber-50`)

Each chip is **clickable** (toggles a preview panel) and has an **✕** button to remove.

On the **first message** of the session, all attachment content is prepended as context to the message sent to the LLM API.

## Session Persistence

Attachments are stored on `ChatSession.attachments` in localStorage. When switching between sessions, attachments are restored/cleared accordingly.

## Debugging

Open browser DevTools console and look for:
```
[askLlm] data.diff: 12 files, 46231 bytes patch    ← diff data exists
[askLlm] attachments to send: 3 ["All Prompts (3)", "Changed Files (12)", "Unified Diff (45.2KB)"]
```

If you see:
```
[askLlm] data.diff: NULL (no checkpoint)
[askLlm] attachments to send: 1 ["All Prompts (3)"]
```

Then the task has no checkpoint workspace / diff data — only prompts can be attached.

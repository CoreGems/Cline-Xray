# SUBTASK_FI — Subtask Detection from `<feedback>` Tags

## Problem Statement

When we read conversation history for a Cline task, we correctly identify the **first prompt** (the initial `<task>` tag). However, tasks are often multi-phase — the user provides additional instructions via `<feedback>` tags **after** seeing the initial result. These feedback-driven prompts are essentially **subtasks** that break a single Cline task into logical segments, but our backend and UI currently ignore them.

**Example from `task_sample/ui_messages.json`:**

```
Task:        "write simple python script to generate fibonacchi number (do not test)"
Subtask #1:  "now write java script doing the same . do not test"   ← <feedback> tag
```

We need to detect, extract, and surface these subtasks in both the REST API and the UI.

---

## Data Sources & Detection Strategy

### Two data sources contain subtask information:

#### 1. `ui_messages.json` (PRIMARY — clean, structured)

Subtask signals appear as distinct UI message entries:

| `say` field     | Meaning                        | Example `text`                                          |
|-----------------|--------------------------------|---------------------------------------------------------|
| `"task"`        | Initial task prompt            | `"write simple python script to generate fibonacchi…"`  |
| `"user_feedback"` | Subsequent subtask prompt   | `"now write java script doing the same . do not test"`  |

Key fields per entry:
- `ts` — epoch ms timestamp
- `say` — `"task"` or `"user_feedback"`
- `text` — the raw prompt text
- `conversationHistoryIndex` — which `api_conversation_history` message was active when this happened

The `conversationHistoryIndex` on a `"task"` entry is `-1` (before any API messages).
The `conversationHistoryIndex` on a `"user_feedback"` entry is the **last message index of the PREVIOUS subtask** — meaning the next subtask's API messages start at `conversationHistoryIndex + 1`.

#### 2. `api_conversation_history.json` (SECONDARY — embedded in text blocks)

Subtask prompts appear as `<feedback>` XML tags inside user-role text blocks:

```json
{
  "role": "user",
  "content": [
    {"type":"text","text":"[attempt_completion] Result: Done"},
    {"type":"text","text":"Command executed.\nOutput:\n..."},
    {"type":"text","text":"The user has provided feedback on the results..."},
    {"type":"text","text":"<feedback>\nnow write java script doing the same . do not test\n</feedback>"},
    {"type":"text","text":"<environment_details>..."}
  ]
}
```

The initial task appears as a `<task>` tag in the first user message:

```json
{"type":"text","text":"<task>\nwrite simple python script to generate fibonacchi number (do not test)\n</task>"}
```

### Chosen Strategy: Parse `ui_messages.json` (primary), cross-reference `api_conversation_history.json`

**Why `ui_messages.json` is better:**
- Clean separation: `say: "task"` vs `say: "user_feedback"` — no regex needed
- Clean text: the prompt text is already extracted, no XML wrapper to strip
- Has the `conversationHistoryIndex` linking back to the API history, giving us message boundaries

**Why `api_conversation_history.json` as secondary:**
- Provides the `<feedback>` tag content that can be cross-validated
- Gives us exact message index boundaries for each subtask's conversation segment
- Can serve as fallback if `ui_messages.json` is missing or corrupt

---

## Subtask Boundary Model

A task's conversation is split into **subtask segments** based on feedback boundaries:

```
┌──────────────────────────────────────────────────────┐
│ TASK: "write simple python script to generate fib…"  │
│                                                      │
│  api_conversation_history messages: 0 → 3            │
│  ├─ msg 0: user (task prompt)                        │
│  ├─ msg 1: assistant (writes fibonacci.py)           │
│  ├─ msg 2: user (tool result: file saved)            │
│  └─ msg 3: assistant (attempt_completion)            │
│                                                      │
│  Tools: write_to_file, attempt_completion            │
│  Files created: fibonacci.py                         │
├──────────────────────────────────────────────────────┤
│ SUBTASK #1: "now write java script doing the same…"  │
│                                                      │
│  api_conversation_history messages: 4 → 7            │
│  ├─ msg 4: user (feedback + env)                     │
│  ├─ msg 5: assistant (writes fibonacci.js)           │
│  ├─ msg 6: user (tool result: file saved)            │
│  └─ msg 7: assistant (attempt_completion)            │
│                                                      │
│  Tools: write_to_file, attempt_completion            │
│  Files created: fibonacci.js                         │
└──────────────────────────────────────────────────────┘
```

**Boundary rule:** A subtask's API messages start at `prev_subtask.conversationHistoryIndex + 1` and end at `next_subtask.conversationHistoryIndex` (or end of array).

---

## Current `RawUiMessage` Type — What Needs to Change

Current definition (in `types.rs`):

```rust
pub struct RawUiMessage {
    pub ts: u64,
    pub msg_type: Option<String>,    // "say" or "ask"
    pub conversation_history_index: Option<i64>,
}
```

**Missing fields for subtask detection:**
- `say: Option<String>` — needed to identify `"task"` vs `"user_feedback"` vs other
- `text: Option<String>` — needed to extract the subtask prompt text

These fields can be added as `#[serde(default)]` optional fields without breaking existing parsing.

---

## Proposed Types

### Rust (backend)

```rust
/// A detected subtask within a Cline task conversation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskEntry {
    /// Subtask index (0 = initial task, 1+ = feedback-driven subtasks)
    pub subtask_index: usize,
    /// The subtask prompt text
    pub prompt: String,
    /// ISO 8601 timestamp when this subtask was issued
    pub timestamp: String,
    /// Whether this is the initial task (true) or a feedback subtask (false)
    pub is_initial_task: bool,
    /// First message index in api_conversation_history for this subtask
    pub message_range_start: usize,
    /// Last message index (inclusive). None if extends to end of conversation.
    pub message_range_end: Option<usize>,
    /// Number of API messages in this subtask's range
    pub message_count: usize,
    /// Number of tool calls within this subtask's message range
    pub tool_call_count: usize,
    /// Tool names used in this subtask (e.g. ["write_to_file", "attempt_completion"])
    pub tools_used: Vec<String>,
}

/// Response for GET /history/tasks/:taskId/subtasks
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtasksResponse {
    /// Task ID
    pub task_id: String,
    /// Total number of subtasks (including initial task)
    pub total_subtasks: usize,
    /// Whether this task has any feedback-driven subtasks (total > 1)
    pub has_subtasks: bool,
    /// The detected subtasks in chronological order
    pub subtasks: Vec<SubtaskEntry>,
}
```

### TypeScript (frontend)

```typescript
export interface SubtaskEntry {
  subtaskIndex: number;
  prompt: string;
  timestamp: string;
  isInitialTask: boolean;
  messageRangeStart: number;
  messageRangeEnd: number | null;
  messageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
}

export interface SubtasksResponse {
  taskId: string;
  totalSubtasks: number;
  hasSubtasks: boolean;
  subtasks: SubtaskEntry[];
}
```

---

## Proposed API Endpoint

```
GET /history/tasks/{task_id}/subtasks
```

**Response:** `SubtasksResponse`

**Parsing flow:**
1. Read `ui_messages.json` → find all entries where `say = "task"` or `say = "user_feedback"`
2. For each entry, extract `text`, `ts`, `conversationHistoryIndex`
3. Compute message ranges using `conversationHistoryIndex` boundaries
4. Read `api_conversation_history.json` → count tool calls within each range
5. Build and return `SubtasksResponse`

---

## Parsing Algorithm (pseudocode)

```
fn parse_subtasks(task_id):
    ui_msgs = read_json("ui_messages.json")
    api_msgs = read_json("api_conversation_history.json")
    total_api_msgs = len(api_msgs)
    
    # Step 1: Collect subtask markers from ui_messages
    markers = []
    for msg in ui_msgs:
        if msg.say == "task":
            markers.push({
                prompt: msg.text,
                ts: msg.ts,
                conv_idx: msg.conversationHistoryIndex,  # typically -1
                is_initial: true
            })
        elif msg.say == "user_feedback":
            markers.push({
                prompt: msg.text,
                ts: msg.ts,
                conv_idx: msg.conversationHistoryIndex,
                is_initial: false
            })
    
    # Step 2: Compute message ranges
    subtasks = []
    for i, marker in enumerate(markers):
        if marker.is_initial:
            range_start = 0
        else:
            range_start = marker.conv_idx + 1  # feedback appears AFTER this index
        
        if i + 1 < len(markers):
            range_end = markers[i + 1].conv_idx  # exclusive boundary
        else:
            range_end = total_api_msgs - 1  # last subtask goes to end
        
        # Step 3: Count tools in this range
        tool_count, tools_used = count_tools_in_range(api_msgs, range_start, range_end)
        
        subtasks.push(SubtaskEntry {
            subtask_index: i,
            prompt: marker.prompt,
            timestamp: epoch_to_iso(marker.ts),
            is_initial_task: marker.is_initial,
            message_range_start: range_start,
            message_range_end: range_end,
            message_count: range_end - range_start + 1,
            tool_call_count: tool_count,
            tools_used: tools_used,
        })
    
    return SubtasksResponse { subtasks, total: len(subtasks), has_subtasks: len(subtasks) > 1 }
```

### Fallback: `<feedback>` regex on `api_conversation_history.json`

If `ui_messages.json` is missing, parse user-role messages in the API history for:
- `<task>\n...\n</task>` → initial task prompt
- `<feedback>\n...\n</feedback>` → subtask prompt

Regex: `<feedback>\s*(.*?)\s*</feedback>` (dotall mode)

---

## Files to Modify (implementation plan)

### Backend (Rust)

| File | Change |
|------|--------|
| `types.rs` | Add `SubtaskEntry`, `SubtasksResponse` types. Add `say: Option<String>` and `text: Option<String>` to `RawUiMessage`. |
| **`subtasks.rs`** (NEW) | New parsing module: `parse_task_subtasks(task_id) -> Option<SubtasksResponse>` |
| `mod.rs` | Add `pub(crate) mod subtasks;` |
| **`handlers/subtasks.rs`** (NEW) | New handler: `get_task_subtasks_handler` for `GET /history/tasks/:task_id/subtasks` |
| `handlers/mod.rs` | Add `pub mod subtasks;` and re-export handler + utoipa path |
| `server.rs` | Add `.route("/history/tasks/:task_id/subtasks", get(...))` to history_routes |
| `openapi.rs` | Register handler path and schema types |

### Frontend (Svelte/TypeScript)

| File | Change |
|------|--------|
| `types.ts` | Add `SubtaskEntry`, `SubtasksResponse` interfaces |
| `api.ts` | Add `fetchTaskSubtasks(taskId)` function |
| `TaskDetailView.svelte` | Add "Subtasks" section tab (icon: 📋) showing a timeline of subtask cards |

---

## UI Design

### New "Subtasks" tab in TaskDetailView

Add to the section tabs bar (between Messages and Tools):

```
💬 Messages  📋 Subtasks  🔧 Tools  💭 Thinking  📁 Files  ⚙️ Environment  ✅ Focus Chain
```

### Subtask cards layout:

```
┌────────────────────────────────────────────────────────────┐
│ 📋 Subtasks (2)                                  loaded 5ms│
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ┌─ #0 · INITIAL TASK ─────────────────────────────────┐   │
│  │ "write simple python script to generate fibonacchi   │   │
│  │  number (do not test)"                               │   │
│  │                                                      │   │
│  │ 📅 2026-02-08 13:54:12  ·  Messages: 0–3 (4 msgs)  │   │
│  │ 🔧 2 tool calls: write_to_file, attempt_completion   │   │
│  └──────────────────────────────────────────────────────┘   │
│                         ↓                                   │
│  ┌─ #1 · FEEDBACK SUBTASK ─────────────────────────────┐   │
│  │ "now write java script doing the same . do not test" │   │
│  │                                                      │   │
│  │ 📅 2026-02-08 13:55:22  ·  Messages: 4–7 (4 msgs)  │   │
│  │ 🔧 2 tool calls: write_to_file, attempt_completion   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                            │
│  No subtask = single-phase task.                           │
│  Click a subtask card to jump to its messages range.       │
└────────────────────────────────────────────────────────────┘
```

### Interaction: Click a subtask card → navigate to Messages tab with offset/filter set to that subtask's message range.

### Also show subtask count in TaskDetailView header:
```
Task 1770576852118
2026-02-08 13:54:12 — 2026-02-08 13:55:46 · 1m 34s · 3.2 KB · 42ms
[8 msgs] [4 tools] [2 thinking] [6 files] [2 subtasks]  ← NEW chip
```

### Also show `hasSubtasks` / subtask count on the task list (summary level):

In the task list table, add a small indicator when `totalSubtasks > 1`:
```
Task 1770576852118  ·  claude-opus-4-6  ·  8 msgs  ·  📋×2
```

This requires adding `subtask_count` to `TaskHistorySummary` (or computing client-side from the subtasks endpoint).

---

## Edge Cases

1. **No feedback (single-phase task):** `subtasks` array has exactly 1 entry (the initial task). `has_subtasks = false`.
2. **Multiple feedbacks:** Each `user_feedback` entry creates another subtask. Chain: #0 → #1 → #2 → …
3. **Missing `ui_messages.json`:** Fall back to regex-based `<feedback>` tag parsing on `api_conversation_history.json`.
4. **Empty feedback text:** Include the subtask but with `prompt: ""` or `prompt: "(empty feedback)"`.
5. **Feedback with no subsequent work:** The message range may be 0 messages if the conversation was abandoned after feedback was given. `message_count: 0`.
6. **`conversationHistoryIndex = -1`:** The initial task always has index -1. Range starts at 0.
7. **Task prompt extraction (`task_prompt` field):** Currently extracts from the first text block raw. Consider also stripping `<task>...</task>` wrapper to get clean text. This is a separate improvement but related.

---

## Relationship to Existing `task_prompt`

Currently `TaskHistorySummary.task_prompt` is extracted from the first text block of the first user message in `api_conversation_history.json`, truncated to 200 chars. This text includes the `<task>` XML wrapper:

```
<task>\nwrite simple python script to generate fibonacchi number (do not test)\n</task>
```

The subtask feature gives us a cleaner extraction path:
- Initial task prompt = `ui_messages[say="task"].text` (no XML wrapper)
- This could also improve the existing `task_prompt` field

**Optional enhancement:** Strip `<task>` wrapper from `task_prompt` in summary parsing. Not strictly required for subtask feature but is a natural improvement.

---

## Performance Considerations

- Subtask parsing is lightweight: only reads `ui_messages.json` (small) + scans `api_conversation_history.json` for tool counts (already parsed elsewhere).
- Could be cached alongside the task detail cache.
- For the summary-level subtask count: can be computed during initial task scan by doing a quick `ui_messages.json` scan for `say: "user_feedback"` entries. This adds minimal overhead since we already read `ui_messages.json` for end-time.

---

## Summary

| What | Where | How |
|------|-------|-----|
| Detect subtasks | `ui_messages.json` → `say: "task"` / `say: "user_feedback"` | Scan for marker entries |
| Compute boundaries | `conversationHistoryIndex` field | Range: `prev.idx+1` to `next.idx` |
| Count tools per subtask | `api_conversation_history.json` | Scan tool_use blocks in message range |
| Expose via API | `GET /history/tasks/:id/subtasks` | New endpoint, new handler |
| Show in UI | TaskDetailView.svelte "Subtasks" tab | Timeline of subtask cards |
| Show in list | TaskHistorySummary badge | `📋×N` indicator |

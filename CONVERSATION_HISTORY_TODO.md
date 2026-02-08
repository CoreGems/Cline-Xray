# Conversation History API — Assessment & TODO

> **Companion to:** `CLINE_SHADOW_GIT_HOWTO.md`  
> **Date:** 2026-02-07  
> **Status:** P0 IMPLEMENTED — Task index API is live

### Implementation Status (2026-02-07)

| Phase | Status | What |
|-------|--------|------|
| **P0: Task index** | ✅ **DONE** | `GET /history/tasks` — scans all 185 task dirs, returns summary stats per task |
| **P1: Task detail** | ✅ **DONE** | `GET /history/tasks/:taskId` — full task deep-dive: messages, tool calls, files, model info, env, focus chain |
| **P2: UI — History tab** | ✅ **DONE** | Standalone "History" top-level tab with Tasks subtab (task list + expandable details) |
| **P3: Cross-reference** | ⬜ TODO | Link conversation ↔ shadow git checkpoints |
| **P4: Analytics** | ✅ **DONE** | `GET /history/stats` — aggregate stats: totals, averages, tool/model/version breakdowns, file stats, time range |
| **P5: Search** | ⬜ TODO | Full-text search across histories |

**Files created (P0):**
```
src-tauri/src/conversation_history/
├── mod.rs           ← module registration + re-exports
├── types.rs         ← TaskHistorySummary, raw Cline JSON types, API response types
├── parser.rs        ← scan_all_tasks() — parses api_conversation_history + ui_messages + task_metadata
├── handlers.rs      ← GET /history/tasks handler (Axum, with memory + disk cache)
└── cache.rs         ← disk-based JSON cache for task index

scripts/test_history.ps1  ← PowerShell test script (same pattern as test_jira_list.ps1)
```

**Wired into:**
- `src-tauri/src/main.rs` — `mod conversation_history;`
- `src-tauri/src/server.rs` — `/history/tasks` route (auth-protected)
- `src-tauri/src/openapi.rs` — registered in PublicApiDoc schemas + paths

**Answers to Open Questions (resolved by implementation):**
1. ✅ Separate `/history/` prefix (not `/changes/`) — cleaner separation won
2. ✅ `api_conversation_history.json` for content, `ui_messages.json` for timestamps — implemented as dual parse
3. ✅ Active tasks handled gracefully — parser skips files that fail to parse
4. ✅ Thinking blocks counted but not exposed in P0 (summary only)

---

## 1. What's There (Raw Data Audit)

We examined the **physical** Cline task storage on disk:

```
%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\
└── tasks\
    └── <task-id>\                       ← 185 task directories
        ├── api_conversation_history.json ← THE target (Anthropic wire format)
        ├── ui_messages.json              ← Cline UI-level messages (say/ask)
        ├── task_metadata.json            ← files-in-context, model, environment
        └── focus_chain_taskid_<id>.md    ← task progress checklist (165 files)
```

### Numbers (as of 2026-02-07)

| Metric | Value |
|--------|-------|
| Total task directories | **185** |
| Total files | **760** (595 `.json` + 165 `.md`) |
| Total disk usage | **179.9 MB** |
| `api_conversation_history.json` files | **181** (4 tasks have none) |
| api_conversation_history total size | **84.4 MB** |
| Smallest task | 2.1 KB (1 exchange) |
| Largest task | 3,870 KB (~3.8 MB, long session) |
| Average task | **477 KB** |

---

## 2. What's Inside `api_conversation_history.json`

It's a **raw Anthropic API message log** — an array of `{role, content}` objects, exactly as sent/received from the Claude API.

### Message structure

```
[
  { role: "user",      content: [ {type:"text", text:"<task>..."} ] },
  { role: "assistant", content: [ {type:"thinking", thinking:"..."}, {type:"text", text:"..."}, {type:"tool_use", id, name, input} ] },
  { role: "user",      content: [ {type:"tool_result", tool_use_id, content:[{type:"text",text:"..."}]} ] },
  ...
]
```

### Content block types observed

| Block type | Where | What it contains |
|-----------|-------|-----------------|
| `text` | user & assistant | Natural language (task prompts, reasoning, instructions) |
| `thinking` | assistant only | Extended thinking / chain-of-thought (hidden from user in Cline UI) |
| `tool_use` | assistant only | `{id, name, input}` — the tool call with all parameters |
| `tool_result` | user only | `{tool_use_id, content}` — tool output returned to the model |

### Tool usage breakdown (sampled 30 tasks = 754 tool calls)

| Tool | Count | % |
|------|-------|---|
| `execute_command` | 194 | 25.7% |
| `replace_in_file` | 175 | 23.2% |
| `attempt_completion` | 171 | 22.7% |
| `write_to_file` | 131 | 17.4% |
| `read_file` | 47 | 6.2% |
| `browser_action` | 17 | 2.3% |
| `list_files` | 11 | 1.5% |
| `ask_followup_question` | 5 | 0.7% |
| `search_files` | 3 | 0.4% |

Average: **~25 tool calls per task**, **~52 messages per task** (user+assistant), **~25 thinking blocks per task**.

---

## 3. The Other Files (Supporting Context)

### `task_metadata.json`

```json
{
  "files_in_context": [
    { "path": "src/foo.ts", "record_state": "active|stale",
      "record_source": "file_mentioned|read_tool|cline_edited|user_edited",
      "cline_read_date": 1764791884344,
      "cline_edit_date": null,
      "user_edit_date": null }
  ],
  "model_usage": [
    { "ts": 1764791864842, "model_id": "claude-sonnet-4-5-20250929",
      "model_provider_id": "anthropic", "mode": "act" }
  ],
  "environment_history": [
    { "ts": 1764791864815, "os_name": "win32", "os_version": "10.0.17763",
      "host_name": "Visual Studio Code", "host_version": "1.106.3",
      "cline_version": "3.39.2" }
  ]
}
```

**Value:** Model/provider tracking, file-level read/write audit trail, Cline version history.

### `ui_messages.json`

```json
[
  { "ts": 1764791864800, "type": "say|ask", "say": "text",
    "text": "...", "images": [], "files": [],
    "modelInfo": { "providerId": "anthropic", "modelId": "claude-sonnet-4-5-20250929" },
    "conversationHistoryIndex": -1 }
]
```

**Value:** Timestamps per message (api_conversation_history has none!), UI-level interaction types (say vs ask = approval flow), **`conversationHistoryIndex`** links each UI message back to the api_conversation_history array index.

**Key insight:** `ui_messages.json` has **timestamps** and **296 entries** vs api_conversation_history's **83 entries**. It's a higher-resolution view — it includes intermediate UI events (approval dialogs, command outputs, etc.) that get folded into a single API message.

### `focus_chain_taskid_<id>.md`

Markdown checklist — the task progress tracker. Human-readable, machine-parseable.

---

## 4. Does It Make Sense? — YES, With Caveats

### 4.1 Why YES

| Reason | Detail |
|--------|--------|
| **Complementary to shadow git** | Shadow git shows WHAT changed (diffs). Conversation history shows WHY and HOW (reasoning, tool calls, errors, retries). Together they tell the full story. |
| **Rich structured data** | Tool calls with typed inputs, thinking blocks, file audit trails — all machine-parseable JSON. Not just blobs. |
| **Cross-reference opportunity** | Task IDs are shared: `tasks/<task-id>/` ↔ checkpoint commits tagged `checkpoint-<ws>-<task-id>`. We can join conversation turns with code diffs. |
| **Analytics goldmine** | Tool usage patterns, model switching, error rates, task duration (via ui_messages timestamps), files-per-task metrics. No one else surfaces this. |
| **Already paid for** | The data exists on disk, ~180 MB, structured, stable format. We're just reading it. |
| **Agent self-awareness** | Our own agent tab could query past task histories to learn patterns, avoid repeated mistakes, understand codebase evolution. |

### 4.2 The Caveats

| Concern | Severity | Detail |
|---------|----------|--------|
| **Size** | 🟡 Medium | 84 MB of JSON across 181 files. The largest single file is 3.8 MB. Parsing all on startup is unwise. Needs lazy loading + indexing. |
| **No timestamps in api_conversation_history** | 🔴 High | The API log has NO timestamps. Must join with `ui_messages.json` (which has `ts` per entry and `conversationHistoryIndex`) or use task-id (which is epoch ms) as a proxy for start time only. |
| **Thinking blocks are HUGE** | 🟡 Medium | Extended thinking can be thousands of chars per turn. Must strip or summarize for list views, only load on demand. |
| **Tool result payloads are HUGE** | 🟡 Medium | `tool_result` blocks contain full file contents (from `read_file`) and full command outputs. A single block can be 50+ KB. Same treatment: strip for summaries, load on demand. |
| **Not our data format** | 🟡 Medium | This is Cline's internal format. It could change between versions (currently Cline 3.39.2). Need version-awareness and graceful degradation. |
| **Privacy / sensitivity** | 🟠 Medium-High | Conversation history contains file contents, command outputs, API keys (if they leaked into prompts), thinking about proprietary code. Any API exposing this needs auth + careful scoping. |
| **Duplication with ui_messages** | 🟡 Medium | `ui_messages.json` is ~1.1x the size of `api_conversation_history.json` and contains overlapping data plus timestamps. Need to decide which is the primary source. |
| **Cross-workspace** | 🟢 Low | Tasks span multiple workspaces (task 1764791864790 was for `jbot`, not `jira-dashboard`). Need workspace filtering or accept that this is a global Cline history browser. |

---

## 5. What Would the APIs Look Like?

### 5.1 Natural API Surface

Following the pattern from `shadow_git/handlers.rs`:

| Endpoint | Purpose | Source file |
|----------|---------|-------------|
| `GET /history/tasks` | List all task IDs with summary stats (msg count, tool count, model, start time, last time) | Parse all task dirs, light indexing |
| `GET /history/tasks/:taskId` | Full task summary: messages, tools used, files touched, model info, env, focus chain | Parse api_conversation_history + task_metadata + focus_chain |
| `GET /history/tasks/:taskId/messages` | Paginated message list (role, content-type, truncated text) | Parse api_conversation_history, join ui_messages for timestamps |
| `GET /history/tasks/:taskId/messages/:index` | Single message with full content (including thinking, tool_result) | Direct array access |
| `GET /history/tasks/:taskId/tools` | Tool call timeline: tool name, input summary, result summary, success/fail | Filter tool_use + tool_result blocks |
| `GET /history/tasks/:taskId/thinking` | Thinking blocks only (agent reasoning chain) | Filter thinking blocks |
| `GET /history/tasks/:taskId/files` | Files-in-context audit trail (from task_metadata) | Parse task_metadata.json |
| `GET /history/stats` | Aggregate stats: total tasks, tool breakdown, model usage, avg task size | Scan all tasks |

### 5.2 Cross-Reference with Shadow Git

The killer feature — joining the two data sources:

| What | How |
|------|-----|
| "Show me the conversation that produced this diff" | Match task-id from checkpoint commit → load conversation history for that task |
| "Show me the diff that resulted from this tool call" | Match `write_to_file`/`replace_in_file` tool_use → find the checkpoint commit closest in time |
| "Timeline view: conversation turns interleaved with diffs" | Merge ui_messages (timestamped) with checkpoint commits (timestamped) by task-id, sort by time |

This is something **no existing tool does** — not Cline's built-in history, not any Git GUI. It's the "agent forensics" view.

---

## 6. Architecture Fit Assessment

### 6.1 Where it fits in our app

```
Option A: Extend the existing "Changes" tab
  Changes Tab → [Tasks] [Diff] [Export] [*History*]   ← new subtab
  
Option B: Standalone "History" top-level tab
  [My Jiras] [Activity] [API] [Agent] [Changes] [*History*]

Option C: Enhance "Changes" Task List with conversation data
  Task list row → click → split view: left=diff, right=conversation
```

**Recommendation:** Start with **Option A** (subtab in Changes). The data sources are deeply linked by task-id. A combined tab avoids context-switching. Promote to top-level tab later if it grows.

### 6.2 Backend module

```
src-tauri/src/
├── shadow_git/          ← existing: diffs, checkpoints
│   ├── handlers.rs
│   ├── discovery.rs
│   ├── types.rs
│   └── cache.rs
└── conversation_history/ ← NEW: api_conversation_history parser
    ├── mod.rs
    ├── types.rs          ← TaskHistorySummary, ConversationMessage, ToolCall, etc.
    ├── parser.rs         ← JSON parsing, indexing, summarization
    ├── handlers.rs       ← Axum route handlers
    └── cache.rs          ← Index cache (task list + summaries, NOT full content)
```

### 6.3 Key technical decisions

| Decision | Recommendation | Why |
|----------|---------------|-----|
| Primary data source | `api_conversation_history.json` + `ui_messages.json` (join) | api has content, ui has timestamps |
| Parsing strategy | **Lazy** — index task dirs on startup (just file sizes + task-id), parse individual files on demand | 84 MB is too much to parse eagerly |
| Caching | Index cache only (task list + summary stats). Full messages loaded on demand, LRU-cached in memory | Avoid 84 MB RAM usage |
| Content truncation | Summaries truncate text/thinking to first 200 chars, tool_result to first 100 chars. Full content via `/messages/:index` | UI responsiveness |
| Auth | Same Bearer token as all `/changes/` endpoints | Conversation data is sensitive |

---

## 7. Relationship to Shadow Git (Synergy Map)

```
                    ┌───────────────────────────┐
                    │    SHARED KEY: task-id     │
                    │  (directory name = epoch)  │
                    └─────────────┬─────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                                       ▼
┌──────────────────────────┐          ┌──────────────────────────────┐
│   Shadow Git (existing)  │          │  Conversation History (new)  │
│                          │          │                              │
│  WHAT changed            │          │  WHY it changed              │
│  ● file diffs            │          │  ● user prompts              │
│  ● line counts           │          │  ● agent thinking            │
│  ● checkpoint timeline   │          │  ● tool call decisions       │
│  ● patch export          │          │  ● error messages & retries  │
│                          │          │  ● file read/write audit     │
│  Source: checkpoints/.git│          │  Source: tasks/<id>/*.json   │
│  Size: varies (git repo) │          │  Size: 84 MB (181 files)     │
└──────────────────────────┘          └──────────────────────────────┘
              │                                       │
              └───────────────────┬───────────────────┘
                                  ▼
                    ┌─────────────────────────────┐
                    │   Combined "Agent Forensics" │
                    │                             │
                    │  ● Timeline: turns + diffs  │
                    │  ● "Why did this file change │
                    │    this way?"                │
                    │  ● Productivity analytics   │
                    │  ● Error pattern detection  │
                    └─────────────────────────────┘
```

---

## 8. What's Unique (Nobody Else Does This)

| Feature | Cline UI | VS Code Git | Our App (proposed) |
|---------|----------|-------------|-------------------|
| Browse task history | ✅ (basic list) | ❌ | ✅ (with stats) |
| View conversation | ✅ (current task only) | ❌ | ✅ (any past task) |
| View diffs per task | ✅ (limited) | ❌ | ✅ (via shadow git) |
| Cross-ref diffs ↔ conversation | ❌ | ❌ | **✅ (unique)** |
| Tool usage analytics | ❌ | ❌ | **✅ (unique)** |
| Thinking block inspection | ❌ (hidden in UI) | ❌ | **✅ (unique)** |
| File audit trail | ❌ | ❌ | **✅ (unique)** |
| Export conversation as report | ❌ | ❌ | **✅ (unique)** |
| Multi-task aggregate stats | ❌ | ❌ | **✅ (unique)** |
| Model/provider usage tracking | ❌ | ❌ | **✅ (unique)** |

---

## 9. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|-----------|
| Cline changes JSON format between versions | 🟡 Medium | Version-detect from `environment_history.cline_version` in task_metadata. Schema-validate, fail gracefully. |
| 84 MB of JSON is slow to parse | 🟡 Medium | Lazy loading + index cache. Never parse all files at once. Summary cache is ~1 MB. |
| Conversation data contains secrets | 🟠 High | Bearer auth on all endpoints. No data leaves the local machine unless explicitly exported. Consider redaction filters. |
| task-id collision across workspaces | 🟢 Low | task-id is epoch milliseconds, unique across workspaces. But conversation history is global (not per-workspace), while checkpoints are per-workspace. |
| Large thinking/tool_result blocks crash the UI | 🟡 Medium | Truncation in list views. Virtual scrolling. Full content only on expand. |
| Active task's JSON is being written to by Cline | 🟡 Medium | Read with shared access. Accept stale data for the currently-active task. Or detect active task and skip/warn. |
| `ui_messages.json` is even larger than api_conversation_history | 🟢 Low | Only parse ui_messages for timestamps when needed (on-demand join, not eager). |

---

## 10. Effort Estimate

| Phase | Work | Est. |
|-------|------|------|
| **P0: Task index** | Scan task dirs, parse metadata only, build index | 1 day |
| **P1: Task detail** | Parse api_conversation_history for single task, return structured messages | 1 day |
| **P2: UI — History subtab** | Task list + message viewer in Changes tab | 2 days |
| **P3: Cross-reference** | Link conversation messages to shadow git checkpoints | 1 day |
| **P4: Analytics** | Aggregate stats endpoint + dashboard charts | 1-2 days |
| **P5: Search** | Full-text search across conversation histories | 1 day |

**Total: ~7-8 days** for the full feature. **P0+P1+P2 = 4 days** for MVP.

---

## 11. Verdict

### ✅ YES — Build it, but incrementally

**Why it makes sense:**
1. The data is already there, structured, and free to read
2. It's **complementary** to shadow git, not duplicative — different angle on the same tasks
3. The cross-reference (conversation ↔ diffs) is genuinely novel
4. It fits naturally into the existing Changes tab as a subtab
5. The backend pattern is identical to what we already built for shadow git (Axum handlers, caching, lazy loading)

**What to build first:**
1. Task index endpoint (P0) — cheap, proves the concept
2. Single-task conversation viewer (P1+P2) — the "aha moment"
3. Cross-reference with shadow git (P3) — the unique value

**What to defer:**
- Full-text search (needs an index, possibly SQLite FTS)
- Analytics dashboards (nice-to-have, not core)
- Export/report generation (can reuse the existing Export subtab pattern)

---

## 12. Open Questions

1. **Should conversation history APIs live under `/changes/` or a separate `/history/` prefix?**
   - Pro `/changes/`: single route group, shared auth, shared caching infrastructure
   - Pro `/history/`: cleaner separation, independent lifecycle, different cache strategies

2. **Do we parse `ui_messages.json` or `api_conversation_history.json` as primary?**
   - api_conversation_history: raw API format, tool calls clearly structured, but NO timestamps
   - ui_messages: timestamps, UI interaction types, but less structured content, larger files
   - **Likely answer:** api_conversation_history for content, join ui_messages for timestamps only

3. **How do we handle the currently-active task?**
   - Cline is actively writing to its JSON files during a session
   - Options: skip it, read with shared lock, detect "incomplete" (no attempt_completion at end)

4. **Should thinking blocks be exposed or redacted by default?**
   - They contain raw reasoning that may include sensitive observations
   - Probably: expose in admin API, redact in public API

5. **Multi-machine / sync scenario?**
   - If `%APPDATA%` is roaming or backed up, task data could come from different machines
   - `environment_history` in task_metadata tracks this — could filter by host

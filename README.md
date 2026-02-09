# 🔬 Cline X-Ray

**Post-execution inspection for agentic coding sessions — built for Gemini 3.**

> _Turn opaque agent runs into verifiable execution timelines._

Cline X-Ray reads the artifacts that AI coding agents already produce — shadow-Git repositories, conversation histories, tool-call logs, and task metadata — and reconstructs them into a navigable execution history that Gemini 3 can reason over directly.

[![Built with Tauri](https://img.shields.io/badge/Tauri-2.x-blue?logo=tauri)](https://tauri.app)
[![Frontend](https://img.shields.io/badge/Svelte-5-orange?logo=svelte)](https://svelte.dev)
[![Backend](https://img.shields.io/badge/Rust-Axum-red?logo=rust)](https://github.com/tokio-rs/axum)
[![AI](https://img.shields.io/badge/Gemini-3-green?logo=google)](https://ai.google.dev)

---

## 📖 Table of Contents

- [Inspiration](#-inspiration)
- [What It Does](#-what-it-does)
- [Architecture](#-architecture)
- [Tech Stack](#-tech-stack)
- [Features](#-features)
- [Getting Started](#-getting-started)
- [API Reference](#-api-reference)
- [Challenges](#-challenges)
- [Accomplishments](#-accomplishments)
- [What We Learned](#-what-we-learned)
- [What's Next](#-whats-next)

---

## 💡 Inspiration

Cline X-Ray was built specifically for **Gemini 3**.

Gemini 3 introduces a new class of reasoning models — capable of analyzing complex systems, long timelines, and multi-step causality. While experimenting with Gemini 3 during agent-assisted coding, we discovered a critical limitation:

> **Gemini's reasoning power is only as good as the structure of the evidence it receives.**

Modern AI coding agents like Cline generate large, multi-step changes across files, tools, and models. After execution, most of this context is flattened into summaries or explanations — exactly the kind of abstraction that weakens Gemini's ability to reason precisely.

Cline X-Ray was inspired by a simple question:

> *What if Gemini 3 could inspect agent runs the same way developers inspect Git history — using diffs, timelines, and concrete artifacts instead of summaries?*

Instead of asking agents to explain themselves, Cline X-Ray **exposes the artifacts they already produce** — commits, diffs, logs, and metadata — and reconstructs execution in a form that Gemini 3 can reason over directly.

---

## 🔍 What It Does

Cline X-Ray is a **post-execution explorer** for agentic coding sessions, designed to act as a **structured evidence layer** for Gemini 3.

It reads Cline's on-disk work artifacts — shadow-Git repositories, task metadata, message history, and tool logs — and presents them as a navigable execution history.

Rather than summarizing behavior, it shows:

| Dimension | What You See |
|---|---|
| **File Changes** | Which files were read and edited, with full diffs |
| **Execution Timeline** | How changes evolved across tasks and subtasks |
| **Prompt → Diff Mapping** | Which prompts and tool calls produced which diffs |
| **Model Analytics** | How models, token usage, and timing varied over time |
| **Thinking Blocks** | Raw model reasoning chains extracted per task |

Because this output is grounded in **concrete artifacts**, Gemini 3 can be used as a second-party reasoning model to:

- 🧠 **Explain** why changes happened
- ⚠️ **Identify** risky diffs or scope creep
- 🔧 **Suggest** safer follow-up refactors
- 📊 **Reason** about agent behavior across multiple tasks

---

## 🏗 Architecture

Cline X-Ray is implemented as a **single-process Tauri 2 application** with a privileged Rust backend.

```
┌─────────────────────────────────────────────────────┐
│                   Cline X-Ray                       │
│                                                     │
│  ┌──────────────┐         ┌──────────────────────┐  │
│  │  Svelte 5 UI │◄──REST──►  Rust / Axum Backend │  │
│  │  (WebView)   │         │  (localhost API)      │  │
│  └──────────────┘         └──────────┬───────────┘  │
│                                      │              │
│                           ┌──────────▼───────────┐  │
│                           │  On-Disk Artifacts    │  │
│                           │  ┌─────────────────┐  │  │
│                           │  │ Shadow Git Repos │  │  │
│                           │  │ Task Metadata    │  │  │
│                           │  │ Message History  │  │  │
│                           │  │ Tool Call Logs   │  │  │
│                           │  └─────────────────┘  │  │
│                           └──────────────────────┘  │
│                                      │              │
│                           ┌──────────▼───────────┐  │
│                           │  Gemini 3 (API)      │  │
│                           │  Reasoning Layer     │  │
│                           └──────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Key design principles:**

- The **backend parses Cline's existing artifacts** directly from disk — no import step required
- An **embedded localhost REST API** exposes data via a stable **OpenAPI 3.1** contract
- The **UI consumes the same API** that an external system or LLM would
- Parsing is done **on demand**, aligned with task boundaries, diffs, and timestamps
- **Gemini 3 integrates cleanly** as an independent reasoning layer, without coupling to any agent or UI

> **Security:** The REST API always binds to `127.0.0.1` (loopback only) with per-session Bearer token authentication. It is never exposed to the network.

---

## 🛠 Tech Stack

| Layer | Technology |
|---|---|
| **Desktop Runtime** | [Tauri 2](https://tauri.app) |
| **Backend** | Rust, [Axum](https://github.com/tokio-rs/axum), [Tokio](https://tokio.rs) |
| **Frontend** | [Svelte 5](https://svelte.dev), [Tailwind CSS 4](https://tailwindcss.com), TypeScript |
| **API Spec** | [utoipa](https://github.com/juhaku/utoipa) (OpenAPI 3.1 auto-generated from Rust types) |
| **AI Integration** | Google Gemini 3 API |
| **Git Parsing** | Native Rust — reads Cline shadow-Git repos directly |
| **Build Tools** | Vite 6, Cargo |

---

## ✨ Features

### 🗂 Six Integrated Tabs

| Tab | Purpose |
|---|---|
| **My Jiras** | Browse and inspect Jira issues with full detail panels |
| **Activity** | Real-time REST access logs and Gemini inference logs |
| **API** | Interactive OpenAPI explorer, tool console with circuit breakers and fixtures |
| **Agent** | Chat with Gemini 3 using structured context from agent artifacts |
| **Changes** | Shadow-Git diff explorer — tasks, steps, subtasks, and composite "Latest" view |
| **History** | Conversation history browser — messages, tool calls, thinking blocks, file context, and aggregate stats |

### 🔎 Changes Tab — Shadow Git Inspector

- **Latest** — composite view of the most recent diffs across all workspaces
- **Tasks** — browse all Cline task checkpoints with per-task and per-step diffs
- **Subtask Diffs** — drill into sub-step boundaries within a single task
- **Workspace Management** — discover and clean up shadow-Git repos

### 📜 History Tab — Conversation Forensics

- **Task List** — every Cline task with token counts, model info, and timestamps
- **Task Detail View** — full message timeline with:
  - Paginated messages with content block summaries
  - Tool call timeline (which tools, when, with what arguments)
  - Thinking blocks (raw model reasoning chains)
  - Files in context (what the model could see)
  - Subtask breakdown
- **Stats** — aggregate analytics across all tasks

### 🤖 Agent Tab — Gemini 3 Chat

- Direct chat with Gemini 3 models
- Model selection from available Gemini models
- Designed for follow-up reasoning over inspected artifacts

### 🔧 API & Tools Console

- Auto-generated OpenAPI 3.1 spec (public + admin)
- Tool runtime with invoke, validate, and log capabilities
- Circuit breakers and fixture management for safe tool execution
- Full execution logs with timing and error tracking

---

## 🚀 Getting Started

### Prerequisites

- **Rust** (latest stable) — [rustup.rs](https://rustup.rs)
- **Node.js** ≥ 18 — [nodejs.org](https://nodejs.org)
- **Tauri 2 CLI** — installed via npm (included in devDependencies)

### Setup

1. **Clone the repository:**
   ```bash
   git clone https://github.com/CoreGems/Jira-Xray-Gem.git
   cd Jira-Xray-Gem
   ```

2. **Install dependencies:**
   ```bash
   npm install
   ```

3. **Configure environment:**
   ```bash
   cp .env.example .env
   ```
   Edit `.env` and set your keys:
   ```env
   # Required — powers the Agent chat and reasoning features
   GEMINI_API_KEY=your_gemini_api_key

   # Optional — enables the My Jiras tab
   JIRA_URL=https://yourcompany.atlassian.net
   JIRA_EMAIL=your.name@yourcompany.com
   JIRA_API_TOKEN=your_jira_api_token
   ```

4. **Run in development mode:**
   ```bash
   npm run tauri dev
   ```

5. **Build for production:**
   ```bash
   npm run tauri build
   ```

### Verify the API

Once running, the REST API is available at the auto-assigned localhost port (printed in logs and saved to `.env`):

```
GET  /health              # Health check (no auth)
GET  /openapi.json        # Public OpenAPI spec (no auth)
GET  /changes/workspaces  # Shadow Git workspaces (Bearer token)
GET  /history/tasks       # Conversation history (Bearer token)
GET  /latest              # Composite latest view (Bearer token)
```

---

## 📡 API Reference

Cline X-Ray exposes two OpenAPI specifications:

| Spec | Endpoint | Description |
|---|---|---|
| **Public** | `GET /openapi.json` | All user-facing and agent-facing endpoints |
| **Admin** | `GET /openapi_admin.json` | Internal diagnostics, logging, tool config (not auto-discoverable) |

### Key API Groups

| Group | Endpoints | Description |
|---|---|---|
| **System** | `/health` | Health check and status |
| **Jira** | `/jira/list` | Jira issue listing |
| **Agent** | `/agent/chat`, `/agent/models` | Gemini 3 chat and model discovery |
| **Tools** | `/tools`, `/tools/invoke` | Tool discovery and execution |
| **Changes** | `/changes/*` | Shadow-Git workspace, task, step, and subtask diffs |
| **History** | `/history/*` | Task listing, detail, messages, tools, thinking, files, subtasks, stats |
| **Latest** | `/latest` | Composite latest-activity view |

All protected endpoints require a `Bearer` token (auto-generated per session).

---

## 🧩 Challenges

The primary challenge was **structural scale**.

Agent runs generate thousands of messages, multi-megabyte JSON histories, and many small diffs across steps. Early designs collapsed too much logic into large files and endpoints, which made both human review and Gemini-based reasoning brittle and inefficient.

**Gemini performs best when context is well-structured.** To support that, we redesigned the system around diff-aware and execution-aware boundaries, splitting parsing, handlers, and views by responsibility rather than convenience:

- **Conversation history** was decomposed into 12+ focused modules (messages, tools, thinking, files, subtasks, stats, cache, detail…)
- **Shadow Git** parsing was separated into discovery, types, handlers, cleanup, and caching layers
- **API handlers** were split into public and admin OpenAPI specs to keep agent-facing context clean

---

## 🏆 Accomplishments

- ✅ Turning opaque agent runs into **verifiable execution timelines**
- ✅ Aligning prompts, tool calls, and diffs into a **single coherent view**
- ✅ Enabling Gemini 3 to **reason over agent behavior** using real artifacts
- ✅ Making AI-generated code **reviewable without relying on self-explanations**
- ✅ Designing an inspection layer that **scales as agents and models improve**
- ✅ Auto-generated **OpenAPI 3.1 spec** from Rust types — the UI and external tools see the same contract

---

## 📚 What We Learned

Gemini 3 reinforced an important truth:

> **Stronger reasoning models amplify both good and bad structure.**

Large files, monolithic handlers, and implicit boundaries don't just hurt maintainability — they **actively degrade LLM reasoning quality**. Evidence-first workflows built on diffs, logs, and timelines dramatically improve how models like Gemini analyze complex systems.

**Inspection is not optional infrastructure for AI systems — it is foundational.**

---

## 🔮 What's Next

We want to move from **inspection** into **orchestration** using Gemini 3:

| Goal | Description |
|---|---|
| **Diff-Aware Prompts** | Generate follow-up prompts for refactors and fixes grounded in real diffs |
| **Risk Flagging** | Use Gemini to flag risky changes and explain complex diffs automatically |
| **Regression Detection** | Compare agent runs over time to detect drift or behavioral regressions |
| **Governance & Policy** | Enable compliance and policy checks driven by execution history |

Our long-term goal is to make agent-written code as **inspectable**, **explainable**, and **governable** as human-written code — with Gemini 3 acting as a first-class reasoning partner, not just a chat interface.

---

## 📁 Project Structure

```
Jira-Xray-Gem/
├── src/                          # Svelte 5 frontend
│   ├── App.svelte                # Main app shell
│   ├── lib/
│   │   ├── TopBar.svelte         # Navigation bar
│   │   ├── SettingsModal.svelte   # Configuration UI
│   │   ├── stores/               # Svelte 5 reactive stores
│   │   └── tabs/                 # Feature tabs
│   │       ├── my-jiras/         #   Jira issue browser
│   │       ├── activity/         #   Access & inference logs
│   │       ├── api/              #   OpenAPI explorer & tools console
│   │       ├── agent/            #   Gemini 3 chat interface
│   │       ├── changes/          #   Shadow-Git diff explorer
│   │       └── history/          #   Conversation history browser
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs              # App entry point & Tauri commands
│   │   ├── server.rs            # Axum router configuration
│   │   ├── openapi.rs           # OpenAPI 3.1 spec definitions
│   │   ├── state.rs             # Shared application state
│   │   ├── api/                 # Core API handlers & middleware
│   │   ├── shadow_git/          # Shadow-Git parsing & diff engine
│   │   ├── conversation_history/# Cline history parsing (12+ modules)
│   │   ├── latest/              # Composite latest-activity endpoint
│   │   ├── tool_runtime/        # Tool execution engine
│   │   └── jira.rs              # Jira API client
│   └── Cargo.toml
├── .env.example                  # Environment template
├── package.json                  # Node dependencies
└── index.html                    # Vite entry point
```

---

## 📄 License

This project was built for the **Gemini 3 Hackathon**.

---

<p align="center">
  <b>Cline X-Ray</b> — because agent-written code deserves the same scrutiny as human-written code.
</p>

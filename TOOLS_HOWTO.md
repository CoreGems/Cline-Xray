# TOOLS HOWTO Guide

This document explains how to expose public APIs from `openapi.json` to AI agents as tools, and where to configure/display them in the UI.

## Overview: OpenAPI as Single Source of Truth

The **recommended approach** is to auto-generate agent tools directly from the OpenAPI spec, eliminating manual duplication:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Single Source of Truth Architecture                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│                    ┌─────────────────────┐                              │
│                    │    OpenAPI Spec      │                              │
│                    │   (openapi.rs)       │                              │
│                    │   /openapi.json      │                              │
│                    └──────────┬──────────┘                              │
│                               │                                         │
│              ┌────────────────┼────────────────┐                        │
│              ▼                ▼                 ▼                        │
│  ┌─────────────────┐ ┌────────────────┐ ┌─────────────────────┐        │
│  │  Agent Tools    │ │  UI Display    │ │  REST Routing       │        │
│  │  (auto-gen)     │ │  (auto-gen)    │ │  (server.rs)        │        │
│  └────────┬────────┘ └────────────────┘ └─────────────────────┘        │
│           │                                                             │
│     ┌─────┼─────┐                                                      │
│     ▼     ▼     ▼                                                      │
│  Gemini OpenAI Claude                                                  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

| System | Location | Purpose |
|--------|----------|---------|
| **OpenAPI Spec** (source) | `src-tauri/src/openapi.rs` | Single source - defines ALL endpoints |
| **Agent Tools** (generated) | Parse `/openapi.json` at runtime | Auto-generated from OpenAPI |
| **UI Endpoints** (generated) | Fetch `/openapi.json` in frontend | Auto-populated from OpenAPI |
| **UI Tools List** (generated) | Fetch `/openapi.json` in frontend | Show tools in API → Tools tab |

---

## Current Public APIs (from openapi.json)

These endpoints are defined in `src-tauri/src/openapi.rs`:

| Method | Path | Description | Tag | Auth |
|--------|------|-------------|-----|------|
| GET | `/health` | Health check - returns service status and uptime | system | No |
| GET | `/openapi.json` | Public OpenAPI specification | system | No |
| GET | `/jira/list` | List Jira issues based on JQL query | jira | Yes |
| POST | `/agent/chat` | Chat with Google Gemini AI | agent | Yes |
| GET | `/agent/models` | List available Gemini AI models | agent | Yes |

---

## Auto-Generating Tools from OpenAPI (Recommended)

Instead of manually defining tools in `agent/tools.rs`, **parse the OpenAPI spec at runtime** to auto-generate tool definitions. This way, any endpoint you add to `openapi.rs` automatically becomes an agent tool.

### Backend: Rust - Parse OpenAPI into ToolDefinitions

```rust
// In agent/tools.rs (or a new agent/openapi_tools.rs)

use crate::openapi::PublicApiDoc;
use utoipa::OpenApi;

/// Auto-generate tool definitions from the OpenAPI spec.
/// Each OpenAPI path+method becomes one tool.
///
/// Naming convention: `{method}_{path_segments}` 
///   e.g. GET /jira/list → "get_jira_list"
///        POST /agent/chat → "post_agent_chat"
pub fn tools_from_openapi() -> Vec<ToolDefinition> {
    let spec = PublicApiDoc::openapi();
    let mut tools = Vec::new();

    if let Some(paths) = &spec.paths {
        for (path, path_item) in paths.iter() {
            // Iterate over all HTTP methods on this path
            for (method, operation) in path_item.operations() {
                // Skip endpoints that ARE the agent (avoid recursion)
                if path == "/agent/chat" { continue; }
                // Skip meta endpoints
                if path == "/openapi.json" { continue; }

                let method_str = format!("{:?}", method).to_lowercase(); // "get", "post", etc.
                
                // Build tool name: get_jira_list, get_health, get_agent_models
                let path_slug = path.trim_start_matches('/')
                    .replace('/', "_")
                    .replace('{', "")
                    .replace('}', "");
                let tool_name = format!("{}_{}", method_str, path_slug);

                let description = operation.summary
                    .as_deref()
                    .or(operation.description.as_deref())
                    .unwrap_or("No description")
                    .to_string();

                let mut tool = ToolDefinition::new(&tool_name, &description);

                // Extract parameters from OpenAPI spec
                if let Some(params) = &operation.parameters {
                    for param in params {
                        let schema_type = param.schema
                            .as_ref()
                            .and_then(|s| s.schema_type.as_ref())
                            .map(|t| format!("{:?}", t).to_lowercase())
                            .unwrap_or_else(|| "string".to_string());
                        
                        let param_desc = param.description
                            .as_deref()
                            .unwrap_or("");
                        
                        tool.parameters.push(ToolParameter {
                            name: param.name.clone(),
                            param_type: schema_type,
                            description: param_desc.to_string(),
                            required: param.required.unwrap_or(false),
                            default: None,
                            enum_values: None,
                        });
                    }
                }

                // Extract request body parameters (for POST endpoints)
                if let Some(request_body) = &operation.request_body {
                    if let Some(content) = request_body.content.get("application/json") {
                        if let Some(schema) = &content.schema {
                            // Extract properties from the request body schema
                            if let Some(properties) = &schema.properties {
                                for (prop_name, prop_schema) in properties {
                                    let prop_type = prop_schema.schema_type
                                        .as_ref()
                                        .map(|t| format!("{:?}", t).to_lowercase())
                                        .unwrap_or_else(|| "string".to_string());
                                    
                                    tool.parameters.push(ToolParameter {
                                        name: prop_name.clone(),
                                        param_type: prop_type,
                                        description: prop_schema.description
                                            .as_deref()
                                            .unwrap_or("")
                                            .to_string(),
                                        required: schema.required
                                            .as_ref()
                                            .map(|r| r.contains(prop_name))
                                            .unwrap_or(false),
                                        default: None,
                                        enum_values: None,
                                    });
                                }
                            }
                        }
                    }
                }

                tools.push(tool);
            }
        }
    }

    tools
}
```

### What This Produces

From the current `openapi.json`, this auto-generates:

| Auto-Generated Tool | Source Endpoint | Parameters |
|---------------------|----------------|------------|
| `get_health` | `GET /health` | None |
| `get_jira_list` | `GET /jira/list` | `jql: string`, `maxResults: integer` |
| `get_agent_models` | `GET /agent/models` | None |

**Excluded automatically:**
- `POST /agent/chat` — skipped (this IS the agent)
- `GET /openapi.json` — skipped (meta endpoint)

### Generic Tool Executor

Since tools are auto-generated from REST APIs, execution is also generic:

```rust
// In agent/executor.rs

use reqwest::Client;

/// Execute any auto-generated tool by calling its corresponding REST endpoint.
/// The tool name encodes the method and path: "get_jira_list" → GET /jira/list
async fn execute_openapi_tool(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<String> {
    // Parse tool name back to method + path
    let parts: Vec<&str> = tool_name.splitn(2, '_').collect();
    let method = parts[0]; // "get", "post", etc.
    let path = format!("/{}", parts.get(1).unwrap_or(&"").replace('_', "/"));
    let url = format!("{}{}", base_url, path);

    let response = match method {
        "get" => {
            // Build query params from args
            let mut request = client.get(&url);
            if let Some(obj) = args.as_object() {
                for (key, value) in obj {
                    request = request.query(&[(key, value.to_string().trim_matches('"'))]);
                }
            }
            request.bearer_auth(auth_token).send().await?
        }
        "post" => {
            client.post(&url)
                .bearer_auth(auth_token)
                .json(args)
                .send()
                .await?
        }
        _ => return Err(anyhow!("Unsupported method: {}", method))
    };

    let body = response.text().await?;
    Ok(body)
}
```

### Frontend: Auto-Fetch for UI Display

The UI can also fetch tools from OpenAPI instead of hardcoding:

```typescript
// In src/lib/tabs/agent/api.ts

export interface AgentTool {
  name: string;
  method: string;
  path: string;
  description: string;
  parameters: { name: string; type: string; required: boolean }[];
  auth: boolean;
}

/** Parse /openapi.json into agent tool definitions */
export async function fetchToolsFromOpenApi(): Promise<AgentTool[]> {
  const resp = await fetch('http://localhost:3030/openapi.json');
  const spec = await resp.json();
  const tools: AgentTool[] = [];

  const SKIP_PATHS = ['/agent/chat', '/openapi.json'];

  for (const [path, methods] of Object.entries(spec.paths || {})) {
    if (SKIP_PATHS.includes(path)) continue;

    for (const [method, operation] of Object.entries(methods as any)) {
      const pathSlug = path.replace(/^\//, '').replace(/\//g, '_').replace(/[{}]/g, '');
      tools.push({
        name: `${method}_${pathSlug}`,
        method: method.toUpperCase(),
        path,
        description: operation.summary || operation.description || '',
        parameters: (operation.parameters || []).map((p: any) => ({
          name: p.name,
          type: p.schema?.type || 'string',
          required: p.required || false,
        })),
        auth: !!operation.security?.length,
      });
    }
  }

  return tools;
}
```

### Benefits of Auto-Generation

| Aspect | Manual (`tools.rs`) | Auto-Generated (from OpenAPI) |
|--------|---------------------|-------------------------------|
| Add new endpoint | Edit 3 files (handler, openapi, tools) | Edit 2 files (handler, openapi) — tools auto-appear |
| Sync risk | Tools can get out of sync with API | Always in sync — single source of truth |
| Parameters | Must manually duplicate | Extracted from OpenAPI schema |
| UI display | Must manually add to `endpoints.ts` | Frontend fetches from `/openapi.json` |
| Maintenance | High — multiple places to update | Low — just add the endpoint |

---

## Where to View/Manage in the UI

### Current State

| What | Where | Status |
|------|-------|--------|
| REST API Endpoints | **API Tab → REST** subtab | ✅ Exists |
| Agent Tools | — | ❌ Not listed in UI yet |

### Proposed: Add "Tools" Subtab to API Tab

Since both REST endpoints and agent tools come from OpenAPI, the **API tab** is the natural home. Add a **Tools subtab** alongside the existing REST subtab:

```
┌──────────────────────────────────────────────────────────────────────┐
│ API Tab                                                               │
│ [REST] [Tools]    ← Add "Tools" subtab here                          │
└──────────────────────────────────────────────────────────────────────┘
```

**1. Update types (`src/lib/tabs/api/types.ts`):**

```typescript
export type ApiSubTab = 'REST' | 'Tools';

export interface SubTabDefinition {
  id: ApiSubTab;
  label: string;
}
```

**2. Create ToolsSubtab (`src/lib/tabs/api/ToolsSubtab.svelte`) — auto-fetches from OpenAPI:**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchToolsFromOpenApi, type AgentTool } from './utils';

  let tools: AgentTool[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    try {
      tools = await fetchToolsFromOpenApi();
    } catch (e) {
      error = `Failed to load tools: ${e}`;
    } finally {
      loading = false;
    }
  });
</script>

<div class="flex-1 overflow-auto p-6">
  <div class="max-w-4xl mx-auto w-full">
    <div class="mb-6">
      <h2 class="text-2xl font-bold text-gray-800 mb-2">Agent Tools</h2>
      <p class="text-gray-600">
        Auto-generated from <code class="bg-gray-200 px-2 py-1 rounded text-sm">/openapi.json</code>
        — tools available to the AI agent for function calling
      </p>
    </div>

    {#if loading}
      <p class="text-gray-500">Loading tools from OpenAPI spec...</p>
    {:else if error}
      <p class="text-red-600">{error}</p>
    {:else}
      <div class="bg-white rounded-lg shadow-sm border border-gray-200 divide-y divide-gray-200">
        {#each tools as tool}
          <div class="p-4 hover:bg-gray-50 transition-colors">
            <div class="flex items-start gap-3">
              <!-- Method Badge -->
              <span class="px-2 py-1 rounded text-xs font-bold uppercase min-w-[60px] text-center
                {tool.method === 'GET' ? 'bg-green-100 text-green-700' : 'bg-blue-100 text-blue-700'}">
                {tool.method}
              </span>
              
              <!-- Path and Description -->
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-1">
                  <code class="text-sm font-mono text-gray-800">{tool.path}</code>
                  {#if tool.auth}
                    <span class="px-2 py-0.5 rounded text-xs bg-orange-100 text-orange-700 flex items-center gap-1">
                      🔒 Auth
                    </span>
                  {/if}
                </div>
                <p class="text-sm text-gray-600">{tool.description}</p>
                <div class="text-xs text-gray-400 mt-1">
                  Tool name: <code class="bg-gray-100 px-1 rounded">{tool.name}</code>
                </div>
                {#if tool.parameters.length > 0}
                  <div class="mt-2">
                    <span class="text-xs font-medium text-gray-500">Parameters:</span>
                    {#each tool.parameters as param}
                      <span class="ml-2 text-xs font-mono">
                        <span class="text-blue-600">{param.name}</span><span class="text-gray-400">: {param.type}</span>
                        {#if param.required}<span class="text-red-500 ml-0.5">*</span>{/if}
                      </span>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>

      <!-- Info Footer -->
      <div class="mt-6 p-4 bg-blue-50 rounded-lg border border-blue-200">
        <div class="flex items-start gap-3">
          <svg class="w-5 h-5 text-blue-600 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
            <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path>
          </svg>
          <div class="text-sm text-blue-800">
            <p class="font-medium mb-1">How Tools Work</p>
            <p>Tools are auto-generated from the OpenAPI spec. When the agent needs data, it calls these tools which map directly to REST API endpoints. Add a new endpoint to <code class="bg-blue-100 px-1 rounded">openapi.rs</code> and it automatically appears here.</p>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>
```

**3. Add to ApiTab (`src/lib/tabs/api/ApiTab.svelte`):**

```svelte
<script lang="ts">
  import RESTSubtab from "./RESTSubtab.svelte";
  import ToolsSubtab from "./ToolsSubtab.svelte";
  import type { ApiSubTab, SubTabDefinition } from "./types";
  
  let activeSubTab: ApiSubTab = $state('REST');
  
  const subTabs: SubTabDefinition[] = [
    { id: 'REST', label: 'REST' },
    { id: 'Tools', label: 'Tools' }   // ADD THIS
  ];
</script>

<!-- In the template, add: -->
{#if activeSubTab === 'REST'}
  <RESTSubtab />
{:else if activeSubTab === 'Tools'}
  <ToolsSubtab />
{/if}
```

**4. Add `fetchToolsFromOpenApi` to utils (`src/lib/tabs/api/utils.ts`):**

```typescript
export interface AgentTool {
  name: string;
  method: string;
  path: string;
  description: string;
  parameters: { name: string; type: string; required: boolean }[];
  auth: boolean;
}

export async function fetchToolsFromOpenApi(): Promise<AgentTool[]> {
  const resp = await fetch('http://localhost:3030/openapi.json');
  const spec = await resp.json();
  const tools: AgentTool[] = [];
  const SKIP_PATHS = ['/agent/chat', '/openapi.json'];

  for (const [path, methods] of Object.entries(spec.paths || {})) {
    if (SKIP_PATHS.includes(path)) continue;
    for (const [method, operation] of Object.entries(methods as any)) {
      const pathSlug = path.replace(/^\//, '').replace(/\//g, '_').replace(/[{}]/g, '');
      tools.push({
        name: `${method}_${pathSlug}`,
        method: method.toUpperCase(),
        path,
        description: operation.summary || operation.description || '',
        parameters: (operation.parameters || []).map((p: any) => ({
          name: p.name, type: p.schema?.type || 'string', required: p.required || false,
        })),
        auth: !!operation.security?.length,
      });
    }
  }
  return tools;
}
```

### Result: API Tab with REST + Tools Subtabs

```
┌──────────────────────────────────────────────────────────────────────┐
│ API Tab                                                               │
│ [REST] [Tools]                                                        │
├──────────────────────────────────────────────────────────────────────┤
│                                                                       │
│ Agent Tools                                                           │
│ Auto-generated from /openapi.json                                     │
│                                                                       │
│ ┌────────────────────────────────────────────────────────────────┐   │
│ │ GET  /health                                                    │   │
│ │ Health check endpoint - returns service status and uptime       │   │
│ │ Tool name: get_health                                           │   │
│ └────────────────────────────────────────────────────────────────┘   │
│                                                                       │
│ ┌────────────────────────────────────────────────────────────────┐   │
│ │ GET  /jira/list  🔒 Auth                                        │   │
│ │ List Jira issues based on JQL query                             │   │
│ │ Tool name: get_jira_list                                        │   │
│ │ Parameters: jql: string*  maxResults: integer                   │   │
│ └────────────────────────────────────────────────────────────────┘   │
│                                                                       │
│ ┌────────────────────────────────────────────────────────────────┐   │
│ │ GET  /agent/models  🔒 Auth                                     │   │
│ │ List available Gemini AI models                                 │   │
│ │ Tool name: get_agent_models                                     │   │
│ └────────────────────────────────────────────────────────────────┘   │
│                                                                       │
│ ℹ️ Tools are auto-generated from the OpenAPI spec. Add a new         │
│   endpoint to openapi.rs and it automatically appears here.          │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Why API Tab (not Agent Tab)?

| Consideration | API Tab ✅ | Agent Tab ❌ |
|--------------|-----------|-------------|
| Both REST & Tools come from OpenAPI | Natural grouping | Different concern (chat) |
| Already has subtab infrastructure | REST subtab exists | Would mix chat + tools |
| Consistent with existing pattern | API → REST, API → Tools | Agent → Chat, Agent → Tools? |
| Tools = API surface for agents | Yes, it's an API view | Agent tab is for interacting |

---

## Auto-Generated Tool Naming (OpenAPI → Tool Name)

With auto-generation, tool names are derived from the endpoint:

| OpenAPI Endpoint | Auto-Generated Tool Name | Included? |
|-----------------|--------------------------|-----------|
| `GET /health` | `get_health` | ✅ Yes |
| `GET /jira/list` | `get_jira_list` | ✅ Yes |
| `GET /agent/models` | `get_agent_models` | ✅ Yes |
| `POST /agent/chat` | — | ❌ Skipped (this IS the agent) |
| `GET /openapi.json` | — | ❌ Skipped (meta endpoint) |

### Skip List (Non-Tool Endpoints)

Configure which paths should NOT become tools:

```rust
// In tools_from_openapi()
const SKIP_PATHS: &[&str] = &[
    "/agent/chat",     // This IS the agent - don't recurse
    "/openapi.json",   // Meta endpoint
];
```

### Design Considerations

1. **Not all APIs should be tools** - The `/agent/chat` endpoint IS the agent, it shouldn't call itself as a tool.

2. **Tools should be GPT-safe** - Only expose read-only, bounded operations:
   - ✅ `list_*`, `get_*`, `search_*` - Safe read operations
   - ⚠️ `create_*`, `update_*`, `delete_*` - Requires careful consideration
   - ❌ `execute_arbitrary_sql` - Never expose unbounded execution

3. **Auth is inherited** - The generic executor passes the user's Bearer token to all tool API calls.

---

## Adding a New Endpoint (Auto-Becomes a Tool)

With the auto-generation approach, adding a new endpoint to OpenAPI **automatically** creates a tool. No manual tool definition needed!

### Example: Adding "Get Jira Issue Details"

Only **3 steps** (vs. 6 with manual approach):

**Step 1. Implement the handler (`handlers.rs`):**

```rust
#[utoipa::path(
    get,
    path = "/jira/issue/{key}",
    params(("key" = String, Path, description = "Jira issue key")),
    responses(
        (status = 200, description = "Issue details", body = JiraIssueDetails),
        (status = 404, description = "Issue not found")
    ),
    tag = "jira",
    security(("bearerAuth" = []))
)]
pub async fn get_jira_issue_handler(...) { ... }
```

**Step 2. Register in OpenAPI (`openapi.rs`):**

```rust
#[openapi(
    paths(
        // ... existing ...
        crate::api::handlers::get_jira_issue_handler,
    ),
)]
pub struct PublicApiDoc;
```

**Step 3. Register the route (`server.rs`):**

```rust
.route("/jira/issue/:key", get(get_jira_issue_handler))
```

**Done!** The following happens automatically:

- ✅ Tool `get_jira_issue_key` auto-generated from OpenAPI
- ✅ Generic executor calls `GET /jira/issue/{key}` with args
- ✅ UI Tools tab shows it (fetches from `/openapi.json`)
- ✅ UI REST tab shows it (fetches from `/openapi.json`)
- ✅ Converted to Gemini/OpenAI format automatically

---

## Tool Format Conversion (Vendor-Agnostic Design)

**Important:** Tools are defined ONCE in a **vendor-neutral format** (`ToolDefinition`), then automatically converted to each AI provider's specific format.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Vendor-Agnostic Tool Architecture                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              ToolDefinition (vendor-neutral)                     │   │
│  │                                                                  │   │
│  │   name: "search_jira_issues"                                     │   │
│  │   description: "Search Jira using JQL..."                        │   │
│  │   parameters: [{ name: "jql", type: "string", ... }]            │   │
│  └─────────────────────┬───────────────────────────────────────────┘   │
│                        │                                                │
│            ┌───────────┼───────────┐                                   │
│            ▼           ▼           ▼                                   │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐                   │
│  │   Gemini     │ │   OpenAI     │ │  Claude      │                   │
│  │   Format     │ │   Format     │ │  Format      │                   │
│  │              │ │              │ │  (future)    │                   │
│  └──────────────┘ └──────────────┘ └──────────────┘                   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Auto-Generate + Convert

```rust
use agent::tools::{tools_from_openapi, to_gemini_functions, to_openai_functions};

// Auto-generate tools from OpenAPI spec
let tools = tools_from_openapi();

// Convert to Gemini format
let gemini_format = to_gemini_functions(&tools);

// Convert to OpenAI format
let openai_format = to_openai_functions(&tools);

// Both produce the correct JSON for their respective APIs
```

### Gemini Output Format

```json
{
  "name": "search_jira_issues",
  "description": "Search for Jira issues using JQL",
  "parameters": {
    "type": "object",
    "properties": { "jql": { "type": "string", "description": "..." } },
    "required": ["jql"]
  }
}
```

### OpenAI Output Format

```json
{
  "type": "function",
  "function": {
    "name": "search_jira_issues",
    "description": "Search for Jira issues using JQL",
    "parameters": {
      "type": "object",
      "properties": { "jql": { "type": "string", "description": "..." } },
      "required": ["jql"]
    }
  }
}
```

### Adding Support for New Providers

To add a new AI provider (e.g., Claude, Mistral), just add a new converter function:

```rust
// In agent/tools.rs

pub fn to_claude_functions(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
    // Convert to Claude's tool format
    tools.iter().map(|tool| {
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": {
                "type": "object",
                "properties": /* ... */,
                "required": /* ... */
            }
        })
    }).collect()
}
```

**Key benefit:** Tool definitions stay the same - only the conversion layer changes per vendor.

---

## Security Considerations

### GPT-Safe Tools

All tools exposed to the agent should be "GPT-safe":

- ✅ **Read-only operations** - `list_*`, `get_*`, `search_*`
- ✅ **Bounded results** - Always limit rows/results returned
- ✅ **Timeout enforcement** - Prevent long-running operations
- ✅ **Input validation** - Sanitize all parameters
- ❌ **DDL/DML operations** - No CREATE, UPDATE, DELETE without human approval
- ❌ **Arbitrary code execution** - Never expose `eval()` or similar

### Authentication Flow

```
┌──────────┐     ┌─────────────┐     ┌─────────────┐     ┌──────────┐
│   User   │────▶│ Agent Chat  │────▶│ Tool Exec   │────▶│ REST API │
│          │     │ /agent/chat │     │ executor.rs │     │ /jira/*  │
└──────────┘     └─────────────┘     └─────────────┘     └──────────┘
      │                                     │                  │
      │ Bearer Token                        │ Reuse Token      │ Validate
      └─────────────────────────────────────┴──────────────────┘
```

The agent executor inherits the user's authentication token and uses it when calling REST APIs.

---

## Quick Reference

### With Auto-Generation (Recommended)

To add a new endpoint that automatically becomes an agent tool:

| Step | File to Edit | What |
|------|--------------|------|
| 1 | `src-tauri/src/api/handlers.rs` | Implement handler with `#[utoipa::path]` |
| 2 | `src-tauri/src/openapi.rs` | Add to `paths()` in `PublicApiDoc` |
| 3 | `src-tauri/src/server.rs` | Register the route |

**That's it!** Tool definition, UI display, and provider format conversion all happen automatically.

### Files Overview

| File | Role |
|------|------|
| `src-tauri/src/openapi.rs` | **Single source of truth** — defines all endpoints |
| `agent/tools.rs` | Tool struct definitions + `tools_from_openapi()` auto-generator |
| `agent/executor.rs` | Generic `execute_openapi_tool()` — no per-tool code needed |
| `src/lib/tabs/api/utils.ts` | `fetchToolsFromOpenApi()` for UI |
| `src/lib/tabs/api/ToolsSubtab.svelte` | UI display of tools (auto-populated) |

---

## Verifying Configuration

### Check OpenAPI Spec (source of all tools)

```powershell
# List all paths in the public OpenAPI spec
curl http://localhost:3030/openapi.json | jq '.paths | keys'

# Or use the script
.\scripts\list_openapi.ps1
```

### Check Auto-Generated Tools

```powershell
# In Rust tests
cargo test --package agent test_tools_from_openapi

# Or call the API and see what tools are derived
curl http://localhost:3030/openapi.json | jq '[.paths | to_entries[] | .key]'
```

### Check UI Display

- **API Tab → REST** subtab — see all REST endpoints
- **API Tab → Tools** subtab — see all auto-generated agent tools

---

## Summary

With the **OpenAPI-as-single-source-of-truth** approach:

1. **Add endpoint** to `handlers.rs` with `#[utoipa::path]` annotation
2. **Register** in `openapi.rs` and `server.rs`
3. **Everything else is automatic:**
   - ✅ Agent tool auto-generated from OpenAPI spec
   - ✅ Generic executor calls the REST API (no per-tool code)
   - ✅ UI Tools tab auto-populates by fetching `/openapi.json`
   - ✅ Vendor format conversion (Gemini/OpenAI/Claude) happens via existing converters
   - ✅ Parameters, descriptions, auth requirements all extracted from OpenAPI

**No more manual sync** between `openapi.rs`, `tools.rs`, `endpoints.ts`, and `executor.rs`.

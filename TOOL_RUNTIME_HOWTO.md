# ToolRuntime - Unified Tool Invocation Choke-Point

## Overview

The **ToolRuntime** is a centralized execution layer that mediates ALL tool invocations in the application. Whether the call comes from:
- **AI Agent** (function calling)
- **UI Tools Console** (manual testing)
- **CLI scripts** (automated testing)

All calls go through the same choke-point, enabling consistent behavior, monitoring, and control.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ToolRuntime                                  │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    call(operation_id, args)                  │    │
│  └──────────────────────────┬──────────────────────────────────┘    │
│                             │                                        │
│  ┌──────────────────────────┼──────────────────────────────────┐    │
│  │ 1. Check enabled         │                                   │    │
│  │ 2. Circuit breaker       │                                   │    │
│  │ 3. Arg clamps            ▼                                   │    │
│  │ 4. Validate request  ─────────                               │    │
│  │ 5. Check fixtures        │                                   │    │
│  │ 6. Dry-run mode          │                                   │    │
│  │ 7. Execute HTTP          │                                   │    │
│  │ 8. Update circuit        │                                   │    │
│  │ 9. Record fixture        │                                   │    │
│  │ 10. Validate response    ▼                                   │    │
│  │ 11. Log & return     ─────────► ToolCallResult               │    │
│  └──────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

### 1. Enable/Disable States
Toggle individual tools on/off without code changes:

```rust
// Disable a specific tool
runtime.configure_tool("get_jira_list", ToolConfig {
    enabled: false,
    ..Default::default()
});
```

### 2. Arg Clamps
Enforce min/max values on parameters:

```rust
runtime.configure_tool("search_issues", ToolConfig {
    enabled: true,
    arg_clamps: Some(serde_json::json!({
        "maxResults": { "min": 1, "max": 100 }
    })),
    ..Default::default()
});
```

### 3. Dry-Run Mode
Return mock responses without actual execution:

```rust
// Global dry-run
runtime.set_global_config(GlobalRuntimeConfig {
    dry_run: true,
    ..Default::default()
});

// Per-tool dry-run
runtime.configure_tool("delete_issue", ToolConfig {
    dry_run: true,
    ..Default::default()
});
```

### 4. Contract Validation
Validate requests and responses against OpenAPI schema:

```rust
runtime.set_global_config(GlobalRuntimeConfig {
    validate_requests: true,
    validate_responses: true,
    fail_on_validation_error: true,
    ..Default::default()
});
```

### 5. Fixtures Replay/Record
Record and replay tool responses for testing:

```rust
// Enable recording
runtime.configure_tool("get_jira_list", ToolConfig {
    record_fixtures: true,
    ..Default::default()
});

// Enable replay
runtime.configure_tool("get_jira_list", ToolConfig {
    use_fixtures: true,
    ..Default::default()
});
```

### 6. Circuit Breaker
Protect against cascading failures:

```rust
runtime.set_global_config(GlobalRuntimeConfig {
    default_max_failures: 5,           // Open after 5 failures
    circuit_breaker_reset_ms: 60000,   // Reset after 1 minute
    ..Default::default()
});
```

## REST API Endpoints

### Tool Invocation (Main Entry Point)
```http
POST /tools/invoke
Content-Type: application/json

{
  "operationId": "get_jira_list",
  "args": { "jql": "project = TEST", "maxResults": 10 },
  "source": "ui_console",
  "dryRun": false
}
```

### List Tools
```http
GET /tools
```

### Execution Logs
```http
GET /tools/logs
DELETE /tools/logs
```

### Configuration
```http
GET /tools/config
PUT /tools/config
PUT /tools/{operation_id}/config
```

### Circuit Breakers
```http
GET /tools/circuit-breakers
DELETE /tools/circuit-breakers
DELETE /tools/{operation_id}/circuit-breaker
```

### Fixtures
```http
GET /tools/fixtures
POST /tools/fixtures
DELETE /tools/fixtures
```

### Bulk Operations
```http
POST /tools/enable-all
POST /tools/disable-all
```

## UI Tools Console

Navigate to **API → Tools Console** in the application to:

1. **Invoke Tools** - Select a tool, provide JSON arguments, and execute
2. **View Logs** - See execution history with timing, success/failure, and validation results
3. **Configure** - View tool configurations and status

## Module Structure

```
src-tauri/src/tool_runtime/
├── mod.rs              # Main module, ToolRuntime struct
├── types.rs            # Type definitions (ToolCallResult, etc.)
├── config.rs           # Configuration types (GlobalRuntimeConfig, ToolConfig)
├── executor.rs         # HTTP execution logic
├── validator.rs        # OpenAPI schema validation
├── fixtures.rs         # Fixture replay/record
├── circuit_breaker.rs  # Rate limiting/circuit breaker
└── handlers.rs         # HTTP API handlers
```

## Usage Examples

### From Agent (Rust)
```rust
let result = runtime.call(
    "get_jira_list",
    serde_json::json!({"jql": "project = TEST"}),
    ToolCallSource::Agent
).await;
```

### From UI Console (JavaScript)
```javascript
const response = await fetch(`${apiUrl}/tools/invoke`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
        operationId: 'get_jira_list',
        args: { jql: 'project = TEST' },
        source: 'ui_console'
    })
});
const result = await response.json();
```

### From CLI Script (curl)
```bash
curl -X POST http://127.0.0.1:PORT/tools/invoke \
  -H "Content-Type: application/json" \
  -d '{"operationId":"get_jira_list","args":{"jql":"project = TEST"}}'
```

## Configuration Reference

### GlobalRuntimeConfig
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `dry_run` | bool | false | Global dry-run mode |
| `replay_fixtures` | bool | false | Global fixture replay |
| `validate_requests` | bool | false | Validate requests against OpenAPI |
| `validate_responses` | bool | false | Validate responses against OpenAPI |
| `fail_on_validation_error` | bool | false | Fail calls on validation errors |
| `default_max_failures` | u32 | 5 | Default circuit breaker threshold |
| `circuit_breaker_reset_ms` | u64 | 60000 | Circuit breaker reset time |

### ToolConfig
| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | true | Tool is enabled |
| `dry_run` | bool | false | Per-tool dry-run mode |
| `use_fixtures` | bool | false | Replay recorded fixtures |
| `record_fixtures` | bool | false | Record responses as fixtures |
| `timeout_ms` | Option<u64> | None | Custom timeout |
| `max_failures` | Option<u32> | None | Custom circuit breaker threshold |
| `arg_clamps` | Option<Value> | None | Argument constraints |

## Benefits

1. **Consistency** - Same behavior regardless of caller
2. **Observability** - Centralized logging and metrics
3. **Control** - Enable/disable tools without code changes
4. **Testing** - Dry-run and fixture modes for development
5. **Reliability** - Circuit breakers prevent cascading failures
6. **Validation** - Contract testing against OpenAPI specs

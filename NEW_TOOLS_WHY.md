# Assessment: Why /tools/* APIs?

## Overview

This document assesses the rationale for having dedicated `/tools/*` REST API endpoints in the Jira Dashboard application, specifically the Tool Runtime system.

---

## Current /tools/* Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/tools` | GET | List all available tools |
| `/tools/invoke` | POST | Invoke a tool with arguments |
| `/tools/logs` | GET/DELETE | Tool execution history |
| `/tools/config` | GET/PUT | Global runtime configuration |
| `/tools/{id}/config` | PUT | Per-tool configuration |
| `/tools/circuit-breakers` | GET/DELETE | Circuit breaker status/reset |
| `/tools/fixtures` | GET/POST/DELETE | Mock data management |
| `/tools/enable-all` | POST | Bulk enable tools |
| `/tools/disable-all` | POST | Bulk disable tools |

---

## Assessment: Do We Need These?

### ✅ Arguments FOR /tools/* APIs

1. **Unified Choke-Point for Agent Calls**
   - AI agents making function calls need a controlled gateway
   - Enables consistent logging, validation, and rate limiting
   - Single place to enable/disable tool access for agents

2. **Developer Console Use Case**
   - UI Tools Console allows manual testing of tools
   - Developers can invoke tools with custom arguments
   - View execution logs and debug responses

3. **Testing Infrastructure**
   - Fixtures allow recording/replaying responses
   - Dry-run mode tests without side effects
   - Circuit breakers protect against cascading failures

4. **Observability**
   - Centralized logging of all tool invocations
   - Track which tools are called, by whom, and results
   - Monitor circuit breaker states

### ❌ Arguments AGAINST /tools/* APIs

1. **Complexity Overhead**
   - Adds another layer between agent and actual functionality
   - 15+ endpoints to maintain
   - Additional schemas and types

2. **Redundancy**
   - Each underlying API (Jira, Agent, etc.) already exists
   - `/tools/invoke` just proxies to existing endpoints
   - Could call `/jira/list` directly instead of `/tools/invoke {operation_id: "get_jira_list"}`

3. **Limited Current Usage**
   - Tools Console is primarily for development
   - Production agents could call APIs directly
   - Fixtures/circuit breakers rarely used in practice

4. **Performance**
   - Extra hop for every tool call
   - Validation overhead
   - Additional logging overhead

---

## Recommendation

### Option A: Keep as Development Feature Only
- Mark `/tools/*` endpoints as **development/admin only**
- Move to admin API spec (`/openapi_admin.json`)
- Not intended for production agent use
- Keep for Tools Console debugging

### Option B: Full Integration
- Make `/tools/invoke` the **mandatory** path for agents
- Benefits: consistent logging, rate limiting, circuit breakers
- Cost: performance overhead, maintenance burden

### Option C: Simplify
- Keep only essential endpoints:
  - `/tools` (list) - for discovery
  - `/tools/invoke` - for agent calls (if centralization needed)
  - `/tools/logs` - for debugging
- Remove fixtures, circuit breakers, bulk operations
- Reduce maintenance surface

---

## Current State

The `/tools/*` APIs are currently:
- Implemented and functional
- Registered with `tag = "tools"` in handlers
- Primarily used by the UI Tools Console

---

## ✅ DECISION MADE (February 2026)

### Public API (Agent-Facing)

Only **2 endpoints** exposed in `/openapi.json`:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/tools` | GET | **Discovery** - Agent learns available tools |
| `/tools/invoke` | POST | **Execution** - Agent calls tools through choke-point |

### Admin API (Internal/Dev-Only)

Moved to `/openapi_admin.json`:

| Endpoint | Purpose |
|----------|---------|
| `/tools/logs` | Execution logs for debugging |
| `/tools/config` | Global runtime configuration |
| `/tools/{id}/config` | Per-tool configuration |
| `/tools/circuit-breakers` | Circuit breaker monitoring |
| `/tools/fixtures` | Test fixture management |
| `/tools/enable-all` | Bulk enable |
| `/tools/disable-all` | Bulk disable |

### Rationale

- Agents need only discovery and execution
- Admin endpoints are for development/debugging
- Reduces public API surface area
- Keeps internal tooling separate from external interface

---

*Document created for assessment purposes - February 2026*
*Decision finalized - February 7, 2026*

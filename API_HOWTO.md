# API HOWTO Guide

This document explains the API architecture in the Jira Dashboard application and answers common questions about how endpoints are managed and displayed.

## Architecture Overview

The application has **two separate systems** for documenting REST API endpoints:

### 1. Backend OpenAPI Specification (Auto-Generated)

**Location:** `src-tauri/src/openapi.rs`

The Rust backend uses the `utoipa` crate to auto-generate an OpenAPI 3.1 specification from code annotations. When you add a handler function with `#[utoipa::path(...)]` annotations, it automatically becomes part of the OpenAPI spec.

**What it controls:**
- `/openapi.json` endpoint output
- Swagger-style API documentation
- Actual route registration in the Axum web server

**Example from handlers.rs:**
```rust
#[utoipa::path(
    get,
    path = "/agent/models",
    responses(...),
    tag = "agent"
)]
pub async fn list_models_handler(...) { ... }
```

### 2. Frontend Static Endpoint List (Manually Maintained)

**Location:** `src/lib/tabs/api/endpoints.ts`

The **API → REST** tab in the UI displays a static list of endpoints defined in `endpoints.ts`. This is a **manually maintained** TypeScript array that is **completely separate** from the backend OpenAPI spec.

**What it controls:**
- What endpoints appear in the API tab's REST sub-tab
- Display names, descriptions, and tags shown in the UI
- Which endpoints show "Auth" badges

---

## FAQ: Why is `/agent/models` Not Listed in the REST Tab?

**Answer:** The `/agent/models` endpoint IS implemented in the backend and IS included in the OpenAPI specification, but it is **NOT listed in the frontend's static `endpoints.ts` file**.

### The Two Systems Are Independent

| System | File | Updated By |
|--------|------|-----------|
| Backend OpenAPI | `openapi.rs` + handler annotations | Auto-generated from Rust code |
| Frontend UI List | `endpoints.ts` | **Manual developer update required** |

When new endpoints are added to the backend, developers must **manually add** them to `endpoints.ts` for them to appear in the REST tab.

### Currently Missing from `endpoints.ts`

As of the last update, these backend endpoints exist but are NOT in the frontend list:

1. **GET `/agent/models`** - List available Gemini AI models
2. **GET `/inference-logs`** - Get AI inference log entries  
3. **DELETE `/inference-logs`** - Clear AI inference logs

### Internal UI/Admin APIs (Intentionally Excluded from OpenAPI)

These endpoints are **internal UI/admin APIs** used by the application's Activity tab to display diagnostic logs. They are **intentionally excluded** from the public OpenAPI specification:

| Endpoint | Handler | Purpose |
|----------|---------|---------|
| GET `/access-logs` | `access_logs_handler` | Used by Activity tab → REST subtab to show HTTP access logs |
| DELETE `/access-logs` | `clear_access_logs_handler` | Clear button in the Activity tab |
| GET `/inference-logs` | `inference_logs_handler` | Used by Activity tab → Inference subtab to show AI call logs |
| DELETE `/inference-logs` | `clear_inference_logs_handler` | Clear button in the Activity tab |

**Why are they excluded from OpenAPI?**

1. **Internal Use Only** - These are admin/debugging endpoints consumed by the app's own UI, not intended for external API consumers
2. **No Authentication** - They don't require auth (unlike `/jira/list` or `/agent/chat`), as they're meant for local development diagnostics
3. **OpenAPI is for Public API** - The OpenAPI spec is meant to document the "public" API surface for external integrations

**Technical Note:** These handlers DO have `#[utoipa::path]` annotations in `handlers.rs` (for completeness), but they are intentionally **not registered** in the `paths()` macro in `openapi.rs`. This is a deliberate design choice to keep them out of the public API documentation.

If you wanted to include them in OpenAPI (not recommended), you would add them to the `paths()` macro:

```rust
#[openapi(
    paths(
        // Public APIs
        crate::api::handlers::health_handler,
        crate::api::handlers::jira_list_handler,
        crate::api::handlers::chat_handler,
        crate::api::handlers::list_models_handler,
        // Internal/Admin APIs (currently excluded):
        // crate::api::handlers::access_logs_handler,
        // crate::api::handlers::clear_access_logs_handler,
        // crate::api::handlers::inference_logs_handler,
        // crate::api::handlers::clear_inference_logs_handler,
    ),
    // ...
)]
```

---

## API Categories Summary

| Category | Endpoints | In OpenAPI? | In REST Tab UI? | Purpose |
|----------|-----------|-------------|-----------------|---------|
| **System** | `/health`, `/openapi.json` | ✅ Yes | ✅ Yes | Public health/status |
| **Jira** | `/jira/list` | ✅ Yes | ✅ Yes | Core Jira functionality |
| **Agent** | `/agent/chat`, `/agent/models` | ✅ Yes | ⚠️ Partial | AI chat features |
| **Admin/Logs** | `/access-logs`, `/inference-logs` | ❌ No (intentional) | ✅ Yes | Internal UI diagnostics |

---

## How to Add a New Endpoint to the REST Tab

When you add a new endpoint to the backend, follow these steps:

### Step 1: Implement the Handler (Backend)

Add your handler function in `src-tauri/src/api/handlers.rs` with utoipa annotations:

```rust
#[utoipa::path(
    get,
    path = "/your/endpoint",
    responses(...),
    tag = "your-tag"
)]
pub async fn your_handler(...) { ... }
```

### Step 2: Register in OpenAPI (Backend)

Add the handler to `src-tauri/src/openapi.rs`:

```rust
#[openapi(
    paths(
        // ... existing paths ...
        crate::api::handlers::your_handler,
    ),
    // ...
)]
```

### Step 3: Register the Route (Backend)

Add the route in `src-tauri/src/server.rs`:

```rust
.route("/your/endpoint", get(your_handler))
```

### Step 4: Update Frontend List (Frontend) ← **Often Forgotten!**

Add the endpoint to `src/lib/tabs/api/endpoints.ts`:

```typescript
export const endpoints: ApiEndpoint[] = [
  // ... existing endpoints ...
  {
    method: 'GET',
    path: '/your/endpoint',
    description: 'Description of what the endpoint does',
    tag: 'your-tag',
    auth: true  // or false
  }
];
```

---

## Why Have Two Separate Systems?

### Design Rationale

1. **Backend OpenAPI** - Serves as the canonical, machine-readable API specification that can be used by API clients, testing tools, and external documentation generators.

2. **Frontend Static List** - Provides a curated, user-friendly view of the endpoints. Not all backend endpoints may be relevant to display in the UI, and the frontend can add UI-specific metadata.

### Trade-offs

| Approach | Pros | Cons |
|----------|------|------|
| Static frontend list | Curated view, fast loading, no runtime dependency | Manual sync required, can get out of date |
| Fetch from `/openapi.json` | Always in sync with backend | Requires parsing OpenAPI at runtime, more complex |

### Future Improvements

#### Option 1: Dynamic Frontend Sync (Merged View, Grouped by Type)

Have the frontend **dynamically fetch and merge both specs** (`/openapi.json` + `/openapi_admin.json`) to auto-populate the REST tab, **grouped by API type**:

```typescript
// In RESTSubtab.svelte or api.ts

interface MergedEndpoint {
  method: string;
  path: string;
  description: string;
  tag: string;
  auth: boolean;
  apiType: 'public' | 'admin';  // New field for grouping
}

async function fetchAllEndpoints(): Promise<MergedEndpoint[]> {
  const endpoints: MergedEndpoint[] = [];
  
  // Fetch public API spec
  try {
    const publicSpec = await fetch('/openapi.json').then(r => r.json());
    endpoints.push(...parseOpenApiSpec(publicSpec, 'public'));
  } catch (e) { console.error('Failed to fetch public spec'); }
  
  // Fetch admin API spec (if available)
  try {
    const adminSpec = await fetch('/openapi_admin.json').then(r => r.json());
    endpoints.push(...parseOpenApiSpec(adminSpec, 'admin'));
  } catch (e) { /* Admin spec not available - that's OK */ }
  
  return endpoints;
}

function parseOpenApiSpec(spec: any, apiType: 'public' | 'admin'): MergedEndpoint[] {
  const endpoints: MergedEndpoint[] = [];
  for (const [path, methods] of Object.entries(spec.paths || {})) {
    for (const [method, details] of Object.entries(methods as any)) {
      endpoints.push({
        method: method.toUpperCase(),
        path,
        description: details.summary || details.description || '',
        tag: details.tags?.[0] || 'other',
        auth: !!details.security?.length,
        apiType
      });
    }
  }
  return endpoints;
}
```

**UI Display (Grouped by Type):**

```svelte
<!-- RESTSubtab.svelte -->
<script>
  // Group endpoints by apiType, then by tag
  $: groupedEndpoints = endpoints.reduce((acc, ep) => {
    const group = ep.apiType === 'public' ? 'Public API' : 'Admin API';
    if (!acc[group]) acc[group] = {};
    if (!acc[group][ep.tag]) acc[group][ep.tag] = [];
    acc[group][ep.tag].push(ep);
    return acc;
  }, {});
</script>

{#each Object.entries(groupedEndpoints) as [apiType, tagGroups]}
  <h2>{apiType}</h2>
  {#each Object.entries(tagGroups) as [tag, endpoints]}
    <h3>{tag}</h3>
    {#each endpoints as endpoint}
      <EndpointCard {endpoint} />
    {/each}
  {/each}
{/each}
```

**Result in UI:**

```
REST API Endpoints
├── Public API
│   ├── system
│   │   ├── GET /health
│   │   └── GET /openapi.json
│   ├── jira
│   │   └── GET /jira/list 🔒
│   └── agent
│       ├── POST /agent/chat 🔒
│       └── GET /agent/models 🔒
│
└── Admin API
    └── admin
        ├── GET /access-logs
        ├── DELETE /access-logs
        ├── GET /inference-logs
        └── DELETE /inference-logs
```

**Benefits:**
- ✅ Always in sync with backend (no manual `endpoints.ts` updates)
- ✅ Clear visual separation between public and admin APIs
- ✅ Single unified view for developers
- ✅ Admin spec still hidden from external tools (not linked from public spec)

#### Option 2: Separate OpenAPI Specs (Recommended for Admin APIs)

Create a **separate OpenAPI spec for admin/internal APIs**:

| Endpoint | Spec | Purpose |
|----------|------|---------|
| `/openapi.json` | Public API spec | External integrations, third-party consumers |
| `/openapi_admin.json` | Admin API spec | Internal UI, debugging, diagnostics |

**Benefits of separate specs:**
1. **Clear separation of concerns** - Public vs internal APIs are clearly delineated
2. **Different audiences** - Public spec for external devs, admin spec for internal tooling
3. **Security consideration** - Admin endpoints can be documented without exposing them in public docs
4. **Versioning flexibility** - Public API can be versioned independently of admin APIs

**Implementation approach:**
```rust
// In openapi.rs - Create two separate OpenAPI structs

/// Public API specification
#[derive(OpenApi)]
#[openapi(
    info(title = "Jira Dashboard API", version = "1.0.0"),
    paths(
        health_handler,
        jira_list_handler,
        chat_handler,
        list_models_handler,
    ),
    // ...
)]
pub struct PublicApiDoc;

/// Admin/Internal API specification  
#[derive(OpenApi)]
#[openapi(
    info(title = "Jira Dashboard Admin API", version = "1.0.0"),
    paths(
        access_logs_handler,
        clear_access_logs_handler,
        inference_logs_handler,
        clear_inference_logs_handler,
    ),
    tags(
        (name = "admin", description = "Internal admin/diagnostic endpoints")
    )
)]
pub struct AdminApiDoc;
```

Then expose both:
```rust
// In server.rs
.route("/openapi.json", get(|| async { Json(PublicApiDoc::openapi()) }))
.route("/openapi_admin.json", get(|| async { Json(AdminApiDoc::openapi()) }))
```

**Discoverability (Intentionally Hidden):**

Unlike `/openapi.json` which is a well-known standard path that tools auto-discover, `/openapi_admin.json` is **intentionally NOT auto-discoverable**. This is a **security feature**:

- ✅ External tools scanning for `/openapi.json` will only find the public API
- ✅ Admin endpoints remain hidden from automated API discovery
- ✅ Only developers who know the path can access the admin spec
- ✅ Reduces attack surface by not advertising internal endpoints

**Access for internal use:** Developers who need the admin spec simply access it directly at `http://localhost:3030/openapi_admin.json`. No linking, no advertising - you either know it exists or you don't.

**When to implement:** This is a good enhancement if:
- You plan to expose the public API to third parties
- You want formal documentation for admin/debug endpoints
- You need different security policies for different API groups

---

## Verifying Endpoint Registration

### Check if Endpoint is in Backend OpenAPI

```powershell
# Run the app, then:
curl http://localhost:3030/openapi.json | jq '.paths'
```

Or use the provided script:
```powershell
.\scripts\list_openapi.ps1
```

### Check if Endpoint is in Frontend List

Open `src/lib/tabs/api/endpoints.ts` and verify the endpoint is in the array.

### Test the Actual Endpoint

```powershell
# Example: Test agent/models endpoint
curl http://localhost:3030/agent/models
```

---

## Summary

**Q: Why doesn't `/agent/models` appear in the API → REST tab?**

**A:** The endpoint exists in the backend and works correctly, but someone forgot to add it to the frontend's static list in `endpoints.ts`. The backend OpenAPI spec and frontend display list are **independent systems** that require **manual synchronization**.

To fix this, add the missing endpoint to `endpoints.ts`:

```typescript
{
  method: 'GET',
  path: '/agent/models',
  description: 'List available Gemini AI models',
  tag: 'agent',
  auth: true
}
```

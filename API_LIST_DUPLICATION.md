# API List Duplication - RESOLVED ✅

## Status: IMPLEMENTED (Option 2)

**Implementation Date:** February 2026

The API endpoint duplication issue has been resolved by implementing **Option 2: Fetch from OpenAPI at Runtime**.

---

## What Was Changed

### Files Modified:
- **`src/lib/tabs/api/utils.ts`** - Added `fetchEndpointsFromOpenApi()` function that fetches and merges both OpenAPI specs
- **`src/lib/tabs/api/RESTSubtab.svelte`** - Updated to fetch endpoints at runtime with loading/error states
- **`src/lib/tabs/api/index.ts`** - Updated exports to use new runtime functions

### Files Removed:
- **`src/lib/tabs/api/endpoints.ts`** - Deleted (static endpoint list no longer needed)

---

## How It Works Now

### Single Source of Truth
The backend OpenAPI specs (`/openapi.json` and `/openapi_admin.json`) are now the **only** source of endpoint definitions.

### Runtime Fetching
When the REST tab loads:
1. Fetches `/openapi.json` (public endpoints)
2. Fetches `/openapi_admin.json` (admin endpoints)
3. Parses both specs into `ApiEndpoint[]` objects
4. Derives `apiType: 'public' | 'admin'` based on which spec the endpoint came from
5. Merges and deduplicates (public endpoints take precedence)
6. Caches results for the session

### Session Caching
Results are cached in memory to avoid repeated network requests:
- First load: Fetches from backend
- Subsequent navigations: Uses cached data
- Refresh button: Clears cache and fetches fresh data

### Loading & Error States
- Shows spinner while fetching
- Displays error message if backend is unavailable
- "Try Again" button for retry

---

## Implementation Details

### Key Function: `fetchEndpointsFromOpenApi()`
```typescript
// In src/lib/tabs/api/utils.ts
export async function fetchEndpointsFromOpenApi(forceRefresh = false): Promise<ApiEndpoint[]> {
  // Return cached data if available
  if (cachedEndpoints && !forceRefresh) {
    return cachedEndpoints;
  }

  // Fetch both specs in parallel
  const [publicResp, adminResp] = await Promise.all([
    fetch(`${apiInfo.base_url}/openapi.json`),
    fetch(`${apiInfo.base_url}/openapi_admin.json`)
  ]);

  // Parse and merge endpoints
  const publicEndpoints = parseOpenApiSpec(publicSpec, 'public');
  const adminEndpoints = parseOpenApiSpec(adminSpec, 'admin');
  
  // Merge, dedupe, sort, and cache
  cachedEndpoints = mergeAndSort(publicEndpoints, adminEndpoints);
  return cachedEndpoints;
}
```

### Component Usage
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchEndpointsFromOpenApi } from './utils';

  let endpoints: ApiEndpoint[] = [];
  let isLoading = true;
  let error: string | null = null;

  onMount(() => loadEndpoints());

  async function loadEndpoints() {
    isLoading = true;
    try {
      endpoints = await fetchEndpointsFromOpenApi();
    } catch (e) {
      error = e.message;
    } finally {
      isLoading = false;
    }
  }
</script>
```

---

## Benefits Achieved

| Benefit | Description |
|---------|-------------|
| ✅ Single source of truth | Backend OpenAPI spec is the only definition |
| ✅ Always in sync | UI automatically reflects backend changes |
| ✅ No duplication | Removed `endpoints.ts` entirely |
| ✅ No maintenance burden | No need to update two files when adding endpoints |
| ✅ Session caching | Efficient - only fetches once per session |
| ✅ Refresh capability | Manual refresh available via button |

---

## Trade-offs Accepted

| Trade-off | Mitigation |
|-----------|------------|
| Requires backend running | Clear error message with retry button |
| Loading state needed | Implemented loading spinner |
| Network latency on first load | Session caching minimizes impact |

---

## Original Problem (Archived)

<details>
<summary>Click to expand original problem description</summary>

### The Problem

We had API endpoint definitions in **two places**:

| Location | File | Purpose |
|----------|------|---------|
| Frontend | `src/lib/tabs/api/endpoints.ts` | Static list for UI display |
| Backend | `src-tauri/src/openapi.rs` | OpenAPI spec generation (source of truth) |

This created **maintenance overhead** - when adding/modifying an endpoint, you had to update BOTH files.

### Risk
Every time a new endpoint was added:
1. Developer adds handler in `handlers.rs`
2. Developer registers in `openapi.rs` ✓
3. Developer **forgets** to update `endpoints.ts` ✗

Result: UI REST tab shows stale/incomplete list.

</details>

---

*Document updated: February 2026*

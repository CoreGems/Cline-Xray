# UI vs CLI Comparison: Jira API Parsing Issue

## Problem Summary

The Tauri UI application failed to display Jira issues despite the API returning a successful 200 OK response with valid JSON data containing 10 issues. The standalone CLI Rust script (`src-tauri/src/bin/jlist_requests.rs`) worked correctly while the Tauri module (`src-tauri/src/jira.rs`) failed.

## Root Cause

The `/rest/api/3/search/jql` endpoint returns a **different JSON structure** than the older `/rest/api/3/search` endpoint:

### Actual API Response Structure from `/search/jql`:
```json
{
  "issues": [...],
  "isLast": true
}
```

### Expected Structure by Tauri Code (INCORRECT):
```rust
struct JiraSearchResponse {
    issues: Vec<JiraIssue>,
    total: i32,  // ❌ REQUIRED - but NOT present in API response!
}
```

### CLI Code Structure (CORRECT):
```rust
struct SearchResult {
    issues: Vec<Issue>,
    total: Option<usize>,  // ✅ OPTIONAL - works when field is missing
}
```

## Key Differences

| Aspect | CLI Version | Tauri Version (Before Fix) |
|--------|-------------|---------------------------|
| **File** | `src-tauri/src/bin/jlist_requests.rs` | `src-tauri/src/jira.rs` |
| **`total` field** | `Option<usize>` (optional) | `i32` (required) |
| **All other fields** | Most marked as `Option<T>` | Mix of required and optional |
| **Error handling** | Gracefully handles missing fields | Fails on missing required fields |
| **Result** | ✅ Works | ❌ Parse error: "missing field `total`" |

## The Fix

Changed `JiraSearchResponse` in `src-tauri/src/jira.rs`:

```rust
// BEFORE (broken):
#[derive(Debug, Deserialize)]
struct JiraSearchResponse {
    issues: Vec<JiraIssue>,
    total: i32,  // ❌ Required field not in API response
}

// AFTER (fixed):
#[derive(Debug, Deserialize)]
struct JiraSearchResponse {
    issues: Vec<JiraIssue>,
    #[serde(default)]
    total: Option<i32>,  // ✅ Optional - defaults to None if missing
    #[serde(rename = "isLast", default)]
    is_last: Option<bool>,  // ✅ Capture the actual field from API
}
```

And updated the code that uses `total`:

```rust
// BEFORE:
Ok(SearchResult {
    total: data.total,  // ❌ Assumes i32
    issues,
})

// AFTER:
let total = data.total.unwrap_or(data.issues.len() as i32);  // ✅ Fallback to count
Ok(SearchResult {
    total,
    issues,
})
```

## Lessons Learned

1. **Always use `#[serde(default)]` or `Option<T>` for API fields** that might not be present
2. **The `/search/jql` endpoint has a different response format** than `/search`
   - `/search` returns: `total`, `startAt`, `maxResults`, `issues`
   - `/search/jql` returns: `issues`, `isLast` (no `total`)
3. **Test with actual API responses**, not just documentation
4. **Add comprehensive logging** to trace parsing failures (we added `log_to_file()`)

## Debug Logging Added

The fix also includes file-based logging to `%TEMP%\jira_viewer_debug.log` for easier debugging:

```rust
fn log_to_file(message: &str) {
    // Write to temp directory to avoid triggering rebuilds
    let log_path = std::env::temp_dir().join("jira_viewer_debug.log");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{}] {}", timestamp, message);
    }
}
```

**Note:** The log file is written to the temp directory (e.g., `C:\Users\<user>\AppData\Local\Temp\jira_viewer_debug.log`) to avoid triggering rebuild loops when the file is modified.

This logs:
- API URL and JQL query
- Response status and body length
- First 2000 characters of response body
- Parse success/failure with exact error location
- Number of issues parsed and returned

## Verification

After the fix, the debug log shows:
```
[2026-02-05 12:21:XX] === Starting search_issues ===
[2026-02-05 12:21:XX] Response status: 200 OK
[2026-02-05 12:21:XX] JSON parsing successful!
[2026-02-05 12:21:XX] Parsed 10 issues, total: 10
[2026-02-05 12:21:XX] Returning 10 issues to frontend
```

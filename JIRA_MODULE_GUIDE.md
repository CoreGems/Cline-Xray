# Jira Module Guide

This document describes the Jira module structure created from converting `jlist_requests.py` to Rust.

## Module Structure

### Files Created/Modified

1. **`src-tauri/src/jira.rs`** - New Jira client module
   - Contains all Jira API interaction logic
   - Provides a clean, reusable `JiraClient` struct
   - Handles authentication, requests, and response parsing

2. **`src-tauri/src/main.rs`** - Refactored to use the module
   - Now imports and uses the `jira` module
   - Simplified code by delegating API calls to `JiraClient`
   - Maintains app state and Tauri command handlers

3. **`src-tauri/src/bin/jlist_requests.rs`** - Standalone CLI tool
   - Equivalent to the original Python script
   - Can be run independently for testing
   - Demonstrates how to use the Jira module

4. **`src-tauri/Cargo.toml`** - Updated dependencies
   - Added `dotenvy = "0.15"` for `.env` file support

## Jira Module API

### JiraClient

The main struct for interacting with Jira:

```rust
pub struct JiraClient {
    base_url: String,
    email: String,
    api_token: String,
    client: Client,
}
```

#### Methods

**`new(base_url: String, email: String, api_token: String) -> Self`**
- Creates a new Jira client instance

**`get_current_user() -> Result<JiraCurrentUser, String>`**
- Fetches the currently authenticated user's information
- Returns user display name, email, and account ID

**`search_issues(jql: &str, max_results: u32) -> Result<SearchResult, String>`**
- Searches for issues using JQL (Jira Query Language)
- Uses the `/rest/api/3/search/jql` endpoint
- Returns a list of issue summaries with status, priority, etc.

**`get_issue(key: &str) -> Result<IssueDetails, String>`**
- Fetches detailed information about a specific issue
- Returns full issue details including description, labels, components, etc.

### Data Models

#### Public Models
- `JiraSettings` - Configuration for connecting to Jira
- `IssueSummary` - Brief issue information for list views
- `IssueDetails` - Complete issue information
- `SearchResult` - Container for search results with total count
- `JiraCurrentUser` - User information from authentication

## Usage Examples

### In Tauri Commands

```rust
#[tauri::command]
async fn list_issues(jql: String) -> Result<SearchResult, String> {
    let client = create_jira_client()?;
    client.search_issues(&jql, 100).await
}
```

### Standalone CLI

Run the standalone binary:
```bash
cargo run --bin jlist_requests --manifest-path src-tauri/Cargo.toml
```

This will:
1. Load credentials from `.env` file
2. Authenticate with Jira
3. Fetch issues assigned to the current user
4. Display them in a formatted list

## Benefits of the Module Structure

1. **Separation of Concerns**: API logic separated from UI/app logic
2. **Reusability**: The `JiraClient` can be used in multiple contexts
3. **Type Safety**: Strong typing with Rust's type system
4. **Testability**: Module can be tested independently
5. **Maintainability**: Changes to Jira API only affect one module
6. **Performance**: Rust's performance benefits over Python
7. **Error Handling**: Explicit Result types for better error handling

## Integration with Your Tauri App

The module is already integrated into your `jira-viewer` Tauri app:

- **Settings Management**: `is_configured()`, `get_settings()`, `save_settings()`
- **User Info**: `get_current_user()`
- **Issue Operations**: `list_issues()`, `get_issue()`

All Tauri commands use the centralized `JiraClient` through the `create_jira_client()` helper function.

## Future Enhancements

The module can be easily extended to add:
- Issue creation/updates
- Comment management
- Attachment handling
- Transition workflows
- Custom field support
- Batch operations
- Caching strategies
- Rate limiting

## Comparison: Python vs Rust

### Python (jlist_requests.py)
- Uses `requests` library
- Manual header management
- Dynamic typing
- Interpreted execution

### Rust (jira.rs module)
- Uses `reqwest` (async HTTP client)
- Type-safe header handling
- Static typing with `serde` for JSON
- Compiled, optimized code
- Memory safety guarantees
- Better error handling with `Result` types

Both implementations provide the same functionality, but the Rust version offers better performance, type safety, and integration with the Tauri framework.

# Logging Implementation for jira-viewer (Python-Like Logging in Rust/Tauri)

## Overview

This document reasons through implementing Python-like logging for the **jira-viewer** Tauri application, specifically focused on logging Jira API calls with configurable INFO/DEBUG levels.

---

## Current State

The codebase already has a basic logging mechanism in `src-tauri/src/jira.rs`:

```rust
fn log_to_file(message: &str) {
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

**Problems with current approach:**
1. No log levels (always logs everything)
2. Hardcoded to temp directory
3. No configuration file support
4. Not reusable across modules
5. No structured logging

---

## Python Logging Model (Reference)

Python's logging module provides an elegant, hierarchical logging system:

```python
import logging

# Configure from file or code
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)

logger.debug("Detailed diagnostic info")  # Only when DEBUG
logger.info("General operational info")    # INFO and above
logger.warning("Something unexpected")     # WARNING and above
logger.error("Something failed")           # ERROR and above
```

**Key Python logging features we want to emulate:**
1. **Log Levels**: DEBUG, INFO, WARNING, ERROR (severity hierarchy)
2. **Config File**: External configuration (not hardcoded)
3. **Formatters**: Timestamp, level, module name, message
4. **Handlers**: Where logs go (file, console, etc.)

---

## Rust Logging Ecosystem

### Option 1: `log` + `env_logger` (Simple)

The `log` crate is Rust's de-facto logging facade (like Python's `logging` interface).

```toml
# Cargo.toml additions
log = "0.4"
env_logger = "0.11"
```

```rust
use log::{debug, info, warn, error};

fn main() {
    env_logger::init();  // Reads RUST_LOG env var
    
    info!("Application started");
    debug!("Raw request: {:?}", request);
}
```

**Pros:** Simple, widely used, environment variable config
**Cons:** Limited config file support, basic formatting

---

### Option 2: `log` + `simplelog` (File-Based Config) ⭐ RECOMMENDED

The `simplelog` crate provides Python-like file logging with easy configuration.

```toml
# Cargo.toml additions
log = "0.4"
simplelog = "0.12"
```

```rust
use log::{debug, info, error, LevelFilter};
use simplelog::*;
use std::fs::File;

fn init_logging(level: LevelFilter, log_path: &str) {
    CombinedLogger::init(vec![
        // Console output (for dev)
        TermLogger::new(
            level,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto
        ),
        // File output (always)
        WriteLogger::new(
            level,
            Config::default(),
            File::create(log_path).unwrap()
        ),
    ]).unwrap();
}
```

**Pros:** Multiple outputs, simple config, good defaults
**Cons:** Less flexible than tracing

---

### Option 3: `tracing` + `tracing-subscriber` (Production-Grade)

The `tracing` ecosystem is more modern and feature-rich.

```toml
# Cargo.toml additions
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

```rust
use tracing::{debug, info, error, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[instrument]  // Automatic function entry/exit logging
async fn search_issues(&self, jql: &str) -> Result<SearchResult, String> {
    info!(jql = %jql, "Searching issues");
    // ...
}
```

**Pros:** Async-aware, structured logging, spans, instruments
**Cons:** More complex, heavier dependency

---

## Recommended Solution: `log` + `simplelog` + TOML Config

For a Tauri app with Python-like simplicity, I recommend:

### 1. Config File Structure

Create `config.toml` in the app data directory:

```toml
# jira-viewer config
[logging]
level = "INFO"          # Options: "DEBUG", "INFO", "WARN", "ERROR"
log_file = "jira_viewer.log"
log_to_console = true
max_file_size_mb = 10   # Optional: rotate logs
```

### 2. Dependencies to Add

```toml
# src-tauri/Cargo.toml additions
[dependencies]
log = "0.4"
simplelog = "0.12"
toml = "0.8"           # For config file parsing
directories = "5.0"    # For app data paths
```

### 3. Implementation Architecture

```
src-tauri/src/
├── main.rs           # Initialize logging on startup
├── jira.rs           # Jira API calls (use log macros)
├── config.rs         # NEW: Config file handling
└── logging.rs        # NEW: Logging initialization
```

---

## Implementation Code

### `src-tauri/src/config.rs`

```rust
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,           // "DEBUG", "INFO", "WARN", "ERROR"
    pub log_file: String,
    pub log_to_console: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "INFO".to_string(),
            log_file: "jira_viewer.log".to_string(),
            log_to_console: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            logging: LoggingConfig::default(),
        }
    }
}

pub fn get_config_path() -> PathBuf {
    // Use app data directory (cross-platform)
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "jira", "viewer") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).ok();
        config_dir.join("config.toml")
    } else {
        // Fallback to current directory
        PathBuf::from("config.toml")
    }
}

pub fn get_log_path() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "jira", "viewer") {
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir).ok();
        data_dir.join("jira_viewer.log")
    } else {
        std::env::temp_dir().join("jira_viewer.log")
    }
}

pub fn load_config() -> AppConfig {
    let config_path = get_config_path();
    
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<AppConfig>(&content) {
                return config;
            }
        }
    }
    
    // Create default config file if it doesn't exist
    let default_config = AppConfig::default();
    let toml_content = r#"# jira-viewer configuration

[logging]
# Log level: "DEBUG", "INFO", "WARN", "ERROR"
# DEBUG includes raw HTTP request/response data
level = "INFO"
log_file = "jira_viewer.log"
log_to_console = true
"#;
    
    fs::write(&config_path, toml_content).ok();
    default_config
}
```

### `src-tauri/src/logging.rs`

```rust
use log::LevelFilter;
use simplelog::*;
use std::fs::File;
use crate::config::{load_config, get_log_path};

pub fn init_logging() {
    let config = load_config();
    let log_path = get_log_path();
    
    // Parse log level from config
    let level = match config.logging.level.to_uppercase().as_str() {
        "DEBUG" => LevelFilter::Debug,
        "INFO" => LevelFilter::Info,
        "WARN" | "WARNING" => LevelFilter::Warn,
        "ERROR" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };
    
    // Build logger configuration
    let log_config = ConfigBuilder::new()
        .set_time_format_rfc3339()
        .set_target_level(LevelFilter::Error)
        .set_location_level(LevelFilter::Debug)
        .build();
    
    let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();
    
    // Console logger (for development)
    if config.logging.log_to_console {
        loggers.push(TermLogger::new(
            level,
            log_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ));
    }
    
    // File logger (always, if path is writable)
    if let Ok(file) = File::create(&log_path) {
        loggers.push(WriteLogger::new(level, log_config.clone(), file));
    }
    
    // Initialize combined logger
    if !loggers.is_empty() {
        CombinedLogger::init(loggers).ok();
    }
    
    log::info!("Logging initialized at level: {:?}", level);
    log::info!("Log file: {:?}", log_path);
}
```

### Updated `src-tauri/src/jira.rs` (Key Changes)

```rust
use log::{debug, info, error};
// ... other imports

impl JiraClient {
    /// Search for issues using JQL
    pub async fn search_issues(&self, jql: &str, max_results: u32) -> Result<SearchResult, String> {
        let url = format!("{}/rest/api/3/search/jql", self.base_url);
        
        // INFO level: Basic operation logging
        info!("Searching issues with JQL: {}", jql);
        
        // DEBUG level: Full request details (raw data)
        debug!("Request URL: {}", url);
        debug!("Request params: maxResults={}, fields=key,summary,status,...", max_results);
        
        let response = self
            .client
            .get(&url)
            .query(&[
                ("jql", jql),
                ("maxResults", &max_results.to_string()),
                ("fields", "key,summary,status,updated,assignee,priority,issuetype"),
            ])
            .header("Authorization", self.get_auth_header())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                // ERROR level: Always log failures
                error!("HTTP request failed: {}", e);
                // DEBUG level: Include full error details
                debug!("Request error details: {:?}", e);
                format!("Request failed: {}", e)
            })?;

        let status = response.status();
        info!("Response status: {}", status);
        
        let body_text = response.text().await.map_err(|e| {
            error!("Failed to read response body: {}", e);
            format!("Failed to read response body: {}", e)
        })?;

        // DEBUG level: Raw response body
        debug!("Response body length: {} bytes", body_text.len());
        debug!("Response body (truncated): {}", &body_text[..body_text.len().min(2000)]);

        if !status.is_success() {
            error!("API error {}: {}", status, &body_text[..body_text.len().min(500)]);
            return Err(format!("API error {}: {}", status, body_text));
        }

        // Parse response
        let data: JiraSearchResponse = serde_json::from_str(&body_text).map_err(|e| {
            error!("JSON parse error: {}", e);
            debug!("Failed to parse response: {}", body_text);
            format!("Failed to parse response: {}", e)
        })?;
        
        let total = data.total.unwrap_or(data.issues.len() as i32);
        info!("Found {} issues (total: {})", data.issues.len(), total);

        // ... rest of implementation
    }
}
```

### Updated `src-tauri/src/main.rs` (Initialization)

```rust
mod config;
mod jira;
mod logging;

use log::info;

fn main() {
    // Initialize logging FIRST
    logging::init_logging();
    
    info!("Starting Jira Viewer application");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            is_configured,
            get_settings,
            save_settings,
            get_current_user,
            list_issues,
            get_issue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Log Level Behavior Summary

| Level | What Gets Logged |
|-------|------------------|
| **ERROR** | Failures, exceptions, API errors |
| **WARN** | Deprecated usage, recoverable issues |
| **INFO** | API calls (operation name, basic params, result count) |
| **DEBUG** | Raw HTTP request URLs, query params, response bodies, full error traces |

### Example Log Output

**At INFO level:**
```
[2026-02-05T12:30:15] INFO  jira_viewer::jira - Searching issues with JQL: assignee = currentUser()
[2026-02-05T12:30:16] INFO  jira_viewer::jira - Response status: 200 OK
[2026-02-05T12:30:16] INFO  jira_viewer::jira - Found 25 issues (total: 42)
```

**At DEBUG level:**
```
[2026-02-05T12:30:15] INFO  jira_viewer::jira - Searching issues with JQL: assignee = currentUser()
[2026-02-05T12:30:15] DEBUG jira_viewer::jira - Request URL: https://jira.example.com/rest/api/3/search/jql
[2026-02-05T12:30:15] DEBUG jira_viewer::jira - Request params: maxResults=100, fields=key,summary,status,...
[2026-02-05T12:30:16] INFO  jira_viewer::jira - Response status: 200 OK
[2026-02-05T12:30:16] DEBUG jira_viewer::jira - Response body length: 45230 bytes
[2026-02-05T12:30:16] DEBUG jira_viewer::jira - Response body (truncated): {"issues":[{"key":"PROJ-123",...
[2026-02-05T12:30:16] INFO  jira_viewer::jira - Found 25 issues (total: 42)
```

---

## Config File Location

The config file is stored in the standard app config directory:

| OS | Path |
|----|------|
| Windows | `C:\Users\<user>\AppData\Roaming\com.jira.viewer\config\config.toml` |
| macOS | `~/Library/Application Support/com.jira.viewer/config.toml` |
| Linux | `~/.config/com.jira.viewer/config.toml` |

---

## Migration Steps

1. **Add dependencies** to `Cargo.toml`:
   ```toml
   log = "0.4"
   simplelog = "0.12"
   toml = "0.8"
   directories = "5.0"
   ```

2. **Create** `src-tauri/src/config.rs` and `src-tauri/src/logging.rs`

3. **Update** `src-tauri/src/main.rs` to call `logging::init_logging()` at startup

4. **Replace** all `log_to_file()` calls in `jira.rs` with appropriate `log::info!()`, `log::debug!()`, etc.

5. **Remove** the old `log_to_file()` function from `jira.rs`

6. **Test** by:
   - Setting `level = "DEBUG"` in config.toml
   - Making API calls
   - Checking the log file

---

## Advanced Features (Future)

1. **Log Rotation**: Use `tracing-appender` with rolling file appender
2. **Remote Logging**: Send logs to a server for centralized monitoring  
3. **Structured JSON Logs**: Use `tracing-subscriber` JSON formatter
4. **Per-Module Levels**: Configure different levels for different modules
5. **UI Log Viewer**: Add a Tauri command to read recent logs for display in the app

---

## Security Considerations

⚠️ **IMPORTANT**: The DEBUG level logs raw request/response data which may include:
- API tokens (in Authorization headers - should be masked)
- Sensitive issue content
- User information

**Recommendations:**
1. Default to INFO level in production
2. Add header masking for Authorization:
   ```rust
   fn mask_auth_header(headers: &str) -> String {
       // Replace "Basic <token>" with "Basic <MASKED>"
       headers.replace(|c: char| c.is_alphanumeric(), "*")
   }
   ```
3. Warn users that DEBUG mode logs sensitive data
4. Implement log file permissions (read-only for current user)

---

## Summary

This implementation provides Python-like logging semantics in Rust:

| Python | Rust Equivalent |
|--------|-----------------|
| `logging.basicConfig(level=logging.DEBUG)` | `init_logging()` reads from config.toml |
| `logger.info("message")` | `log::info!("message")` |
| `logger.debug("raw data: %s", data)` | `log::debug!("raw data: {:?}", data)` |
| Config in `.ini` or `.yaml` | Config in `config.toml` |

The `log` + `simplelog` combination gives us:
- ✅ Configurable log levels
- ✅ File-based configuration
- ✅ Multiple outputs (console + file)
- ✅ Timestamps and formatting
- ✅ Minimal dependencies
- ✅ Familiar Python-like API

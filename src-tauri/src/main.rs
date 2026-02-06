// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod config;
mod jira;
mod logging;
mod openapi;
mod server;
mod state;

use config::get_config_dir;
use jira::{IssueDetails, IssueSummary, JiraClient, JiraSettings, SearchResult};
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use state::{AccessLogEntry, InferenceLogEntry, AppState};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};

// ============ App State ============

static SETTINGS: Lazy<Mutex<Option<JiraSettings>>> = Lazy::new(|| {
    // Pre-configured settings from jtest.py
    Mutex::new(Some(JiraSettings {
        base_url: "https://sonymusicpub.atlassian.net".to_string(),
        email: "olek.buzunov@sonymusicpub.com".to_string(),
        default_jql: "assignee = currentUser() ORDER BY updated DESC".to_string(),
    }))
});

static API_TOKEN: Lazy<Mutex<Option<String>>> = Lazy::new(|| {
    // Pre-configured token from jtest.py
    Mutex::new(Some("ATATT3xFfGF05WlwNQcT5vfYU5ZgU4eYmOTdpLbiDCvjVDuUR8OIQtW8wpNFFmvA40nJ3Uz009el9BCQrW8xtXjYn7eDFIOiEmoOgU-hhZgvk4vhQwQ2TxCp9gSsEGmDGPDfne-1E1lIQasPxb-romVXEIp8dRKcS-AyYnS8spIA4fqJpVyleSk=026AE52B".to_string()))
});

static ISSUE_CACHE: Lazy<Mutex<HashMap<String, IssueDetails>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// ============ Issues List Cache ============

/// Cached issues list structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IssuesCache {
    issues: Vec<IssueSummary>,
    jql: String,
    cached_at: String,
}

/// Get the path to the issues cache file
fn get_issues_cache_path() -> std::path::PathBuf {
    get_config_dir().join("issues_cache.json")
}

/// Save issues to cache file
fn save_issues_cache(issues: &[IssueSummary], jql: &str) -> Result<(), String> {
    let cache = IssuesCache {
        issues: issues.to_vec(),
        jql: jql.to_string(),
        cached_at: chrono::Local::now().to_rfc3339(),
    };
    
    let cache_path = get_issues_cache_path();
    let json = serde_json::to_string_pretty(&cache)
        .map_err(|e| format!("Failed to serialize cache: {}", e))?;
    
    fs::write(&cache_path, json)
        .map_err(|e| format!("Failed to write cache file: {}", e))?;
    
    info!("Saved {} issues to cache at {:?}", issues.len(), cache_path);
    Ok(())
}

/// Load issues from cache file
fn load_issues_cache() -> Result<Option<IssuesCache>, String> {
    let cache_path = get_issues_cache_path();
    
    if !cache_path.exists() {
        info!("No cache file found at {:?}", cache_path);
        return Ok(None);
    }
    
    let content = fs::read_to_string(&cache_path)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;
    
    let cache: IssuesCache = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cache file: {}", e))?;
    
    info!("Loaded {} issues from cache (cached at: {})", cache.issues.len(), cache.cached_at);
    Ok(Some(cache))
}

// ============ Issue Details Cache ============

/// Cached issue details structure
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IssueDetailsCache {
    details: IssueDetails,
    cached_at: String,
}

/// Get the path to the issue details cache directory
fn get_issue_details_cache_dir() -> std::path::PathBuf {
    get_config_dir().join("issue_details")
}

/// Get the path to a specific issue's cache file
fn get_issue_details_cache_path(key: &str) -> std::path::PathBuf {
    // Replace any characters that might be problematic in filenames
    let safe_key = key.replace("/", "_").replace("\\", "_");
    get_issue_details_cache_dir().join(format!("{}.json", safe_key))
}

/// Save issue details to cache file
fn save_issue_details_cache(details: &IssueDetails) -> Result<(), String> {
    let cache = IssueDetailsCache {
        details: details.clone(),
        cached_at: chrono::Local::now().to_rfc3339(),
    };
    
    // Ensure cache directory exists
    let cache_dir = get_issue_details_cache_dir();
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {}", e))?;
    }
    
    let cache_path = get_issue_details_cache_path(&details.key);
    let json = serde_json::to_string_pretty(&cache)
        .map_err(|e| format!("Failed to serialize issue details cache: {}", e))?;
    
    fs::write(&cache_path, json)
        .map_err(|e| format!("Failed to write issue details cache file: {}", e))?;
    
    info!("Saved issue {} details to cache at {:?}", details.key, cache_path);
    Ok(())
}

/// Load issue details from cache file
fn load_issue_details_cache(key: &str) -> Result<Option<IssueDetailsCache>, String> {
    let cache_path = get_issue_details_cache_path(key);
    
    if !cache_path.exists() {
        return Ok(None);
    }
    
    let content = fs::read_to_string(&cache_path)
        .map_err(|e| format!("Failed to read issue details cache file: {}", e))?;
    
    let cache: IssueDetailsCache = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse issue details cache file: {}", e))?;
    
    info!("Loaded issue {} from cache (cached at: {})", key, cache.cached_at);
    Ok(Some(cache))
}

// ============ Helper Functions ============

fn create_jira_client() -> Result<JiraClient, String> {
    let settings = SETTINGS
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?
        .clone()
        .ok_or("Settings not configured")?;

    let token = API_TOKEN
        .lock()
        .map_err(|e| format!("Failed to lock token: {}", e))?
        .clone()
        .ok_or("API token not configured")?;

    Ok(JiraClient::new(settings.base_url, settings.email, token))
}

// ============ Tauri Commands ============

#[tauri::command]
fn is_configured() -> bool {
    let settings = SETTINGS.lock().ok();
    let token = API_TOKEN.lock().ok();

    settings.map(|s| s.is_some()).unwrap_or(false)
        && token.map(|t| t.is_some()).unwrap_or(false)
}

#[tauri::command]
fn get_settings() -> Result<JiraSettings, String> {
    let settings = SETTINGS
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    settings
        .clone()
        .ok_or_else(|| "Settings not configured".to_string())
}

#[tauri::command]
fn save_settings(settings: JiraSettings, api_token: String) -> Result<(), String> {
    let mut stored_settings = SETTINGS
        .lock()
        .map_err(|e| format!("Failed to lock settings: {}", e))?;
    let mut stored_token = API_TOKEN
        .lock()
        .map_err(|e| format!("Failed to lock token: {}", e))?;

    *stored_settings = Some(settings);
    *stored_token = Some(api_token);

    Ok(())
}

#[tauri::command]
async fn get_current_user() -> Result<serde_json::Value, String> {
    let client = create_jira_client()?;
    let user = client.get_current_user().await?;
    
    // Convert to generic JSON for backward compatibility
    Ok(serde_json::json!({
        "displayName": user.display_name,
        "emailAddress": user.email_address,
        "accountId": user.account_id,
    }))
}

#[tauri::command]
async fn list_issues(jql: String) -> Result<SearchResult, String> {
    let client = create_jira_client()?;
    let result = client.search_issues(&jql, 100).await?;
    
    // Save to cache after successful fetch
    if let Err(e) = save_issues_cache(&result.issues, &jql) {
        error!("Failed to save issues to cache: {}", e);
    }
    
    Ok(result)
}

/// Get cached issues from disk (for app startup)
#[tauri::command]
fn get_cached_issues() -> Result<Option<SearchResult>, String> {
    match load_issues_cache()? {
        Some(cache) => Ok(Some(SearchResult {
            issues: cache.issues,
            total: 0, // We don't store total in cache
        })),
        None => Ok(None),
    }
}

#[tauri::command]
async fn get_issue(key: String) -> Result<IssueDetails, String> {
    // Check memory cache first
    {
        let cache = ISSUE_CACHE
            .lock()
            .map_err(|e| format!("Failed to lock cache: {}", e))?;
        if let Some(cached) = cache.get(&key) {
            info!("Returning issue {} from memory cache", key);
            return Ok(cached.clone());
        }
    }

    // Check disk cache next
    if let Ok(Some(disk_cache)) = load_issue_details_cache(&key) {
        info!("Returning issue {} from disk cache", key);
        // Also put in memory cache for faster subsequent access
        let mut cache = ISSUE_CACHE
            .lock()
            .map_err(|e| format!("Failed to lock cache: {}", e))?;
        cache.insert(key, disk_cache.details.clone());
        return Ok(disk_cache.details);
    }

    // Fetch from API
    let client = create_jira_client()?;
    let details = client.get_issue(&key).await?;

    // Cache in memory
    {
        let mut cache = ISSUE_CACHE
            .lock()
            .map_err(|e| format!("Failed to lock cache: {}", e))?;
        cache.insert(key, details.clone());
    }

    // Cache to disk
    if let Err(e) = save_issue_details_cache(&details) {
        error!("Failed to save issue details to disk cache: {}", e);
    }

    Ok(details)
}

/// Get cached issue details from disk (for app startup / offline access)
#[tauri::command]
fn get_cached_issue(key: String) -> Result<Option<IssueDetails>, String> {
    // Check memory cache first
    {
        let cache = ISSUE_CACHE
            .lock()
            .map_err(|e| format!("Failed to lock cache: {}", e))?;
        if let Some(cached) = cache.get(&key) {
            return Ok(Some(cached.clone()));
        }
    }

    // Check disk cache
    match load_issue_details_cache(&key)? {
        Some(disk_cache) => {
            // Also populate memory cache
            let mut cache = ISSUE_CACHE
                .lock()
                .map_err(|e| format!("Failed to lock cache: {}", e))?;
            cache.insert(key, disk_cache.details.clone());
            Ok(Some(disk_cache.details))
        },
        None => Ok(None),
    }
}

// ============ REST API Info ============

/// API connection info returned to the UI
#[derive(Clone, Serialize)]
pub struct ApiInfo {
    pub base_url: String,
    pub token: String,
}

/// Global API info storage
static REST_API_INFO: Lazy<Mutex<Option<ApiInfo>>> = Lazy::new(|| Mutex::new(None));

/// Global AppState storage (for access logs)
static APP_STATE: Lazy<Mutex<Option<Arc<AppState>>>> = Lazy::new(|| Mutex::new(None));

/// Tauri command: Get REST API connection info
#[tauri::command]
fn get_api_info() -> Result<ApiInfo, String> {
    let info = REST_API_INFO
        .lock()
        .map_err(|e| format!("Failed to lock API info: {}", e))?;
    info.clone().ok_or_else(|| "REST API not started".to_string())
}

/// Tauri command: Get access logs
#[tauri::command]
fn get_access_logs() -> Result<Vec<AccessLogEntry>, String> {
    let state = APP_STATE
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    match state.as_ref() {
        Some(app_state) => Ok(app_state.get_access_logs()),
        None => Ok(Vec::new()),
    }
}

/// Tauri command: Clear access logs
#[tauri::command]
fn clear_access_logs() -> Result<(), String> {
    let state = APP_STATE
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    if let Some(app_state) = state.as_ref() {
        app_state.clear_access_logs();
    }
    Ok(())
}

/// Tauri command: Get inference logs
#[tauri::command]
fn get_inference_logs() -> Result<Vec<InferenceLogEntry>, String> {
    let state = APP_STATE
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    match state.as_ref() {
        Some(app_state) => Ok(app_state.get_inference_logs()),
        None => Ok(Vec::new()),
    }
}

/// Tauri command: Clear inference logs
#[tauri::command]
fn clear_inference_logs() -> Result<(), String> {
    let state = APP_STATE
        .lock()
        .map_err(|e| format!("Failed to lock app state: {}", e))?;
    if let Some(app_state) = state.as_ref() {
        app_state.clear_inference_logs();
    }
    Ok(())
}

/// Generate a secure random auth token
fn generate_auth_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

/// Save API info to the .env file for CLI scripts to read
fn save_api_info_to_file(api_info: &ApiInfo) -> Result<(), String> {
    // Get the project root directory (where .env file is located)
    // The Tauri dev server runs from src-tauri, so we need to go up one level
    // Priority: project root .env > current dir .env
    let project_root_env = std::path::PathBuf::from("../.env");
    let current_env = std::path::PathBuf::from(".env");
    
    // Prefer project root .env if it exists, otherwise use current dir
    let env_path = if project_root_env.exists() {
        project_root_env
    } else if current_env.exists() {
        // Check if this is src-tauri/.env (we don't want that)
        // If we're in src-tauri, use project root anyway
        let cwd = std::env::current_dir().unwrap_or_default();
        if cwd.ends_with("src-tauri") || cwd.to_string_lossy().contains("src-tauri") {
            project_root_env
        } else {
            current_env
        }
    } else {
        // Create in project root
        project_root_env
    };
    
    info!("Using .env file at: {:?}", env_path);
    
    // Read existing .env content
    let existing_content = if env_path.exists() {
        fs::read_to_string(&env_path)
            .map_err(|e| format!("Failed to read .env file: {}", e))?
    } else {
        String::new()
    };
    
    // Remove existing REST_API_* lines
    let filtered_lines: Vec<&str> = existing_content
        .lines()
        .filter(|line| !line.starts_with("REST_API_URL=") && !line.starts_with("REST_API_TOKEN="))
        .collect();
    
    // Build new content
    let mut new_content = filtered_lines.join("\n");
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    
    // Add REST API info
    new_content.push_str(&format!("\n# REST API Info (auto-generated on app start)\n"));
    new_content.push_str(&format!("REST_API_URL={}\n", api_info.base_url));
    new_content.push_str(&format!("REST_API_TOKEN={}\n", api_info.token));
    
    // Write back to .env file
    fs::write(&env_path, &new_content)
        .map_err(|e| format!("Failed to write .env file: {}", e))?;
    
    info!("Saved REST API info to .env file");
    info!("  REST_API_URL={}", api_info.base_url);
    info!("  REST_API_TOKEN={}...", &api_info.token[..8]);
    Ok(())
}

/// Start the Axum REST server
/// SECURITY: Always binds to 127.0.0.1, never 0.0.0.0
fn start_rest_server(app_state: Arc<AppState>) -> Result<String, String> {
    use tokio::net::TcpListener;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let (actual_addr, server_future) = runtime.block_on(async {
        // SECURITY: Bind to loopback only!
        let listener = TcpListener::bind("127.0.0.1:0").await
            .map_err(|e| format!("Failed to bind TCP listener: {}", e))?;
        let actual_addr = listener.local_addr()
            .map_err(|e| format!("Failed to get local address: {}", e))?;

        let app = server::create_router(app_state);

        let server = axum::serve(listener, app);

        Ok::<_, String>((actual_addr, server))
    })?;

    // Run server in background thread
    std::thread::spawn(move || {
        runtime.block_on(async {
            if let Err(e) = server_future.await {
                error!("REST server error: {}", e);
            }
        });
    });

    let base_url = format!("http://{}", actual_addr);
    Ok(base_url)
}

// ============ Main Entry Point ============

fn main() {
    // Initialize logging FIRST before anything else
    logging::init_logging();
    info!("Tauri application starting...");

    // Load .env file from project root
    // Try multiple paths since we might be running from different directories
    let env_paths = vec![
        std::path::PathBuf::from(".env"),           // Current dir
        std::path::PathBuf::from("../.env"),        // Parent dir (when running from src-tauri)
    ];
    
    for env_path in env_paths {
        if env_path.exists() {
            match dotenvy::from_path(&env_path) {
                Ok(_) => {
                    info!("Loaded .env file from: {:?}", env_path);
                    break;
                }
                Err(e) => {
                    info!("Failed to load .env from {:?}: {}", env_path, e);
                }
            }
        }
    }

    // Get Jira config for REST server
    let jira_settings = SETTINGS.lock().unwrap().clone().unwrap();
    let jira_token = API_TOKEN.lock().unwrap().clone().unwrap();

    // Get Gemini API key from environment (now loaded from .env)
    let gemini_api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        info!("GEMINI_API_KEY not set in environment");
        "YOUR_GEMINI_API_KEY_HERE".to_string()
    });
    if gemini_api_key != "YOUR_GEMINI_API_KEY_HERE" {
        info!("Gemini API key configured ({}...)", &gemini_api_key[..8.min(gemini_api_key.len())]);
    }

    // Generate random auth token for this session
    let rest_auth_token = generate_auth_token();
    info!("Generated REST API auth token");

    // Create app state for REST server
    let app_state = AppState::new(
        rest_auth_token.clone(),
        jira_settings.base_url,
        jira_settings.email,
        jira_token,
        gemini_api_key,
    );

    // Store app_state globally for Tauri commands to access
    *APP_STATE.lock().unwrap() = Some(app_state.clone());

    // Start REST server
    match start_rest_server(app_state) {
        Ok(base_url) => {
            info!("REST API server started at {}", base_url);
            
            // Store API info for UI access
            let api_info = ApiInfo {
                base_url: base_url.clone(),
                token: rest_auth_token,
            };
            *REST_API_INFO.lock().unwrap() = Some(api_info.clone());
            
            // Save API info to file for CLI scripts
            if let Err(e) = save_api_info_to_file(&api_info) {
                error!("Failed to save API info to file: {}", e);
            }
            
            info!("REST API endpoints:");
            info!("  GET {}/health - Health check (no auth)", base_url);
            info!("  GET {}/openapi.json - OpenAPI spec (no auth)", base_url);
            info!("  GET {}/jira/list - List Jira issues (requires Bearer token)", base_url);
        }
        Err(e) => {
            error!("Failed to start REST API server: {}", e);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            is_configured,
            get_settings,
            save_settings,
            get_current_user,
            list_issues,
            get_cached_issues,
            get_issue,
            get_cached_issue,
            get_api_info,
            get_access_logs,
            clear_access_logs,
            get_inference_logs,
            clear_inference_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

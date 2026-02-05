//! Gemini provider module
//!
//! This module implements the Google Gemini provider for the agent system.
//! All Gemini-specific code (client, provider, models) is contained here.
//!
//! Model list is fetched from Google API on startup and cached to disk.

pub mod client;
pub mod provider;

pub use client::GeminiClient;
pub use provider::GeminiProvider;

use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Cache TTL for model list (24 hours)
const MODEL_CACHE_TTL_HOURS: i64 = 24;

/// Static storage for Gemini provider config (base config)
static GEMINI_CONFIG: OnceLock<ProviderConfig> = OnceLock::new();

/// Dynamic model cache (fetched from API)
static MODEL_CACHE: OnceLock<RwLock<ModelCache>> = OnceLock::new();

/// Path to the cache directory (set on startup)
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Cached model list with expiration
#[derive(Debug, Serialize, Deserialize)]
struct ModelCache {
    models: Vec<ModelDef>,
    fetched_at: Option<DateTime<Utc>>,
    #[serde(skip)]
    fetch_error: Option<String>,
}

impl ModelCache {
    fn new() -> Self {
        Self {
            models: Vec::new(),
            fetched_at: None,
            fetch_error: None,
        }
    }
    
    fn is_expired(&self) -> bool {
        match self.fetched_at {
            Some(t) => Utc::now() - t > Duration::hours(MODEL_CACHE_TTL_HOURS),
            None => true,
        }
    }
}

/// Provider configuration loaded from models.json
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "google")
    pub provider: String,
    
    /// Environment variable key for API key
    pub env_key: String,
    
    /// Default model for this provider
    pub default_model: String,
    
    /// List of available models (fallback/static)
    pub models: Vec<ModelDef>,
}

/// Model definition for this provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDef {
    /// Unique model ID
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Context window size
    #[serde(default = "default_context_window")]
    pub context_window: u32,
    
    /// Whether this model is recommended
    #[serde(default)]
    pub recommended: bool,
    
    /// Model description
    #[serde(default)]
    pub description: String,
}

fn default_context_window() -> u32 {
    1_000_000
}

/// Initialize cache directory (call on startup)
pub fn init_cache_dir(app_data_dir: &std::path::Path) {
    let cache_dir = app_data_dir.join("gemini_cache");
    if let Err(e) = std::fs::create_dir_all(&cache_dir) {
        tracing::warn!("Failed to create cache directory: {}", e);
    }
    let _ = CACHE_DIR.set(cache_dir);
}

/// Get the cache file path
fn get_cache_file_path() -> Option<PathBuf> {
    CACHE_DIR.get().map(|dir| dir.join("models.json"))
}

/// Load cache from disk
fn load_cache_from_disk() -> Option<ModelCache> {
    let path = get_cache_file_path()?;
    if !path.exists() {
        return None;
    }
    
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save cache to disk
fn save_cache_to_disk(cache: &ModelCache) {
    if let Some(path) = get_cache_file_path() {
        // Create wrapper structure matching the expected format
        #[derive(Serialize)]
        struct CacheFile {
            provider: String,
            env_key: String,
            default_model: String,
            fetched_at: Option<DateTime<Utc>>,
            models: Vec<ModelDef>,
        }
        
        let config = get_config();
        let cache_file = CacheFile {
            provider: config.provider.clone(),
            env_key: config.env_key.clone(),
            default_model: config.default_model.clone(),
            fetched_at: cache.fetched_at,
            models: cache.models.clone(),
        };
        
        match serde_json::to_string_pretty(&cache_file) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    tracing::warn!("Failed to write cache file: {}", e);
                } else {
                    tracing::debug!("Saved {} models to cache: {:?}", cache.models.len(), path);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to serialize cache: {}", e);
            }
        }
    }
}

/// Load Gemini provider config (hardcoded, no models - models come from API cache)
fn load_config() -> ProviderConfig {
    ProviderConfig {
        provider: "google".to_string(),
        env_key: "GEMINI_API_KEY".to_string(),
        default_model: "gemini-2.5-flash".to_string(),
        models: vec![], // Models loaded from disk cache only
    }
}

/// Get the Gemini provider configuration (base config)
pub fn get_config() -> &'static ProviderConfig {
    GEMINI_CONFIG.get_or_init(load_config)
}

/// Get the model cache (initialize if needed)
fn get_cache() -> &'static RwLock<ModelCache> {
    MODEL_CACHE.get_or_init(|| {
        // Try to load from disk first
        if let Some(disk_cache) = load_cache_from_disk() {
            tracing::info!("Loaded {} models from disk cache", disk_cache.models.len());
            RwLock::new(disk_cache)
        } else {
            RwLock::new(ModelCache::new())
        }
    })
}

/// Check if Gemini is configured (API key is set)
pub fn is_configured() -> bool {
    let config = get_config();
    std::env::var(&config.env_key).is_ok()
}

/// Get the API key for Gemini (if configured)
pub fn get_api_key() -> Option<String> {
    let config = get_config();
    std::env::var(&config.env_key).ok()
}

/// Get models - returns cached models or fallback from static config
pub fn get_models() -> Vec<ModelDef> {
    let cache = get_cache().read();
    
    if !cache.models.is_empty() {
        return cache.models.clone();
    }
    
    // Fall back to static config if cache is empty
    get_config().models.clone()
}

/// Check if this provider handles a given model ID
pub fn handles_model(model_id: &str) -> bool {
    // Accept any model that looks like a Gemini/Google model
    let gemini_prefixes = ["gemini-", "gemma-", "models/gemini-", "models/gemma-"];
    if gemini_prefixes.iter().any(|p| model_id.starts_with(p)) {
        return true;
    }
    
    // Also check dynamic cache
    let cache = get_cache().read();
    if !cache.models.is_empty() && cache.models.iter().any(|m| m.id == model_id) {
        return true;
    }
    
    // Fall back to static config
    get_config().models.iter().any(|m| m.id == model_id)
}

/// Refresh the model cache from Google API
pub async fn refresh_models() -> Result<Vec<ModelDef>, String> {
    let api_key = match get_api_key() {
        Some(key) => key,
        None => return Err("GEMINI_API_KEY not configured".to_string()),
    };
    
    tracing::info!("Fetching Gemini models from API...");
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    
    let response = client.get(&url).send().await
        .map_err(|e| format!("API request failed: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error HTTP {}: {}", status, body));
    }
    
    let body = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    // Parse API response
    #[derive(Deserialize)]
    struct ApiResponse {
        models: Vec<ApiModel>,
    }
    
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApiModel {
        name: String,
        #[serde(default)]
        display_name: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        input_token_limit: Option<u32>,
        #[serde(default)]
        supported_generation_methods: Vec<String>,
    }
    
    let api_response: ApiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse API response: {}", e))?;
    
    // Filter and convert models
    let models: Vec<ModelDef> = api_response.models
        .into_iter()
        .filter(|m| m.supported_generation_methods.contains(&"generateContent".to_string()))
        .map(|m| {
            // Extract model ID from name (e.g., "models/gemini-2.5-flash" -> "gemini-2.5-flash")
            let id = m.name.strip_prefix("models/").unwrap_or(&m.name).to_string();
            let is_recommended = id.contains("2.5-flash") || id.contains("2.0-flash");
            
            ModelDef {
                id: id.clone(),
                name: m.display_name.unwrap_or_else(|| id.clone()),
                context_window: m.input_token_limit.unwrap_or(1_000_000),
                recommended: is_recommended,
                description: m.description.unwrap_or_default(),
            }
        })
        .collect();
    
    tracing::info!("Fetched {} Gemini models from API", models.len());
    
    // Update memory cache and save to disk
    {
        let mut cache = get_cache().write();
        cache.models = models.clone();
        cache.fetched_at = Some(Utc::now());
        cache.fetch_error = None;
        
        // Save to disk
        save_cache_to_disk(&cache);
    }
    
    Ok(models)
}

/// Get models, refreshing from API if cache is expired
pub async fn get_models_cached() -> Vec<ModelDef> {
    let cache = get_cache().read();
    
    if !cache.is_expired() && !cache.models.is_empty() {
        return cache.models.clone();
    }
    drop(cache); // Release read lock
    
    // Try to refresh from API
    match refresh_models().await {
        Ok(models) => models,
        Err(e) => {
            tracing::warn!("Failed to refresh models from API: {}", e);
            // Store error in cache
            {
                let mut cache = get_cache().write();
                cache.fetch_error = Some(e);
            }
            // Return static fallback
            get_config().models.clone()
        }
    }
}

/// Check if the model cache needs refresh
pub fn cache_needs_refresh() -> bool {
    get_cache().read().is_expired()
}

/// Get the last fetch time
pub fn get_cache_fetch_time() -> Option<DateTime<Utc>> {
    get_cache().read().fetched_at
}

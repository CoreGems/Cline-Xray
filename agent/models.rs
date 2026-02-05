//! Model registry - aggregates models from all providers
//!
//! This module collects model information from each provider module.
//! Each provider (gemini, openai, etc.) can have dynamic models fetched from API.
//! This module aggregates them into a unified API response.

use serde::Serialize;

use crate::agent::gemini;

/// Information about an available LLM model (for API response)
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ModelInfo {
    /// Unique model identifier (e.g., "gemini-2.5-flash")
    pub id: String,
    
    /// Human-readable model name (e.g., "Gemini 2.5 Flash")
    pub name: String,
    
    /// Provider name (e.g., "google", "openai")
    pub provider: String,
    
    /// Context window size (token limit)
    pub context_window: u32,
    
    /// Whether this model is recommended for general use
    pub recommended: bool,
    
    /// Model description/notes
    pub description: String,
    
    /// Whether this model is currently available (API key configured)
    pub available: bool,
}

/// Response for listing available models
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ModelsResponse {
    /// List of available models
    pub models: Vec<ModelInfo>,
    
    /// Total count
    pub count: usize,
    
    /// Default model ID
    pub default_model: String,
    
    /// Whether cache was refreshed from API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_refreshed: Option<bool>,
}

/// Get all models from all providers (sync, uses cached data)
pub fn get_available_models() -> Vec<ModelInfo> {
    let mut models = Vec::new();
    
    // ========================================
    // Gemini provider models (from cache or static)
    // ========================================
    let gemini_config = gemini::get_config();
    let gemini_available = gemini::is_configured();
    let gemini_models = gemini::get_models();
    
    for model in gemini_models {
        models.push(ModelInfo {
            id: model.id.clone(),
            name: model.name.clone(),
            provider: gemini_config.provider.clone(),
            context_window: model.context_window,
            recommended: model.recommended,
            description: model.description.clone(),
            available: gemini_available,
        });
    }
    
    // ========================================
    // OpenAI provider models (when implemented)
    // ========================================
    // let openai_config = openai::get_config();
    // let openai_available = openai::is_configured();
    // for model in &openai_config.models {
    //     models.push(ModelInfo { ... });
    // }
    
    // ========================================
    // Add more providers here...
    // ========================================
    
    models
}

/// Get all models from all providers (async, refreshes cache if needed)
pub async fn get_available_models_async() -> (Vec<ModelInfo>, bool) {
    let mut models = Vec::new();
    let mut refreshed = false;
    
    // ========================================
    // Gemini provider models (refresh if needed)
    // ========================================
    let gemini_config = gemini::get_config();
    let gemini_available = gemini::is_configured();
    
    // Check if cache needs refresh
    if gemini::cache_needs_refresh() && gemini_available {
        tracing::debug!("Model cache expired, refreshing from API...");
        refreshed = true;
    }
    
    // Get models (will fetch from API if configured and cache expired)
    let gemini_models = gemini::get_models_cached().await;
    
    for model in gemini_models {
        models.push(ModelInfo {
            id: model.id.clone(),
            name: model.name.clone(),
            provider: gemini_config.provider.clone(),
            context_window: model.context_window,
            recommended: model.recommended,
            description: model.description.clone(),
            available: gemini_available,
        });
    }
    
    // ========================================
    // OpenAI provider models (when implemented)
    // ========================================
    
    (models, refreshed)
}

/// Force refresh model cache for all providers
pub async fn refresh_all_models() -> Result<(), String> {
    // Refresh Gemini models
    if gemini::is_configured() {
        gemini::refresh_models().await?;
    }
    
    // Refresh other providers when implemented
    // if openai::is_configured() {
    //     openai::refresh_models().await?;
    // }
    
    Ok(())
}

/// Get the default model ID
pub fn get_default_model() -> String {
    // Use Gemini's default if configured, otherwise first available
    if gemini::is_configured() {
        return gemini::get_config().default_model.clone();
    }
    
    // Fallback to first model from first configured provider
    let models = get_available_models();
    models
        .into_iter()
        .find(|m| m.available)
        .map(|m| m.id)
        .unwrap_or_else(|| "gemini-2.5-flash".to_string())
}

/// Get the provider name for a model ID
pub fn get_provider_for_model(model_id: &str) -> Option<&'static str> {
    if gemini::handles_model(model_id) {
        return Some("google");
    }
    // if openai::handles_model(model_id) {
    //     return Some("openai");
    // }
    None
}

/// Check if a model ID is valid (exists in any provider)
pub fn is_valid_model(model_id: &str) -> bool {
    gemini::handles_model(model_id)
    // || openai::handles_model(model_id)
}

/// Check if any provider is configured
pub fn any_provider_configured() -> bool {
    gemini::is_configured()
    // || openai::is_configured()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_models() {
        let models = get_available_models();
        
        // Should have Gemini models (at least fallback)
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.provider == "google"));
    }

    #[test]
    fn test_get_default_model() {
        let default = get_default_model();
        // Should return a valid model ID
        assert!(!default.is_empty());
    }

    #[test]
    fn test_get_provider_for_model() {
        // Static fallback models should work
        let models = get_available_models();
        if let Some(model) = models.first() {
            assert_eq!(get_provider_for_model(&model.id), Some("google"));
        }
        assert_eq!(get_provider_for_model("unknown-model-xyz"), None);
    }
}

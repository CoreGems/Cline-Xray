//! Gemini API client
//!
//! This module implements the HTTP client for Google's Gemini API.
//! It handles request/response serialization and error handling.

use crate::agent::types::{AgentError, ToolCall, TokenUsage};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Gemini API client
pub struct GeminiClient {
    client: Client,
    api_key: String,
    timeout: Duration,
}

impl GeminiClient {
    /// Create a new Gemini client
    pub fn new(api_key: &str, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            api_key: api_key.to_string(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }
    
    /// Generate content with function calling support
    pub async fn generate_content(
        &self,
        model: &str,
        contents: Vec<GeminiContent>,
        tools: Option<Vec<GeminiTool>>,
        system_instruction: Option<&str>,
    ) -> Result<GeminiResponse, AgentError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );
        
        let mut request = GeminiRequest {
            contents,
            tools,
            tool_config: None,
            system_instruction: system_instruction.map(|s| GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart::Text { text: s.to_string() }],
            }),
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(0.1), // Low temperature for consistent tool use
                max_output_tokens: Some(8192),
                ..Default::default()
            }),
        };
        
        // Enable automatic function calling mode
        if request.tools.is_some() {
            request.tool_config = Some(GeminiToolConfig {
                function_calling_config: GeminiFunctionCallingConfig {
                    mode: "AUTO".to_string(),
                },
            });
        }
        
        tracing::debug!("Sending request to Gemini API: {}", model);
        
        let response = self.client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AgentError::Timeout
                } else {
                    AgentError::ApiError(format!("HTTP error: {}", e))
                }
            })?;
        
        let status = response.status();
        let body = response.text().await
            .map_err(|e| AgentError::ApiError(format!("Failed to read response: {}", e)))?;
        
        if !status.is_success() {
            // Parse error response
            if let Ok(error) = serde_json::from_str::<GeminiErrorResponse>(&body) {
                let message = error.error.message;
                if message.contains("429") || message.contains("quota") || message.contains("rate") {
                    return Err(AgentError::RateLimited);
                }
                return Err(AgentError::ApiError(message));
            }
            return Err(AgentError::ApiError(format!("HTTP {}: {}", status, body)));
        }
        
        // Parse success response
        let response: GeminiResponse = serde_json::from_str(&body)
            .map_err(|e| AgentError::ApiError(format!("Failed to parse response: {} - Body: {}", e, body)))?;
        
        Ok(response)
    }
}

// ============================================================================
// Gemini API Types
// ============================================================================

/// Gemini API request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<GeminiToolConfig>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

/// Gemini content (message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

/// Gemini content part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeminiPart {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponse,
    },
}

/// Gemini function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

/// Gemini function response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

/// Gemini tool definition
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiTool {
    pub function_declarations: Vec<serde_json::Value>,
}

/// Gemini tool config
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiToolConfig {
    pub function_calling_config: GeminiFunctionCallingConfig,
}

/// Gemini function calling config
#[derive(Debug, Serialize)]
pub struct GeminiFunctionCallingConfig {
    pub mode: String, // "AUTO", "ANY", "NONE"
}

/// Gemini generation config
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Gemini API response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
    
    #[serde(default)]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

/// Gemini candidate response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Gemini usage metadata
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: Option<u32>,
    pub candidates_token_count: Option<u32>,
    pub total_token_count: Option<u32>,
}

/// Gemini error response
#[derive(Debug, Deserialize)]
pub struct GeminiErrorResponse {
    pub error: GeminiError,
}

/// Gemini error detail
#[derive(Debug, Deserialize)]
pub struct GeminiError {
    pub code: i32,
    pub message: String,
    pub status: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

impl GeminiResponse {
    /// Extract text content from the response
    pub fn get_text(&self) -> Option<String> {
        self.candidates.first()
            .and_then(|c| c.content.parts.iter()
                .find_map(|p| match p {
                    GeminiPart::Text { text } => Some(text.clone()),
                    _ => None,
                }))
    }
    
    /// Extract function calls from the response
    pub fn get_function_calls(&self) -> Vec<ToolCall> {
        let mut calls = Vec::new();
        
        if let Some(candidate) = self.candidates.first() {
            for part in &candidate.content.parts {
                if let GeminiPart::FunctionCall { function_call } = part {
                    // Convert args from Value to HashMap
                    let arguments: HashMap<String, serde_json::Value> = match &function_call.args {
                        serde_json::Value::Object(map) => {
                            map.iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect()
                        }
                        _ => HashMap::new(),
                    };
                    
                    calls.push(ToolCall {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: function_call.name.clone(),
                        arguments,
                    });
                }
            }
        }
        
        calls
    }
    
    /// Check if the response contains function calls
    pub fn has_function_calls(&self) -> bool {
        self.candidates.first()
            .map(|c| c.content.parts.iter()
                .any(|p| matches!(p, GeminiPart::FunctionCall { .. })))
            .unwrap_or(false)
    }
    
    /// Get token usage
    pub fn get_token_usage(&self) -> Option<TokenUsage> {
        self.usage_metadata.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.prompt_token_count.unwrap_or(0),
            completion_tokens: u.candidates_token_count.unwrap_or(0),
            total_tokens: u.total_token_count.unwrap_or(0),
        })
    }
}

impl GeminiContent {
    /// Create a user message
    pub fn user(text: &str) -> Self {
        Self {
            role: "user".to_string(),
            parts: vec![GeminiPart::Text { text: text.to_string() }],
        }
    }
    
    /// Create a model message
    pub fn model(text: &str) -> Self {
        Self {
            role: "model".to_string(),
            parts: vec![GeminiPart::Text { text: text.to_string() }],
        }
    }
    
    /// Create a model message with function calls
    pub fn model_with_function_calls(calls: Vec<GeminiFunctionCall>) -> Self {
        Self {
            role: "model".to_string(),
            parts: calls.into_iter()
                .map(|fc| GeminiPart::FunctionCall { function_call: fc })
                .collect(),
        }
    }
    
    /// Create a user message with function responses
    pub fn user_with_function_responses(responses: Vec<GeminiFunctionResponse>) -> Self {
        Self {
            role: "user".to_string(),
            parts: responses.into_iter()
                .map(|fr| GeminiPart::FunctionResponse { function_response: fr })
                .collect(),
        }
    }
}

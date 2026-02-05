//! Agent types shared across all LLM providers
//!
//! These types are provider-agnostic and used by the tool executor,
//! agent handlers, and individual provider implementations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request to the agent
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct AgentRequest {
    /// The natural language question to answer
    pub question: String,
    
    /// Unique chat ID for grouping related conversations
    #[serde(default)]
    pub chat_id: Option<String>,
    
    /// Agent ID (for future multi-agent support)
    #[serde(default)]
    pub agent_id: Option<String>,
    
    /// Model ID to use (e.g., "gemini-1.5-pro", "gemini-1.5-flash")
    #[serde(default)]
    pub model_id: Option<String>,
    
    /// Session ID for conversation continuity
    #[serde(default)]
    pub session_id: Option<String>,
    
    /// Optional schema filter to focus queries
    #[serde(default)]
    pub schema_filter: Option<String>,
    
    /// Connection ID to use for database operations
    #[serde(default)]
    pub connection_id: Option<String>,
}

/// Response from the agent
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgentResponse {
    /// The final answer from the agent
    pub answer: String,
    
    /// Sources cited in the answer
    pub sources: Vec<SourceCitation>,
    
    /// Trace of tool calls made during reasoning
    pub trace: Vec<TraceEntry>,
    
    /// Session ID for continuing the conversation
    pub session_id: String,
    
    /// Token usage statistics (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<TokenUsage>,
    
    /// Model used for this response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

/// Citation for a source used in the answer
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourceCitation {
    /// Type of source (e.g., "table", "query_result", "schema")
    pub source_type: String,
    
    /// Human-readable description of the source
    pub description: String,
    
    /// Optional: specific object reference (e.g., "HR.EMPLOYEES")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// Entry in the agent's reasoning trace
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TraceEntry {
    /// Step number in the reasoning process
    pub step: u32,
    
    /// Type of action: "thought", "tool_call", "tool_result", "answer"
    pub action_type: String,
    
    /// Content of this trace entry
    pub content: String,
    
    /// Timestamp of this entry (ISO 8601 format)
    #[schema(value_type = String, example = "2024-01-15T10:30:00Z")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Tool name (if action_type is "tool_call" or "tool_result")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    
    /// Duration in milliseconds (for tool calls)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TokenUsage {
    /// Tokens used in the prompt
    pub prompt_tokens: u32,
    
    /// Tokens used in the completion
    pub completion_tokens: u32,
    
    /// Total tokens used
    pub total_tokens: u32,
}

/// Tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    
    /// Name of the tool to call
    pub name: String,
    
    /// Arguments as key-value pairs
    pub arguments: HashMap<String, serde_json::Value>,
}

impl ToolCall {
    /// Get a required string parameter
    pub fn get_param(&self, name: &str) -> Result<String, ToolError> {
        self.arguments
            .get(name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ToolError::MissingParameter(name.to_string()))
    }
    
    /// Get an optional string parameter with default
    pub fn get_param_or(&self, name: &str, default: &str) -> String {
        self.arguments
            .get(name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| default.to_string())
    }
    
    /// Get an optional integer parameter with default
    pub fn get_int_param_or(&self, name: &str, default: u32) -> u32 {
        self.arguments
            .get(name)
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
            .unwrap_or(default)
    }
}

/// Result from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// ID of the tool call this is responding to
    pub tool_call_id: String,
    
    /// Whether the tool executed successfully
    pub success: bool,
    
    /// Result data (JSON)
    pub data: serde_json::Value,
    
    /// Error message if success is false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result with JSON data
    pub fn json<T: Serialize>(tool_call_id: &str, data: T) -> Self {
        Self {
            tool_call_id: tool_call_id.to_string(),
            success: true,
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
            error: None,
        }
    }
    
    /// Create an error result
    pub fn error(tool_call_id: &str, message: &str) -> Self {
        Self {
            tool_call_id: tool_call_id.to_string(),
            success: false,
            data: serde_json::Value::Null,
            error: Some(message.to_string()),
        }
    }
}

/// Tool execution error
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
    
    #[error("Invalid parameter value: {0}")]
    InvalidParameter(String),
    
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("No active connection")]
    NoConnection,
}

/// Agent execution error
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Provider error: {0}")]
    ProviderError(String),
    
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Rate limit exceeded")]
    RateLimited,
    
    #[error("Tool execution failed: {0}")]
    ToolError(#[from] ToolError),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Max iterations exceeded")]
    MaxIterationsExceeded,
}

/// Configuration for the agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Gemini API key
    pub gemini_api_key: Option<String>,
    
    /// OpenAI API key (for future use)
    pub openai_api_key: Option<String>,
    
    /// Default model to use
    pub default_model: String,
    
    /// Maximum iterations for the agent loop
    pub max_iterations: u32,
    
    /// Timeout for LLM API calls (in seconds)
    pub api_timeout_secs: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            default_model: std::env::var("DEFAULT_MODEL")
                .unwrap_or_else(|_| "gemini-1.5-flash".to_string()),
            max_iterations: 10,
            api_timeout_secs: 60,
        }
    }
}

impl AgentConfig {
    /// Check if Gemini is configured
    pub fn has_gemini(&self) -> bool {
        self.gemini_api_key.is_some()
    }
    
    /// Check if OpenAI is configured
    pub fn has_openai(&self) -> bool {
        self.openai_api_key.is_some()
    }
    
    /// Check if any provider is configured
    pub fn is_configured(&self) -> bool {
        self.has_gemini() || self.has_openai()
    }
}

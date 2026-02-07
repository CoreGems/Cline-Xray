//! Type definitions for ToolRuntime
//!
//! Core types used throughout the tool runtime system.

use serde::{Deserialize, Serialize};

/// Source of the tool call
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallSource {
    /// Call from the AI agent during function calling
    Agent,
    /// Call from the UI developer console
    UiConsole,
    /// Call from CLI/script testing
    CliTest,
    /// Call from automated tests
    AutomatedTest,
    /// Unknown/unspecified source
    Unknown,
}

impl Default for ToolCallSource {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Result of a tool call
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// Whether the call succeeded
    pub success: bool,
    /// Response data (if successful)
    pub data: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Whether this was a dry-run (no actual execution)
    pub dry_run: bool,
    /// Whether the response came from a fixture
    pub from_fixture: bool,
    /// Validation result (if validation was enabled)
    pub validation: Option<ValidationResult>,
}

impl ToolCallResult {
    /// Create a successful result
    pub fn success(data: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            duration_ms,
            dry_run: false,
            from_fixture: false,
            validation: None,
        }
    }

    /// Create an error result
    pub fn error(message: &str, duration_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.to_string()),
            duration_ms,
            dry_run: false,
            from_fixture: false,
            validation: None,
        }
    }
}

/// Validation result from OpenAPI schema validation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// Overall validity
    pub valid: bool,
    /// List of validation errors
    pub errors: Vec<String>,
    /// List of validation warnings (non-fatal)
    pub warnings: Vec<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl ValidationResult {
    /// Create a valid result
    pub fn valid() -> Self {
        Self::default()
    }

    /// Create an invalid result with errors
    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.valid = false;
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Error types for ToolRuntime
#[derive(Debug, thiserror::Error)]
pub enum ToolRuntimeError {
    #[error("Tool '{0}' is disabled")]
    ToolDisabled(String),

    #[error("Tool '{0}' not found")]
    ToolNotFound(String),

    #[error("Circuit breaker open for tool '{0}'")]
    CircuitBreakerOpen(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Request to invoke a tool
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvokeRequest {
    /// The operation ID (e.g., "get_jira_list")
    pub operation_id: String,
    /// Arguments as JSON object
    #[serde(default)]
    pub args: serde_json::Value,
    /// Source of the call (defaults to UiConsole)
    #[serde(default)]
    pub source: Option<ToolCallSource>,
    /// Override dry-run setting for this call
    #[serde(default)]
    pub dry_run: Option<bool>,
    /// Override fixture usage for this call
    #[serde(default)]
    pub use_fixture: Option<bool>,
}

/// Response from tool invocation
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvokeResponse {
    /// The operation ID that was invoked
    pub operation_id: String,
    /// The result of the call
    #[serde(flatten)]
    pub result: ToolCallResult,
}

/// Request to configure a tool
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfigRequest {
    /// The operation ID to configure
    pub operation_id: String,
    /// Configuration to apply
    pub config: super::ToolConfig,
}

/// Response listing all tools
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResponse {
    /// List of available tools
    pub tools: Vec<super::ToolInfo>,
    /// Total count
    pub total: usize,
}

/// Response for tool execution logs
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionLogsResponse {
    /// List of log entries
    pub logs: Vec<super::ToolExecutionLog>,
    /// Total count
    pub total: usize,
}

/// Response for global runtime config
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigResponse {
    /// Current global configuration
    pub config: super::GlobalRuntimeConfig,
    /// Per-tool configurations
    pub tool_configs: std::collections::HashMap<String, super::ToolConfig>,
}

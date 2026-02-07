//! HTTP handlers for ToolRuntime API
//!
//! Provides REST endpoints for the UI Tools Console.

use super::{
    GlobalRuntimeConfig, ToolCallSource, ToolConfig, ToolExecutionLog, ToolInfo, ToolInvokeRequest,
    ToolRuntime,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ============ Response Types ============

/// Response for tool invocation
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvokeResponse {
    pub operation_id: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub dry_run: bool,
    pub from_fixture: bool,
    pub validation: Option<super::ValidationResult>,
}

/// Response listing all tools
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResponse {
    pub tools: Vec<ToolInfo>,
    pub total: usize,
}

/// Response for tool execution logs
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionLogsResponse {
    pub logs: Vec<ToolExecutionLog>,
    pub total: usize,
}

/// Response for runtime configuration
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigResponse {
    pub config: GlobalRuntimeConfig,
    pub tool_configs: HashMap<String, ToolConfig>,
}

/// Response for circuit breaker status
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerStatusResponse {
    pub breakers: HashMap<String, super::CircuitBreakerState>,
    pub total: usize,
}

/// Response for fixtures
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FixturesResponse {
    pub fixtures: serde_json::Value,
    pub count: usize,
}

/// Error response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ToolErrorResponse {
    pub error: String,
    pub code: u16,
}

// ============ Request Types ============

/// Request to update global config
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGlobalConfigRequest {
    pub config: GlobalRuntimeConfig,
}

/// Request to configure a tool
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureToolRequest {
    pub config: ToolConfig,
}

// ============ Handlers ============

/// Invoke a tool
///
/// This is the main entry point for the UI Tools Console.
/// Executes a tool through the ToolRuntime choke-point.
#[utoipa::path(
    post,
    path = "/tools/invoke",
    request_body = ToolInvokeRequest,
    responses(
        (status = 200, description = "Tool invocation result", body = ToolInvokeResponse),
        (status = 400, description = "Bad request", body = ToolErrorResponse),
        (status = 500, description = "Internal error", body = ToolErrorResponse)
    ),
    tag = "tools"
)]
pub async fn invoke_tool_handler(
    State(runtime): State<Arc<ToolRuntime>>,
    Json(request): Json<ToolInvokeRequest>,
) -> Result<Json<ToolInvokeResponse>, (StatusCode, Json<ToolErrorResponse>)> {
    tracing::info!(
        "Tools Console: Invoking {} with args: {:?}",
        request.operation_id,
        request.args
    );

    let source = request.source.unwrap_or(ToolCallSource::UiConsole);
    
    // If dry_run or use_fixture overrides are specified, temporarily configure
    if request.dry_run.is_some() || request.use_fixture.is_some() {
        let mut config = runtime.get_tool_config(&request.operation_id);
        if let Some(dry_run) = request.dry_run {
            config.dry_run = dry_run;
        }
        if let Some(use_fixture) = request.use_fixture {
            config.use_fixtures = use_fixture;
        }
        runtime.configure_tool(&request.operation_id, config);
    }

    let result = runtime.call(&request.operation_id, request.args, source).await;

    Ok(Json(ToolInvokeResponse {
        operation_id: request.operation_id,
        success: result.success,
        data: result.data,
        error: result.error,
        duration_ms: result.duration_ms,
        dry_run: result.dry_run,
        from_fixture: result.from_fixture,
        validation: result.validation,
    }))
}

/// List all available tools
#[utoipa::path(
    get,
    path = "/tools",
    responses(
        (status = 200, description = "List of available tools", body = ToolsListResponse)
    ),
    tag = "tools"
)]
pub async fn list_tools_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> Json<ToolsListResponse> {
    let tools = runtime.list_tools();
    let total = tools.len();
    Json(ToolsListResponse { tools, total })
}

/// Get tool execution logs
#[utoipa::path(
    get,
    path = "/tools/logs",
    responses(
        (status = 200, description = "Tool execution logs", body = ToolExecutionLogsResponse)
    ),
    tag = "tools"
)]
pub async fn get_tool_logs_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> Json<ToolExecutionLogsResponse> {
    let logs = runtime.get_execution_logs();
    let total = logs.len();
    Json(ToolExecutionLogsResponse { logs, total })
}

/// Clear tool execution logs
#[utoipa::path(
    delete,
    path = "/tools/logs",
    responses(
        (status = 200, description = "Logs cleared")
    ),
    tag = "tools"
)]
pub async fn clear_tool_logs_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> StatusCode {
    runtime.clear_execution_logs();
    tracing::info!("Tools Console: Execution logs cleared");
    StatusCode::OK
}

/// Get runtime configuration
#[utoipa::path(
    get,
    path = "/tools/config",
    responses(
        (status = 200, description = "Runtime configuration", body = RuntimeConfigResponse)
    ),
    tag = "tools"
)]
pub async fn get_config_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> Json<RuntimeConfigResponse> {
    let config = runtime.get_global_config();
    let tool_configs = runtime.get_all_tool_configs();
    Json(RuntimeConfigResponse { config, tool_configs })
}

/// Update global runtime configuration
#[utoipa::path(
    put,
    path = "/tools/config",
    request_body = UpdateGlobalConfigRequest,
    responses(
        (status = 200, description = "Configuration updated", body = RuntimeConfigResponse)
    ),
    tag = "tools"
)]
pub async fn update_config_handler(
    State(runtime): State<Arc<ToolRuntime>>,
    Json(request): Json<UpdateGlobalConfigRequest>,
) -> Json<RuntimeConfigResponse> {
    runtime.set_global_config(request.config);
    tracing::info!("Tools Console: Global config updated");
    
    let config = runtime.get_global_config();
    let tool_configs = runtime.get_all_tool_configs();
    Json(RuntimeConfigResponse { config, tool_configs })
}

/// Configure a specific tool
#[utoipa::path(
    put,
    path = "/tools/{operation_id}/config",
    params(
        ("operation_id" = String, Path, description = "Tool operation ID")
    ),
    request_body = ConfigureToolRequest,
    responses(
        (status = 200, description = "Tool configured", body = ToolConfig)
    ),
    tag = "tools"
)]
pub async fn configure_tool_handler(
    State(runtime): State<Arc<ToolRuntime>>,
    Path(operation_id): Path<String>,
    Json(request): Json<ConfigureToolRequest>,
) -> Json<ToolConfig> {
    runtime.configure_tool(&operation_id, request.config.clone());
    tracing::info!("Tools Console: Tool {} configured", operation_id);
    Json(request.config)
}

/// Get circuit breaker status for all tools
#[utoipa::path(
    get,
    path = "/tools/circuit-breakers",
    responses(
        (status = 200, description = "Circuit breaker status", body = CircuitBreakerStatusResponse)
    ),
    tag = "tools"
)]
pub async fn get_circuit_breakers_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> Json<CircuitBreakerStatusResponse> {
    let breakers = runtime.get_circuit_breaker_status();
    let total = breakers.len();
    Json(CircuitBreakerStatusResponse { breakers, total })
}

/// Reset all circuit breakers
#[utoipa::path(
    delete,
    path = "/tools/circuit-breakers",
    responses(
        (status = 200, description = "Circuit breakers reset")
    ),
    tag = "tools"
)]
pub async fn reset_circuit_breakers_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> StatusCode {
    runtime.reset_circuit_breakers();
    tracing::info!("Tools Console: All circuit breakers reset");
    StatusCode::OK
}

/// Reset circuit breaker for a specific tool
#[utoipa::path(
    delete,
    path = "/tools/{operation_id}/circuit-breaker",
    params(
        ("operation_id" = String, Path, description = "Tool operation ID")
    ),
    responses(
        (status = 200, description = "Circuit breaker reset")
    ),
    tag = "tools"
)]
pub async fn reset_tool_circuit_breaker_handler(
    State(runtime): State<Arc<ToolRuntime>>,
    Path(operation_id): Path<String>,
) -> StatusCode {
    runtime.reset_circuit_breaker(&operation_id);
    tracing::info!("Tools Console: Circuit breaker for {} reset", operation_id);
    StatusCode::OK
}

/// Get all fixtures
#[utoipa::path(
    get,
    path = "/tools/fixtures",
    responses(
        (status = 200, description = "All fixtures", body = FixturesResponse)
    ),
    tag = "tools"
)]
pub async fn get_fixtures_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> Json<FixturesResponse> {
    let fixtures = runtime.export_fixtures();
    let count = runtime.get_fixtures().count();
    Json(FixturesResponse { fixtures, count })
}

/// Import fixtures
#[utoipa::path(
    post,
    path = "/tools/fixtures",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Fixtures imported", body = FixturesResponse)
    ),
    tag = "tools"
)]
pub async fn import_fixtures_handler(
    State(runtime): State<Arc<ToolRuntime>>,
    Json(fixtures): Json<super::FixturesStorage>,
) -> Json<FixturesResponse> {
    runtime.import_fixtures(fixtures);
    tracing::info!("Tools Console: Fixtures imported");
    
    let fixtures = runtime.export_fixtures();
    let count = runtime.get_fixtures().count();
    Json(FixturesResponse { fixtures, count })
}

/// Clear all fixtures
#[utoipa::path(
    delete,
    path = "/tools/fixtures",
    responses(
        (status = 200, description = "Fixtures cleared")
    ),
    tag = "tools"
)]
pub async fn clear_fixtures_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> StatusCode {
    runtime.clear_fixtures();
    tracing::info!("Tools Console: All fixtures cleared");
    StatusCode::OK
}

/// Enable all tools
#[utoipa::path(
    post,
    path = "/tools/enable-all",
    responses(
        (status = 200, description = "All tools enabled")
    ),
    tag = "tools"
)]
pub async fn enable_all_tools_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> StatusCode {
    runtime.enable_all_tools();
    tracing::info!("Tools Console: All tools enabled");
    StatusCode::OK
}

/// Disable all tools
#[utoipa::path(
    post,
    path = "/tools/disable-all",
    responses(
        (status = 200, description = "All tools disabled")
    ),
    tag = "tools"
)]
pub async fn disable_all_tools_handler(
    State(runtime): State<Arc<ToolRuntime>>,
) -> StatusCode {
    runtime.disable_all_tools();
    tracing::info!("Tools Console: All tools disabled");
    StatusCode::OK
}

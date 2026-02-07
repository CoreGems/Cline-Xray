//! Tool Runtime - Unified choke-point for all tool invocations
//!
//! This module provides a single execution path for all tool calls, whether from:
//! - AI Agent (function calling)
//! - UI Dev Console (manual testing)
//! - CLI scripts (automated testing)
//!
//! ## Features
//!
//! - **Enable/Disable states**: Toggle individual tools on/off
//! - **Arg clamps**: Enforce min/max values on parameters
//! - **Dry-run mode**: Return mock responses without execution
//! - **Contract validation**: Validate requests/responses against OpenAPI schema
//! - **Fixtures replay/record**: Record and replay tool responses for testing
//! - **Budgets / Circuit breaker**: Rate limiting and failure protection
//!
//! ## Usage
//!
//! ```rust,ignore
//! let runtime = ToolRuntime::new(app_state.clone());
//!
//! // Configure tool
//! runtime.configure_tool("get_jira_list", ToolConfig {
//!     enabled: true,
//!     dry_run: false,
//!     ..Default::default()
//! });
//!
//! // Execute tool
//! let result = runtime.call("get_jira_list", json!({"jql": "..."})).await;
//! ```

mod types;
mod config;
mod executor;
mod validator;
mod fixtures;
mod circuit_breaker;
pub mod handlers;

pub use types::*;
pub use config::*;
pub use executor::*;
pub use validator::*;
pub use fixtures::*;
pub use circuit_breaker::*;
pub use handlers::*;

use crate::state::AppState;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// Tool execution log entry
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionLog {
    pub id: u64,
    pub timestamp: String,
    pub operation_id: String,
    pub source: ToolCallSource,
    pub args: serde_json::Value,
    pub success: bool,
    pub response: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub dry_run: bool,
    pub from_fixture: bool,
    pub validation_result: Option<ValidationResult>,
}

/// The central ToolRuntime that mediates all tool calls
pub struct ToolRuntime {
    state: Arc<AppState>,
    /// Per-tool configuration
    tool_configs: RwLock<HashMap<String, ToolConfig>>,
    /// Global runtime settings
    global_config: RwLock<GlobalRuntimeConfig>,
    /// Execution log
    execution_log: RwLock<Vec<ToolExecutionLog>>,
    log_counter: RwLock<u64>,
    /// Circuit breaker per tool
    circuit_breakers: RwLock<HashMap<String, CircuitBreakerState>>,
    /// Fixtures storage
    fixtures: RwLock<FixturesStorage>,
    /// OpenAPI spec cache for validation
    openapi_spec: RwLock<Option<serde_json::Value>>,
}

impl ToolRuntime {
    /// Create a new ToolRuntime instance
    pub fn new(state: Arc<AppState>) -> Arc<Self> {
        Arc::new(Self {
            state,
            tool_configs: RwLock::new(HashMap::new()),
            global_config: RwLock::new(GlobalRuntimeConfig::default()),
            execution_log: RwLock::new(Vec::new()),
            log_counter: RwLock::new(0),
            circuit_breakers: RwLock::new(HashMap::new()),
            fixtures: RwLock::new(FixturesStorage::default()),
            openapi_spec: RwLock::new(None),
        })
    }

    /// Get reference to app state
    pub fn app_state(&self) -> Arc<AppState> {
        self.state.clone()
    }

    /// Initialize the OpenAPI spec for validation
    pub fn set_openapi_spec(&self, spec: serde_json::Value) {
        *self.openapi_spec.write() = Some(spec);
    }

    /// Configure a specific tool
    pub fn configure_tool(&self, operation_id: &str, config: ToolConfig) {
        self.tool_configs.write().insert(operation_id.to_string(), config);
    }

    /// Get tool configuration (with defaults)
    pub fn get_tool_config(&self, operation_id: &str) -> ToolConfig {
        self.tool_configs
            .read()
            .get(operation_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Update global configuration
    pub fn set_global_config(&self, config: GlobalRuntimeConfig) {
        *self.global_config.write() = config;
    }

    /// Get global configuration
    pub fn get_global_config(&self) -> GlobalRuntimeConfig {
        self.global_config.read().clone()
    }

    /// Get all tool configurations
    pub fn get_all_tool_configs(&self) -> HashMap<String, ToolConfig> {
        self.tool_configs.read().clone()
    }

    /// The main entry point - call a tool by operation_id
    ///
    /// This is THE choke-point. All tool invocations (agent, UI console, tests)
    /// must go through this method.
    pub async fn call(
        &self,
        operation_id: &str,
        args: serde_json::Value,
        source: ToolCallSource,
    ) -> ToolCallResult {
        let start = Instant::now();
        let tool_config = self.get_tool_config(operation_id);
        let global_config = self.get_global_config();

        // Step 1: Check if tool is enabled
        if !tool_config.enabled {
            return self.log_and_return(
                operation_id,
                source,
                &args,
                Err(ToolRuntimeError::ToolDisabled(operation_id.to_string())),
                start,
                false,
                false,
                None,
            );
        }

        // Step 2: Check circuit breaker
        if let Err(e) = self.check_circuit_breaker(operation_id) {
            return self.log_and_return(
                operation_id,
                source,
                &args,
                Err(e),
                start,
                false,
                false,
                None,
            );
        }

        // Step 3: Apply arg clamps
        let clamped_args = self.apply_arg_clamps(operation_id, args.clone(), &tool_config);

        // Step 4: Validate request against OpenAPI schema (if enabled)
        let validation_result = if global_config.validate_requests {
            Some(self.validate_request(operation_id, &clamped_args))
        } else {
            None
        };

        if let Some(ref vr) = validation_result {
            if !vr.valid && global_config.fail_on_validation_error {
                return self.log_and_return(
                    operation_id,
                    source,
                    &clamped_args,
                    Err(ToolRuntimeError::ValidationFailed(vr.errors.join(", "))),
                    start,
                    false,
                    false,
                    validation_result,
                );
            }
        }

        // Step 5: Check for fixture replay
        if tool_config.use_fixtures || global_config.replay_fixtures {
            if let Some(fixture) = self.get_fixture(operation_id, &clamped_args) {
                return self.log_and_return(
                    operation_id,
                    source,
                    &clamped_args,
                    Ok(fixture),
                    start,
                    false,
                    true,
                    validation_result,
                );
            }
        }

        // Step 6: Dry-run mode
        if tool_config.dry_run || global_config.dry_run {
            let mock_response = self.generate_dry_run_response(operation_id, &clamped_args);
            return self.log_and_return(
                operation_id,
                source,
                &clamped_args,
                Ok(mock_response),
                start,
                true,
                false,
                validation_result,
            );
        }

        // Step 7: Execute the actual tool
        let result = self.execute_tool(operation_id, &clamped_args).await;

        // Step 8: Update circuit breaker state
        self.update_circuit_breaker(operation_id, result.is_ok());

        // Step 9: Record fixture if enabled
        if tool_config.record_fixtures && result.is_ok() {
            if let Ok(ref response) = result {
                self.record_fixture(operation_id, &clamped_args, response.clone());
            }
        }

        // Step 10: Validate response against OpenAPI schema (if enabled)
        let response_validation = if global_config.validate_responses && result.is_ok() {
            if let Ok(ref response) = result {
                Some(self.validate_response(operation_id, response))
            } else {
                None
            }
        } else {
            None
        };

        // Combine validations
        let combined_validation = match (validation_result, response_validation) {
            (Some(req_val), Some(resp_val)) => Some(ValidationResult {
                valid: req_val.valid && resp_val.valid,
                errors: [req_val.errors, resp_val.errors].concat(),
                warnings: [req_val.warnings, resp_val.warnings].concat(),
            }),
            (Some(v), None) | (None, Some(v)) => Some(v),
            (None, None) => None,
        };

        self.log_and_return(
            operation_id,
            source,
            &clamped_args,
            result,
            start,
            false,
            false,
            combined_validation,
        )
    }

    /// Log execution and return result
    fn log_and_return(
        &self,
        operation_id: &str,
        source: ToolCallSource,
        args: &serde_json::Value,
        result: Result<serde_json::Value, ToolRuntimeError>,
        start: Instant,
        dry_run: bool,
        from_fixture: bool,
        validation_result: Option<ValidationResult>,
    ) -> ToolCallResult {
        let duration_ms = start.elapsed().as_millis() as u64;

        // Generate log entry
        let mut counter = self.log_counter.write();
        *counter += 1;
        let id = *counter;

        let (success, response, error) = match &result {
            Ok(resp) => (true, Some(resp.clone()), None),
            Err(e) => (false, None, Some(e.to_string())),
        };

        let log_entry = ToolExecutionLog {
            id,
            timestamp: chrono::Local::now().to_rfc3339(),
            operation_id: operation_id.to_string(),
            source: source.clone(),
            args: args.clone(),
            success,
            response: response.clone(),
            error: error.clone(),
            duration_ms,
            dry_run,
            from_fixture,
            validation_result: validation_result.clone(),
        };

        let mut log = self.execution_log.write();
        log.push(log_entry);

        // Keep only last 500 entries
        let len = log.len();
        if len > 500 {
            log.drain(0..len - 500);
        }

        // Build result
        ToolCallResult {
            success,
            data: response,
            error,
            duration_ms,
            dry_run,
            from_fixture,
            validation: validation_result,
        }
    }

    /// Get execution logs
    pub fn get_execution_logs(&self) -> Vec<ToolExecutionLog> {
        self.execution_log.read().clone()
    }

    /// Clear execution logs
    pub fn clear_execution_logs(&self) {
        self.execution_log.write().clear();
    }

    /// List all available tools from OpenAPI spec
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        let spec = self.openapi_spec.read();
        if spec.is_none() {
            return Vec::new();
        }

        let spec = spec.as_ref().unwrap();
        let mut tools = Vec::new();

        if let Some(paths) = spec.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    for (method, operation) in methods_obj {
                        if !["get", "post", "put", "delete", "patch"].contains(&method.as_str()) {
                            continue;
                        }

                        let path_slug = path
                            .trim_start_matches('/')
                            .replace('/', "_")
                            .replace('{', "")
                            .replace('}', "");
                        let operation_id = format!("{}_{}", method, path_slug);

                        let description = operation
                            .get("summary")
                            .and_then(|s| s.as_str())
                            .or_else(|| operation.get("description").and_then(|s| s.as_str()))
                            .unwrap_or("No description")
                            .to_string();

                        // Extract tags from operation (OpenAPI tags array)
                        let tags: Vec<String> = operation
                            .get("tags")
                            .and_then(|t| t.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_else(|| vec!["uncategorized".to_string()]);

                        let config = self.get_tool_config(&operation_id);

                        tools.push(ToolInfo {
                            operation_id,
                            method: method.to_uppercase(),
                            path: path.clone(),
                            description,
                            tags,
                            config,
                        });
                    }
                }
            }
        }

        tools
    }

    /// Enable all tools
    pub fn enable_all_tools(&self) {
        for (_, config) in self.tool_configs.write().iter_mut() {
            config.enabled = true;
        }
    }

    /// Disable all tools
    pub fn disable_all_tools(&self) {
        for (_, config) in self.tool_configs.write().iter_mut() {
            config.enabled = false;
        }
    }

    /// Reset all circuit breakers
    pub fn reset_circuit_breakers(&self) {
        self.circuit_breakers.write().clear();
    }

    /// Get circuit breaker status for all tools
    pub fn get_circuit_breaker_status(&self) -> HashMap<String, CircuitBreakerState> {
        self.circuit_breakers.read().clone()
    }

    /// Clear all fixtures
    pub fn clear_fixtures(&self) {
        self.fixtures.write().clear();
    }

    /// Get all fixtures
    pub fn get_fixtures(&self) -> FixturesStorage {
        self.fixtures.read().clone()
    }

    /// Import fixtures from JSON
    pub fn import_fixtures(&self, fixtures: FixturesStorage) {
        *self.fixtures.write() = fixtures;
    }

    /// Export fixtures as JSON
    pub fn export_fixtures(&self) -> serde_json::Value {
        serde_json::to_value(self.fixtures.read().clone()).unwrap_or(serde_json::json!({}))
    }
}

/// Information about a tool
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub operation_id: String,
    pub method: String,
    pub path: String,
    pub description: String,
    /// OpenAPI tags for grouping/categorization
    pub tags: Vec<String>,
    pub config: ToolConfig,
}

//! Tool executor for ToolRuntime
//!
//! Handles actual HTTP execution of tools via the REST API.

use super::{ToolConfig, ToolRuntime, ToolRuntimeError};
use std::time::Duration;

impl ToolRuntime {
    /// Execute a tool by calling the REST API
    ///
    /// The operation_id encodes the HTTP method and path:
    ///   "get_jira_list" → GET /jira/list
    ///   "get_health"    → GET /health
    ///   "post_agent_chat" → POST /agent/chat
    pub async fn execute_tool(
        &self,
        operation_id: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, ToolRuntimeError> {
        // Parse operation_id back to method + path
        let (method, path) = self.parse_operation_id(operation_id)?;

        // Get API base URL from app state
        let base_url = self
            .state
            .api_base_url
            .read()
            .clone()
            .ok_or_else(|| ToolRuntimeError::InternalError("API base URL not set".to_string()))?;

        let url = format!("{}{}", base_url, path);

        // Get timeout from config
        let tool_config = self.get_tool_config(operation_id);
        let global_config = self.get_global_config();
        let timeout_ms = tool_config
            .timeout_ms
            .unwrap_or(global_config.default_timeout_ms);

        tracing::debug!(
            "Executing tool {} → {} {} (timeout: {}ms)",
            operation_id,
            method.to_uppercase(),
            url,
            timeout_ms
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| ToolRuntimeError::InternalError(format!("Failed to create client: {}", e)))?;

        let response = match method.as_str() {
            "get" => {
                let mut request = client.get(&url);
                // Build query params from args
                if let Some(obj) = args.as_object() {
                    for (key, value) in obj {
                        let val_str = match value {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        request = request.query(&[(key.as_str(), val_str.as_str())]);
                    }
                }
                request
                    .bearer_auth(&self.state.auth_token)
                    .send()
                    .await
                    .map_err(|e| self.map_reqwest_error(e))?
            }
            "post" => {
                client
                    .post(&url)
                    .bearer_auth(&self.state.auth_token)
                    .json(args)
                    .send()
                    .await
                    .map_err(|e| self.map_reqwest_error(e))?
            }
            "put" => {
                client
                    .put(&url)
                    .bearer_auth(&self.state.auth_token)
                    .json(args)
                    .send()
                    .await
                    .map_err(|e| self.map_reqwest_error(e))?
            }
            "delete" => {
                client
                    .delete(&url)
                    .bearer_auth(&self.state.auth_token)
                    .send()
                    .await
                    .map_err(|e| self.map_reqwest_error(e))?
            }
            "patch" => {
                client
                    .patch(&url)
                    .bearer_auth(&self.state.auth_token)
                    .json(args)
                    .send()
                    .await
                    .map_err(|e| self.map_reqwest_error(e))?
            }
            _ => {
                return Err(ToolRuntimeError::InvalidArguments(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| ToolRuntimeError::HttpError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            tracing::warn!(
                "Tool {} returned HTTP {}: {}",
                operation_id,
                status,
                &body[..body.len().min(200)]
            );
            return Err(ToolRuntimeError::HttpError(format!(
                "API returned HTTP {}: {}",
                status,
                &body[..body.len().min(500)]
            )));
        }

        // Parse response as JSON
        let json_response: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            ToolRuntimeError::InternalError(format!("Failed to parse response as JSON: {}", e))
        })?;

        tracing::debug!(
            "Tool {} completed successfully ({} bytes)",
            operation_id,
            body.len()
        );

        Ok(json_response)
    }

    /// Parse operation_id to (method, path)
    /// "get_jira_list" → ("get", "/jira/list")
    pub fn parse_operation_id(&self, operation_id: &str) -> Result<(String, String), ToolRuntimeError> {
        let parts: Vec<&str> = operation_id.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(ToolRuntimeError::InvalidArguments(format!(
                "Invalid operation_id format: {}",
                operation_id
            )));
        }

        let method = parts[0].to_string();
        let path = format!("/{}", parts[1].replace('_', "/"));

        Ok((method, path))
    }

    /// Map reqwest error to ToolRuntimeError
    fn map_reqwest_error(&self, e: reqwest::Error) -> ToolRuntimeError {
        if e.is_timeout() {
            ToolRuntimeError::Timeout
        } else if e.is_connect() {
            ToolRuntimeError::HttpError(format!("Connection failed: {}", e))
        } else {
            ToolRuntimeError::HttpError(format!("HTTP request failed: {}", e))
        }
    }

    /// Apply argument clamps to the input args
    pub fn apply_arg_clamps(
        &self,
        operation_id: &str,
        mut args: serde_json::Value,
        tool_config: &ToolConfig,
    ) -> serde_json::Value {
        if let Some(obj) = args.as_object_mut() {
            for (param_name, clamp) in &tool_config.arg_clamps {
                if let Some(value) = obj.get_mut(param_name) {
                    if let Some(num) = value.as_f64() {
                        let clamped = clamp.clamp(num);
                        if (clamped - num).abs() > f64::EPSILON {
                            tracing::debug!(
                                "Clamped {} from {} to {} for tool {}",
                                param_name,
                                num,
                                clamped,
                                operation_id
                            );
                            *value = serde_json::json!(clamped);
                        }
                    } else if let Some(num) = value.as_i64() {
                        let clamped = clamp.clamp(num as f64) as i64;
                        if clamped != num {
                            tracing::debug!(
                                "Clamped {} from {} to {} for tool {}",
                                param_name,
                                num,
                                clamped,
                                operation_id
                            );
                            *value = serde_json::json!(clamped);
                        }
                    }
                } else if let Some(default) = clamp.default {
                    // Apply default if param not provided
                    obj.insert(param_name.clone(), serde_json::json!(default));
                    tracing::debug!(
                        "Applied default {} = {} for tool {}",
                        param_name,
                        default,
                        operation_id
                    );
                }
            }
        }
        args
    }

    /// Generate a dry-run mock response
    pub fn generate_dry_run_response(
        &self,
        operation_id: &str,
        args: &serde_json::Value,
    ) -> serde_json::Value {
        // Try to generate a reasonable mock based on operation type
        let (method, _path) = self.parse_operation_id(operation_id).unwrap_or(("get".to_string(), String::new()));

        match method.as_str() {
            "get" => {
                // For list endpoints, return empty array
                if operation_id.contains("list") || operation_id.ends_with("s") {
                    serde_json::json!({
                        "items": [],
                        "total": 0,
                        "_dry_run": true,
                        "_operation_id": operation_id,
                        "_args": args
                    })
                } else {
                    // For single item endpoints
                    serde_json::json!({
                        "id": "dry-run-id",
                        "_dry_run": true,
                        "_operation_id": operation_id,
                        "_args": args
                    })
                }
            }
            "post" | "put" | "patch" => serde_json::json!({
                "success": true,
                "_dry_run": true,
                "_operation_id": operation_id,
                "_args": args
            }),
            "delete" => serde_json::json!({
                "deleted": true,
                "_dry_run": true,
                "_operation_id": operation_id,
                "_args": args
            }),
            _ => serde_json::json!({
                "_dry_run": true,
                "_operation_id": operation_id,
                "_args": args
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppState;
    use std::sync::Arc;

    fn create_test_runtime() -> Arc<ToolRuntime> {
        let state = AppState::new(
            "test-token".to_string(),
            "https://jira.test".to_string(),
            "test@test.com".to_string(),
            "api-token".to_string(),
            "gemini-key".to_string(),
        );
        ToolRuntime::new(state)
    }

    #[test]
    fn test_parse_operation_id() {
        let runtime = create_test_runtime();

        let (method, path) = runtime.parse_operation_id("get_jira_list").unwrap();
        assert_eq!(method, "get");
        assert_eq!(path, "/jira/list");

        let (method, path) = runtime.parse_operation_id("post_agent_chat").unwrap();
        assert_eq!(method, "post");
        assert_eq!(path, "/agent/chat");

        let (method, path) = runtime.parse_operation_id("get_health").unwrap();
        assert_eq!(method, "get");
        assert_eq!(path, "/health");
    }

    #[test]
    fn test_dry_run_response() {
        let runtime = create_test_runtime();
        let args = serde_json::json!({"test": "value"});

        let response = runtime.generate_dry_run_response("get_jira_list", &args);
        assert!(response.get("_dry_run").unwrap().as_bool().unwrap());
        assert!(response.get("items").is_some());

        let response = runtime.generate_dry_run_response("post_agent_chat", &args);
        assert!(response.get("_dry_run").unwrap().as_bool().unwrap());
        assert!(response.get("success").is_some());
    }
}

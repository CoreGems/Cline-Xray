//! Tool executor for agent operations
//!
//! This module executes tools by directly calling Rust functions from
//! the db module. NO HTTP calls are made - everything stays in-process.
//!
//! Key Insight: Direct Rust Calls, Not HTTP
//! ```
//! ❌ HTTP-to-HTTP (what we DON'T do):
//!    agent.rs → HTTP POST /db/query → server.rs → query.rs
//!
//! ✅ Direct Rust calls (what we DO):
//!    agent.rs → query::execute() directly
//! ```

use crate::agent::types::{ToolCall, ToolError, ToolResult};
use crate::db::query::execute_query;
use crate::db::validation::validate_gpt_query;
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

/// Tool executor that directly calls database functions
pub struct ToolExecutor {
    state: Arc<AppState>,
    /// Optional connection ID override (from request)
    connection_id: Option<String>,
}

impl ToolExecutor {
    /// Create a new tool executor with the given app state
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            connection_id: None,
        }
    }
    
    /// Create an executor with a specific connection ID
    pub fn with_connection_id(state: Arc<AppState>, connection_id: Option<String>) -> Self {
        Self {
            state,
            connection_id,
        }
    }
    
    /// Execute a tool call and return the result
    pub async fn execute(&self, tool_call: &ToolCall) -> ToolResult {
        let start = Instant::now();
        
        tracing::debug!("Executing tool: {} with args: {:?}", tool_call.name, tool_call.arguments);
        
        let result = match tool_call.name.as_str() {
            "list_schemas" => self.execute_list_schemas(tool_call).await,
            "list_tables" => self.execute_list_tables(tool_call).await,
            "list_columns" => self.execute_list_columns(tool_call).await,
            "list_indexes" => self.execute_list_indexes(tool_call).await,
            "list_constraints" => self.execute_list_constraints(tool_call).await,
            "execute_query" => self.execute_query(tool_call).await,
            "list_connections" => self.execute_list_connections(tool_call).await,
            "get_active_connection" => self.execute_get_active_connection(tool_call).await,
            _ => Err(ToolError::UnknownTool(tool_call.name.clone())),
        };
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        match result {
            Ok(data) => {
                tracing::debug!(
                    "Tool {} completed successfully in {}ms",
                    tool_call.name,
                    duration_ms
                );
                ToolResult {
                    tool_call_id: tool_call.id.clone(),
                    success: true,
                    data,
                    error: None,
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Tool {} failed in {}ms: {}",
                    tool_call.name,
                    duration_ms,
                    e
                );
                ToolResult::error(&tool_call.id, &e.to_string())
            }
        }
    }
    
    /// Get the connection ID to use (from request or active connection)
    fn get_connection_id(&self) -> Result<String, ToolError> {
        self.connection_id
            .clone()
            .or_else(|| self.state.get_active_connection_id())
            .ok_or(ToolError::NoConnection)
    }
    
    /// Get connection credentials and string
    fn get_connection_info(&self) -> Result<(String, String, String), ToolError> {
        let conn_id = self.get_connection_id()?;
        
        let conn_string = self.state.connection_pool
            .build_connection_string(&conn_id)
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        let (username, password) = self.state.connection_pool
            .get_credentials(&conn_id)
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        Ok((conn_string, username, password))
    }
    
    // ========================================================================
    // Tool Implementations
    // ========================================================================
    
    /// List all accessible schemas
    async fn execute_list_schemas(&self, _tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let (conn_string, username, password) = self.get_connection_info()?;
        
        let sql = "SELECT username as schema_name FROM all_users WHERE oracle_maintained = 'N' ORDER BY username";
        
        let result = execute_query(&conn_string, &username, &password, sql, 1000, 5000)
            .await
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        // Convert to a more useful format
        let schemas: Vec<String> = result.rows
            .iter()
            .filter_map(|row| row.first().and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();
        
        Ok(serde_json::json!({
            "schemas": schemas,
            "count": schemas.len()
        }))
    }
    
    /// List tables in a schema
    async fn execute_list_tables(&self, tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let schema = tool_call.get_param("schema")?;
        let (conn_string, username, password) = self.get_connection_info()?;
        
        let sql = format!(
            "SELECT table_name, num_rows, TO_CHAR(last_analyzed, 'YYYY-MM-DD') as last_analyzed \
             FROM all_tables WHERE owner = '{}' ORDER BY table_name",
            schema.to_uppercase().replace('\'', "''")
        );
        
        let result = execute_query(&conn_string, &username, &password, &sql, 1000, 5000)
            .await
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        // Convert rows to table info objects
        let tables: Vec<serde_json::Value> = result.rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "table_name": row.get(0).cloned().unwrap_or(serde_json::Value::Null),
                    "num_rows": row.get(1).cloned().unwrap_or(serde_json::Value::Null),
                    "last_analyzed": row.get(2).cloned().unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        
        Ok(serde_json::json!({
            "schema": schema.to_uppercase(),
            "tables": tables,
            "count": tables.len()
        }))
    }
    
    /// List columns in a table
    async fn execute_list_columns(&self, tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let schema = tool_call.get_param("schema")?;
        let table = tool_call.get_param("table")?;
        let (conn_string, username, password) = self.get_connection_info()?;
        
        let sql = format!(
            "SELECT column_name, data_type, data_length, nullable, column_id \
             FROM all_tab_columns WHERE owner = '{}' AND table_name = '{}' ORDER BY column_id",
            schema.to_uppercase().replace('\'', "''"),
            table.to_uppercase().replace('\'', "''")
        );
        
        let result = execute_query(&conn_string, &username, &password, &sql, 1000, 5000)
            .await
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        let columns: Vec<serde_json::Value> = result.rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "column_name": row.get(0).cloned().unwrap_or(serde_json::Value::Null),
                    "data_type": row.get(1).cloned().unwrap_or(serde_json::Value::Null),
                    "data_length": row.get(2).cloned().unwrap_or(serde_json::Value::Null),
                    "nullable": row.get(3).cloned().unwrap_or(serde_json::Value::Null),
                    "column_id": row.get(4).cloned().unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        
        Ok(serde_json::json!({
            "schema": schema.to_uppercase(),
            "table": table.to_uppercase(),
            "columns": columns,
            "count": columns.len()
        }))
    }
    
    /// List indexes in a schema
    async fn execute_list_indexes(&self, tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let schema = tool_call.get_param("schema")?;
        let (conn_string, username, password) = self.get_connection_info()?;
        
        let sql = format!(
            "SELECT index_name, table_name, index_type, uniqueness, status \
             FROM all_indexes WHERE owner = '{}' ORDER BY table_name, index_name",
            schema.to_uppercase().replace('\'', "''")
        );
        
        let result = execute_query(&conn_string, &username, &password, &sql, 1000, 5000)
            .await
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        let indexes: Vec<serde_json::Value> = result.rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "index_name": row.get(0).cloned().unwrap_or(serde_json::Value::Null),
                    "table_name": row.get(1).cloned().unwrap_or(serde_json::Value::Null),
                    "index_type": row.get(2).cloned().unwrap_or(serde_json::Value::Null),
                    "uniqueness": row.get(3).cloned().unwrap_or(serde_json::Value::Null),
                    "status": row.get(4).cloned().unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        
        Ok(serde_json::json!({
            "schema": schema.to_uppercase(),
            "indexes": indexes,
            "count": indexes.len()
        }))
    }
    
    /// List constraints in a schema
    async fn execute_list_constraints(&self, tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let schema = tool_call.get_param("schema")?;
        let (conn_string, username, password) = self.get_connection_info()?;
        
        let sql = format!(
            "SELECT constraint_name, constraint_type, table_name, status \
             FROM all_constraints WHERE owner = '{}' ORDER BY table_name, constraint_name",
            schema.to_uppercase().replace('\'', "''")
        );
        
        let result = execute_query(&conn_string, &username, &password, &sql, 1000, 5000)
            .await
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        let constraints: Vec<serde_json::Value> = result.rows
            .iter()
            .map(|row| {
                serde_json::json!({
                    "constraint_name": row.get(0).cloned().unwrap_or(serde_json::Value::Null),
                    "constraint_type": row.get(1).cloned().unwrap_or(serde_json::Value::Null),
                    "table_name": row.get(2).cloned().unwrap_or(serde_json::Value::Null),
                    "status": row.get(3).cloned().unwrap_or(serde_json::Value::Null)
                })
            })
            .collect();
        
        Ok(serde_json::json!({
            "schema": schema.to_uppercase(),
            "constraints": constraints,
            "count": constraints.len()
        }))
    }
    
    /// Execute a SELECT query
    async fn execute_query(&self, tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let sql = tool_call.get_param("sql")?;
        let max_rows = tool_call.get_int_param_or("max_rows", 100).min(1000);
        
        // SECURITY: Validate that this is a SELECT-only query
        validate_gpt_query(&sql)
            .map_err(|e| ToolError::ValidationError(e.to_string()))?;
        
        let (conn_string, username, password) = self.get_connection_info()?;
        
        let result = execute_query(&conn_string, &username, &password, &sql, max_rows, 5000)
            .await
            .map_err(|e| ToolError::DatabaseError(e.to_string()))?;
        
        // Format the result in a readable way for the LLM
        Ok(serde_json::json!({
            "columns": result.columns,
            "rows": result.rows,
            "row_count": result.rows.len(),
            "truncated": result.truncated,
            "execution_time_ms": result.execution_time_ms
        }))
    }
    
    /// List all available connections (GPT-safe, no credentials)
    async fn execute_list_connections(&self, _tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let active_id = self.state.get_active_connection_id();
        let connections = self.state.connection_pool.list_all_metadata();
        
        #[derive(Serialize)]
        struct ConnectionSummary {
            id: String,
            name: String,
            environment: Option<String>,
            service_name: String,
            is_active: bool,
        }
        
        let summaries: Vec<ConnectionSummary> = connections
            .iter()
            .map(|meta| ConnectionSummary {
                id: meta.id.clone(),
                name: meta.name.clone(),
                environment: meta.environment.clone(),
                service_name: meta.service_name.clone(),
                is_active: active_id.as_ref() == Some(&meta.id),
            })
            .collect();
        
        Ok(serde_json::json!({
            "connections": summaries,
            "count": summaries.len(),
            "active_id": active_id
        }))
    }
    
    /// Get the currently active connection
    async fn execute_get_active_connection(&self, _tool_call: &ToolCall) -> Result<serde_json::Value, ToolError> {
        let active_id = self.state.get_active_connection_id()
            .ok_or(ToolError::NoConnection)?;
        
        let meta = self.state.connection_pool
            .get_connection(&active_id)
            .ok_or(ToolError::NoConnection)?;
        
        Ok(serde_json::json!({
            "id": meta.id,
            "name": meta.name,
            "environment": meta.environment,
            "service_name": meta.service_name,
            "is_active": true
        }))
    }
}

// ============================================================================
// OpenAPI-Based Generic Tool Executor
// ============================================================================

/// Execute any auto-generated OpenAPI tool by calling its corresponding REST endpoint.
///
/// The tool name encodes the HTTP method and path:
///   "get_jira_list" → GET /jira/list
///   "get_health"    → GET /health
///   "post_agent_chat" → POST /agent/chat
///
/// For GET requests, args are sent as query parameters.
/// For POST requests, args are sent as JSON body.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `base_url` - Base URL of the REST API (e.g., "http://127.0.0.1:3030")
/// * `auth_token` - Bearer token for authenticated endpoints
/// * `tool_name` - Auto-generated tool name (e.g., "get_jira_list")
/// * `args` - Tool arguments as JSON (from LLM function call)
///
/// # Returns
/// The response body as a string (usually JSON)
pub async fn execute_openapi_tool(
    client: &reqwest::Client,
    base_url: &str,
    auth_token: &str,
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<String, ToolError> {
    // Parse tool name back to method + path
    // "get_jira_list" → method="get", path_part="jira_list" → "/jira/list"
    let parts: Vec<&str> = tool_name.splitn(2, '_').collect();
    let method = parts.first().copied().unwrap_or("get");
    let path = format!(
        "/{}",
        parts.get(1).unwrap_or(&"").replace('_', "/")
    );
    let url = format!("{}{}", base_url, path);

    tracing::debug!(
        "Executing OpenAPI tool: {} → {} {}",
        tool_name,
        method.to_uppercase(),
        url
    );

    let response = match method {
        "get" => {
            // Build query params from args
            let mut request = client.get(&url);
            if let Some(obj) = args.as_object() {
                for (key, value) in obj {
                    // Strip quotes from string values for query params
                    let val_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    request = request.query(&[(key.as_str(), val_str.as_str())]);
                }
            }
            request
                .bearer_auth(auth_token)
                .send()
                .await
                .map_err(|e| ToolError::DatabaseError(format!("HTTP request failed: {}", e)))?
        }
        "post" => {
            client
                .post(&url)
                .bearer_auth(auth_token)
                .json(args)
                .send()
                .await
                .map_err(|e| ToolError::DatabaseError(format!("HTTP request failed: {}", e)))?
        }
        "put" => {
            client
                .put(&url)
                .bearer_auth(auth_token)
                .json(args)
                .send()
                .await
                .map_err(|e| ToolError::DatabaseError(format!("HTTP request failed: {}", e)))?
        }
        "delete" => {
            client
                .delete(&url)
                .bearer_auth(auth_token)
                .send()
                .await
                .map_err(|e| ToolError::DatabaseError(format!("HTTP request failed: {}", e)))?
        }
        _ => {
            return Err(ToolError::InvalidParameter(format!(
                "Unsupported HTTP method: {}",
                method
            )));
        }
    };

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| ToolError::DatabaseError(format!("Failed to read response: {}", e)))?;

    if !status.is_success() {
        tracing::warn!(
            "OpenAPI tool {} returned HTTP {}: {}",
            tool_name,
            status,
            &body[..body.len().min(200)]
        );
        return Err(ToolError::DatabaseError(format!(
            "API returned HTTP {}: {}",
            status,
            &body[..body.len().min(500)]
        )));
    }

    tracing::debug!(
        "OpenAPI tool {} completed successfully ({} bytes)",
        tool_name,
        body.len()
    );
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_get_param() {
        let mut args = std::collections::HashMap::new();
        args.insert("schema".to_string(), serde_json::json!("HR"));
        args.insert("max_rows".to_string(), serde_json::json!(50));
        
        let tool_call = ToolCall {
            id: "test-1".to_string(),
            name: "list_tables".to_string(),
            arguments: args,
        };
        
        assert_eq!(tool_call.get_param("schema").unwrap(), "HR");
        assert!(tool_call.get_param("missing").is_err());
        assert_eq!(tool_call.get_int_param_or("max_rows", 100), 50);
        assert_eq!(tool_call.get_int_param_or("missing", 100), 100);
    }
}

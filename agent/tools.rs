//! Tool definitions for the agent
//!
//! These tools are shared across all LLM providers. Each provider
//! converts these definitions to their specific format.
//!
//! SECURITY: All tools here are GPT-safe (read-only, constrained queries).

use serde::{Deserialize, Serialize};

/// Parameter definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    
    /// Parameter type ("string", "integer", "boolean", "array", "object")
    pub param_type: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Whether this parameter is required
    pub required: bool,
    
    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    
    /// Enum values (if restricted to specific values)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Definition of a tool available to the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (must be a valid function name)
    pub name: String,
    
    /// Human-readable description of what the tool does
    pub description: String,
    
    /// Parameters the tool accepts
    pub parameters: Vec<ToolParameter>,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters: Vec::new(),
        }
    }
    
    /// Add a required string parameter
    pub fn with_string_param(mut self, name: &str, description: &str) -> Self {
        self.parameters.push(ToolParameter {
            name: name.to_string(),
            param_type: "string".to_string(),
            description: description.to_string(),
            required: true,
            default: None,
            enum_values: None,
        });
        self
    }
    
    /// Add an optional string parameter with default
    pub fn with_optional_string_param(mut self, name: &str, description: &str, default: &str) -> Self {
        self.parameters.push(ToolParameter {
            name: name.to_string(),
            param_type: "string".to_string(),
            description: description.to_string(),
            required: false,
            default: Some(serde_json::Value::String(default.to_string())),
            enum_values: None,
        });
        self
    }
    
    /// Add an optional integer parameter with default
    pub fn with_optional_int_param(mut self, name: &str, description: &str, default: i64) -> Self {
        self.parameters.push(ToolParameter {
            name: name.to_string(),
            param_type: "integer".to_string(),
            description: description.to_string(),
            required: false,
            default: Some(serde_json::Value::Number(default.into())),
            enum_values: None,
        });
        self
    }
}

/// Paths to skip when auto-generating tools from OpenAPI.
/// These should NOT become agent tools.
const SKIP_PATHS: &[&str] = &[
    "/agent/chat",     // This IS the agent - don't recurse
    "/openapi.json",   // Meta endpoint - not useful as a tool
];

/// Auto-generate tool definitions from the OpenAPI spec.
///
/// Each OpenAPI path+method becomes one tool, with naming convention:
///   `{method}_{path_segments}` — e.g. GET /jira/list → "get_jira_list"
///
/// Skips endpoints in SKIP_PATHS (the agent itself, meta endpoints).
///
/// # How it works
/// 1. Serializes the OpenAPI spec to JSON (avoids complex utoipa type introspection)
/// 2. Iterates over all paths and HTTP methods
/// 3. Extracts parameters from query params and request body schemas
/// 4. Returns vendor-neutral ToolDefinition structs
///
/// # Example
/// ```rust,ignore
/// let tools = tools_from_openapi();
/// // tools = [
/// //   ToolDefinition { name: "get_health", ... },
/// //   ToolDefinition { name: "get_jira_list", params: [jql, maxResults], ... },
/// //   ToolDefinition { name: "get_agent_models", ... },
/// // ]
/// ```
pub fn tools_from_openapi() -> Vec<ToolDefinition> {
    use crate::openapi::PublicApiDoc;
    use utoipa::OpenApi;

    // Serialize OpenAPI spec to JSON for easier parsing
    let spec = PublicApiDoc::openapi();
    let spec_json = match serde_json::to_value(&spec) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to serialize OpenAPI spec: {}", e);
            return Vec::new();
        }
    };

    let mut tools = Vec::new();

    let paths = match spec_json.get("paths").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return tools,
    };

    for (path, methods) in paths {
        // Skip non-tool endpoints
        if SKIP_PATHS.contains(&path.as_str()) {
            continue;
        }

        let methods_obj = match methods.as_object() {
            Some(m) => m,
            None => continue,
        };

        for (method, operation) in methods_obj {
            // Skip non-operation keys (e.g., "summary", "description", "parameters")
            if !["get", "post", "put", "delete", "patch"].contains(&method.as_str()) {
                continue;
            }

            // Build tool name: get_jira_list, get_health, get_agent_models
            let path_slug = path
                .trim_start_matches('/')
                .replace('/', "_")
                .replace('{', "")
                .replace('}', "");
            let tool_name = format!("{}_{}", method, path_slug);

            let description = operation
                .get("summary")
                .and_then(|s| s.as_str())
                .or_else(|| operation.get("description").and_then(|s| s.as_str()))
                .unwrap_or("No description")
                .to_string();

            let mut tool = ToolDefinition::new(&tool_name, &description);

            // Extract query/path parameters
            if let Some(params) = operation.get("parameters").and_then(|p| p.as_array()) {
                for param in params {
                    let name = param.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let schema_type = param
                        .get("schema")
                        .and_then(|s| s.get("type"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("string");
                    let param_desc = param
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    let required = param
                        .get("required")
                        .and_then(|r| r.as_bool())
                        .unwrap_or(false);

                    if !name.is_empty() {
                        tool.parameters.push(ToolParameter {
                            name: name.to_string(),
                            param_type: schema_type.to_string(),
                            description: param_desc.to_string(),
                            required,
                            default: None,
                            enum_values: None,
                        });
                    }
                }
            }

            // Extract request body parameters (for POST endpoints)
            if let Some(request_body) = operation.get("requestBody") {
                let schema = request_body
                    .get("content")
                    .and_then(|c| c.get("application/json"))
                    .and_then(|j| j.get("schema"));

                if let Some(schema) = schema {
                    // Resolve $ref if present
                    let resolved = if let Some(ref_path) = schema.get("$ref").and_then(|r| r.as_str()) {
                        resolve_json_ref(&spec_json, ref_path)
                    } else {
                        Some(schema)
                    };

                    if let Some(resolved_schema) = resolved {
                        let required_fields: Vec<&str> = resolved_schema
                            .get("required")
                            .and_then(|r| r.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                            .unwrap_or_default();

                        if let Some(properties) = resolved_schema.get("properties").and_then(|p| p.as_object()) {
                            for (prop_name, prop_schema) in properties {
                                let prop_type = prop_schema
                                    .get("type")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("string");
                                let prop_desc = prop_schema
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("");

                                tool.parameters.push(ToolParameter {
                                    name: prop_name.clone(),
                                    param_type: prop_type.to_string(),
                                    description: prop_desc.to_string(),
                                    required: required_fields.contains(&prop_name.as_str()),
                                    default: None,
                                    enum_values: None,
                                });
                            }
                        }
                    }
                }
            }

            tools.push(tool);
        }
    }

    tools
}

/// Resolve a JSON $ref pointer (e.g., "#/components/schemas/ChatRequest")
fn resolve_json_ref<'a>(spec: &'a serde_json::Value, ref_path: &str) -> Option<&'a serde_json::Value> {
    if !ref_path.starts_with("#/") {
        return None;
    }
    let parts: Vec<&str> = ref_path[2..].split('/').collect();
    let mut current = spec;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

/// Get all tool definitions available to the agent
///
/// SECURITY: These tools are GPT-safe:
/// - Schema exploration (read-only)
/// - Constrained SELECT queries (row-limited, timeout-enforced)
///
/// NOT INCLUDED (UI-only):
/// - Connection management
/// - DDL/DML execution
/// - Raw SQL execution
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // ============================================================
        // Schema Exploration Tools
        // ============================================================
        ToolDefinition::new(
            "list_schemas",
            "List all accessible database schemas. Returns schema names that the current user can access.",
        ),
        
        ToolDefinition::new(
            "list_tables",
            "List all tables in a specific schema. Returns table names, row counts, and last analyzed dates.",
        )
        .with_string_param("schema", "The schema name to list tables from (e.g., 'HR', 'SALES')"),
        
        ToolDefinition::new(
            "list_columns",
            "List all columns in a specific table. Returns column names, data types, lengths, and nullability.",
        )
        .with_string_param("schema", "The schema name (e.g., 'HR')")
        .with_string_param("table", "The table name (e.g., 'EMPLOYEES')"),
        
        ToolDefinition::new(
            "list_indexes",
            "List all indexes in a specific schema. Returns index names, types, uniqueness, and status.",
        )
        .with_string_param("schema", "The schema name to list indexes from"),
        
        ToolDefinition::new(
            "list_constraints",
            "List all constraints in a specific schema. Returns constraint names, types, table names, and status.",
        )
        .with_string_param("schema", "The schema name to list constraints from"),
        
        // ============================================================
        // Query Execution Tool
        // ============================================================
        ToolDefinition::new(
            "execute_query",
            "Execute a SELECT query against the database. Only SELECT statements are allowed. \
             Results are limited to prevent overwhelming responses. Use this to answer questions \
             about the actual data in the database.",
        )
        .with_string_param(
            "sql",
            "The SELECT SQL query to execute. Must be a valid Oracle SELECT statement. \
             No DDL (CREATE, ALTER, DROP) or DML (INSERT, UPDATE, DELETE) allowed.",
        )
        .with_optional_int_param(
            "max_rows",
            "Maximum number of rows to return (default: 100, max: 1000)",
            100,
        ),
        
        // ============================================================
        // Connection Tools (GPT-safe, no credentials)
        // ============================================================
        ToolDefinition::new(
            "list_connections",
            "List all available database connections. Returns connection IDs, names, and environments. \
             Use this to find which connections are available before running queries.",
        ),
        
        ToolDefinition::new(
            "get_active_connection",
            "Get the currently active database connection. Returns the connection ID, name, and environment \
             of the connection that will be used for queries if no specific connection_id is provided.",
        ),
    ]
}

/// Convert tool definitions to Gemini function declaration format
pub fn to_gemini_functions(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
    tools.iter().map(|tool| {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        for param in &tool.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), serde_json::Value::String(param.param_type.clone()));
            prop.insert("description".to_string(), serde_json::Value::String(param.description.clone()));
            
            if let Some(ref enum_values) = param.enum_values {
                prop.insert(
                    "enum".to_string(),
                    serde_json::Value::Array(
                        enum_values.iter().map(|v| serde_json::Value::String(v.clone())).collect()
                    ),
                );
            }
            
            properties.insert(param.name.clone(), serde_json::Value::Object(prop));
            
            if param.required {
                required.push(serde_json::Value::String(param.name.clone()));
            }
        }
        
        let parameters = if properties.is_empty() {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        } else {
            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            })
        };
        
        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "parameters": parameters
        })
    }).collect()
}

/// Convert tool definitions to OpenAI function format (for future use)
#[allow(dead_code)]
pub fn to_openai_functions(tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
    tools.iter().map(|tool| {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        for param in &tool.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".to_string(), serde_json::Value::String(param.param_type.clone()));
            prop.insert("description".to_string(), serde_json::Value::String(param.description.clone()));
            
            if let Some(ref enum_values) = param.enum_values {
                prop.insert(
                    "enum".to_string(),
                    serde_json::Value::Array(
                        enum_values.iter().map(|v| serde_json::Value::String(v.clone())).collect()
                    ),
                );
            }
            
            properties.insert(param.name.clone(), serde_json::Value::Object(prop));
            
            if param.required {
                required.push(serde_json::Value::String(param.name.clone()));
            }
        }
        
        serde_json::json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tool_definitions() {
        let tools = get_tool_definitions();
        
        // Should have the expected tools
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"list_schemas"));
        assert!(tool_names.contains(&"list_tables"));
        assert!(tool_names.contains(&"list_columns"));
        assert!(tool_names.contains(&"execute_query"));
        assert!(tool_names.contains(&"list_connections"));
    }

    #[test]
    fn test_to_gemini_functions() {
        let tools = get_tool_definitions();
        let gemini_funcs = to_gemini_functions(&tools);
        
        // Should produce valid JSON for each tool
        for func in gemini_funcs {
            assert!(func.get("name").is_some());
            assert!(func.get("description").is_some());
            assert!(func.get("parameters").is_some());
        }
    }

    #[test]
    fn test_execute_query_tool_has_sql_param() {
        let tools = get_tool_definitions();
        let execute_query = tools.iter().find(|t| t.name == "execute_query").unwrap();
        
        let sql_param = execute_query.parameters.iter().find(|p| p.name == "sql");
        assert!(sql_param.is_some());
        assert!(sql_param.unwrap().required);
    }
}

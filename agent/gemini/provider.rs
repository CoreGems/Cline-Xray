//! Gemini provider implementation
//!
//! This module implements the AgentProvider trait for Google's Gemini API.
//! It handles the agent loop: sending messages, processing function calls,
//! and returning results.

use crate::agent::executor::ToolExecutor;
use crate::agent::gemini::client::{
    GeminiClient, GeminiContent, GeminiFunctionCall, GeminiFunctionResponse, GeminiPart, GeminiTool,
};
use crate::agent::provider::AgentProvider;
use crate::agent::tools::{to_gemini_functions, ToolDefinition};
use crate::agent::types::{
    AgentError, AgentRequest, AgentResponse, SourceCitation, TokenUsage, TraceEntry,
};
use async_trait::async_trait;
use std::time::Instant;

/// System prompt for the Oracle X-Ray agent
const SYSTEM_PROMPT: &str = r#"You are an Oracle Database expert assistant. You help users explore and understand Oracle databases by using available tools proactively.

**IMPORTANT: BE PROACTIVE WITH TOOLS**
- ALWAYS use tools to find information instead of asking the user
- When a user mentions a table without a schema, use list_schemas() then list_tables(schema) to FIND which schema contains that table
- When asked about columns, ALWAYS call list_columns() to get the actual data
- When asked about data, ALWAYS execute a query to show results
- NEVER ask the user for information you can discover with tools

**Tools Available**:
1. **Schema Exploration**: 
   - list_schemas() - Get all accessible schemas
   - list_tables(schema) - Get tables in a schema
   - list_columns(schema, table) - Get columns in a table
   - list_indexes(schema) - Get indexes in a schema
   - list_constraints(schema) - Get constraints in a schema

2. **Data Analysis**: 
   - execute_query(sql, max_rows) - Run SELECT queries (only SELECT allowed)

3. **Connection Awareness**: 
   - list_connections() - Get available database connections
   - get_active_connection() - Get current connection info

**Workflow for "Show columns in TABLE_NAME"**:
1. Call list_schemas() to get available schemas
2. Call list_tables(schema) for each schema to find where TABLE_NAME exists
3. Call list_columns(schema, table) to get the column details
4. Present the results in a formatted way

**Guidelines**:
- Act on requests immediately using tools - don't ask clarifying questions if you can find the answer
- If a table exists in multiple schemas, show all of them or ask which one
- Write efficient queries (use WHERE clauses, ROWNUM/FETCH FIRST for limits)
- Explain your findings clearly with context
- If a query fails, explain the error and try a different approach

**Important**: Be action-oriented. Use your tools first, explain later.
"#;

/// Gemini provider for the agent
pub struct GeminiProvider {
    client: GeminiClient,
    api_key: String,
    default_model: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider
    pub fn new(api_key: &str, timeout_secs: u64) -> Self {
        Self {
            client: GeminiClient::new(api_key, timeout_secs),
            api_key: api_key.to_string(),
            default_model: "gemini-1.5-flash".to_string(),
        }
    }
    
    /// Create a new Gemini provider with a specific default model
    pub fn with_model(api_key: &str, model: &str, timeout_secs: u64) -> Self {
        Self {
            client: GeminiClient::new(api_key, timeout_secs),
            api_key: api_key.to_string(),
            default_model: model.to_string(),
        }
    }
}

#[async_trait]
impl AgentProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }
    
    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
    
    fn default_model(&self) -> &str {
        &self.default_model
    }
    
    fn supported_models(&self) -> Vec<&str> {
        vec![
            "gemini-1.5-flash",
            "gemini-1.5-flash-8b",
            "gemini-1.5-pro",
            "gemini-2.0-flash",
        ]
    }
    
    async fn run(
        &self,
        request: &AgentRequest,
        tools: &[ToolDefinition],
        executor: &ToolExecutor,
        max_iterations: u32,
    ) -> Result<AgentResponse, AgentError> {
        let model = request.model_id.as_deref().unwrap_or(&self.default_model);
        let session_id = request.session_id.clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        tracing::info!(
            "Starting Gemini agent (model: {}, session: {})",
            model,
            session_id
        );
        
        // Convert tools to Gemini format
        let gemini_tools = if tools.is_empty() {
            None
        } else {
            Some(vec![GeminiTool {
                function_declarations: to_gemini_functions(tools),
            }])
        };
        
        // Build initial conversation
        let mut contents = vec![GeminiContent::user(&request.question)];
        let mut trace = Vec::new();
        let mut sources = Vec::new();
        let mut total_tokens = TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };
        
        // Add initial trace entry
        trace.push(TraceEntry {
            step: 0,
            action_type: "question".to_string(),
            content: request.question.clone(),
            timestamp: chrono::Utc::now(),
            tool_name: None,
            duration_ms: None,
        });
        
        // Agent loop
        for iteration in 0..max_iterations {
            let step = iteration + 1;
            let iter_start = Instant::now();
            
            tracing::debug!("Agent iteration {} (model: {})", step, model);
            
            // Call Gemini API
            let response = self.client
                .generate_content(model, contents.clone(), gemini_tools.clone(), Some(SYSTEM_PROMPT))
                .await?;
            
            // Update token usage
            if let Some(usage) = response.get_token_usage() {
                total_tokens.prompt_tokens += usage.prompt_tokens;
                total_tokens.completion_tokens += usage.completion_tokens;
                total_tokens.total_tokens += usage.total_tokens;
            }
            
            // Check if response has function calls
            if response.has_function_calls() {
                let function_calls = response.get_function_calls();
                
                tracing::debug!(
                    "Gemini requested {} function call(s)",
                    function_calls.len()
                );
                
                // Execute each function call
                let mut function_responses = Vec::new();
                let mut model_function_calls = Vec::new();
                
                for tool_call in &function_calls {
                    let tool_start = Instant::now();
                    
                    // Add trace entry for tool call
                    trace.push(TraceEntry {
                        step,
                        action_type: "tool_call".to_string(),
                        content: format!(
                            "Calling {} with {:?}",
                            tool_call.name, tool_call.arguments
                        ),
                        timestamp: chrono::Utc::now(),
                        tool_name: Some(tool_call.name.clone()),
                        duration_ms: None,
                    });
                    
                    // Execute the tool
                    let result = executor.execute(tool_call).await;
                    let tool_duration = tool_start.elapsed().as_millis() as u64;
                    
                    // Add trace entry for result
                    let result_summary = if result.success {
                        // Truncate large results for trace
                        let data_str = result.data.to_string();
                        if data_str.len() > 500 {
                            format!("{}...(truncated)", &data_str[..500])
                        } else {
                            data_str
                        }
                    } else {
                        format!("Error: {}", result.error.as_deref().unwrap_or("Unknown"))
                    };
                    
                    trace.push(TraceEntry {
                        step,
                        action_type: "tool_result".to_string(),
                        content: result_summary,
                        timestamp: chrono::Utc::now(),
                        tool_name: Some(tool_call.name.clone()),
                        duration_ms: Some(tool_duration),
                    });
                    
                    // Track sources from tool calls
                    if result.success {
                        match tool_call.name.as_str() {
                            "list_schemas" => sources.push(SourceCitation {
                                source_type: "metadata".to_string(),
                                description: "Database schema listing".to_string(),
                                reference: None,
                            }),
                            "list_tables" => {
                                if let Some(schema) = tool_call.arguments.get("schema") {
                                    sources.push(SourceCitation {
                                        source_type: "metadata".to_string(),
                                        description: format!("Tables in {}", schema),
                                        reference: Some(schema.as_str().unwrap_or("").to_uppercase()),
                                    });
                                }
                            }
                            "list_columns" => {
                                let schema = tool_call.arguments.get("schema")
                                    .and_then(|v| v.as_str()).unwrap_or("");
                                let table = tool_call.arguments.get("table")
                                    .and_then(|v| v.as_str()).unwrap_or("");
                                sources.push(SourceCitation {
                                    source_type: "metadata".to_string(),
                                    description: format!("Columns in {}.{}", schema, table),
                                    reference: Some(format!("{}.{}", schema.to_uppercase(), table.to_uppercase())),
                                });
                            }
                            "execute_query" => {
                                if let Some(sql) = tool_call.arguments.get("sql") {
                                    sources.push(SourceCitation {
                                        source_type: "query_result".to_string(),
                                        description: "Query execution result".to_string(),
                                        reference: Some(sql.as_str().unwrap_or("").to_string()),
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    
                    // Build Gemini function call for history
                    model_function_calls.push(GeminiFunctionCall {
                        name: tool_call.name.clone(),
                        args: serde_json::to_value(&tool_call.arguments)
                            .unwrap_or(serde_json::Value::Object(Default::default())),
                    });
                    
                    // Build function response
                    function_responses.push(GeminiFunctionResponse {
                        name: tool_call.name.clone(),
                        response: if result.success {
                            result.data
                        } else {
                            serde_json::json!({
                                "error": result.error.unwrap_or_else(|| "Unknown error".to_string())
                            })
                        },
                    });
                }
                
                // Add model's function calls to history
                contents.push(GeminiContent::model_with_function_calls(model_function_calls));
                
                // Add function responses to history
                contents.push(GeminiContent::user_with_function_responses(function_responses));
                
            } else {
                // No function calls - this is the final answer
                let answer = response.get_text()
                    .unwrap_or_else(|| "I couldn't generate a response.".to_string());
                
                tracing::info!(
                    "Agent completed in {} iterations (total tokens: {})",
                    step,
                    total_tokens.total_tokens
                );
                
                // Add final trace entry
                trace.push(TraceEntry {
                    step,
                    action_type: "answer".to_string(),
                    content: answer.clone(),
                    timestamp: chrono::Utc::now(),
                    tool_name: None,
                    duration_ms: Some(iter_start.elapsed().as_millis() as u64),
                });
                
                // Deduplicate sources
                let mut unique_sources = Vec::new();
                for source in sources {
                    if !unique_sources.iter().any(|s: &SourceCitation| {
                        s.source_type == source.source_type && s.reference == source.reference
                    }) {
                        unique_sources.push(source);
                    }
                }
                
                return Ok(AgentResponse {
                    answer,
                    sources: unique_sources,
                    trace,
                    session_id,
                    tokens_used: Some(total_tokens),
                    model_id: Some(model.to_string()),
                });
            }
        }
        
        // If we reach here, we exceeded max iterations
        Err(AgentError::MaxIterationsExceeded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let provider = GeminiProvider::new("test-key", 60);
        
        assert_eq!(provider.name(), "gemini");
        assert!(provider.is_configured());
        assert_eq!(provider.default_model(), "gemini-1.5-flash");
        assert!(provider.supports_model("gemini-1.5-flash"));
        assert!(provider.supports_model("gemini-1.5-pro"));
        assert!(!provider.supports_model("gpt-4"));
    }

    #[test]
    fn test_supported_models() {
        let provider = GeminiProvider::new("test-key", 60);
        let models = provider.supported_models();
        
        assert!(models.contains(&"gemini-1.5-flash"));
        assert!(models.contains(&"gemini-1.5-pro"));
        assert!(models.contains(&"gemini-2.0-flash"));
    }
}

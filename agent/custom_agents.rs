//! Custom Agent Management
//!
//! This module implements the storage and management of custom agents.
//! Agents are stored as JSON files in the user's app data directory.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Custom agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CustomAgent {
    /// Unique agent identifier
    pub id: String,
    
    /// Agent display name
    pub name: String,
    
    /// Icon (emoji or icon name)
    pub icon: String,
    
    /// Short description
    pub description: String,
    
    /// LLM provider
    #[serde(default = "default_provider")]
    pub provider: String,
    
    /// Model ID (e.g., "gemini-1.5-flash")
    pub model: String,
    
    /// Temperature (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// Max tokens for response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    
    /// System prompt (may contain template variables like {{SCHEMAS}})
    pub system_prompt: String,
    
    /// Conversation starters
    #[serde(default)]
    pub starters: Vec<String>,
    
    /// Tool source: "builtin" or "custom"
    #[serde(default = "default_tool_source")]
    pub tool_source: String,
    
    /// Custom OpenAPI URL (only if tool_source == "custom")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_openapi_url: Option<String>,
    
    /// Enabled tool IDs (operationIds)
    #[serde(default = "default_enabled_tools")]
    pub enabled_tools: Vec<String>,
    
    /// Tool description overrides
    #[serde(default)]
    pub tool_overrides: HashMap<String, ToolOverride>,
    
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    
    /// Last update timestamp (ISO 8601)
    pub updated_at: String,
    
    /// Whether this is a built-in agent
    #[serde(default)]
    pub is_builtin: bool,
}

/// Tool description override
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ToolOverride {
    /// Custom description for this tool
    pub description: String,
}

/// Request to create a new agent
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct CreateAgentRequest {
    /// Agent display name
    pub name: String,
    
    /// Icon (emoji or icon name)
    #[serde(default = "default_icon")]
    pub icon: String,
    
    /// Short description
    #[serde(default)]
    pub description: String,
    
    /// LLM provider
    #[serde(default = "default_provider")]
    pub provider: String,
    
    /// Model ID
    #[serde(default = "default_model")]
    pub model: String,
    
    /// Temperature
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// Max tokens
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    
    /// System prompt
    #[serde(default)]
    pub system_prompt: String,
    
    /// Conversation starters
    #[serde(default)]
    pub starters: Vec<String>,
    
    /// Tool source
    #[serde(default = "default_tool_source")]
    pub tool_source: String,
    
    /// Custom OpenAPI URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_openapi_url: Option<String>,
    
    /// Enabled tool IDs
    #[serde(default = "default_enabled_tools")]
    pub enabled_tools: Vec<String>,
    
    /// Tool description overrides
    #[serde(default)]
    pub tool_overrides: HashMap<String, ToolOverride>,
}

/// Request to update an agent
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct UpdateAgentRequest {
    /// Agent display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    
    /// Short description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// LLM provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    
    /// Model ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    /// Temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    /// Max tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    
    /// Conversation starters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starters: Option<Vec<String>>,
    
    /// Tool source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_source: Option<String>,
    
    /// Custom OpenAPI URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_openapi_url: Option<String>,
    
    /// Enabled tool IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_tools: Option<Vec<String>>,
    
    /// Tool description overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_overrides: Option<HashMap<String, ToolOverride>>,
}

/// Response for listing agents
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgentsListResponse {
    /// List of agents
    pub agents: Vec<CustomAgent>,
    
    /// Total count
    pub count: usize,
}

/// Response for create/update operations
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AgentResponse {
    /// The agent
    pub agent: CustomAgent,
}

/// Error type for agent operations
#[derive(Debug, thiserror::Error)]
pub enum AgentStorageError {
    #[error("Agent not found: {0}")]
    NotFound(String),
    
    #[error("Cannot modify builtin agent: {0}")]
    CannotModifyBuiltin(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Invalid agent data: {0}")]
    InvalidData(String),
}

// Default value functions
fn default_provider() -> String { "gemini".to_string() }
fn default_model() -> String { "gemini-1.5-flash".to_string() }
fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 4096 }
fn default_tool_source() -> String { "builtin".to_string() }
fn default_icon() -> String { "🤖".to_string() }
fn default_enabled_tools() -> Vec<String> { vec!["*".to_string()] }

/// Get the storage directory for agents
fn get_agents_dir() -> Result<PathBuf, AgentStorageError> {
    let dir = dirs::data_dir()
        .ok_or_else(|| AgentStorageError::StorageError("Could not find data directory".to_string()))?
        .join("dev.gemfoundry.oracle-xray")
        .join("agents");
    
    // Create directory if it doesn't exist
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| AgentStorageError::StorageError(format!("Failed to create agents directory: {}", e)))?;
    }
    
    Ok(dir)
}

/// Get the path to an agent's JSON file
fn get_agent_path(id: &str) -> Result<PathBuf, AgentStorageError> {
    let dir = get_agents_dir()?;
    Ok(dir.join(format!("{}.json", id)))
}

/// Create the default "Generic Chat" agent (no database tools - pure Gemini chat)
fn create_builtin_agent() -> CustomAgent {
    let now = chrono::Utc::now().to_rfc3339();
    
    CustomAgent {
        id: "default".to_string(),
        name: "Generic Chat".to_string(),
        icon: "💬".to_string(),
        description: "General AI assistant without database tools".to_string(),
        provider: "gemini".to_string(),
        model: "gemini-1.5-flash".to_string(),
        temperature: 0.7,
        max_tokens: 4096,
        system_prompt: "You are a helpful AI assistant. Answer questions clearly and concisely.".to_string(),
        starters: vec![
            "Hello, how can you help me?".to_string(),
            "What can you do?".to_string(),
            "Tell me about yourself".to_string(),
        ],
        tool_source: "none".to_string(),
        custom_openapi_url: None,
        enabled_tools: vec![],  // No tools - pure Gemini chat
        tool_overrides: HashMap::new(),
        created_at: now.clone(),
        updated_at: now,
        is_builtin: true,
    }
}

/// Initialize the agents directory with builtin agents
pub fn init_agents() -> Result<(), AgentStorageError> {
    let builtin_path = get_agent_path("default")?;
    
    // Create builtin agent if it doesn't exist
    if !builtin_path.exists() {
        let builtin = create_builtin_agent();
        save_agent(&builtin)?;
        tracing::info!("Created builtin 'Generic Chat' agent");
    }
    
    Ok(())
}

/// List all agents
pub fn list_agents() -> Result<Vec<CustomAgent>, AgentStorageError> {
    let dir = get_agents_dir()?;
    let mut agents = Vec::new();
    
    // Read all JSON files in the directory
    for entry in fs::read_dir(&dir)
        .map_err(|e| AgentStorageError::StorageError(format!("Failed to read agents directory: {}", e)))?
    {
        let entry = entry
            .map_err(|e| AgentStorageError::StorageError(format!("Failed to read directory entry: {}", e)))?;
        
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<CustomAgent>(&content) {
                        Ok(agent) => agents.push(agent),
                        Err(e) => {
                            tracing::warn!("Failed to parse agent file {:?}: {}", path, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read agent file {:?}: {}", path, e);
                }
            }
        }
    }
    
    // Sort: builtin first, then by name
    agents.sort_by(|a, b| {
        match (a.is_builtin, b.is_builtin) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    
    Ok(agents)
}

/// Get a single agent by ID
pub fn get_agent(id: &str) -> Result<CustomAgent, AgentStorageError> {
    let path = get_agent_path(id)?;
    
    if !path.exists() {
        return Err(AgentStorageError::NotFound(id.to_string()));
    }
    
    let content = fs::read_to_string(&path)
        .map_err(|e| AgentStorageError::StorageError(format!("Failed to read agent file: {}", e)))?;
    
    serde_json::from_str(&content)
        .map_err(|e| AgentStorageError::InvalidData(format!("Failed to parse agent: {}", e)))
}

/// Save an agent to disk
fn save_agent(agent: &CustomAgent) -> Result<(), AgentStorageError> {
    let path = get_agent_path(&agent.id)?;
    
    let content = serde_json::to_string_pretty(agent)
        .map_err(|e| AgentStorageError::InvalidData(format!("Failed to serialize agent: {}", e)))?;
    
    fs::write(&path, content)
        .map_err(|e| AgentStorageError::StorageError(format!("Failed to write agent file: {}", e)))?;
    
    Ok(())
}

/// Create a new agent
pub fn create_agent(request: CreateAgentRequest) -> Result<CustomAgent, AgentStorageError> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    
    let agent = CustomAgent {
        id,
        name: request.name,
        icon: request.icon,
        description: request.description,
        provider: request.provider,
        model: request.model,
        temperature: request.temperature.clamp(0.0, 1.0),
        max_tokens: request.max_tokens,
        system_prompt: request.system_prompt,
        starters: request.starters,
        tool_source: request.tool_source,
        custom_openapi_url: request.custom_openapi_url,
        enabled_tools: request.enabled_tools,
        tool_overrides: request.tool_overrides,
        created_at: now.clone(),
        updated_at: now,
        is_builtin: false,
    };
    
    save_agent(&agent)?;
    tracing::info!("Created custom agent: {} ({})", agent.name, agent.id);
    
    Ok(agent)
}

/// Update an existing agent
pub fn update_agent(id: &str, request: UpdateAgentRequest) -> Result<CustomAgent, AgentStorageError> {
    let mut agent = get_agent(id)?;
    
    // Cannot modify builtin agents (except some fields)
    if agent.is_builtin {
        // For builtin agents, only allow updating starters
        if let Some(starters) = request.starters {
            agent.starters = starters;
        }
        agent.updated_at = chrono::Utc::now().to_rfc3339();
        save_agent(&agent)?;
        return Ok(agent);
    }
    
    // Update fields
    if let Some(name) = request.name {
        agent.name = name;
    }
    if let Some(icon) = request.icon {
        agent.icon = icon;
    }
    if let Some(description) = request.description {
        agent.description = description;
    }
    if let Some(provider) = request.provider {
        agent.provider = provider;
    }
    if let Some(model) = request.model {
        agent.model = model;
    }
    if let Some(temperature) = request.temperature {
        agent.temperature = temperature.clamp(0.0, 1.0);
    }
    if let Some(max_tokens) = request.max_tokens {
        agent.max_tokens = max_tokens;
    }
    if let Some(system_prompt) = request.system_prompt {
        agent.system_prompt = system_prompt;
    }
    if let Some(starters) = request.starters {
        agent.starters = starters;
    }
    if let Some(tool_source) = request.tool_source {
        agent.tool_source = tool_source;
    }
    if let Some(custom_openapi_url) = request.custom_openapi_url {
        agent.custom_openapi_url = Some(custom_openapi_url);
    }
    if let Some(enabled_tools) = request.enabled_tools {
        agent.enabled_tools = enabled_tools;
    }
    if let Some(tool_overrides) = request.tool_overrides {
        agent.tool_overrides = tool_overrides;
    }
    
    agent.updated_at = chrono::Utc::now().to_rfc3339();
    save_agent(&agent)?;
    
    tracing::info!("Updated agent: {} ({})", agent.name, agent.id);
    Ok(agent)
}

/// Delete an agent
pub fn delete_agent(id: &str) -> Result<(), AgentStorageError> {
    let agent = get_agent(id)?;
    
    if agent.is_builtin {
        return Err(AgentStorageError::CannotModifyBuiltin(id.to_string()));
    }
    
    let path = get_agent_path(id)?;
    fs::remove_file(&path)
        .map_err(|e| AgentStorageError::StorageError(format!("Failed to delete agent file: {}", e)))?;
    
    tracing::info!("Deleted agent: {} ({})", agent.name, id);
    Ok(())
}

/// Resolve template variables in a system prompt
pub fn resolve_template_variables(
    prompt: &str,
    connection_name: Option<&str>,
    connection_user: Option<&str>,
    schemas: Option<&[String]>,
) -> String {
    let mut result = prompt.to_string();
    
    // Replace {{CONNECTION_NAME}}
    result = result.replace(
        "{{CONNECTION_NAME}}",
        connection_name.unwrap_or("Not connected"),
    );
    
    // Replace {{CONNECTION_USER}}
    result = result.replace(
        "{{CONNECTION_USER}}",
        connection_user.unwrap_or("Unknown"),
    );
    
    // Replace {{SCHEMAS}}
    let schemas_str = schemas
        .map(|s| s.join(", "))
        .unwrap_or_else(|| "None available".to_string());
    result = result.replace("{{SCHEMAS}}", &schemas_str);
    
    // Replace {{TIMESTAMP}}
    result = result.replace(
        "{{TIMESTAMP}}",
        &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    );
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolve_template_variables() {
        let prompt = "Connection: {{CONNECTION_NAME}}, User: {{CONNECTION_USER}}, Schemas: {{SCHEMAS}}";
        let result = resolve_template_variables(
            prompt,
            Some("ORADEV"),
            Some("HR"),
            Some(&["HR".to_string(), "SALES".to_string()]),
        );
        
        assert!(result.contains("ORADEV"));
        assert!(result.contains("HR"));
        assert!(result.contains("HR, SALES"));
    }
    
    #[test]
    fn test_default_values() {
        assert_eq!(default_provider(), "gemini");
        assert_eq!(default_model(), "gemini-1.5-flash");
        assert_eq!(default_temperature(), 0.7);
        assert_eq!(default_max_tokens(), 4096);
    }
}

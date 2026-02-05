//! Agent API handlers
//!
//! This module implements the REST API handlers for the agent system.
//! These endpoints are GPT-safe (read-only operations).

use crate::agent::custom_agents::{
    self, AgentsListResponse, CreateAgentRequest, CustomAgent, UpdateAgentRequest,
    AgentResponse as CustomAgentResponse,
};
use crate::agent::executor::ToolExecutor;
use crate::agent::gemini::{self, GeminiProvider};
use crate::agent::models::{self, get_default_model, ModelsResponse};
use crate::agent::provider::AgentProvider;
use crate::agent::tools::get_tool_definitions;
use crate::agent::types::{AgentError, AgentRequest, AgentResponse};
use crate::api::middleware::AuthInfo;
use crate::server::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use serde::Serialize;
use std::sync::Arc;

// ============================================================================
// Response Types
// ============================================================================

/// Agent status response
#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentStatusResponse {
    /// Whether the agent is configured (has API key)
    pub configured: bool,
    
    /// Available providers
    pub providers: Vec<ProviderStatus>,
    
    /// Default model ID
    pub default_model: String,
}

/// Provider status
#[derive(Serialize, utoipa::ToSchema)]
pub struct ProviderStatus {
    /// Provider name
    pub name: String,
    
    /// Whether this provider is configured
    pub configured: bool,
    
    /// Number of available models
    pub model_count: usize,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get agent status
/// 
/// Check if the agent is configured and ready to use.
#[utoipa::path(
    get,
    path = "/agent/status",
    responses(
        (status = 200, description = "Agent status", body = AgentStatusResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agent"
)]
pub async fn agent_status_handler(
    Extension(_auth): Extension<AuthInfo>,
) -> Json<AgentStatusResponse> {
    let gemini_configured = gemini::is_configured();
    // Use cached models count
    let gemini_models = gemini::get_models().len();
    
    Json(AgentStatusResponse {
        configured: models::any_provider_configured(),
        providers: vec![
            ProviderStatus {
                name: "google".to_string(),
                configured: gemini_configured,
                model_count: if gemini_configured { gemini_models } else { 0 },
            },
            // OpenAI provider will be added here when implemented
        ],
        default_model: get_default_model(),
    })
}

/// List available models
/// 
/// Get a list of all available LLM models for the agent.
/// Returns model IDs, names, providers, and availability status.
/// 
/// **Note**: Models are fetched from Google API on startup and cached for 24 hours.
/// If cache is empty, this endpoint will wait for the API fetch to complete.
#[utoipa::path(
    get,
    path = "/agent/models",
    responses(
        (status = 200, description = "List of available models", body = ModelsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agent"
)]
pub async fn agent_models_handler(
    Extension(_auth): Extension<AuthInfo>,
) -> Json<ModelsResponse> {
    // Check if we need to fetch from API
    let needs_refresh = gemini::cache_needs_refresh() && gemini::is_configured();
    
    let all_models = if needs_refresh {
        // Wait for API fetch (first request after startup or cache expiry)
        tracing::debug!("Fetching models from API...");
        match gemini::refresh_models().await {
            Ok(_) => {
                tracing::debug!("Models fetched successfully");
                models::get_available_models()
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models from API: {}", e);
                // Return fallback
                models::get_available_models()
            }
        }
    } else {
        // Return cached models
        models::get_available_models()
    };
    
    Json(ModelsResponse {
        count: all_models.len(),
        default_model: get_default_model(),
        models: all_models,
        cache_refreshed: if needs_refresh { Some(true) } else { None },
    })
}

/// Ask the agent a question
/// 
/// Send a natural language question to the agent. The agent will use
/// available tools (schema exploration, query execution) to answer.
/// 
/// **Request fields:**
/// - `question` (required): The natural language question
/// - `model_id` (optional): Model to use (default: gemini-1.5-flash)
/// - `session_id` (optional): Session ID for conversation continuity
/// - `connection_id` (optional): Database connection to use
/// 
/// **Response includes:**
/// - `answer`: The agent's response
/// - `sources`: Citations for data used
/// - `trace`: Step-by-step reasoning trace
/// - `tokens_used`: Token usage statistics
#[utoipa::path(
    post,
    path = "/agent/ask",
    request_body = AgentRequest,
    responses(
        (status = 200, description = "Agent response", body = AgentResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Agent not configured", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agent"
)]
pub async fn agent_ask_handler(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthInfo>,
    Json(request): Json<AgentRequest>,
) -> Result<Json<AgentResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check if any provider is configured
    if !models::any_provider_configured() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Agent is not configured. Please set GEMINI_API_KEY in your environment.".to_string(),
                code: "AGENT_NOT_CONFIGURED".to_string(),
            }),
        ));
    }
    
    // Validate question
    if request.question.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Question cannot be empty".to_string(),
                code: "EMPTY_QUESTION".to_string(),
            }),
        ));
    }
    
    tracing::info!(
        "Agent request from {} (model: {:?}, connection: {:?})",
        auth.caller_type,
        request.model_id,
        request.connection_id
    );
    
    // Determine which provider to use based on model_id
    let default_model = get_default_model();
    let model_id = request.model_id.as_deref().unwrap_or(&default_model);
    
    // Create the appropriate provider based on model
    let provider: Box<dyn AgentProvider> = if gemini::handles_model(model_id) {
        let api_key = gemini::get_api_key().ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "Gemini API key not configured".to_string(),
                    code: "GEMINI_NOT_CONFIGURED".to_string(),
                }),
            )
        })?;
        Box::new(GeminiProvider::new(&api_key, 60)) // 60 second timeout
    } else if model_id.starts_with("gpt") {
        // OpenAI support coming soon
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(ErrorResponse {
                error: "OpenAI provider not yet implemented".to_string(),
                code: "NOT_IMPLEMENTED".to_string(),
            }),
        ));
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Unknown model: {}", model_id),
                code: "UNKNOWN_MODEL".to_string(),
            }),
        ));
    };
    
    // Create tool executor with connection context
    let executor = ToolExecutor::with_connection_id(
        state.clone(),
        request.connection_id.clone(),
    );
    
    // Get tool definitions
    let tools = get_tool_definitions();
    
    // Run the agent (max 10 iterations)
    let response = provider
        .run(&request, &tools, &executor, 10)
        .await
        .map_err(|e| map_agent_error(e))?;
    
    tracing::info!(
        "Agent completed (tokens: {:?}, sources: {})",
        response.tokens_used.as_ref().map(|t| t.total_tokens),
        response.sources.len()
    );
    
    Ok(Json(response))
}

/// Map agent errors to HTTP responses
fn map_agent_error(error: AgentError) -> (StatusCode, Json<ErrorResponse>) {
    let (status, code) = match &error {
        AgentError::ProviderError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "PROVIDER_ERROR"),
        AgentError::ApiError(_) => (StatusCode::BAD_GATEWAY, "API_ERROR"),
        AgentError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
        AgentError::ToolError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "TOOL_ERROR"),
        AgentError::ConfigError(_) => (StatusCode::SERVICE_UNAVAILABLE, "CONFIG_ERROR"),
        AgentError::Timeout => (StatusCode::GATEWAY_TIMEOUT, "TIMEOUT"),
        AgentError::MaxIterationsExceeded => (StatusCode::INTERNAL_SERVER_ERROR, "MAX_ITERATIONS"),
    };
    
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
            code: code.to_string(),
        }),
    )
}

// ============================================================================
// Custom Agent CRUD Handlers
// ============================================================================

/// List all custom agents
/// 
/// Get a list of all configured custom agents, including the built-in "Generic Chat" agent.
#[utoipa::path(
    get,
    path = "/agents",
    responses(
        (status = 200, description = "List of agents", body = AgentsListResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Storage error", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agents"
)]
pub async fn list_agents_handler(
    Extension(_auth): Extension<AuthInfo>,
) -> Result<Json<AgentsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let agents = custom_agents::list_agents().map_err(map_storage_error)?;
    
    Ok(Json(AgentsListResponse {
        count: agents.len(),
        agents,
    }))
}

/// Get a single custom agent
/// 
/// Get the configuration for a specific agent by ID.
#[utoipa::path(
    get,
    path = "/agents/{id}",
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Agent configuration", body = CustomAgent),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agents"
)]
pub async fn get_agent_handler(
    Extension(_auth): Extension<AuthInfo>,
    Path(id): Path<String>,
) -> Result<Json<CustomAgent>, (StatusCode, Json<ErrorResponse>)> {
    let agent = custom_agents::get_agent(&id).map_err(map_storage_error)?;
    Ok(Json(agent))
}

/// Create a new custom agent
/// 
/// Create a new custom agent with the specified configuration.
#[utoipa::path(
    post,
    path = "/agents",
    request_body = CreateAgentRequest,
    responses(
        (status = 201, description = "Agent created", body = CustomAgentResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Storage error", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agents"
)]
pub async fn create_agent_handler(
    Extension(_auth): Extension<AuthInfo>,
    Json(request): Json<CreateAgentRequest>,
) -> Result<(StatusCode, Json<CustomAgentResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate required fields
    if request.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Agent name cannot be empty".to_string(),
                code: "INVALID_NAME".to_string(),
            }),
        ));
    }
    
    let agent = custom_agents::create_agent(request).map_err(map_storage_error)?;
    
    Ok((
        StatusCode::CREATED,
        Json(CustomAgentResponse { agent }),
    ))
}

/// Update an existing custom agent
/// 
/// Update the configuration for an existing agent.
/// Built-in agents can only have their conversation starters updated.
#[utoipa::path(
    put,
    path = "/agents/{id}",
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    request_body = UpdateAgentRequest,
    responses(
        (status = 200, description = "Agent updated", body = CustomAgentResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agent not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agents"
)]
pub async fn update_agent_handler(
    Extension(_auth): Extension<AuthInfo>,
    Path(id): Path<String>,
    Json(request): Json<UpdateAgentRequest>,
) -> Result<Json<CustomAgentResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate name if provided
    if let Some(ref name) = request.name {
        if name.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Agent name cannot be empty".to_string(),
                    code: "INVALID_NAME".to_string(),
                }),
            ));
        }
    }
    
    let agent = custom_agents::update_agent(&id, request).map_err(map_storage_error)?;
    Ok(Json(CustomAgentResponse { agent }))
}

/// Delete a custom agent
/// 
/// Delete a custom agent by ID. Built-in agents cannot be deleted.
#[utoipa::path(
    delete,
    path = "/agents/{id}",
    params(
        ("id" = String, Path, description = "Agent ID")
    ),
    responses(
        (status = 204, description = "Agent deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Cannot delete built-in agent", body = ErrorResponse),
        (status = 404, description = "Agent not found", body = ErrorResponse),
        (status = 500, description = "Storage error", body = ErrorResponse)
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "agents"
)]
pub async fn delete_agent_handler(
    Extension(_auth): Extension<AuthInfo>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    custom_agents::delete_agent(&id).map_err(map_storage_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Map agent storage errors to HTTP responses
fn map_storage_error(error: custom_agents::AgentStorageError) -> (StatusCode, Json<ErrorResponse>) {
    use custom_agents::AgentStorageError;
    
    let (status, code) = match &error {
        AgentStorageError::NotFound(_) => (StatusCode::NOT_FOUND, "AGENT_NOT_FOUND"),
        AgentStorageError::CannotModifyBuiltin(_) => (StatusCode::FORBIDDEN, "CANNOT_MODIFY_BUILTIN"),
        AgentStorageError::StorageError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR"),
        AgentStorageError::InvalidData(_) => (StatusCode::BAD_REQUEST, "INVALID_DATA"),
    };
    
    (
        status,
        Json(ErrorResponse {
            error: error.to_string(),
            code: code.to_string(),
        }),
    )
}

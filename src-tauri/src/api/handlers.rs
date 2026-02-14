use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::{AccessLogEntry, InferenceLogEntry, AppState};
use std::time::Instant;

// ============ Gemini API Types ============

/// Request body for chat endpoint
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    /// The user's message to send to Gemini
    pub message: String,
    /// Optional conversation history for context
    #[serde(default)]
    pub history: Vec<ChatMessage>,
    /// Optional model to use (defaults to "gemini-2.0-flash")
    #[serde(default)]
    pub model: Option<String>,
}

/// A single chat message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChatMessage {
    /// Role of the message sender: "user" or "model"
    pub role: String,
    /// The content of the message
    pub content: String,
}

/// Response from the chat endpoint
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ChatResponse {
    /// The AI's response message
    pub response: String,
    /// The updated conversation history
    pub history: Vec<ChatMessage>,
}

/// Gemini API request structures
#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

/// Gemini API response structures
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
}

// ============ Response Types ============

/// Health check response
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
}

/// Jira issue summary for list endpoint
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JiraIssueSummary {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub status_category: String,
    pub assignee: Option<String>,
    pub priority: String,
    pub updated: String,
}

/// Response for jira/list endpoint
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JiraListResponse {
    pub issues: Vec<JiraIssueSummary>,
    pub total: i32,
    pub jql: String,
}

/// Error response
#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

// ============ Gemini Models Types ============

/// A single Gemini model
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeminiModel {
    /// The resource name of the model (e.g., "models/gemini-2.0-flash")
    pub name: String,
    /// The base model ID (e.g., "models/gemini-2.0-flash")
    #[serde(default)]
    pub base_model_id: Option<String>,
    /// Display name of the model
    #[serde(default)]
    pub display_name: Option<String>,
    /// Description of what the model does
    #[serde(default)]
    pub description: Option<String>,
    /// Version of the model
    #[serde(default)]
    pub version: Option<String>,
    /// Maximum input tokens the model supports
    #[serde(default)]
    pub input_token_limit: Option<u32>,
    /// Maximum output tokens the model can generate
    #[serde(default)]
    pub output_token_limit: Option<u32>,
    /// Supported generation methods (e.g., ["generateContent", "countTokens"])
    #[serde(default)]
    pub supported_generation_methods: Vec<String>,
    /// Temperature range supported by the model
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Top-P value for sampling
    #[serde(default)]
    pub top_p: Option<f32>,
    /// Top-K value for sampling
    #[serde(default)]
    pub top_k: Option<u32>,
}

/// Response from the list models endpoint
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeminiModelsResponse {
    /// List of available models
    pub models: Vec<GeminiModel>,
    /// Total count of models
    pub total: usize,
}

/// Internal struct for parsing Gemini API models list response
#[derive(Debug, Deserialize)]
struct GeminiModelsApiResponse {
    models: Option<Vec<GeminiModel>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

// ============ Request Types ============

/// Query parameters for jira/list endpoint
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct JiraListQuery {
    /// JQL query string (optional, defaults to "assignee = currentUser() ORDER BY updated DESC")
    pub jql: Option<String>,
    /// Maximum number of results (optional, defaults to 100)
    pub max_results: Option<u32>,
}

// ============ Handlers ============

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service healthy", body = HealthResponse)
    ),
    tag = "system"
)]
pub async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: state.uptime_secs(),
    })
}

/// List Jira issues
/// 
/// Returns a list of Jira issues based on the provided JQL query.
/// If no JQL is provided, defaults to showing issues assigned to the current user.
/// 
/// This endpoint is suitable for AI agent tool use.
#[utoipa::path(
    get,
    path = "/jira/list",
    params(JiraListQuery),
    responses(
        (status = 200, description = "List of Jira issues", body = JiraListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tags = ["jira", "tool", "gpt"]
)]
pub async fn jira_list_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<JiraListQuery>,
) -> Result<Json<JiraListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let jql = params
        .jql
        .unwrap_or_else(|| "assignee = currentUser() ORDER BY updated DESC".to_string());
    let max_results = params.max_results.unwrap_or(100);

    log::info!("REST API: jira/list called with JQL: {}", jql);

    let client = state.create_jira_client();

    match client.search_issues(&jql, max_results).await {
        Ok(result) => {
            log::info!("REST API: Found {} issues", result.issues.len());
            
            // Convert from jira::IssueSummary to our API response type
            let issues: Vec<JiraIssueSummary> = result
                .issues
                .into_iter()
                .map(|issue| JiraIssueSummary {
                    key: issue.key,
                    summary: issue.summary,
                    status: issue.status,
                    status_category: issue.status_category,
                    assignee: issue.assignee,
                    priority: issue.priority,
                    updated: issue.updated,
                })
                .collect();

            Ok(Json(JiraListResponse {
                issues,
                total: result.total,
                jql,
            }))
        }
        Err(e) => {
            log::error!("REST API: Jira search failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e,
                    code: 500,
                }),
            ))
        }
    }
}

/// Response for access logs endpoint
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessLogsResponse {
    pub logs: Vec<AccessLogEntry>,
    pub total: usize,
}

/// Get access logs
/// 
/// Returns a list of all HTTP access log entries.
#[utoipa::path(
    get,
    path = "/access-logs",
    responses(
        (status = 200, description = "Access log entries", body = AccessLogsResponse)
    ),
    tag = "system"
)]
pub async fn access_logs_handler(State(state): State<Arc<AppState>>) -> Json<AccessLogsResponse> {
    let logs = state.get_access_logs();
    let total = logs.len();
    Json(AccessLogsResponse { logs, total })
}

/// Clear access logs
/// 
/// Clears all HTTP access log entries.
#[utoipa::path(
    delete,
    path = "/access-logs",
    responses(
        (status = 200, description = "Access logs cleared")
    ),
    tag = "system"
)]
pub async fn clear_access_logs_handler(State(state): State<Arc<AppState>>) -> StatusCode {
    state.clear_access_logs();
    log::info!("REST API: Access logs cleared");
    StatusCode::OK
}

// ============ Inference Log Handlers ============

/// Response for inference logs endpoint
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InferenceLogsResponse {
    pub logs: Vec<InferenceLogEntry>,
    pub total: usize,
}

/// Get inference logs
/// 
/// Returns a list of all AI inference log entries (Gemini API calls).
#[utoipa::path(
    get,
    path = "/inference-logs",
    responses(
        (status = 200, description = "Inference log entries", body = InferenceLogsResponse)
    ),
    tag = "system"
)]
pub async fn inference_logs_handler(State(state): State<Arc<AppState>>) -> Json<InferenceLogsResponse> {
    let logs = state.get_inference_logs();
    let total = logs.len();
    Json(InferenceLogsResponse { logs, total })
}

/// Clear inference logs
/// 
/// Clears all AI inference log entries.
#[utoipa::path(
    delete,
    path = "/inference-logs",
    responses(
        (status = 200, description = "Inference logs cleared")
    ),
    tag = "system"
)]
pub async fn clear_inference_logs_handler(State(state): State<Arc<AppState>>) -> StatusCode {
    state.clear_inference_logs();
    log::info!("REST API: Inference logs cleared");
    StatusCode::OK
}

// ============ Agent/Chat Handlers ============

/// Chat with Gemini AI
/// 
/// Sends a message to Google Gemini and returns the AI response.
/// Supports conversation history for multi-turn conversations.
#[utoipa::path(
    post,
    path = "/agent/chat",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Chat response from Gemini", body = ChatResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "agent"
)]
pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start_time = Instant::now();
    let model = request.model.as_deref().unwrap_or("gemini-2.0-flash");
    let user_message_preview: String = request.message.chars().take(100).collect();
    
    log::info!("REST API: agent/chat called with model: {}, message: {}...", 
        model, &request.message.chars().take(50).collect::<String>());

    // Check if Gemini API key is configured
    if state.gemini_api_key.is_empty() || state.gemini_api_key == "YOUR_GEMINI_API_KEY_HERE" {
        // Log failed inference attempt
        state.add_inference_log(
            "gemini".to_string(),
            model.to_string(),
            "chat".to_string(),
            false,
            Some(400),
            start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some("Gemini API key not configured".to_string()),
            None,
            Some(user_message_preview),
            None,
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Gemini API key not configured. Please set GEMINI_API_KEY in .env file.".to_string(),
                code: 400,
            }),
        ));
    }

    // Build conversation contents for Gemini API
    let mut contents: Vec<GeminiContent> = request
        .history
        .iter()
        .map(|msg| GeminiContent {
            role: msg.role.clone(),
            parts: vec![GeminiPart { text: msg.content.clone() }],
        })
        .collect();

    // Add the current user message
    contents.push(GeminiContent {
        role: "user".to_string(),
        parts: vec![GeminiPart { text: request.message.clone() }],
    });

    let gemini_request = GeminiRequest { contents };

    // Call Gemini API
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, state.gemini_api_key
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&gemini_request)
        .send()
        .await
        .map_err(|e| {
            log::error!("REST API: Failed to call Gemini API: {}", e);
            // Log failed inference
            state.add_inference_log(
                "gemini".to_string(),
                model.to_string(),
                "chat".to_string(),
                false,
                None,
                start_time.elapsed().as_millis() as u64,
                None, None, None,
                Some(format!("HTTP error: {}", e)),
                None,
                Some(user_message_preview.clone()),
                None,
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to call Gemini API: {}", e),
                    code: 500,
                }),
            )
        })?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        log::error!("REST API: Failed to read Gemini response: {}", e);
        // Log failed inference
        state.add_inference_log(
            "gemini".to_string(),
            model.to_string(),
            "chat".to_string(),
            false,
            Some(status.as_u16()),
            start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(format!("Failed to read response: {}", e)),
            None,
            Some(user_message_preview.clone()),
            None,
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read Gemini response: {}", e),
                code: 500,
            }),
        )
    })?;

    if !status.is_success() {
        log::error!("REST API: Gemini API error ({}): {}", status, response_text);
        // Log failed inference
        state.add_inference_log(
            "gemini".to_string(),
            model.to_string(),
            "chat".to_string(),
            false,
            Some(status.as_u16()),
            start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(format!("API error: {}", response_text)),
            None,
            Some(user_message_preview),
            None,
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Gemini API error: {}", response_text),
                code: status.as_u16(),
            }),
        ));
    }

    let gemini_response: GeminiResponse = serde_json::from_str(&response_text).map_err(|e| {
        log::error!("REST API: Failed to parse Gemini response: {}", e);
        // Log failed inference
        state.add_inference_log(
            "gemini".to_string(),
            model.to_string(),
            "chat".to_string(),
            false,
            Some(status.as_u16()),
            start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(format!("Failed to parse response: {}", e)),
            None,
            Some(user_message_preview.clone()),
            None,
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse Gemini response: {}", e),
                code: 500,
            }),
        )
    })?;

    // Check for API error in response
    if let Some(error) = gemini_response.error {
        log::error!("REST API: Gemini API returned error: {}", error.message);
        // Log failed inference
        state.add_inference_log(
            "gemini".to_string(),
            model.to_string(),
            "chat".to_string(),
            false,
            Some(status.as_u16()),
            start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(error.message.clone()),
            None,
            Some(user_message_preview),
            None,
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: error.message,
                code: 500,
            }),
        ));
    }

    // Extract the response text
    let ai_response = gemini_response
        .candidates
        .and_then(|c| c.into_iter().next())
        .map(|c| c.content.parts.into_iter().map(|p| p.text).collect::<Vec<_>>().join(""))
        .unwrap_or_else(|| "No response from Gemini".to_string());

    let duration_ms = start_time.elapsed().as_millis() as u64;
    log::info!("REST API: Gemini responded with {} chars in {}ms", ai_response.len(), duration_ms);

    // Log successful inference with full details
    state.add_inference_log(
        "gemini".to_string(),
        model.to_string(),
        "chat".to_string(),
        true,
        Some(200),
        duration_ms,
        None, None, None, // Token counts not available from simple API
        None,
        None, // No system prompt in this simple chat
        Some(user_message_preview),
        Some(serde_json::json!({
            "question": request.message,
            "response": ai_response.clone(),
            "response_length": ai_response.len(),
            "history_length": request.history.len(),
            "history": request.history.iter().map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content
            })).collect::<Vec<_>>(),
            "api_endpoint": format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model),
            "generation_config": {
                "temperature": null,
                "max_output_tokens": null
            }
        })),
    );

    // Build updated history
    let mut updated_history = request.history.clone();
    updated_history.push(ChatMessage {
        role: "user".to_string(),
        content: request.message,
    });
    updated_history.push(ChatMessage {
        role: "model".to_string(),
        content: ai_response.clone(),
    });

    Ok(Json(ChatResponse {
        response: ai_response,
        history: updated_history,
    }))
}

// ============ OpenAI API Types ============

/// Request body for OpenAI chat endpoint
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct OpenAIChatRequest {
    /// The user's message to send to OpenAI
    pub message: String,
    /// Optional conversation history for context
    #[serde(default)]
    pub history: Vec<ChatMessage>,
    /// Optional model to use (defaults to "gpt-4o-mini")
    #[serde(default)]
    pub model: Option<String>,
}

/// A single OpenAI model returned by the list endpoint
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIModel {
    /// The model identifier (e.g., "gpt-4o-mini")
    pub id: String,
    /// Object type (always "model")
    #[serde(default)]
    pub object: Option<String>,
    /// Unix timestamp of when the model was created
    #[serde(default)]
    pub created: Option<u64>,
    /// The organization that owns the model
    #[serde(default)]
    pub owned_by: Option<String>,
}

/// Response from the OpenAI models list endpoint
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OpenAIModelsResponse {
    /// List of available models
    pub models: Vec<OpenAIModel>,
    /// Total count of models
    pub total: usize,
}

/// Internal struct for parsing OpenAI API models list response
#[derive(Debug, Deserialize)]
struct OpenAIModelsApiResponse {
    data: Option<Vec<OpenAIModel>>,
}

/// OpenAI chat completion request structures
#[derive(Debug, Serialize)]
struct OpenAIChatCompletionRequest {
    model: String,
    messages: Vec<OpenAIChatCompletionMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct OpenAIChatCompletionMessage {
    role: String,
    content: String,
}

/// OpenAI chat completion response structures
#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionResponse {
    choices: Option<Vec<OpenAIChatCompletionChoice>>,
    usage: Option<OpenAIUsage>,
    error: Option<OpenAIErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionChoice {
    message: OpenAIChatCompletionChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatCompletionChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
}

// ============ OpenAI Handlers ============

/// Chat with OpenAI
/// 
/// Sends a message to OpenAI's chat completions API and returns the AI response.
/// Supports conversation history for multi-turn conversations.
#[utoipa::path(
    post,
    path = "/agent/openai/chat",
    request_body = OpenAIChatRequest,
    responses(
        (status = 200, description = "Chat response from OpenAI", body = ChatResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "agent"
)]
pub async fn openai_chat_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OpenAIChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start_time = Instant::now();
    let model = request.model.as_deref().unwrap_or("gpt-4o-mini");
    let user_message_preview: String = request.message.chars().take(100).collect();

    log::info!("REST API: agent/openai/chat called with model: {}, message: {}...",
        model, &request.message.chars().take(50).collect::<String>());

    // Check if OpenAI API key is configured
    if state.openai_api_key.is_empty() {
        state.add_inference_log(
            "openai".to_string(), model.to_string(), "chat".to_string(),
            false, Some(400), start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some("OpenAI API key not configured".to_string()),
            None, Some(user_message_preview), None,
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "OpenAI API key not configured. Please set OPENAI_API_KEY in .env file.".to_string(),
                code: 400,
            }),
        ));
    }

    // Build messages array for OpenAI chat completions API
    let mut messages: Vec<OpenAIChatCompletionMessage> = request
        .history
        .iter()
        .map(|msg| OpenAIChatCompletionMessage {
            role: if msg.role == "model" { "assistant".to_string() } else { msg.role.clone() },
            content: msg.content.clone(),
        })
        .collect();

    // Add the current user message
    messages.push(OpenAIChatCompletionMessage {
        role: "user".to_string(),
        content: request.message.clone(),
    });

    let openai_request = OpenAIChatCompletionRequest {
        model: model.to_string(),
        messages,
        temperature: None,
        max_tokens: None,
    };

    // Call OpenAI API
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", state.openai_api_key))
        .json(&openai_request)
        .send()
        .await
        .map_err(|e| {
            log::error!("REST API: Failed to call OpenAI API: {}", e);
            state.add_inference_log(
                "openai".to_string(), model.to_string(), "chat".to_string(),
                false, None, start_time.elapsed().as_millis() as u64,
                None, None, None,
                Some(format!("HTTP error: {}", e)),
                None, Some(user_message_preview.clone()), None,
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to call OpenAI API: {}", e),
                    code: 500,
                }),
            )
        })?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        log::error!("REST API: Failed to read OpenAI response: {}", e);
        state.add_inference_log(
            "openai".to_string(), model.to_string(), "chat".to_string(),
            false, Some(status.as_u16()), start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(format!("Failed to read response: {}", e)),
            None, Some(user_message_preview.clone()), None,
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read OpenAI response: {}", e),
                code: 500,
            }),
        )
    })?;

    if !status.is_success() {
        log::error!("REST API: OpenAI API error ({}): {}", status, response_text);
        state.add_inference_log(
            "openai".to_string(), model.to_string(), "chat".to_string(),
            false, Some(status.as_u16()), start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(format!("API error: {}", response_text)),
            None, Some(user_message_preview), None,
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("OpenAI API error: {}", response_text),
                code: status.as_u16(),
            }),
        ));
    }

    let openai_response: OpenAIChatCompletionResponse = serde_json::from_str(&response_text).map_err(|e| {
        log::error!("REST API: Failed to parse OpenAI response: {}", e);
        state.add_inference_log(
            "openai".to_string(), model.to_string(), "chat".to_string(),
            false, Some(status.as_u16()), start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(format!("Failed to parse response: {}", e)),
            None, Some(user_message_preview.clone()), None,
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to parse OpenAI response: {}", e),
                code: 500,
            }),
        )
    })?;

    // Check for API error in response
    if let Some(error) = openai_response.error {
        log::error!("REST API: OpenAI API returned error: {}", error.message);
        state.add_inference_log(
            "openai".to_string(), model.to_string(), "chat".to_string(),
            false, Some(status.as_u16()), start_time.elapsed().as_millis() as u64,
            None, None, None,
            Some(error.message.clone()),
            None, Some(user_message_preview), None,
        );
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: error.message,
                code: 500,
            }),
        ));
    }

    // Extract the response text
    let ai_response = openai_response
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message.content)
        .unwrap_or_else(|| "No response from OpenAI".to_string());

    let duration_ms = start_time.elapsed().as_millis() as u64;
    log::info!("REST API: OpenAI responded with {} chars in {}ms", ai_response.len(), duration_ms);

    // Extract token usage
    let (prompt_tokens, completion_tokens, total_tokens) = openai_response
        .usage
        .map(|u| (u.prompt_tokens, u.completion_tokens, u.total_tokens))
        .unwrap_or((None, None, None));

    // Log successful inference
    state.add_inference_log(
        "openai".to_string(), model.to_string(), "chat".to_string(),
        true, Some(200), duration_ms,
        prompt_tokens, completion_tokens, total_tokens,
        None, None, Some(user_message_preview),
        Some(serde_json::json!({
            "response_length": ai_response.len(),
            "history_length": request.history.len(),
        })),
    );

    // Build updated history (use "model" role for frontend compatibility)
    let mut updated_history = request.history.clone();
    updated_history.push(ChatMessage {
        role: "user".to_string(),
        content: request.message,
    });
    updated_history.push(ChatMessage {
        role: "model".to_string(),
        content: ai_response.clone(),
    });

    Ok(Json(ChatResponse {
        response: ai_response,
        history: updated_history,
    }))
}

/// List available OpenAI models
/// 
/// Returns a list of available OpenAI models, filtered to chat-capable models.
#[utoipa::path(
    get,
    path = "/agent/openai/models",
    responses(
        (status = 200, description = "List of available OpenAI models", body = OpenAIModelsResponse),
        (status = 400, description = "Bad request - API key not configured", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "agent"
)]
pub async fn openai_list_models_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<OpenAIModelsResponse>, (StatusCode, Json<ErrorResponse>)> {
    log::info!("REST API: agent/openai/models called");

    // Check if OpenAI API key is configured
    if state.openai_api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "OpenAI API key not configured. Please set OPENAI_API_KEY in .env file.".to_string(),
                code: 400,
            }),
        ));
    }

    // Call OpenAI API to list models
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", state.openai_api_key))
        .send()
        .await
        .map_err(|e| {
            log::error!("REST API: Failed to call OpenAI models API: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to call OpenAI API: {}", e),
                    code: 500,
                }),
            )
        })?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        log::error!("REST API: Failed to read OpenAI models response: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Failed to read OpenAI response: {}", e), code: 500 }))
    })?;

    if !status.is_success() {
        log::error!("REST API: OpenAI models API error ({}): {}", status, response_text);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("OpenAI API error: {}", response_text), code: status.as_u16() })));
    }

    let api_response: OpenAIModelsApiResponse = serde_json::from_str(&response_text).map_err(|e| {
        log::error!("REST API: Failed to parse OpenAI models response: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Failed to parse OpenAI response: {}", e), code: 500 }))
    })?;

    let models = api_response.data.unwrap_or_default();
    let total = models.len();

    log::info!("REST API: Retrieved {} OpenAI models", total);

    Ok(Json(OpenAIModelsResponse { models, total }))
}

// ============ Gemini Models Handler ============

/// List available Gemini models
///
/// Returns a list of all available Google Gemini models.
#[utoipa::path(
    get,
    path = "/agent/models",
    responses(
        (status = 200, description = "List of available Gemini models", body = GeminiModelsResponse),
        (status = 400, description = "Bad request - API key not configured", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "agent"
)]
pub async fn list_models_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GeminiModelsResponse>, (StatusCode, Json<ErrorResponse>)> {
    log::info!("REST API: agent/models called");

    if state.gemini_api_key.is_empty() || state.gemini_api_key == "YOUR_GEMINI_API_KEY_HERE" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Gemini API key not configured. Please set GEMINI_API_KEY in .env file.".to_string(),
                code: 400,
            }),
        ));
    }

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        state.gemini_api_key
    );

    let response = client.get(&url).send().await.map_err(|e| {
        log::error!("REST API: Failed to call Gemini models API: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Failed to call Gemini API: {}", e), code: 500 }))
    })?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        log::error!("REST API: Failed to read Gemini models response: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Failed to read Gemini response: {}", e), code: 500 }))
    })?;

    if !status.is_success() {
        log::error!("REST API: Gemini models API error ({}): {}", status, response_text);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Gemini API error: {}", response_text), code: status.as_u16() })));
    }

    let api_response: GeminiModelsApiResponse = serde_json::from_str(&response_text).map_err(|e| {
        log::error!("REST API: Failed to parse Gemini models response: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Failed to parse Gemini response: {}", e), code: 500 }))
    })?;

    let models = api_response.models.unwrap_or_default();
    let total = models.len();
    log::info!("REST API: Retrieved {} Gemini models", total);

    Ok(Json(GeminiModelsResponse { models, total }))
}

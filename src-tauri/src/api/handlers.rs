use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::{AccessLogEntry, AppState};

// ============ Gemini API Types ============

/// Request body for chat endpoint
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ChatRequest {
    /// The user's message to send to Gemini
    pub message: String,
    /// Optional conversation history for context
    #[serde(default)]
    pub history: Vec<ChatMessage>,
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
    tag = "jira"
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
    log::info!("REST API: agent/chat called with message: {}...", 
        &request.message.chars().take(50).collect::<String>());

    // Check if Gemini API key is configured
    if state.gemini_api_key.is_empty() || state.gemini_api_key == "YOUR_GEMINI_API_KEY_HERE" {
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
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        state.gemini_api_key
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&gemini_request)
        .send()
        .await
        .map_err(|e| {
            log::error!("REST API: Failed to call Gemini API: {}", e);
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

    log::info!("REST API: Gemini responded with {} chars", ai_response.len());

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

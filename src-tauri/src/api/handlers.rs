use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::{AccessLogEntry, AppState};

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

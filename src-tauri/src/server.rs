use crate::api::{handlers, middleware::{auth_middleware, access_log_middleware}};
use crate::conversation_history;
use crate::latest;
use crate::openapi::{PublicApiDoc, AdminApiDoc};
use crate::shadow_git;
use crate::state::AppState;
use crate::tool_runtime::{self, ToolRuntime};
use axum::{middleware, response::Json, routing::{get, delete, post, put}, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

/// Create the Axum router with all routes
pub fn create_router(state: Arc<AppState>, tool_runtime: Arc<ToolRuntime>) -> Router {
    // CORS configuration - adjust for production
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/openapi.json", get(openapi_public_handler))
        .route("/openapi_admin.json", get(openapi_admin_handler))
        .route("/access-logs", get(handlers::access_logs_handler))
        .route("/access-logs", delete(handlers::clear_access_logs_handler))
        .route("/inference-logs", get(handlers::inference_logs_handler))
        .route("/inference-logs", delete(handlers::clear_inference_logs_handler));

    // Protected routes (require Bearer token auth)
    let protected_routes = Router::new()
        .route("/jira/list", get(handlers::jira_list_handler))
        .route("/agent/chat", post(handlers::chat_handler))
        .route("/agent/models", get(handlers::list_models_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Tool Runtime routes (Tools Console)
    let tool_routes = Router::new()
        .route("/tools", get(tool_runtime::list_tools_handler))
        .route("/tools/invoke", post(tool_runtime::invoke_tool_handler))
        .route("/tools/logs", get(tool_runtime::get_tool_logs_handler))
        .route("/tools/logs", delete(tool_runtime::clear_tool_logs_handler))
        .route("/tools/config", get(tool_runtime::get_config_handler))
        .route("/tools/config", put(tool_runtime::update_config_handler))
        .route("/tools/:operation_id/config", put(tool_runtime::configure_tool_handler))
        .route("/tools/circuit-breakers", get(tool_runtime::get_circuit_breakers_handler))
        .route("/tools/circuit-breakers", delete(tool_runtime::reset_circuit_breakers_handler))
        .route("/tools/:operation_id/circuit-breaker", delete(tool_runtime::reset_tool_circuit_breaker_handler))
        .route("/tools/fixtures", get(tool_runtime::get_fixtures_handler))
        .route("/tools/fixtures", post(tool_runtime::import_fixtures_handler))
        .route("/tools/fixtures", delete(tool_runtime::clear_fixtures_handler))
        .route("/tools/enable-all", post(tool_runtime::enable_all_tools_handler))
        .route("/tools/disable-all", post(tool_runtime::disable_all_tools_handler))
        .with_state(tool_runtime);

    // Shadow Git / Changes routes (protected)
    let changes_routes = Router::new()
        .route("/changes/workspaces", get(shadow_git::list_workspaces_handler))
        .route("/changes/tasks", get(shadow_git::list_tasks_handler))
        .route("/changes/tasks/:task_id/diff", get(shadow_git::task_diff_handler))
        .route("/changes/tasks/:task_id/steps", get(shadow_git::list_steps_handler))
        .route("/changes/tasks/:task_id/steps/:index/diff", get(shadow_git::step_diff_handler))
        .route("/changes/tasks/:task_id/subtasks/:subtask_index/diff", get(shadow_git::subtask_diff_handler))
        .route("/changes/workspaces/:id/nuke", post(shadow_git::nuke_workspace_handler))
        .route("/changes/file-contents", post(shadow_git::file_contents_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Latest composite route (protected)
    let latest_routes = Router::new()
        .route("/latest", get(latest::get_latest_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Conversation History routes (protected)
    let history_routes = Router::new()
        .route("/history/tasks", get(conversation_history::list_history_tasks_handler))
        .route("/history/stats", get(conversation_history::get_history_stats_handler))
        .route("/history/tasks/:task_id", get(conversation_history::get_task_detail_handler))
        .route("/history/tasks/:task_id/messages", get(conversation_history::get_task_messages_handler))
        .route("/history/tasks/:task_id/messages/:index", get(conversation_history::get_single_message_handler))
        .route("/history/tasks/:task_id/tools", get(conversation_history::get_task_tools_handler))
        .route("/history/tasks/:task_id/thinking", get(conversation_history::get_task_thinking_handler))
        .route("/history/tasks/:task_id/files", get(conversation_history::get_task_files_handler))
        .route("/history/tasks/:task_id/subtasks", get(conversation_history::get_task_subtasks_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(tool_routes)
        .merge(changes_routes)
        .merge(latest_routes)
        .merge(history_routes)
        // Add access logging middleware to all routes
        .layer(middleware::from_fn_with_state(state.clone(), access_log_middleware))
        .layer(cors)
        .with_state(state)
}

/// Serve public OpenAPI spec as JSON
/// 
/// This is the standard `/openapi.json` endpoint that external API tools
/// will auto-discover. It contains only public API endpoints.
async fn openapi_public_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(PublicApiDoc::openapi())
}

/// Serve admin OpenAPI spec as JSON
/// 
/// This endpoint serves the internal/admin API specification at `/openapi_admin.json`.
/// It is intentionally NOT auto-discoverable - only developers who know this
/// path can access the admin API documentation.
async fn openapi_admin_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(AdminApiDoc::openapi())
}

use crate::api::{handlers, middleware::{auth_middleware, access_log_middleware}};
use crate::openapi::ApiDoc;
use crate::state::AppState;
use axum::{middleware, response::Json, routing::{get, delete}, Router};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;

/// Create the Axum router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    // CORS configuration - adjust for production
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/openapi.json", get(openapi_handler))
        .route("/access-logs", get(handlers::access_logs_handler))
        .route("/access-logs", delete(handlers::clear_access_logs_handler));

    // Protected routes (require Bearer token auth)
    let protected_routes = Router::new()
        .route("/jira/list", get(handlers::jira_list_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // Add access logging middleware to all routes
        .layer(middleware::from_fn_with_state(state.clone(), access_log_middleware))
        .layer(cors)
        .with_state(state)
}

/// Serve OpenAPI spec as JSON
async fn openapi_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

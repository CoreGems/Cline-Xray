use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use crate::state::AppState;

/// Auth middleware - validates Bearer token
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = auth_header
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            log::warn!("REST API: Missing or invalid Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    if !state.verify_token(token) {
        log::warn!("REST API: Invalid auth token");
        return Err(StatusCode::UNAUTHORIZED);
    }

    log::debug!("REST API: Auth successful");
    Ok(next.run(request).await)
}

/// Access logging middleware - logs all HTTP requests
pub async fn access_log_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    
    // Get client IP from request extensions (set by ConnectInfo)
    let client_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Process the request
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    let status_code = response.status().as_u16();

    // Log the access
    log::info!(
        "REST API: {} {} {} {}ms",
        method,
        path,
        status_code,
        duration.as_millis()
    );

    // Store in access log
    state.add_access_log(
        method,
        path,
        status_code,
        duration.as_millis() as u64,
        client_ip,
    );

    response
}

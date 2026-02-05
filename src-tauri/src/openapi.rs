use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// OpenAPI specification for the Jira Dashboard REST API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Jira Dashboard API",
        version = "1.0.0",
        description = "REST API for Jira Dashboard application"
    ),
    paths(
        crate::api::handlers::health_handler,
        crate::api::handlers::jira_list_handler,
        crate::api::handlers::chat_handler,
    ),
    components(
        schemas(
            crate::api::handlers::HealthResponse,
            crate::api::handlers::JiraIssueSummary,
            crate::api::handlers::JiraListResponse,
            crate::api::handlers::ErrorResponse,
            crate::api::handlers::ChatRequest,
            crate::api::handlers::ChatMessage,
            crate::api::handlers::ChatResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "System health and status endpoints"),
        (name = "jira", description = "Jira issue management endpoints"),
        (name = "agent", description = "AI agent and chat endpoints")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // Set OpenAPI 3.1.0
        openapi.openapi = utoipa::openapi::OpenApiVersion::Version31;

        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}

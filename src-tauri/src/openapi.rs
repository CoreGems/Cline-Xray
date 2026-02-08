use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

// ============================================================================
// PUBLIC API SPECIFICATION
// ============================================================================

/// Public OpenAPI specification for the Jira Dashboard REST API
/// 
/// This spec is served at `/openapi.json` and contains endpoints intended
/// for external consumers and integrations.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Jira Dashboard API",
        version = "1.0.0",
        description = "Public REST API for Jira Dashboard application"
    ),
    paths(
        crate::api::handlers::health_handler,
        crate::api::handlers::jira_list_handler,
        crate::api::handlers::chat_handler,
        crate::api::handlers::list_models_handler,
        // Tool runtime - Agent-facing endpoints only
        crate::tool_runtime::handlers::list_tools_handler,      // GET /tools - Discovery
        crate::tool_runtime::handlers::invoke_tool_handler,     // POST /tools/invoke - Execution
        // Shadow Git / Changes
        crate::shadow_git::handlers::list_workspaces_handler,   // GET /changes/workspaces
        crate::shadow_git::handlers::list_tasks_handler,        // GET /changes/tasks
        crate::shadow_git::handlers::task_diff_handler,         // GET /changes/tasks/:taskId/diff
        crate::shadow_git::handlers::list_steps_handler,        // GET /changes/tasks/:taskId/steps
        crate::shadow_git::handlers::step_diff_handler,         // GET /changes/tasks/:taskId/steps/:index/diff
        crate::shadow_git::handlers::subtask_diff_handler,      // GET /changes/tasks/:taskId/subtasks/:subtaskIndex/diff
        // Conversation History
        crate::conversation_history::handlers::list_history_tasks_handler, // GET /history/tasks
        crate::conversation_history::handlers::get_task_detail_handler,    // GET /history/tasks/:taskId
        crate::conversation_history::handlers::get_task_messages_handler,  // GET /history/tasks/:taskId/messages
        crate::conversation_history::handlers::get_single_message_handler, // GET /history/tasks/:taskId/messages/:index
        crate::conversation_history::handlers::get_task_tools_handler,     // GET /history/tasks/:taskId/tools
        crate::conversation_history::handlers::get_task_thinking_handler,  // GET /history/tasks/:taskId/thinking
        crate::conversation_history::handlers::get_task_files_handler,     // GET /history/tasks/:taskId/files
        crate::conversation_history::handlers::get_task_subtasks_handler,  // GET /history/tasks/:taskId/subtasks
        crate::conversation_history::handlers::get_history_stats_handler,  // GET /history/stats
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
            crate::api::handlers::GeminiModel,
            crate::api::handlers::GeminiModelsResponse,
            // Tool runtime - Agent-facing schemas only
            crate::tool_runtime::ToolInvokeRequest,
            crate::tool_runtime::ToolCallSource,
            crate::tool_runtime::ValidationResult,
            crate::tool_runtime::ToolConfig,
            crate::tool_runtime::ArgClamp,
            crate::tool_runtime::ToolInfo,
            crate::tool_runtime::handlers::ToolInvokeResponse,
            crate::tool_runtime::handlers::ToolsListResponse,
            crate::tool_runtime::handlers::ToolErrorResponse,
            // Shadow Git / Changes schemas
            crate::shadow_git::WorkspaceInfo,
            crate::shadow_git::WorkspacesResponse,
            crate::shadow_git::ClineTaskSummary,
            crate::shadow_git::TasksResponse,
            crate::shadow_git::CheckpointStep,
            crate::shadow_git::StepsResponse,
            crate::shadow_git::DiffFile,
            crate::shadow_git::DiffResult,
            crate::shadow_git::handlers::ChangesErrorResponse,
            // Conversation History schemas
            crate::conversation_history::TaskHistorySummary,
            crate::conversation_history::TaskHistoryListResponse,
            crate::conversation_history::TaskDetailResponse,
            crate::conversation_history::ConversationMessage,
            crate::conversation_history::ContentBlockSummary,
            crate::conversation_history::ToolCallDetail,
            crate::conversation_history::FileInContextDetail,
            crate::conversation_history::ModelUsageDetail,
            crate::conversation_history::EnvironmentDetail,
            crate::conversation_history::PaginatedMessagesResponse,
            crate::conversation_history::FullMessageResponse,
            crate::conversation_history::FullContentBlock,
            crate::conversation_history::ToolCallTimelineEntry,
            crate::conversation_history::ToolCallTimelineResponse,
            crate::conversation_history::TaskToolsQuery,
            crate::conversation_history::ThinkingBlockEntry,
            crate::conversation_history::ThinkingBlocksResponse,
            crate::conversation_history::TaskThinkingQuery,
            crate::conversation_history::TaskFilesResponse,
            crate::conversation_history::TaskFilesQuery,
            crate::conversation_history::HistoryStatsResponse,
            crate::conversation_history::SubtaskEntry,
            crate::conversation_history::SubtasksResponse,
            crate::conversation_history::HistoryErrorResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "System health and status endpoints"),
        (name = "jira", description = "Jira issue management endpoints"),
        (name = "agent", description = "AI agent and chat endpoints"),
        (name = "tools", description = "Tool discovery and execution endpoints for AI agents"),
        (name = "tool", description = "APIs suitable for AI agent tool use"),
        (name = "changes", description = "Cline shadow Git checkpoint discovery and diff endpoints"),
        (name = "history", description = "Cline conversation history browsing and analytics endpoints")
    )
)]
pub struct PublicApiDoc;

// ============================================================================
// ADMIN API SPECIFICATION
// ============================================================================

/// Admin/Internal OpenAPI specification for the Jira Dashboard
/// 
/// This spec is served at `/openapi_admin.json` and contains internal
/// endpoints used by the application's UI for diagnostics and logging.
/// 
/// **Security Note:** This spec is intentionally NOT auto-discoverable.
/// External API scanners looking for `/openapi.json` will not find these endpoints.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Jira Dashboard Admin API",
        version = "1.0.0",
        description = "Internal admin API for Jira Dashboard diagnostics and logging. Not intended for external use."
    ),
    paths(
        // Logging endpoints
        crate::api::handlers::access_logs_handler,
        crate::api::handlers::clear_access_logs_handler,
        crate::api::handlers::inference_logs_handler,
        crate::api::handlers::clear_inference_logs_handler,
        // Tool runtime admin endpoints
        crate::tool_runtime::handlers::get_tool_logs_handler,
        crate::tool_runtime::handlers::clear_tool_logs_handler,
        crate::tool_runtime::handlers::get_config_handler,
        crate::tool_runtime::handlers::update_config_handler,
        crate::tool_runtime::handlers::configure_tool_handler,
        crate::tool_runtime::handlers::get_circuit_breakers_handler,
        crate::tool_runtime::handlers::reset_circuit_breakers_handler,
        crate::tool_runtime::handlers::reset_tool_circuit_breaker_handler,
        crate::tool_runtime::handlers::get_fixtures_handler,
        crate::tool_runtime::handlers::import_fixtures_handler,
        crate::tool_runtime::handlers::clear_fixtures_handler,
        crate::tool_runtime::handlers::enable_all_tools_handler,
        crate::tool_runtime::handlers::disable_all_tools_handler,
    ),
    components(
        schemas(
            crate::api::handlers::AccessLogsResponse,
            crate::api::handlers::InferenceLogsResponse,
            // Tool runtime admin schemas
            crate::tool_runtime::ToolCallResult,
            crate::tool_runtime::GlobalRuntimeConfig,
            crate::tool_runtime::ToolExecutionLog,
            crate::tool_runtime::CircuitBreakerState,
            crate::tool_runtime::CircuitState,
            crate::tool_runtime::handlers::ToolExecutionLogsResponse,
            crate::tool_runtime::handlers::RuntimeConfigResponse,
            crate::tool_runtime::handlers::CircuitBreakerStatusResponse,
            crate::tool_runtime::handlers::FixturesResponse,
            crate::tool_runtime::handlers::UpdateGlobalConfigRequest,
            crate::tool_runtime::handlers::ConfigureToolRequest,
        )
    ),
    tags(
        (name = "admin", description = "Internal admin and diagnostic endpoints"),
        (name = "tools", description = "Tool runtime admin endpoints - configuration, logging, circuit breakers, fixtures")
    )
)]
pub struct AdminApiDoc;

// ============================================================================
// SECURITY ADDON (shared)
// ============================================================================

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

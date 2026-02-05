//! Agent module for LLM-powered database assistance
//!
//! This module implements an AI agent that can answer natural language
//! questions about Oracle databases using function calling.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Tauri Backend (Rust)                       │
//! │                                                              │
//! │  ┌──────────────┐     ┌─────────────┐                       │
//! │  │ server.rs    │────▶│ agent/      │                       │
//! │  │ /agent/ask   │     │             │                       │
//! │  └──────────────┘     │  ┌────────────────────────┐        │
//! │                       │  │ LLM Provider           │        │
//! │                       │  │ (Gemini/OpenAI/etc.)   │        │
//! │                       │  │ (HTTPS to LLM API)     │        │
//! │                       │  └────────────────────────┘        │
//! │                       │             │                       │
//! │                       │             ▼                       │
//! │                       │  ┌────────────────────────┐        │
//! │                       │  │ Tool Executor          │        │
//! │                       │  │ - metadata::schemas()  │        │
//! │                       │  │ - metadata::tables()   │        │
//! │                       │  │ - query::execute()     │        │
//! │                       │  │ - validation::check()  │        │
//! │                       │  └────────────────────────┘        │
//! │                       └─────────────┘                       │
//! │                                                              │
//! │                       ┌─────────────────┐                   │
//! │                       │  Oracle DB      │                   │
//! │                       │  (OCI/ODPI-C)   │                   │
//! │                       └─────────────────┘                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Insight: Direct Rust Calls, Not HTTP
//!
//! The agent module imports and calls the same Rust functions that
//! the HTTP handlers use - no HTTP loopback:
//!
//! - `metadata::get_schemas(&pool, ...)` - not `GET /db/schemas`
//! - `query::execute(&pool, ...)` - not `POST /db/query`
//!
//! ## Modules
//!
//! - `types`: Shared types (AgentRequest, AgentResponse, ToolCall, etc.)
//! - `tools`: Tool definitions for function calling
//! - `executor`: Tool execution that calls db functions directly
//! - `provider`: AgentProvider trait for LLM abstraction
//! - `models`: Model registry for available LLMs
//! - `handlers`: HTTP handlers for agent endpoints
//! - `gemini`: Google Gemini provider implementation
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Create a tool executor with app state
//! let executor = ToolExecutor::new(state.clone());
//!
//! // Get a provider for the selected model
//! let provider = GeminiProvider::new(&api_key, 60);
//!
//! // Run the agent
//! let response = provider.run(&request, &tools, &executor, 10).await?;
//! ```

pub mod custom_agents;
pub mod executor;
pub mod gemini;
pub mod handlers;
pub mod models;
pub mod provider;
pub mod tools;
pub mod types;

// Re-exports for convenience
pub use executor::ToolExecutor;
pub use handlers::{agent_ask_handler, agent_models_handler, agent_status_handler};
pub use models::{get_available_models, ModelInfo, ModelsResponse};
pub use provider::AgentProvider;
pub use tools::{get_tool_definitions, ToolDefinition};
pub use types::{
    AgentConfig, AgentError, AgentRequest, AgentResponse, SourceCitation, TokenUsage,
    ToolCall, ToolError, ToolResult, TraceEntry,
};

// Provider re-exports
pub use gemini::GeminiProvider;

// Custom agents re-exports
pub use custom_agents::{
    init_agents, list_agents, get_agent, create_agent, update_agent, delete_agent,
    resolve_template_variables, CustomAgent, CreateAgentRequest, UpdateAgentRequest,
    AgentsListResponse, AgentResponse as CustomAgentResponse, AgentStorageError, ToolOverride,
};

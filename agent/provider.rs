//! Agent provider trait
//!
//! This trait defines the interface that all LLM providers must implement.
//! Each provider (Gemini, OpenAI, etc.) implements this trait differently
//! but exposes the same interface to the agent system.

use crate::agent::executor::ToolExecutor;
use crate::agent::tools::ToolDefinition;
use crate::agent::types::{AgentError, AgentRequest, AgentResponse};
use async_trait::async_trait;

/// Trait for LLM provider implementations
///
/// Each provider (Gemini, OpenAI, Claude, etc.) implements this trait
/// to provide a consistent interface for the agent system.
#[async_trait]
pub trait AgentProvider: Send + Sync {
    /// Get the provider name (for logging and configuration)
    fn name(&self) -> &'static str;
    
    /// Check if this provider is configured and ready to use
    fn is_configured(&self) -> bool;
    
    /// Get the default model for this provider
    fn default_model(&self) -> &str;
    
    /// Get all supported model IDs for this provider
    fn supported_models(&self) -> Vec<&str>;
    
    /// Check if a model ID is supported by this provider
    fn supports_model(&self, model_id: &str) -> bool {
        self.supported_models().contains(&model_id)
    }
    
    /// Run the agent loop with the given request
    ///
    /// This method:
    /// 1. Sends the user's question to the LLM with available tools
    /// 2. Handles function calling - when LLM requests a tool, execute it
    /// 3. Sends tool results back to LLM
    /// 4. Repeats until LLM produces a final answer
    /// 5. Returns the response with answer, sources, and trace
    async fn run(
        &self,
        request: &AgentRequest,
        tools: &[ToolDefinition],
        executor: &ToolExecutor,
        max_iterations: u32,
    ) -> Result<AgentResponse, AgentError>;
}

/// Factory function type for creating providers
pub type ProviderFactory = fn(api_key: &str) -> Box<dyn AgentProvider>;

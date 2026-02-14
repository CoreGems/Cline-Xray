use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Single access log entry for HTTP requests
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessLogEntry {
    pub id: u64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub client_ip: String,
}

/// Single inference log entry for AI model calls
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InferenceLogEntry {
    pub id: u64,
    pub timestamp: String,
    pub provider: String,          // e.g., "gemini", "openai"
    pub model: String,             // e.g., "gemini-2.0-flash"
    pub request_type: String,      // e.g., "chat", "completion"
    pub success: bool,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub error_message: Option<String>,
    // Extensible fields for future use
    pub system_prompt: Option<String>,
    pub user_message_preview: Option<String>, // First 100 chars of user message
    pub metadata: Option<serde_json::Value>,  // For any additional details
}

/// Shared application state for the REST server
pub struct AppState {
    pub auth_token: String,        // Generated at startup for session auth
    pub start_time: Instant,       // For uptime tracking
    pub api_base_url: RwLock<Option<String>>,
    
    // Jira configuration
    pub jira_base_url: String,
    pub jira_email: String,
    pub jira_api_token: String,
    
    // Gemini API configuration
    pub gemini_api_key: String,
    
    // OpenAI API configuration
    pub openai_api_key: String,
    
    // Access log storage
    access_log: RwLock<Vec<AccessLogEntry>>,
    log_counter: RwLock<u64>,
    
    // Inference log storage
    inference_log: RwLock<Vec<InferenceLogEntry>>,
    inference_log_counter: RwLock<u64>,
}

impl AppState {
    pub fn new(
        auth_token: String,
        jira_base_url: String,
        jira_email: String,
        jira_api_token: String,
        gemini_api_key: String,
        openai_api_key: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            auth_token,
            start_time: Instant::now(),
            api_base_url: RwLock::new(None),
            jira_base_url,
            jira_email,
            jira_api_token,
            gemini_api_key,
            openai_api_key,
            access_log: RwLock::new(Vec::new()),
            log_counter: RwLock::new(0),
            inference_log: RwLock::new(Vec::new()),
            inference_log_counter: RwLock::new(0),
        })
    }

    /// Verify Bearer token
    pub fn verify_token(&self, token: &str) -> bool {
        self.auth_token == token
    }

    /// Get server uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Create a JiraClient from app state
    pub fn create_jira_client(&self) -> crate::jira::JiraClient {
        crate::jira::JiraClient::new(
            self.jira_base_url.clone(),
            self.jira_email.clone(),
            self.jira_api_token.clone(),
        )
    }

    /// Add an access log entry
    pub fn add_access_log(&self, method: String, path: String, status_code: u16, duration_ms: u64, client_ip: String) {
        let mut counter = self.log_counter.write();
        *counter += 1;
        let id = *counter;

        let entry = AccessLogEntry {
            id,
            timestamp: chrono::Local::now().to_rfc3339(),
            method,
            path,
            status_code,
            duration_ms,
            client_ip,
        };

        let mut log = self.access_log.write();
        log.push(entry);
        
        // Keep only the last 1000 entries to prevent memory bloat
        let len = log.len();
        if len > 1000 {
            log.drain(0..len - 1000);
        }
    }

    /// Get all access log entries
    pub fn get_access_logs(&self) -> Vec<AccessLogEntry> {
        self.access_log.read().clone()
    }

    /// Clear access log
    pub fn clear_access_logs(&self) {
        self.access_log.write().clear();
    }

    /// Add an inference log entry
    pub fn add_inference_log(
        &self,
        provider: String,
        model: String,
        request_type: String,
        success: bool,
        status_code: Option<u16>,
        duration_ms: u64,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        total_tokens: Option<u32>,
        error_message: Option<String>,
        system_prompt: Option<String>,
        user_message_preview: Option<String>,
        metadata: Option<serde_json::Value>,
    ) {
        let mut counter = self.inference_log_counter.write();
        *counter += 1;
        let id = *counter;

        let entry = InferenceLogEntry {
            id,
            timestamp: chrono::Local::now().to_rfc3339(),
            provider,
            model,
            request_type,
            success,
            status_code,
            duration_ms,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            error_message,
            system_prompt,
            user_message_preview,
            metadata,
        };

        let mut log = self.inference_log.write();
        log.push(entry);
        
        // Keep only the last 500 entries to prevent memory bloat
        let len = log.len();
        if len > 500 {
            log.drain(0..len - 500);
        }
    }

    /// Get all inference log entries
    pub fn get_inference_logs(&self) -> Vec<InferenceLogEntry> {
        self.inference_log.read().clone()
    }

    /// Clear inference log
    pub fn clear_inference_logs(&self) {
        self.inference_log.write().clear();
    }
}

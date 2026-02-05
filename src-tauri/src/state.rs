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
    
    // Access log storage
    access_log: RwLock<Vec<AccessLogEntry>>,
    log_counter: RwLock<u64>,
}

impl AppState {
    pub fn new(
        auth_token: String,
        jira_base_url: String,
        jira_email: String,
        jira_api_token: String,
        gemini_api_key: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            auth_token,
            start_time: Instant::now(),
            api_base_url: RwLock::new(None),
            jira_base_url,
            jira_email,
            jira_api_token,
            gemini_api_key,
            access_log: RwLock::new(Vec::new()),
            log_counter: RwLock::new(0),
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
}

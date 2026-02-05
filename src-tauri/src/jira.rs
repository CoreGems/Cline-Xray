use base64::{engine::general_purpose::STANDARD, Engine};
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ============ Public Models ============

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JiraSettings {
    pub base_url: String,
    pub email: String,
    pub default_jql: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IssueSummary {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub status_category: String,
    pub assignee: Option<String>,
    pub priority: String,
    pub updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IssueDetails {
    pub key: String,
    pub summary: String,
    pub description: Option<String>,
    pub status: String,
    pub status_category: String,
    pub resolution: Option<String>,
    pub issue_type: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub created: String,
    pub updated: String,
    pub labels: Vec<String>,
    pub components: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub issues: Vec<IssueSummary>,
    pub total: i32,
}

// ============ Internal Jira API Response Types ============

#[derive(Debug, Deserialize)]
pub struct JiraCurrentUser {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(rename = "accountId")]
    pub account_id: String,
}

#[derive(Debug, Deserialize)]
struct JiraSearchResponse {
    issues: Vec<JiraIssue>,
    #[serde(default)]
    total: Option<i32>,
    #[serde(rename = "isLast", default)]
    is_last: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct JiraIssue {
    key: String,
    fields: JiraFields,
}

#[derive(Debug, Deserialize)]
struct JiraFields {
    summary: String,
    #[serde(default)]
    description: Option<serde_json::Value>,
    status: JiraStatus,
    #[serde(default)]
    resolution: Option<JiraResolution>,
    issuetype: JiraIssueType,
    #[serde(default)]
    priority: Option<JiraPriority>,
    #[serde(default)]
    assignee: Option<JiraUser>,
    #[serde(default)]
    reporter: Option<JiraUser>,
    #[serde(default)]
    created: Option<String>,
    updated: String,
    #[serde(default)]
    labels: Option<Vec<String>>,
    #[serde(default)]
    components: Option<Vec<JiraComponent>>,
}

#[derive(Debug, Deserialize)]
struct JiraStatus {
    name: String,
    #[serde(rename = "statusCategory")]
    status_category: JiraStatusCategory,
}

#[derive(Debug, Deserialize)]
struct JiraStatusCategory {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraResolution {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraIssueType {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraPriority {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraUser {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct JiraComponent {
    name: String,
}

// ============ Jira Client ============

pub struct JiraClient {
    base_url: String,
    email: String,
    api_token: String,
    client: Client,
}

impl JiraClient {
    /// Create a new Jira client with the given configuration
    pub fn new(base_url: String, email: String, api_token: String) -> Self {
        Self {
            base_url,
            email,
            api_token,
            client: Client::new(),
        }
    }

    /// Get the Basic Auth header value
    fn get_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.email, self.api_token);
        let encoded = STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    /// Get the current authenticated user
    pub async fn get_current_user(&self) -> Result<JiraCurrentUser, String> {
        info!("=== get_current_user: Fetching current user ===");
        let url = format!("{}/rest/api/3/myself", self.base_url);
        debug!("Request URL: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed: {}", e);
                debug!("Request error details: {:?}", e);
                format!("Request failed: {}", e)
            })?;

        let status = response.status();
        info!("Response status: {}", status);

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("API error {}: {}", status, &body[..body.len().min(500)]);
            debug!("Full error response: {}", body);
            return Err(format!("API error {}: {}", status, body));
        }

        let body_text = response.text().await.map_err(|e| {
            error!("Failed to read response body: {}", e);
            format!("Failed to read response body: {}", e)
        })?;
        
        debug!("Response body: {}", body_text);

        let user: JiraCurrentUser = serde_json::from_str(&body_text).map_err(|e| {
            error!("JSON parse error: {}", e);
            debug!("Failed to parse response: {}", body_text);
            format!("Failed to parse response: {}", e)
        })?;

        info!("Current user: {} ({})", user.display_name, user.email_address);
        Ok(user)
    }

    /// Search for issues using JQL (using the /search/jql endpoint)
    pub async fn search_issues(&self, jql: &str, max_results: u32) -> Result<SearchResult, String> {
        info!("=== search_issues: Starting JQL search ===");
        let url = format!("{}/rest/api/3/search/jql", self.base_url);
        
        // INFO: Basic operation info
        info!("Searching issues with JQL: {}", jql);
        
        // DEBUG: Full request details
        debug!("Request URL: {}", url);
        debug!("Request params: maxResults={}, fields=key,summary,status,updated,assignee,priority,issuetype", max_results);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("jql", jql),
                ("maxResults", &max_results.to_string()),
                ("fields", "key,summary,status,updated,assignee,priority,issuetype"),
            ])
            .header("Authorization", self.get_auth_header())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed: {}", e);
                debug!("Request error details: {:?}", e);
                format!("Request failed: {}", e)
            })?;

        let status = response.status();
        info!("Response status: {}", status);
        
        // Get the response body as text first for better error reporting
        let body_text = response
            .text()
            .await
            .map_err(|e| {
                error!("Failed to read response body: {}", e);
                format!("Failed to read response body: {}", e)
            })?;

        // DEBUG: Raw response data
        debug!("Response body length: {} bytes", body_text.len());
        debug!("Response body (first 2000 chars): {}", &body_text[..body_text.len().min(2000)]);

        if !status.is_success() {
            error!("API error {}: {}", status, &body_text[..body_text.len().min(500)]);
            return Err(format!("API error {}: {}", status, body_text));
        }

        // Parse the JSON response
        debug!("Attempting to parse JSON response...");
        let data: JiraSearchResponse = match serde_json::from_str(&body_text) {
            Ok(d) => {
                debug!("JSON parsing successful!");
                d
            },
            Err(e) => {
                error!("JSON parse error: {}. Line: {}, Column: {}", e, e.line(), e.column());
                debug!("Failed to parse response body: {}", body_text);
                return Err(format!("Failed to parse response: {}. Line: {}, Column: {}", e, e.line(), e.column()));
            }
        };
        
        let total = data.total.unwrap_or(data.issues.len() as i32);
        info!("Found {} issues (total: {})", data.issues.len(), total);

        let issues: Vec<IssueSummary> = data
            .issues
            .into_iter()
            .map(|issue| IssueSummary {
                key: issue.key,
                summary: issue.fields.summary,
                status: issue.fields.status.name,
                status_category: issue.fields.status.status_category.name,
                assignee: issue.fields.assignee.map(|a| a.display_name),
                priority: issue
                    .fields
                    .priority
                    .map(|p| p.name)
                    .unwrap_or_else(|| "None".to_string()),
                updated: issue.fields.updated,
            })
            .collect();

        debug!("Returning {} issues to frontend", issues.len());

        Ok(SearchResult {
            total,
            issues,
        })
    }

    /// Get detailed information about a specific issue
    pub async fn get_issue(&self, key: &str) -> Result<IssueDetails, String> {
        info!("=== get_issue: Fetching issue {} ===", key);
        let url = format!("{}/rest/api/3/issue/{}", self.base_url, key);
        debug!("Request URL: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.get_auth_header())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                error!("HTTP request failed for issue {}: {}", key, e);
                debug!("Request error details: {:?}", e);
                format!("Request failed: {}", e)
            })?;

        let status = response.status();
        info!("Response status: {}", status);

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("API error {} for issue {}: {}", status, key, &body[..body.len().min(500)]);
            debug!("Full error response: {}", body);
            return Err(format!("API error {}: {}", status, body));
        }

        let body_text = response.text().await.map_err(|e| {
            error!("Failed to read response body: {}", e);
            format!("Failed to read response body: {}", e)
        })?;
        
        debug!("Response body length: {} bytes", body_text.len());
        debug!("Response body (first 2000 chars): {}", &body_text[..body_text.len().min(2000)]);

        let issue: JiraIssue = serde_json::from_str(&body_text).map_err(|e| {
            error!("JSON parse error for issue {}: {}", key, e);
            debug!("Failed to parse response: {}", body_text);
            format!("Failed to parse response: {}", e)
        })?;

        let details = IssueDetails {
            key: issue.key.clone(),
            summary: issue.fields.summary,
            description: extract_description_text(&issue.fields.description),
            status: issue.fields.status.name,
            status_category: issue.fields.status.status_category.name,
            resolution: issue.fields.resolution.map(|r| r.name),
            issue_type: issue.fields.issuetype.name,
            priority: issue
                .fields
                .priority
                .map(|p| p.name)
                .unwrap_or_else(|| "None".to_string()),
            assignee: issue.fields.assignee.map(|a| a.display_name),
            reporter: issue.fields.reporter.map(|r| r.display_name),
            created: issue.fields.created.unwrap_or_else(|| "Unknown".to_string()),
            updated: issue.fields.updated,
            labels: issue.fields.labels.unwrap_or_default(),
            components: issue
                .fields
                .components
                .map(|c| c.into_iter().map(|comp| comp.name).collect())
                .unwrap_or_default(),
        };

        info!("Successfully fetched issue {}: {} [{}]", details.key, details.summary, details.status);
        Ok(details)
    }
}

// ============ Helper Functions ============

fn extract_description_text(description: &Option<serde_json::Value>) -> Option<String> {
    match description {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Object(obj)) => {
            // Handle Atlassian Document Format (ADF)
            if let Some(content) = obj.get("content") {
                extract_text_from_adf(content)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_text_from_adf(value: &serde_json::Value) -> Option<String> {
    let mut text_parts: Vec<String> = Vec::new();

    match value {
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(text) = extract_text_from_adf(item) {
                    text_parts.push(text);
                }
            }
        }
        serde_json::Value::Object(obj) => {
            // Check for text type
            if let Some(serde_json::Value::String(t)) = obj.get("type") {
                if t == "text" {
                    if let Some(serde_json::Value::String(text)) = obj.get("text") {
                        return Some(text.clone());
                    }
                }
            }
            // Recurse into content
            if let Some(content) = obj.get("content") {
                if let Some(text) = extract_text_from_adf(content) {
                    text_parts.push(text);
                }
            }
        }
        _ => {}
    }

    if text_parts.is_empty() {
        None
    } else {
        Some(text_parts.join("\n"))
    }
}

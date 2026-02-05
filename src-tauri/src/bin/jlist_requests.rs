use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct User {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Status {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Priority {
    name: String,
}

#[derive(Debug, Deserialize)]
struct IssueType {
    name: String,
}

#[derive(Debug, Deserialize)]
struct IssueFields {
    summary: Option<String>,
    status: Option<Status>,
    updated: Option<String>,
    priority: Option<Priority>,
    issuetype: Option<IssueType>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    key: String,
    fields: IssueFields,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    issues: Vec<Issue>,
    total: Option<usize>,
}

/// Get the current authenticated user
async fn get_current_user(
    client: &reqwest::Client,
    jira_url: &str,
) -> Result<User, Box<dyn std::error::Error>> {
    let url = format!("{}/rest/api/3/myself", jira_url);
    let response = client.get(&url).send().await?;

    if response.status().is_success() {
        let user = response.json::<User>().await?;
        Ok(user)
    } else {
        let status = response.status();
        let text = response.text().await?;
        Err(format!("Error getting current user: {} {}", status, text).into())
    }
}

/// Search for issues using JQL (using new /search/jql endpoint)
async fn search_issues(
    client: &reqwest::Client,
    jira_url: &str,
    jql: &str,
    max_results: u32,
) -> Result<SearchResult, Box<dyn std::error::Error>> {
    let url = format!("{}/rest/api/3/search/jql", jira_url);

    let response = client
        .get(&url)
        .query(&[
            ("jql", jql),
            ("maxResults", &max_results.to_string()),
            ("fields", "key,summary,status,updated,assignee,priority,issuetype"),
        ])
        .send()
        .await?;

    let status = response.status();
    println!("Response: {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));

    if response.status().is_success() {
        let result = response.json::<SearchResult>().await?;
        Ok(result)
    } else {
        let text = response.text().await?;
        Err(format!("Error searching issues: {}", text).into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Get Jira configuration from environment
    let jira_url = env::var("JIRA_URL")
        .expect("JIRA_URL must be set in environment or .env file");
    let jira_email = env::var("JIRA_EMAIL")
        .expect("JIRA_EMAIL must be set in environment or .env file");
    let jira_api_token = env::var("JIRA_API_TOKEN")
        .expect("JIRA_API_TOKEN must be set in environment or .env file");

    // Create Basic Auth header
    let auth_string = format!("{}:{}", jira_email, jira_api_token);
    let auth_encoded = BASE64.encode(auth_string.as_bytes());
    let auth_header = format!("Basic {}", auth_encoded);

    // Set up headers
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_header)?,
    );

    // Create HTTP client with headers
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    // Get current user
    match get_current_user(&client, &jira_url).await {
        Ok(user) => {
            let display = user.display_name
                .as_ref()
                .or(user.email_address.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            println!("Listing Jira issues for: {}\n", display);
        }
        Err(e) => {
            eprintln!("Error getting user: {}", e);
        }
    }

    // Query for issues assigned to current user
    let jql_query = "assignee = currentUser() ORDER BY updated DESC";

    // Get issues
    match search_issues(&client, &jira_url, jql_query, 100).await {
        Ok(result) => {
            let issues = &result.issues;
            let total = result.total.unwrap_or(issues.len());

            println!("Total issues found: {} (showing {})\n", total, issues.len());
            println!("{}", "-".repeat(80));

            for issue in issues {
                let key = &issue.key;
                let summary = issue.fields.summary
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("No summary");
                let status = issue.fields.status
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or("Unknown");
                let updated = issue.fields.updated
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("Unknown");
                let priority_name = issue.fields.priority
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or("None");
                let issue_type = issue.fields.issuetype
                    .as_ref()
                    .map(|t| t.name.as_str())
                    .unwrap_or("Unknown");

                println!("[{}] {}", key, summary);
                println!("  Type: {} | Status: {} | Priority: {}", issue_type, status, priority_name);
                println!("  Updated: {}", updated);
                println!("{}", "-".repeat(80));
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

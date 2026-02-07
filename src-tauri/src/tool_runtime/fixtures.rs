//! Fixtures replay/record for ToolRuntime
//!
//! Allows recording tool responses and replaying them for testing.

use super::ToolRuntime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Storage for recorded fixtures
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FixturesStorage {
    /// Map of operation_id -> list of fixtures
    #[serde(default)]
    pub fixtures: HashMap<String, Vec<Fixture>>,
}

impl FixturesStorage {
    /// Create empty storage
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all fixtures
    pub fn clear(&mut self) {
        self.fixtures.clear();
    }

    /// Add a fixture
    pub fn add(&mut self, operation_id: &str, fixture: Fixture) {
        self.fixtures
            .entry(operation_id.to_string())
            .or_default()
            .push(fixture);
    }

    /// Get fixtures for an operation
    pub fn get(&self, operation_id: &str) -> Option<&Vec<Fixture>> {
        self.fixtures.get(operation_id)
    }

    /// Find a matching fixture
    pub fn find_match(
        &self,
        operation_id: &str,
        args: &serde_json::Value,
    ) -> Option<&serde_json::Value> {
        let fixtures = self.fixtures.get(operation_id)?;
        
        for fixture in fixtures {
            if fixture.matches(args) {
                return Some(&fixture.response);
            }
        }
        
        None
    }

    /// Get total fixture count
    pub fn count(&self) -> usize {
        self.fixtures.values().map(|v| v.len()).sum()
    }
}

/// A single recorded fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    /// Arguments that trigger this fixture
    pub args: FixtureMatch,
    /// Recorded response
    pub response: serde_json::Value,
    /// When this fixture was recorded
    pub recorded_at: String,
    /// Description/name for this fixture
    #[serde(default)]
    pub name: Option<String>,
    /// Tags for filtering/organizing
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Fixture {
    /// Create a new fixture
    pub fn new(args: serde_json::Value, response: serde_json::Value) -> Self {
        Self {
            args: FixtureMatch::Exact(args),
            response,
            recorded_at: chrono::Local::now().to_rfc3339(),
            name: None,
            tags: Vec::new(),
        }
    }

    /// Create a fixture that matches any args
    pub fn any(response: serde_json::Value) -> Self {
        Self {
            args: FixtureMatch::Any,
            response,
            recorded_at: chrono::Local::now().to_rfc3339(),
            name: None,
            tags: Vec::new(),
        }
    }

    /// Set name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Check if this fixture matches the given args
    pub fn matches(&self, args: &serde_json::Value) -> bool {
        self.args.matches(args)
    }
}

/// Matching strategy for fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FixtureMatch {
    /// Match any arguments
    Any,
    /// Match exact arguments
    Exact(serde_json::Value),
    /// Match if args contain these keys with these values
    Contains(serde_json::Value),
    /// Match if args match this regex pattern (for string values)
    Regex { patterns: HashMap<String, String> },
}

impl FixtureMatch {
    /// Check if the given args match this pattern
    pub fn matches(&self, args: &serde_json::Value) -> bool {
        match self {
            FixtureMatch::Any => true,
            FixtureMatch::Exact(expected) => args == expected,
            FixtureMatch::Contains(expected) => {
                if let (Some(args_obj), Some(expected_obj)) =
                    (args.as_object(), expected.as_object())
                {
                    expected_obj.iter().all(|(key, value)| {
                        args_obj.get(key).map(|v| v == value).unwrap_or(false)
                    })
                } else {
                    false
                }
            }
            FixtureMatch::Regex { patterns } => {
                if let Some(args_obj) = args.as_object() {
                    patterns.iter().all(|(key, pattern)| {
                        if let Some(serde_json::Value::String(value)) = args_obj.get(key) {
                            regex::Regex::new(pattern)
                                .map(|re| re.is_match(value))
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    })
                } else {
                    false
                }
            }
        }
    }
}

impl ToolRuntime {
    /// Get a fixture response if available
    pub fn get_fixture(
        &self,
        operation_id: &str,
        args: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        let fixtures = self.fixtures.read();
        fixtures.find_match(operation_id, args).cloned()
    }

    /// Record a fixture
    pub fn record_fixture(
        &self,
        operation_id: &str,
        args: &serde_json::Value,
        response: serde_json::Value,
    ) {
        let fixture = Fixture::new(args.clone(), response);
        
        tracing::debug!(
            "Recording fixture for {} with args: {:?}",
            operation_id,
            args
        );
        
        self.fixtures.write().add(operation_id, fixture);
    }

    /// Add a manual fixture
    pub fn add_fixture(&self, operation_id: &str, fixture: Fixture) {
        self.fixtures.write().add(operation_id, fixture);
    }

    /// Remove all fixtures for an operation
    pub fn remove_fixtures(&self, operation_id: &str) {
        self.fixtures.write().fixtures.remove(operation_id);
    }

    /// Get fixture count for an operation
    pub fn fixture_count(&self, operation_id: &str) -> usize {
        self.fixtures
            .read()
            .fixtures
            .get(operation_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_exact_match() {
        let fixture = Fixture::new(
            serde_json::json!({"jql": "test", "maxResults": 10}),
            serde_json::json!({"result": "ok"}),
        );

        assert!(fixture.matches(&serde_json::json!({"jql": "test", "maxResults": 10})));
        assert!(!fixture.matches(&serde_json::json!({"jql": "test", "maxResults": 20})));
        assert!(!fixture.matches(&serde_json::json!({"jql": "test"})));
    }

    #[test]
    fn test_fixture_any_match() {
        let fixture = Fixture::any(serde_json::json!({"result": "ok"}));

        assert!(fixture.matches(&serde_json::json!({})));
        assert!(fixture.matches(&serde_json::json!({"anything": "works"})));
    }

    #[test]
    fn test_fixture_contains_match() {
        let fixture = Fixture {
            args: FixtureMatch::Contains(serde_json::json!({"jql": "test"})),
            response: serde_json::json!({"result": "ok"}),
            recorded_at: "2024-01-01".to_string(),
            name: None,
            tags: Vec::new(),
        };

        assert!(fixture.matches(&serde_json::json!({"jql": "test", "extra": "ignored"})));
        assert!(!fixture.matches(&serde_json::json!({"jql": "different"})));
    }

    #[test]
    fn test_fixtures_storage() {
        let mut storage = FixturesStorage::new();
        
        storage.add(
            "get_jira_list",
            Fixture::new(
                serde_json::json!({"jql": "test"}),
                serde_json::json!({"issues": []}),
            ),
        );
        
        let result = storage.find_match("get_jira_list", &serde_json::json!({"jql": "test"}));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), &serde_json::json!({"issues": []}));
        
        let no_result = storage.find_match("get_jira_list", &serde_json::json!({"jql": "other"}));
        assert!(no_result.is_none());
    }
}

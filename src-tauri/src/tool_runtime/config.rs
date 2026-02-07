//! Configuration types for ToolRuntime
//!
//! Per-tool and global configuration settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a specific tool
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    /// Whether this tool is enabled
    pub enabled: bool,

    /// Whether to run in dry-run mode (return mock data)
    pub dry_run: bool,

    /// Whether to use fixtures for this tool
    pub use_fixtures: bool,

    /// Whether to record responses as fixtures
    pub record_fixtures: bool,

    /// Argument clamps - min/max values for numeric parameters
    #[serde(default)]
    pub arg_clamps: HashMap<String, ArgClamp>,

    /// Timeout override for this tool (in milliseconds)
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Maximum retries before circuit breaker opens
    #[serde(default)]
    pub max_failures: Option<u32>,

    /// Custom metadata for this tool
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dry_run: false,
            use_fixtures: false,
            record_fixtures: false,
            arg_clamps: HashMap::new(),
            timeout_ms: None,
            max_failures: None,
            metadata: HashMap::new(),
        }
    }
}

impl ToolConfig {
    /// Create a new enabled tool config
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Create a disabled tool config
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Create a dry-run tool config
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Self::default()
        }
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set dry-run mode
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Add an arg clamp
    pub fn with_arg_clamp(mut self, param: &str, clamp: ArgClamp) -> Self {
        self.arg_clamps.insert(param.to_string(), clamp);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// Argument clamp for numeric parameters
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ArgClamp {
    /// Minimum value (inclusive)
    pub min: Option<f64>,
    /// Maximum value (inclusive)
    pub max: Option<f64>,
    /// Default value if not provided or out of range
    pub default: Option<f64>,
}

impl ArgClamp {
    /// Create a clamp with min and max
    pub fn new(min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            min,
            max,
            default: None,
        }
    }

    /// Create a clamp with default
    pub fn with_default(min: Option<f64>, max: Option<f64>, default: f64) -> Self {
        Self {
            min,
            max,
            default: Some(default),
        }
    }

    /// Clamp a value
    pub fn clamp(&self, value: f64) -> f64 {
        let mut result = value;
        if let Some(min) = self.min {
            result = result.max(min);
        }
        if let Some(max) = self.max {
            result = result.min(max);
        }
        result
    }

    /// Check if a value is in range
    pub fn is_in_range(&self, value: f64) -> bool {
        if let Some(min) = self.min {
            if value < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if value > max {
                return false;
            }
        }
        true
    }
}

/// Global runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GlobalRuntimeConfig {
    /// Global dry-run mode (affects all tools)
    pub dry_run: bool,

    /// Validate requests against OpenAPI schema
    pub validate_requests: bool,

    /// Validate responses against OpenAPI schema
    pub validate_responses: bool,

    /// Fail execution if validation fails
    pub fail_on_validation_error: bool,

    /// Replay fixtures if available
    pub replay_fixtures: bool,

    /// Global timeout for tool execution (milliseconds)
    pub default_timeout_ms: u64,

    /// Maximum retries before circuit breaker opens
    pub default_max_failures: u32,

    /// Circuit breaker reset time (milliseconds)
    pub circuit_breaker_reset_ms: u64,

    /// Enable detailed logging
    pub verbose_logging: bool,

    /// Rate limit - max calls per minute (0 = unlimited)
    pub rate_limit_per_minute: u32,
}

impl Default for GlobalRuntimeConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            validate_requests: true,
            validate_responses: true,
            fail_on_validation_error: false,
            replay_fixtures: false,
            default_timeout_ms: 30_000, // 30 seconds
            default_max_failures: 5,
            circuit_breaker_reset_ms: 60_000, // 1 minute
            verbose_logging: false,
            rate_limit_per_minute: 0, // unlimited
        }
    }
}

impl GlobalRuntimeConfig {
    /// Create a strict config (validation enabled, fail on error)
    pub fn strict() -> Self {
        Self {
            validate_requests: true,
            validate_responses: true,
            fail_on_validation_error: true,
            ..Self::default()
        }
    }

    /// Create a permissive config (no validation)
    pub fn permissive() -> Self {
        Self {
            validate_requests: false,
            validate_responses: false,
            fail_on_validation_error: false,
            ..Self::default()
        }
    }

    /// Create a testing config (dry-run, fixtures)
    pub fn testing() -> Self {
        Self {
            dry_run: true,
            replay_fixtures: true,
            validate_requests: true,
            validate_responses: true,
            fail_on_validation_error: true,
            verbose_logging: true,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_clamp() {
        let clamp = ArgClamp::new(Some(1.0), Some(100.0));

        assert_eq!(clamp.clamp(0.5), 1.0);
        assert_eq!(clamp.clamp(50.0), 50.0);
        assert_eq!(clamp.clamp(150.0), 100.0);

        assert!(!clamp.is_in_range(0.5));
        assert!(clamp.is_in_range(50.0));
        assert!(!clamp.is_in_range(150.0));
    }

    #[test]
    fn test_tool_config_builder() {
        let config = ToolConfig::enabled()
            .with_dry_run(true)
            .with_timeout(5000)
            .with_arg_clamp("maxResults", ArgClamp::new(Some(1.0), Some(1000.0)));

        assert!(config.enabled);
        assert!(config.dry_run);
        assert_eq!(config.timeout_ms, Some(5000));
        assert!(config.arg_clamps.contains_key("maxResults"));
    }
}

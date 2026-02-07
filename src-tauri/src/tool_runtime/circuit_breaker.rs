//! Circuit breaker and rate limiting for ToolRuntime
//!
//! Protects against cascading failures and rate limit exhaustion.

use super::{ToolRuntime, ToolRuntimeError};
use serde::{Deserialize, Serialize};

/// State of a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerState {
    /// Current state
    pub state: CircuitState,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Number of consecutive successes (when half-open)
    pub success_count: u32,
    /// Total number of calls
    pub total_calls: u64,
    /// Total number of failures
    pub total_failures: u64,
    /// When the circuit breaker was last opened
    pub last_failure_at: Option<String>,
    /// When the circuit breaker will reset (if open)
    pub reset_at: Option<String>,
    /// Last failure reason
    pub last_failure_reason: Option<String>,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            total_calls: 0,
            total_failures: 0,
            last_failure_at: None,
            reset_at: None,
            last_failure_reason: None,
        }
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    /// Circuit is closed, requests flow through normally
    Closed,
    /// Circuit is open, requests are blocked
    Open,
    /// Circuit is half-open, testing if the service has recovered
    HalfOpen,
}

impl CircuitBreakerState {
    /// Create a new circuit breaker in closed state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the circuit should allow a request through
    pub fn should_allow(&self, _reset_ms: u64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true, // Allow test requests
            CircuitState::Open => {
                // Check if reset time has passed
                if let Some(reset_at) = &self.reset_at {
                    if let Ok(reset_time) = chrono::DateTime::parse_from_rfc3339(reset_at) {
                        let now = chrono::Utc::now();
                        if now >= reset_time {
                            return true; // Time to test again
                        }
                    }
                }
                false
            }
        }
    }

    /// Record a successful call
    pub fn record_success(&mut self) {
        self.total_calls += 1;
        
        match self.state {
            CircuitState::Closed => {
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                // After a few successes, close the circuit
                if self.success_count >= 3 {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    tracing::info!("Circuit breaker closed after successful recovery");
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but transition to half-open
                self.state = CircuitState::HalfOpen;
                self.success_count = 1;
            }
        }
    }

    /// Record a failed call
    pub fn record_failure(&mut self, max_failures: u32, reset_ms: u64, reason: Option<String>) {
        self.total_calls += 1;
        self.total_failures += 1;
        self.failure_count += 1;
        self.last_failure_at = Some(chrono::Utc::now().to_rfc3339());
        self.last_failure_reason = reason;

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= max_failures {
                    self.state = CircuitState::Open;
                    let reset_time = chrono::Utc::now()
                        + chrono::Duration::milliseconds(reset_ms as i64);
                    self.reset_at = Some(reset_time.to_rfc3339());
                    tracing::warn!(
                        "Circuit breaker opened after {} failures, will reset at {}",
                        self.failure_count,
                        self.reset_at.as_ref().unwrap()
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open state reopens the circuit
                self.state = CircuitState::Open;
                let reset_time = chrono::Utc::now()
                    + chrono::Duration::milliseconds(reset_ms as i64);
                self.reset_at = Some(reset_time.to_rfc3339());
                self.success_count = 0;
                tracing::warn!("Circuit breaker reopened after failure in half-open state");
            }
            CircuitState::Open => {
                // Already open, just update reset time
                let reset_time = chrono::Utc::now()
                    + chrono::Duration::milliseconds(reset_ms as i64);
                self.reset_at = Some(reset_time.to_rfc3339());
            }
        }
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.reset_at = None;
    }

    /// Get statistics as JSON
    pub fn stats(&self) -> serde_json::Value {
        let failure_rate = if self.total_calls > 0 {
            (self.total_failures as f64 / self.total_calls as f64) * 100.0
        } else {
            0.0
        };

        serde_json::json!({
            "state": self.state,
            "failure_count": self.failure_count,
            "success_count": self.success_count,
            "total_calls": self.total_calls,
            "total_failures": self.total_failures,
            "failure_rate_percent": format!("{:.2}", failure_rate),
            "last_failure_at": self.last_failure_at,
            "reset_at": self.reset_at,
            "last_failure_reason": self.last_failure_reason,
        })
    }
}

impl ToolRuntime {
    /// Check if a tool call should be allowed (circuit breaker)
    pub fn check_circuit_breaker(&self, operation_id: &str) -> Result<(), ToolRuntimeError> {
        let circuit_breakers = self.circuit_breakers.read();
        let global_config = self.get_global_config();
        
        if let Some(state) = circuit_breakers.get(operation_id) {
            if !state.should_allow(global_config.circuit_breaker_reset_ms) {
                return Err(ToolRuntimeError::CircuitBreakerOpen(operation_id.to_string()));
            }
            
            // Transition from open to half-open if reset time passed
            if state.state == CircuitState::Open && state.should_allow(global_config.circuit_breaker_reset_ms) {
                drop(circuit_breakers);
                let mut circuit_breakers = self.circuit_breakers.write();
                if let Some(state) = circuit_breakers.get_mut(operation_id) {
                    state.state = CircuitState::HalfOpen;
                    state.success_count = 0;
                    tracing::info!("Circuit breaker for {} transitioned to half-open", operation_id);
                }
            }
        }
        
        Ok(())
    }

    /// Update circuit breaker state after a call
    pub fn update_circuit_breaker(&self, operation_id: &str, success: bool) {
        let tool_config = self.get_tool_config(operation_id);
        let global_config = self.get_global_config();
        
        let max_failures = tool_config
            .max_failures
            .unwrap_or(global_config.default_max_failures);
        let reset_ms = global_config.circuit_breaker_reset_ms;

        let mut circuit_breakers = self.circuit_breakers.write();
        let state = circuit_breakers
            .entry(operation_id.to_string())
            .or_insert_with(CircuitBreakerState::new);

        if success {
            state.record_success();
        } else {
            state.record_failure(max_failures, reset_ms, None);
        }
    }

    /// Update circuit breaker with failure reason
    pub fn update_circuit_breaker_with_reason(
        &self,
        operation_id: &str,
        success: bool,
        reason: Option<String>,
    ) {
        let tool_config = self.get_tool_config(operation_id);
        let global_config = self.get_global_config();
        
        let max_failures = tool_config
            .max_failures
            .unwrap_or(global_config.default_max_failures);
        let reset_ms = global_config.circuit_breaker_reset_ms;

        let mut circuit_breakers = self.circuit_breakers.write();
        let state = circuit_breakers
            .entry(operation_id.to_string())
            .or_insert_with(CircuitBreakerState::new);

        if success {
            state.record_success();
        } else {
            state.record_failure(max_failures, reset_ms, reason);
        }
    }

    /// Reset circuit breaker for a specific tool
    pub fn reset_circuit_breaker(&self, operation_id: &str) {
        let mut circuit_breakers = self.circuit_breakers.write();
        if let Some(state) = circuit_breakers.get_mut(operation_id) {
            state.reset();
            tracing::info!("Circuit breaker for {} manually reset", operation_id);
        }
    }

    /// Get circuit breaker state for a tool
    pub fn get_circuit_breaker(&self, operation_id: &str) -> Option<CircuitBreakerState> {
        self.circuit_breakers.read().get(operation_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed() {
        let mut state = CircuitBreakerState::new();
        
        assert!(state.should_allow(60000));
        assert_eq!(state.state, CircuitState::Closed);
        
        state.record_success();
        assert_eq!(state.total_calls, 1);
        assert_eq!(state.failure_count, 0);
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let mut state = CircuitBreakerState::new();
        
        // Record failures until circuit opens
        for i in 0..5 {
            state.record_failure(5, 60000, Some(format!("Error {}", i)));
        }
        
        assert_eq!(state.state, CircuitState::Open);
        assert!(!state.should_allow(60000));
    }

    #[test]
    fn test_circuit_breaker_half_open_recovery() {
        let mut state = CircuitBreakerState::new();
        
        // Open the circuit
        for _ in 0..5 {
            state.record_failure(5, 0, None); // 0ms reset for testing
        }
        
        assert_eq!(state.state, CircuitState::Open);
        
        // Simulate time passing and transitioning to half-open
        state.state = CircuitState::HalfOpen;
        state.success_count = 0;
        
        // Record successes to close the circuit
        state.record_success();
        assert_eq!(state.state, CircuitState::HalfOpen);
        
        state.record_success();
        assert_eq!(state.state, CircuitState::HalfOpen);
        
        state.record_success();
        assert_eq!(state.state, CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_stats() {
        let mut state = CircuitBreakerState::new();
        
        state.record_success();
        state.record_success();
        state.record_failure(5, 60000, Some("Test error".to_string()));
        
        let stats = state.stats();
        assert_eq!(stats["total_calls"], 3);
        assert_eq!(stats["total_failures"], 1);
        assert_eq!(stats["failure_count"], 1);
    }
}

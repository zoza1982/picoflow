//! Retry logic with exponential backoff for task execution
//!
//! This module provides configurable retry mechanisms for failed task executions.
//! It implements exponential backoff to avoid overwhelming systems during transient failures.
//!
//! # Example
//!
//! ```no_run
//! use picoflow::retry::{RetryConfig, RetryState};
//! use std::time::Duration;
//!
//! let config = RetryConfig::new(3, Duration::from_secs(1), Duration::from_secs(60));
//! let mut state = RetryState::new();
//!
//! // First attempt
//! if !state.should_retry(&config) {
//!     println!("Max retries exceeded");
//! }
//!
//! // Calculate delay for next retry
//! let delay = state.calculate_delay(&config);
//! ```

use std::time::Duration;
use tracing::{debug, warn};

/// Retry configuration for task execution
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (not including initial attempt)
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub base_delay: Duration,
    /// Maximum delay cap to prevent excessive waiting
    pub max_delay: Duration,
}

impl RetryConfig {
    /// Create a new retry configuration
    ///
    /// # Arguments
    ///
    /// * `max_retries` - Maximum number of retry attempts (not including initial)
    /// * `base_delay` - Base delay for exponential backoff (e.g., 1 second)
    /// * `max_delay` - Maximum delay cap (e.g., 60 seconds)
    ///
    /// # Example
    ///
    /// ```
    /// use picoflow::retry::RetryConfig;
    /// use std::time::Duration;
    ///
    /// let config = RetryConfig::new(3, Duration::from_secs(1), Duration::from_secs(60));
    /// ```
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
        }
    }

    /// Create default retry configuration (3 retries, 1s base, 60s max)
    pub fn default_config() -> Self {
        Self::new(3, Duration::from_secs(1), Duration::from_secs(60))
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Retry state tracking for a specific task execution
#[derive(Debug, Clone)]
pub struct RetryState {
    /// Current attempt number (starts at 1 for first attempt)
    pub attempt: u32,
    /// Number of retries performed (0 for first attempt)
    pub retry_count: u32,
}

impl RetryState {
    /// Create a new retry state (starts at attempt 1)
    pub fn new() -> Self {
        Self {
            attempt: 1,
            retry_count: 0,
        }
    }

    /// Check if we should retry based on the configuration
    ///
    /// Returns `true` if we haven't exceeded max retries, `false` otherwise.
    pub fn should_retry(&self, config: &RetryConfig) -> bool {
        self.retry_count < config.max_retries
    }

    /// Calculate exponential backoff delay for the next retry
    ///
    /// Formula: delay = base_delay * 2^(retry_count)
    /// Capped at max_delay to prevent excessive waiting.
    ///
    /// # Returns
    ///
    /// Duration to wait before next retry attempt
    pub fn calculate_delay(&self, config: &RetryConfig) -> Duration {
        let exponential_delay = config
            .base_delay
            .as_secs()
            .saturating_mul(2u64.saturating_pow(self.retry_count));

        let capped_delay = exponential_delay.min(config.max_delay.as_secs());

        debug!(
            "Calculated backoff delay: {}s (attempt {}, retry {})",
            capped_delay, self.attempt, self.retry_count
        );

        Duration::from_secs(capped_delay)
    }

    /// Record a retry attempt, incrementing counters
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
        self.attempt += 1;

        warn!(
            "Recording retry attempt (total retries: {}, attempt: {})",
            self.retry_count, self.attempt
        );
    }

    /// Get the next retry timestamp (now + delay)
    pub fn next_retry_time(&self, config: &RetryConfig) -> chrono::DateTime<chrono::Utc> {
        let delay = self.calculate_delay(config);
        chrono::Utc::now() + chrono::Duration::from_std(delay).unwrap()
    }
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate exponential backoff delay (legacy function for backward compatibility)
///
/// This function is kept for compatibility with existing code but new code
/// should use `RetryState::calculate_delay` instead.
///
/// # Arguments
///
/// * `attempt` - The attempt number (1 for first attempt, 2 for first retry, etc.)
///
/// # Returns
///
/// Duration to wait, capped at 60 seconds
pub fn calculate_backoff_delay(attempt: u32) -> Duration {
    let base_delay_secs = 1;
    let delay_secs = base_delay_secs * 2u64.pow(attempt.saturating_sub(1));
    Duration::from_secs(delay_secs.min(60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_new() {
        let config = RetryConfig::new(5, Duration::from_secs(2), Duration::from_secs(120));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay, Duration::from_secs(2));
        assert_eq!(config.max_delay, Duration::from_secs(120));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay, Duration::from_secs(1));
        assert_eq!(config.max_delay, Duration::from_secs(60));
    }

    #[test]
    fn test_retry_state_new() {
        let state = RetryState::new();
        assert_eq!(state.attempt, 1);
        assert_eq!(state.retry_count, 0);
    }

    #[test]
    fn test_should_retry() {
        let config = RetryConfig::new(3, Duration::from_secs(1), Duration::from_secs(60));
        let mut state = RetryState::new();

        // Initial state: 0 retries, should retry
        assert!(state.should_retry(&config));

        // After 1 retry: 1 retry, should retry
        state.record_retry();
        assert!(state.should_retry(&config));

        // After 2 retries: 2 retries, should retry
        state.record_retry();
        assert!(state.should_retry(&config));

        // After 3 retries: 3 retries, should NOT retry (max reached)
        state.record_retry();
        assert!(!state.should_retry(&config));
    }

    #[test]
    fn test_calculate_delay() {
        let config = RetryConfig::new(5, Duration::from_secs(1), Duration::from_secs(60));
        let mut state = RetryState::new();

        // Attempt 1 (retry_count 0): delay = 1 * 2^0 = 1s
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(1));

        // Attempt 2 (retry_count 1): delay = 1 * 2^1 = 2s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(2));

        // Attempt 3 (retry_count 2): delay = 1 * 2^2 = 4s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(4));

        // Attempt 4 (retry_count 3): delay = 1 * 2^3 = 8s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(8));

        // Attempt 5 (retry_count 4): delay = 1 * 2^4 = 16s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(16));

        // Attempt 6 (retry_count 5): delay = 1 * 2^5 = 32s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(32));

        // Attempt 7 (retry_count 6): delay = 1 * 2^6 = 64s, capped at 60s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(60));
    }

    #[test]
    fn test_calculate_delay_with_different_base() {
        let config = RetryConfig::new(3, Duration::from_secs(2), Duration::from_secs(100));
        let mut state = RetryState::new();

        // Attempt 1 (retry_count 0): delay = 2 * 2^0 = 2s
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(2));

        // Attempt 2 (retry_count 1): delay = 2 * 2^1 = 4s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(4));

        // Attempt 3 (retry_count 2): delay = 2 * 2^2 = 8s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(8));

        // Attempt 4 (retry_count 3): delay = 2 * 2^3 = 16s
        state.record_retry();
        assert_eq!(state.calculate_delay(&config), Duration::from_secs(16));
    }

    #[test]
    fn test_record_retry() {
        let mut state = RetryState::new();
        assert_eq!(state.attempt, 1);
        assert_eq!(state.retry_count, 0);

        state.record_retry();
        assert_eq!(state.attempt, 2);
        assert_eq!(state.retry_count, 1);

        state.record_retry();
        assert_eq!(state.attempt, 3);
        assert_eq!(state.retry_count, 2);
    }

    #[test]
    fn test_next_retry_time() {
        let config = RetryConfig::new(3, Duration::from_secs(1), Duration::from_secs(60));
        let state = RetryState::new();

        let now = chrono::Utc::now();
        let next_retry = state.next_retry_time(&config);

        // Should be approximately 1 second in the future
        let diff = next_retry.signed_duration_since(now);
        assert!(diff.num_seconds() >= 0);
        assert!(diff.num_seconds() <= 2); // Allow 1 second tolerance
    }

    #[test]
    fn test_calculate_backoff_delay_legacy() {
        // Test legacy function for backward compatibility
        assert_eq!(calculate_backoff_delay(1), Duration::from_secs(1));
        assert_eq!(calculate_backoff_delay(2), Duration::from_secs(2));
        assert_eq!(calculate_backoff_delay(3), Duration::from_secs(4));
        assert_eq!(calculate_backoff_delay(4), Duration::from_secs(8));
        assert_eq!(calculate_backoff_delay(5), Duration::from_secs(16));
        assert_eq!(calculate_backoff_delay(6), Duration::from_secs(32));
        assert_eq!(calculate_backoff_delay(7), Duration::from_secs(60)); // Capped
        assert_eq!(calculate_backoff_delay(10), Duration::from_secs(60)); // Capped
    }

    #[test]
    fn test_overflow_protection() {
        let config = RetryConfig::new(100, Duration::from_secs(1), Duration::from_secs(3600));
        let mut state = RetryState::new();

        // Set a very high retry count
        state.retry_count = 100;

        // Should not panic or overflow
        let delay = state.calculate_delay(&config);
        assert_eq!(delay, Duration::from_secs(3600)); // Capped at max_delay
    }
}

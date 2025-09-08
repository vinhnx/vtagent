//! Orchestrator retry and error handling module
//!
//! This module provides robust error handling for orchestrator response failures,
//! including retry mechanisms with exponential backoff and fallback strategies.

use crate::models::ModelId;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Configuration for retry behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries per task
    pub max_retries: u32,
    /// Initial delay in seconds
    pub initial_delay_secs: u64,
    /// Maximum delay in seconds (cap for exponential backoff)
    pub max_delay_secs: u64,
    /// Backoff multiplier (delay *= multiplier each retry)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_secs: 1,
            max_delay_secs: 60,
            backoff_multiplier: 2.0,
        }
    }
}

/// Statistics about retry attempts
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    pub total_attempts: u32,
    pub successful_retries: u32,
    pub failed_retries: u32,
    pub fallback_activations: u32,
    pub total_backoff_time: Duration,
}

/// Retry manager for orchestrator operations
#[derive(Debug)]
pub struct RetryManager {
    config: RetryConfig,
    stats: RetryStats,
}

impl RetryManager {
    /// Create a new retry manager with default configuration
    pub fn new() -> Self {
        Self {
            config: RetryConfig::default(),
            stats: RetryStats::default(),
        }
    }

    /// Create a new retry manager with custom configuration
    pub fn with_config(config: RetryConfig) -> Self {
        Self {
            config,
            stats: RetryStats::default(),
        }
    }

    /// Get the current retry statistics
    pub fn stats(&self) -> &RetryStats {
        &self.stats
    }

    /// Reset retry statistics
    pub fn reset_stats(&mut self) {
        self.stats = RetryStats::default();
    }

    /// Execute an operation with retry and fallback logic
    pub async fn execute_with_retry<F, Fut, T>(
        &mut self,
        operation_name: &str,
        primary_model: &ModelId,
        fallback_model: Option<&ModelId>,
        operation: F,
    ) -> Result<T>
    where
        F: Fn(ModelId) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        T: Clone,
    {
        let start_time = Instant::now();
        let mut delay_secs = self.config.initial_delay_secs;

        // Try with primary model first
        for attempt in 0..=self.config.max_retries {
            self.stats.total_attempts += 1;

            eprintln!(
                "Attempt {}/{} for {} using model {}",
                attempt + 1,
                self.config.max_retries + 1,
                operation_name,
                primary_model
            );

            match operation(primary_model.clone()).await {
                Ok(result) => {
                    if attempt > 0 {
                        self.stats.successful_retries += 1;
                        eprintln!(
                            "Operation '{}' succeeded on attempt {} with model {}",
                            operation_name,
                            attempt + 1,
                            primary_model
                        );
                    }
                    return Ok(result);
                }
                Err(err) => {
                    eprintln!(
                        "Attempt {}/{} failed for {} with model {}: {}",
                        attempt + 1,
                        self.config.max_retries + 1,
                        operation_name,
                        primary_model,
                        err
                    );

                    // If this is not the last attempt, wait before retrying
                    if attempt < self.config.max_retries {
                        let backoff_duration = Duration::from_secs(delay_secs);
                        self.stats.total_backoff_time += backoff_duration;

                        eprintln!(
                            "Waiting {} seconds before retry {} for {}",
                            delay_secs,
                            attempt + 2,
                            operation_name
                        );

                        sleep(backoff_duration).await;

                        // Apply exponential backoff with cap
                        delay_secs = std::cmp::min(
                            (delay_secs as f64 * self.config.backoff_multiplier) as u64,
                            self.config.max_delay_secs,
                        );
                    } else {
                        self.stats.failed_retries += 1;
                    }
                }
            }
        }

        // If we have a fallback model and primary failed, try fallback
        if let Some(fallback) = fallback_model {
        eprintln!(
            "Primary model {} failed after {} attempts. Trying fallback model {}",
            primary_model,
            self.config.max_retries + 1,
            fallback
        );            self.stats.fallback_activations += 1;

            match operation(fallback.clone()).await {
                Ok(result) => {
                    eprintln!(
                        "Fallback model {} succeeded for operation '{}'",
                        fallback,
                        operation_name
                    );
                    return Ok(result);
                }
                Err(err) => {
                    eprintln!(
                        "Fallback model {} also failed for operation '{}': {}",
                        fallback,
                        operation_name,
                        err
                    );
                }
            }
        }

        let total_time = start_time.elapsed();
        eprintln!(
            "Operation '{}' failed completely after {} attempts and {} total time. Primary model: {}, Fallback model: {:?}",
            operation_name,
            self.config.max_retries + 1,
            humantime::format_duration(total_time),
            primary_model,
            fallback_model
        );

        Err(anyhow!(
            "Operation '{}' failed after {} attempts with model {} and fallback {:?}",
            operation_name,
            self.config.max_retries + 1,
            primary_model,
            fallback_model
        ))
    }
}

/// Check if a response is considered empty or invalid
pub fn is_empty_response(response: &serde_json::Value) -> bool {
    match response {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.trim().is_empty(),
        serde_json::Value::Object(obj) => {
            obj.is_empty() ||
            // Check for common empty response patterns
            (obj.get("candidates").map_or(false, |c| c.as_array().map_or(false, |arr| arr.is_empty()))) ||
            (obj.get("content").map_or(false, |c| match c {
                serde_json::Value::String(s) => s.trim().is_empty(),
                serde_json::Value::Array(arr) => arr.is_empty(),
                _ => false,
            }))
        },
        serde_json::Value::Array(arr) => arr.is_empty(),
        _ => false,
    }
}

/// Detect if an error indicates a temporary failure that should be retried
pub fn is_retryable_error(error: &anyhow::Error) -> bool {
    let error_msg = error.to_string().to_lowercase();

    // Common temporary error patterns
    error_msg.contains("timeout") ||
    error_msg.contains("rate limit") ||
    error_msg.contains("503") ||
    error_msg.contains("502") ||
    error_msg.contains("500") ||
    error_msg.contains("connection") ||
    error_msg.contains("network") ||
    error_msg.contains("temporary") ||
    error_msg.contains("overloaded") ||
    error_msg.contains("quota")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_empty_response_detection() {
        assert!(is_empty_response(&serde_json::Value::Null));
        assert!(is_empty_response(&json!("")));
        assert!(is_empty_response(&json!("  ")));
        assert!(is_empty_response(&json!({})));
        assert!(is_empty_response(&json!([])));
        assert!(is_empty_response(&json!({"candidates": []})));
        assert!(is_empty_response(&json!({"content": ""})));
        assert!(is_empty_response(&json!({"content": []})));

        assert!(!is_empty_response(&json!("hello")));
        assert!(!is_empty_response(&json!({"content": "hello"})));
        assert!(!is_empty_response(&json!({"candidates": [{"content": "hello"}]})));
    }

    #[test]
    fn test_retryable_error_detection() {
        assert!(is_retryable_error(&anyhow!("Connection timeout")));
        assert!(is_retryable_error(&anyhow!("Rate limit exceeded")));
        assert!(is_retryable_error(&anyhow!("HTTP 503 Service Unavailable")));
        assert!(is_retryable_error(&anyhow!("Network error")));

        assert!(!is_retryable_error(&anyhow!("Invalid API key")));
        assert!(!is_retryable_error(&anyhow!("Permission denied")));
        assert!(!is_retryable_error(&anyhow!("Invalid model")));
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_secs, 1);
        assert_eq!(config.max_delay_secs, 60);
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[tokio::test]
    async fn test_retry_manager_success_first_attempt() {
        let mut manager = RetryManager::new();
        let result = manager
            .execute_with_retry(
                "test_operation",
                &ModelId::Gemini25Flash,
                Some(&ModelId::Gemini25FlashLite),
                |_model| async { Ok::<String, anyhow::Error>("success".to_string()) },
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(manager.stats().total_attempts, 1);
        assert_eq!(manager.stats().successful_retries, 0);
        assert_eq!(manager.stats().fallback_activations, 0);
    }

    #[tokio::test]
    async fn test_retry_manager_success_after_retry() {
        let mut manager = RetryManager::with_config(RetryConfig {
            max_retries: 2,
            initial_delay_secs: 0, // No delay for test
            max_delay_secs: 1,
            backoff_multiplier: 2.0,
        });

        let mut attempt_count = 0;
        let result = manager
            .execute_with_retry(
                "test_operation",
                &ModelId::Gemini25Flash,
                Some(&ModelId::Gemini25FlashLite),
                |_model| async {
                    attempt_count += 1;
                    if attempt_count < 2 {
                        Err(anyhow!("Temporary failure"))
                    } else {
                        Ok::<String, anyhow::Error>("success".to_string())
                    }
                },
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(manager.stats().total_attempts, 2);
        assert_eq!(manager.stats().successful_retries, 1);
    }
}

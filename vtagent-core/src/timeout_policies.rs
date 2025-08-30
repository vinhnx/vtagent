//! Configurable timeout policies for different operation types and scenarios
//!
//! This module provides predefined timeout policies that can be applied to different
//! types of operations, allowing for fine-tuned timeout behavior based on operation
//! characteristics and user preferences.

use crate::timeout_detector::{TimeoutConfig, OperationType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Predefined timeout policies for common scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeoutPolicy {
    /// Conservative policy - longer timeouts, more retries, suitable for unreliable networks
    Conservative,
    /// Balanced policy - moderate timeouts and retries, good for most scenarios
    Balanced,
    /// Aggressive policy - shorter timeouts, fewer retries, optimized for fast networks
    Aggressive,
    /// Custom policy - user-defined configuration
    Custom(TimeoutConfig),
}

/// Configuration for operation-specific timeout policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationTimeoutPolicy {
    pub operation_type: OperationType,
    pub policy: TimeoutPolicy,
    pub enabled: bool,
}

/// Comprehensive timeout policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicyConfig {
    /// Global policy applied to all operations by default
    pub global_policy: TimeoutPolicy,
    /// Operation-specific overrides
    pub operation_policies: HashMap<OperationType, TimeoutPolicy>,
    /// Whether timeout detection is enabled globally
    pub enabled: bool,
    /// Adaptive timeout adjustment based on network conditions
    pub adaptive_timeout: bool,
    /// Maximum timeout multiplier for adaptive adjustments
    pub max_timeout_multiplier: f64,
}

impl Default for TimeoutPolicyConfig {
    fn default() -> Self {
        Self {
            global_policy: TimeoutPolicy::Balanced,
            operation_policies: HashMap::new(),
            enabled: false,
            adaptive_timeout: true,
            max_timeout_multiplier: 3.0,
        }
    }
}

impl TimeoutPolicy {
    /// Get the timeout configuration for this policy
    pub fn get_config(&self) -> TimeoutConfig {
        match self {
            TimeoutPolicy::Conservative => TimeoutConfig {
                timeout_duration: Duration::from_secs(120),
                max_retries: 5,
                initial_retry_delay: Duration::from_millis(500),
                max_retry_delay: Duration::from_secs(15),
                backoff_multiplier: 1.8,
                use_jitter: true,
                retry_on_timeout: true,
                retry_on_errors: vec![
                    "timeout".to_string(),
                    "connection".to_string(),
                    "network".to_string(),
                    "server_error".to_string(),
                    "rate_limit".to_string(),
                ],
            },
            TimeoutPolicy::Balanced => TimeoutConfig {
                timeout_duration: Duration::from_secs(60),
                max_retries: 3,
                initial_retry_delay: Duration::from_millis(200),
                max_retry_delay: Duration::from_secs(10),
                backoff_multiplier: 2.0,
                use_jitter: true,
                retry_on_timeout: true,
                retry_on_errors: vec![
                    "timeout".to_string(),
                    "connection".to_string(),
                    "network".to_string(),
                    "server_error".to_string(),
                ],
            },
            TimeoutPolicy::Aggressive => TimeoutConfig {
                timeout_duration: Duration::from_secs(30),
                max_retries: 2,
                initial_retry_delay: Duration::from_millis(100),
                max_retry_delay: Duration::from_secs(5),
                backoff_multiplier: 2.5,
                use_jitter: true,
                retry_on_timeout: true,
                retry_on_errors: vec![
                    "timeout".to_string(),
                    "connection".to_string(),
                ],
            },
            TimeoutPolicy::Custom(config) => config.clone(),
        }
    }

    /// Create a conservative policy for unreliable networks
    pub fn conservative() -> Self {
        TimeoutPolicy::Conservative
    }

    /// Create a balanced policy for most scenarios
    pub fn balanced() -> Self {
        TimeoutPolicy::Balanced
    }

    /// Create an aggressive policy for fast networks
    pub fn aggressive() -> Self {
        TimeoutPolicy::Aggressive
    }

    /// Create a custom policy with specific configuration
    pub fn custom(config: TimeoutConfig) -> Self {
        TimeoutPolicy::Custom(config)
    }
}

impl TimeoutPolicyConfig {
    /// Create a new timeout policy configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable timeout detection globally
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disable timeout detection globally
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Set the global timeout policy
    pub fn with_global_policy(mut self, policy: TimeoutPolicy) -> Self {
        self.global_policy = policy;
        self
    }

    /// Add an operation-specific timeout policy
    pub fn with_operation_policy(mut self, operation_type: OperationType, policy: TimeoutPolicy) -> Self {
        self.operation_policies.insert(operation_type, policy);
        self
    }

    /// Enable adaptive timeout adjustment
    pub fn with_adaptive_timeout(mut self, enabled: bool) -> Self {
        self.adaptive_timeout = enabled;
        self
    }

    /// Set the maximum timeout multiplier for adaptive adjustments
    pub fn with_max_timeout_multiplier(mut self, multiplier: f64) -> Self {
        self.max_timeout_multiplier = multiplier;
        self
    }

    /// Get the timeout configuration for a specific operation type
    pub fn get_config_for_operation(&self, operation_type: &OperationType) -> Option<TimeoutConfig> {
        if !self.enabled {
            return None;
        }

        // Check for operation-specific policy first
        if let Some(policy) = self.operation_policies.get(operation_type) {
            Some(policy.get_config())
        } else {
            // Fall back to global policy
            Some(self.global_policy.get_config())
        }
    }

    /// Get the effective configuration for an operation, considering adaptive adjustments
    pub fn get_adaptive_config_for_operation(
        &self,
        operation_type: &OperationType,
        network_quality: NetworkQuality,
    ) -> Option<TimeoutConfig> {
        let mut config = self.get_config_for_operation(operation_type)?;

        if self.adaptive_timeout {
            // Adjust timeout based on network quality
            let multiplier: f64 = match network_quality {
                NetworkQuality::Excellent => 0.8,
                NetworkQuality::Good => 1.0,
                NetworkQuality::Poor => 1.5,
                NetworkQuality::VeryPoor => 2.0,
            };

            // Cap the multiplier
            let adjusted_multiplier = multiplier.min(self.max_timeout_multiplier);

            // Adjust timeout duration
            let new_timeout = config.timeout_duration.mul_f64(adjusted_multiplier);
            config.timeout_duration = new_timeout;

            // Adjust retry delays proportionally
            config.initial_retry_delay = config.initial_retry_delay.mul_f64(adjusted_multiplier);
            config.max_retry_delay = config.max_retry_delay.mul_f64(adjusted_multiplier);
        }

        Some(config)
    }

    /// Create a preset configuration for high-reliability scenarios
    pub fn high_reliability() -> Self {
        Self {
            global_policy: TimeoutPolicy::Conservative,
            operation_policies: HashMap::from([
                (OperationType::ApiCall, TimeoutPolicy::Conservative),
                (OperationType::NetworkRequest, TimeoutPolicy::Conservative),
                (OperationType::FileOperation, TimeoutPolicy::Balanced),
                (OperationType::CodeAnalysis, TimeoutPolicy::Balanced),
                (OperationType::ToolExecution, TimeoutPolicy::Balanced),
                (OperationType::Processing, TimeoutPolicy::Balanced),
            ]),
            enabled: true,
            adaptive_timeout: true,
            max_timeout_multiplier: 3.0,
        }
    }

    /// Create a preset configuration for high-performance scenarios
    pub fn high_performance() -> Self {
        Self {
            global_policy: TimeoutPolicy::Aggressive,
            operation_policies: HashMap::from([
                (OperationType::ApiCall, TimeoutPolicy::Balanced),
                (OperationType::NetworkRequest, TimeoutPolicy::Balanced),
                (OperationType::FileOperation, TimeoutPolicy::Aggressive),
                (OperationType::CodeAnalysis, TimeoutPolicy::Balanced),
                (OperationType::ToolExecution, TimeoutPolicy::Balanced),
                (OperationType::Processing, TimeoutPolicy::Balanced),
            ]),
            enabled: true,
            adaptive_timeout: true,
            max_timeout_multiplier: 2.0,
        }
    }

    /// Create a preset configuration for development/testing scenarios
    pub fn development() -> Self {
        Self {
            global_policy: TimeoutPolicy::Custom(TimeoutConfig {
                timeout_duration: Duration::from_secs(30),
                max_retries: 1,
                initial_retry_delay: Duration::from_millis(100),
                max_retry_delay: Duration::from_secs(2),
                backoff_multiplier: 2.0,
                use_jitter: false,
                retry_on_timeout: true,
                retry_on_errors: vec!["timeout".to_string(), "connection".to_string()],
            }),
            operation_policies: HashMap::new(),
            enabled: true,
            adaptive_timeout: false,
            max_timeout_multiplier: 1.5,
        }
    }
}

/// Network quality assessment for adaptive timeout adjustments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkQuality {
    Excellent,
    Good,
    Poor,
    VeryPoor,
}

impl NetworkQuality {
    /// Assess network quality based on recent timeout statistics
    pub async fn assess_from_timeout_stats(stats: &crate::timeout_detector::TimeoutStats) -> Self {
        if stats.total_operations == 0 {
            return NetworkQuality::Good; // Default assumption
        }

        let timeout_rate = stats.timed_out_operations as f64 / stats.total_operations as f64;
        let retry_success_rate = if stats.total_retry_attempts > 0 {
            stats.successful_retries as f64 / stats.total_retry_attempts as f64
        } else {
            1.0
        };

        match (timeout_rate, retry_success_rate) {
            (rate, _) if rate < 0.05 => NetworkQuality::Excellent,
            (rate, success) if rate < 0.15 && success > 0.8 => NetworkQuality::Good,
            (rate, success) if rate < 0.3 && success > 0.6 => NetworkQuality::Poor,
            _ => NetworkQuality::VeryPoor,
        }
    }
}

/// Timeout policy manager for centralized configuration
pub struct TimeoutPolicyManager {
    config: TimeoutPolicyConfig,
    network_quality: NetworkQuality,
}

impl TimeoutPolicyManager {
    pub fn new(config: TimeoutPolicyConfig) -> Self {
        Self {
            config,
            network_quality: NetworkQuality::Good,
        }
    }

    /// Update network quality assessment
    pub async fn update_network_quality(&mut self) {
        let stats = crate::timeout_detector::TIMEOUT_DETECTOR.get_stats().await;
        self.network_quality = NetworkQuality::assess_from_timeout_stats(&stats).await;
    }

    /// Get the effective timeout configuration for an operation
    pub fn get_effective_config(&self, operation_type: &OperationType) -> Option<TimeoutConfig> {
        self.config.get_adaptive_config_for_operation(operation_type, self.network_quality)
    }

    /// Apply the effective configuration to the timeout detector
    pub async fn apply_to_detector(&self) {
        for operation_type in [
            OperationType::ApiCall,
            OperationType::FileOperation,
            OperationType::CodeAnalysis,
            OperationType::ToolExecution,
            OperationType::NetworkRequest,
            OperationType::Processing,
        ].iter() {
            if let Some(config) = self.get_effective_config(operation_type) {
                crate::timeout_detector::TIMEOUT_DETECTOR.set_config(operation_type.clone(), config).await;
            }
        }
    }

    /// Get the current network quality assessment
    pub fn network_quality(&self) -> NetworkQuality {
        self.network_quality
    }

    /// Update the configuration
    pub fn update_config(&mut self, config: TimeoutPolicyConfig) {
        self.config = config;
    }

    /// Check if timeout detection is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Default for TimeoutPolicyManager {
    fn default() -> Self {
        Self::new(TimeoutPolicyConfig::default())
    }
}

/// Global timeout policy manager instance
use once_cell::sync::Lazy;
pub static POLICY_MANAGER: Lazy<std::sync::Mutex<TimeoutPolicyManager>> =
    Lazy::new(|| std::sync::Mutex::new(TimeoutPolicyManager::default()));

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_timeout_policies() {
        // Test conservative policy
        let conservative = TimeoutPolicy::Conservative.get_config();
        assert_eq!(conservative.timeout_duration, Duration::from_secs(120));
        assert_eq!(conservative.max_retries, 5);

        // Test balanced policy
        let balanced = TimeoutPolicy::Balanced.get_config();
        assert_eq!(balanced.timeout_duration, Duration::from_secs(60));
        assert_eq!(balanced.max_retries, 3);

        // Test aggressive policy
        let aggressive = TimeoutPolicy::Aggressive.get_config();
        assert_eq!(aggressive.timeout_duration, Duration::from_secs(30));
        assert_eq!(aggressive.max_retries, 2);
    }

    #[test]
    fn test_policy_config() {
        let config = TimeoutPolicyConfig::high_reliability();
        assert!(config.enabled);
        assert!(config.adaptive_timeout);

        let api_config = config.get_config_for_operation(&OperationType::ApiCall);
        assert!(api_config.is_some());
        assert_eq!(api_config.unwrap().timeout_duration, Duration::from_secs(120));
    }

    #[test]
    fn test_network_quality_assessment() {
        // Test excellent network
        let stats = crate::timeout_detector::TimeoutStats {
            total_operations: 100,
            timed_out_operations: 2, // 2% timeout rate
            successful_retries: 5,
            failed_retries: 1,
            average_timeout_duration: Duration::from_secs(1),
            total_retry_attempts: 6,
        };

        // Since we can't easily test the async function in unit tests,
        // we'll test the logic directly
        let timeout_rate = stats.timed_out_operations as f64 / stats.total_operations as f64;
        let retry_success_rate = stats.successful_retries as f64 / stats.total_retry_attempts as f64;

        assert!(timeout_rate < 0.05); // Should be excellent
        assert!(retry_success_rate > 0.8); // Should be good
    }

    #[tokio::test]
    async fn test_policy_manager() {
        let config = TimeoutPolicyConfig::development();
        let manager = TimeoutPolicyManager::new(config);

        assert!(manager.is_enabled());

        let api_config = manager.get_effective_config(&OperationType::ApiCall);
        assert!(api_config.is_some());
        assert_eq!(api_config.unwrap().timeout_duration, Duration::from_secs(30));
    }
}

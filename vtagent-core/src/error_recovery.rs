use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::timeout_detector::{OperationType, TIMEOUT_DETECTOR};

/// Represents an error that occurred during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub id: String,
    pub timestamp: u64,
    pub error_type: ErrorType,
    pub message: String,
    pub context: ErrorContext,
    pub recovery_attempts: Vec<RecoveryAttempt>,
    pub resolved: bool,
}

/// Type of error that can occur
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum ErrorType {
    ToolExecution,
    ApiCall,
    ContextCompression,
    FileSystem,
    Network,
    Validation,
    Unknown,
}

/// Context information about where and why the error occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub conversation_turn: usize,
    pub user_input: Option<String>,
    pub tool_name: Option<String>,
    pub tool_args: Option<Value>,
    pub api_request_size: Option<usize>,
    pub context_size: Option<usize>,
    pub stack_trace: Option<String>,
}

/// A recovery attempt that was made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub timestamp: u64,
    pub strategy: RecoveryStrategy,
    pub success: bool,
    pub result: String,
    pub new_context_size: Option<usize>,
}

/// Recovery strategy used to handle the error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    RetryWithBackoff {
        delay_ms: u64,
        attempt_number: usize,
    },
    ContextCompression {
        compression_ratio: f64,
    },
    SimplifyRequest {
        removed_parameters: Vec<String>,
    },
    AlternativeTool {
        original_tool: String,
        alternative_tool: String,
    },
    ContextReset {
        preserved_data: HashMap<String, Value>,
    },
    ManualIntervention,
}

/// Error recovery manager
pub struct ErrorRecoveryManager {
    errors: Vec<ExecutionError>,
    recovery_strategies: HashMap<ErrorType, Vec<RecoveryStrategy>>,
    context_compression_threshold: usize,
    operation_type_mapping: HashMap<ErrorType, OperationType>,
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        let mut recovery_strategies = HashMap::new();
        let mut operation_type_mapping = HashMap::new();

        // Define recovery strategies for different error types
        recovery_strategies.insert(
            ErrorType::ToolExecution,
            vec![
                RecoveryStrategy::RetryWithBackoff {
                    delay_ms: 1000,
                    attempt_number: 1,
                },
                RecoveryStrategy::AlternativeTool {
                    original_tool: "".to_string(),
                    alternative_tool: "".to_string(),
                },
                RecoveryStrategy::ContextCompression {
                    compression_ratio: 0.5,
                },
            ],
        );

        recovery_strategies.insert(
            ErrorType::ApiCall,
            vec![
                RecoveryStrategy::RetryWithBackoff {
                    delay_ms: 2000,
                    attempt_number: 1,
                },
                RecoveryStrategy::ContextCompression {
                    compression_ratio: 0.7,
                },
                RecoveryStrategy::ContextReset {
                    preserved_data: HashMap::new(),
                },
            ],
        );

        recovery_strategies.insert(
            ErrorType::ContextCompression,
            vec![RecoveryStrategy::ContextReset {
                preserved_data: HashMap::new(),
            }],
        );

        // Map error types to operation types for timeout detector integration
        operation_type_mapping.insert(ErrorType::ToolExecution, OperationType::ToolExecution);
        operation_type_mapping.insert(ErrorType::ApiCall, OperationType::ApiCall);
        operation_type_mapping.insert(ErrorType::Network, OperationType::NetworkRequest);
        operation_type_mapping.insert(ErrorType::FileSystem, OperationType::FileOperation);
        operation_type_mapping.insert(ErrorType::Validation, OperationType::Processing);
        operation_type_mapping.insert(ErrorType::Unknown, OperationType::Processing);

        Self {
            errors: Vec::new(),
            recovery_strategies,
            context_compression_threshold: 50000, // tokens
            operation_type_mapping,
        }
    }

    /// Record a new error
    pub fn record_error(
        &mut self,
        error_type: ErrorType,
        message: String,
        context: ErrorContext,
    ) -> String {
        let error_id = format!(
            "error_{}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            self.errors.len()
        );

        let error = ExecutionError {
            id: error_id.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            error_type: error_type.clone(),
            message,
            context,
            recovery_attempts: Vec::new(),
            resolved: false,
        };

        self.errors.push(error);
        error_id
    }

    /// Record a recovery attempt
    pub fn record_recovery_attempt(
        &mut self,
        error_id: &str,
        strategy: RecoveryStrategy,
        success: bool,
        result: String,
        new_context_size: Option<usize>,
    ) {
        let attempt = RecoveryAttempt {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            strategy,
            success,
            result,
            new_context_size,
        };

        if let Some(error) = self.errors.iter_mut().find(|e| e.id == error_id) {
            error.recovery_attempts.push(attempt);
            if success {
                error.resolved = true;
            }
        }
    }

    /// Get recovery strategies for a specific error type
    pub fn get_recovery_strategies(&self, error_type: &ErrorType) -> &[RecoveryStrategy] {
        self.recovery_strategies
            .get(error_type)
            .map(|strategies| strategies.as_slice())
            .unwrap_or(&[])
    }

    /// Determine if context compression is needed based on current context size
    pub fn should_compress_context(&self, context_size: usize) -> bool {
        context_size > self.context_compression_threshold
    }

    /// Generate a context preservation plan
    pub fn generate_context_preservation_plan(
        &self,
        context_size: usize,
        error_count: usize,
    ) -> ContextPreservationPlan {
        let compression_needed = context_size > self.context_compression_threshold;
        let critical_errors = error_count > 5;

        let strategies = if critical_errors {
            vec![
                PreservationStrategy::ImmediateCompression { target_ratio: 0.5 },
                PreservationStrategy::SelectiveRetention {
                    preserve_decisions: true,
                    preserve_errors: true,
                },
                PreservationStrategy::ContextReset {
                    preserve_session_data: true,
                },
            ]
        } else if compression_needed {
            vec![
                PreservationStrategy::GradualCompression { target_ratio: 0.7 },
                PreservationStrategy::SelectiveRetention {
                    preserve_decisions: true,
                    preserve_errors: false,
                },
            ]
        } else {
            vec![PreservationStrategy::NoAction]
        };

        ContextPreservationPlan {
            current_context_size: context_size,
            error_count,
            recommended_strategies: strategies,
            urgency: if critical_errors {
                Urgency::Critical
            } else if compression_needed {
                Urgency::High
            } else {
                Urgency::Low
            },
        }
    }

    /// Get error statistics
    pub fn get_error_statistics(&self) -> ErrorStatistics {
        let total_errors = self.errors.len();
        let resolved_errors = self.errors.iter().filter(|e| e.resolved).count();
        let unresolved_errors = total_errors - resolved_errors;

        let errors_by_type = self.errors.iter().fold(HashMap::new(), |mut acc, error| {
            *acc.entry(error.error_type.clone()).or_insert(0) += 1;
            acc
        });

        let avg_recovery_attempts = if total_errors > 0 {
            self.errors
                .iter()
                .map(|e| e.recovery_attempts.len())
                .sum::<usize>() as f64
                / total_errors as f64
        } else {
            0.0
        };

        ErrorStatistics {
            total_errors,
            resolved_errors,
            unresolved_errors,
            errors_by_type,
            avg_recovery_attempts,
            recent_errors: self.errors.iter().rev().take(5).cloned().collect(),
        }
    }

    /// Check if a specific error pattern is recurring
    pub fn detect_error_pattern(&self, error_type: &ErrorType, time_window_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let recent_errors = self
            .errors
            .iter()
            .filter(|e| e.error_type == *error_type && (now - e.timestamp) < time_window_seconds)
            .count();

        recent_errors >= 3 // Consider it a pattern if 3+ similar errors in time window
    }

    /// Get the corresponding operation type for an error type
    pub fn get_operation_type(&self, error_type: &ErrorType) -> OperationType {
        self.operation_type_mapping
            .get(error_type)
            .cloned()
            .unwrap_or(OperationType::Processing)
    }

    /// Execute an operation with intelligent timeout detection and recovery
    pub async fn execute_with_recovery<F, Fut, T>(
        &mut self,
        operation_id: String,
        error_type: ErrorType,
        _context: ErrorContext,
        operation: F,
    ) -> Result<T, anyhow::Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
    {
        let operation_type = self.get_operation_type(&error_type);

        TIMEOUT_DETECTOR.execute_with_timeout_retry(
            operation_id,
            operation_type,
            operation
        ).await
    }

    /// Check if an operation should be retried based on error analysis
    pub async fn should_retry_operation(
        &self,
        error_type: &ErrorType,
        error: &anyhow::Error,
        attempt: u32,
    ) -> bool {
        let operation_type = self.get_operation_type(error_type);
        TIMEOUT_DETECTOR.should_retry(&operation_type, error, attempt).await
    }

    /// Get timeout statistics for monitoring and optimization
    pub async fn get_timeout_stats(&self) -> crate::timeout_detector::TimeoutStats {
        TIMEOUT_DETECTOR.get_stats().await
    }

    /// Configure timeout settings for a specific error type
    pub async fn configure_timeout_for_error_type(
        &self,
        error_type: ErrorType,
        config: crate::timeout_detector::TimeoutConfig,
    ) {
        let operation_type = self.get_operation_type(&error_type);
        TIMEOUT_DETECTOR.set_config(operation_type, config).await;
    }

    /// Generate enhanced recovery plan based on timeout detector insights
    pub async fn generate_enhanced_recovery_plan(
        &self,
        context_size: usize,
        error_count: usize,
    ) -> EnhancedContextPreservationPlan {
        let timeout_stats = self.get_timeout_stats().await;
        let base_plan = self.generate_context_preservation_plan(context_size, error_count);

        // Enhance the plan based on timeout detector insights
        let timeout_rate = if timeout_stats.total_operations > 0 {
            timeout_stats.timed_out_operations as f64 / timeout_stats.total_operations as f64
        } else {
            0.0
        };

        let retry_success_rate = if timeout_stats.total_retry_attempts > 0 {
            timeout_stats.successful_retries as f64 / timeout_stats.total_retry_attempts as f64
        } else {
            1.0
        };

        // Adjust urgency based on timeout patterns
        let _adjusted_urgency = if timeout_rate > 0.3 {
            // High timeout rate indicates systemic issues
            Urgency::Critical
        } else if retry_success_rate < 0.5 {
            // Low retry success rate indicates recovery issues
            Urgency::High
        } else {
            base_plan.urgency.clone()
        };

        EnhancedContextPreservationPlan {
            base_plan,
            timeout_rate,
            retry_success_rate,
            timeout_stats,
        }
    }

    /// Get the number of errors (for context preservation plan)
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

/// Plan for preserving context during error recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPreservationPlan {
    pub current_context_size: usize,
    pub error_count: usize,
    pub recommended_strategies: Vec<PreservationStrategy>,
    pub urgency: Urgency,
}

/// Strategy for preserving context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreservationStrategy {
    ImmediateCompression {
        target_ratio: f64,
    },
    GradualCompression {
        target_ratio: f64,
    },
    SelectiveRetention {
        preserve_decisions: bool,
        preserve_errors: bool,
    },
    ContextReset {
        preserve_session_data: bool,
    },
    NoAction,
}

/// Urgency level for context preservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Urgency {
    Low,
    High,
    Critical,
}

/// Statistics about errors in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorStatistics {
    pub total_errors: usize,
    pub resolved_errors: usize,
    pub unresolved_errors: usize,
    pub errors_by_type: HashMap<ErrorType, usize>,
    pub avg_recovery_attempts: f64,
    pub recent_errors: Vec<ExecutionError>,
}

/// Enhanced context preservation plan with timeout detector insights
#[derive(Debug, Clone)]
pub struct EnhancedContextPreservationPlan {
    pub base_plan: ContextPreservationPlan,
    pub timeout_rate: f64,
    pub retry_success_rate: f64,
    pub timeout_stats: crate::timeout_detector::TimeoutStats,
}

impl Default for ErrorRecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

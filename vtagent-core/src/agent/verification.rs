//! Agent verification workflows for quality assurance in multi-agent systems
//!
//! This module provides verification mechanisms to ensure task results meet quality standards
//! before being accepted by the orchestrator.

use crate::agent::multi_agent::{AgentType, Task, TaskResults};
use crate::models::ModelId;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Verification criteria for task results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCriteria {
    /// Minimum confidence score (0.0 to 1.0)
    pub min_confidence: f64,
    /// Required completeness level (0.0 to 1.0)
    pub min_completeness: f64,
    /// Whether code compilation is required
    pub require_compilation: bool,
    /// Whether unit tests must pass
    pub require_tests: bool,
    /// Maximum allowed errors
    pub max_errors: usize,
    /// Required file formats/extensions
    pub required_formats: Vec<String>,
    /// Custom validation rules
    pub custom_rules: Vec<ValidationRule>,
}

impl Default for VerificationCriteria {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            min_completeness: 0.8,
            require_compilation: false,
            require_tests: false,
            max_errors: 0,
            required_formats: vec![],
            custom_rules: vec![],
        }
    }
}

/// Custom validation rule for task verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule identifier
    pub id: String,
    /// Rule description
    pub description: String,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Rule parameters
    pub parameters: HashMap<String, String>,
}

/// Types of validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Check file existence
    FileExists,
    /// Check content patterns
    ContentPattern,
    /// Check syntax validity
    SyntaxCheck,
    /// Check code style compliance
    StyleCheck,
    /// Check security vulnerabilities
    SecurityCheck,
    /// Custom shell command validation
    ShellCommand,
}

/// Result of verification process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether verification passed
    pub passed: bool,
    /// Overall confidence score
    pub confidence: f64,
    /// Completeness score
    pub completeness: f64,
    /// Number of errors found
    pub errors: usize,
    /// Detailed findings
    pub findings: Vec<VerificationFinding>,
    /// Verification duration
    pub duration: Duration,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Individual verification finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationFinding {
    /// Finding type
    pub finding_type: FindingType,
    /// Severity level
    pub severity: Severity,
    /// Finding description
    pub description: String,
    /// File path (if applicable)
    pub file_path: Option<String>,
    /// Line number (if applicable)
    pub line_number: Option<usize>,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Types of verification findings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FindingType {
    /// Syntax or compilation error
    SyntaxError,
    /// Logic or runtime error
    LogicError,
    /// Style or formatting issue
    StyleIssue,
    /// Security vulnerability
    SecurityVulnerability,
    /// Performance concern
    PerformanceConcern,
    /// Missing requirement
    MissingRequirement,
    /// Quality improvement suggestion
    Improvement,
}

/// Severity levels for findings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Critical issue that must be fixed
    Critical,
    /// High priority issue
    High,
    /// Medium priority issue
    Medium,
    /// Low priority issue
    Low,
    /// Informational note
    Info,
}

/// Agent verification workflow manager
pub struct VerificationWorkflow {
    /// Verification criteria for different agent types
    criteria_by_agent: HashMap<AgentType, VerificationCriteria>,
    /// Verification history
    history: Vec<VerificationRecord>,
    /// Configuration
    config: VerificationConfig,
}

/// Verification configuration
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Whether to enable peer review
    pub enable_peer_review: bool,
    /// Model to use for verification
    pub verification_model: ModelId,
    /// Maximum verification timeout
    pub max_timeout: Duration,
    /// Whether to cache verification results
    pub enable_caching: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            enable_peer_review: true,
            verification_model: ModelId::default_orchestrator(),
            max_timeout: Duration::from_secs(120),
            enable_caching: true,
        }
    }
}

/// Historical verification record
#[derive(Debug, Clone)]
pub struct VerificationRecord {
    /// Task ID that was verified
    pub task_id: String,
    /// Agent type that performed the task
    pub agent_type: AgentType,
    /// Verification result
    pub result: VerificationResult,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Verifier information
    pub verifier: VerifierInfo,
}

/// Information about who/what performed the verification
#[derive(Debug, Clone)]
pub struct VerifierInfo {
    /// Verifier type
    pub verifier_type: VerifierType,
    /// Model used for verification (if AI)
    pub model: Option<ModelId>,
    /// Agent ID (if peer review)
    pub agent_id: Option<String>,
}

/// Types of verifiers
#[derive(Debug, Clone)]
pub enum VerifierType {
    /// AI-powered verification
    AiVerification,
    /// Peer review by another agent
    PeerReview,
    /// Automated static analysis
    StaticAnalysis,
    /// Compilation check
    CompilationCheck,
    /// Test execution
    TestExecution,
}

impl VerificationWorkflow {
    /// Create a new verification workflow
    pub fn new(config: VerificationConfig) -> Self {
        let mut criteria_by_agent = HashMap::new();

        // Set default criteria for each agent type
        criteria_by_agent.insert(
            AgentType::Coder,
            VerificationCriteria {
                min_confidence: 0.8,
                min_completeness: 0.9,
                require_compilation: true,
                require_tests: false,
                max_errors: 0,
                required_formats: vec!["rs".to_string(), "py".to_string(), "js".to_string()],
                custom_rules: vec![],
            },
        );

        criteria_by_agent.insert(
            AgentType::Explorer,
            VerificationCriteria {
                min_confidence: 0.7,
                min_completeness: 0.8,
                require_compilation: false,
                require_tests: false,
                max_errors: 2,
                required_formats: vec![],
                custom_rules: vec![],
            },
        );

        criteria_by_agent.insert(
            AgentType::Orchestrator,
            VerificationCriteria {
                min_confidence: 0.9,
                min_completeness: 0.95,
                require_compilation: false,
                require_tests: false,
                max_errors: 0,
                required_formats: vec![],
                custom_rules: vec![],
            },
        );

        Self {
            criteria_by_agent,
            history: vec![],
            config,
        }
    }

    /// Verify task results against criteria
    pub async fn verify_task_results(
        &mut self,
        task: &Task,
        results: &TaskResults,
        agent_type: AgentType,
    ) -> Result<VerificationResult> {
        let start_time = SystemTime::now();

        let criteria = self
            .criteria_by_agent
            .get(&agent_type)
            .cloned()
            .unwrap_or_default();

        eprintln!(
            "Starting verification for task {} by agent type {:?}",
            task.id, agent_type
        );

        let mut findings = vec![];
        let mut confidence = 1.0;
        let mut completeness = 1.0;
        let mut errors = 0;

        // Check basic completeness
        if results.summary.trim().is_empty() {
            findings.push(VerificationFinding {
                finding_type: FindingType::MissingRequirement,
                severity: Severity::Critical,
                description: "Task summary is empty".to_string(),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Provide meaningful summary for the task".to_string()),
            });
            completeness = 0.0;
            errors += 1;
        }

        // Check for error indicators in summary and warnings
        if results.summary.to_lowercase().contains("error")
            || results.summary.to_lowercase().contains("failed")
            || !results.warnings.is_empty()
        {
            findings.push(VerificationFinding {
                finding_type: FindingType::LogicError,
                severity: Severity::High,
                description: "Task contains error indicators or warnings".to_string(),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Review and fix reported errors and warnings".to_string()),
            });
            confidence *= 0.7;
            errors += 1;
        }

        // Check summary length and detail
        if results.summary.len() < 50 {
            findings.push(VerificationFinding {
                finding_type: FindingType::MissingRequirement,
                severity: Severity::Medium,
                description: "Summary appears too brief for the task complexity".to_string(),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Provide more detailed output".to_string()),
            });
            completeness *= 0.8;
        }

        // Check if task has warnings (indicates issues)
        if !results.warnings.is_empty() {
            findings.push(VerificationFinding {
                finding_type: FindingType::LogicError,
                severity: Severity::Medium,
                description: format!("Task completed with {} warnings", results.warnings.len()),
                file_path: None,
                line_number: None,
                suggested_fix: Some("Review and address warnings".to_string()),
            });
            confidence *= 0.8;
            errors += results.warnings.len();
        }

        // Agent-specific verification
        match agent_type {
            AgentType::Coder => {
                // Check for code-specific requirements
                if !results.summary.contains("```")
                    && !results.summary.contains("fn ")
                    && !results.summary.contains("def ")
                    && !results.summary.contains("function ")
                    && results.modified_files.is_empty()
                {
                    findings.push(VerificationFinding {
                        finding_type: FindingType::MissingRequirement,
                        severity: Severity::Medium,
                        description:
                            "No code blocks or file modifications detected in coder output"
                                .to_string(),
                        file_path: None,
                        line_number: None,
                        suggested_fix: Some(
                            "Include actual code implementation or file modifications".to_string(),
                        ),
                    });
                    completeness *= 0.8;
                }
            }
            AgentType::Explorer => {
                // Check for exploration-specific requirements
                if !results.summary.contains("found")
                    && !results.summary.contains("discovered")
                    && !results.summary.contains("analysis")
                    && results.created_contexts.is_empty()
                {
                    findings.push(VerificationFinding {
                        finding_type: FindingType::MissingRequirement,
                        severity: Severity::Low,
                        description:
                            "Explorer output lacks discovery indicators or created contexts"
                                .to_string(),
                        file_path: None,
                        line_number: None,
                        suggested_fix: Some(
                            "Include findings, analysis details, or context creation".to_string(),
                        ),
                    });
                    completeness *= 0.9;
                }
            }
            _ => {}
        }

        // Apply criteria thresholds
        let passed = confidence >= criteria.min_confidence
            && completeness >= criteria.min_completeness
            && errors <= criteria.max_errors;

        let duration = start_time.elapsed().unwrap_or(Duration::from_secs(0));

        let mut recommendations = vec![];
        if confidence < criteria.min_confidence {
            recommendations.push(format!(
                "Improve confidence from {:.2} to {:.2}",
                confidence, criteria.min_confidence
            ));
        }
        if completeness < criteria.min_completeness {
            recommendations.push(format!(
                "Improve completeness from {:.2} to {:.2}",
                completeness, criteria.min_completeness
            ));
        }
        if errors > criteria.max_errors {
            recommendations.push(format!(
                "Reduce errors from {} to {}",
                errors, criteria.max_errors
            ));
        }

        let result = VerificationResult {
            passed,
            confidence,
            completeness,
            errors,
            findings,
            duration,
            recommendations,
        };

        // Record verification history
        self.history.push(VerificationRecord {
            task_id: task.id.clone(),
            agent_type,
            result: result.clone(),
            timestamp: SystemTime::now(),
            verifier: VerifierInfo {
                verifier_type: VerifierType::AiVerification,
                model: Some(self.config.verification_model.clone()),
                agent_id: None,
            },
        });

        eprintln!(
            "Verification completed for task {}: passed={}, confidence={:.2}, completeness={:.2}, errors={}",
            task.id, result.passed, result.confidence, result.completeness, result.errors
        );

        Ok(result)
    }

    /// Get verification statistics
    pub fn get_statistics(&self) -> VerificationStatistics {
        let total_verifications = self.history.len();
        let passed_verifications = self.history.iter().filter(|r| r.result.passed).count();

        let avg_confidence = if total_verifications > 0 {
            self.history
                .iter()
                .map(|r| r.result.confidence)
                .sum::<f64>()
                / total_verifications as f64
        } else {
            0.0
        };

        let avg_completeness = if total_verifications > 0 {
            self.history
                .iter()
                .map(|r| r.result.completeness)
                .sum::<f64>()
                / total_verifications as f64
        } else {
            0.0
        };

        VerificationStatistics {
            total_verifications,
            passed_verifications,
            success_rate: if total_verifications > 0 {
                passed_verifications as f64 / total_verifications as f64
            } else {
                0.0
            },
            avg_confidence,
            avg_completeness,
        }
    }

    /// Update criteria for a specific agent type
    pub fn update_criteria(&mut self, agent_type: AgentType, criteria: VerificationCriteria) {
        self.criteria_by_agent.insert(agent_type, criteria);
    }
}

/// Verification statistics
#[derive(Debug, Clone)]
pub struct VerificationStatistics {
    pub total_verifications: usize,
    pub passed_verifications: usize,
    pub success_rate: f64,
    pub avg_confidence: f64,
    pub avg_completeness: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verification_workflow() {
        let mut workflow = VerificationWorkflow::new(VerificationConfig::default());

        let task = Task {
            id: "test_task".to_string(),
            agent_type: AgentType::Coder,
            title: "Test Task".to_string(),
            description: "Test description".to_string(),
            context_refs: vec![],
            context_bootstrap: vec![],
            priority: crate::agent::multi_agent::TaskPriority::Normal,
            status: crate::agent::multi_agent::TaskStatus::Pending,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            results: None,
            created_by: "test_session".to_string(),
            dependencies: vec![],
        };

        let results = TaskResults {
            created_contexts: vec!["test_context".to_string()],
            modified_files: vec!["test.rs".to_string()],
            executed_commands: vec!["cargo test".to_string()],
            summary: "```rust\nfn test() { println!(\"Hello\"); }\n```".to_string(),
            warnings: vec![],
        };

        let verification = workflow
            .verify_task_results(&task, &results, AgentType::Coder)
            .await;
        assert!(verification.is_ok());

        let result = verification.unwrap();
        assert!(result.passed);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_verification_criteria_defaults() {
        let criteria = VerificationCriteria::default();
        assert_eq!(criteria.min_confidence, 0.7);
        assert_eq!(criteria.min_completeness, 0.8);
        assert!(!criteria.require_compilation);
    }
}

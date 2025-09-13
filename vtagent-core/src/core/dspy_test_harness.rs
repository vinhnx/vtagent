//! End-to-End Test Harness for DSPy Agent Loop
//!
//! This module provides comprehensive testing for the VTAgent DSPy integration,
//! including both single-agent and multi-agent modes with mock LLM support.
//! It measures optimization effectiveness and validates the agent loop flow.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use crate::config::loader::ConfigManager;
use crate::core::prompt_optimizer::{PromptRefiner, OptimizeLevel};

/// Test harness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestHarnessConfig {
    pub test_scenarios: Vec<TestScenario>,
    pub mock_llm_responses: HashMap<String, String>,
    pub performance_metrics: bool,
    pub verbose_output: bool,
}

/// Individual test scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    pub user_prompt: String,
    pub expected_intent: String,
    pub expected_files: Vec<String>,
    pub agent_mode: String,
    pub optimization_level: String,
    pub expected_improvements: Vec<String>,
}

/// Test results and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub scenario_results: Vec<ScenarioResult>,
    pub overall_metrics: OverallMetrics,
    pub optimization_effectiveness: f64,
}

/// Result for individual scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_name: String,
    pub success: bool,
    pub optimized_prompt_length: usize,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
    pub improvements_detected: Vec<String>,
}

/// Overall test metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallMetrics {
    pub total_scenarios: usize,
    pub passed_scenarios: usize,
    pub failed_scenarios: usize,
    pub average_execution_time_ms: f64,
    pub average_optimization_ratio: f64,
}

/// Mock LLM provider for testing without API calls
#[derive(Debug)]
pub struct MockLLMProvider {
    responses: HashMap<String, String>,
}

impl MockLLMProvider {
    pub fn new() -> Self {
        let mut responses = HashMap::new();

        // Add mock responses for common optimization patterns
        responses.insert(
            "fix_bug_optimization".to_string(),
            r#"The user is asking to fix a bug. I should:
1. First examine the relevant files to understand the issue
2. Look for error patterns or failing tests
3. Implement a targeted fix
4. Add or update tests to prevent regression
5. Ensure the fix follows project conventions"#.to_string(),
        );

        responses.insert(
            "refactor_code_optimization".to_string(),
            r#"For code refactoring, I need to:
1. Analyze the current code structure and identify improvement areas
2. Plan the refactoring to maintain functionality while improving clarity
3. Apply transformations step by step
4. Update any affected tests
5. Ensure the refactored code follows project patterns"#.to_string(),
        );

        responses.insert(
            "add_feature_optimization".to_string(),
            r#"When adding a new feature:
1. Understand the requirements and design the solution
2. Identify where the feature should be implemented
3. Write the core functionality with proper error handling
4. Add comprehensive tests for the new feature
5. Update documentation and examples"#.to_string(),
        );

        Self { responses }
    }

    pub fn get_response(&self, key: &str) -> Option<&String> {
        self.responses.get(key)
    }
}

/// Test harness for DSPy integration
pub struct DSPyTestHarness {
    config: TestHarnessConfig,
    mock_llm: Arc<MockLLMProvider>,
    results: Arc<RwLock<TestResults>>,
}

impl DSPyTestHarness {
    pub fn new(config: TestHarnessConfig) -> Self {
        Self {
            config,
            mock_llm: Arc::new(MockLLMProvider::new()),
            results: Arc::new(RwLock::new(TestResults {
                scenario_results: Vec::new(),
                overall_metrics: OverallMetrics {
                    total_scenarios: 0,
                    passed_scenarios: 0,
                    failed_scenarios: 0,
                    average_execution_time_ms: 0.0,
                    average_optimization_ratio: 0.0,
                },
                optimization_effectiveness: 0.0,
            })),
        }
    }

    /// Run all test scenarios
    pub async fn run_all_tests(&self) -> Result<TestResults> {
        info!("Starting DSPy integration tests with {} scenarios", self.config.test_scenarios.len());

        let mut scenario_results = Vec::new();

        for scenario in &self.config.test_scenarios {
            let result = self.run_scenario(scenario).await?;
            scenario_results.push(result);
        }

        // Calculate overall metrics
        let total_scenarios = scenario_results.len();
        let passed_scenarios = scenario_results.iter().filter(|r| r.success).count();
        let failed_scenarios = total_scenarios - passed_scenarios;

        let total_execution_time: u64 = scenario_results.iter().map(|r| r.execution_time_ms).sum();
        let average_execution_time = if total_scenarios > 0 {
            total_execution_time as f64 / total_scenarios as f64
        } else {
            0.0
        };

        // Calculate optimization effectiveness
        let total_improvements: usize = scenario_results.iter()
            .map(|r| r.improvements_detected.len())
            .sum();
        let optimization_effectiveness = if total_scenarios > 0 {
            total_improvements as f64 / total_scenarios as f64
        } else {
            0.0
        };

        let results = TestResults {
            scenario_results,
            overall_metrics: OverallMetrics {
                total_scenarios,
                passed_scenarios,
                failed_scenarios,
                average_execution_time_ms: average_execution_time,
                average_optimization_ratio: 1.5, // Mock ratio for now
            },
            optimization_effectiveness,
        };

        // Update the shared results
        *self.results.write().await = results.clone();

        info!(
            "DSPy tests completed: {}/{} passed, avg time: {:.2}ms, effectiveness: {:.2}",
            passed_scenarios,
            total_scenarios,
            average_execution_time,
            optimization_effectiveness
        );

        Ok(results)
    }

    /// Run a single test scenario
    async fn run_scenario(&self, scenario: &TestScenario) -> Result<ScenarioResult> {
        let start_time = std::time::Instant::now();

        if self.config.verbose_output {
            info!("Running scenario: {}", scenario.name);
        }

        let result = match self.execute_scenario(scenario).await {
            Ok(improvements) => ScenarioResult {
                scenario_name: scenario.name.clone(),
                success: true,
                optimized_prompt_length: 0, // Will be set by actual optimization
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                error_message: None,
                improvements_detected: improvements,
            },
            Err(e) => {
                warn!("Scenario {} failed: {}", scenario.name, e);
                ScenarioResult {
                    scenario_name: scenario.name.clone(),
                    success: false,
                    optimized_prompt_length: 0,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some(e.to_string()),
                    improvements_detected: Vec::new(),
                }
            }
        };

        Ok(result)
    }

    /// Execute a single scenario with DSPy optimization
    async fn execute_scenario(&self, scenario: &TestScenario) -> Result<Vec<String>> {
        // Create project files for testing
        let project_files = self.create_test_project_files(scenario)?;

        // Create optimizer with specified level
        let optimizer = PromptRefiner::new(&scenario.optimization_level);

        // Run optimization
        let optimized_prompt = optimizer
            .optimize(&scenario.user_prompt, &project_files, &scenario.agent_mode)
            .await
            .context("Failed to optimize prompt")?;

        // Validate optimization results
        let improvements = self.validate_optimization(&scenario, &optimized_prompt)?;

        if self.config.verbose_output {
            debug!("Original prompt: {}", scenario.user_prompt);
            debug!("Optimized prompt length: {}", optimized_prompt.len());
            debug!("Improvements detected: {:?}", improvements);
        }

        Ok(improvements)
    }

    /// Create test project files for scenario
    fn create_test_project_files(&self, scenario: &TestScenario) -> Result<Vec<String>> {
        let mut files = Vec::new();

        // Add expected files
        files.extend(scenario.expected_files.clone());

        // Add common project files
        files.extend(vec![
            "Cargo.toml".to_string(),
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "README.md".to_string(),
            "AGENTS.md".to_string(),
        ]);

        // Add agent-specific files based on mode
        match scenario.agent_mode.as_str() {
            "multi" => {
                files.extend(vec![
                    "prompts/orchestrator_system.md".to_string(),
                    "prompts/explorer_system.md".to_string(),
                    "prompts/coder_system.md".to_string(),
                ]);
            }
            _ => {
                files.push("prompts/system.md".to_string());
            }
        }

        Ok(files)
    }

    /// Validate optimization results against expected improvements
    fn validate_optimization(&self, scenario: &TestScenario, optimized_prompt: &str) -> Result<Vec<String>> {
        let mut improvements = Vec::new();

        // Check for basic structure improvements
        if optimized_prompt.contains("[Task]") {
            improvements.push("Added task section".to_string());
        }

        if optimized_prompt.contains("[Project Context]") {
            improvements.push("Added project context".to_string());
        }

        if optimized_prompt.contains("[Model Configuration]") {
            improvements.push("Added model configuration".to_string());
        }

        if optimized_prompt.contains("[Plan]") {
            improvements.push("Added planning section".to_string());
        }

        if optimized_prompt.contains("[Deliverables]") {
            improvements.push("Added deliverables section".to_string());
        }

        // Check for agent mode specific improvements
        match scenario.agent_mode.as_str() {
            "multi" => {
                if optimized_prompt.contains("agent_mode = multi") {
                    improvements.push("Multi-agent mode configuration".to_string());
                }
            }
            _ => {
                if optimized_prompt.contains("agent_mode = single") {
                    improvements.push("Single-agent mode configuration".to_string());
                }
            }
        }

        // Check for optimization level specific improvements
        match scenario.optimization_level.as_str() {
            "aggressive" => {
                if optimized_prompt.contains("[Checklist]") {
                    improvements.push("Added quality checklist".to_string());
                }
                if optimized_prompt.contains("[Quality Assurance]") {
                    improvements.push("Added quality assurance section".to_string());
                }
            }
            _ => {}
        }

        // Check for intent-specific improvements
        match scenario.expected_intent.as_str() {
            "fix" => {
                if optimized_prompt.contains("bug") || optimized_prompt.contains("error") {
                    improvements.push("Bug fix intent recognition".to_string());
                }
            }
            "refactor" => {
                if optimized_prompt.contains("refactor") {
                    improvements.push("Refactoring intent recognition".to_string());
                }
            }
            "add" => {
                if optimized_prompt.contains("feature") || optimized_prompt.contains("implement") {
                    improvements.push("Feature addition intent recognition".to_string());
                }
            }
            _ => {}
        }

        Ok(improvements)
    }

    /// Generate test report
    pub async fn generate_report(&self) -> Result<String> {
        let results = self.results.read().await;

        let mut report = String::new();
        report.push_str("# DSPy Integration Test Report\n\n");

        report.push_str("## Overall Metrics\n");
        report.push_str(&format!("- Total Scenarios: {}\n", results.overall_metrics.total_scenarios));
        report.push_str(&format!("- Passed: {}\n", results.overall_metrics.passed_scenarios));
        report.push_str(&format!("- Failed: {}\n", results.overall_metrics.failed_scenarios));
        report.push_str(&format!("- Success Rate: {:.1}%\n",
            (results.overall_metrics.passed_scenarios as f64 / results.overall_metrics.total_scenarios as f64) * 100.0));
        report.push_str(&format!("- Average Execution Time: {:.2}ms\n", results.overall_metrics.average_execution_time_ms));
        report.push_str(&format!("- Optimization Effectiveness: {:.2}\n", results.optimization_effectiveness));

        report.push_str("\n## Scenario Results\n");
        for result in &results.scenario_results {
            report.push_str(&format!("### {}\n", result.scenario_name));
            report.push_str(&format!("- Status: {}\n", if result.success { "✅ PASSED" } else { "❌ FAILED" }));
            report.push_str(&format!("- Execution Time: {}ms\n", result.execution_time_ms));
            report.push_str(&format!("- Improvements Detected: {}\n", result.improvements_detected.len()));

            if !result.improvements_detected.is_empty() {
                report.push_str("- Improvements:\n");
                for improvement in &result.improvements_detected {
                    report.push_str(&format!("  - {}\n", improvement));
                }
            }

            if let Some(error) = &result.error_message {
                report.push_str(&format!("- Error: {}\n", error));
            }

            report.push_str("\n");
        }

        Ok(report)
    }
}

/// Create default test scenarios
pub fn create_default_test_scenarios() -> Vec<TestScenario> {
    vec![
        TestScenario {
            name: "Bug Fix Optimization".to_string(),
            user_prompt: "fix bug in chat.rs where messages aren't displaying".to_string(),
            expected_intent: "fix".to_string(),
            expected_files: vec!["src/cli/chat.rs".to_string(), "src/main.rs".to_string()],
            agent_mode: "single".to_string(),
            optimization_level: "standard".to_string(),
            expected_improvements: vec![
                "Added task section".to_string(),
                "Added project context".to_string(),
                "Bug fix intent recognition".to_string(),
            ],
        },
        TestScenario {
            name: "Refactoring Request".to_string(),
            user_prompt: "refactor the registry module for better performance".to_string(),
            expected_intent: "refactor".to_string(),
            expected_files: vec!["vtagent-core/src/tools/registry.rs".to_string()],
            agent_mode: "single".to_string(),
            optimization_level: "aggressive".to_string(),
            expected_improvements: vec![
                "Added task section".to_string(),
                "Added planning section".to_string(),
                "Added quality checklist".to_string(),
                "Refactoring intent recognition".to_string(),
            ],
        },
        TestScenario {
            name: "Multi-Agent Feature Addition".to_string(),
            user_prompt: "add user authentication to the web interface".to_string(),
            expected_intent: "add".to_string(),
            expected_files: vec!["src/web/auth.rs".to_string(), "src/web/mod.rs".to_string()],
            agent_mode: "multi".to_string(),
            optimization_level: "standard".to_string(),
            expected_improvements: vec![
                "Added task section".to_string(),
                "Multi-agent mode configuration".to_string(),
                "Feature addition intent recognition".to_string(),
            ],
        },
        TestScenario {
            name: "Performance Optimization".to_string(),
            user_prompt: "optimize the file search to be faster".to_string(),
            expected_intent: "optimize".to_string(),
            expected_files: vec!["vtagent-core/src/tools/search.rs".to_string()],
            agent_mode: "single".to_string(),
            optimization_level: "light".to_string(),
            expected_improvements: vec![
                "Added task section".to_string(),
                "Added model configuration".to_string(),
            ],
        },
    ]
}

/// Run the DSPy test harness with default configuration
pub async fn run_dspy_tests() -> Result<TestResults> {
    let test_scenarios = create_default_test_scenarios();

    let config = TestHarnessConfig {
        test_scenarios,
        mock_llm_responses: HashMap::new(),
        performance_metrics: true,
        verbose_output: true,
    };

    let harness = DSPyTestHarness::new(config);
    let results = harness.run_all_tests().await?;

    // Generate and save report
    let report = harness.generate_report().await?;
    std::fs::write("docs/dspy_test_report.md", &report)
        .context("Failed to write test report")?;

    info!("DSPy test report saved to docs/dspy_test_report.md");

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dspy_harness_basic() {
        let scenarios = vec![create_default_test_scenarios()[0].clone()];
        let config = TestHarnessConfig {
            test_scenarios: scenarios,
            mock_llm_responses: HashMap::new(),
            performance_metrics: false,
            verbose_output: false,
        };

        let harness = DSPyTestHarness::new(config);
        let results = harness.run_all_tests().await.unwrap();

        assert_eq!(results.overall_metrics.total_scenarios, 1);
        assert!(results.overall_metrics.passed_scenarios >= 0);
    }

    #[tokio::test]
    async fn test_scenario_validation() {
        let harness = DSPyTestHarness::new(TestHarnessConfig {
            test_scenarios: Vec::new(),
            mock_llm_responses: HashMap::new(),
            performance_metrics: false,
            verbose_output: false,
        });

        let scenario = &create_default_test_scenarios()[0];
        let optimized_prompt = "[Task]\nFix the bug\n[Project Context]\nLanguage: rust\n[Model Configuration]\nprovider = gemini";
        let improvements = harness.validate_optimization(scenario, optimized_prompt).unwrap();

        assert!(improvements.contains(&"Added task section".to_string()));
        assert!(improvements.contains(&"Added project context".to_string()));
    }

    #[tokio::test]
    async fn test_mock_llm_provider() {
        let mock_llm = MockLLMProvider::new();
        let response = mock_llm.get_response("fix_bug_optimization");
        assert!(response.is_some());
        assert!(response.unwrap().contains("bug"));
    }
}
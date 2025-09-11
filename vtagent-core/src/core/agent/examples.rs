//! Multi-Agent System Usage Examples
//!
//! This module demonstrates how to use the complete multi-agent system
//! with all optimization features enabled.

use crate::config::models::ModelId;
use crate::config::{MultiAgentDefaults, ScenarioDefaults};
use crate::core::agent::integration::{MultiAgentSystem, OptimizedTaskResult};
use crate::core::agent::multi_agent::{AgentType, MultiAgentConfig};
use anyhow::Result;
use std::path::PathBuf;

/// Example usage of the complete multi-agent system
pub async fn demonstrate_multi_agent_system() -> Result<()> {
    println!("Starting Multi-Agent System Demonstration");
    println!("===========================================");

    // Create configuration with optimized settings
    let config = MultiAgentConfig {
        enable_multi_agent: MultiAgentDefaults::ENABLE_MULTI_AGENT,
        enable_task_management: MultiAgentDefaults::ENABLE_TASK_MANAGEMENT,
        enable_context_sharing: MultiAgentDefaults::ENABLE_CONTEXT_SHARING,
        enable_performance_monitoring: MultiAgentDefaults::ENABLE_PERFORMANCE_MONITORING,
        orchestrator_model: ModelId::default_orchestrator().as_str().to_string(),
        subagent_model: ModelId::default_subagent().as_str().to_string(),
        max_concurrent_subagents: MultiAgentDefaults::MAX_CONCURRENT_SUBAGENTS,
        task_timeout: MultiAgentDefaults::task_timeout(),
        context_window_size: MultiAgentDefaults::CONTEXT_WINDOW_SIZE,
        max_context_items: MultiAgentDefaults::MAX_CONTEXT_ITEMS,
        ..Default::default()
    };

    // Initialize system (requires actual API key in real usage)
    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "demo_key".to_string());

    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    println!("Configuration:");
    println!("  - Orchestrator Model: {}", config.orchestrator_model);
    println!("  - Subagent Model: {}", config.subagent_model);
    println!(
        "  - Max Concurrent Agents: {}",
        config.max_concurrent_subagents
    );
    println!("  - Task Timeout: {:?}", config.task_timeout);

    // Create multi-agent system
    let mut system = MultiAgentSystem::new(config, api_key, workspace).await?;

    println!("\n[EXEC] Executing Tasks Through Multi-Agent System");
    println!("=============================================");

    // Example 1: Code analysis task
    println!("\nTask 1: Code Analysis");
    let result1 = execute_example_task(
        &mut system,
        "Analyze Rust Code Structure",
        "Analyze the structure of the multi-agent system code and identify key components",
        AgentType::Explorer,
    )
    .await?;

    print_task_result(&result1, 1);

    // Example 2: Code implementation task
    println!("\n💻 Task 2: Code Implementation");
    let result2 = execute_example_task(
        &mut system,
        "Implement Error Handler",
        "Create a robust error handling mechanism for the agent communication system",
        AgentType::Coder,
    )
    .await?;

    print_task_result(&result2, 2);

    // Example 3: Architecture design task
    println!("\nTask 3: Architecture Design");
    let result3 = execute_example_task(
        &mut system,
        "Design Scalability Solution",
        "Design a solution for scaling the multi-agent system to handle 100+ concurrent tasks",
        AgentType::Orchestrator,
    )
    .await?;

    print_task_result(&result3, 3);

    // Show system status
    println!("[STATUS] System Status Report");
    println!("======================");
    let status = system.get_status_report().await;
    print_status_report(&status);

    // Demonstrate performance optimization
    println!("\n[PERF] Performance Optimization");
    println!("===========================");
    demonstrate_performance_features(&status).await;

    // Shutdown system gracefully
    println!("\nShutting Down System");
    println!("======================");
    system.shutdown().await?;

    println!("\nMulti-Agent System Demonstration Complete!");
    Ok(())
}

/// Execute an example task and handle the result
async fn execute_example_task(
    system: &mut MultiAgentSystem,
    title: &str,
    description: &str,
    agent_type: AgentType,
) -> Result<OptimizedTaskResult> {
    println!("  Executing: {}", title);
    println!("     Agent Type: {:?}", agent_type);
    println!("     Description: {}", description);

    let result = system
        .execute_task_optimized(title.to_string(), description.to_string(), agent_type)
        .await?;

    Ok(result)
}

/// Print formatted task result
fn print_task_result(result: &OptimizedTaskResult, task_num: usize) {
    println!("  Task {} Results:", task_num);
    println!("     Task ID: {}", result.task_id);
    println!("     Agent Used: {}", result.agent_used);
    println!("     Execution Time: {:?}", result.execution_time);
    println!("     Success: {}", result.results.warnings.is_empty());
    println!("     Verification Passed: {}", result.verification.passed);
    println!(
        "     Confidence Score: {:.2}",
        result.verification.confidence
    );
    println!(
        "     Completeness Score: {:.2}",
        result.verification.completeness
    );

    if !result.results.warnings.is_empty() {
        println!("     Warnings: {:?}", result.results.warnings);
    }

    if !result.verification.recommendations.is_empty() {
        println!("     Recommendations:");
        for rec in &result.verification.recommendations {
            println!("       - {}", rec);
        }
    }

    // Show truncated summary
    let summary = &result.results.summary;
    if summary.len() > 200 {
        println!("     Summary: {}...", &summary[..200]);
    } else {
        println!("     Summary: {}", summary);
    }
}

/// Print formatted status report
fn print_status_report(status: &crate::core::agent::integration::SystemStatusReport) {
    println!("  Session ID: {}", status.session_id);
    println!("  Uptime: {:?}", status.uptime);
    println!("  Active Tasks: {}", status.active_tasks);
    println!("  Completed Tasks: {}", status.completed_tasks);

    println!("  \n  Agent Status:");
    for (agent_type, agents) in &status.agent_statuses {
        println!("    {:?} Agents:", agent_type);
        for (id, status, stats) in agents {
            println!("      {} - {:?}", id, status);
            println!("        Tasks Completed: {}", stats.tasks_completed);
            println!("        Success Rate: {:.2}%", stats.success_rate * 100.0);
            println!(
                "        Avg Completion Time: {:?}",
                stats.avg_completion_time
            );
        }
    }

    println!("  \n  Performance Summary:");
    if let Some(success_rate) = status
        .performance_metrics
        .success_rate
        .get(&AgentType::Coder)
    {
        println!("    Coder Success Rate: {:.2}%", success_rate * 100.0);
    }
    if let Some(success_rate) = status
        .performance_metrics
        .success_rate
        .get(&AgentType::Explorer)
    {
        println!("    Explorer Success Rate: {:.2}%", success_rate * 100.0);
    }

    println!("  \n  Verification Statistics:");
    println!(
        "    Total Verifications: {}",
        status.verification_statistics.total_verifications
    );
    println!(
        "    Passed Verifications: {}",
        status.verification_statistics.passed_verifications
    );
    println!(
        "    Success Rate: {:.2}%",
        status.verification_statistics.success_rate * 100.0
    );
    println!(
        "    Avg Confidence: {:.2}",
        status.verification_statistics.avg_confidence
    );
    println!(
        "    Avg Completeness: {:.2}",
        status.verification_statistics.avg_completeness
    );
}

/// Demonstrate performance optimization features
async fn demonstrate_performance_features(
    status: &crate::core::agent::integration::SystemStatusReport,
) {
    println!("  [TARGET] Active Optimizations:");

    if !status.recommendations.is_empty() {
        println!("    Current Recommendations:");
        for (i, rec) in status.recommendations.iter().enumerate() {
            println!(
                "      {}. {} (Expected improvement: {:.1}%)",
                i + 1,
                rec.description,
                rec.expected_improvement * 100.0
            );
            println!("         Complexity: {:?}", rec.complexity);
        }
    } else {
        println!("    System running optimally - no recommendations");
    }

    println!("  \n  [INSIGHT] Performance Insights:");

    // Analyze queue performance
    let queue_stats = &status.performance_metrics.queue_stats;
    println!("    Queue Performance:");
    println!(
        "      Average Queue Length: {:.1}",
        queue_stats.avg_queue_length
    );
    println!("      Peak Queue Length: {}", queue_stats.peak_queue_length);
    println!("      Average Wait Time: {:?}", queue_stats.avg_wait_time);
    println!(
        "      Processing Rate: {:.2} tasks/min",
        queue_stats.processing_rate
    );

    // Resource utilization
    let resources = &status.performance_metrics.resource_utilization;
    println!("    Resource Utilization:");
    println!(
        "      Memory Usage: {:.1} MB (Peak: {:.1} MB)",
        resources.memory_usage.current_mb, resources.memory_usage.peak_mb
    );
    println!("      CPU Utilization: {:.1}%", resources.cpu_utilization);
    println!("      Concurrent Agents: {}", resources.concurrent_agents);

    // Model performance comparison
    println!("    Model Performance Comparison:");
    for (model_id, perf) in &status.performance_metrics.model_performance {
        println!("      {}:", model_id);
        println!("        Avg Response Time: {:?}", perf.avg_response_time);
        println!("        Success Rate: {:.2}%", perf.success_rate * 100.0);
        println!(
            "        Quality Score: {:.2}",
            perf.quality_scores.avg_confidence
        );
    }
}

/// Demonstrate real-world multi-agent scenarios
pub async fn demonstrate_real_world_scenarios() -> Result<()> {
    println!("\n🌍 Real-World Multi-Agent Scenarios");
    println!("===================================");

    // This would include more complex examples like:
    println!("  Scenario 1: Large Codebase Refactoring");
    println!("    - Explorer agents analyze codebase structure");
    println!("    - Orchestrator creates refactoring plan");
    println!("    - Coder agents implement changes in parallel");
    println!("    - Verification workflow ensures quality");

    println!("\n  Scenario 2: Documentation Generation");
    println!("    - Explorer agents scan code for functions/classes");
    println!("    - Coder agents generate documentation");
    println!("    - Orchestrator coordinates consistency");
    println!("    - Performance optimization reduces redundancy");

    println!("\n  Scenario 3: Test Suite Generation");
    println!("    - Explorer agents identify testable functions");
    println!("    - Coder agents write comprehensive tests");
    println!("    - Verification ensures test coverage");
    println!("    - Performance monitoring tracks efficiency");

    println!("\n  Scenario 4: Bug Fix Coordination");
    println!("    - Explorer agents locate bug patterns");
    println!("    - Orchestrator prioritizes fixes");
    println!("    - Coder agents implement solutions");
    println!("    - Verification prevents regressions");

    Ok(())
}

/// Configuration helper for different use cases
pub fn create_specialized_configs() -> Vec<(String, MultiAgentConfig)> {
    vec![
        (
            "High Performance".to_string(),
            MultiAgentConfig {
                enable_multi_agent: MultiAgentDefaults::ENABLE_MULTI_AGENT,
                enable_task_management: MultiAgentDefaults::ENABLE_TASK_MANAGEMENT,
                enable_context_sharing: MultiAgentDefaults::ENABLE_CONTEXT_SHARING,
                enable_performance_monitoring: MultiAgentDefaults::ENABLE_PERFORMANCE_MONITORING,
                orchestrator_model: ModelId::Gemini25Flash.as_str().to_string(),
                subagent_model: ModelId::Gemini25FlashLite.as_str().to_string(),
                max_concurrent_subagents: ScenarioDefaults::HIGH_PERF_MAX_AGENTS,
                task_timeout: ScenarioDefaults::high_perf_timeout(),
                context_window_size: ScenarioDefaults::HIGH_PERF_CONTEXT_WINDOW,
                max_context_items: ScenarioDefaults::HIGH_PERF_MAX_CONTEXTS,
                ..Default::default()
            },
        ),
        (
            "High Quality".to_string(),
            MultiAgentConfig {
                enable_multi_agent: MultiAgentDefaults::ENABLE_MULTI_AGENT,
                enable_task_management: MultiAgentDefaults::ENABLE_TASK_MANAGEMENT,
                enable_context_sharing: MultiAgentDefaults::ENABLE_CONTEXT_SHARING,
                enable_performance_monitoring: MultiAgentDefaults::ENABLE_PERFORMANCE_MONITORING,
                orchestrator_model: ModelId::Gemini25Pro.as_str().to_string(),
                subagent_model: ModelId::Gemini25Flash.as_str().to_string(),
                max_concurrent_subagents: ScenarioDefaults::HIGH_QUALITY_MAX_AGENTS,
                task_timeout: ScenarioDefaults::high_quality_timeout(),
                context_window_size: ScenarioDefaults::HIGH_QUALITY_CONTEXT_WINDOW,
                max_context_items: ScenarioDefaults::HIGH_QUALITY_MAX_CONTEXTS,
                ..Default::default()
            },
        ),
        (
            "Balanced".to_string(),
            MultiAgentConfig {
                enable_multi_agent: MultiAgentDefaults::ENABLE_MULTI_AGENT,
                enable_task_management: MultiAgentDefaults::ENABLE_TASK_MANAGEMENT,
                enable_context_sharing: MultiAgentDefaults::ENABLE_CONTEXT_SHARING,
                enable_performance_monitoring: MultiAgentDefaults::ENABLE_PERFORMANCE_MONITORING,
                orchestrator_model: ModelId::Gemini25Flash.as_str().to_string(),
                subagent_model: ModelId::Gemini25FlashLite.as_str().to_string(),
                max_concurrent_subagents: ScenarioDefaults::BALANCED_MAX_AGENTS,
                task_timeout: ScenarioDefaults::balanced_timeout(),
                context_window_size: ScenarioDefaults::BALANCED_CONTEXT_WINDOW,
                max_context_items: ScenarioDefaults::BALANCED_MAX_CONTEXTS,
                ..Default::default()
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_specialized_configs() {
        let configs = create_specialized_configs();
        assert_eq!(configs.len(), 3);

        let (name, config) = &configs[0];
        assert_eq!(name, "High Performance");
        assert_eq!(config.max_concurrent_subagents, 5);
    }

    #[tokio::test]
    async fn test_demonstration_structure() {
        // Test that the demonstration functions exist and can be called
        // In a real environment with API keys, these would actually execute

        let configs = create_specialized_configs();
        assert!(!configs.is_empty());

        // Verify configuration variety
        let performance_config = &configs[0].1;
        let quality_config = &configs[1].1;

        assert!(
            performance_config.max_concurrent_subagents > quality_config.max_concurrent_subagents
        );
        assert!(quality_config.task_timeout > performance_config.task_timeout);
    }
}

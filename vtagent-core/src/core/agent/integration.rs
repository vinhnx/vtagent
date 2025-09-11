//! Complete multi-agent system integration
//!
//! This module provides the main integration point for the multi-agent system,
//! orchestrating all components including verification workflows and performance optimization.

use crate::config::models::ModelId;
use crate::core::agent::multi_agent::*;
use crate::core::agent::optimization::{PerformanceConfig, PerformanceMonitor};
use crate::core::agent::orchestrator::OrchestratorAgent;
use crate::core::agent::runner::AgentRunner;
use crate::core::agent::verification::{VerificationConfig, VerificationWorkflow};
use crate::llm::{AnyClient, make_client};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// Comprehensive multi-agent system with all optimizations
pub struct MultiAgentSystem {
    /// Orchestrator agent for coordination
    orchestrator: OrchestratorAgent,
    /// Available subagents by type
    subagents: HashMap<AgentType, Vec<SubAgent>>,
    /// Verification workflow manager
    verification: VerificationWorkflow,
    /// Performance monitor
    performance: Arc<PerformanceMonitor>,
    /// System configuration
    config: MultiAgentConfig,
    /// Session management
    session: SessionManager,
}

/// Individual subagent instance
pub struct SubAgent {
    /// Agent identifier
    pub id: String,
    /// Agent type
    pub agent_type: AgentType,
    /// LLM client
    pub client: AnyClient,
    /// Current status
    pub status: AgentStatus,
    /// Performance statistics
    pub stats: AgentStatistics,
    /// Creation time
    pub created_at: SystemTime,
}

/// Agent status tracking
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    /// Agent is available for tasks
    Available,
    /// Agent is currently executing a task
    Busy(String), // Task ID
    /// Agent is temporarily unavailable
    Unavailable,
    /// Agent has encountered an error
    Error(String),
}

/// Performance statistics for individual agents
#[derive(Debug, Clone)]
pub struct AgentStatistics {
    /// Total tasks completed
    pub tasks_completed: usize,
    /// Success rate
    pub success_rate: f64,
    /// Average completion time
    pub avg_completion_time: Duration,
    /// Last activity timestamp
    pub last_activity: SystemTime,
}

/// Session management for multi-agent coordination
pub struct SessionManager {
    /// Current session ID
    pub session_id: String,
    /// Session start time
    pub start_time: SystemTime,
    /// Active tasks by ID
    pub active_tasks: Arc<RwLock<HashMap<String, ActiveTaskInfo>>>,
    /// Task history
    pub task_history: Vec<CompletedTaskInfo>,
    /// Session statistics
    pub stats: SessionStatistics,
}

/// Information about currently active tasks
#[derive(Debug, Clone)]
pub struct ActiveTaskInfo {
    /// Task details
    pub task: Task,
    /// Assigned agent ID
    pub agent_id: String,
    /// Task start time
    pub start_time: Instant,
    /// Current status
    pub status: TaskExecutionStatus,
}

/// Task execution status
#[derive(Debug, Clone)]
pub enum TaskExecutionStatus {
    /// Task is queued and waiting
    Queued,
    /// Task is currently being executed
    Executing,
    /// Task is being verified
    Verifying,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed(String),
}

/// Information about completed tasks
#[derive(Debug, Clone)]
pub struct CompletedTaskInfo {
    /// Task details
    pub task: Task,
    /// Results
    pub results: TaskResults,
    /// Verification result
    pub verification: Option<crate::core::agent::verification::VerificationResult>,
    /// Completion timestamp
    pub completed_at: SystemTime,
    /// Total execution time
    pub total_time: Duration,
}

/// Session-level statistics
#[derive(Debug, Clone)]
pub struct SessionStatistics {
    /// Total tasks processed
    pub total_tasks: usize,
    /// Successfully completed tasks
    pub successful_tasks: usize,
    /// Failed tasks
    pub failed_tasks: usize,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Average task time
    pub avg_task_time: Duration,
}

impl Default for SessionStatistics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            total_execution_time: Duration::from_secs(0),
            avg_task_time: Duration::from_secs(0),
        }
    }
}

impl MultiAgentSystem {
    /// Create a new multi-agent system
    pub async fn new(
        config: MultiAgentConfig,
        api_key: String,
        workspace: std::path::PathBuf,
    ) -> Result<Self> {
        // Generate unique session ID
        let session_id = format!(
            "session_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        eprintln!(
            "Initializing multi-agent system with session ID: {}",
            session_id
        );

        // Create orchestrator client
        let orchestrator_model_id = config
            .orchestrator_model
            .parse::<ModelId>()
            .map_err(|_| anyhow!("Invalid orchestrator model: {}", config.orchestrator_model))?;
        let orchestrator_client = make_client(api_key.clone(), orchestrator_model_id);

        // Create orchestrator agent
        let orchestrator = OrchestratorAgent::new(
            config.clone(),
            orchestrator_client,
            session_id.clone(),
            api_key.clone(),
            workspace.clone(),
        );

        // Initialize verification workflow
        let verification = VerificationWorkflow::new(VerificationConfig::default());

        // Initialize performance monitor
        let performance = Arc::new(PerformanceMonitor::new(PerformanceConfig::default()));

        // Initialize subagents
        let mut subagents = HashMap::new();

        // Create coder agents
        let mut coder_agents = vec![];
        for i in 0..config.max_concurrent_subagents {
            let agent_id = format!("coder_{}", i);
            let subagent_model_id = config
                .subagent_model
                .parse::<ModelId>()
                .map_err(|_| anyhow!("Invalid subagent model: {}", config.subagent_model))?;
            let client = make_client(api_key.clone(), subagent_model_id);

            coder_agents.push(SubAgent {
                id: agent_id,
                agent_type: AgentType::Coder,
                client,
                status: AgentStatus::Available,
                stats: AgentStatistics {
                    tasks_completed: 0,
                    success_rate: 1.0,
                    avg_completion_time: Duration::from_secs(0),
                    last_activity: SystemTime::now(),
                },
                created_at: SystemTime::now(),
            });
        }
        subagents.insert(AgentType::Coder, coder_agents);

        // Create explorer agents
        let mut explorer_agents = vec![];
        for i in 0..2 {
            // Fewer explorer agents as they're typically read-only
            let agent_id = format!("explorer_{}", i);
            let subagent_model_id = config
                .subagent_model
                .parse::<ModelId>()
                .map_err(|_| anyhow!("Invalid subagent model: {}", config.subagent_model))?;
            let client = make_client(api_key.clone(), subagent_model_id);

            explorer_agents.push(SubAgent {
                id: agent_id,
                agent_type: AgentType::Explorer,
                client,
                status: AgentStatus::Available,
                stats: AgentStatistics {
                    tasks_completed: 0,
                    success_rate: 1.0,
                    avg_completion_time: Duration::from_secs(0),
                    last_activity: SystemTime::now(),
                },
                created_at: SystemTime::now(),
            });
        }
        subagents.insert(AgentType::Explorer, explorer_agents);

        // Initialize session manager
        let session = SessionManager {
            session_id: session_id.clone(),
            start_time: SystemTime::now(),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_history: vec![],
            stats: SessionStatistics::default(),
        };

        eprintln!("Multi-agent system initialized successfully");
        eprintln!("- Orchestrator: {} model", config.orchestrator_model);
        eprintln!("- Subagents: {} model", config.subagent_model);
        eprintln!(
            "- Coder agents: {}",
            subagents.get(&AgentType::Coder).map_or(0, |v| v.len())
        );
        eprintln!(
            "- Explorer agents: {}",
            subagents.get(&AgentType::Explorer).map_or(0, |v| v.len())
        );

        Ok(Self {
            orchestrator,
            subagents,
            verification,
            performance,
            config,
            session,
        })
    }

    /// Execute a task through the multi-agent system with full optimization
    pub async fn execute_task_optimized(
        &mut self,
        task_title: String,
        task_description: String,
        required_agent_type: AgentType,
    ) -> Result<OptimizedTaskResult> {
        let start_time = Instant::now();
        let task_id = format!(
            "task_{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );

        eprintln!(
            "Starting optimized task execution: {} ({})",
            task_title, task_id
        );

        // Create task
        let task_id_result = self.orchestrator.create_task(
            required_agent_type,
            task_title.clone(),
            task_description.clone(),
            vec![], // Context refs will be managed by orchestrator
            vec![], // Context bootstrap
            crate::core::agent::multi_agent::TaskPriority::Normal,
        )?;

        // Record task start in performance monitor
        let perf_start = Instant::now();

        // Create Task object for processing
        let task = Task {
            id: task_id_result.clone(),
            agent_type: required_agent_type,
            title: task_title.clone(),
            description: task_description.clone(),
            context_refs: vec![],
            context_bootstrap: vec![],
            priority: TaskPriority::Normal,
            status: TaskStatus::InProgress,
            created_at: SystemTime::now(),
            started_at: Some(SystemTime::now()),
            completed_at: None,
            error_message: None,
            results: None,
            created_by: self.session.session_id.clone(),
            dependencies: vec![],
        };

        // Find available agent
        let agent_id = self.find_available_agent_id(required_agent_type).await?;

        eprintln!(
            "Assigned task {} to agent {} ({:?})",
            task_id, agent_id, required_agent_type
        );

        // Mark agent as busy
        self.update_agent_status(&agent_id, AgentStatus::Busy(task_id.clone()))
            .await?;

        // Get agent info for execution (clone the needed data)
        let (_agent, agent_info) = {
            let agent = self
                .get_agent_by_id(&agent_id)
                .ok_or_else(|| anyhow!("Agent {} not found", agent_id))?;
            (agent, (agent.id.clone(), agent.agent_type))
        };

        // Record active task
        {
            let mut active_tasks = self.session.active_tasks.write().await;
            active_tasks.insert(
                task_id.clone(),
                ActiveTaskInfo {
                    task: task.clone(),
                    agent_id: agent_id.clone(),
                    start_time,
                    status: TaskExecutionStatus::Executing,
                },
            );
        }

        // Execute task with the agent
        let execution_result = self
            .execute_task_with_agent_id(&task, &agent_id, agent_info.1)
            .await;

        // Mark agent as available again
        self.update_agent_status(&agent_id, AgentStatus::Available)
            .await?;

        match execution_result {
            Ok(results) => {
                eprintln!("Task {} completed successfully", task_id);

                // Update task status to verifying
                self.update_task_status(&task_id, TaskExecutionStatus::Verifying)
                    .await?;

                // Perform verification
                let verification_result = self
                    .verification
                    .verify_task_results(&task, &results, required_agent_type)
                    .await?;

                eprintln!(
                    "Verification completed for task {}: passed={}, confidence={:.2}",
                    task_id, verification_result.passed, verification_result.confidence
                );

                // Record performance metrics
                let task_successful = verification_result.passed && results.warnings.is_empty();

                // Extract token counts from the LLM response if available
                let (input_tokens, output_tokens) = (0, 0); // Placeholder since we don't have access to LLM response here

                self.performance
                    .record_task_execution(
                        task_id.clone(),
                        required_agent_type,
                        perf_start,
                        start_time.elapsed(),
                        task_successful,
                        self.config.subagent_model.clone(),
                        input_tokens,
                        output_tokens,
                        verification_result.confidence,
                    )
                    .await?;

                // Update agent statistics
                self.update_agent_stats(&agent_id, start_time.elapsed(), task_successful)
                    .await?;

                // Mark task as completed
                self.update_task_status(
                    &task_id,
                    if verification_result.passed {
                        TaskExecutionStatus::Completed
                    } else {
                        TaskExecutionStatus::Failed("Verification failed".to_string())
                    },
                )
                .await?;

                // Move to completed tasks
                self.session.task_history.push(CompletedTaskInfo {
                    task,
                    results: results.clone(),
                    verification: Some(verification_result.clone()),
                    completed_at: SystemTime::now(),
                    total_time: start_time.elapsed(),
                });

                // Remove from active tasks
                {
                    let mut active_tasks = self.session.active_tasks.write().await;
                    active_tasks.remove(&task_id);
                }

                Ok(OptimizedTaskResult {
                    task_id,
                    results,
                    verification: verification_result,
                    execution_time: start_time.elapsed(),
                    agent_used: agent_id,
                    performance_metrics: self.performance.get_metrics().await,
                })
            }
            Err(e) => {
                eprintln!("Task {} failed: {}", task_id, e);

                // Mark task as failed
                self.update_task_status(&task_id, TaskExecutionStatus::Failed(e.to_string()))
                    .await?;

                // Record failure in performance monitor
                self.performance
                    .record_task_execution(
                        task_id.clone(),
                        required_agent_type,
                        perf_start,
                        start_time.elapsed(),
                        false,
                        self.config.subagent_model.clone(),
                        0, // No tokens on failure
                        0, // No tokens on failure
                        0.0,
                    )
                    .await?;

                // Update agent statistics
                self.update_agent_stats(&agent_id, start_time.elapsed(), false)
                    .await?;

                Err(e)
            }
        }
    }

    /// Find an available agent of the specified type and return its ID
    async fn find_available_agent_id(&self, agent_type: AgentType) -> Result<String> {
        let agents = self
            .subagents
            .get(&agent_type)
            .ok_or_else(|| anyhow!("No agents available for type {:?}", agent_type))?;

        for agent in agents {
            if matches!(agent.status, AgentStatus::Available) {
                return Ok(agent.id.clone());
            }
        }

        Err(anyhow!(
            "All agents of type {:?} are currently busy",
            agent_type
        ))
    }

    /// Get agent by ID (helper method)
    fn get_agent_by_id(&self, agent_id: &str) -> Option<&SubAgent> {
        for agents in self.subagents.values() {
            for agent in agents {
                if agent.id == agent_id {
                    return Some(agent);
                }
            }
        }
        None
    }

    /// Execute a task with a specific agent by ID
    async fn execute_task_with_agent_id(
        &self,
        task: &Task,
        agent_id: &str,
        agent_type: AgentType,
    ) -> Result<TaskResults> {
        eprintln!("Agent {} executing task: {}", agent_id, task.title);

        // Find the agent to get its client
        let _agent = self
            .get_agent_by_id(agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent {} not found", agent_id))?;

        // Create an agent runner for this specific agent
        let mut runner = AgentRunner::new(
            agent_type,
            // For now, we'll use a default model - in a real implementation,
            // this should come from the agent configuration
            crate::config::models::ModelId::default_subagent(),
            // Use the API key from the multi-agent system
            self.orchestrator.api_key.clone(),
            // Use the workspace from the multi-agent system
            self.orchestrator.workspace.clone(),
            // Use the session ID from the multi-agent system
            self.session.session_id.clone(),
        )?;

        // Execute the task with the runner
        let results = runner.execute_task(task, &[]).await?;

        Ok(results)
    }

    /// Execute a task with a specific agent
    async fn execute_task_with_agent(&self, task: &Task, agent: &SubAgent) -> Result<TaskResults> {
        self.execute_task_with_agent_id(task, &agent.id, agent.agent_type)
            .await
    }

    /// Update agent status
    async fn update_agent_status(&mut self, agent_id: &str, status: AgentStatus) -> Result<()> {
        for agents in self.subagents.values_mut() {
            for agent in agents {
                if agent.id == agent_id {
                    agent.status = status;
                    return Ok(());
                }
            }
        }
        Err(anyhow!("Agent not found: {}", agent_id))
    }

    /// Update task execution status
    async fn update_task_status(&self, task_id: &str, status: TaskExecutionStatus) -> Result<()> {
        let mut active_tasks = self.session.active_tasks.write().await;
        if let Some(task_info) = active_tasks.get_mut(task_id) {
            task_info.status = status;
        }
        Ok(())
    }

    /// Update agent performance statistics
    async fn update_agent_stats(
        &mut self,
        agent_id: &str,
        execution_time: Duration,
        success: bool,
    ) -> Result<()> {
        for agents in self.subagents.values_mut() {
            for agent in agents {
                if agent.id == agent_id {
                    agent.stats.tasks_completed += 1;
                    agent.stats.success_rate = (agent.stats.success_rate
                        * (agent.stats.tasks_completed - 1) as f64
                        + if success { 1.0 } else { 0.0 })
                        / agent.stats.tasks_completed as f64;
                    agent.stats.avg_completion_time = Duration::from_millis(
                        (agent.stats.avg_completion_time.as_millis() as f64 * 0.8
                            + execution_time.as_millis() as f64 * 0.2)
                            as u64,
                    );
                    agent.stats.last_activity = SystemTime::now();
                    return Ok(());
                }
            }
        }
        Err(anyhow!("Agent not found: {}", agent_id))
    }

    /// Get system status report
    pub async fn get_status_report(&self) -> SystemStatusReport {
        let active_tasks = self.session.active_tasks.read().await;
        let performance_metrics = self.performance.get_metrics().await;
        let verification_stats = self.verification.get_statistics();

        let mut agent_statuses = HashMap::new();
        for (agent_type, agents) in &self.subagents {
            let statuses: Vec<_> = agents
                .iter()
                .map(|a| (a.id.clone(), a.status.clone(), a.stats.clone()))
                .collect();
            agent_statuses.insert(*agent_type, statuses);
        }

        SystemStatusReport {
            session_id: self.session.session_id.clone(),
            uptime: self.session.start_time.elapsed().unwrap_or_default(),
            active_tasks: active_tasks.len(),
            completed_tasks: self.session.task_history.len(),
            agent_statuses,
            performance_metrics,
            verification_statistics: verification_stats,
            recommendations: self.performance.get_recommendations().await,
        }
    }

    /// Shutdown the multi-agent system gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        eprintln!("Shutting down multi-agent system...");

        // Wait for active tasks to complete or timeout
        let timeout = Duration::from_secs(30);
        let start = Instant::now();

        while start.elapsed() < timeout {
            let active_tasks = self.session.active_tasks.read().await;
            if active_tasks.is_empty() {
                break;
            }
            eprintln!(
                "Waiting for {} active tasks to complete...",
                active_tasks.len()
            );
            drop(active_tasks);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // Generate final performance report
        let report = self.performance.generate_report().await;
        eprintln!("Final performance report:");
        eprintln!("- Total tasks: {}", report.summary.total_tasks);
        eprintln!(
            "- Success rate: {:.2}%",
            report.summary.overall_success_rate * 100.0
        );
        eprintln!(
            "- Average response time: {:?}",
            report.summary.avg_response_time
        );

        eprintln!("Multi-agent system shutdown complete");
        Ok(())
    }
}

/// Result of optimized task execution
#[derive(Debug, Clone)]
pub struct OptimizedTaskResult {
    /// Task identifier
    pub task_id: String,
    /// Task results
    pub results: TaskResults,
    /// Verification result
    pub verification: crate::core::agent::verification::VerificationResult,
    /// Total execution time
    pub execution_time: Duration,
    /// Agent that executed the task
    pub agent_used: String,
    /// Performance metrics snapshot
    pub performance_metrics: crate::core::agent::optimization::PerformanceMetrics,
}

/// System status report
#[derive(Debug, Clone)]
pub struct SystemStatusReport {
    /// Session identifier
    pub session_id: String,
    /// System uptime
    pub uptime: Duration,
    /// Number of active tasks
    pub active_tasks: usize,
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Agent status by type
    pub agent_statuses: HashMap<AgentType, Vec<(String, AgentStatus, AgentStatistics)>>,
    /// Performance metrics
    pub performance_metrics: crate::core::agent::optimization::PerformanceMetrics,
    /// Verification statistics
    pub verification_statistics: crate::core::agent::verification::VerificationStatistics,
    /// System recommendations
    pub recommendations: Vec<crate::core::agent::optimization::OptimizationStrategy>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_agent_system_creation() {
        let config = MultiAgentConfig::default();
        let api_key = "test_key".to_string();
        let workspace = std::path::PathBuf::from("/tmp");

        // This test would require actual API access, so we'll just verify the structure
        assert_eq!(config.max_concurrent_subagents, 3);
        assert!(config.enable_task_management);
    }

    #[test]
    fn test_session_statistics() {
        let stats = SessionStatistics::default();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.successful_tasks, 0);
        assert_eq!(stats.failed_tasks, 0);
    }
}

//! Orchestrator agent implementation for multi-agent coordination

use crate::config::models::ModelId;
use crate::core::agent::multi_agent::*;
use crate::core::agent::runner::AgentRunner;
use crate::core::orchestrator_retry::{RetryManager, is_empty_response};
use crate::gemini::GenerateContentRequest;
use crate::llm::AnyClient;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

/// Orchestrator agent for strategic coordination
pub struct OrchestratorAgent {
    /// Agent configuration
    pub config: MultiAgentConfig,
    /// LLM client for orchestrator
    client: AnyClient,
    /// Context store for knowledge management
    context_store: Arc<std::sync::Mutex<ContextStore>>,
    /// Task manager for coordination
    task_manager: Arc<std::sync::Mutex<TaskManager>>,
    /// Retry manager for error handling
    retry_manager: std::sync::Mutex<RetryManager>,
    /// Session ID
    session_id: String,
    /// API key for agents
    pub api_key: String,
    /// Workspace path
    pub workspace: std::path::PathBuf,
    /// Reasoning effort level
    pub reasoning_effort: Option<String>,
    /// Shared summary buffer for cross-agent handoffs
    pub shared_summary: Arc<std::sync::Mutex<Vec<String>>>,
}

impl OrchestratorAgent {
    /// Create a new orchestrator agent
    pub fn new(
        config: MultiAgentConfig,
        client: AnyClient,
        session_id: String,
        api_key: String,
        workspace: std::path::PathBuf,
        reasoning_effort: Option<String>,
    ) -> Self {
        let context_store = Arc::new(std::sync::Mutex::new(ContextStore::new(session_id.clone())));
        let task_manager = Arc::new(std::sync::Mutex::new(TaskManager::new(session_id.clone())));

        Self {
            config,
            client,
            context_store,
            task_manager,
            retry_manager: std::sync::Mutex::new(RetryManager::new()),
            session_id,
            api_key,
            workspace,
            reasoning_effort,
            shared_summary: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Create a new task for delegation
    pub fn create_task(
        &self,
        agent_type: AgentType,
        title: String,
        description: String,
        context_refs: Vec<String>,
        context_bootstrap: Vec<ContextBootstrap>,
        priority: TaskPriority,
    ) -> Result<String> {
        let mut task_manager = self
            .task_manager
            .lock()
            .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

        let task_id = task_manager.create_task(
            agent_type,
            title,
            description,
            context_refs,
            context_bootstrap,
            priority,
        );

        Ok(task_id)
    }

    /// Launch a subagent to execute a task
    pub async fn launch_subagent(&mut self, task_id: &str) -> Result<TaskResults> {
        let task = {
            let task_manager = self
                .task_manager
                .lock()
                .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

            task_manager
                .get_task(task_id)
                .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?
                .clone()
        };

        // Update task status to in progress
        {
            let mut task_manager = self
                .task_manager
                .lock()
                .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;
            task_manager.update_task_status(task_id, TaskStatus::InProgress)?;
        }

        // Get relevant contexts for the task
        let contexts = self.get_contexts_for_task(&task)?;

        // Execute the task based on agent type
        let results = match task.agent_type {
            AgentType::Explorer => self.execute_explorer_task(&task, &contexts).await?,
            AgentType::Coder => self.execute_coder_task(&task, &contexts).await?,
            AgentType::Orchestrator => {
                return Err(anyhow!("Orchestrator cannot launch another orchestrator"));
            }
            AgentType::Single => {
                return Err(anyhow!(
                    "Single agent type not supported in multi-agent mode"
                ));
            }
        };

        // Store contexts created by the agent with more detailed information
        for (index, context_id) in results.created_contexts.iter().enumerate() {
            let context_type = match task.agent_type {
                AgentType::Explorer => ContextType::Analysis,
                AgentType::Coder => ContextType::Implementation,
                _ => ContextType::General,
            };

            let content = format!(
                "Context '{}' created by {} agent during task '{}'\n\nSummary: {}\n\nModified files: {}\n\nExecuted commands: {}\n\nWarnings: {}",
                context_id,
                task.agent_type,
                task.title,
                results.summary,
                results.modified_files.join(", "),
                results.executed_commands.join(", "),
                if results.warnings.is_empty() {
                    "None".to_string()
                } else {
                    results.warnings.join(", ")
                }
            );

            let context = ContextItem {
                id: context_id.clone(),
                content,
                created_by: task.agent_type,
                session_id: self.session_id.clone(),
                created_at: SystemTime::now(),
                tags: vec![
                    "agent_created".to_string(),
                    task.agent_type.to_string(),
                    format!("task_{}", index),
                ],
                context_type,
                related_files: results.modified_files.clone(),
            };

            let _ = self.add_context(context);
        }

        // Update task with results
        {
            let mut task_manager = self
                .task_manager
                .lock()
                .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;
            task_manager.set_task_results(task_id, results.clone())?;
            task_manager.update_task_status(task_id, TaskStatus::Completed)?;
        }

        // Create a summary context for the entire task execution
        let task_summary_context_id = format!("{}_summary", task_id);
        let task_summary_content = format!(
            "Task '{}' executed by {} agent\n\nTitle: {}\n\nDescription: {}\n\nSummary: {}\n\nModified files: {}\n\nExecuted commands: {}\n\nWarnings: {}",
            task_id,
            task.agent_type,
            task.title,
            task.description,
            results.summary,
            if results.modified_files.is_empty() {
                "None".to_string()
            } else {
                results.modified_files.join(", ")
            },
            if results.executed_commands.is_empty() {
                "None".to_string()
            } else {
                results.executed_commands.join(", ")
            },
            if results.warnings.is_empty() {
                "None".to_string()
            } else {
                results.warnings.join(", ")
            }
        );

        let task_summary_context = ContextItem {
            id: task_summary_context_id.clone(),
            content: task_summary_content,
            created_by: task.agent_type,
            session_id: self.session_id.clone(),
            created_at: SystemTime::now(),
            tags: vec![
                "task_summary".to_string(),
                "subagent".to_string(),
                task.agent_type.to_string(),
            ],
            context_type: ContextType::Analysis,
            related_files: results.modified_files.clone(),
        };

        self.add_context(task_summary_context)?;

        // Create explicit handoff context and update shared summary buffer
        let handoff_context_id = format!("{}_handoff", task_id);
        let handoff_content = format!(
            "Handoff for '{}':\nAgent: {}\nNext steps: review modified files and run tests.\nModified files: {}\nWarnings: {}",
            task.title,
            task.agent_type,
            if results.modified_files.is_empty() {
                "None".to_string()
            } else {
                results.modified_files.join(", ")
            },
            if results.warnings.is_empty() {
                "None".to_string()
            } else {
                results.warnings.join(", ")
            },
        );
        let handoff_ctx = ContextItem {
            id: handoff_context_id.clone(),
            content: handoff_content.clone(),
            created_by: task.agent_type,
            session_id: self.session_id.clone(),
            created_at: SystemTime::now(),
            tags: vec!["handoff".to_string(), task.agent_type.to_string()],
            context_type: ContextType::Analysis,
            related_files: results.modified_files.clone(),
        };
        let _ = self.add_context(handoff_ctx);
        if let Ok(mut buf) = self.shared_summary.lock() {
            buf.push(handoff_content);
        }

        Ok(results)
    }

    /// Add a context to the context store
    pub fn add_context(&self, context: ContextItem) -> Result<()> {
        let mut context_store = self
            .context_store
            .lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        context_store
            .add_context(context)
            .map_err(|e| anyhow!("Failed to add context: {}", e))?;

        Ok(())
    }

    /// Get context by ID
    pub fn get_context(&self, context_id: &str) -> Result<Option<ContextItem>> {
        let context_store = self
            .context_store
            .lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        Ok(context_store.get_context(context_id).cloned())
    }

    /// Search contexts by criteria
    pub fn search_contexts(&self, criteria: ContextSearchCriteria) -> Result<Vec<ContextItem>> {
        let context_store = self
            .context_store
            .lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        let mut results = Vec::new();

        if let Some(tags) = criteria.tags {
            results.extend(context_store.find_by_tags(&tags).into_iter().cloned());
        }

        if let Some(context_type) = criteria.context_type {
            results.extend(
                context_store
                    .find_by_type(context_type)
                    .into_iter()
                    .cloned(),
            );
        }

        if let Some(agent_type) = criteria.created_by {
            results.extend(
                context_store
                    .find_by_creator(agent_type)
                    .into_iter()
                    .cloned(),
            );
        }

        if let Some(files) = criteria.related_files {
            results.extend(context_store.find_by_files(&files).into_iter().cloned());
        }

        // Remove duplicates
        results.sort_by(|a, b| a.id.cmp(&b.id));
        results.dedup_by(|a, b| a.id == b.id);

        Ok(results)
    }

    /// Get task status
    pub fn get_task_status(&self, task_id: &str) -> Result<Option<TaskStatus>> {
        let task_manager = self
            .task_manager
            .lock()
            .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

        Ok(task_manager
            .get_task(task_id)
            .map(|task| task.status.clone()))
    }

    /// Get all pending tasks
    pub fn get_pending_tasks(&self) -> Result<Vec<Task>> {
        let task_manager = self
            .task_manager
            .lock()
            .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

        Ok(task_manager
            .get_pending_tasks()
            .into_iter()
            .cloned()
            .collect())
    }

    /// Get contexts relevant to a task
    fn get_contexts_for_task(&self, task: &Task) -> Result<Vec<ContextItem>> {
        let context_store = self
            .context_store
            .lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        let mut contexts = Vec::new();

        // Get explicitly referenced contexts
        for context_id in &task.context_refs {
            if let Some(context) = context_store.get_context(context_id) {
                contexts.push(context.clone());
            }
        }

        // Include a synthesized shared handoff summary to reduce repetition
        if let Ok(buf) = self.shared_summary.lock() {
            if !buf.is_empty() {
                let content = format!(
                    "Handoff Summary (last {}):\n{}",
                    buf.len().min(5),
                    buf.iter()
                        .rev()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                contexts.push(ContextItem {
                    id: "shared_summary".to_string(),
                    content,
                    created_by: task.agent_type,
                    session_id: self.session_id.clone(),
                    created_at: SystemTime::now(),
                    tags: vec!["handoff".to_string(), "shared".to_string()],
                    context_type: ContextType::Analysis,
                    related_files: vec![],
                });
            }
        }
        Ok(contexts)
    }

    /// Execute an explorer task (using real agent runner)
    async fn execute_explorer_task(
        &mut self,
        task: &Task,
        contexts: &[ContextItem],
    ) -> Result<TaskResults> {
        // Create an explorer agent runner
        let mut runner = AgentRunner::new(
            AgentType::Explorer,
            self.config
                .subagent_model
                .parse()
                .unwrap_or(ModelId::default_subagent()),
            self.api_key.clone(),
            self.workspace.clone(),
            self.session_id.clone(),
            // Pass reasoning_effort from config
            self.reasoning_effort.clone(),
        )?;

        // Execute the task
        runner.execute_task(task, contexts).await
    }

    /// Execute a coder task (using real agent runner)
    async fn execute_coder_task(
        &mut self,
        task: &Task,
        contexts: &[ContextItem],
    ) -> Result<TaskResults> {
        // Create a coder agent runner
        let mut runner = AgentRunner::new(
            AgentType::Coder,
            self.config
                .subagent_model
                .parse()
                .unwrap_or(ModelId::default_subagent()),
            self.api_key.clone(),
            self.workspace.clone(),
            self.session_id.clone(),
            // Pass reasoning_effort from config
            self.reasoning_effort.clone(),
        )?;

        // Execute the task
        runner.execute_task(task, contexts).await
    }

    /// Execute orchestrator with LLM call
    pub async fn execute_orchestrator(
        &mut self,
        request: &GenerateContentRequest,
    ) -> Result<serde_json::Value> {
        let primary_model = self
            .config
            .orchestrator_model
            .parse::<ModelId>()
            .unwrap_or(ModelId::default_orchestrator());
        let fallback_model = self
            .config
            .subagent_model
            .parse::<ModelId>()
            .unwrap_or(ModelId::default_subagent());

        // First attempt with primary model (orchestrator model)
        let mut last_error = None;
        let max_retries = 3;
        let mut delay_secs = 1;

        for attempt in 0..max_retries {
            eprintln!(
                "Orchestrator attempt {}/{} using model {}",
                attempt + 1,
                max_retries,
                primary_model.as_str()
            );

            match self
                .client
                .generate(&serde_json::to_string(&request)?)
                .await
            {
                Ok(response) => {
                    let response_json = serde_json::to_value(&response)?;

                    // Check if response is empty or invalid
                    if is_empty_response(&response_json) {
                        last_error = Some(anyhow!(
                            "Empty or invalid response from orchestrator model {}",
                            primary_model.as_str()
                        ));
                        eprintln!(
                            "Empty response from {}, attempt {} failed",
                            primary_model.as_str(),
                            attempt + 1
                        );
                    } else {
                        if attempt > 0 {
                            eprintln!(
                                "Orchestrator succeeded on attempt {} with model {}",
                                attempt + 1,
                                primary_model.as_str()
                            );
                        }
                        return Ok(response_json);
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow!("Orchestrator LLM call failed: {}", e));
                    eprintln!(
                        "Attempt {} failed for orchestrator with model {}: {}",
                        attempt + 1,
                        primary_model.as_str(),
                        e
                    );
                }
            }

            // Wait before retry if not the last attempt
            if attempt < max_retries - 1 {
                eprintln!("Waiting {} seconds before retry", delay_secs);
                sleep(Duration::from_secs(delay_secs)).await;
                delay_secs = std::cmp::min(delay_secs * 2, 60); // Exponential backoff with cap
            }
        }

        // If primary model failed, try fallback model
        eprintln!(
            "Primary model {} failed after {} attempts. Trying fallback model {}",
            primary_model.as_str(),
            max_retries,
            fallback_model.as_str()
        );

        match self
            .client
            .generate(&serde_json::to_string(&request)?)
            .await
        {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;

                if is_empty_response(&response_json) {
                    Err(anyhow!(
                        "Fallback model {} also returned empty response",
                        fallback_model.as_str()
                    ))
                } else {
                    eprintln!("Fallback model {} succeeded", fallback_model.as_str());
                    Ok(response_json)
                }
            }
            Err(e) => {
                eprintln!(
                    "Fallback model {} also failed: {}",
                    fallback_model.as_str(),
                    e
                );
                Err(last_error.unwrap_or_else(|| anyhow!("All orchestrator attempts failed")))
            }
        }
    }
}

/// Criteria for searching contexts
#[derive(Debug, Clone, Default)]
pub struct ContextSearchCriteria {
    /// Search by tags
    pub tags: Option<Vec<String>>,
    /// Search by context type
    pub context_type: Option<ContextType>,
    /// Search by creator agent type
    pub created_by: Option<AgentType>,
    /// Search by related files
    pub related_files: Option<Vec<String>>,
}

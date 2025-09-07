//! Orchestrator agent implementation for multi-agent coordination

use crate::agent::multi_agent::*;
use crate::llm::AnyClient;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::SystemTime;

/// Orchestrator agent for strategic coordination
pub struct OrchestratorAgent {
    /// Agent configuration
    config: MultiAgentConfig,
    /// LLM client for orchestrator
    client: AnyClient,
    /// Context store for knowledge management
    context_store: Arc<std::sync::Mutex<ContextStore>>,
    /// Task manager for coordination
    task_manager: Arc<std::sync::Mutex<TaskManager>>,
    /// Session ID
    session_id: String,
}

impl OrchestratorAgent {
    /// Create a new orchestrator agent
    pub fn new(
        config: MultiAgentConfig,
        client: AnyClient,
        session_id: String,
    ) -> Self {
        let context_store = Arc::new(std::sync::Mutex::new(
            ContextStore::new(session_id.clone())
        ));
        let task_manager = Arc::new(std::sync::Mutex::new(
            TaskManager::new(session_id.clone())
        ));

        Self {
            config,
            client,
            context_store,
            task_manager,
            session_id,
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
        let mut task_manager = self.task_manager.lock()
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
    pub async fn launch_subagent(&self, task_id: &str) -> Result<TaskResults> {
        let task = {
            let task_manager = self.task_manager.lock()
                .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

            task_manager.get_task(task_id)
                .ok_or_else(|| anyhow!("Task '{}' not found", task_id))?
                .clone()
        };

        // Update task status to in progress
        {
            let mut task_manager = self.task_manager.lock()
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
                return Err(anyhow!("Single agent type not supported in multi-agent mode"));
            }
        };

        // Update task with results
        {
            let mut task_manager = self.task_manager.lock()
                .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;
            task_manager.set_task_results(task_id, results.clone())?;
            task_manager.update_task_status(task_id, TaskStatus::Completed)?;
        }

        // Store any new contexts created by the subagent
        for context_id in &results.created_contexts {
            // Note: This is a simplified implementation
            // In a real implementation, the subagent would return the actual context items
            self.add_context(ContextItem {
                id: context_id.clone(),
                content: format!("Context created by {} during task {}", task.agent_type, task_id),
                created_by: task.agent_type,
                session_id: self.session_id.clone(),
                created_at: SystemTime::now(),
                tags: vec!["subagent".to_string(), task.agent_type.to_string()],
                context_type: ContextType::General,
                related_files: Vec::new(),
            })?;
        }

        Ok(results)
    }

    /// Add a context to the context store
    pub fn add_context(&self, context: ContextItem) -> Result<()> {
        let mut context_store = self.context_store.lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        context_store.add_context(context)
            .map_err(|e| anyhow!("Failed to add context: {}", e))?;

        Ok(())
    }

    /// Get context by ID
    pub fn get_context(&self, context_id: &str) -> Result<Option<ContextItem>> {
        let context_store = self.context_store.lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        Ok(context_store.get_context(context_id).cloned())
    }

    /// Search contexts by criteria
    pub fn search_contexts(&self, criteria: ContextSearchCriteria) -> Result<Vec<ContextItem>> {
        let context_store = self.context_store.lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        let mut results = Vec::new();

        if let Some(tags) = criteria.tags {
            results.extend(context_store.find_by_tags(&tags).into_iter().cloned());
        }

        if let Some(context_type) = criteria.context_type {
            results.extend(context_store.find_by_type(context_type).into_iter().cloned());
        }

        if let Some(agent_type) = criteria.created_by {
            results.extend(context_store.find_by_creator(agent_type).into_iter().cloned());
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
        let task_manager = self.task_manager.lock()
            .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

        Ok(task_manager.get_task(task_id).map(|task| task.status.clone()))
    }

    /// Get all pending tasks
    pub fn get_pending_tasks(&self) -> Result<Vec<Task>> {
        let task_manager = self.task_manager.lock()
            .map_err(|e| anyhow!("Failed to lock task manager: {}", e))?;

        Ok(task_manager.get_pending_tasks().into_iter().cloned().collect())
    }

    /// Get contexts relevant to a task
    fn get_contexts_for_task(&self, task: &Task) -> Result<Vec<ContextItem>> {
        let context_store = self.context_store.lock()
            .map_err(|e| anyhow!("Failed to lock context store: {}", e))?;

        let mut contexts = Vec::new();

        // Get explicitly referenced contexts
        for context_id in &task.context_refs {
            if let Some(context) = context_store.get_context(context_id) {
                contexts.push(context.clone());
            }
        }

        Ok(contexts)
    }

    /// Execute an explorer task (simplified implementation)
    async fn execute_explorer_task(
        &self,
        task: &Task,
        _contexts: &[ContextItem],
    ) -> Result<TaskResults> {
        // This is a simplified implementation
        // In reality, this would create and run an Explorer agent

        Ok(TaskResults {
            created_contexts: vec![format!("explorer_result_{}", task.id)],
            modified_files: Vec::new(),
            executed_commands: vec!["analysis".to_string()],
            summary: format!("Explorer task '{}' completed", task.title),
            warnings: Vec::new(),
        })
    }

    /// Execute a coder task (simplified implementation)
    async fn execute_coder_task(
        &self,
        task: &Task,
        _contexts: &[ContextItem],
    ) -> Result<TaskResults> {
        // This is a simplified implementation
        // In reality, this would create and run a Coder agent

        Ok(TaskResults {
            created_contexts: vec![format!("coder_result_{}", task.id)],
            modified_files: vec!["example.rs".to_string()],
            executed_commands: vec!["cargo check".to_string()],
            summary: format!("Coder task '{}' completed", task.title),
            warnings: Vec::new(),
        })
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

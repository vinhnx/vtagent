//! Multi-agent system types and structures

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Agent types in the multi-agent system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// Strategic coordinator and persistent intelligence layer
    Orchestrator,
    /// Read-only investigation and verification specialist
    Explorer,
    /// Implementation specialist with full write access
    Coder,
    /// Single-agent mode (legacy compatibility)
    Single,
}

impl AgentType {
    /// Get the system prompt file path for this agent type
    pub fn system_prompt_path(&self) -> &'static str {
        match self {
            AgentType::Orchestrator => "prompts/orchestrator_system.md",
            AgentType::Explorer => "prompts/explorer_system.md",
            AgentType::Coder => "prompts/coder_system.md",
            AgentType::Single => "prompts/default_system.md",
        }
    }

    /// Get allowed tools for this agent type
    pub fn allowed_tools(&self) -> Vec<&'static str> {
        match self {
            AgentType::Orchestrator => vec![
                "task_create",
                "launch_subagent",
                "add_context",
                "finish",
                "context_search",
                "task_status",
            ],
            AgentType::Explorer => vec![
                "read_file",
                "grep_search",
                "run_command",
                "file_metadata",
                "project_overview",
                "tree_sitter_analyze",
                "ast_grep_search",
            ],
            AgentType::Coder => vec![
                "*", // Full access to all tools
            ],
            AgentType::Single => vec![
                "*", // Full access for backward compatibility
            ],
        }
    }

    /// Get restricted tools for this agent type
    pub fn restricted_tools(&self) -> Vec<&'static str> {
        match self {
            AgentType::Orchestrator => vec![
                "read_file",
                "write_file",
                "edit_file",
                "delete_file",
                "run_command",
            ],
            AgentType::Explorer => vec![
                "write_file",
                "edit_file",
                "delete_file",
                "create_file",
            ],
            AgentType::Coder => vec![], // No restrictions
            AgentType::Single => vec![], // No restrictions
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Orchestrator => write!(f, "orchestrator"),
            AgentType::Explorer => write!(f, "explorer"),
            AgentType::Coder => write!(f, "coder"),
            AgentType::Single => write!(f, "single"),
        }
    }
}

/// Task status in the multi-agent system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task has been created but not yet started
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed with error
    Failed,
    /// Task was cancelled before completion
    Cancelled,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Task definition for multi-agent coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: String,
    /// Agent type that should execute this task
    pub agent_type: AgentType,
    /// Human-readable task title
    pub title: String,
    /// Detailed task description and instructions
    pub description: String,
    /// Context IDs to inject into the agent's initial state
    pub context_refs: Vec<String>,
    /// Files or directories to bootstrap into agent context
    pub context_bootstrap: Vec<ContextBootstrap>,
    /// Task priority level
    pub priority: TaskPriority,
    /// Current task status
    pub status: TaskStatus,
    /// Task creation timestamp
    pub created_at: SystemTime,
    /// Task start timestamp
    pub started_at: Option<SystemTime>,
    /// Task completion timestamp
    pub completed_at: Option<SystemTime>,
    /// Error message if task failed
    pub error_message: Option<String>,
    /// Results or output from task execution
    pub results: Option<TaskResults>,
    /// Agent session ID that created this task
    pub created_by: String,
    /// Dependencies on other tasks
    pub dependencies: Vec<String>,
}

/// File or directory to bootstrap into agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBootstrap {
    /// Path to file or directory
    pub path: String,
    /// Explanation of why this context is relevant
    pub reason: String,
}

/// Results from task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResults {
    /// New contexts created during task execution
    pub created_contexts: Vec<String>,
    /// Files modified during task execution
    pub modified_files: Vec<String>,
    /// Commands executed during task
    pub executed_commands: Vec<String>,
    /// Summary of work performed
    pub summary: String,
    /// Any warnings or issues encountered
    pub warnings: Vec<String>,
}

/// Context item in the context store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Unique context identifier (should be descriptive)
    pub id: String,
    /// Context content
    pub content: String,
    /// Agent type that created this context
    pub created_by: AgentType,
    /// Session ID that created this context
    pub session_id: String,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Tags for organization and search
    pub tags: Vec<String>,
    /// Context type classification
    pub context_type: ContextType,
    /// Related file paths (if applicable)
    pub related_files: Vec<String>,
}

/// Types of contexts for organization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextType {
    /// Environmental discoveries (system configs, directory structures)
    Environmental,
    /// Diagnostic findings (errors, root causes, behaviors)
    Diagnostic,
    /// Implementation details (code changes, new features)
    Implementation,
    /// Analysis results (code quality, dependencies, patterns)
    Analysis,
    /// Strategic plans and decisions
    Strategic,
    /// Verification and test results
    Verification,
    /// General information and discoveries
    General,
}

/// Context store for persistent knowledge management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStore {
    /// All stored contexts indexed by ID
    pub contexts: HashMap<String, ContextItem>,
    /// Session ID this store belongs to
    pub session_id: String,
    /// Store creation timestamp
    pub created_at: SystemTime,
    /// Last update timestamp
    pub updated_at: SystemTime,
}

impl ContextStore {
    /// Create a new context store
    pub fn new(session_id: String) -> Self {
        let now = SystemTime::now();
        Self {
            contexts: HashMap::new(),
            session_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a context item to the store
    pub fn add_context(&mut self, mut context: ContextItem) -> Result<(), anyhow::Error> {
        // Ensure unique ID
        if self.contexts.contains_key(&context.id) {
            return Err(anyhow::anyhow!("Context with ID '{}' already exists", context.id));
        }

        context.session_id = self.session_id.clone();
        context.created_at = SystemTime::now();
        self.updated_at = SystemTime::now();

        self.contexts.insert(context.id.clone(), context);
        Ok(())
    }    /// Get context by ID
    pub fn get_context(&self, id: &str) -> Option<&ContextItem> {
        self.contexts.get(id)
    }

    /// Search contexts by tags
    pub fn find_by_tags(&self, tags: &[String]) -> Vec<&ContextItem> {
        self.contexts
            .values()
            .filter(|context| {
                tags.iter().any(|tag| context.tags.contains(tag))
            })
            .collect()
    }

    /// Search contexts by type
    pub fn find_by_type(&self, context_type: ContextType) -> Vec<&ContextItem> {
        self.contexts
            .values()
            .filter(|context| context.context_type == context_type)
            .collect()
    }

    /// Search contexts by agent type
    pub fn find_by_creator(&self, agent_type: AgentType) -> Vec<&ContextItem> {
        self.contexts
            .values()
            .filter(|context| context.created_by == agent_type)
            .collect()
    }

    /// Get contexts related to specific files
    pub fn find_by_files(&self, file_paths: &[String]) -> Vec<&ContextItem> {
        self.contexts
            .values()
            .filter(|context| {
                file_paths.iter().any(|path| {
                    context.related_files.iter().any(|related| related.contains(path))
                })
            })
            .collect()
    }

    /// Get all context IDs
    pub fn get_all_ids(&self) -> Vec<String> {
        self.contexts.keys().cloned().collect()
    }

    /// Get total number of contexts
    pub fn len(&self) -> usize {
        self.contexts.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.contexts.is_empty()
    }
}

/// Task manager for coordinating multi-agent work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManager {
    /// All tasks indexed by ID
    pub tasks: HashMap<String, Task>,
    /// Session ID this manager belongs to
    pub session_id: String,
    /// Task counter for generating unique IDs
    pub task_counter: u64,
    /// Manager creation timestamp
    pub created_at: SystemTime,
}

impl TaskManager {
    /// Create a new task manager
    pub fn new(session_id: String) -> Self {
        Self {
            tasks: HashMap::new(),
            session_id,
            task_counter: 0,
            created_at: SystemTime::now(),
        }
    }

    /// Create a new task
    pub fn create_task(
        &mut self,
        agent_type: AgentType,
        title: String,
        description: String,
        context_refs: Vec<String>,
        context_bootstrap: Vec<ContextBootstrap>,
        priority: TaskPriority,
    ) -> String {
        self.task_counter += 1;
        let task_id = format!("task_{:04}", self.task_counter);

        let task = Task {
            id: task_id.clone(),
            agent_type,
            title,
            description,
            context_refs,
            context_bootstrap,
            priority,
            status: TaskStatus::Pending,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            results: None,
            created_by: self.session_id.clone(),
            dependencies: Vec::new(),
        };

        self.tasks.insert(task_id.clone(), task);
        task_id
    }

    /// Update task status
    pub fn update_task_status(&mut self, task_id: &str, status: TaskStatus) -> Result<(), anyhow::Error> {
        let task = self.tasks.get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_id))?;

        let now = SystemTime::now();
        match status {
            TaskStatus::InProgress => {
                task.started_at = Some(now);
            }
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                task.completed_at = Some(now);
            }
            _ => {}
        }

        task.status = status;
        Ok(())
    }

    /// Set task results
    pub fn set_task_results(&mut self, task_id: &str, results: TaskResults) -> Result<(), anyhow::Error> {
        let task = self.tasks.get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_id))?;

        task.results = Some(results);
        Ok(())
    }

    /// Set task error
    pub fn set_task_error(&mut self, task_id: &str, error: String) -> Result<(), anyhow::Error> {
        let task = self.tasks.get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_id))?;

        task.error_message = Some(error);
        task.status = TaskStatus::Failed;
        task.completed_at = Some(SystemTime::now());
        Ok(())
    }    /// Get task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Get tasks by status
    pub fn get_tasks_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|task| task.status == status)
            .collect()
    }

    /// Get tasks by agent type
    pub fn get_tasks_by_agent(&self, agent_type: AgentType) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|task| task.agent_type == agent_type)
            .collect()
    }

    /// Get all task IDs
    pub fn get_all_task_ids(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    /// Get pending tasks sorted by priority
    pub fn get_pending_tasks(&self) -> Vec<&Task> {
        let mut tasks: Vec<&Task> = self.tasks
            .values()
            .filter(|task| task.status == TaskStatus::Pending)
            .collect();

        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks
    }
}

/// Multi-agent execution mode
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Single agent handles all tasks
    Single,
    /// Multi-agent coordination with orchestrator
    Multi,
    /// Automatic mode selection based on task complexity
    Auto,
}

/// Multi-agent system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentConfig {
    /// Execution mode
    pub execution_mode: ExecutionMode,
    /// Model to use for orchestrator agent
    pub orchestrator_model: String,
    /// Model to use for subagents
    pub subagent_model: String,
    /// Maximum concurrent subagents
    pub max_concurrent_subagents: usize,
    /// Enable context store
    pub context_store_enabled: bool,
    /// Enable task management
    pub enable_task_management: bool,
    /// Verification strategy
    pub verification_strategy: VerificationStrategy,
    /// Delegation strategy
    pub delegation_strategy: DelegationStrategy,
    /// Context store configuration
    pub context_store: ContextStoreConfig,
}

/// Verification strategy for multi-agent execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStrategy {
    /// Always verify implementations with explorer agents
    Always,
    /// Only verify complex implementations
    ComplexOnly,
    /// Never verify (fast but risky)
    Never,
}

/// Delegation strategy for task distribution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationStrategy {
    /// Adaptive delegation based on task complexity
    Adaptive,
    /// Conservative approach with more verification
    Conservative,
    /// Aggressive approach with minimal verification
    Aggressive,
}

/// Context store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStoreConfig {
    /// Maximum number of contexts to store
    pub max_contexts: usize,
    /// Auto-cleanup after days
    pub auto_cleanup_days: u64,
    /// Enable persistence to disk
    pub enable_persistence: bool,
    /// Enable context compression
    pub compression_enabled: bool,
    /// Storage directory for persistent contexts
    pub storage_dir: String,
}

impl Default for MultiAgentConfig {
    fn default() -> Self {
        Self {
            execution_mode: ExecutionMode::Auto,
            orchestrator_model: "gemini-1.5-pro".to_string(),
            subagent_model: "gemini-1.5-flash".to_string(),
            max_concurrent_subagents: 3,
            context_store_enabled: true,
            enable_task_management: true,
            verification_strategy: VerificationStrategy::Always,
            delegation_strategy: DelegationStrategy::Adaptive,
            context_store: ContextStoreConfig {
                max_contexts: 1000,
                auto_cleanup_days: 7,
                enable_persistence: true,
                compression_enabled: true,
                storage_dir: ".vtagent/contexts".to_string(),
            },
        }
    }
}

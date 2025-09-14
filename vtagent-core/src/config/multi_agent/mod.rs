use serde::{Deserialize, Serialize};

/// Execution mode for multi-agent system
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum ExecutionMode {
    /// Single agent mode
    #[default]
    Single,
    /// Multi-agent coordination with orchestrator
    Multi,
    /// Automatic mode selection based on task complexity
    Auto,
}

/// Multi-agent system configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiAgentSystemConfig {
    /// Enable multi-agent mode
    #[serde(default)]
    pub enabled: bool,

    /// Use single model for all agents when multi-agent is enabled (default: true)
    #[serde(default = "default_true")]
    pub use_single_model: bool,

    /// Model to use for orchestrator agent
    #[serde(default)]
    pub orchestrator_model: String,

    /// Model to use for executor agent (used for single-agent mode and as subagents in multi-agent mode)
    #[serde(default)]
    pub executor_model: String,

    /// Maximum concurrent subagents
    #[serde(default = "default_max_concurrent_subagents")]
    pub max_concurrent_subagents: usize,

    /// Enable context sharing between agents
    #[serde(default = "default_true")]
    pub context_sharing_enabled: bool,

    /// Task execution timeout in seconds
    #[serde(default = "default_task_timeout_seconds")]
    pub task_timeout_seconds: u64,
}

impl Default for MultiAgentSystemConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_single_model: default_true(),
            orchestrator_model: "gemini-2.5-flash-lite".to_string(),
            executor_model: String::new(),
            max_concurrent_subagents: default_max_concurrent_subagents(),
            context_sharing_enabled: default_true(),
            task_timeout_seconds: default_task_timeout_seconds(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_max_concurrent_subagents() -> usize {
    3
}
fn default_task_timeout_seconds() -> u64 {
    300
}

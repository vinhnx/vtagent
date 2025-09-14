use crate::config::models::Provider;
use crate::config::models::ModelId;
use crate::core::agent::multi_agent::AgentType;
use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

/// Verification strategy for multi-agent execution
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum VerificationStrategy {
    /// Always verify implementations with explorer agents
    #[default]
    Always,
    /// Verify only when confidence is low
    OnLowConfidence,
    /// Never verify (fastest but least safe)
    Never,
}

/// Delegation strategy for task distribution
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub enum DelegationStrategy {
    /// Delegate based on agent capabilities
    #[default]
    ByCapability,
    /// Delegate round-robin
    RoundRobin,
    /// Delegate to most specialized agent
    BySpecialization,
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

    /// Maximum number of agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,

    /// Context store configuration
    #[serde(default)]
    pub context_store: ContextStoreConfiguration,

    /// Agent-specific configurations
    #[serde(default)]
    pub agents: AgentSpecificConfigs,

    /// Execution mode
    #[serde(default)]
    pub execution_mode: ExecutionMode,

    /// AI provider for the agents (Gemini, OpenAI, Anthropic)
    #[serde(default)]
    pub provider: Provider,

    /// Model to use for orchestrator agent
    #[serde(default)]
    pub orchestrator_model: String,

    /// Model to use for executor agent (used for single-agent mode and as subagents in multi-agent mode)
    #[serde(default)]
    pub executor_model: String,

    /// Model to use for subagents
    #[serde(default)]
    pub subagent_model: String,

    /// Maximum concurrent subagents
    #[serde(default = "default_max_concurrent_subagents")]
    pub max_concurrent_subagents: usize,

    /// Enable context store
    #[serde(default = "default_true")]
    pub context_store_enabled: bool,

    /// Enable debug mode for verbose logging and internal state inspection
    #[serde(default)]
    pub debug_mode: bool,

    /// Task execution timeout
    #[serde(default = "default_task_timeout")]
    pub task_timeout: Duration,

    /// Enable task management
    #[serde(default = "default_true")]
    pub enable_task_management: bool,

    /// Enable context sharing between agents
    #[serde(default = "default_true")]
    pub enable_context_sharing: bool,

    /// Enable performance monitoring
    #[serde(default = "default_true")]
    pub enable_performance_monitoring: bool,

    /// Context window size for agents
    #[serde(default = "default_context_window_size")]
    pub context_window_size: usize,

    /// Maximum number of context items
    #[serde(default = "default_max_context_items")]
    pub max_context_items: usize,

    /// Verification strategy
    #[serde(default)]
    pub verification_strategy: VerificationStrategy,

    /// Delegation strategy
    #[serde(default)]
    pub delegation_strategy: DelegationStrategy,
}

impl Default for MultiAgentSystemConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            use_single_model: default_true(),
            max_agents: default_max_agents(),
            context_store: ContextStoreConfiguration::default(),
            agents: AgentSpecificConfigs::default(),
            execution_mode: ExecutionMode::Single,
            provider: Provider::Gemini,
            orchestrator_model: String::new(),
            executor_model: String::new(),
            subagent_model: String::new(),
            max_concurrent_subagents: default_max_concurrent_subagents(),
            context_store_enabled: default_true(),
            debug_mode: false,
            task_timeout: default_task_timeout(),
            enable_task_management: default_true(),
            enable_context_sharing: default_true(),
            enable_performance_monitoring: default_true(),
            context_window_size: default_context_window_size(),
            max_context_items: default_max_context_items(),
            verification_strategy: VerificationStrategy::Always,
            delegation_strategy: DelegationStrategy::ByCapability,
        }
    }
}

/// Context store configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextStoreConfiguration {
    /// Maximum context size
    #[serde(default = "default_max_context_size")]
    pub max_context_size: usize,

    /// Enable compression
    #[serde(default = "default_true")]
    pub compression_enabled: bool,
}

impl Default for ContextStoreConfiguration {
    fn default() -> Self {
        Self {
            max_context_size: default_max_context_size(),
            compression_enabled: default_true(),
        }
    }
}

/// Agent-specific configurations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSpecificConfigs {
    /// Configurations by agent type
    #[serde(default)]
    pub by_type: IndexMap<String, AgentTypeConfig>,
}

impl Default for AgentSpecificConfigs {
    fn default() -> Self {
        Self {
            by_type: IndexMap::new(),
        }
    }
}

/// Configuration for a specific agent type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentTypeConfig {
    /// Model to use for this agent type
    pub model: Option<String>,

    /// System instruction for this agent type
    pub system_instruction: Option<String>,

    /// Maximum steps for this agent type
    pub max_steps: Option<usize>,
}

impl Default for AgentTypeConfig {
    fn default() -> Self {
        Self {
            model: None,
            system_instruction: None,
            max_steps: None,
        }
    }
}

fn default_max_agents() -> usize {
    5
}
fn default_max_context_size() -> usize {
    100000
}
fn default_true() -> bool {
    true
}
fn default_max_concurrent_subagents() -> usize {
    3
}
fn default_task_timeout() -> Duration {
    Duration::from_secs(300) // 5 minutes
}
fn default_context_window_size() -> usize {
    32768
}
fn default_max_context_items() -> usize {
    100
}

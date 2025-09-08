use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Multi-agent system configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiAgentSystemConfig {
    /// Enable multi-agent mode
    #[serde(default)]
    pub enabled: bool,

    /// Maximum number of agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,

    /// Context store configuration
    #[serde(default)]
    pub context_store: ContextStoreConfiguration,

    /// Agent-specific configurations
    #[serde(default)]
    pub agents: AgentSpecificConfigs,
}

impl Default for MultiAgentSystemConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_agents: default_max_agents(),
            context_store: ContextStoreConfiguration::default(),
            agents: AgentSpecificConfigs::default(),
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
    pub by_type: HashMap<String, AgentTypeConfig>,
}

impl Default for AgentSpecificConfigs {
    fn default() -> Self {
        Self {
            by_type: HashMap::new(),
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

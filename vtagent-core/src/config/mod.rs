//! VTAgent Configuration Module
//!
//! This module handles loading and managing configuration from vtagent.toml files.
//! It provides a centralized way to manage agent policies, tool permissions, and
//! command allow lists.

pub mod core;
pub mod defaults;
pub mod loader;
pub mod multi_agent;

// Re-export main types for backward compatibility
pub use core::{AgentConfig, CommandsConfig, SecurityConfig, ToolPolicy, ToolsConfig};
pub use defaults::{
    ContextStoreDefaults, MultiAgentDefaults, PerformanceDefaults, ScenarioDefaults,
};
pub use loader::{ConfigManager, VTAgentConfig};
pub use multi_agent::{
    AgentSpecificConfigs, AgentTypeConfig, ContextStoreConfiguration, MultiAgentSystemConfig,
};

use serde::{Deserialize, Serialize};

/// PTY configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PtyConfig {
    /// Enable PTY functionality
    #[serde(default = "default_pty_enabled")]
    pub enabled: bool,

    /// Default terminal rows
    #[serde(default = "default_pty_rows")]
    pub default_rows: u16,

    /// Default terminal columns
    #[serde(default = "default_pty_cols")]
    pub default_cols: u16,

    /// Maximum number of concurrent PTY sessions
    #[serde(default = "default_max_pty_sessions")]
    pub max_sessions: usize,

    /// Command timeout in seconds
    #[serde(default = "default_pty_timeout")]
    pub command_timeout_seconds: u64,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            enabled: default_pty_enabled(),
            default_rows: default_pty_rows(),
            default_cols: default_pty_cols(),
            max_sessions: default_max_pty_sessions(),
            command_timeout_seconds: default_pty_timeout(),
        }
    }
}

fn default_pty_enabled() -> bool {
    true
}
fn default_pty_rows() -> u16 {
    24
}
fn default_pty_cols() -> u16 {
    80
}
fn default_max_pty_sessions() -> usize {
    10
}
fn default_pty_timeout() -> u64 {
    300
}

//! VTCode Configuration Module
//!
//! This module handles loading and managing configuration from vtcode.toml files.
//! It provides a centralized way to manage agent policies, tool permissions, and
//! command allow lists.

pub mod api_keys;
pub mod constants;
pub mod context;
pub mod core;
pub mod defaults;
pub mod loader;
pub mod models;
pub mod router;
pub mod telemetry;
pub mod types;

// Re-export main types for backward compatibility
pub use context::{ContextFeaturesConfig, LedgerConfig};
pub use core::{
    AgentConfig, AutomationConfig, CommandsConfig, FullAutoConfig, SecurityConfig, ToolPolicy,
    ToolsConfig,
};
pub use defaults::{ContextStoreDefaults, PerformanceDefaults, ScenarioDefaults};
pub use loader::{ConfigManager, VTCodeConfig};
pub use router::{ComplexityModelMap, ResourceBudget, RouterConfig};
pub use telemetry::TelemetryConfig;

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

//! VTAgent Configuration Module
//!
//! This module handles loading and managing configuration from vtagent.toml files.
//! It provides a centralized way to manage agent policies, tool permissions, and
//! command allow lists.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Main configuration structure for VTAgent
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VTAgentConfig {
    /// Agent-wide settings
    #[serde(default)]
    pub agent: AgentConfig,

    /// Tool execution policies
    #[serde(default)]
    pub tools: ToolsConfig,

    /// Unix command permissions
    #[serde(default)]
    pub commands: CommandsConfig,

    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,

    /// PTY settings
    #[serde(default)]
    pub pty: PtyConfig,

    /// Multi-agent system configuration
    #[serde(default)]
    pub multi_agent: MultiAgentSystemConfig,
}

/// Agent-wide configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Maximum number of conversation turns before auto-termination
    #[serde(default = "default_max_conversation_turns")]
    pub max_conversation_turns: usize,

    /// Maximum session duration in minutes
    #[serde(default = "default_max_session_duration_minutes")]
    pub max_session_duration_minutes: u64,

    /// Enable verbose logging
    #[serde(default)]
    pub verbose_logging: bool,

    /// Maximum conversation history to keep
    #[serde(default = "default_max_conversation_history")]
    pub max_conversation_history: usize,

    /// Maximum steps per turn
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,

    /// Maximum empty responses before terminating
    #[serde(default = "default_max_empty_responses")]
    pub max_empty_responses: usize,

    /// Default Gemini model to use
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Default API key environment variable
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Default system instruction fallback
    #[serde(default = "default_system_instruction")]
    pub default_system_instruction: String,
}
/// Tools configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolsConfig {
    /// Default policy for tools not explicitly listed
    #[serde(default = "default_tool_policy")]
    pub default_policy: ToolPolicy,

    /// Specific tool policies
    #[serde(default)]
    pub policies: HashMap<String, ToolPolicy>,
}

/// Command execution configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommandsConfig {
    /// Commands that can be executed without prompting
    #[serde(default)]
    pub allow_list: Vec<String>,

    /// Commands that are always denied
    #[serde(default)]
    pub deny_list: Vec<String>,

    /// Dangerous patterns that require extra confirmation
    #[serde(default)]
    pub dangerous_patterns: Vec<String>,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    /// Require human confirmation for critical actions
    #[serde(default = "default_true")]
    pub human_in_the_loop: bool,

    /// Confirm destructive actions
    #[serde(default = "default_true")]
    pub confirm_destructive_actions: bool,

    /// Log all commands
    #[serde(default = "default_true")]
    pub log_all_commands: bool,

    /// Maximum file size in MB
    #[serde(default = "default_max_file_size_mb")]
    pub max_file_size_mb: u64,

    /// Allowed file extensions
    #[serde(default)]
    pub allowed_file_extensions: Vec<String>,
}

/// PTY configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PtyConfig {
    /// Enable PTY functionality
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Default terminal rows
    #[serde(default = "default_pty_rows")]
    pub default_rows: u16,

    /// Default terminal columns
    #[serde(default = "default_pty_cols")]
    pub default_cols: u16,

    /// Maximum PTY sessions allowed
    #[serde(default = "default_max_pty_sessions")]
    pub max_sessions: usize,

    /// Timeout for PTY commands in seconds
    #[serde(default = "default_pty_timeout_seconds")]
    pub command_timeout_seconds: u64,
}

/// Multi-agent system configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MultiAgentSystemConfig {
    /// Enable multi-agent execution mode
    #[serde(default)]
    pub enabled: bool,

    /// Execution mode (single, multi, auto)
    #[serde(default = "default_execution_mode")]
    pub execution_mode: String,

    /// Model to use for orchestrator agent
    #[serde(default = "default_orchestrator_model")]
    pub orchestrator_model: String,

    /// Model to use for subagents
    #[serde(default = "default_subagent_model")]
    pub subagent_model: String,

    /// Maximum concurrent subagents
    #[serde(default = "default_max_concurrent_subagents")]
    pub max_concurrent_subagents: usize,

    /// Enable context store
    #[serde(default = "default_true")]
    pub context_store_enabled: bool,

    /// Enable task management
    #[serde(default = "default_true")]
    pub enable_task_management: bool,

    /// Verification strategy (always, complex_only, never)
    #[serde(default = "default_verification_strategy")]
    pub verification_strategy: String,

    /// Delegation strategy (adaptive, conservative, aggressive)
    #[serde(default = "default_delegation_strategy")]
    pub delegation_strategy: String,

    /// Context store configuration
    #[serde(default)]
    pub context_store: ContextStoreConfiguration,

    /// Agent-specific configurations
    #[serde(default)]
    pub agents: AgentSpecificConfigs,
}

/// Context store configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextStoreConfiguration {
    /// Maximum number of contexts to store
    #[serde(default = "default_max_contexts")]
    pub max_contexts: usize,

    /// Auto-cleanup after days
    #[serde(default = "default_auto_cleanup_days")]
    pub auto_cleanup_days: u64,

    /// Enable persistence to disk
    #[serde(default = "default_true")]
    pub enable_persistence: bool,

    /// Enable context compression
    #[serde(default = "default_true")]
    pub compression_enabled: bool,

    /// Storage directory for persistent contexts
    #[serde(default = "default_storage_dir")]
    pub storage_dir: String,
}

/// Agent-specific configurations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSpecificConfigs {
    /// Orchestrator agent configuration
    #[serde(default)]
    pub orchestrator: AgentTypeConfig,

    /// Explorer agent configuration
    #[serde(default)]
    pub explorer: AgentTypeConfig,

    /// Coder agent configuration
    #[serde(default)]
    pub coder: AgentTypeConfig,
}

/// Configuration for a specific agent type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentTypeConfig {
    /// Tools allowed for this agent type
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Tools restricted for this agent type
    #[serde(default)]
    pub restricted_tools: Vec<String>,

    /// Maximum execution time for tasks (seconds)
    #[serde(default = "default_max_task_time")]
    pub max_task_time_seconds: u64,

    /// Maximum context window size
    #[serde(default = "default_max_context_window")]
    pub max_context_window: usize,
}

/// Tool execution policy
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolPolicy {
    /// Allow automatic execution
    Allow,
    /// Prompt user for confirmation
    Prompt,
    /// Always deny execution
    Deny,
}

impl Default for VTAgentConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            tools: ToolsConfig::default(),
            commands: CommandsConfig::default(),
            security: SecurityConfig::default(),
            pty: PtyConfig::default(),
            multi_agent: MultiAgentSystemConfig::default(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_conversation_turns: default_max_conversation_turns(),
            max_session_duration_minutes: default_max_session_duration_minutes(),
            verbose_logging: false,
            max_conversation_history: default_max_conversation_history(),
            max_steps: default_max_steps(),
            max_empty_responses: default_max_empty_responses(),
            default_model: default_model(),
            api_key_env: default_api_key_env(),
            default_system_instruction: default_system_instruction(),
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            default_policy: default_tool_policy(),
            policies: HashMap::new(),
        }
    }
}

impl Default for CommandsConfig {
    fn default() -> Self {
        Self {
            allow_list: default_allow_list(),
            deny_list: default_deny_list(),
            dangerous_patterns: default_dangerous_patterns(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            human_in_the_loop: default_true(),
            confirm_destructive_actions: default_true(),
            log_all_commands: default_true(),
            max_file_size_mb: default_max_file_size_mb(),
            allowed_file_extensions: vec![],
        }
    }
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            default_rows: default_pty_rows(),
            default_cols: default_pty_cols(),
            max_sessions: default_max_pty_sessions(),
            command_timeout_seconds: default_pty_timeout_seconds(),
        }
    }
}

impl Default for MultiAgentSystemConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            execution_mode: default_execution_mode(),
            orchestrator_model: default_orchestrator_model(),
            subagent_model: default_subagent_model(),
            max_concurrent_subagents: default_max_concurrent_subagents(),
            context_store_enabled: default_true(),
            enable_task_management: default_true(),
            verification_strategy: default_verification_strategy(),
            delegation_strategy: default_delegation_strategy(),
            context_store: ContextStoreConfiguration::default(),
            agents: AgentSpecificConfigs::default(),
        }
    }
}

impl Default for ContextStoreConfiguration {
    fn default() -> Self {
        Self {
            max_contexts: default_max_contexts(),
            auto_cleanup_days: default_auto_cleanup_days(),
            enable_persistence: default_true(),
            compression_enabled: default_true(),
            storage_dir: default_storage_dir(),
        }
    }
}

impl Default for AgentSpecificConfigs {
    fn default() -> Self {
        Self {
            orchestrator: AgentTypeConfig {
                allowed_tools: vec![
                    "task_create".to_string(),
                    "launch_subagent".to_string(),
                    "add_context".to_string(),
                    "context_search".to_string(),
                    "task_status".to_string(),
                    "finish".to_string(),
                ],
                restricted_tools: vec![
                    "read_file".to_string(),
                    "write_file".to_string(),
                    "edit_file".to_string(),
                    "run_command".to_string(),
                ],
                max_task_time_seconds: default_max_task_time(),
                max_context_window: default_max_context_window(),
            },
            explorer: AgentTypeConfig {
                allowed_tools: vec![
                    "read_file".to_string(),
                    "grep_search".to_string(),
                    "run_command".to_string(),
                    "file_metadata".to_string(),
                    "project_overview".to_string(),
                    "tree_sitter_analyze".to_string(),
                    "ast_grep_search".to_string(),
                ],
                restricted_tools: vec![
                    "write_file".to_string(),
                    "edit_file".to_string(),
                    "delete_file".to_string(),
                    "create_file".to_string(),
                ],
                max_task_time_seconds: default_max_task_time(),
                max_context_window: default_max_context_window(),
            },
            coder: AgentTypeConfig {
                allowed_tools: vec!["*".to_string()], // Full access
                restricted_tools: Vec::new(),
                max_task_time_seconds: default_max_task_time(),
                max_context_window: default_max_context_window(),
            },
        }
    }
}

impl Default for AgentTypeConfig {
    fn default() -> Self {
        Self {
            allowed_tools: Vec::new(),
            restricted_tools: Vec::new(),
            max_task_time_seconds: default_max_task_time(),
            max_context_window: default_max_context_window(),
        }
    }
}

// Default value functions
fn default_max_conversation_turns() -> usize {
    1000
}
fn default_max_session_duration_minutes() -> u64 {
    60
}
fn default_max_conversation_history() -> usize {
    100
}
/// Maximum tool calls allowed per turn
pub const MAX_TOOL_CALLS_PER_TURN: usize = 30;

fn default_max_steps() -> usize {
    MAX_TOOL_CALLS_PER_TURN
}
fn default_max_empty_responses() -> usize {
    10
}
fn default_model() -> String {
    "gemini-2.5-flash-lite".to_string()
}
fn default_api_key_env() -> String {
    "GEMINI_API_KEY".to_string()
}
fn default_system_instruction() -> String {
    "You are a helpful coding assistant.".to_string()
}
fn default_tool_policy() -> ToolPolicy {
    ToolPolicy::Prompt
}
fn default_true() -> bool {
    true
}
fn default_max_file_size_mb() -> u64 {
    50
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

fn default_pty_timeout_seconds() -> u64 {
    300
}

fn default_allow_list() -> Vec<String> {
    vec![
        "ls".to_string(),
        "pwd".to_string(),
        "cd".to_string(),
        "cat".to_string(),
        "grep".to_string(),
        "find".to_string(),
        "head".to_string(),
        "tail".to_string(),
        "wc".to_string(),
        "sort".to_string(),
        "uniq".to_string(),
        "git status".to_string(),
        "git diff".to_string(),
        "git log".to_string(),
        "git branch".to_string(),
        "git show".to_string(),
        "cargo check".to_string(),
        "cargo clippy".to_string(),
        "cargo fmt".to_string(),
    ]
}

fn default_deny_list() -> Vec<String> {
    vec![
        "rm -rf".to_string(),
        "sudo rm".to_string(),
        "format".to_string(),
        "shutdown".to_string(),
        "reboot".to_string(),
        "halt".to_string(),
        "curl | sh".to_string(),
        "wget | sh".to_string(),
        "chmod 777".to_string(),
        "passwd".to_string(),
    ]
}

fn default_dangerous_patterns() -> Vec<String> {
    vec![
        "rm -f".to_string(),
        "git reset --hard".to_string(),
        "git clean -f".to_string(),
        "docker system prune".to_string(),
        "npm install -g".to_string(),
        "pip install".to_string(),
    ]
}

fn default_allowed_extensions() -> Vec<String> {
    vec![
        ".rs".to_string(),
        ".toml".to_string(),
        ".json".to_string(),
        ".md".to_string(),
        ".txt".to_string(),
        ".yaml".to_string(),
        ".yml".to_string(),
        ".js".to_string(),
        ".ts".to_string(),
        ".py".to_string(),
    ]
}

// Multi-agent configuration defaults
fn default_execution_mode() -> String {
    "auto".to_string()
}

fn default_orchestrator_model() -> String {
    "gemini-1.5-pro".to_string()
}

fn default_subagent_model() -> String {
    "gemini-1.5-flash".to_string()
}

fn default_max_concurrent_subagents() -> usize {
    3
}

fn default_verification_strategy() -> String {
    "always".to_string()
}

fn default_delegation_strategy() -> String {
    "adaptive".to_string()
}

fn default_max_contexts() -> usize {
    1000
}

fn default_auto_cleanup_days() -> u64 {
    7
}

fn default_storage_dir() -> String {
    ".vtagent/contexts".to_string()
}

fn default_max_task_time() -> u64 {
    300 // 5 minutes
}

fn default_max_context_window() -> usize {
    32000
}

impl VTAgentConfig {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: VTAgentConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        Ok(config)
    }

    /// Load configuration with fallbacks
    /// 1. Try to load from workspace/vtagent.toml
    /// 2. Try to load from workspace/.vtagent/vtagent.toml
    /// 3. Fall back to default configuration
    pub fn load_with_fallbacks<P: AsRef<Path>>(workspace: P) -> Result<Self> {
        let workspace = workspace.as_ref();

        // Try workspace/vtagent.toml first
        let primary_config = workspace.join("vtagent.toml");
        if primary_config.exists() {
            return Self::load_from_file(primary_config);
        }

        // Try workspace/.vtagent/vtagent.toml
        let fallback_config = workspace.join(".vtagent").join("vtagent.toml");
        if fallback_config.exists() {
            return Self::load_from_file(fallback_config);
        }

        // Return default configuration
        Ok(Self::default())
    }

    /// Get policy for a specific tool
    pub fn get_tool_policy(&self, tool_name: &str) -> ToolPolicy {
        self.tools
            .policies
            .get(tool_name)
            .cloned()
            .unwrap_or_else(|| self.tools.default_policy.clone())
    }

    /// Check if a command is in the allow list
    pub fn is_command_allowed(&self, command: &str) -> bool {
        let command_lower = command.to_lowercase();

        // Check deny list first
        for denied in &self.commands.deny_list {
            if command_lower.contains(&denied.to_lowercase()) {
                return false;
            }
        }

        // Check allow list
        for allowed in &self.commands.allow_list {
            if command_lower.starts_with(&allowed.to_lowercase()) {
                return true;
            }
        }

        // If no explicit allow list, default to allowed
        self.commands.allow_list.is_empty()
    }

    /// Check if a command is considered dangerous
    pub fn is_command_dangerous(&self, command: &str) -> bool {
        let command_lower = command.to_lowercase();
        for pattern in &self.commands.dangerous_patterns {
            if command_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Check if PTY functionality is enabled
    pub fn is_pty_enabled(&self) -> bool {
        self.pty.enabled
    }

    /// Get the default terminal size for PTY sessions
    pub fn get_default_terminal_size(&self) -> (u16, u16) {
        (self.pty.default_rows, self.pty.default_cols)
    }

    /// Get the maximum number of PTY sessions allowed
    pub fn get_max_pty_sessions(&self) -> usize {
        self.pty.max_sessions
    }

    /// Get the timeout for PTY commands
    pub fn get_pty_timeout_seconds(&self) -> u64 {
        self.pty.command_timeout_seconds
    }

    /// Get session duration as std::time::Duration
    pub fn session_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.agent.max_session_duration_minutes * 60)
    }

    /// Create a sample configuration file
    pub fn create_sample_config<P: AsRef<Path>>(path: P) -> Result<()> {
        let sample_config = VTAgentConfig::default();
        let toml_content = toml::to_string_pretty(&sample_config)
            .context("Failed to serialize sample configuration")?;

        std::fs::write(&path, toml_content).with_context(|| {
            format!(
                "Failed to write sample config to: {}",
                path.as_ref().display()
            )
        })?;

        Ok(())
    }

    /// Bootstrap a new project with vtagent.toml and .vtagentgitignore
    pub fn bootstrap_project<P: AsRef<Path>>(workspace: P, force: bool) -> Result<Vec<String>> {
        let workspace = workspace.as_ref();
        let mut created_files = Vec::new();

        // Create vtagent.toml
        let config_path = workspace.join("vtagent.toml");
        if !config_path.exists() || force {
            Self::create_sample_config(&config_path)?;
            created_files.push("vtagent.toml".to_string());
        }

        // Create .vtagentgitignore
        let gitignore_path = workspace.join(".vtagentgitignore");
        if !gitignore_path.exists() || force {
            let gitignore_content = Self::default_vtagent_gitignore();
            std::fs::write(&gitignore_path, gitignore_content).with_context(|| {
                format!(
                    "Failed to write .vtagentgitignore to: {}",
                    gitignore_path.display()
                )
            })?;
            created_files.push(".vtagentgitignore".to_string());
        }

        Ok(created_files)
    }

    /// Get default .vtagentgitignore content
    fn default_vtagent_gitignore() -> String {
        r#"# .vtagentgitignore - Controls which files the agent can access
# This file works like .gitignore but only affects the agent's file operations
# It does NOT affect your project's actual .gitignore file

# Exclude log files
*.log
logs/

# Exclude build artifacts
target/
build/
dist/

# Exclude temporary files
*.tmp
*.temp
.cache/

# Exclude sensitive files
.env
.env.local
secrets/
.aws/
.ssh/

# Exclude large binary files
*.exe
*.dll
*.so
*.dylib
*.bin

# Exclude IDE files
.vscode/
.idea/
*.swp
*.swo

# Exclude node_modules (if present)
node_modules/

# Exclude dependency directories
vendor/

# Exclude database files
*.db
*.sqlite
*.sqlite3

# Allow specific important files
!important.log
!CHANGELOG.md
!README.md
"#
        .to_string()
    }
}

/// Configuration manager that handles loading and caching
pub struct ConfigManager {
    config: VTAgentConfig,
    config_path: Option<PathBuf>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new<P: AsRef<Path>>(workspace: P) -> Result<Self> {
        let config = VTAgentConfig::load_with_fallbacks(&workspace)?;
        let config_path = Self::find_config_path(&workspace);

        Ok(Self {
            config,
            config_path,
        })
    }

    /// Find the actual config file path being used
    fn find_config_path<P: AsRef<Path>>(workspace: P) -> Option<PathBuf> {
        let workspace = workspace.as_ref();

        let primary = workspace.join("vtagent.toml");
        if primary.exists() {
            return Some(primary);
        }

        let fallback = workspace.join(".vtagent").join("vtagent.toml");
        if fallback.exists() {
            return Some(fallback);
        }

        None
    }

    /// Get the current configuration
    pub fn config(&self) -> &VTAgentConfig {
        &self.config
    }

    /// Get the path to the config file being used
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Reload configuration from disk
    pub fn reload(&mut self) -> Result<()> {
        if let Some(path) = &self.config_path {
            self.config = VTAgentConfig::load_from_file(path)?;
        }
        Ok(())
    }
}

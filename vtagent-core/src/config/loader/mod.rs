use crate::config::PtyConfig;
use crate::config::core::{AgentConfig, CommandsConfig, SecurityConfig, ToolsConfig};
use crate::config::multi_agent::MultiAgentSystemConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
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

impl VTAgentConfig {
    /// Bootstrap project with config + gitignore
    pub fn bootstrap_project<P: AsRef<Path>>(workspace: P, force: bool) -> Result<Vec<String>> {
        let workspace = workspace.as_ref();
        let mut created_files = Vec::new();

        // Create vtagent.toml
        let config_path = workspace.join("vtagent.toml");
        if !config_path.exists() || force {
            let default_config = VTAgentConfig::default();
            let config_content = toml::to_string_pretty(&default_config)
                .context("Failed to serialize default configuration")?;
            
            fs::write(&config_path, config_content)
                .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
            
            created_files.push("vtagent.toml".to_string());
        }

        // Create .vtagentgitignore
        let gitignore_path = workspace.join(".vtagentgitignore");
        if !gitignore_path.exists() || force {
            let gitignore_content = Self::default_vtagent_gitignore();
            fs::write(&gitignore_path, gitignore_content)
                .with_context(|| format!("Failed to write gitignore file: {}", gitignore_path.display()))?;
            
            created_files.push(".vtagentgitignore".to_string());
        }

        Ok(created_files)
    }

    /// Generate default .vtagentgitignore content
    fn default_vtagent_gitignore() -> String {
        r#"# Security-focused exclusions
.env, .env.local, secrets/, .aws/, .ssh/

# Development artifacts
target/, build/, dist/, node_modules/, vendor/

# Database files
*.db, *.sqlite, *.sqlite3

# Binary files
*.exe, *.dll, *.so, *.dylib, *.bin

# IDE files (comprehensive)
.vscode/, .idea/, *.swp, *.swo
"#.to_string()
    }

    /// Create sample configuration file
    pub fn create_sample_config<P: AsRef<Path>>(output: P) -> Result<()> {
        let output = output.as_ref();
        let default_config = VTAgentConfig::default();
        let config_content = toml::to_string_pretty(&default_config)
            .context("Failed to serialize default configuration")?;
        
        fs::write(output, config_content)
            .with_context(|| format!("Failed to write config file: {}", output.display()))?;
        
        Ok(())
    }
}

/// Configuration manager for loading and validating configurations
pub struct ConfigManager {
    config: VTAgentConfig,
    config_path: Option<PathBuf>,
}

impl ConfigManager {
    /// Load configuration from the default locations
    pub fn load() -> Result<Self> {
        Self::load_from_workspace(std::env::current_dir()?)
    }

    /// Load configuration from a specific workspace
    pub fn load_from_workspace(workspace: impl AsRef<Path>) -> Result<Self> {
        let workspace = workspace.as_ref();

        // Try vtagent.toml in workspace root first
        let config_path = workspace.join("vtagent.toml");
        if config_path.exists() {
            return Self::load_from_file(&config_path);
        }

        // Try .vtagent/vtagent.toml as fallback
        let fallback_path = workspace.join(".vtagent").join("vtagent.toml");
        if fallback_path.exists() {
            return Self::load_from_file(&fallback_path);
        }

        // Use default configuration if no file found
        Ok(Self {
            config: VTAgentConfig::default(),
            config_path: None,
        })
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: VTAgentConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        Ok(Self {
            config,
            config_path: Some(path.to_path_buf()),
        })
    }

    /// Get the loaded configuration
    pub fn config(&self) -> &VTAgentConfig {
        &self.config
    }

    /// Get the configuration file path (if loaded from file)
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Get session duration from agent config
    pub fn session_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.agent.max_session_duration_minutes * 60)
    }
}

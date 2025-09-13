//! Tool policy management system
//!
//! This module manages user preferences for tool usage, storing choices in
//! ~/.vtagent/tool-policy.json to minimize repeated prompts while maintaining
//! user control over which tools the agent can use.

use anyhow::{Context, Result};
use console::style;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Tool execution policy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolPolicy {
    /// Allow tool execution without prompting
    Allow,
    /// Prompt user for confirmation each time
    Prompt,
    /// Never allow tool execution
    Deny,
}

impl Default for ToolPolicy {
    fn default() -> Self {
        ToolPolicy::Prompt
    }
}

/// Tool policy configuration stored in ~/.vtagent/tool-policy.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicyConfig {
    /// Configuration version for future compatibility
    pub version: u32,
    /// Available tools at time of last update
    pub available_tools: Vec<String>,
    /// Policy for each tool
    pub policies: HashMap<String, ToolPolicy>,
}

impl Default for ToolPolicyConfig {
    fn default() -> Self {
        Self {
            version: 1,
            available_tools: Vec::new(),
            policies: HashMap::new(),
        }
    }
}

/// Alternative tool policy configuration format (user's format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeToolPolicyConfig {
    /// Configuration version for future compatibility
    pub version: u32,
    /// Default policy settings
    pub default: AlternativeDefaultPolicy,
    /// Tool-specific policies
    pub tools: HashMap<String, AlternativeToolPolicy>,
}

/// Default policy in alternative format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeDefaultPolicy {
    /// Whether to allow by default
    pub allow: bool,
    /// Rate limit per run
    pub rate_limit_per_run: u32,
    /// Max concurrent executions
    pub max_concurrent: u32,
    /// Allow filesystem writes
    pub fs_write: bool,
    /// Allow network access
    pub network: bool,
}

/// Tool policy in alternative format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeToolPolicy {
    /// Whether to allow this tool
    pub allow: bool,
    /// Allow filesystem writes (optional)
    #[serde(default)]
    pub fs_write: bool,
    /// Allow network access (optional)
    #[serde(default)]
    pub network: bool,
    /// Arguments policy (optional)
    #[serde(default)]
    pub args_policy: Option<AlternativeArgsPolicy>,
}

/// Arguments policy in alternative format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeArgsPolicy {
    /// Substrings to deny
    pub deny_substrings: Vec<String>,
}

/// Tool policy manager
#[derive(Clone)]
pub struct ToolPolicyManager {
    config_path: PathBuf,
    config: ToolPolicyConfig,
}

impl ToolPolicyManager {
    /// Create a new tool policy manager
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_or_create_config(&config_path)?;

        Ok(Self {
            config_path,
            config,
        })
    }

    /// Create a new tool policy manager with workspace-specific config
    pub fn new_with_workspace(workspace_root: &PathBuf) -> Result<Self> {
        let config_path = Self::get_workspace_config_path(workspace_root)?;
        let config = Self::load_or_create_config(&config_path)?;

        Ok(Self {
            config_path,
            config,
        })
    }

    /// Get the path to the tool policy configuration file
    fn get_config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;

        let vtagent_dir = home_dir.join(".vtagent");
        if !vtagent_dir.exists() {
            fs::create_dir_all(&vtagent_dir).context("Failed to create ~/.vtagent directory")?;
        }

        Ok(vtagent_dir.join("tool-policy.json"))
    }

    /// Get the path to the workspace-specific tool policy configuration file
    fn get_workspace_config_path(workspace_root: &PathBuf) -> Result<PathBuf> {
        let workspace_vtagent_dir = workspace_root.join(".vtagent");

        // Check if workspace has a .vtagent/tool-policy.json file
        let workspace_policy_path = workspace_vtagent_dir.join("tool-policy.json");
        if workspace_policy_path.exists() {
            return Ok(workspace_policy_path);
        }

        // Fall back to home directory
        Self::get_config_path()
    }

    /// Load existing config or create new one with all tools as "prompt"
    fn load_or_create_config(config_path: &PathBuf) -> Result<ToolPolicyConfig> {
        if config_path.exists() {
            let content =
                fs::read_to_string(config_path).context("Failed to read tool policy config")?;

            // Try to parse as alternative format first
            if let Ok(alt_config) = serde_json::from_str::<AlternativeToolPolicyConfig>(&content) {
                // Convert alternative format to standard format
                return Ok(Self::convert_from_alternative(alt_config));
            }

            // Fall back to standard format
            serde_json::from_str(&content).context("Failed to parse tool policy config")
        } else {
            // Create new config with empty tools list
            let config = ToolPolicyConfig::default();
            Ok(config)
        }
    }

    /// Convert alternative format to standard format
    fn convert_from_alternative(alt_config: AlternativeToolPolicyConfig) -> ToolPolicyConfig {
        let mut policies = HashMap::new();

        // Convert tool policies
        for (tool_name, alt_policy) in alt_config.tools {
            let policy = if alt_policy.allow {
                ToolPolicy::Allow
            } else {
                ToolPolicy::Deny
            };
            policies.insert(tool_name, policy);
        }

        ToolPolicyConfig {
            version: alt_config.version,
            available_tools: policies.keys().cloned().collect(),
            policies,
        }
    }

    /// Update the tool list and save configuration
    pub fn update_available_tools(&mut self, tools: Vec<String>) -> Result<()> {
        let current_tools: std::collections::HashSet<_> =
            self.config.available_tools.iter().collect();
        let new_tools: std::collections::HashSet<_> = tools.iter().collect();

        // Add new tools as "prompt"
        for tool in &tools {
            if !current_tools.contains(tool) {
                self.config
                    .policies
                    .insert(tool.clone(), ToolPolicy::Prompt);
            }
        }

        // Remove deleted tools
        let tools_to_remove: Vec<_> = self
            .config
            .policies
            .keys()
            .filter(|tool| !new_tools.contains(tool))
            .cloned()
            .collect();

        for tool in tools_to_remove {
            self.config.policies.remove(&tool);
        }

        // Update available tools list
        self.config.available_tools = tools;

        self.save_config()
    }

    /// Get policy for a specific tool
    pub fn get_policy(&self, tool_name: &str) -> ToolPolicy {
        self.config
            .policies
            .get(tool_name)
            .cloned()
            .unwrap_or(ToolPolicy::Prompt)
    }

    /// Check if tool should be executed based on policy
    pub fn should_execute_tool(&mut self, tool_name: &str) -> Result<bool> {
        match self.get_policy(tool_name) {
            ToolPolicy::Allow => Ok(true),
            ToolPolicy::Deny => Ok(false),
            ToolPolicy::Prompt => {
                let should_execute = self.prompt_user_for_tool(tool_name)?;

                // Update policy based on user choice
                let new_policy = if should_execute {
                    ToolPolicy::Allow
                } else {
                    ToolPolicy::Deny
                };

                self.set_policy(tool_name, new_policy)?;
                Ok(should_execute)
            }
        }
    }

    /// Prompt user for tool execution permission
    fn prompt_user_for_tool(&mut self, tool_name: &str) -> Result<bool> {
        println!(
            "{}",
            style(format!("Tool Permission Request: {}", tool_name))
                .yellow()
                .bold()
        );
        println!("The agent wants to use the '{}' tool.", tool_name);
        println!();
        println!("Your choice will be remembered for future runs.");
        println!("You can change this later via configuration or CLI flags.");
        println!();

        // Try to prompt user, but handle non-interactive environments
        match Confirm::new()
            .with_prompt(format!("Allow the agent to use '{}'?", tool_name))
            .default(false)
            .interact()
        {
            Ok(confirmed) => {
                if confirmed {
                    println!(
                        "{}",
                        style(format!(
                            "✓ Approved: '{}' tool will be allowed in future runs",
                            tool_name
                        ))
                        .green()
                    );
                } else {
                    println!(
                        "{}",
                        style(format!(
                            "✗ Denied: '{}' tool will be blocked in future runs",
                            tool_name
                        ))
                        .red()
                    );
                }
                Ok(confirmed)
            }
            Err(e) => {
                // Handle non-interactive environments
                println!(
                    "{}",
                    style(format!("Non-interactive environment detected: {}", e)).yellow()
                );
                println!(
                    "Since we can't prompt you interactively, '{}' tool will be denied for security.",
                    tool_name
                );
                println!("You can change this later via configuration.");
                println!();

                // In non-interactive mode, deny by default for security
                Ok(false)
            }
        }
    }

    /// Set policy for a specific tool
    pub fn set_policy(&mut self, tool_name: &str, policy: ToolPolicy) -> Result<()> {
        self.config.policies.insert(tool_name.to_string(), policy);
        self.save_config()
    }

    /// Reset all tools to prompt
    pub fn reset_all_to_prompt(&mut self) -> Result<()> {
        for policy in self.config.policies.values_mut() {
            *policy = ToolPolicy::Prompt;
        }
        self.save_config()
    }

    /// Allow all tools
    pub fn allow_all_tools(&mut self) -> Result<()> {
        for policy in self.config.policies.values_mut() {
            *policy = ToolPolicy::Allow;
        }
        self.save_config()
    }

    /// Deny all tools
    pub fn deny_all_tools(&mut self) -> Result<()> {
        for policy in self.config.policies.values_mut() {
            *policy = ToolPolicy::Deny;
        }
        self.save_config()
    }

    /// Get summary of current policies
    pub fn get_policy_summary(&self) -> HashMap<String, ToolPolicy> {
        self.config.policies.clone()
    }

    /// Save configuration to file
    fn save_config(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.config)
            .context("Failed to serialize tool policy config")?;

        fs::write(&self.config_path, content).context("Failed to write tool policy config")?;

        Ok(())
    }

    /// Print current policy status
    pub fn print_status(&self) {
        println!("{}", style("Tool Policy Status").cyan().bold());
        println!("Config file: {}", self.config_path.display());
        println!();

        if self.config.policies.is_empty() {
            println!("No tools configured yet.");
            return;
        }

        let mut allow_count = 0;
        let mut prompt_count = 0;
        let mut deny_count = 0;

        for (tool, policy) in &self.config.policies {
            let (status, color_name) = match policy {
                ToolPolicy::Allow => {
                    allow_count += 1;
                    ("ALLOW", "green")
                }
                ToolPolicy::Prompt => {
                    prompt_count += 1;
                    ("PROMPT", "yellow")
                }
                ToolPolicy::Deny => {
                    deny_count += 1;
                    ("DENY", "red")
                }
            };

            let status_styled = match color_name {
                "green" => style(status).green(),
                "yellow" => style(status).yellow(),
                "red" => style(status).red(),
                _ => style(status),
            };

            println!(
                "  {} {}",
                style(format!("{:15}", tool)).cyan(),
                status_styled
            );
        }

        println!();
        println!(
            "Summary: {} allowed, {} prompt, {} denied",
            style(allow_count).green(),
            style(prompt_count).yellow(),
            style(deny_count).red()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_tool_policy_config_serialization() {
        let mut config = ToolPolicyConfig::default();
        config.available_tools = vec![tools::READ_FILE.to_string(), tools::WRITE_FILE.to_string()];
        config
            .policies
            .insert(tools::READ_FILE.to_string(), ToolPolicy::Allow);
        config
            .policies
            .insert(tools::WRITE_FILE.to_string(), ToolPolicy::Prompt);

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: ToolPolicyConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.available_tools, deserialized.available_tools);
        assert_eq!(config.policies, deserialized.policies);
    }

    #[test]
    fn test_policy_updates() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("tool-policy.json");

        let mut config = ToolPolicyConfig::default();
        config.available_tools = vec!["tool1".to_string()];
        config
            .policies
            .insert("tool1".to_string(), ToolPolicy::Prompt);

        // Save initial config
        let content = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, content).unwrap();

        // Load and update
        let mut loaded_config = ToolPolicyManager::load_or_create_config(&config_path).unwrap();

        // Add new tool
        let new_tools = vec!["tool1".to_string(), "tool2".to_string()];
        let current_tools: std::collections::HashSet<_> =
            loaded_config.available_tools.iter().collect();

        for tool in &new_tools {
            if !current_tools.contains(tool) {
                loaded_config
                    .policies
                    .insert(tool.clone(), ToolPolicy::Prompt);
            }
        }

        loaded_config.available_tools = new_tools;

        assert_eq!(loaded_config.policies.len(), 2);
        assert_eq!(
            loaded_config.policies.get("tool2"),
            Some(&ToolPolicy::Prompt)
        );
    }
}

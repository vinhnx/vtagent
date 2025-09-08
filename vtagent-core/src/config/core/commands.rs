use serde::{Deserialize, Serialize};

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

impl Default for CommandsConfig {
    fn default() -> Self {
        Self {
            allow_list: vec![
                "ls".to_string(),
                "pwd".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "git status".to_string(),
                "cargo check".to_string(),
            ],
            deny_list: vec![
                "rm -rf".to_string(),
                "sudo rm".to_string(),
                "shutdown".to_string(),
                "format".to_string(),
            ],
            dangerous_patterns: vec![
                "rm -f".to_string(),
                "git reset --hard".to_string(),
                "pip install".to_string(),
            ],
        }
    }
}

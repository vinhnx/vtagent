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

    /// Glob patterns allowed for shell commands (applies to run_terminal_cmd/Bash)
    #[serde(default)]
    pub allow_glob: Vec<String>,

    /// Glob patterns denied for shell commands
    #[serde(default)]
    pub deny_glob: Vec<String>,

    /// Regex allow patterns for shell commands
    #[serde(default)]
    pub allow_regex: Vec<String>,

    /// Regex deny patterns for shell commands
    #[serde(default)]
    pub deny_regex: Vec<String>,
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
            allow_glob: vec![],
            deny_glob: vec!["rm *".to_string(), "sudo *".to_string()],
            allow_regex: vec![],
            deny_regex: vec![r"rm\s+-rf\b".to_string()],
        }
    }
}

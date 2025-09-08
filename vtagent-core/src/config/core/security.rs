use serde::{Deserialize, Serialize};

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

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            human_in_the_loop: default_true(),
            confirm_destructive_actions: default_true(),
            log_all_commands: default_true(),
            max_file_size_mb: default_max_file_size_mb(),
            allowed_file_extensions: vec![
                ".rs".to_string(),
                ".toml".to_string(),
                ".md".to_string(),
                ".txt".to_string(),
                ".json".to_string(),
                ".yaml".to_string(),
                ".yml".to_string(),
            ],
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_max_file_size_mb() -> u64 {
    50
}

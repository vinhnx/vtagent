pub mod rustfmt;
pub mod prettier;
pub mod black;

use crate::code_quality::config::FormatConfig;
use std::path::Path;

/// Result of formatting operation
#[derive(Debug, Clone)]
pub struct FormatResult {
    pub success: bool,
    pub formatted_content: Option<String>,
    pub error_message: Option<String>,
    pub tool_used: String,
}

/// Formatting orchestrator that manages multiple formatters
pub struct FormattingOrchestrator {
    configs: Vec<FormatConfig>,
}

impl FormattingOrchestrator {
    pub fn new() -> Self {
        let mut orchestrator = Self {
            configs: Vec::new(),
        };
        
        // Register default formatters
        orchestrator.register(FormatConfig::rustfmt());
        orchestrator.register(FormatConfig::prettier());
        orchestrator.register(FormatConfig::black());
        
        orchestrator
    }

    /// Register a formatting configuration
    pub fn register(&mut self, config: FormatConfig) {
        self.configs.push(config);
    }

    /// Format a file based on its extension
    pub async fn format_file(&self, file_path: &Path) -> Option<FormatResult> {
        let extension = file_path.extension()?.to_str()?;
        let extension_with_dot = format!(".{}", extension);

        for config in &self.configs {
            if config.enabled && config.file_extensions.contains(&extension_with_dot) {
                return Some(self.run_formatter(config, file_path).await);
            }
        }

        None
    }

    async fn run_formatter(&self, config: &FormatConfig, file_path: &Path) -> FormatResult {
        // Simplified implementation - would use actual tool execution
        FormatResult {
            success: true,
            formatted_content: None,
            error_message: None,
            tool_used: config.tool_name.clone(),
        }
    }
}

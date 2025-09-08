use crate::tools::tree_sitter::LanguageSupport;

/// Code formatting tool configuration
#[derive(Debug, Clone)]
pub struct FormatConfig {
    pub language: LanguageSupport,
    pub tool_name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
    pub enabled: bool,
}

impl FormatConfig {
    /// Create rustfmt configuration
    pub fn rustfmt() -> Self {
        Self {
            language: LanguageSupport::Rust,
            tool_name: "rustfmt".to_string(),
            command: vec!["rustfmt".to_string()],
            args: vec!["--edition".to_string(), "2021".to_string()],
            file_extensions: vec![".rs".to_string()],
            enabled: true,
        }
    }

    /// Create prettier configuration
    pub fn prettier() -> Self {
        Self {
            language: LanguageSupport::TypeScript,
            tool_name: "prettier".to_string(),
            command: vec!["prettier".to_string()],
            args: vec!["--write".to_string()],
            file_extensions: vec![".ts".to_string(), ".js".to_string(), ".json".to_string()],
            enabled: true,
        }
    }

    /// Create black configuration
    pub fn black() -> Self {
        Self {
            language: LanguageSupport::Python,
            tool_name: "black".to_string(),
            command: vec!["black".to_string()],
            args: vec![],
            file_extensions: vec![".py".to_string()],
            enabled: true,
        }
    }
}

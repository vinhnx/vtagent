use crate::tree_sitter::LanguageSupport;
use std::collections::HashMap;

/// Linting tool configuration
#[derive(Debug, Clone)]
pub struct LintConfig {
    pub language: LanguageSupport,
    pub tool_name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub severity_levels: HashMap<String, LintSeverity>,
    pub enabled: bool,
}

/// Lint result severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl LintConfig {
    /// Create clippy configuration
    pub fn clippy() -> Self {
        Self {
            language: LanguageSupport::Rust,
            tool_name: "clippy".to_string(),
            command: vec!["cargo".to_string(), "clippy".to_string()],
            args: vec!["--".to_string(), "-D".to_string(), "warnings".to_string()],
            severity_levels: HashMap::new(),
            enabled: true,
        }
    }

    /// Create ESLint configuration
    pub fn eslint() -> Self {
        Self {
            language: LanguageSupport::TypeScript,
            tool_name: "eslint".to_string(),
            command: vec!["eslint".to_string()],
            args: vec!["--format".to_string(), "json".to_string()],
            severity_levels: HashMap::new(),
            enabled: true,
        }
    }

    /// Create pylint configuration
    pub fn pylint() -> Self {
        Self {
            language: LanguageSupport::Python,
            tool_name: "pylint".to_string(),
            command: vec!["pylint".to_string()],
            args: vec!["--output-format".to_string(), "json".to_string()],
            severity_levels: HashMap::new(),
            enabled: true,
        }
    }
}

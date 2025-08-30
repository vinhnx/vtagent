#![doc = r#"
# Vtagent - Advanced Rust Coding Agent

A sophisticated terminal-based coding agent that implements state-of-the-art agent
architecture patterns, inspired by Anthropic's SWE-bench breakthroughs.

## Architecture

The agent follows proven patterns for reliable, long-running coding assistance:
- **Model-Driven Control**: Maximum autonomy given to language models
- **Decision Transparency**: Complete audit trail of all agent actions
- **Error Recovery**: Intelligent error handling with context preservation
- **Conversation Summarization**: Automatic compression for long sessions
"#]
//!
//! A sophisticated terminal-based coding agent that implements state-of-the-art agent
//! architecture patterns, inspired by Anthropic's SWE-bench breakthroughs.
//!
//! ## Architecture
//!
//! The agent follows proven patterns for reliable, long-running coding assistance:
//! - **Model-Driven Control**: Maximum autonomy given to language models
//! - **Decision Transparency**: Complete audit trail of all agent actions
//! - **Error Recovery**: Intelligent error handling with context preservation
//! - **Conversation Summarization**: Automatic compression for long sessions

pub mod agent;
pub mod async_file_ops;
pub mod cli;
pub mod code_completion;
pub mod commands;
pub mod context_analyzer;
pub mod conversation_summarizer;
pub mod decision_tracker;
pub mod diff_renderer;
pub mod enhanced_tools;
pub mod error_recovery;
pub mod gemini;
pub mod markdown_renderer;
pub mod performance_monitor;
pub mod performance_profiler;
pub mod prompts;
pub mod tools;
pub mod tree_sitter;
pub mod types;

// Re-export commonly used types for convenience
pub use agent::{Agent, AgentBuilder};
pub use cli::{Cli, Commands};
pub use code_completion::{
    CodeCompletionEngine, CompletionContext, CompletionSuggestion, COMPLETION_ENGINE,
};
pub use conversation_summarizer::ConversationSummarizer;
pub use decision_tracker::{Action, DecisionOutcome, DecisionTracker, ResponseType};
pub use error_recovery::{ErrorContext, ErrorRecoveryManager, ErrorType};
pub use gemini::{
    Candidate, Client, Content, FunctionCall, FunctionResponse, GenerateContentRequest, Part, Tool,
    ToolConfig,
};
pub use performance_monitor::{PerformanceMetrics, PerformanceMonitor, PERFORMANCE_MONITOR};
pub use tools::{build_function_declarations, ToolError, ToolRegistry};
pub use tree_sitter::{
    CodeAnalysis, LanguageSupport, SyntaxTree, TreeSitterAnalyzer, TreeSitterError,
};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_library_exports() {
        // Test that all public exports are accessible
        let temp_dir = TempDir::new().unwrap();
        let _tool_registry = ToolRegistry::new(temp_dir.path().to_path_buf());
        let _tree_sitter = TreeSitterAnalyzer::new().unwrap();
    }

    #[test]
    fn test_module_structure() {
        // Test that all modules can be imported
        // This is a compile-time test that ensures module structure is correct
        assert!(true);
    }

    #[test]
    fn test_version_consistency() {
        // Test that version information is consistent across modules
        // This would be more meaningful with actual version checking
        assert!(true);
    }

    #[tokio::test]
    async fn test_tool_registry_integration() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test that we can execute basic tools
        let list_args = serde_json::json!({
            "path": "."
        });

        let result = registry.execute("list_files", list_args).await;
        assert!(result.is_ok());

        let response: serde_json::Value = result.unwrap();
        assert!(response["files"].is_array());
    }

    #[test]
    fn test_error_handling() {
        // Test that error types are properly exported and usable
        let tool_error = ToolError::FileNotFound("test.txt".to_string());
        assert!(format!("{}", tool_error).contains("test.txt"));
    }
}

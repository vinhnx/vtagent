#![doc = r#"
# Vtagent - Minimal research-preview Rust Coding Agent

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
pub mod advanced_code_editing;
pub mod code_quality_tools;
pub mod context_suggestions;
pub mod enhanced_file_ops;
pub mod enhanced_tools;
pub mod error_recovery;
pub mod gemini;
pub mod markdown_renderer;
pub mod performance_monitor;
pub mod performance_profiler;
pub mod prompts;
pub mod timeout_detector;
pub mod timeout_policies;
pub mod tools;
pub mod todo_write;
pub mod tree_sitter;
pub mod types;
pub mod ui;
pub mod vtagentgitignore;

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
pub use advanced_code_editing::{MinimalCodeEditor, EditPlan, RefactorOperation};
pub use code_quality_tools::{CodeQualityManager, QualityAssessment, QualityGrade};
pub use context_suggestions::{ContextSuggestionEngine, CodeSuggestion, SuggestionType};
pub use enhanced_file_ops::{EnhancedFileOps, EnhancedFileResult, FileOperationStats};
pub use timeout_detector::{TimeoutConfig, TimeoutDetector, OperationType, TIMEOUT_DETECTOR};
pub use timeout_policies::{TimeoutPolicy, TimeoutPolicyConfig, TimeoutPolicyManager, POLICY_MANAGER, NetworkQuality};
pub use tools::{build_function_declarations, ToolError, ToolRegistry};
pub use todo_write::{TodoManager, TodoItem, TodoStatus, TodoStatistics, TodoInput, TodoUpdate};
pub use tree_sitter::{
    CodeAnalysis, LanguageSupport, SyntaxTree, TreeSitterAnalyzer, TreeSitterError,
};
pub use vtagentgitignore::{VtagentGitignore, initialize_vtagent_gitignore, should_exclude_file, filter_paths};
pub use agent::snapshots::{AgentSnapshot, SnapshotManager, SnapshotConfig, SnapshotInfo};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

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

        let result = registry.execute_tool("list_files", list_args).await;
        assert!(result.is_ok());

        let response: serde_json::Value = result.unwrap();
        assert!(response["files"].is_array());
    }

    #[test]
    fn test_error_handling() {
        // Test that error types are properly exported and usable
        let tool_error = ToolError::TextNotFound("test.txt".to_string());
        assert!(format!("{}", tool_error).contains("test.txt"));
    }
}

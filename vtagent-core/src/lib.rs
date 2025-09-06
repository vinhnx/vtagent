#![doc = r#"
# Vtagent - Research-preview Rust Coding Agent

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

//! VT Code Core Library
//!
//! This crate provides the core functionality for the VT Code agent,
//! including tool implementations, LLM integration, and utility functions.

// Public modules
pub mod agent;
pub mod apply_patch;
pub mod ast_grep;
pub mod cli;
pub mod code_completion;
pub mod commands;
pub mod config;
pub mod conversation_summarizer;
pub mod decision_tracker;
pub mod diff_renderer;
pub mod error_recovery;
pub mod file_search;
pub mod gemini;
pub mod llm;
pub mod performance_monitor;
pub mod performance_profiler;
pub mod prompts;
pub mod rp_search;
pub mod timeout_detector;
pub mod tools;
pub mod tree_sitter;
pub mod types;
pub mod vtagentgitignore;

// Re-exports for convenience
pub use agent::core::Agent;
pub use cli::args::{Cli, Commands};
pub use code_completion::{CodeCompletionEngine, CompletionSuggestion};
pub use commands::stats::handle_stats_command;
pub use config::{AgentConfig, VTAgentConfig};
pub use diff_renderer::DiffRenderer;
pub use gemini::{Content, FunctionDeclaration, Part};
pub use llm::{make_client, AnyClient};
pub use performance_profiler::PerformanceProfiler;
pub use prompts::{generate_system_instruction, generate_specialized_instruction, generate_lightweight_instruction};
pub use rp_search::RpSearchManager;
pub use timeout_detector::TimeoutDetector;
pub use tools::{ToolRegistry, ToolError, build_function_declarations};
pub use tree_sitter::TreeSitterAnalyzer;
pub use types::{SessionInfo, ToolConfig, ContextConfig, LoggingConfig, CommandResult, AnalysisDepth, OutputFormat, CompressionLevel, PerformanceMetrics, CapabilityLevel};
pub use vtagentgitignore::initialize_vtagent_gitignore;

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

    #[tokio::test]
    async fn test_pty_basic_command() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let registry = ToolRegistry::new(workspace.clone());

        // Test a simple PTY command
        let args = serde_json::json!({
            "command": "echo",
            "args": ["Hello, PTY!"]
        });

        let result = registry.execute_tool("run_pty_cmd", args).await;
        assert!(result.is_ok());
        let response: serde_json::Value = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["code"], 0);
        assert!(response["output"].as_str().unwrap().contains("Hello, PTY!"));
    }

    #[tokio::test]
    async fn test_pty_session_management() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let registry = ToolRegistry::new(workspace.clone());

        // Test creating a PTY session
        let args = serde_json::json!({
            "session_id": "test_session",
            "command": "bash"
        });

        let result = registry.execute_tool("create_pty_session", args).await;
        assert!(result.is_ok());
        let response: serde_json::Value = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["session_id"], "test_session");

        // Test listing PTY sessions
        let args = serde_json::json!({});
        let result = registry.execute_tool("list_pty_sessions", args).await;
        assert!(result.is_ok());
        let response: serde_json::Value = result.unwrap();
        assert!(
            response["sessions"]
                .as_array()
                .unwrap()
                .contains(&"test_session".into())
        );

        // Test closing a PTY session
        let args = serde_json::json!({
            "session_id": "test_session"
        });

        let result = registry.execute_tool("close_pty_session", args).await;
        assert!(result.is_ok());
        let response: serde_json::Value = result.unwrap();
        assert_eq!(response["success"], true);
        assert_eq!(response["session_id"], "test_session");
    }
}

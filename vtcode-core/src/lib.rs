#![doc = r#"
# VTCode - Research-preview Rust Coding Agent

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

//! VTCode Core Library
//!
//! This crate provides the core functionality for the VTCode agent,
//! including tool implementations, LLM integration, and utility functions.

// Public modules
pub mod bash_runner;
pub mod cli;
pub mod code;
pub mod commands;
pub mod config;
pub mod constants;
pub mod core;
pub mod gemini;
pub mod llm;
pub mod markdown_storage;
pub mod models;
pub mod project;
pub mod prompts;
pub mod safety;
pub mod simple_indexer;
pub mod tool_policy;
pub mod tools;
pub mod types;
pub mod ui;
pub mod utils;

// Re-exports for convenience
pub use bash_runner::BashRunner;
pub use cli::args::{Cli, Commands};
pub use code::code_completion::{CompletionEngine, CompletionSuggestion};
pub use commands::stats::handle_stats_command;
pub use config::types::{
    AnalysisDepth, CapabilityLevel, CommandResult, CompressionLevel, ContextConfig, LoggingConfig,
    OutputFormat, PerformanceMetrics, SessionInfo, ToolConfig,
};
pub use config::{AgentConfig, VTCodeConfig};
pub use core::agent::core::Agent;
pub use core::context_compression::{
    CompressedContext, ContextCompressionConfig, ContextCompressor,
};
pub use core::conversation_summarizer::ConversationSummarizer;
pub use core::performance_profiler::PerformanceProfiler;
pub use core::prompt_caching::{CacheStats, PromptCache, PromptCacheConfig, PromptOptimizer};
pub use core::timeout_detector::TimeoutDetector;
pub use gemini::{Content, FunctionDeclaration, Part};
pub use llm::{AnyClient, make_client};
pub use markdown_storage::{MarkdownStorage, ProjectData, ProjectStorage, SimpleKVStorage};
pub use project::{SimpleCache, SimpleProjectManager};
pub use prompts::{
    generate_lightweight_instruction, generate_specialized_instruction, generate_system_instruction,
};
pub use simple_indexer::SimpleIndexer;
pub use tool_policy::{ToolPolicy, ToolPolicyManager};
pub use tools::advanced_search::{AdvancedSearchTool, SearchOptions};
pub use tools::grep_search::GrepSearchManager;
pub use tools::tree_sitter::TreeSitterAnalyzer;
pub use tools::{
    ToolRegistration, ToolRegistry, build_function_declarations,
    build_function_declarations_for_level,
};
pub use ui::diff_renderer::DiffRenderer;
pub use utils::dot_config::{
    CacheConfig, DotConfig, DotManager, ProviderConfigs, UiConfig, UserPreferences,
    initialize_dot_folder, load_user_config, save_user_config, update_theme_preference,
};
pub use utils::vtcodegitignore::initialize_vtcode_gitignore;

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_library_exports() {
        // Test that all public exports are accessible
        let _cache = PromptCache::new();
    }

    #[test]
    fn test_module_structure() {
        // Test that all modules can be imported
        // This is a compile-time test that ensures module structure is correct
    }

    #[test]
    fn test_version_consistency() {
        // Test that version information is consistent across modules
        // This would be more meaningful with actual version checking
    }

    #[tokio::test]
    async fn test_tool_registry_integration() {
        use crate::config::constants::tools;

        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test that we can execute basic tools
        let list_args = serde_json::json!({
            "path": "."
        });

        let result = registry.execute_tool(tools::LIST_FILES, list_args).await;
        assert!(result.is_ok());

        let response: serde_json::Value = result.unwrap();
        assert!(response["files"].is_array());
    }

    #[tokio::test]
    async fn test_pty_basic_command() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let mut registry = ToolRegistry::new(workspace.clone());

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
        let mut registry = ToolRegistry::new(workspace.clone());

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

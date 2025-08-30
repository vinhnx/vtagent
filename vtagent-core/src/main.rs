//! VTAgent Core Library
//!
//! This is the core library for vtagent, containing all the main functionality.
//! The actual binary entry point is in the main crate (src/main.rs).
//!
//! This file serves as the main module file for the vtagent-core library and
//! should not contain any main functions or binary-specific code.

// Re-export all public modules
pub mod agent;
pub mod async_file_ops;
pub mod cli;
pub mod commands;
pub mod context_analyzer;
pub mod conversation_summarizer;
pub mod decision_tracker;
pub mod diff_renderer;
pub mod advanced_code_editing;
pub mod code_completion;
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
pub use cli::{Cli, Commands, RateLimiter};
pub use commands::{analyze_workspace, handle_ask_command, handle_stats_command, handle_revert_command};
pub use gemini::{
    Candidate, Client, Content, FunctionCall, FunctionResponse,
    GenerateContentRequest, Part, Tool, ToolConfig,
};
pub use tools::{build_function_declarations, ToolError, ToolRegistry};
pub use types::*;

// Note: This library should not contain main functions.
// The main binary is in src/main.rs of the main crate.

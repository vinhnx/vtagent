//! # Tool System Architecture
//!
//! This module provides a modular, composable architecture for VTCode agent tools,
//! implementing a registry-based system for tool discovery, execution, and management.
//!
//! ## Architecture Overview
//!
//! The tool system is designed around several key principles:
//!
//! - **Modularity**: Each tool is a focused, reusable component
//! - **Registry Pattern**: Centralized tool registration and discovery
//! - **Policy-Based Execution**: Configurable execution policies and safety checks
//! - **Type Safety**: Strong typing for tool parameters and results
//! - **Async Support**: Full async/await support for all tool operations
//!
//! ## Core Components
//!
//! ### Tool Registry
//! ```rust,no_run
//! use vtcode_core::tools::{ToolRegistry, ToolRegistration};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let workspace = std::env::current_dir()?;
//!     let mut registry = ToolRegistry::new(workspace);
//!
//!     // Register a custom tool
//!     let tool = ToolRegistration {
//!         name: "my_tool".to_string(),
//!         description: "A custom tool".to_string(),
//!         parameters: serde_json::json!({"type": "object"}),
//!         handler: |args| async move {
//!             Ok(serde_json::json!({"result": "success"}))
//!         },
//!     };
//!
//!     registry.register_tool(tool).await?;
//!     Ok(())
//! }
//! ```
//!
//! ### Tool Categories
//!
//! #### File Operations
//! - **File Operations**: Read, write, create, delete files
//! - **Search Tools**: Grep, AST-based search, advanced search
//! - **Cache Management**: File caching and performance optimization
//!
//! #### Terminal Integration
//! - **Bash Tools**: Shell command execution
//! - **PTY Support**: Full terminal emulation
//! - **Command Policies**: Safety and execution controls
//!
//! #### Code Analysis
//! - **Tree-Sitter**: Syntax-aware code analysis
//! - **AST Grep**: Structural code search and transformation
//! - **Srgn**: Syntax-aware code modification
//!
//! ## Tool Execution
//!
//! ```rust,no_run
//! use vtcode_core::tools::ToolRegistry;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut registry = ToolRegistry::new(std::env::current_dir()?);
//!
//!     // Execute a tool
//!     let args = serde_json::json!({"path": "."});
//!     let result = registry.execute_tool("list_files", args).await?;
//!
//!     println!("Result: {}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Safety & Policies
//!
//! The tool system includes comprehensive safety features:
//!
//! - **Path Validation**: All file operations check workspace boundaries
//! - **Command Policies**: Configurable allow/deny lists for terminal commands
//! - **Execution Limits**: Timeout and resource usage controls
//! - **Audit Logging**: Complete trail of tool executions
//!
//! ## Custom Tool Development
//!
//! ```rust,no_run
//! use vtcode_core::tools::traits::Tool;
//! use serde_json::Value;
//!
//! struct MyCustomTool;
//!
//! #[async_trait::async_trait]
//! impl Tool for MyCustomTool {
//!     async fn execute(&self, args: Value) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
//!         // Tool implementation
//!         Ok(serde_json::json!({"status": "completed"}))
//!     }
//!
//!     fn name(&self) -> &str {
//!         "my_custom_tool"
//!     }
//!
//!     fn description(&self) -> &str {
//!         "A custom tool for specific tasks"
//!     }
//!
//!     fn parameters(&self) -> Value {
//!         serde_json::json!({
//!             "type": "object",
//!             "properties": {
//!                 "input": {"type": "string"}
//!             }
//!         })
//!     }
//! }
//! ```
//!
//! Modular tool system for VTCode
//!
//! This module provides a composable architecture for agent tools, breaking down
//! the monolithic implementation into focused, reusable components.

pub mod advanced_search;
pub mod apply_patch;
pub mod ast_grep;
pub mod ast_grep_tool;
pub mod bash_tool;
pub mod cache;
pub mod command;
pub mod file_ops;
pub mod file_search;
pub mod grep_search;
pub mod registry;
pub mod search;
pub mod simple_search;
pub mod srgn;
pub mod traits;
pub mod tree_sitter;
pub mod types;

// Re-export main types and traits for backward compatibility
pub use ast_grep_tool::AstGrepTool;
pub use bash_tool::BashTool;
pub use cache::FileCache;
pub use grep_search::GrepSearchManager;
pub use registry::{ToolRegistration, ToolRegistry};
pub use simple_search::SimpleSearchTool;
pub use srgn::SrgnTool;
pub use traits::{Tool, ToolExecutor};
pub use types::*;

// Re-export function declarations for external use
pub use registry::build_function_declarations;
pub use registry::build_function_declarations_for_level;

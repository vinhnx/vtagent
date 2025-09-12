//! Modular tool system for VTAgent
//!
//! This module provides a composable architecture for agent tools, breaking down
//! the monolithic implementation into focused, reusable components.

pub mod advanced_search;
pub mod apply_patch;
pub mod ast_grep;
pub mod ast_grep_tool;
pub mod bash_tool;
pub mod cache;
pub mod ck_tool;
pub mod command;
pub mod file_ops;
pub mod file_search;
pub mod registry;
pub mod rg_search;
pub mod search;
pub mod simple_search;
pub mod traits;
pub mod tree_sitter;
pub mod types;

// Re-export main types and traits for backward compatibility
pub use ast_grep_tool::AstGrepTool;
pub use bash_tool::BashTool;
pub use cache::FileCache;
pub use ck_tool::CkTool;
pub use registry::ToolRegistry;
pub use rg_search::RgSearchManager;
pub use simple_search::SimpleSearchTool;
pub use traits::{Tool, ToolExecutor};
pub use types::*;

// Re-export function declarations for external use
pub use registry::build_function_declarations;
pub use registry::build_function_declarations_for_level;

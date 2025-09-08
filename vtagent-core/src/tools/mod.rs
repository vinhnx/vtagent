//! Modular tool system for VTAgent
//!
//! This module provides a composable architecture for agent tools, breaking down
//! the monolithic implementation into focused, reusable components.

pub mod cache;
pub mod file_ops;
pub mod search;
pub mod command;
pub mod registry;
pub mod traits;
pub mod types;

// Re-export main types and traits for backward compatibility
pub use registry::ToolRegistry;
pub use traits::{Tool, ToolExecutor};
pub use types::*;
pub use cache::FileCache;

// Re-export function declarations for external use
pub use registry::build_function_declarations;
pub use registry::build_function_declarations_for_level;

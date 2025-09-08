//! Modular tool system for VTAgent
//!
//! This module provides a composable architecture for agent tools, breaking down
//! the monolithic implementation into focused, reusable components.

pub mod cache;
pub mod command;
pub mod file_ops;
pub mod registry;
pub mod search;
pub mod traits;
pub mod types;

// Re-export main types and traits for backward compatibility
pub use cache::FileCache;
pub use registry::ToolRegistry;
pub use traits::{Tool, ToolExecutor};
pub use types::*;

// Re-export function declarations for external use
pub use registry::build_function_declarations;
pub use registry::build_function_declarations_for_level;

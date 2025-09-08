//! Code completion engine with modular architecture
//!
//! This module provides intelligent code completion with learning capabilities,
//! context analysis, and language-specific optimizations.

pub mod engine;
pub mod context;
pub mod learning;
pub mod languages;
pub mod cache;

// Re-export main types for backward compatibility
pub use engine::{CompletionEngine, CompletionSuggestion, CompletionKind};
pub use context::{CompletionContext, ContextAnalyzer};
pub use learning::{CompletionLearningData, LearningSystem};
pub use cache::CompletionCache;

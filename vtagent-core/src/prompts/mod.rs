//! System prompt generation with modular architecture
//!
//! This module provides flexible system prompt generation with
//! template-based composition and context-aware customization.

pub mod templates;
pub mod generator;
pub mod context;
pub mod config;
pub mod system;

// Re-export main types for backward compatibility
pub use generator::{SystemPromptGenerator, generate_system_instruction_with_config};
pub use config::SystemPromptConfig;
pub use context::PromptContext;
pub use templates::PromptTemplates;
pub use system::{generate_lightweight_instruction, generate_specialized_instruction, generate_system_instruction};

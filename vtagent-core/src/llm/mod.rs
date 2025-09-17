//! LLM abstraction layer with modular architecture
//!
//! This module provides a unified interface for different LLM providers
//! with provider-specific implementations.

pub mod client;
pub mod error_display;
pub mod factory;
pub mod provider;
pub mod providers;
pub mod stream;
pub mod types;

#[cfg(test)]
mod error_display_test;

// Re-export main types for backward compatibility
pub use client::{AnyClient, make_client};
pub use factory::{create_provider_with_config, get_factory};
pub use providers::{AnthropicProvider, GeminiProvider, OpenAIProvider};
pub use types::{BackendKind, LLMError, LLMResponse};

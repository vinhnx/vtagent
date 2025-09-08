//! LLM abstraction layer with modular architecture
//!
//! This module provides a unified interface for different LLM providers
//! with provider-specific implementations.

pub mod providers;
pub mod client;
pub mod provider;
pub mod types;

// Re-export main types for backward compatibility
pub use client::{make_client, AnyClient};
pub use types::{BackendKind, LLMResponse, LLMError};
pub use providers::{GeminiProvider, OpenAIProvider, AnthropicProvider};

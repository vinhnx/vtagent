//! Gemini API client with modular architecture
//!
//! This module provides a clean separation between HTTP client configuration,
//! API models, streaming functionality, and function calling integration.

pub mod client;
pub mod function_calling;
pub mod models;
pub mod streaming;

// Re-export main types for backward compatibility
pub use client::{Client, ClientConfig, RetryConfig};
pub use function_calling::{FunctionCall, FunctionCallingConfig, FunctionResponse};
pub use models::{
    Candidate, Content, FunctionDeclaration, GenerateContentRequest, GenerateContentResponse, Part,
    Tool, ToolConfig,
};
pub use streaming::{
    StreamingCandidate, StreamingConfig, StreamingError, StreamingMetrics, StreamingProcessor,
    StreamingResponse,
};

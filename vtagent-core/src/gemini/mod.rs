//! Gemini API client with modular architecture
//! 
//! This module provides a clean separation between HTTP client configuration,
//! API models, streaming functionality, and function calling integration.

pub mod client;
pub mod models;
pub mod streaming;
pub mod function_calling;

// Re-export main types for backward compatibility
pub use client::{Client, ClientConfig, RetryConfig};
pub use models::{GenerateContentRequest, GenerateContentResponse, Content, Part, Tool, ToolConfig, FunctionDeclaration, Candidate};
pub use streaming::{StreamingResponse, StreamingCandidate, StreamingError, StreamingMetrics};
pub use function_calling::{FunctionCall, FunctionResponse, FunctionCallingConfig};

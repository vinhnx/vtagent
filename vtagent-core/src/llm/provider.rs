use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Universal LLM request structure
#[derive(Debug, Clone)]
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

/// Universal message structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Universal tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Universal tool call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Universal LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: Option<Usage>,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error(String),
}

/// Universal LLM provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider name (e.g., "gemini", "openai", "anthropic")
    fn name(&self) -> &str;

    /// Generate completion
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;

    /// Stream completion (optional)
    async fn stream(
        &self,
        request: LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = LLMResponse> + Unpin + Send>, LLMError> {
        // Default implementation falls back to non-streaming
        let response = self.generate(request).await?;
        Ok(Box::new(futures::stream::once(async { response }).boxed()))
    }

    /// Get supported models
    fn supported_models(&self) -> Vec<String>;

    /// Validate request for this provider
    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Provider error: {0}")]
    Provider(String),
}

// Implement conversion from provider::LLMError to llm::types::LLMError
impl From<LLMError> for crate::llm::types::LLMError {
    fn from(err: LLMError) -> crate::llm::types::LLMError {
        match err {
            LLMError::Authentication(msg) => crate::llm::types::LLMError::ApiError(msg),
            LLMError::RateLimit => crate::llm::types::LLMError::RateLimit,
            LLMError::InvalidRequest(msg) => crate::llm::types::LLMError::InvalidRequest(msg),
            LLMError::Network(msg) => crate::llm::types::LLMError::NetworkError(msg),
            LLMError::Provider(msg) => crate::llm::types::LLMError::ApiError(msg),
        }
    }
}

use futures::StreamExt;

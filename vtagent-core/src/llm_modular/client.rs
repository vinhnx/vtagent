use super::providers::{GeminiProvider, OpenAIProvider, AnthropicProvider};
use super::types::{BackendKind, LLMResponse, LLMError};
use crate::models::ModelId;
use async_trait::async_trait;

/// Unified LLM client trait
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn generate(&mut self, prompt: &str) -> Result<LLMResponse, LLMError>;
    fn backend_kind(&self) -> BackendKind;
    fn model_id(&self) -> &str;
}

/// Type-erased LLM client
pub type AnyClient = Box<dyn LLMClient>;

/// Create a client based on the model ID
pub fn make_client(api_key: String, model: ModelId) -> AnyClient {
    match model.provider() {
        crate::models::Provider::Gemini => {
            Box::new(GeminiProvider::new(api_key, model.to_string()))
        }
        crate::models::Provider::OpenAI => {
            Box::new(OpenAIProvider::new(api_key, model.to_string()))
        }
        crate::models::Provider::Anthropic => {
            Box::new(AnthropicProvider::new(api_key, model.to_string()))
        }
    }
}

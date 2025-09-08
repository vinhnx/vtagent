use crate::llm_modular::client::LLMClient;
use crate::llm_modular::types::{BackendKind, LLMResponse, LLMError};
use async_trait::async_trait;

/// Anthropic LLM provider
pub struct AnthropicProvider {
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMClient for AnthropicProvider {
    async fn generate(&mut self, _prompt: &str) -> Result<LLMResponse, LLMError> {
        // Anthropic implementation would go here
        Err(LLMError::ApiError("Anthropic provider not implemented".to_string()))
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::Anthropic
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

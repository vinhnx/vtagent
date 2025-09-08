use crate::llm_modular::client::LLMClient;
use crate::llm_modular::types::{BackendKind, LLMResponse, LLMError};
use async_trait::async_trait;

/// OpenAI LLM provider
pub struct OpenAIProvider {
    api_key: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMClient for OpenAIProvider {
    async fn generate(&mut self, _prompt: &str) -> Result<LLMResponse, LLMError> {
        // OpenAI implementation would go here
        Err(LLMError::ApiError("OpenAI provider not implemented".to_string()))
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::OpenAI
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

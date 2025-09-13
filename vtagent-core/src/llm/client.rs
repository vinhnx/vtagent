use super::provider::LLMError;
use super::providers::{AnthropicProvider, GeminiProvider, LMStudioProvider, OpenAIProvider};
use super::types::{BackendKind, LLMResponse};
use crate::config::models::{ModelId, Provider};
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
        Provider::Gemini => Box::new(GeminiProvider::with_model(
            api_key,
            model.as_str().to_string(),
        )),
        Provider::OpenAI => Box::new(OpenAIProvider::new(api_key)),
        Provider::Anthropic => Box::new(AnthropicProvider::new(api_key)),
        Provider::LMStudio => Box::new(LMStudioProvider::new(
            None,
            Some(crate::config::constants::urls::LMSTUDIO_DEFAULT_BASE_URL.to_string()),
        )),
    }
}

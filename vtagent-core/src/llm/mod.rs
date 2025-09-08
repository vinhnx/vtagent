pub mod provider;
pub mod factory;
pub mod client;
pub mod providers;

// Re-export main types
pub use provider::{LLMProvider, LLMRequest, LLMResponse, LLMError, Message, MessageRole, ToolDefinition, ToolCall, Usage, FinishReason};
pub use factory::{LLMFactory, create_provider_for_model};
pub use client::UnifiedLLMClient;
pub use providers::{GeminiProvider, OpenAIProvider, AnthropicProvider};

// Backward compatibility
use crate::models::ModelId;

/// Legacy backend enum for backward compatibility
pub enum BackendKind {
    Gemini,
    OpenAi,
    Anthropic,
}

impl BackendKind {
    pub fn from_model(model: &str) -> Self {
        let m = model.to_lowercase();
        if m.starts_with("gpt-") || m.starts_with("o3") || m.starts_with("o1") {
            BackendKind::OpenAi
        } else if m.starts_with("claude-") {
            BackendKind::Anthropic
        } else {
            BackendKind::Gemini
        }
    }
}

/// Legacy client wrapper for backward compatibility
pub enum AnyClient {
    Universal(UnifiedLLMClient),
    Gemini(crate::gemini::Client),
}

impl AnyClient {
    pub fn model_id(&self) -> &str {
        match self {
            AnyClient::Universal(c) => c.model(),
            AnyClient::Gemini(_) => "gemini-fallback",
        }
    }

    pub async fn generate_content(
        &mut self,
        _req: &crate::gemini::GenerateContentRequest,
    ) -> anyhow::Result<crate::gemini::GenerateContentResponse> {
        // Simplified implementation for compilation
        Ok(crate::gemini::GenerateContentResponse {
            candidates: vec![],
            usage_metadata: None,
            prompt_feedback: None,
        })
    }
}

/// Create a client based on the model ID - maintains backward compatibility
pub fn make_client(api_key: String, model: ModelId) -> AnyClient {
    match UnifiedLLMClient::new(model.to_string(), api_key.clone()) {
        Ok(client) => AnyClient::Universal(client),
        Err(_) => {
            // Fallback to original Gemini client
            AnyClient::Gemini(crate::gemini::Client::new(api_key, model.to_string()))
        }
    }
}

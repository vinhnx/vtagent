pub mod client;
pub mod factory;
pub mod provider;
pub mod providers;

// Re-export main types
pub use client::UnifiedLLMClient;
pub use factory::{LLMFactory, create_provider_for_model};
pub use provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
    ToolDefinition, Usage,
};
pub use providers::{AnthropicProvider, GeminiProvider, OpenAIProvider};

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
        req: &crate::gemini::GenerateContentRequest,
    ) -> anyhow::Result<crate::gemini::GenerateContentResponse> {
        match self {
            AnyClient::Universal(client) => {
                // Convert the Gemini request to the unified format
                let prompt = req
                    .contents
                    .iter()
                    .map(|c| {
                        c.parts
                            .iter()
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                match client.generate(&prompt).await {
                    Ok(response) => {
                        // Convert back to Gemini format for compatibility
                        Ok(crate::gemini::GenerateContentResponse {
                            candidates: vec![crate::gemini::Candidate {
                                content: crate::gemini::Content {
                                    parts: vec![crate::gemini::Part::Text(response.content)],
                                    role: crate::gemini::Role::Model,
                                },
                                finish_reason: Some(crate::gemini::FinishReason::Stop),
                                index: 0,
                                safety_ratings: vec![],
                            }],
                            usage_metadata: response.usage.map(|u| crate::gemini::UsageMetadata {
                                prompt_token_count: u.prompt_tokens,
                                candidates_token_count: u.completion_tokens,
                                total_token_count: u.total_tokens,
                                cached_content_token_count: 0,
                            }),
                            prompt_feedback: None,
                        })
                    }
                    Err(e) => Err(anyhow::anyhow!("LLM call failed: {}", e)),
                }
            }
            AnyClient::Gemini(client) => client.generate_content(req).await,
        }
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

use crate::llm_modular::client::LLMClient;
use crate::llm_modular::types::{BackendKind, LLMResponse, LLMError, Usage};
use crate::gemini::{Client, GenerateContentRequest, Content};
use async_trait::async_trait;

/// Gemini LLM provider
pub struct GeminiProvider {
    client: Client,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(api_key, model.clone()),
            model,
        }
    }
}

#[async_trait]
impl LLMClient for GeminiProvider {
    async fn generate(&mut self, prompt: &str) -> Result<LLMResponse, LLMError> {
        let request = GenerateContentRequest {
            contents: vec![Content::user_text(prompt)],
            tools: None,
            tool_config: None,
            system_instruction: None,
            generation_config: None,
        };

        match self.client.generate_content(&request).await {
            Ok(response) => {
                let content = response.candidates
                    .first()
                    .and_then(|c| c.content.parts.first())
                    .and_then(|p| p.as_text())
                    .unwrap_or("")
                    .to_string();

                Ok(LLMResponse {
                    content,
                    model: self.model.clone(),
                    usage: None, // Gemini doesn't provide detailed usage in this simplified version
                })
            }
            Err(e) => Err(LLMError::ApiError(e.to_string())),
        }
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::Gemini
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

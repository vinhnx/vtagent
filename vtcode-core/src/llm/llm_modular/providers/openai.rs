use crate::llm_modular::client::LLMClient;
use crate::llm_modular::types::{BackendKind, LLMResponse, LLMError, Usage};
use async_trait::async_trait;
use reqwest;
use serde_json::{Value, json};

/// OpenAI LLM provider
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LLMClient for OpenAIProvider {
    async fn generate(&mut self, prompt: &str) -> Result<LLMResponse, LLMError> {
        let request_body = json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "temperature": 0.7
        });

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| LLMError::ApiError(format!("Failed to send request: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::ApiError(format!("API error: {}", error_text)));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| LLMError::ApiError(format!("Failed to parse response: {}", e)))?;

        let message = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .cloned()
            .unwrap_or_else(|| json!({"content": ""}));

        let (content, reasoning_from_content) = match message.get("content") {
            Some(Value::String(text)) => {
                let trimmed = text.trim();
                let content = if trimmed.is_empty() {
                    String::new()
                } else {
                    text.to_string()
                };
                (content, None)
            }
            Some(Value::Array(parts)) => {
                let mut content_buffer = String::new();
                let mut reasoning_segments = Vec::new();

                for part in parts {
                    let text = part.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    if text.trim().is_empty() {
                        continue;
                    }

                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("reasoning") | Some("analysis") | Some("thought")
                        | Some("chain_of_thought") => {
                            reasoning_segments.push(text.trim().to_string());
                        }
                        _ => content_buffer.push_str(text),
                    }
                }

                let reasoning = if reasoning_segments.is_empty() {
                    None
                } else {
                    let joined = reasoning_segments.join("\n");
                    let trimmed = joined.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                };

                (content_buffer, reasoning)
            }
            _ => (String::new(), None),
        };

        let usage = response_json["usage"]
            .as_object()
            .map(|usage_obj| Usage {
                prompt_tokens: usage_obj.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as usize,
                completion_tokens: usage_obj.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as usize,
                total_tokens: usage_obj.get("total_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as usize,
            });

        Ok(LLMResponse {
            content,
            model: self.model.clone(),
            usage,
            reasoning: reasoning_from_content,
        })
    }

    fn backend_kind(&self) -> BackendKind {
        BackendKind::OpenAI
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}
use crate::config::constants::{model_helpers, models};
use crate::llm::client::LLMClient;
use crate::llm::error_display;
use crate::llm::provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
};
use crate::llm::types as llm_types;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};

pub struct AnthropicProvider {
    api_key: String,
    http_client: HttpClient,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: HttpClient::new(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let anthropic_request = self.convert_to_anthropic_format(&request)?;

        let url = format!("{}/messages", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| {
                let formatted_error =
                    error_display::format_llm_error("Anthropic", &format!("Network error: {}", e));
                LLMError::Network(formatted_error)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let formatted_error = error_display::format_llm_error(
                "Anthropic",
                &format!("HTTP {}: {}", status, error_text),
            );
            return Err(LLMError::Provider(formatted_error));
        }

        let anthropic_response: Value = response.json().await.map_err(|e| {
            let formatted_error = error_display::format_llm_error(
                "Anthropic",
                &format!("Failed to parse response: {}", e),
            );
            LLMError::Provider(formatted_error)
        })?;

        self.parse_anthropic_response(anthropic_response)
    }

    fn supported_models(&self) -> Vec<String> {
        models::anthropic::SUPPORTED_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if request.messages.is_empty() {
            return Err(LLMError::InvalidRequest(
                "Messages cannot be empty".to_string(),
            ));
        }

        if request.model.is_empty() {
            return Err(LLMError::InvalidRequest(
                "Model cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

impl AnthropicProvider {
    fn convert_to_anthropic_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut messages = Vec::new();

        // Convert messages (Anthropic doesn't have system messages in the messages array)
        for msg in &request.messages {
            // Skip system messages as they're handled separately
            if msg.role == MessageRole::System {
                continue;
            }

            // Anthropic handles tool responses as user messages
            // Based on official docs: tool responses are treated as user messages
            let role = match msg.role {
                MessageRole::Tool => "user", // Anthropic treats tool responses as user messages
                _ => msg.role.as_anthropic_str(),
            };

            let message = json!({
                "role": role,
                "content": msg.content
            });

            // Add tool_call_id for tool responses if present
            // This helps maintain context for tool call chains
            if msg.role == MessageRole::Tool {
                if let Some(_tool_call_id) = &msg.tool_call_id {
                    // Note: Anthropic doesn't use tool_call_id in the same way as OpenAI
                    // but we can include it as metadata in the content or ignore it
                    // For now, we'll include the tool response content as-is
                }
            }

            messages.push(message);
        }

        let mut anthropic_request = json!({
            "model": request.model,
            "messages": messages,
            "stream": request.stream
        });

        // Add system message if present
        if let Some(system_prompt) = &request.system_prompt {
            anthropic_request["system"] = json!(system_prompt);
        }

        // Add optional parameters
        if let Some(max_tokens) = request.max_tokens {
            anthropic_request["max_tokens"] = json!(max_tokens);
        } else {
            // Anthropic requires max_tokens
            anthropic_request["max_tokens"] = json!(4096);
        }

        if let Some(temperature) = request.temperature {
            anthropic_request["temperature"] = json!(temperature);
        }

        // Add tools if present
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                let tools_json: Vec<Value> = tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "name": tool.function.name,
                            "description": tool.function.description,
                            "input_schema": tool.function.parameters
                        })
                    })
                    .collect();
                anthropic_request["tools"] = Value::Array(tools_json);
            }
        }

        // Add tool_choice if specified - Anthropic format
        if let Some(tool_choice) = &request.tool_choice {
            anthropic_request["tool_choice"] = tool_choice.to_provider_format("anthropic");
        }

        // Note: Anthropic doesn't support parallel_tool_calls parameter
        // Tool calls are handled sequentially by default

        Ok(anthropic_request)
    }

    fn parse_anthropic_response(&self, response_json: Value) -> Result<LLMResponse, LLMError> {
        let content = response_json
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or_else(|| {
                LLMError::Provider("Invalid response format: missing content".to_string())
            })?
            .iter()
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("");

        // Parse tool calls from Anthropic content blocks
        // Anthropic uses content blocks with tool_use type instead of top-level tool_calls
        let tool_calls = response_json
            .get("content")
            .and_then(|content| content.as_array())
            .map(|content_blocks| {
                content_blocks
                    .iter()
                    .filter(|block| block.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
                    .filter_map(|block| {
                        let id = block.get("id")?.as_str()?.to_string();
                        let name = block.get("name")?.as_str()?.to_string();
                        let input = block.get("input").cloned().unwrap_or(json!({}));

                        Some(ToolCall {
                            id,
                            call_type: "function".to_string(),
                            function: crate::llm::provider::FunctionCall {
                                name,
                                arguments: serde_json::to_string(&input)
                                    .unwrap_or("{}".to_string()),
                            },
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|calls| !calls.is_empty());

        // Parse finish reason
        let stop_reason = response_json
            .get("stop_reason")
            .and_then(|sr| sr.as_str())
            .unwrap_or("end_turn");

        let finish_reason = match stop_reason {
            "end_turn" => FinishReason::Stop,
            "max_tokens" => FinishReason::Length,
            "stop_sequence" => FinishReason::Stop,
            "tool_use" => FinishReason::ToolCalls,
            _ => FinishReason::Stop,
        };

        // Parse usage
        let usage = response_json
            .get("usage")
            .map(|u| crate::llm::provider::Usage {
                prompt_tokens: u
                    .get("input_tokens")
                    .and_then(|it| it.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: u
                    .get("output_tokens")
                    .and_then(|ot| ot.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: (u
                    .get("input_tokens")
                    .and_then(|it| it.as_u64())
                    .unwrap_or(0)
                    + u.get("output_tokens")
                        .and_then(|ot| ot.as_u64())
                        .unwrap_or(0)) as u32,
            });

        Ok(LLMResponse {
            content: if content.is_empty() {
                None
            } else {
                Some(content)
            },
            tool_calls,
            usage,
            finish_reason,
        })
    }
}

#[async_trait]
impl LLMClient for AnthropicProvider {
    async fn generate(&mut self, prompt: &str) -> Result<llm_types::LLMResponse, LLMError> {
        let model = models::anthropic::DEFAULT_MODEL.to_string();

        // Validate the model
        if !model_helpers::is_valid("anthropic", &model) {
            return Err(LLMError::InvalidRequest(format!(
                "Invalid Anthropic model '{}'. See docs/models.json",
                model
            )));
        }

        let request = LLMRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            system_prompt: None,
            tools: None,
            model: model.clone(),
            max_tokens: None,
            temperature: None,
            stream: false,
            tool_choice: None,
            parallel_tool_calls: None,
            parallel_tool_config: None,
            reasoning_effort: None,
        };

        let response = LLMProvider::generate(self, request.clone()).await?;

        Ok(llm_types::LLMResponse {
            content: response.content.unwrap_or("".to_string()),
            model,
            usage: response.usage.map(|u| llm_types::Usage {
                prompt_tokens: u.prompt_tokens as usize,
                completion_tokens: u.completion_tokens as usize,
                total_tokens: u.total_tokens as usize,
            }),
        })
    }

    fn backend_kind(&self) -> llm_types::BackendKind {
        llm_types::BackendKind::Anthropic
    }

    fn model_id(&self) -> &str {
        models::anthropic::DEFAULT_MODEL
    }
}

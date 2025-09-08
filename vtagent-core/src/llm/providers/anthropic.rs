use super::super::provider::{LLMProvider, LLMRequest, LLMResponse, LLMError, Message, MessageRole, Usage, FinishReason, ToolCall};
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};

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

        let response = self.http_client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!("HTTP {}: {}", status, error_text)));
        }

        let anthropic_response: Value = response.json().await
            .map_err(|e| LLMError::Provider(e.to_string()))?;

        self.convert_from_anthropic_format(anthropic_response)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
        ]
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if !self.supported_models().contains(&request.model) {
            return Err(LLMError::InvalidRequest(format!("Unsupported model: {}", request.model)));
        }
        Ok(())
    }
}

impl AnthropicProvider {
    fn convert_to_anthropic_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut messages = Vec::new();
        
        for message in &request.messages {
            match message.role {
                MessageRole::User => {
                    messages.push(json!({
                        "role": "user",
                        "content": message.content
                    }));
                }
                MessageRole::Assistant => {
                    let mut content = Vec::new();
                    
                    // Add text content if present
                    if !message.content.is_empty() {
                        content.push(json!({
                            "type": "text",
                            "text": message.content
                        }));
                    }
                    
                    // Add tool_use blocks if present
                    if let Some(tool_calls) = &message.tool_calls {
                        for tool_call in tool_calls {
                            content.push(json!({
                                "type": "tool_use",
                                "id": tool_call.id,
                                "name": tool_call.name,
                                "input": tool_call.arguments
                            }));
                        }
                    }
                    
                    messages.push(json!({
                        "role": "assistant",
                        "content": content
                    }));
                }
                MessageRole::System => continue, // Handle separately
                MessageRole::Tool => {
                    // Tool results should be user messages with tool_result content blocks
                    if let Some(tool_calls) = &message.tool_calls {
                        let tool_results: Vec<Value> = tool_calls.iter().map(|call| {
                            json!({
                                "type": "tool_result",
                                "tool_use_id": call.id,
                                "content": message.content
                            })
                        }).collect();
                        
                        messages.push(json!({
                            "role": "user",
                            "content": tool_results
                        }));
                    } else {
                        // Fallback: treat as regular user message
                        messages.push(json!({
                            "role": "user",
                            "content": message.content
                        }));
                    }
                }
            }
        }

        let mut anthropic_request = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096)
        });

        if let Some(system) = &request.system_prompt {
            anthropic_request["system"] = json!(system);
        }
        
        if let Some(temperature) = request.temperature {
            anthropic_request["temperature"] = json!(temperature);
        }

        // Handle tools if present
        if let Some(tools) = &request.tools {
            let anthropic_tools: Vec<Value> = tools.iter().map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.parameters
                })
            }).collect();
            anthropic_request["tools"] = json!(anthropic_tools);
        }

        Ok(anthropic_request)
    }

    fn convert_from_anthropic_format(&self, response: Value) -> Result<LLMResponse, LLMError> {
        let content_array = response["content"].as_array()
            .ok_or_else(|| LLMError::Provider("No content array in response".to_string()))?;

        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        // Parse content blocks
        for content_block in content_array {
            match content_block["type"].as_str() {
                Some("text") => {
                    if let Some(text) = content_block["text"].as_str() {
                        text_content.push_str(text);
                    }
                }
                Some("tool_use") => {
                    if let (Some(id), Some(name)) = (
                        content_block["id"].as_str(),
                        content_block["name"].as_str()
                    ) {
                        let input = content_block["input"].clone();
                        tool_calls.push(ToolCall {
                            id: id.to_string(),
                            name: name.to_string(),
                            arguments: input,
                        });
                    }
                }
                _ => {} // Ignore unknown content types
            }
        }

        let usage = response["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: (u["input_tokens"].as_u64().unwrap_or(0) + u["output_tokens"].as_u64().unwrap_or(0)) as u32,
        });

        let finish_reason = match response["stop_reason"].as_str() {
            Some("end_turn") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("stop_sequence") => FinishReason::Stop,
            Some("tool_use") => FinishReason::ToolCalls,
            Some(other) => FinishReason::Error(other.to_string()),
            None => FinishReason::Stop,
        };

        Ok(LLMResponse {
            content: if text_content.is_empty() { None } else { Some(text_content) },
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            usage,
            finish_reason,
        })
    }
}

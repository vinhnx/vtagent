use crate::config::constants::{models, urls};
use crate::llm::client::LLMClient;
use crate::llm::error_display;
use crate::llm::provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
    ToolChoice, ToolDefinition,
};
use crate::llm::types as llm_types;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};

use super::extract_reasoning_trace;

pub struct OpenRouterProvider {
    api_key: String,
    http_client: HttpClient,
    base_url: String,
    model: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, models::openrouter::DEFAULT_MODEL.to_string())
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            api_key,
            http_client: HttpClient::new(),
            base_url: urls::OPENROUTER_API_BASE.to_string(),
            model,
        }
    }

    pub fn from_config(
        api_key: Option<String>,
        model: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let api_key_value = api_key.unwrap_or_default();
        let mut provider = if let Some(model_value) = model {
            Self::with_model(api_key_value, model_value)
        } else {
            Self::new(api_key_value)
        };
        if let Some(base) = base_url {
            provider.base_url = base;
        }
        provider
    }

    fn default_request(&self, prompt: &str) -> LLMRequest {
        LLMRequest {
            messages: vec![Message::user(prompt.to_string())],
            system_prompt: None,
            tools: None,
            model: self.model.clone(),
            max_tokens: None,
            temperature: None,
            stream: false,
            tool_choice: None,
            parallel_tool_calls: None,
            parallel_tool_config: None,
            reasoning_effort: None,
        }
    }

    fn parse_client_prompt(&self, prompt: &str) -> LLMRequest {
        let trimmed = prompt.trim_start();
        if trimmed.starts_with('{') {
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                if let Some(request) = self.parse_chat_request(&value) {
                    return request;
                }
            }
        }

        self.default_request(prompt)
    }

    fn parse_chat_request(&self, value: &Value) -> Option<LLMRequest> {
        let messages_value = value.get("messages")?.as_array()?;
        let mut system_prompt = None;
        let mut messages = Vec::new();

        for entry in messages_value {
            let role = entry
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or(crate::config::constants::message_roles::USER);
            let content = entry.get("content");
            let text_content = content.map(Self::extract_content_text).unwrap_or_default();

            match role {
                "system" => {
                    if system_prompt.is_none() && !text_content.is_empty() {
                        system_prompt = Some(text_content);
                    }
                }
                "assistant" => {
                    let tool_calls = entry
                        .get("tool_calls")
                        .and_then(|tc| tc.as_array())
                        .map(|calls| {
                            calls
                                .iter()
                                .filter_map(|call| {
                                    let id = call.get("id").and_then(|v| v.as_str())?;
                                    let function = call.get("function")?;
                                    let name = function.get("name").and_then(|v| v.as_str())?;
                                    let arguments = function.get("arguments");
                                    let serialized = arguments.map_or("{}".to_string(), |value| {
                                        if value.is_string() {
                                            value.as_str().unwrap_or("").to_string()
                                        } else {
                                            value.to_string()
                                        }
                                    });
                                    Some(ToolCall::function(
                                        id.to_string(),
                                        name.to_string(),
                                        serialized,
                                    ))
                                })
                                .collect::<Vec<_>>()
                        })
                        .filter(|calls| !calls.is_empty());

                    let message = if let Some(calls) = tool_calls {
                        Message {
                            role: MessageRole::Assistant,
                            content: text_content,
                            tool_calls: Some(calls),
                            tool_call_id: None,
                        }
                    } else {
                        Message::assistant(text_content)
                    };
                    messages.push(message);
                }
                "tool" => {
                    let tool_call_id = entry
                        .get("tool_call_id")
                        .and_then(|id| id.as_str())
                        .map(|s| s.to_string());
                    let content_value = entry
                        .get("content")
                        .map(|value| {
                            if text_content.is_empty() {
                                value.to_string()
                            } else {
                                text_content.clone()
                            }
                        })
                        .unwrap_or_else(|| text_content.clone());
                    messages.push(Message {
                        role: MessageRole::Tool,
                        content: content_value,
                        tool_calls: None,
                        tool_call_id,
                    });
                }
                _ => {
                    messages.push(Message::user(text_content));
                }
            }
        }

        if messages.is_empty() {
            return None;
        }

        let tools = value.get("tools").and_then(|tools_value| {
            let tools_array = tools_value.as_array()?;
            let converted: Vec<_> = tools_array
                .iter()
                .filter_map(|tool| {
                    let function = tool.get("function")?;
                    let name = function.get("name").and_then(|n| n.as_str())?;
                    let description = function
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();
                    let parameters = function
                        .get("parameters")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    Some(ToolDefinition::function(
                        name.to_string(),
                        description,
                        parameters,
                    ))
                })
                .collect();

            if converted.is_empty() {
                None
            } else {
                Some(converted)
            }
        });

        let max_tokens = value
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let temperature = value
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        let stream = value
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tool_choice = value.get("tool_choice").and_then(Self::parse_tool_choice);
        let parallel_tool_calls = value.get("parallel_tool_calls").and_then(|v| v.as_bool());
        let reasoning_effort = value
            .get("reasoning_effort")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                value
                    .get("reasoning")
                    .and_then(|r| r.get("effort"))
                    .and_then(|effort| effort.as_str())
                    .map(|s| s.to_string())
            });

        let model = value
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or(&self.model)
            .to_string();

        Some(LLMRequest {
            messages,
            system_prompt,
            tools,
            model,
            max_tokens,
            temperature,
            stream,
            tool_choice,
            parallel_tool_calls,
            parallel_tool_config: None,
            reasoning_effort,
        })
    }

    fn extract_content_text(content: &Value) -> String {
        match content {
            Value::String(text) => text.to_string(),
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| {
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        Some(text.to_string())
                    } else if let Some(Value::String(text)) = part.get("content") {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(""),
            _ => String::new(),
        }
    }

    fn parse_tool_choice(choice: &Value) -> Option<ToolChoice> {
        match choice {
            Value::String(value) => match value.as_str() {
                "auto" => Some(ToolChoice::auto()),
                "none" => Some(ToolChoice::none()),
                "required" => Some(ToolChoice::any()),
                _ => None,
            },
            Value::Object(map) => {
                let choice_type = map.get("type").and_then(|t| t.as_str())?;
                match choice_type {
                    "function" => map
                        .get("function")
                        .and_then(|f| f.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|name| ToolChoice::function(name.to_string())),
                    "auto" => Some(ToolChoice::auto()),
                    "none" => Some(ToolChoice::none()),
                    "any" | "required" => Some(ToolChoice::any()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn convert_to_openrouter_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut messages = Vec::new();

        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": crate::config::constants::message_roles::SYSTEM,
                "content": system_prompt
            }));
        }

        for msg in &request.messages {
            let role = msg.role.as_openai_str();
            let mut message = json!({
                "role": role,
                "content": msg.content
            });

            if msg.role == MessageRole::Assistant {
                if let Some(tool_calls) = &msg.tool_calls {
                    if !tool_calls.is_empty() {
                        let tool_calls_json: Vec<Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.function.name,
                                        "arguments": tc.function.arguments
                                    }
                                })
                            })
                            .collect();
                        message["tool_calls"] = Value::Array(tool_calls_json);
                    }
                }
            }

            if msg.role == MessageRole::Tool {
                if let Some(tool_call_id) = &msg.tool_call_id {
                    message["tool_call_id"] = Value::String(tool_call_id.clone());
                }
            }

            messages.push(message);
        }

        if messages.is_empty() {
            let formatted_error =
                error_display::format_llm_error("OpenRouter", "No messages provided");
            return Err(LLMError::InvalidRequest(formatted_error));
        }

        let mut provider_request = json!({
            "model": if request.model.trim().is_empty() {
                &self.model
            } else {
                &request.model
            },
            "messages": messages,
            "stream": request.stream
        });

        if let Some(max_tokens) = request.max_tokens {
            provider_request["max_tokens"] = json!(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            provider_request["temperature"] = json!(temperature);
        }

        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                let tools_json: Vec<Value> = tools
                    .iter()
                    .map(|tool| {
                        json!({
                            "type": "function",
                            "function": {
                                "name": tool.function.name,
                                "description": tool.function.description,
                                "parameters": tool.function.parameters
                            }
                        })
                    })
                    .collect();
                provider_request["tools"] = Value::Array(tools_json);
            }
        }

        if let Some(tool_choice) = &request.tool_choice {
            provider_request["tool_choice"] = tool_choice.to_provider_format("openai");
        }

        if let Some(parallel) = request.parallel_tool_calls {
            provider_request["parallel_tool_calls"] = Value::Bool(parallel);
        }

        if let Some(reasoning) = &request.reasoning_effort {
            provider_request["reasoning"] = json!({"effort": reasoning});
        }

        Ok(provider_request)
    }

    fn parse_openrouter_response(&self, response_json: Value) -> Result<LLMResponse, LLMError> {
        let choices = response_json
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| {
                let formatted_error = error_display::format_llm_error(
                    "OpenRouter",
                    "Invalid response format: missing choices",
                );
                LLMError::Provider(formatted_error)
            })?;

        if choices.is_empty() {
            let formatted_error =
                error_display::format_llm_error("OpenRouter", "No choices in response");
            return Err(LLMError::Provider(formatted_error));
        }

        let choice = &choices[0];
        let message = choice.get("message").ok_or_else(|| {
            let formatted_error = error_display::format_llm_error(
                "OpenRouter",
                "Invalid response format: missing message",
            );
            LLMError::Provider(formatted_error)
        })?;

        let (content, reasoning_from_content) = match message.get("content") {
            Some(Value::String(text)) => {
                let trimmed = text.trim();
                let content = if trimmed.is_empty() {
                    None
                } else {
                    Some(text.to_string())
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
                        Some("reasoning")
                        | Some("analysis")
                        | Some("thought")
                        | Some("chain_of_thought") => {
                            reasoning_segments.push(text.trim().to_string());
                        }
                        _ => {
                            content_buffer.push_str(text);
                        }
                    }
                }

                let content = if content_buffer.trim().is_empty() {
                    None
                } else {
                    Some(content_buffer)
                };

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

                (content, reasoning)
            }
            _ => (None, None),
        };

        let tool_calls = message
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| {
                        let id = call.get("id").and_then(|v| v.as_str())?;
                        let function = call.get("function")?;
                        let name = function.get("name").and_then(|v| v.as_str())?;
                        let arguments = function.get("arguments");
                        let serialized = arguments.map_or("{}".to_string(), |value| {
                            if value.is_string() {
                                value.as_str().unwrap_or("").to_string()
                            } else {
                                value.to_string()
                            }
                        });
                        Some(ToolCall::function(
                            id.to_string(),
                            name.to_string(),
                            serialized,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|calls| !calls.is_empty());

        let reasoning = reasoning_from_content
            .or_else(|| message.get("reasoning").and_then(extract_reasoning_trace))
            .or_else(|| choice.get("reasoning").and_then(extract_reasoning_trace));

        let finish_reason = choice
            .get("finish_reason")
            .and_then(|fr| fr.as_str())
            .map(|fr| match fr {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCalls,
                "content_filter" => FinishReason::ContentFilter,
                other => FinishReason::Error(other.to_string()),
            })
            .unwrap_or(FinishReason::Stop);

        let usage = response_json
            .get("usage")
            .map(|usage_value| crate::llm::provider::Usage {
                prompt_tokens: usage_value
                    .get("prompt_tokens")
                    .and_then(|pt| pt.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: usage_value
                    .get("completion_tokens")
                    .and_then(|ct| ct.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: usage_value
                    .get("total_tokens")
                    .and_then(|tt| tt.as_u64())
                    .unwrap_or(0) as u32,
            });

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            finish_reason,
            reasoning,
        })
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let provider_request = self.convert_to_openrouter_format(&request)?;
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&provider_request)
            .send()
            .await
            .map_err(|e| {
                let formatted_error =
                    error_display::format_llm_error("OpenRouter", &format!("Network error: {}", e));
                LLMError::Network(formatted_error)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 || error_text.contains("quota") {
                return Err(LLMError::RateLimit);
            }

            let formatted_error = error_display::format_llm_error(
                "OpenRouter",
                &format!("HTTP {}: {}", status, error_text),
            );
            return Err(LLMError::Provider(formatted_error));
        }

        let openrouter_response: Value = response.json().await.map_err(|e| {
            let formatted_error = error_display::format_llm_error(
                "OpenRouter",
                &format!("Failed to parse response: {}", e),
            );
            LLMError::Provider(formatted_error)
        })?;

        self.parse_openrouter_response(openrouter_response)
    }

    fn supported_models(&self) -> Vec<String> {
        models::openrouter::SUPPORTED_MODELS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if request.messages.is_empty() {
            let formatted_error =
                error_display::format_llm_error("OpenRouter", "Messages cannot be empty");
            return Err(LLMError::InvalidRequest(formatted_error));
        }

        for message in &request.messages {
            if let Err(err) = message.validate_for_provider("openai") {
                let formatted = error_display::format_llm_error("OpenRouter", &err);
                return Err(LLMError::InvalidRequest(formatted));
            }
        }

        if request.model.trim().is_empty() {
            let formatted_error =
                error_display::format_llm_error("OpenRouter", "Model must be provided");
            return Err(LLMError::InvalidRequest(formatted_error));
        }

        Ok(())
    }
}

#[async_trait]
impl LLMClient for OpenRouterProvider {
    async fn generate(&mut self, prompt: &str) -> Result<llm_types::LLMResponse, LLMError> {
        let request = self.parse_client_prompt(prompt);
        let request_model = request.model.clone();
        let response = LLMProvider::generate(self, request).await?;

        Ok(llm_types::LLMResponse {
            content: response.content.unwrap_or_default(),
            model: request_model,
            usage: response.usage.map(|u| llm_types::Usage {
                prompt_tokens: u.prompt_tokens as usize,
                completion_tokens: u.completion_tokens as usize,
                total_tokens: u.total_tokens as usize,
            }),
            reasoning: response.reasoning,
        })
    }

    fn backend_kind(&self) -> llm_types::BackendKind {
        llm_types::BackendKind::OpenRouter
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

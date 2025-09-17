use crate::config::constants::{model_helpers, models};
use crate::llm::client::LLMClient;
use crate::llm::error_display;
use crate::llm::provider::{
    FinishReason, FunctionCall, LLMError, LLMProvider, LLMRequest, LLMResponse, LLMStream,
    LLMStreamChunk, Message, MessageRole, ToolCall, Usage,
};
use crate::llm::stream;
use crate::llm::types as llm_types;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

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

    async fn stream(&self, request: LLMRequest) -> Result<LLMStream, LLMError> {
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

        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut aggregator = AnthropicStreamAggregator::default();
            let mut byte_stream = response.bytes_stream();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        buffer.push_str(&text.replace("\r\n", "\n"));

                        for payload in stream::drain_sse_events(&mut buffer) {
                            if let Err(err) = aggregator.process_event(&payload, &sender) {
                                let _ = sender.send(Err(err));
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        let formatted_error = error_display::format_llm_error(
                            "Anthropic",
                            &format!("Failed to read stream: {}", err),
                        );
                        let _ = sender.send(Err(LLMError::Network(formatted_error)));
                        return;
                    }
                }
            }

            if let Err(err) = aggregator.finish(&sender) {
                let _ = sender.send(Err(err));
            }
        });

        Ok(Box::pin(UnboundedReceiverStream::new(receiver)))
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

            // Tool responses must be represented as tool_result content in a user message
            if msg.role == MessageRole::Tool {
                if let Some(tool_call_id) = &msg.tool_call_id {
                    messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_call_id,
                            "content": [
                                {"type": "text", "text": msg.content}
                            ]
                        }]
                    }));
                } else {
                    // Fallback: treat as plain user message if id missing
                    messages.push(json!({
                        "role": "user",
                        "content": [{"type": "text", "text": msg.content}]
                    }));
                }
            } else {
                messages.push(message);
            }
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

#[derive(Default)]
struct AnthropicStreamAggregator {
    content: String,
    tool_uses: Vec<AnthropicToolUseBuilder>,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
    done: bool,
}

impl AnthropicStreamAggregator {
    fn process_event(
        &mut self,
        payload: &str,
        sender: &mpsc::UnboundedSender<Result<LLMStreamChunk, LLMError>>,
    ) -> Result<(), LLMError> {
        if payload.trim() == "[DONE]" {
            return self.finish(sender);
        }

        let parsed: Value = serde_json::from_str(payload).map_err(|err| {
            let formatted = error_display::format_llm_error(
                "Anthropic",
                &format!("Failed to parse stream chunk: {}", err),
            );
            LLMError::Provider(formatted)
        })?;

        let event_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            "message_start" => {
                if let Some(usage) = parsed.get("message").and_then(|msg| msg.get("usage")) {
                    self.usage = Some(parse_usage(usage));
                }
            }
            "message_delta" => {
                if let Some(delta) = parsed.get("delta") {
                    if let Some(reason) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                        self.finish_reason = Some(map_anthropic_finish_reason(reason));
                    }
                }
                if let Some(usage) = parsed.get("usage") {
                    self.usage = Some(parse_usage(usage));
                }
            }
            "content_block_start" => {
                let index = parsed.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                if let Some(block) = parsed.get("content_block") {
                    if block.get("type").and_then(|v| v.as_str()).unwrap_or("") == "tool_use" {
                        let builder = self.ensure_tool_builder(index);
                        builder.id = block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        builder.name = block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        if let Some(input) = block.get("input") {
                            builder.set_input_value(input.clone());
                        }
                    }
                }
            }
            "content_block_delta" => {
                let index = parsed.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                if let Some(delta) = parsed.get("delta") {
                    match delta.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                        "text_delta" => {
                            if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                if !text.is_empty() {
                                    if sender
                                        .send(Ok(LLMStreamChunk {
                                            delta: Some(text.to_string()),
                                            response: None,
                                        }))
                                        .is_err()
                                    {
                                        return Err(receiver_dropped_error("Anthropic"));
                                    }
                                    self.content.push_str(text);
                                }
                            }
                        }
                        "input_json_delta" => {
                            let builder = self.ensure_tool_builder(index);
                            if let Some(partial) =
                                delta.get("partial_json").and_then(|v| v.as_str())
                            {
                                builder.append_partial(partial);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "message_stop" => {
                if self.finish_reason.is_none() {
                    self.finish_reason = Some(FinishReason::Stop);
                }
            }
            "error" => {
                let message = parsed
                    .get("error")
                    .and_then(|err| err.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Streaming error");
                return Err(LLMError::Provider(error_display::format_llm_error(
                    "Anthropic",
                    message,
                )));
            }
            _ => {}
        }

        Ok(())
    }

    fn finish(
        &mut self,
        sender: &mpsc::UnboundedSender<Result<LLMStreamChunk, LLMError>>,
    ) -> Result<(), LLMError> {
        if self.done {
            return Ok(());
        }
        self.done = true;

        let tool_calls = if self.tool_uses.is_empty() {
            None
        } else {
            Some(
                self.tool_uses
                    .iter()
                    .enumerate()
                    .map(|(idx, builder)| builder.to_tool_call(idx))
                    .collect::<Vec<_>>(),
            )
        };

        let response = LLMResponse {
            content: if self.content.is_empty() {
                None
            } else {
                Some(self.content.clone())
            },
            tool_calls,
            usage: self.usage.clone(),
            finish_reason: self.finish_reason.clone().unwrap_or(FinishReason::Stop),
        };

        sender
            .send(Ok(LLMStreamChunk {
                delta: None,
                response: Some(response),
            }))
            .map_err(|_| receiver_dropped_error("Anthropic"))
    }

    fn ensure_tool_builder(&mut self, index: usize) -> &mut AnthropicToolUseBuilder {
        if self.tool_uses.len() <= index {
            self.tool_uses
                .resize_with(index + 1, AnthropicToolUseBuilder::default);
        }
        &mut self.tool_uses[index]
    }
}

#[derive(Default)]
struct AnthropicToolUseBuilder {
    id: Option<String>,
    name: Option<String>,
    input_buffer: String,
}

impl AnthropicToolUseBuilder {
    fn set_input_value(&mut self, value: Value) {
        self.input_buffer = match serde_json::to_string(&value) {
            Ok(text) => text,
            Err(_) => "{}".to_string(),
        };
    }

    fn append_partial(&mut self, partial: &str) {
        self.input_buffer.push_str(partial);
    }

    fn to_tool_call(&self, index: usize) -> ToolCall {
        ToolCall {
            id: self
                .id
                .clone()
                .unwrap_or_else(|| format!("tool_use_{}", index)),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: self.name.clone().unwrap_or_default(),
                arguments: if self.input_buffer.is_empty() {
                    "{}".to_string()
                } else {
                    self.input_buffer.clone()
                },
            },
        }
    }
}

fn parse_usage(value: &Value) -> Usage {
    let prompt = value
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let completion = value
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    Usage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: prompt + completion,
    }
}

fn map_anthropic_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "end_turn" => FinishReason::Stop,
        "max_tokens" => FinishReason::Length,
        "stop_sequence" => FinishReason::Stop,
        "tool_use" => FinishReason::ToolCalls,
        other => FinishReason::Error(other.to_string()),
    }
}

fn receiver_dropped_error(provider: &str) -> LLMError {
    let formatted = error_display::format_llm_error(provider, "Stream receiver dropped");
    LLMError::Provider(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn aggregator_streams_text_and_finalizes() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let mut aggregator = AnthropicStreamAggregator::default();

        aggregator
            .process_event(
                r#"{"type":"message_start","message":{"usage":{"input_tokens":5,"output_tokens":0}}}"#,
                &sender,
            )
            .expect("start");
        aggregator
            .process_event(
                r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
                &sender,
            )
            .expect("block start");
        aggregator
            .process_event(
                r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
                &sender,
            )
            .expect("delta");
        aggregator
            .process_event(
                r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}}"#,
                &sender,
            )
            .expect("message delta");
        aggregator
            .process_event("[DONE]", &sender)
            .expect("finalize");

        let delta_chunk = receiver.recv().await.expect("delta").unwrap();
        assert_eq!(delta_chunk.delta.as_deref(), Some("Hi"));

        let final_chunk = receiver.recv().await.expect("final").unwrap();
        let response = final_chunk.response.expect("response");
        assert_eq!(response.content.as_deref(), Some("Hi"));
        assert!(matches!(response.finish_reason, FinishReason::Stop));
        let usage = response.usage.expect("usage");
        assert_eq!(usage.prompt_tokens, 5);
        assert_eq!(usage.completion_tokens, 2);
    }
}

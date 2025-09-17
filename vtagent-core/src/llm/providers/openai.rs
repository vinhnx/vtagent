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
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub struct OpenAIProvider {
    api_key: String,
    http_client: HttpClient,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: HttpClient::new(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let openai_request = self.convert_to_openai_format(&request)?;

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| {
                let formatted_error =
                    error_display::format_llm_error("OpenAI", &format!("Network error: {}", e));
                LLMError::Network(formatted_error)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let formatted_error = error_display::format_llm_error(
                "OpenAI",
                &format!("HTTP {}: {}", status, error_text),
            );
            return Err(LLMError::Provider(formatted_error));
        }

        let openai_response: Value = response.json().await.map_err(|e| {
            let formatted_error = error_display::format_llm_error(
                "OpenAI",
                &format!("Failed to parse response: {}", e),
            );
            LLMError::Provider(formatted_error)
        })?;

        self.parse_openai_response(openai_response)
    }

    async fn stream(&self, request: LLMRequest) -> Result<LLMStream, LLMError> {
        let openai_request = self.convert_to_openai_format(&request)?;

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| {
                let formatted_error =
                    error_display::format_llm_error("OpenAI", &format!("Network error: {}", e));
                LLMError::Network(formatted_error)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let formatted_error = error_display::format_llm_error(
                "OpenAI",
                &format!("HTTP {}: {}", status, error_text),
            );
            return Err(LLMError::Provider(formatted_error));
        }

        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut stream_buffer = String::new();
            let mut aggregator = OpenAIStreamAggregator::default();
            let mut byte_stream = response.bytes_stream();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        stream_buffer.push_str(&text.replace("\r\n", "\n"));

                        for payload in stream::drain_sse_events(&mut stream_buffer) {
                            if let Err(err) = aggregator.process_event(&payload, &sender) {
                                let _ = sender.send(Err(err));
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        let formatted_error = error_display::format_llm_error(
                            "OpenAI",
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
        models::openai::SUPPORTED_MODELS
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

impl OpenAIProvider {
    fn convert_to_openai_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut messages = Vec::new();

        // Add system message if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": crate::config::constants::message_roles::SYSTEM,
                "content": system_prompt
            }));
        }

        // Convert messages
        for msg in &request.messages {
            // Use the proper role mapping for OpenAI
            let role = msg.role.as_openai_str();

            let mut message = json!({
                "role": role,
                "content": msg.content
            });

            // Add tool call information for assistant messages
            // Based on OpenAI docs: only assistant messages can have tool_calls
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

            // Add tool_call_id for tool messages
            // Based on OpenAI docs: tool messages must have tool_call_id
            if msg.role == MessageRole::Tool {
                if let Some(tool_call_id) = &msg.tool_call_id {
                    message["tool_call_id"] = Value::String(tool_call_id.clone());
                } else {
                    // This should not happen in well-formed requests
                    eprintln!("Warning: Tool message without tool_call_id in OpenAI request");
                }
            }

            messages.push(message);
        }

        let mut openai_request = json!({
            "model": request.model,
            "messages": messages,
            "stream": request.stream
        });

        // Add optional parameters
        if let Some(max_tokens) = request.max_tokens {
            openai_request["max_tokens"] = json!(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            openai_request["temperature"] = json!(temperature);
        }

        // Add tools if present
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
                openai_request["tools"] = Value::Array(tools_json);
            }
        }

        // Add tool_choice if specified
        if let Some(tool_choice) = &request.tool_choice {
            openai_request["tool_choice"] = tool_choice.to_provider_format("openai");
        }

        // Add parallel_tool_calls if specified
        if let Some(parallel) = request.parallel_tool_calls {
            openai_request["parallel_tool_calls"] = Value::Bool(parallel);
        }

        // Add reasoning_effort for models that support it (GPT-5 etc.)
        if let Some(reasoning_effort) = &request.reasoning_effort {
            if request.model.contains(models::openai::GPT_5)
                || request.model.contains(models::openai::GPT_5_MINI)
            {
                openai_request["reasoning_effort"] = json!(reasoning_effort);
            }
        }

        Ok(openai_request)
    }

    fn parse_openai_response(&self, response_json: Value) -> Result<LLMResponse, LLMError> {
        let choices = response_json
            .get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| {
                LLMError::Provider("Invalid response format: missing choices".to_string())
            })?;

        if choices.is_empty() {
            return Err(LLMError::Provider("No choices in response".to_string()));
        }

        let choice = &choices[0];
        let message = choice.get("message").ok_or_else(|| {
            LLMError::Provider("Invalid response format: missing message".to_string())
        })?;

        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        // Parse tool calls
        let tool_calls = message
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| {
                        Some(ToolCall {
                            id: call.get("id")?.as_str()?.to_string(),
                            call_type: "function".to_string(),
                            function: crate::llm::provider::FunctionCall {
                                name: call.get("function")?.get("name")?.as_str()?.to_string(),
                                arguments: call
                                    .get("function")
                                    .and_then(|f| f.get("arguments"))
                                    .and_then(|args| args.as_str())
                                    .unwrap_or("{}")
                                    .to_string(),
                            },
                        })
                    })
                    .collect()
            });

        // Parse finish reason
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|fr| fr.as_str())
            .map(|fr| match fr {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCalls,
                "content_filter" => FinishReason::ContentFilter,
                _ => FinishReason::Error(fr.to_string()),
            })
            .unwrap_or(FinishReason::Stop);

        // Parse usage
        let usage = response_json
            .get("usage")
            .map(|u| crate::llm::provider::Usage {
                prompt_tokens: u
                    .get("prompt_tokens")
                    .and_then(|pt| pt.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: u
                    .get("completion_tokens")
                    .and_then(|ct| ct.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: u
                    .get("total_tokens")
                    .and_then(|tt| tt.as_u64())
                    .unwrap_or(0) as u32,
            });

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            finish_reason,
        })
    }
}

#[async_trait]
impl LLMClient for OpenAIProvider {
    async fn generate(&mut self, _prompt: &str) -> Result<llm_types::LLMResponse, LLMError> {
        let model = models::openai::DEFAULT_MODEL.to_string();

        // Validate the model
        if !model_helpers::is_valid("openai", &model) {
            return Err(LLMError::InvalidRequest(format!(
                "Invalid OpenAI model '{}'. See docs/models.json",
                model
            )));
        }

        let request = LLMRequest {
            messages: vec![Message::user("test".to_string())],
            system_prompt: None,
            tools: None,
            model: "test".to_string(),
            max_tokens: Some(100),
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
        llm_types::BackendKind::OpenAI
    }

    fn model_id(&self) -> &str {
        models::openai::DEFAULT_MODEL
    }
}

#[derive(Default)]
struct OpenAIStreamAggregator {
    content: String,
    tool_calls: Vec<OpenAIToolCallBuilder>,
    finish_reason: Option<FinishReason>,
    usage: Option<Usage>,
    done: bool,
}

impl OpenAIStreamAggregator {
    fn process_event(
        &mut self,
        payload: &str,
        sender: &mpsc::UnboundedSender<Result<LLMStreamChunk, LLMError>>,
    ) -> Result<(), LLMError> {
        if payload.trim() == "[DONE]" {
            return self.finish(sender);
        }

        let chunk: OpenAIStreamResponse = serde_json::from_str(payload).map_err(|err| {
            let formatted = error_display::format_llm_error(
                "OpenAI",
                &format!("Failed to parse stream chunk: {}", err),
            );
            LLMError::Provider(formatted)
        })?;

        if let Some(usage) = chunk.usage {
            self.usage = Some(usage.into());
        }

        for choice in chunk.choices {
            if let Some(reason) = choice.finish_reason {
                self.finish_reason = Some(map_openai_finish_reason(&reason));
            }

            if let Some(delta) = choice.delta.content {
                if !delta.is_empty() {
                    if sender
                        .send(Ok(LLMStreamChunk {
                            delta: Some(delta.clone()),
                            response: None,
                        }))
                        .is_err()
                    {
                        return Err(receiver_dropped_error("OpenAI"));
                    }
                    self.content.push_str(&delta);
                }
            }

            for tool_delta in choice.delta.tool_calls.unwrap_or_default() {
                let index = tool_delta.index.unwrap_or(0) as usize;
                if self.tool_calls.len() <= index {
                    self.tool_calls
                        .resize_with(index + 1, OpenAIToolCallBuilder::default);
                }
                let builder = &mut self.tool_calls[index];
                if let Some(id) = tool_delta.id {
                    builder.id = Some(id);
                }
                if let Some(function) = tool_delta.function {
                    if let Some(name) = function.name {
                        builder.name = Some(name);
                    }
                    if let Some(arguments) = function.arguments {
                        builder.arguments.push_str(&arguments);
                    }
                }
            }
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

        let tool_calls = if self.tool_calls.is_empty() {
            None
        } else {
            let converted: Vec<ToolCall> = self
                .tool_calls
                .iter()
                .enumerate()
                .map(|(idx, builder)| builder.to_tool_call(idx))
                .collect();
            Some(converted)
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
            .map_err(|_| receiver_dropped_error("OpenAI"))
    }
}

#[derive(Default)]
struct OpenAIToolCallBuilder {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl OpenAIToolCallBuilder {
    fn to_tool_call(&self, index: usize) -> ToolCall {
        ToolCall {
            id: self.id.clone().unwrap_or_else(|| format!("call_{}", index)),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: self.name.clone().unwrap_or_default(),
                arguments: if self.arguments.is_empty() {
                    "{}".to_string()
                } else {
                    self.arguments.clone()
                },
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
    #[serde(default)]
    usage: Option<OpenAIStreamUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIStreamToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamToolCallDelta {
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAIStreamToolFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamToolFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamUsage {
    #[serde(default)]
    prompt_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
    #[serde(default)]
    total_tokens: Option<u32>,
}

impl From<OpenAIStreamUsage> for Usage {
    fn from(value: OpenAIStreamUsage) -> Self {
        let prompt = value.prompt_tokens.unwrap_or(0);
        let completion = value.completion_tokens.unwrap_or(0);
        let total = value.total_tokens.unwrap_or(prompt + completion);
        Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
        }
    }
}

fn map_openai_finish_reason(reason: &str) -> FinishReason {
    match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        "tool_calls" => FinishReason::ToolCalls,
        "content_filter" => FinishReason::ContentFilter,
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
    async fn aggregator_streams_text_and_final_response() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let mut aggregator = OpenAIStreamAggregator::default();

        aggregator
            .process_event(
                r#"{"choices":[{"index":0,"delta":{"content":"Hel"}}]}"#,
                &sender,
            )
            .expect("first chunk");
        aggregator
            .process_event(
                r#"{"choices":[{"index":0,"delta":{"content":"lo"},"finish_reason":"stop"}]}"#,
                &sender,
            )
            .expect("second chunk");
        aggregator
            .process_event("[DONE]", &sender)
            .expect("finalize");

        let first = receiver.recv().await.expect("first event").unwrap();
        assert_eq!(first.delta.as_deref(), Some("Hel"));

        let second = receiver.recv().await.expect("second event").unwrap();
        assert_eq!(second.delta.as_deref(), Some("lo"));

        let final_chunk = receiver.recv().await.expect("final response").unwrap();
        let response = final_chunk.response.expect("response");
        assert_eq!(response.content.as_deref(), Some("Hello"));
        assert!(matches!(response.finish_reason, FinishReason::Stop));
    }
}

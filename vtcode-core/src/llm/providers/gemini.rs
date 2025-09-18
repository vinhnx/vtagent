use crate::config::constants::models;
use crate::llm::client::LLMClient;
use crate::llm::error_display;
use crate::llm::provider::{
    FinishReason, FunctionCall, LLMError, LLMProvider, LLMRequest, LLMResponse, Message,
    MessageRole, ToolCall, StreamToken,
};
use crate::llm::types as llm_types;
use crate::gemini::{Client as GeminiClient};
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub struct GeminiProvider {
    gemini_client: GeminiClient,
    api_key: String,
    http_client: HttpClient,
    base_url: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_model(api_key, models::GEMINI_2_5_FLASH.to_string())
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            gemini_client: GeminiClient::new(api_key.clone(), model.clone()),
            api_key,
            http_client: HttpClient::new(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            model,
        }
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let gemini_request = self.convert_to_gemini_format(&request)?;

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, request.model, self.api_key
        );

        let response = self
            .http_client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| {
                let formatted_error =
                    error_display::format_llm_error("Gemini", &format!("Network error: {}", e));
                LLMError::Network(formatted_error)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let formatted_error = error_display::format_llm_error(
                "Gemini",
                &format!("HTTP {}: {}", status, error_text),
            );
            return Err(LLMError::Provider(formatted_error));
        }

        let gemini_response: Value = response.json().await.map_err(|e| {
            let formatted_error = error_display::format_llm_error(
                "Gemini",
                &format!("Failed to parse response: {}", e),
            );
            LLMError::Provider(formatted_error)
        })?;

        self.convert_from_gemini_format(gemini_response)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            models::google::GEMINI_2_5_FLASH.to_string(),
            models::google::GEMINI_2_5_FLASH_LITE.to_string(),
            models::google::GEMINI_2_5_PRO.to_string(),
        ]
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if !self.supported_models().contains(&request.model) {
            let formatted_error = error_display::format_llm_error(
                "Gemini",
                &format!("Unsupported model: {}", request.model),
            );
            return Err(LLMError::InvalidRequest(formatted_error));
        }
        Ok(())
    }

    async fn stream_tokens(
        &self,
        request: LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<crate::llm::provider::StreamToken, LLMError>> + Unpin + Send>, LLMError> {
        let gemini_request_value = self.convert_to_gemini_format(&request)?;

        // Deserialize the Value back to GenerateContentRequest
        let gemini_request: crate::gemini::GenerateContentRequest = serde_json::from_value(gemini_request_value)
            .map_err(|e| LLMError::Provider(format!("Failed to deserialize request: {}", e)))?;

        // Create our own streaming implementation to avoid lifetime issues
        let (tx, rx) = mpsc::channel(100);
        let http_client = self.http_client.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let gemini_request_clone = gemini_request.clone();

        tokio::spawn(async move {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
                model, api_key
            );

            match serde_json::to_value(&gemini_request_clone) {
                Ok(request_json) => {
                    let result = Self::fetch_streaming_tokens_static(http_client, url, request_json, tx).await;
                    if let Err(e) = result {
                        eprintln!("Streaming tokens error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to serialize request: {}", e);
                }
            }
        });

        let stream = ReceiverStream::new(rx).map(|result| {
            result.map(|token| crate::llm::provider::StreamToken {
                text: token.text,
                is_final: token.is_final,
                finish_reason: token.finish_reason,
            }).map_err(|e| LLMError::Provider(format!("Streaming error: {}", e)))
        });

        Ok(Box::new(stream))
    }
}

impl GeminiProvider {
    fn convert_to_gemini_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut contents = Vec::new();

        // Map tool_call_id to function name from previous assistant messages
        use std::collections::HashMap;
        let mut call_map: HashMap<String, String> = HashMap::new();
        for message in &request.messages {
            if message.role == MessageRole::Assistant {
                if let Some(tool_calls) = &message.tool_calls {
                    for call in tool_calls {
                        call_map.insert(call.id.clone(), call.function.name.clone());
                    }
                }
            }
        }

        for message in &request.messages {
            // Skip system messages - they should be handled as systemInstruction
            if message.role == MessageRole::System {
                continue;
            }

            let role = message.role.as_gemini_str();
            let mut parts = Vec::new();

            // Add text content if present
            if message.role != MessageRole::Tool && !message.content.is_empty() {
                parts.push(json!({"text": message.content}));
            }

            // Add function calls for assistant messages
            // Based on Gemini docs: function calls are in assistant/model messages
            if message.role == MessageRole::Assistant {
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        // Parse the arguments string to JSON object for Gemini
                        let args: Value = serde_json::from_str(&tool_call.function.arguments)
                            .unwrap_or(json!({}));
                        parts.push(json!({
                            "functionCall": {
                                "name": tool_call.function.name,
                                "args": args
                            }
                        }));
                    }
                }
            }

            // Add function response for tool messages
            // Based on Gemini docs: tool responses become functionResponse parts in user messages
            if message.role == MessageRole::Tool {
                // For tool responses, we need to construct a functionResponse
                // The tool_call_id should help us match this to the original function call
                if let Some(tool_call_id) = &message.tool_call_id {
                    let func_name = call_map
                        .get(tool_call_id)
                        .cloned()
                        .unwrap_or_else(|| tool_call_id.clone());

                    let response_text = serde_json::from_str::<Value>(&message.content)
                        .map(|value| {
                            serde_json::to_string_pretty(&value)
                                .unwrap_or_else(|_| message.content.clone())
                        })
                        .unwrap_or_else(|_| message.content.clone());

                    parts.push(json!({
                        "functionResponse": {
                            "name": func_name.clone(),
                            "response": {
                                "name": func_name,
                                "content": [
                                    {"text": response_text}
                                ]
                            }
                        }
                    }));
                } else {
                    // Fallback: if no tool_call_id, treat as regular text
                    // This shouldn't happen in well-formed tool calling flows
                    parts.push(json!({"text": message.content}));
                }
            }

            // Only add the content if we have parts
            if !parts.is_empty() {
                contents.push(json!({
                    "role": role,
                    "parts": parts
                }));
            }
        }

        let mut gemini_request = json!({
            "contents": contents
        });

        if let Some(system) = &request.system_prompt {
            gemini_request["systemInstruction"] = json!({
                "parts": [{"text": system}]
            });
        }

        // Add tools if present
        if let Some(tools) = &request.tools {
            let gemini_tools: Vec<Value> = tools
                .iter()
                .map(|tool| {
                    json!({
                        "function_declarations": [
                            {
                                "name": tool.function.name,
                                "description": tool.function.description,
                                "parameters": tool.function.parameters
                            }
                        ]
                    })
                })
                .collect();
            gemini_request["tools"] = json!(gemini_tools);
        }

        Ok(gemini_request)
    }

    fn convert_from_gemini_format(&self, response: Value) -> Result<LLMResponse, LLMError> {
        let candidates = response["candidates"].as_array().ok_or_else(|| {
            let formatted_error =
                error_display::format_llm_error("Gemini", "No candidates in response");
            LLMError::Provider(formatted_error)
        })?;

        let candidate = candidates.first().ok_or_else(|| {
            let formatted_error =
                error_display::format_llm_error("Gemini", "No candidate in response");
            LLMError::Provider(formatted_error)
        })?;

        // Check if content exists and has parts
        if let Some(content) = candidate.get("content") {
            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                if parts.is_empty() {
                    return Ok(LLMResponse {
                        content: Some("".to_string()),
                        tool_calls: None,
                        usage: None,
                        finish_reason: FinishReason::Stop,
                    });
                }

                let mut text_content = String::new();
                let mut tool_calls = Vec::new();

                // Parse parts for text and function calls
                for part in parts {
                    if let Some(text) = part["text"].as_str() {
                        text_content.push_str(text);
                    } else if let Some(function_call) = part["functionCall"]
                        .as_object()
                        .or_else(|| part["function_call"].as_object())
                    {
                        let name = function_call["name"].as_str().unwrap_or("").to_string();
                        let args = function_call["args"].clone();
                        // Use timestamp-based unique IDs to avoid conflicts
                        let call_id = format!(
                            "call_{}_{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos(),
                            tool_calls.len()
                        );
                        tool_calls.push(ToolCall {
                            id: call_id,
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name,
                                arguments: serde_json::to_string(&args).unwrap_or_default(),
                            },
                        });
                    }
                }

                let finish_reason = match candidate["finishReason"].as_str() {
                    Some("STOP") => FinishReason::Stop,
                    Some("MAX_TOKENS") => FinishReason::Length,
                    Some("SAFETY") => FinishReason::ContentFilter,
                    Some("FUNCTION_CALL") => FinishReason::ToolCalls,
                    Some(other) => FinishReason::Error(other.to_string()),
                    None => FinishReason::Stop,
                };

                return Ok(LLMResponse {
                    content: if text_content.is_empty() {
                        None
                    } else {
                        Some(text_content)
                    },
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                    usage: None, // Gemini doesn't provide usage in basic response
                    finish_reason,
                });
            } else {
                // Content exists but no parts array - return empty response
                return Ok(LLMResponse {
                    content: Some("".to_string()),
                    tool_calls: None,
                    usage: None,
                    finish_reason: FinishReason::Stop,
                });
            }
        } else {
            // No content in candidate - return empty response
            return Ok(LLMResponse {
                content: Some("".to_string()),
                tool_calls: None,
                usage: None,
                finish_reason: FinishReason::Stop,
            });
        }
    }
}

#[async_trait]
impl LLMClient for GeminiProvider {
    async fn generate(&mut self, prompt: &str) -> Result<llm_types::LLMResponse, LLMError> {
        // Check if the prompt is a serialized GenerateContentRequest
        let request = if prompt.starts_with('{') && prompt.contains("\"contents\"") {
            // Try to parse as JSON GenerateContentRequest
            match serde_json::from_str::<crate::gemini::GenerateContentRequest>(prompt) {
                Ok(gemini_request) => {
                    // Convert GenerateContentRequest to LLMRequest
                    let mut messages = Vec::new();
                    let mut system_prompt = None;

                    // Convert contents to messages
                    for content in &gemini_request.contents {
                        let role = match content.role.as_str() {
                            crate::config::constants::message_roles::USER => MessageRole::User,
                            "model" => MessageRole::Assistant,
                            crate::config::constants::message_roles::SYSTEM => {
                                // Extract system message
                                let text = content
                                    .parts
                                    .iter()
                                    .filter_map(|part| part.as_text())
                                    .collect::<Vec<_>>()
                                    .join("");
                                system_prompt = Some(text);
                                continue;
                            }
                            _ => MessageRole::User, // Default to user
                        };

                        let content_text = content
                            .parts
                            .iter()
                            .filter_map(|part| part.as_text())
                            .collect::<Vec<_>>()
                            .join("");

                        messages.push(Message {
                            role,
                            content: content_text,
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }

                    // Convert tools if present
                    let tools = gemini_request.tools.as_ref().map(|gemini_tools| {
                        gemini_tools
                            .iter()
                            .flat_map(|tool| &tool.function_declarations)
                            .map(|decl| crate::llm::provider::ToolDefinition {
                                tool_type: "function".to_string(),
                                function: crate::llm::provider::FunctionDefinition {
                                    name: decl.name.clone(),
                                    description: decl.description.clone(),
                                    parameters: decl.parameters.clone(),
                                },
                            })
                            .collect::<Vec<_>>()
                    });

                    let llm_request = LLMRequest {
                        messages,
                        system_prompt,
                        tools,
                        model: self.model.clone(),
                        max_tokens: gemini_request
                            .generation_config
                            .as_ref()
                            .and_then(|config| config.get("maxOutputTokens"))
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        temperature: gemini_request
                            .generation_config
                            .as_ref()
                            .and_then(|config| config.get("temperature"))
                            .and_then(|v| v.as_f64())
                            .map(|v| v as f32),
                        stream: false,
                        tool_choice: None,
                        parallel_tool_calls: None,
                        parallel_tool_config: None,
                        reasoning_effort: None,
                    };

                    // Use the standard LLMProvider generate method
                    let response = LLMProvider::generate(self, llm_request).await?;

                    // If there are tool calls, include them in the response content as JSON
                    let content = if let Some(tool_calls) = &response.tool_calls {
                        if !tool_calls.is_empty() {
                            // Create a JSON structure that the agent can parse
                            let tool_call_json = json!({
                                "tool_calls": tool_calls.iter().map(|tc| {
                                    json!({
                                        "function": {
                                            "name": tc.function.name,
                                            "arguments": tc.function.arguments
                                        }
                                    })
                                }).collect::<Vec<_>>()
                            });
                            tool_call_json.to_string()
                        } else {
                            response.content.unwrap_or("".to_string())
                        }
                    } else {
                        response.content.unwrap_or("".to_string())
                    };

                    return Ok(llm_types::LLMResponse {
                        content,
                        model: self.model.clone(),
                        usage: response.usage.map(|u| llm_types::Usage {
                            prompt_tokens: u.prompt_tokens as usize,
                            completion_tokens: u.completion_tokens as usize,
                            total_tokens: u.total_tokens as usize,
                        }),
                    });
                }
                Err(_) => {
                    // Fallback: treat as regular prompt
                    LLMRequest {
                        messages: vec![Message {
                            role: MessageRole::User,
                            content: prompt.to_string(),
                            tool_calls: None,
                            tool_call_id: None,
                        }],
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
            }
        } else {
            // Fallback: treat as regular prompt
            LLMRequest {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: prompt.to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }],
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
        };

        let response = LLMProvider::generate(self, request).await?;

        Ok(llm_types::LLMResponse {
            content: response.content.unwrap_or("".to_string()),
            model: models::GEMINI_2_5_FLASH.to_string(),
            usage: response.usage.map(|u| llm_types::Usage {
                prompt_tokens: u.prompt_tokens as usize,
                completion_tokens: u.completion_tokens as usize,
                total_tokens: u.total_tokens as usize,
            }),
        })
    }

    fn backend_kind(&self) -> llm_types::BackendKind {
        llm_types::BackendKind::Gemini
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}

impl GeminiProvider {
    async fn fetch_streaming_tokens_static(
        http_client: HttpClient,
        url: String,
        request: Value,
        tx: mpsc::Sender<Result<StreamToken, Box<dyn std::error::Error + Send>>>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let response = http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("API Error {}: {}", status, error_text),
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut token_count = 0;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.extend_from_slice(&bytes);

                    // Process complete JSON objects from buffer
                    while let Some(end_pos) = Self::find_json_boundary_static(&buffer) {
                        let json_bytes = buffer.drain(..end_pos).collect::<Vec<_>>();

                        // Skip empty lines or non-JSON content
                        let json_str = String::from_utf8_lossy(&json_bytes).to_string();
                        if json_str.trim().is_empty() || !json_str.trim_start().starts_with('{') {
                            continue;
                        }

                        match serde_json::from_str::<Value>(&json_str) {
                            Ok(json_value) => {
                                // Extract candidates
                                if let Some(candidates) = json_value.get("candidates").and_then(|c| c.as_array()) {
                                    for candidate in candidates {
                                        if let Some(content) = candidate.get("content") {
                                            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                                for part in parts {
                                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                        let is_final = candidate.get("finishReason").is_some();
                                                        let finish_reason = candidate
                                                            .get("finishReason")
                                                            .and_then(|fr| fr.as_str())
                                                            .map(|s| s.to_string());

                                                        let token = StreamToken {
                                                            text: text.to_string(),
                                                            is_final,
                                                            finish_reason,
                                                        };

                                                        token_count += 1;
                                                        if tx.send(Ok(token)).await.is_err() {
                                                            return Ok(());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse JSON: {} - Content: {}", e, json_str);
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Box::new(e))).await;
                    break;
                }
            }
        }

        // Send final token if we haven't already
        let _ = tx.send(Ok(StreamToken {
            text: String::new(),
            is_final: true,
            finish_reason: Some("STOP".to_string()),
        })).await;

        Ok(())
    }

    fn find_json_boundary_static(buffer: &[u8]) -> Option<usize> {
        let s = String::from_utf8_lossy(buffer);
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut start_found = false;

        for (i, c) in s.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '"' if !escape_next => in_string = !in_string,
                '\\' if in_string => escape_next = true,
                '{' if !in_string => {
                    brace_count += 1;
                    start_found = true;
                }
                '}' if !in_string => {
                    brace_count -= 1;
                    if start_found && brace_count == 0 {
                        return Some(i + c.len_utf8());
                    }
                }
                '\n' if !start_found => {
                    // Skip to next line if we haven't found a JSON start
                    return Some(i + c.len_utf8());
                }
                _ => {}
            }
        }
        None
    }
}

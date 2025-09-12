use crate::config::constants::models;
use crate::llm::client::LLMClient;
use crate::llm::provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
};
use crate::llm::types as llm_types;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};

pub struct GeminiProvider {
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
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let gemini_response: Value = response
            .json()
            .await
            .map_err(|e| LLMError::Provider(e.to_string()))?;

        self.convert_from_gemini_format(gemini_response)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            models::GEMINI_2_5_FLASH.to_string(),
            models::GEMINI_2_5_FLASH_LITE.to_string(),
            models::GEMINI_2_5_PRO.to_string(),
        ]
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if !self.supported_models().contains(&request.model) {
            return Err(LLMError::InvalidRequest(format!(
                "Unsupported model: {}",
                request.model
            )));
        }
        Ok(())
    }
}

impl GeminiProvider {
    fn convert_to_gemini_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut contents = Vec::new();

        for message in &request.messages {
            // Skip system messages - they should be handled as systemInstruction
            if message.role == MessageRole::System {
                continue;
            }

            let role = message.role.as_gemini_str();
            let mut parts = Vec::new();

            // Add text content if present
            if !message.content.is_empty() {
                parts.push(json!({"text": message.content}));
            }

            // Add function calls for assistant messages
            // Based on Gemini docs: function calls are in assistant/model messages
            if message.role == MessageRole::Assistant {
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        parts.push(json!({
                            "functionCall": {
                                "name": tool_call.name,
                                "args": tool_call.arguments
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
                    // We need to extract the function name from the tool_call_id or content
                    // For now, we'll try to parse it from the context or use a generic approach
                    parts.push(json!({
                        "functionResponse": {
                            "name": tool_call_id, // This should be the function name
                            "response": {
                                "content": message.content
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
                        "functionDeclarations": [{
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters
                        }]
                    })
                })
                .collect();
            gemini_request["tools"] = json!(gemini_tools);
        }

        Ok(gemini_request)
    }

    fn convert_from_gemini_format(&self, response: Value) -> Result<LLMResponse, LLMError> {
        let candidates = response["candidates"].as_array().ok_or_else(|| {
            LLMError::Provider("No candidates in response".to_string())
        })?;

        let candidate = candidates.first().ok_or_else(|| {
            LLMError::Provider("No candidate in response".to_string())
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
                    } else if let Some(function_call) = part["functionCall"].as_object() {
                        let name = function_call["name"].as_str().unwrap_or("").to_string();
                        let args = function_call["args"].clone();
                        tool_calls.push(ToolCall {
                            id: format!("call_{}", tool_calls.len()), // Gemini doesn't provide IDs
                            name,
                            arguments: args,
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
        let request = if prompt.trim().starts_with("{") {
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
                                name: decl.name.clone(),
                                description: decl.description.clone(),
                                parameters: decl.parameters.clone(),
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
                                            "name": tc.name,
                                            "arguments": tc.arguments
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
                    }
                }
            }
        } else {
            // Regular prompt
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

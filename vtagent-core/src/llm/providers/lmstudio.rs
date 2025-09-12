use crate::config::constants::{model_helpers, models, urls};
use crate::llm::client::LLMClient;
use crate::llm::provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
};
use crate::llm::types as llm_types;
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};

pub struct LMStudioProvider {
    api_key: Option<String>, // LMStudio typically doesn't require API key for local models
    http_client: HttpClient,
    base_url: String,
}

impl LMStudioProvider {
    pub fn new(api_key: Option<String>, base_url: Option<String>) -> Self {
        Self {
            api_key,
            http_client: HttpClient::new(),
            base_url: base_url.unwrap_or_else(|| urls::LMSTUDIO_DEFAULT_BASE_URL.to_string()),
        }
    }

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
            // LMStudio typically follows OpenAI conventions
            let role = msg.role.as_generic_str();

            let mut message = json!({
                "role": role,
                "content": msg.content
            });

            // Add tool call information for assistant messages
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
            if msg.role == MessageRole::Tool {
                if let Some(tool_call_id) = &msg.tool_call_id {
                    message["tool_call_id"] = Value::String(tool_call_id.clone());
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

        // Add tools if present (but skip for LMStudio as it may not support them)
        if let Some(tools) = &request.tools {
            if !tools.is_empty()
                && !request.model.contains("lmstudio")
                && !request.model.contains("qwen")
            {
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
                            function: FunctionCall {
                                name: call.get("function")?.get("name")?.as_str()?.to_string(),
                                arguments: call
                                    .get("function")
                                    .and_then(|f| f.get("arguments"))
                                    .map(|args| serde_json::to_string(args).unwrap_or_default())
                                    .unwrap_or_default(),
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
impl LLMProvider for LMStudioProvider {
    fn name(&self) -> &str {
        "lmstudio"
    }

    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let openai_request = self.convert_to_openai_format(&request)?;

        let url = format!("{}/chat/completions", self.base_url);

        let mut request_builder = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json");

        // Add authorization header if API key is provided
        if let Some(api_key) = &self.api_key {
            request_builder =
                request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read response body".to_string());

            // For local LMStudio, if we get 403 Forbidden, try without authentication
            if status == reqwest::StatusCode::FORBIDDEN && self.api_key.is_none() {
                eprintln!(
                    "Warning: LMStudio server requires authentication but no API key provided. This might indicate LMStudio has authentication enabled."
                );
                eprintln!("To fix this:");
                eprintln!("1. In LMStudio, go to 'Local Inference' -> 'Settings'");
                eprintln!("2. Disable 'Enable API Key' or set an API key");
                eprintln!("3. Or set LMSTUDIO_API_KEY environment variable");
                return Err(LLMError::Provider(format!(
                    "HTTP {}: {}. LMStudio authentication required but not configured.",
                    status, error_text
                )));
            }

            return Err(LLMError::Provider(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| LLMError::Provider(format!("Failed to parse response: {}", e)))?;

        self.parse_openai_response(response_json)
    }

    fn supported_models(&self) -> Vec<String> {
        models::lmstudio::SUPPORTED_MODELS
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

#[async_trait]
impl LLMClient for LMStudioProvider {
    async fn generate(&mut self, prompt: &str) -> Result<llm_types::LLMResponse, LLMError> {
        // Check if the prompt is a serialized GenerateContentRequest
        let request = if prompt.trim().starts_with("GenerateContentRequest") {
            // Try to parse as GenerateContentRequest
            match serde_json::from_str::<crate::gemini::GenerateContentRequest>(prompt) {
                Ok(gemini_request) => {
                    // Convert Gemini format to LLMRequest
                    let messages: Vec<Message> = gemini_request
                        .contents
                        .into_iter()
                        .map(|content| {
                            let role = match content.role.as_str() {
                                crate::config::constants::message_roles::USER => MessageRole::User,
                                "model" => MessageRole::Assistant,
                                crate::config::constants::message_roles::SYSTEM => MessageRole::System,
                                _ => MessageRole::User,
                            };

                            let content_text = content
                                .parts
                                .iter()
                                .filter_map(|part| match part {
                                    crate::gemini::Part::Text { text } => Some(text.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n");

                            Message {
                                role,
                                content: content_text,
                                tool_calls: None,
                                tool_call_id: None,
                            }
                        })
                        .collect();

                    let system_prompt = gemini_request.system_instruction.as_ref().map(|si| {
                        si.parts
                            .iter()
                            .filter_map(|part| match part {
                                crate::gemini::Part::Text { text } => Some(text.clone()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    });

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

                    LLMRequest {
                        messages,
                        system_prompt,
                        tools,
                        model: models::lmstudio::DEFAULT_MODEL.to_string(),
                        max_tokens: Some(1000),
                        temperature: Some(0.7),
                        stream: false,
                        tool_choice: None,
                        parallel_tool_calls: None,
                        reasoning_effort: None,
                    }
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
                        model: models::lmstudio::DEFAULT_MODEL.to_string(),
                        max_tokens: None,
                        temperature: None,
                        stream: false,
                        tool_choice: None,
                        parallel_tool_calls: None,
                        reasoning_effort: None,
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
                model: models::lmstudio::DEFAULT_MODEL.to_string(),
                max_tokens: None,
                temperature: None,
                stream: false,
                tool_choice: None,
                parallel_tool_calls: None,
                reasoning_effort: None,
            }
        };

        let response = LLMProvider::generate(self, request).await?;

        let model = models::lmstudio::DEFAULT_MODEL.to_string();

        // Validate the model
        if !model_helpers::is_valid("lmstudio", &model) {
            return Err(LLMError::InvalidRequest(format!(
                "Invalid LMStudio model '{}'. See docs/models.json",
                model
            )));
        }

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
        llm_types::BackendKind::LMStudio
    }

    fn model_id(&self) -> &str {
        models::lmstudio::DEFAULT_MODEL
    }
}

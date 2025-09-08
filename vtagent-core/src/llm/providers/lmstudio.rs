use crate::llm::provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
    ToolDefinition, Usage,
};
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
            base_url: base_url.unwrap_or_else(|| "http://localhost:1234/v1".to_string()),
        }
    }

    fn convert_to_openai_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut messages = Vec::new();

        // Add system message if present
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system_prompt
            }));
        }

        // Convert messages
        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };

            let mut message = json!({
                "role": role,
                "content": msg.content
            });

            // Add tool call information if present
            if let Some(tool_calls) = &msg.tool_calls {
                if !tool_calls.is_empty() {
                    let tool_calls_json: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments
                                }
                            })
                        })
                        .collect();
                    message["tool_calls"] = Value::Array(tool_calls_json);
                }
            }

            if let Some(tool_call_id) = &msg.tool_call_id {
                message["tool_call_id"] = Value::String(tool_call_id.clone());
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
                                "name": tool.name,
                                "description": tool.description,
                                "parameters": tool.parameters
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
            .ok_or_else(|| LLMError::Provider("Invalid response format: missing choices".to_string()))?;

        if choices.is_empty() {
            return Err(LLMError::Provider("No choices in response".to_string()));
        }

        let choice = &choices[0];
        let message = choice
            .get("message")
            .ok_or_else(|| LLMError::Provider("Invalid response format: missing message".to_string()))?;

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
                            name: call
                                .get("function")?
                                .get("name")?
                                .as_str()?
                                .to_string(),
                            arguments: call
                                .get("function")?
                                .get("arguments")?
                                .clone()
                                .unwrap_or_else(|| Value::Null),
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
            .map(|u| Usage {
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
            request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request_builder
            .json(&openai_request)
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

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| LLMError::Provider(format!("Failed to parse response: {}", e)))?;

        self.parse_openai_response(response_json)
    }

    fn supported_models(&self) -> Vec<String> {
        // LMStudio supports any model loaded locally
        vec![
            "local-model".to_string(), // Generic placeholder
            "llama-2-7b-chat".to_string(),
            "llama-2-13b-chat".to_string(),
            "llama-2-70b-chat".to_string(),
            "codellama-7b-instruct".to_string(),
            "codellama-13b-instruct".to_string(),
            "codellama-34b-instruct".to_string(),
            "mistral-7b-instruct".to_string(),
            "mixtral-8x7b-instruct".to_string(),
            "phi-2".to_string(),
            "orca-2-7b".to_string(),
            "vicuna-7b".to_string(),
            "wizardlm-7b".to_string(),
        ]
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if request.messages.is_empty() {
            return Err(LLMError::InvalidRequest("Messages cannot be empty".to_string()));
        }

        if request.model.is_empty() {
            return Err(LLMError::InvalidRequest("Model cannot be empty".to_string()));
        }

        Ok(())
    }
}

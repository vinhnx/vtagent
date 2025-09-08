use crate::llm::provider::{
    FinishReason, LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole, ToolCall,
    ToolDefinition, Usage,
};
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{Value, json};

pub struct OllamaProvider {
    http_client: HttpClient,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            http_client: HttpClient::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434/api".to_string()),
        }
    }

    fn convert_to_ollama_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
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

        let mut ollama_request = json!({
            "model": request.model,
            "messages": messages,
            "stream": request.stream
        });

        // Add optional parameters
        if let Some(max_tokens) = request.max_tokens {
            ollama_request["max_tokens"] = json!(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            ollama_request["temperature"] = json!(temperature);
        }

        // Add tools if present (Ollama supports tools)
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
                ollama_request["tools"] = Value::Array(tools_json);
            }
        }

        Ok(ollama_request)
    }

    fn parse_ollama_response(&self, response_json: Value) -> Result<LLMResponse, LLMError> {
        // Ollama response format is slightly different from OpenAI
        let message = response_json
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
        let finish_reason = response_json
            .get("done_reason")
            .and_then(|dr| dr.as_str())
            .map(|dr| match dr {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCalls,
                _ => FinishReason::Error(dr.to_string()),
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
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let ollama_request = self.convert_to_ollama_format(&request)?;

        let url = format!("{}/chat", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&ollama_request)
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

        self.parse_ollama_response(response_json)
    }

    fn supported_models(&self) -> Vec<String> {
        // Ollama supports any model pulled locally
        vec![
            "llama2".to_string(),
            "llama2:13b".to_string(),
            "llama2:70b".to_string(),
            "codellama".to_string(),
            "codellama:13b".to_string(),
            "codellama:34b".to_string(),
            "mistral".to_string(),
            "mixtral".to_string(),
            "phi".to_string(),
            "neural-chat".to_string(),
            "starling-lm".to_string(),
            "orca-mini".to_string(),
            "vicuna".to_string(),
            "llava".to_string(),
            "bakllava".to_string(),
            "wizardlm2".to_string(),
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

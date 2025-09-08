use super::super::provider::{LLMProvider, LLMRequest, LLMResponse, LLMError, Message, MessageRole, ToolCall, Usage, FinishReason};
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};

pub struct GeminiProvider {
    api_key: String,
    http_client: HttpClient,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: HttpClient::new(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
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

        let response = self.http_client
            .post(&url)
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!("HTTP {}: {}", status, error_text)));
        }

        let gemini_response: Value = response.json().await
            .map_err(|e| LLMError::Provider(e.to_string()))?;

        self.convert_from_gemini_format(gemini_response)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gemini-2.5-flash".to_string(),
            "gemini-2.5-flash-lite".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ]
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if !self.supported_models().contains(&request.model) {
            return Err(LLMError::InvalidRequest(format!("Unsupported model: {}", request.model)));
        }
        Ok(())
    }
}

impl GeminiProvider {
    fn convert_to_gemini_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut contents = Vec::new();
        
        for message in &request.messages {
            let role = match message.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "model",
                MessageRole::System => continue,
                MessageRole::Tool => "function",
            };
            
            let mut parts = Vec::new();
            
            // Add text content if present
            if !message.content.is_empty() {
                parts.push(json!({"text": message.content}));
            }
            
            // Add function calls for assistant messages
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
            if message.role == MessageRole::Tool {
                if let Some(tool_calls) = &message.tool_calls {
                    for tool_call in tool_calls {
                        parts.push(json!({
                            "functionResponse": {
                                "name": tool_call.name,
                                "response": {
                                    "content": message.content
                                }
                            }
                        }));
                    }
                }
            }
            
            contents.push(json!({
                "role": role,
                "parts": parts
            }));
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
            let gemini_tools: Vec<Value> = tools.iter().map(|tool| {
                json!({
                    "functionDeclarations": [{
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }]
                })
            }).collect();
            gemini_request["tools"] = json!(gemini_tools);
        }

        Ok(gemini_request)
    }

    fn convert_from_gemini_format(&self, response: Value) -> Result<LLMResponse, LLMError> {
        let candidates = response["candidates"].as_array()
            .ok_or_else(|| LLMError::Provider("No candidates in response".to_string()))?;

        let candidate = candidates.first()
            .ok_or_else(|| LLMError::Provider("No candidate in response".to_string()))?;

        let parts = candidate["content"]["parts"].as_array()
            .ok_or_else(|| LLMError::Provider("No parts in response".to_string()))?;

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

        Ok(LLMResponse {
            content: if text_content.is_empty() { None } else { Some(text_content) },
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            usage: None, // Gemini doesn't provide usage in basic response
            finish_reason,
        })
    }
}
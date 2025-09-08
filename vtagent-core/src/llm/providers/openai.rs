use super::super::provider::{LLMProvider, LLMRequest, LLMResponse, LLMError, Message, MessageRole, ToolCall, Usage, FinishReason};
use async_trait::async_trait;
use reqwest::Client as HttpClient;
use serde_json::{json, Value};

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

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| LLMError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Provider(format!("HTTP {}: {}", status, error_text)));
        }

        let openai_response: Value = response.json().await
            .map_err(|e| LLMError::Provider(e.to_string()))?;

        self.convert_from_openai_format(openai_response)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
        ]
    }

    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError> {
        if !self.supported_models().contains(&request.model) {
            return Err(LLMError::InvalidRequest(format!("Unsupported model: {}", request.model)));
        }
        Ok(())
    }
}

impl OpenAIProvider {
    fn convert_to_openai_format(&self, request: &LLMRequest) -> Result<Value, LLMError> {
        let mut messages = Vec::new();
        
        // Add system message if present
        if let Some(system) = &request.system_prompt {
            messages.push(json!({
                "role": "system",
                "content": system
            }));
        }
        
        for message in &request.messages {
            let role = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user", 
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            
            let mut msg = json!({
                "role": role,
                "content": message.content
            });
            
            // Add tool_call_id for tool messages
            if message.role == MessageRole::Tool {
                if let Some(tool_call_id) = &message.tool_call_id {
                    msg["tool_call_id"] = json!(tool_call_id);
                }
            }
            
            // Add tool_calls for assistant messages
            if let Some(tool_calls) = &message.tool_calls {
                msg["tool_calls"] = json!(tool_calls.iter().map(|tc| json!({
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments.to_string()
                    }
                })).collect::<Vec<_>>());
            }
            
            messages.push(msg);
        }

        let mut openai_request = json!({
            "model": request.model,
            "messages": messages
        });

        if let Some(max_tokens) = request.max_tokens {
            openai_request["max_tokens"] = json!(max_tokens);
        }
        
        if let Some(temperature) = request.temperature {
            openai_request["temperature"] = json!(temperature);
        }

        if let Some(tools) = &request.tools {
            let openai_tools: Vec<Value> = tools.iter().map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                })
            }).collect();
            openai_request["tools"] = json!(openai_tools);
        }

        Ok(openai_request)
    }

    fn convert_from_openai_format(&self, response: Value) -> Result<LLMResponse, LLMError> {
        let choices = response["choices"].as_array()
            .ok_or_else(|| LLMError::Provider("No choices in response".to_string()))?;

        let choice = choices.first()
            .ok_or_else(|| LLMError::Provider("No choice in response".to_string()))?;

        let message = &choice["message"];
        let content = message["content"].as_str().map(|s| s.to_string());

        let tool_calls = message["tool_calls"].as_array().map(|calls| {
            calls.iter().filter_map(|call| {
                Some(ToolCall {
                    id: call["id"].as_str()?.to_string(),
                    name: call["function"]["name"].as_str()?.to_string(),
                    arguments: serde_json::from_str(call["function"]["arguments"].as_str()?).ok()?,
                })
            }).collect()
        });

        let usage = response["usage"].as_object().map(|u| Usage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        let finish_reason = match choice["finish_reason"].as_str() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            Some(other) => FinishReason::Error(other.to_string()),
            None => FinishReason::Stop,
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage,
            finish_reason,
        })
    }
}

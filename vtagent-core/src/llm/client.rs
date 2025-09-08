use super::factory::create_provider_for_model;
use super::provider::{LLMError, LLMProvider, LLMRequest, LLMResponse, Message, MessageRole};

/// Unified LLM client that works with any provider
pub struct UnifiedLLMClient {
    provider: Box<dyn LLMProvider>,
    model: String,
}

impl UnifiedLLMClient {
    /// Create client from model name and API key
    pub fn new(model: String, api_key: String) -> Result<Self, LLMError> {
        let provider = create_provider_for_model(&model, api_key)?;

        Ok(Self { provider, model })
    }

    /// Generate completion
    pub async fn generate(
        &self,
        messages: Vec<Message>,
        system_prompt: Option<String>,
    ) -> Result<LLMResponse, LLMError> {
        let request = LLMRequest {
            messages,
            system_prompt,
            tools: None,
            model: self.model.clone(),
            max_tokens: None,
            temperature: None,
            stream: false,
        };

        self.provider.validate_request(&request)?;
        self.provider.generate(request).await
    }

    /// Generate with tools
    pub async fn generate_with_tools(
        &self,
        messages: Vec<Message>,
        system_prompt: Option<String>,
        tools: Vec<super::provider::ToolDefinition>,
    ) -> Result<LLMResponse, LLMError> {
        let request = LLMRequest {
            messages,
            system_prompt,
            tools: Some(tools),
            model: self.model.clone(),
            max_tokens: None,
            temperature: None,
            stream: false,
        };

        self.provider.validate_request(&request)?;
        self.provider.generate(request).await
    }

    /// Get provider name
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Get model name
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Convenience functions for creating messages
impl Message {
    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

//! Universal LLM provider abstraction with API-specific role handling
//!
//! This module provides a unified interface for different LLM providers (OpenAI, Anthropic, Gemini)
//! while properly handling their specific requirements for message roles and tool calling.
//!
//! ## Message Role Mapping
//!
//! Different LLM providers have varying support for message roles, especially for tool calling:
//!
//! ### OpenAI API
//! - **Full Support**: `system`, `user`, `assistant`, `tool`
//! - **Tool Messages**: Must include `tool_call_id` to reference the original tool call
//! - **Tool Calls**: Only `assistant` messages can contain `tool_calls`
//!
//! ### Anthropic API
//! - **Standard Roles**: `user`, `assistant`
//! - **System Messages**: Can be hoisted to system parameter or treated as user messages
//! - **Tool Responses**: Converted to `user` messages (no separate tool role)
//! - **Tool Choice**: Supports `auto`, `any`, `tool`, `none` modes
//!
//! ### Gemini API
//! - **Conversation Roles**: Only `user` and `model` (not `assistant`)
//! - **System Messages**: Handled separately as `systemInstruction` parameter
//! - **Tool Responses**: Converted to `user` messages with `functionResponse` format
//! - **Function Calls**: Uses `functionCall` in `model` messages
//!
//! ## Best Practices
//!
//! 1. Always use `MessageRole::tool_response()` constructor for tool responses
//! 2. Validate messages using `validate_for_provider()` before sending
//! 3. Use appropriate role mapping methods for each provider
//! 4. Handle provider-specific constraints (e.g., Gemini's system instruction requirement)
//!
//! ## Example Usage
//!
//! ```rust
//! use vtagent_core::llm::provider::{Message, MessageRole};
//!
//! // Create a proper tool response message
//! let tool_response = Message::tool_response(
//!     "call_123".to_string(),
//!     "Tool execution completed successfully".to_string()
//! );
//!
//! // Validate for specific provider
//! tool_response.validate_for_provider("openai").unwrap();
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Universal LLM request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub messages: Vec<Message>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,

    /// Tool choice configuration based on official API docs
    /// Supports: "auto" (default), "none", "any", or specific tool selection
    pub tool_choice: Option<ToolChoice>,

    /// Whether to enable parallel tool calls (OpenAI specific)
    pub parallel_tool_calls: Option<bool>,

    /// Reasoning effort level for models that support it (low, medium, high)
    /// Applies to: Claude, GPT-5, Gemini, Qwen3, DeepSeek with reasoning capability
    pub reasoning_effort: Option<String>,
}

/// Tool choice configuration that works across different providers
/// Based on OpenAI, Anthropic, and Gemini API specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Let the model decide whether to call tools ("auto")
    Auto,

    /// Force the model to not call any tools ("none")
    None,

    /// Force the model to call at least one tool ("any")
    Any,

    /// Force the model to call a specific tool
    Specific(SpecificToolChoice),
}

/// Specific tool choice for forcing a particular function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificToolChoice {
    #[serde(rename = "type")]
    pub tool_type: String, // "function"

    pub function: SpecificFunctionChoice,
}

/// Specific function choice details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificFunctionChoice {
    pub name: String,
}

impl ToolChoice {
    /// Create auto tool choice (default behavior)
    pub fn auto() -> Self {
        Self::Auto
    }

    /// Create none tool choice (disable tool calling)
    pub fn none() -> Self {
        Self::None
    }

    /// Create any tool choice (force at least one tool call)
    pub fn any() -> Self {
        Self::Any
    }

    /// Create specific function tool choice
    pub fn function(name: String) -> Self {
        Self::Specific(SpecificToolChoice {
            tool_type: "function".to_string(),
            function: SpecificFunctionChoice { name },
        })
    }

    /// Convert to provider-specific format
    pub fn to_provider_format(&self, provider: &str) -> Value {
        match (self, provider) {
            (Self::Auto, "openai") => json!("auto"),
            (Self::None, "openai") => json!("none"),
            (Self::Any, "openai") => json!("required"), // OpenAI uses "required" instead of "any"
            (Self::Specific(choice), "openai") => json!(choice),

            (Self::Auto, "anthropic") => json!({"type": "auto"}),
            (Self::None, "anthropic") => json!({"type": "none"}),
            (Self::Any, "anthropic") => json!({"type": "any"}),
            (Self::Specific(choice), "anthropic") => json!({"type": "tool", "name": choice.function.name}),

            (Self::Auto, "gemini") => json!({"mode": "auto"}),
            (Self::None, "gemini") => json!({"mode": "none"}),
            (Self::Any, "gemini") => json!({"mode": "any"}),
            (Self::Specific(choice), "gemini") => json!({"mode": "any", "allowed_function_names": [choice.function.name]}),

            // Generic/LMStudio follows OpenAI format
            _ => match self {
                Self::Auto => json!("auto"),
                Self::None => json!("none"),
                Self::Any => json!("required"),
                Self::Specific(choice) => json!(choice),
            }
        }
    }
}

/// Universal message structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a user message
    pub fn user(content: String) -> Self {
        Self {
            role: MessageRole::User,
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message with tool calls
    /// Based on OpenAI Cookbook patterns for function calling
    pub fn assistant_with_tools(content: String, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a system message
    pub fn system(content: String) -> Self {
        Self {
            role: MessageRole::System,
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a tool response message
    /// This follows the exact pattern from OpenAI Cookbook:
    /// ```json
    /// {
    ///   "role": "tool",
    ///   "tool_call_id": "call_123",
    ///   "content": "Function result"
    /// }
    /// ```
    pub fn tool_response(tool_call_id: String, content: String) -> Self {
        Self {
            role: MessageRole::Tool,
            content,
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        }
    }

    /// Create a tool response message with function name (for compatibility)
    /// Some providers might need the function name in addition to tool_call_id
    pub fn tool_response_with_name(tool_call_id: String, _function_name: String, content: String) -> Self {
        // We can store the function name in the content metadata or handle it provider-specifically
        Self::tool_response(tool_call_id, content)
    }

    /// Validate this message for a specific provider
    /// Based on official API documentation constraints
    pub fn validate_for_provider(&self, provider: &str) -> Result<(), String> {
        // Check role-specific constraints
        self.role.validate_for_provider(provider, self.tool_call_id.is_some())?;

        // Check tool call constraints
        if let Some(tool_calls) = &self.tool_calls {
            if !self.role.can_make_tool_calls() {
                return Err(format!("Role {:?} cannot make tool calls", self.role));
            }

            if tool_calls.is_empty() {
                return Err("Tool calls array should not be empty".to_string());
            }

            // Validate each tool call
            for tool_call in tool_calls {
                tool_call.validate()?;
            }
        }

        // Provider-specific validations based on official docs
        match provider {
            "openai" | "lmstudio" => {
                if self.role == MessageRole::Tool && self.tool_call_id.is_none() {
                    return Err(format!("{} requires tool_call_id for tool messages", provider));
                }
            }
            "gemini" => {
                if self.role == MessageRole::Tool && self.tool_call_id.is_none() {
                    return Err("Gemini tool responses need tool_call_id for function name mapping".to_string());
                }
                // Gemini has additional constraints on content structure
                if self.role == MessageRole::System && !self.content.is_empty() {
                    // System messages should be handled as systemInstruction, not in contents
                }
            }
            "anthropic" => {
                // Anthropic is more flexible with tool message format
                // Tool messages are converted to user messages anyway
            }
            _ => {} // Generic validation already done above
        }

        Ok(())
    }

    /// Check if this message has tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.as_ref().map_or(false, |calls| !calls.is_empty())
    }

    /// Get the tool calls if present
    pub fn get_tool_calls(&self) -> Option<&[ToolCall]> {
        self.tool_calls.as_deref()
    }

    /// Check if this is a tool response message
    pub fn is_tool_response(&self) -> bool {
        self.role == MessageRole::Tool
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    /// Get the role string for Gemini API
    /// Note: Gemini API has specific constraints on message roles
    /// - Only accepts "user" and "model" roles in conversations
    /// - System messages are handled separately as system instructions
    /// - Tool responses are sent as "user" role with function response format
    pub fn as_gemini_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system", // Handled as systemInstruction, not in contents
            MessageRole::User => "user",
            MessageRole::Assistant => "model", // Gemini uses "model" instead of "assistant"
            MessageRole::Tool => "user", // Tool responses are sent as user messages with functionResponse
        }
    }

    /// Get the role string for OpenAI API
    /// OpenAI supports all standard role types including:
    /// - system, user, assistant, tool
    /// - function (legacy, now replaced by tool)
    pub fn as_openai_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool", // Full support for tool role with tool_call_id
        }
    }

    /// Get the role string for Anthropic API
    /// Anthropic has specific handling for tool messages:
    /// - Supports user, assistant roles normally
    /// - Tool responses are treated as user messages
    /// - System messages can be handled as system parameter or hoisted
    pub fn as_anthropic_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system", // Can be hoisted to system parameter
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "user", // Anthropic treats tool responses as user messages
        }
    }

    /// Get the role string for generic OpenAI-compatible providers
    /// Most providers follow OpenAI's role conventions
    pub fn as_generic_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }

    /// Check if this role supports tool calls
    /// Only Assistant role can initiate tool calls in most APIs
    pub fn can_make_tool_calls(&self) -> bool {
        matches!(self, MessageRole::Assistant)
    }

    /// Check if this role represents a tool response
    pub fn is_tool_response(&self) -> bool {
        matches!(self, MessageRole::Tool)
    }

    /// Validate message role constraints for a given provider
    /// Based on official API documentation requirements
    pub fn validate_for_provider(&self, provider: &str, has_tool_call_id: bool) -> Result<(), String> {
        match (self, provider) {
            (MessageRole::Tool, "openai") if !has_tool_call_id => {
                Err("OpenAI tool messages must have tool_call_id".to_string())
            }
            (MessageRole::Tool, "lmstudio") if !has_tool_call_id => {
                Err("LMStudio tool messages should have tool_call_id".to_string())
            }
            (MessageRole::Tool, "gemini") if !has_tool_call_id => {
                Err("Gemini tool messages need tool_call_id for function mapping".to_string())
            }
            _ => Ok(())
        }
    }
}

/// Universal tool definition that matches OpenAI/Anthropic/Gemini specifications
/// Based on official API documentation from Context7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The type of tool (always "function" for function calling)
    #[serde(rename = "type")]
    pub tool_type: String,

    /// Function definition containing name, description, and parameters
    pub function: FunctionDefinition,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// The name of the function to be called
    pub name: String,

    /// A description of what the function does
    pub description: String,

    /// The parameters the function accepts, described as a JSON Schema object
    pub parameters: Value,
}

impl ToolDefinition {
    /// Create a new tool definition with function type
    pub fn function(name: String, description: String, parameters: Value) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name,
                description,
                parameters,
            },
        }
    }

    /// Get the function name for easy access
    pub fn function_name(&self) -> &str {
        &self.function.name
    }

    /// Validate that this tool definition is properly formed
    pub fn validate(&self) -> Result<(), String> {
        if self.tool_type != "function" {
            return Err(format!("Only 'function' type is supported, got: {}", self.tool_type));
        }

        if self.function.name.is_empty() {
            return Err("Function name cannot be empty".to_string());
        }

        if self.function.description.is_empty() {
            return Err("Function description cannot be empty".to_string());
        }

        // Validate that parameters is a proper JSON Schema object
        if !self.function.parameters.is_object() {
            return Err("Function parameters must be a JSON object".to_string());
        }

        Ok(())
    }
}

/// Universal tool call that matches the exact structure from OpenAI API
/// Based on OpenAI Cookbook examples and official documentation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call (e.g., "call_123")
    pub id: String,

    /// The type of tool call (always "function" for function calling)
    #[serde(rename = "type")]
    pub call_type: String,

    /// Function call details
    pub function: FunctionCall,
}

/// Function call within a tool call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// The name of the function to call
    pub name: String,

    /// The arguments to pass to the function, as a JSON string
    pub arguments: String,
}

impl ToolCall {
    /// Create a new function tool call
    pub fn function(id: String, name: String, arguments: String) -> Self {
        Self {
            id,
            call_type: "function".to_string(),
            function: FunctionCall { name, arguments },
        }
    }

    /// Parse the arguments as JSON Value
    pub fn parsed_arguments(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&self.function.arguments)
    }

    /// Validate that this tool call is properly formed
    pub fn validate(&self) -> Result<(), String> {
        if self.call_type != "function" {
            return Err(format!("Only 'function' type is supported, got: {}", self.call_type));
        }

        if self.id.is_empty() {
            return Err("Tool call ID cannot be empty".to_string());
        }

        if self.function.name.is_empty() {
            return Err("Function name cannot be empty".to_string());
        }

        // Validate that arguments is valid JSON
        if let Err(e) = self.parsed_arguments() {
            return Err(format!("Invalid JSON in function arguments: {}", e));
        }

        Ok(())
    }
}

/// Universal LLM response
#[derive(Debug, Clone)]
pub struct LLMResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub usage: Option<Usage>,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error(String),
}

/// Universal LLM provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Provider name (e.g., "gemini", "openai", "anthropic")
    fn name(&self) -> &str;

    /// Generate completion
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;

    /// Stream completion (optional)
    async fn stream(
        &self,
        request: LLMRequest,
    ) -> Result<Box<dyn futures::Stream<Item = LLMResponse> + Unpin + Send>, LLMError> {
        // Default implementation falls back to non-streaming
        let response = self.generate(request).await?;
        Ok(Box::new(futures::stream::once(async { response }).boxed()))
    }

    /// Get supported models
    fn supported_models(&self) -> Vec<String>;

    /// Validate request for this provider
    fn validate_request(&self, request: &LLMRequest) -> Result<(), LLMError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Rate limit exceeded")]
    RateLimit,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Provider error: {0}")]
    Provider(String),
}

// Implement conversion from provider::LLMError to llm::types::LLMError
impl From<LLMError> for crate::llm::types::LLMError {
    fn from(err: LLMError) -> crate::llm::types::LLMError {
        match err {
            LLMError::Authentication(msg) => crate::llm::types::LLMError::ApiError(msg),
            LLMError::RateLimit => crate::llm::types::LLMError::RateLimit,
            LLMError::InvalidRequest(msg) => crate::llm::types::LLMError::InvalidRequest(msg),
            LLMError::Network(msg) => crate::llm::types::LLMError::NetworkError(msg),
            LLMError::Provider(msg) => crate::llm::types::LLMError::ApiError(msg),
        }
    }
}

use futures::StreamExt;

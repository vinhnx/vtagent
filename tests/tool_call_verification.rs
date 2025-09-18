//! Comprehensive tool call verification for all LLM providers

use serde_json::json;
use vtcode_core::llm::{
    provider::{
        LLMProvider, LLMRequest, Message, MessageRole, ToolCall, ToolChoice, ToolDefinition,
    },
    providers::{AnthropicProvider, GeminiProvider, OpenAIProvider},
};

#[test]
fn test_openai_tool_call_format() {
    let provider = OpenAIProvider::new("test_key".to_string());

    // Test tool definition
    let tool = ToolDefinition::function(
        "get_weather".to_string(),
        "Get weather for a location".to_string(),
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    );

    // Test assistant message with tool call
    let assistant_msg = Message {
        role: MessageRole::Assistant,
        content: "I'll get the weather for you.".to_string(),
        tool_calls: Some(vec![ToolCall::function(
            "call_123".to_string(),
            "get_weather".to_string(),
            json!({"location": "New York"}).to_string(),
        )]),
        tool_call_id: None,
    };

    // Test tool response message
    let tool_msg = Message {
        role: MessageRole::Tool,
        content: "Sunny, 72°F".to_string(),
        tool_calls: None,
        tool_call_id: Some("call_123".to_string()),
    };

    let request = LLMRequest {
        messages: vec![
            Message::user("What's the weather in New York?".to_string()),
            assistant_msg,
            tool_msg,
        ],
        system_prompt: Some("You are a helpful assistant.".to_string()),
        tools: Some(vec![tool]),
        model: "gpt-5".to_string(),
        max_tokens: Some(1000),
        temperature: Some(0.7),
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    // Only validate shape via provider API; internal conversion details are private
    assert!(provider.validate_request(&request).is_ok());
}

#[test]
fn test_anthropic_tool_call_format() {
    let provider = AnthropicProvider::new("test_key".to_string());

    // Test tool definition
    let tool = ToolDefinition::function(
        "get_weather".to_string(),
        "Get weather for a location".to_string(),
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    );

    // Test assistant message with tool call
    let assistant_msg = Message {
        role: MessageRole::Assistant,
        content: "I'll get the weather for you.".to_string(),
        tool_calls: Some(vec![ToolCall::function(
            "toolu_123".to_string(),
            "get_weather".to_string(),
            json!({"location": "New York"}).to_string(),
        )]),
        tool_call_id: None,
    };

    // Test tool response message
    let tool_msg = Message {
        role: MessageRole::Tool,
        content: "Sunny, 72°F".to_string(),
        tool_calls: Some(vec![ToolCall::function(
            "toolu_123".to_string(),
            "get_weather".to_string(),
            json!({}).to_string(),
        )]),
        tool_call_id: None,
    };

    let request = LLMRequest {
        messages: vec![
            Message::user("What's the weather in New York?".to_string()),
            assistant_msg,
            tool_msg,
        ],
        system_prompt: Some("You are a helpful assistant.".to_string()),
        tools: Some(vec![tool]),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: Some(1000),
        temperature: Some(0.7),
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    // Only validate shape via provider API; internal conversion details are private
    assert!(provider.validate_request(&request).is_ok());
}

#[test]
fn test_gemini_tool_call_format() {
    let provider = GeminiProvider::new("test_key".to_string());

    // Test tool definition
    let tool = ToolDefinition::function(
        "get_weather".to_string(),
        "Get weather for a location".to_string(),
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    );

    // Test assistant message with tool call
    let assistant_msg = Message {
        role: MessageRole::Assistant,
        content: "I'll get the weather for you.".to_string(),
        tool_calls: Some(vec![ToolCall::function(
            "func_123".to_string(),
            "get_weather".to_string(),
            json!({"location": "New York"}).to_string(),
        )]),
        tool_call_id: None,
    };

    // Test tool response message
    let tool_msg = Message {
        role: MessageRole::Tool,
        content: "Sunny, 72°F".to_string(),
        tool_calls: Some(vec![ToolCall::function(
            "func_123".to_string(),
            "get_weather".to_string(),
            json!({"location": "New York"}).to_string(),
        )]),
        tool_call_id: None,
    };

    let request = LLMRequest {
        messages: vec![
            Message::user("What's the weather in New York?".to_string()),
            assistant_msg,
            tool_msg,
        ],
        system_prompt: Some("You are a helpful assistant.".to_string()),
        tools: Some(vec![tool]),
        model: "gemini-2.5-flash".to_string(),
        max_tokens: Some(1000),
        temperature: Some(0.7),
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    assert!(provider.validate_request(&request).is_ok());
}

#[test]
fn test_all_providers_tool_validation() {
    let gemini = GeminiProvider::new("test_key".to_string());
    let openai = OpenAIProvider::new("test_key".to_string());
    let anthropic = AnthropicProvider::new("test_key".to_string());

    // Test valid requests with tools
    let tool = ToolDefinition::function(
        "test_tool".to_string(),
        "A test tool".to_string(),
        json!({"type": "object"}),
    );

    let gemini_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: Some(vec![tool.clone()]),
        model: "gemini-2.5-flash".to_string(),
        max_tokens: Some(1000),
        temperature: Some(0.7),
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    let openai_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: Some(vec![tool.clone()]),
        model: "gpt-5".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: Some(ToolChoice::auto()),
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    let anthropic_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: Some(vec![tool]),
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    assert!(gemini.validate_request(&gemini_request).is_ok());
    assert!(openai.validate_request(&openai_request).is_ok());
    assert!(anthropic.validate_request(&anthropic_request).is_ok());
}

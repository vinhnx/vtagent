//! Comprehensive tool call verification for all LLM providers

use serde_json::json;
use vtagent_core::llm::{
    AnthropicProvider, GeminiProvider, LLMProvider, LLMRequest, Message, MessageRole,
    OpenAIProvider, ToolCall, ToolDefinition,
};

#[test]
fn test_openai_tool_call_format() {
    let provider = OpenAIProvider::new("test_key".to_string());

    // Test tool definition
    let tool = ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get weather for a location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    };

    // Test assistant message with tool call
    let assistant_msg = Message {
        role: MessageRole::Assistant,
        content: "I'll get the weather for you.".to_string(),
        tool_calls: Some(vec![ToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({"location": "New York"}),
        }]),
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
        reasoning_effort: None,
    };

    let result = provider.convert_to_openai_format(&request);
    assert!(result.is_ok(), "OpenAI format conversion should succeed");

    if let Ok(openai_request) = result {
        let messages = openai_request["messages"].as_array().unwrap();

        // Check system message
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "You are a helpful assistant.");

        // Check user message
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "What's the weather in New York?");

        // Check assistant message with tool call
        assert_eq!(messages[2]["role"], "assistant");
        assert!(messages[2]["tool_calls"].is_array());
        let tool_calls = messages[2]["tool_calls"].as_array().unwrap();
        assert_eq!(tool_calls[0]["id"], "call_123");
        assert_eq!(tool_calls[0]["type"], "function");
        assert_eq!(tool_calls[0]["function"]["name"], "get_weather");

        // Check tool message
        assert_eq!(messages[3]["role"], "tool");
        assert_eq!(messages[3]["content"], "Sunny, 72°F");
        assert_eq!(messages[3]["tool_call_id"], "call_123");

        // Check tools definition
        let tools = openai_request["tools"].as_array().unwrap();
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "get_weather");
    }
}

#[test]
fn test_anthropic_tool_call_format() {
    let provider = AnthropicProvider::new("test_key".to_string());

    // Test tool definition
    let tool = ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get weather for a location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    };

    // Test assistant message with tool call
    let assistant_msg = Message {
        role: MessageRole::Assistant,
        content: "I'll get the weather for you.".to_string(),
        tool_calls: Some(vec![ToolCall {
            id: "toolu_123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({"location": "New York"}),
        }]),
        tool_call_id: None,
    };

    // Test tool response message
    let tool_msg = Message {
        role: MessageRole::Tool,
        content: "Sunny, 72°F".to_string(),
        tool_calls: Some(vec![ToolCall {
            id: "toolu_123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({}),
        }]),
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
        reasoning_effort: None,
    };

    let result = provider.convert_to_anthropic_format(&request);
    assert!(result.is_ok(), "Anthropic format conversion should succeed");

    if let Ok(anthropic_request) = result {
        let messages = anthropic_request["messages"].as_array().unwrap();

        // Check system prompt (separate field)
        assert_eq!(anthropic_request["system"], "You are a helpful assistant.");

        // Check user message
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "What's the weather in New York?");

        // Check assistant message with tool_use
        assert_eq!(messages[1]["role"], "assistant");
        let content = messages[1]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "I'll get the weather for you.");
        assert_eq!(content[1]["type"], "tool_use");
        assert_eq!(content[1]["id"], "toolu_123");
        assert_eq!(content[1]["name"], "get_weather");

        // Check tool result message (user role with tool_result)
        assert_eq!(messages[2]["role"], "user");
        let tool_results = messages[2]["content"].as_array().unwrap();
        assert_eq!(tool_results[0]["type"], "tool_result");
        assert_eq!(tool_results[0]["tool_use_id"], "toolu_123");
        assert_eq!(tool_results[0]["content"], "Sunny, 72°F");

        // Check tools definition
        let tools = anthropic_request["tools"].as_array().unwrap();
        assert_eq!(tools[0]["name"], "get_weather");
        assert_eq!(tools[0]["description"], "Get weather for a location");
    }
}

#[test]
fn test_gemini_tool_call_format() {
    let provider = GeminiProvider::new("test_key".to_string());

    // Test tool definition
    let tool = ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get weather for a location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string"}
            },
            "required": ["location"]
        }),
    };

    // Test assistant message with tool call
    let assistant_msg = Message {
        role: MessageRole::Assistant,
        content: "I'll get the weather for you.".to_string(),
        tool_calls: Some(vec![ToolCall {
            id: "func_123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({"location": "New York"}),
        }]),
        tool_call_id: None,
    };

    // Test tool response message
    let tool_msg = Message {
        role: MessageRole::Tool,
        content: "Sunny, 72°F".to_string(),
        tool_calls: Some(vec![ToolCall {
            id: "func_123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({}),
        }]),
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
        reasoning_effort: None,
    };

    let result = provider.convert_to_gemini_format(&request);
    assert!(result.is_ok(), "Gemini format conversion should succeed");

    if let Ok(gemini_request) = result {
        let contents = gemini_request["contents"].as_array().unwrap();

        // Check system instruction (separate field)
        assert_eq!(
            gemini_request["systemInstruction"]["parts"][0]["text"],
            "You are a helpful assistant."
        );

        // Check user message
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(
            contents[0]["parts"][0]["text"],
            "What's the weather in New York?"
        );

        // Check assistant message with function call
        assert_eq!(contents[1]["role"], "model");
        let parts = contents[1]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["text"], "I'll get the weather for you.");
        assert_eq!(parts[1]["functionCall"]["name"], "get_weather");

        // Check function response message
        assert_eq!(contents[2]["role"], "function");
        let func_parts = contents[2]["parts"].as_array().unwrap();
        assert_eq!(func_parts[0]["functionResponse"]["name"], "get_weather");
        assert_eq!(
            func_parts[0]["functionResponse"]["response"]["content"],
            "Sunny, 72°F"
        );

        // Check tools definition
        let tools = gemini_request["tools"].as_array().unwrap();
        assert_eq!(tools[0]["functionDeclarations"][0]["name"], "get_weather");
        assert_eq!(
            tools[0]["functionDeclarations"][0]["description"],
            "Get weather for a location"
        );
    }
}

#[test]
fn test_all_providers_tool_validation() {
    let gemini = GeminiProvider::new("test_key".to_string());
    let openai = OpenAIProvider::new("test_key".to_string());
    let anthropic = AnthropicProvider::new("test_key".to_string());

    // Test valid requests with tools
    let tool = ToolDefinition {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        parameters: json!({"type": "object"}),
    };

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
        tool_choice: None,
        parallel_tool_calls: None,
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
    };

    assert!(gemini.validate_request(&gemini_request).is_ok());
    assert!(openai.validate_request(&openai_request).is_ok());
    assert!(anthropic.validate_request(&anthropic_request).is_ok());
}

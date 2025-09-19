//! Comprehensive tests for LLM providers refactor

use serde_json::json;
use vtcode_core::llm::{
    factory::{LLMFactory, create_provider_for_model},
    provider::{LLMProvider, LLMRequest, Message, MessageRole, ToolCall, ToolDefinition},
    providers::{AnthropicProvider, GeminiProvider, OpenAIProvider},
};

#[test]
fn test_provider_factory_creation() {
    let factory = LLMFactory::new();

    // Test available providers
    let providers = factory.list_providers();
    assert!(providers.contains(&"gemini".to_string()));
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"anthropic".to_string()));
    assert_eq!(providers.len(), 3);
}

#[test]
fn test_provider_auto_detection() {
    let factory = LLMFactory::new();

    // Test OpenAI models
    assert_eq!(
        factory.provider_from_model("gpt-5"),
        Some("openai".to_string())
    );
    assert_eq!(
        factory.provider_from_model("gpt-5-mini"),
        Some("openai".to_string())
    );

    // Test Anthropic models
    assert_eq!(
        factory.provider_from_model("claude-sonnet-4-20250514"),
        Some("anthropic".to_string())
    );
    assert_eq!(
        factory.provider_from_model("claude-opus-4-1-20250805"),
        Some("anthropic".to_string())
    );

    // Test Gemini models
    assert_eq!(
        factory.provider_from_model("gemini-2.5-flash"),
        Some("gemini".to_string())
    );
    assert_eq!(
        factory.provider_from_model("gemini-2.5-flash-lite"),
        Some("gemini".to_string())
    );
    assert_eq!(
        factory.provider_from_model("gemini-2.5-pro"),
        Some("gemini".to_string())
    );

    // Test unknown model
    assert_eq!(factory.provider_from_model("unknown-model"), None);
}

#[test]
fn test_provider_creation() {
    // Test creating providers directly
    let gemini = create_provider_for_model(
        "gemini-2.5-flash-preview-05-20",
        "test_key".to_string(),
    );
    assert!(gemini.is_ok());

    let openai = create_provider_for_model("gpt-5", "test_key".to_string());
    assert!(openai.is_ok());

    let anthropic = create_provider_for_model("claude-sonnet-4-20250514", "test_key".to_string());
    assert!(anthropic.is_ok());

    // Test invalid model
    let invalid = create_provider_for_model("invalid-model", "test_key".to_string());
    assert!(invalid.is_err());
}

#[test]
fn test_unified_client_creation() {
    // Test creating unified clients for different providers
    let gemini_client = create_provider_for_model(
        "gemini-2.5-flash-preview-05-20",
        "test_key".to_string(),
    );
    assert!(gemini_client.is_ok());
    if let Ok(client) = gemini_client {
        assert_eq!(client.name(), "gemini");
    }

    let openai_client = create_provider_for_model("gpt-5", "test_key".to_string());
    assert!(openai_client.is_ok());
    if let Ok(client) = openai_client {
        assert_eq!(client.name(), "openai");
    }

    let anthropic_client =
        create_provider_for_model("claude-sonnet-4-20250514", "test_key".to_string());
    assert!(anthropic_client.is_ok());
    if let Ok(client) = anthropic_client {
        assert_eq!(client.name(), "anthropic");
    }
}

#[test]
fn test_message_creation() {
    // Test message creation helpers
    let user_msg = Message::user("Hello, world!".to_string());
    assert_eq!(user_msg.content, "Hello, world!");
    assert!(matches!(user_msg.role, MessageRole::User));
    assert!(user_msg.tool_calls.is_none());

    let assistant_msg = Message::assistant("Hi there!".to_string());
    assert_eq!(assistant_msg.content, "Hi there!");
    assert!(matches!(assistant_msg.role, MessageRole::Assistant));

    let system_msg = Message::system("You are a helpful assistant".to_string());
    assert_eq!(system_msg.content, "You are a helpful assistant");
    assert!(matches!(system_msg.role, MessageRole::System));
}

#[test]
#[ignore]
fn test_provider_supported_models() {
    // Test that providers report correct supported models
    let gemini = GeminiProvider::new("test_key".to_string());
    let gemini_models = gemini.supported_models();
    assert!(gemini_models.contains(&"gemini-2.5-flash".to_string()));
    assert!(gemini_models.contains(&"gemini-2.5-flash-lite".to_string()));
    assert!(gemini_models.contains(&"gemini-2.5-pro".to_string()));
    assert!(gemini_models.contains(&"gemini-2.5-flash-lite-preview-06-17".to_string()));
    assert!(gemini_models.contains(&"gemini-2.5-flash-preview-05-20".to_string()));
    assert!(gemini_models.len() >= 5);

    let openai = OpenAIProvider::new("test_key".to_string());
    let openai_models = openai.supported_models();
    assert!(openai_models.contains(&"gpt-5".to_string()));
    assert!(openai_models.contains(&"gpt-5-mini".to_string()));
    assert!(openai_models.len() >= 2);

    let anthropic = AnthropicProvider::new("test_key".to_string());
    let anthropic_models = anthropic.supported_models();
    assert!(anthropic_models.contains(&"claude-sonnet-4-20250514".to_string()));
    assert!(anthropic_models.contains(&"claude-opus-4-1-20250805".to_string()));
    assert!(anthropic_models.len() >= 2);
}

#[test]
fn test_provider_names() {
    let gemini = GeminiProvider::new("test_key".to_string());
    assert_eq!(gemini.name(), "gemini");

    let openai = OpenAIProvider::new("test_key".to_string());
    assert_eq!(openai.name(), "openai");

    let anthropic = AnthropicProvider::new("test_key".to_string());
    assert_eq!(anthropic.name(), "anthropic");
}

#[test]
#[ignore]
fn test_request_validation() {
    let gemini = GeminiProvider::new("test_key".to_string());
    let openai = OpenAIProvider::new("test_key".to_string());
    let anthropic = AnthropicProvider::new("test_key".to_string());

    // Test valid requests
    let valid_gemini_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "gemini-2.5-flash-lite-preview-06-17".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };
    assert!(gemini.validate_request(&valid_gemini_request).is_ok());

    let valid_openai_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "gpt-5".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };
    assert!(openai.validate_request(&valid_openai_request).is_ok());

    let valid_anthropic_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };
    assert!(anthropic.validate_request(&valid_anthropic_request).is_ok());

    // Test invalid requests (wrong model for provider)
    let invalid_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "invalid-model".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };
    assert!(gemini.validate_request(&invalid_request).is_err());
    assert!(openai.validate_request(&invalid_request).is_err());
    assert!(anthropic.validate_request(&invalid_request).is_err());
}

#[test]
fn test_anthropic_tool_message_handling() {
    let anthropic = AnthropicProvider::new("test_key".to_string());

    // Test tool message conversion
    let tool_message = Message {
        role: MessageRole::Tool,
        content: "Tool result content".to_string(),
        tool_calls: Some(vec![ToolCall::function(
            "call_123".to_string(),
            "test_tool".to_string(),
            json!({"param": "value"}).to_string(),
        )]),
        tool_call_id: Some("tool_123".to_string()),
    };

    let request = LLMRequest {
        messages: vec![tool_message],
        system_prompt: None,
        tools: None,
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    // Use the public validator as a proxy for ensuring request shape is acceptable
    // (internal conversion is implementation detail and not tested directly here)
    assert!(anthropic.validate_request(&request).is_ok());
}

#[test]
fn test_backward_compatibility() {
    use vtcode_core::llm::make_client;
    use vtcode_core::models::ModelId;

    // Test that the old make_client function still works
    let model = ModelId::from_str("gemini-2.5-flash-preview-05-20").unwrap();
    let client = make_client("test_key".to_string(), model);

    // Should be able to get model ID
    let model_id = client.model_id();
    assert!(!model_id.is_empty());
}

#[test]
fn test_tool_definition_creation() {
    let tool = ToolDefinition::function(
        "get_weather".to_string(),
        "Get weather for a location".to_string(),
        json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "The location to get weather for"}
            },
            "required": ["location"]
        }),
    );

    assert_eq!(tool.function_name(), "get_weather");
    assert_eq!(tool.function.description, "Get weather for a location");
    assert!(tool.function.parameters.is_object());
}

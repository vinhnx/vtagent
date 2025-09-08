//! Comprehensive tests for LLM providers refactor

use vtagent_core::llm::{
    UnifiedLLMClient, Message, MessageRole, LLMFactory, create_provider_for_model,
    GeminiProvider, OpenAIProvider, AnthropicProvider, LLMProvider, LLMRequest, ToolDefinition
};
use serde_json::json;

#[test]
fn test_provider_factory_creation() {
    let factory = LLMFactory::new();
    
    // Test available providers
    let providers = factory.available_providers();
    assert!(providers.contains(&"gemini".to_string()));
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"anthropic".to_string()));
    assert_eq!(providers.len(), 3);
}

#[test]
fn test_provider_auto_detection() {
    let factory = LLMFactory::new();
    
    // Test OpenAI models
    assert_eq!(factory.provider_from_model("gpt-4o"), Some("openai".to_string()));
    assert_eq!(factory.provider_from_model("gpt-4o-mini"), Some("openai".to_string()));
    assert_eq!(factory.provider_from_model("gpt-3.5-turbo"), Some("openai".to_string()));
    assert_eq!(factory.provider_from_model("o1-preview"), Some("openai".to_string()));
    
    // Test Anthropic models
    assert_eq!(factory.provider_from_model("claude-3-5-sonnet"), Some("anthropic".to_string()));
    assert_eq!(factory.provider_from_model("claude-3-opus"), Some("anthropic".to_string()));
    assert_eq!(factory.provider_from_model("claude-3-haiku"), Some("anthropic".to_string()));
    
    // Test Gemini models
    assert_eq!(factory.provider_from_model("gemini-2.5-flash"), Some("gemini".to_string()));
    assert_eq!(factory.provider_from_model("gemini-1.5-pro"), Some("gemini".to_string()));
    
    // Test unknown model
    assert_eq!(factory.provider_from_model("unknown-model"), None);
}

#[test]
fn test_provider_creation() {
    // Test creating providers directly
    let gemini = create_provider_for_model("gemini-2.5-flash", "test_key".to_string());
    assert!(gemini.is_ok());
    
    let openai = create_provider_for_model("gpt-4o", "test_key".to_string());
    assert!(openai.is_ok());
    
    let anthropic = create_provider_for_model("claude-3-5-sonnet", "test_key".to_string());
    assert!(anthropic.is_ok());
    
    // Test invalid model
    let invalid = create_provider_for_model("invalid-model", "test_key".to_string());
    assert!(invalid.is_err());
}

#[test]
fn test_unified_client_creation() {
    // Test creating unified clients for different providers
    let gemini_client = UnifiedLLMClient::new("gemini-2.5-flash".to_string(), "test_key".to_string());
    assert!(gemini_client.is_ok());
    if let Ok(client) = gemini_client {
        assert_eq!(client.model(), "gemini-2.5-flash");
        assert_eq!(client.provider_name(), "gemini");
    }
    
    let openai_client = UnifiedLLMClient::new("gpt-4o".to_string(), "test_key".to_string());
    assert!(openai_client.is_ok());
    if let Ok(client) = openai_client {
        assert_eq!(client.model(), "gpt-4o");
        assert_eq!(client.provider_name(), "openai");
    }
    
    let anthropic_client = UnifiedLLMClient::new("claude-3-5-sonnet".to_string(), "test_key".to_string());
    assert!(anthropic_client.is_ok());
    if let Ok(client) = anthropic_client {
        assert_eq!(client.model(), "claude-3-5-sonnet");
        assert_eq!(client.provider_name(), "anthropic");
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
fn test_provider_supported_models() {
    // Test that providers report correct supported models
    let gemini = GeminiProvider::new("test_key".to_string());
    let gemini_models = gemini.supported_models();
    assert!(gemini_models.contains(&"gemini-2.5-flash".to_string()));
    assert!(gemini_models.contains(&"gemini-1.5-pro".to_string()));
    assert!(gemini_models.len() >= 2);
    
    let openai = OpenAIProvider::new("test_key".to_string());
    let openai_models = openai.supported_models();
    assert!(openai_models.contains(&"gpt-4o".to_string()));
    assert!(openai_models.contains(&"gpt-3.5-turbo".to_string()));
    assert!(openai_models.len() >= 2);
    
    let anthropic = AnthropicProvider::new("test_key".to_string());
    let anthropic_models = anthropic.supported_models();
    assert!(anthropic_models.contains(&"claude-3-5-sonnet-20241022".to_string()));
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
fn test_request_validation() {
    let gemini = GeminiProvider::new("test_key".to_string());
    let openai = OpenAIProvider::new("test_key".to_string());
    let anthropic = AnthropicProvider::new("test_key".to_string());
    
    // Test valid requests
    let valid_gemini_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "gemini-2.5-flash".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
    };
    assert!(gemini.validate_request(&valid_gemini_request).is_ok());
    
    let valid_openai_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "gpt-4o".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
    };
    assert!(openai.validate_request(&valid_openai_request).is_ok());
    
    let valid_anthropic_request = LLMRequest {
        messages: vec![Message::user("test".to_string())],
        system_prompt: None,
        tools: None,
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
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
        tool_calls: Some(vec![vtagent_core::llm::ToolCall {
            id: "tool_123".to_string(),
            name: "test_tool".to_string(),
            arguments: json!({"param": "value"}),
        }]),
        tool_call_id: Some("tool_123".to_string()),
    };
    
    let request = LLMRequest {
        messages: vec![tool_message],
        system_prompt: None,
        tools: None,
        model: "claude-3-5-sonnet-20241022".to_string(),
        max_tokens: None,
        temperature: None,
        stream: false,
    };
    
    // This should not panic and should convert tool messages to user messages
    let result = anthropic.convert_to_anthropic_format(&request);
    assert!(result.is_ok());
    
    if let Ok(anthropic_request) = result {
        let messages = anthropic_request["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"].as_str().unwrap(), "user");
    }
}

#[test]
fn test_backward_compatibility() {
    use vtagent_core::llm::{make_client, AnyClient};
    use vtagent_core::models::ModelId;
    
    // Test that the old make_client function still works
    let model = ModelId::from_str("gemini-2.5-flash").unwrap();
    let client = make_client("test_key".to_string(), model);
    
    // Should be able to get model ID
    let model_id = client.model_id();
    assert!(!model_id.is_empty());
}

#[test]
fn test_tool_definition_creation() {
    let tool = ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get weather for a location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {"type": "string", "description": "The location to get weather for"}
            },
            "required": ["location"]
        }),
    };
    
    assert_eq!(tool.name, "get_weather");
    assert_eq!(tool.description, "Get weather for a location");
    assert!(tool.parameters.is_object());
}

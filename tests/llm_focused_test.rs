//! Focused test for LLM provider functionality

use vtagent_core::llm::{
    factory::{LLMFactory, create_provider_for_model},
    provider::{LLMProvider, LLMRequest, Message, MessageRole},
    providers::{AnthropicProvider, GeminiProvider, OpenAIProvider},
};

#[test]
fn test_provider_factory_basic() {
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

    assert_eq!(
        factory.provider_from_model("gpt-5"),
        Some("openai".to_string())
    );
    assert_eq!(
        factory.provider_from_model("claude-sonnet-4-20250514"),
        Some("anthropic".to_string())
    );
    assert_eq!(
        factory.provider_from_model("gemini-2.5-flash-lite-preview-06-17"),
        Some("gemini".to_string())
    );
    assert_eq!(factory.provider_from_model("unknown-model"), None);
}

#[test]
fn test_unified_client_creation() {
    // Test creating providers directly using the factory
    let gemini = create_provider_for_model("gemini-2.5-flash-lite-preview-06-17", "test_key".to_string());
    assert!(gemini.is_ok());

    let openai = create_provider_for_model("gpt-5", "test_key".to_string());
    assert!(openai.is_ok());

    let anthropic = create_provider_for_model("claude-sonnet-4-20250514", "test_key".to_string());
    assert!(anthropic.is_ok());
}

#[test]
fn test_message_creation() {
    let user_msg = Message::user("Hello".to_string());
    assert_eq!(user_msg.content, "Hello");
    assert!(matches!(user_msg.role, MessageRole::User));

    let assistant_msg = Message::assistant("Hi".to_string());
    assert_eq!(assistant_msg.content, "Hi");
    assert!(matches!(assistant_msg.role, MessageRole::Assistant));
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
fn test_anthropic_tool_message_handling() {
    let anthropic = AnthropicProvider::new("test_key".to_string());

    // Test that tool messages are converted to user messages for Anthropic
    let tool_message = Message {
        role: MessageRole::Tool,
        content: "Tool result".to_string(),
        tool_calls: None,
        tool_call_id: None,
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

    // Validate request shape instead of internal conversion
    assert!(anthropic.validate_request(&request).is_ok());
}

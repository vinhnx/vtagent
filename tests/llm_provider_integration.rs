//! Integration tests for universal LLM provider system

use vtagent_core::llm::{
    UnifiedLLMClient, Message, LLMFactory, create_provider_for_model,
    GeminiProvider, OpenAIProvider, AnthropicProvider
};

#[test]
fn test_provider_factory() {
    let factory = LLMFactory::new();
    
    // Test available providers
    let providers = factory.available_providers();
    assert!(providers.contains(&"gemini".to_string()));
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"anthropic".to_string()));
    
    // Test provider detection from model names
    assert_eq!(factory.provider_from_model("gpt-4o"), Some("openai".to_string()));
    assert_eq!(factory.provider_from_model("claude-3-5-sonnet"), Some("anthropic".to_string()));
    assert_eq!(factory.provider_from_model("gemini-2.5-flash"), Some("gemini".to_string()));
}

#[test]
fn test_provider_creation() {
    // Test creating providers
    let gemini = create_provider_for_model("gemini-2.5-flash", "test_key".to_string());
    assert!(gemini.is_ok());
    
    let openai = create_provider_for_model("gpt-4o", "test_key".to_string());
    assert!(openai.is_ok());
    
    let anthropic = create_provider_for_model("claude-3-5-sonnet", "test_key".to_string());
    assert!(anthropic.is_ok());
}

#[test]
fn test_unified_client_creation() {
    // Test creating unified clients for different providers
    let gemini_client = UnifiedLLMClient::new("gemini-2.5-flash".to_string(), "test_key".to_string());
    assert!(gemini_client.is_ok());
    
    let openai_client = UnifiedLLMClient::new("gpt-4o".to_string(), "test_key".to_string());
    assert!(openai_client.is_ok());
    
    let anthropic_client = UnifiedLLMClient::new("claude-3-5-sonnet".to_string(), "test_key".to_string());
    assert!(anthropic_client.is_ok());
}

#[test]
fn test_message_creation() {
    // Test message creation helpers
    let user_msg = Message::user("Hello, world!".to_string());
    assert_eq!(user_msg.content, "Hello, world!");
    
    let assistant_msg = Message::assistant("Hi there!".to_string());
    assert_eq!(assistant_msg.content, "Hi there!");
    
    let system_msg = Message::system("You are a helpful assistant".to_string());
    assert_eq!(system_msg.content, "You are a helpful assistant");
}

#[test]
fn test_provider_supported_models() {
    // Test that providers report correct supported models
    let gemini = GeminiProvider::new("test_key".to_string());
    let gemini_models = gemini.supported_models();
    assert!(gemini_models.contains(&"gemini-2.5-flash".to_string()));
    assert!(gemini_models.contains(&"gemini-1.5-pro".to_string()));
    
    let openai = OpenAIProvider::new("test_key".to_string());
    let openai_models = openai.supported_models();
    assert!(openai_models.contains(&"gpt-4o".to_string()));
    assert!(openai_models.contains(&"gpt-3.5-turbo".to_string()));
    
    let anthropic = AnthropicProvider::new("test_key".to_string());
    let anthropic_models = anthropic.supported_models();
    assert!(anthropic_models.contains(&"claude-3-5-sonnet-20241022".to_string()));
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

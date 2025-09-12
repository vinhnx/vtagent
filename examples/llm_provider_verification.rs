//! Standalone verification of LLM provider system
//! This verifies the universal LLM provider architecture works correctly

fn main() {
    println!("[VERIFICATION] LLM Provider System Verification");
    println!("===================================");

    // Test 1: Provider Factory
    println!("\n1. Testing Provider Factory...");
    test_provider_factory();

    // Test 2: Provider Auto-Detection
    println!("\n2. Testing Provider Auto-Detection...");
    test_provider_auto_detection();

    // Test 3: Provider Creation
    println!("\n3. Testing Provider Creation...");
    test_provider_creation();

    // Test 4: Unified Client Creation
    println!("\n4. Testing Unified Client Creation...");
    test_unified_client_creation();

    // Test 5: Message Creation
    println!("\n5. Testing Message Creation...");
    test_message_creation();

    // Test 6: Anthropic Tool Handling
    println!("\n6. Testing Anthropic Tool Message Handling...");
    test_anthropic_tool_handling();

    println!("\n[SUCCESS] All LLM Provider Tests Passed!");
    println!("Universal LLM Provider System is working correctly.");
}

fn test_provider_factory() {
    use vtagent_core::llm::LLMFactory;

    let factory = LLMFactory::new();
    let providers = factory.available_providers();

    assert_eq!(providers.len(), 3, "Should have 3 providers");
    assert!(
        providers.contains(&"gemini".to_string()),
        "Should have Gemini"
    );
    assert!(
        providers.contains(&"openai".to_string()),
        "Should have OpenAI"
    );
    assert!(
        providers.contains(&"anthropic".to_string()),
        "Should have Anthropic"
    );

    println!(
        "✓ Provider factory created with {} providers",
        providers.len()
    );
}

fn test_provider_auto_detection() {
    use vtagent_core::llm::LLMFactory;

    let factory = LLMFactory::new();

    // Test OpenAI detection
    assert_eq!(
        factory.provider_from_model("gpt-5"),
        Some("openai".to_string())
    );
    assert_eq!(
        factory.provider_from_model("gpt-5-mini"),
        Some("openai".to_string())
    );

    // Test Anthropic detection
    assert_eq!(
        factory.provider_from_model("claude-sonnet-4-20250514"),
        Some("anthropic".to_string())
    );
    assert_eq!(
        factory.provider_from_model("claude-opus-4-20250514"),
        Some("anthropic".to_string())
    );

    // Test Gemini detection
    assert_eq!(
        factory.provider_from_model("gemini-2.5-flash"),
        Some("gemini".to_string())
    );
    assert_eq!(
        factory.provider_from_model("gemini-2.5-flash-lite"),
        Some("gemini".to_string())
    );

    // Test unknown model
    assert_eq!(factory.provider_from_model("unknown-model"), None);

    println!("✓ Provider auto-detection working correctly");
}

fn test_provider_creation() {
    use vtagent_core::llm::create_provider_for_model;

    let gemini = create_provider_for_model("gemini-2.5-flash", "test_key".to_string());
    assert!(gemini.is_ok(), "Should create Gemini provider");

    let openai = create_provider_for_model("gpt-5", "test_key".to_string());
    assert!(openai.is_ok(), "Should create OpenAI provider");

    let anthropic = create_provider_for_model("claude-sonnet-4-20250514", "test_key".to_string());
    assert!(anthropic.is_ok(), "Should create Anthropic provider");

    let invalid = create_provider_for_model("invalid-model", "test_key".to_string());
    assert!(invalid.is_err(), "Should fail for invalid model");

    println!("✓ Provider creation working correctly");
}

fn test_unified_client_creation() {
    use vtagent_core::llm::UnifiedLLMClient;

    let gemini_client =
        UnifiedLLMClient::new("gemini-2.5-flash".to_string(), "test_key".to_string());
    assert!(gemini_client.is_ok(), "Should create Gemini client");
    if let Ok(client) = gemini_client {
        assert_eq!(client.model(), "gemini-2.5-flash");
        assert_eq!(client.provider_name(), "gemini");
    }

    let openai_client = UnifiedLLMClient::new("gpt-5".to_string(), "test_key".to_string());
    assert!(openai_client.is_ok(), "Should create OpenAI client");
    if let Ok(client) = openai_client {
        assert_eq!(client.model(), "gpt-5");
        assert_eq!(client.provider_name(), "openai");
    }

    let anthropic_client = UnifiedLLMClient::new(
        "claude-sonnet-4-20250514".to_string(),
        "test_key".to_string(),
    );
    assert!(anthropic_client.is_ok(), "Should create Anthropic client");
    if let Ok(client) = anthropic_client {
        assert_eq!(client.model(), "claude-sonnet-4-20250514");
        assert_eq!(client.provider_name(), "anthropic");
    }

    println!("✓ Unified client creation working correctly");
}

fn test_message_creation() {
    use vtagent_core::llm::{Message, MessageRole};

    let user_msg = Message::user("Hello, world!".to_string());
    assert_eq!(user_msg.content, "Hello, world!");
    assert!(matches!(user_msg.role, MessageRole::User));
    assert!(user_msg.tool_calls.is_none());

    let assistant_msg = Message::assistant("Hi there!".to_string());
    assert_eq!(assistant_msg.content, "Hi there!");
    assert!(matches!(assistant_msg.role, MessageRole::Assistant));

    let system_msg = Message::system("You are helpful".to_string());
    assert_eq!(system_msg.content, "You are helpful");
    assert!(matches!(system_msg.role, MessageRole::System));

    println!("✓ Message creation working correctly");
}

fn test_anthropic_tool_handling() {
    use vtagent_core::llm::{AnthropicProvider, LLMProvider, LLMRequest, Message, MessageRole};

    let anthropic = AnthropicProvider::new("test_key".to_string());

    // Test that tool messages are properly converted for Anthropic
    let tool_message = Message {
        role: MessageRole::Tool,
        content: "Tool result content".to_string(),
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
        reasoning_effort: None,
    };

    // This should convert tool messages to user messages for Anthropic
    let result = anthropic.convert_to_anthropic_format(&request);
    assert!(result.is_ok(), "Should convert tool messages for Anthropic");

    println!("✓ Anthropic tool message handling working correctly");
}

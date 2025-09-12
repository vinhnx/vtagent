//! Demo of Universal LLM Provider System
//!
//! This example shows how the universal provider system works
//! with different AI providers through a unified interface.

use vtagent_core::llm::{
    AnthropicProvider, GeminiProvider, LLMFactory, Message, OpenAIProvider, UnifiedLLMClient,
    create_provider_for_model,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[DEMO] Universal LLM Provider System Demo");
    println!("=====================================");

    // Demo 1: Provider Factory
    println!("\n📦 Provider Factory Demo:");
    let factory = LLMFactory::new();

    println!("Available providers: {:?}", factory.available_providers());

    // Test provider detection
    println!("gpt-5 → {:?}", factory.provider_from_model("gpt-5"));
    println!(
        "claude-sonnet-4-20250514 → {:?}",
        factory.provider_from_model("claude-sonnet-4-20250514")
    );
    println!(
        "gemini-2.5-flash → {:?}",
        factory.provider_from_model("gemini-2.5-flash")
    );

    // Demo 2: Provider Creation
    println!("\nProvider Creation Demo:");

    let gemini = create_provider_for_model("gemini-2.5-flash", "test_key".to_string());
    println!("Gemini provider created: {}", gemini.is_ok());

    let openai = create_provider_for_model("gpt-5", "test_key".to_string());
    println!("OpenAI provider created: {}", openai.is_ok());

    let anthropic = create_provider_for_model("claude-sonnet-4-20250514", "test_key".to_string());
    println!("Anthropic provider created: {}", anthropic.is_ok());

    // Demo 3: Unified Client Demo
    println!("\n[CLIENT] Unified Client Demo:");

    let gemini_client =
        UnifiedLLMClient::new("gemini-2.5-flash".to_string(), "test_key".to_string());
    if let Ok(client) = gemini_client {
        println!(
            "[SUCCESS] Gemini client: {} ({})",
            client.model(),
            client.provider_name()
        );
    }

    let openai_client = UnifiedLLMClient::new("gpt-5".to_string(), "test_key".to_string());
    if let Ok(client) = openai_client {
        println!(
            "[SUCCESS] OpenAI client: {} ({})",
            client.model(),
            client.provider_name()
        );
    }

    let anthropic_client = UnifiedLLMClient::new(
        "claude-sonnet-4-20250514".to_string(),
        "test_key".to_string(),
    );
    if let Ok(client) = anthropic_client {
        println!(
            "[SUCCESS] Anthropic client: {} ({})",
            client.model(),
            client.provider_name()
        );
    }

    // Demo 4: Message Creation
    println!("\n[DEMO] Message Creation Demo:");

    let user_msg = Message::user("Hello, world!".to_string());
    println!("User message: '{}'", user_msg.content);

    let assistant_msg = Message::assistant("Hi there!".to_string());
    println!("Assistant message: '{}'", assistant_msg.content);

    let system_msg = Message::system("You are a helpful assistant".to_string());
    println!("System message: '{}'", system_msg.content);

    // Demo 5: Provider Capabilities
    println!("\n[CAPABILITIES] Provider Capabilities Demo:");

    let gemini = GeminiProvider::new("test_key".to_string());
    println!("Gemini models: {:?}", gemini.supported_models());

    let openai = OpenAIProvider::new("test_key".to_string());
    println!("OpenAI models: {:?}", openai.supported_models());

    let anthropic = AnthropicProvider::new("test_key".to_string());
    println!("Anthropic models: {:?}", anthropic.supported_models());

    println!("\n✨ Universal LLM Provider System Demo Complete!");
    println!("The system successfully provides:");
    println!("  • Multi-provider support (Gemini, OpenAI, Anthropic)");
    println!("  • Auto-detection from model names");
    println!("  • Unified interface for all providers");
    println!("  • Extensible architecture for new providers");

    Ok(())
}

//! Simple test to verify LMStudio connection
use vtagent_core::llm::factory::create_provider_with_config;
use vtagent_core::llm::provider::{LLMRequest, Message, MessageRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing LMStudio connection...");

    // Create LMStudio provider
    let provider = create_provider_with_config(
        "lmstudio",
        None, // No API key needed for local LMStudio
        Some("http://localhost:1234/v1".to_string()),
        Some("qwen3-4b-2507".to_string()),
    )?;

    // Create a simple request
    let request = LLMRequest {
        messages: vec![Message {
            role: MessageRole::User,
            content: "Say hello world".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        system_prompt: None,
        tools: None,
        model: "qwen3-4b-2507".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: false,
    };

    println!("Sending request to LMStudio...");

    // Send request
    match provider.generate(request).await {
        Ok(response) => {
            println!("Success! Response: {:?}", response.content);
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    Ok(())
}

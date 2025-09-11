//! Simple LMStudio Qwen test
//! This test verifies that we can connect to LMStudio and get a response from the Qwen model.

use reqwest;
use serde_json::json;

#[tokio::test]
async fn test_lmstudio_qwen_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Check if LMStudio is running
    let client = reqwest::Client::new();

    // Test 1: Check if models endpoint is accessible
    println!("🧪 Testing LMStudio connection...");
    let models_response = client.get("http://localhost:1234/v1/models").send().await?;

    assert!(models_response.status().is_success());
    println!("✅ LMStudio models endpoint is accessible");

    // Test 2: Check if Qwen model is available
    let models_json: serde_json::Value = models_response.json().await?;
    let models = models_json["data"].as_array().unwrap_or(&vec![]);

    let qwen_model_available = models.iter().any(|model| {
        model["id"]
            .as_str()
            .map(|id| id.contains("qwen"))
            .unwrap_or(false)
    });

    if !qwen_model_available {
        println!("⚠️  Qwen model not found in LMStudio - skipping Qwen-specific tests");
        return Ok(());
    }

    println!("✅ Qwen model is available in LMStudio");

    // Test 3: Send a simple completion request to Qwen model
    println!("📤 Sending completion request to Qwen model...");
    let completion_response = client
        .post("http://localhost:1234/v1/chat/completions")
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "qwen/qwen3-1.7b",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello! Please respond with just 'OK' if you can read this message."
                }
            ],
            "temperature": 0.7,
            "max_tokens": 100
        }))
        .send()
        .await?;

    assert!(completion_response.status().is_success());
    println!("✅ Got successful response from Qwen model");

    // Test 4: Verify response content
    let completion_json: serde_json::Value = completion_response.json().await?;
    let response_content = completion_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    assert!(!response_content.is_empty(), "Response content is empty");
    println!("✅ Received non-empty response: {}", response_content);

    println!("🎉 All LMStudio/Qwen connection tests passed!");
    Ok(())
}

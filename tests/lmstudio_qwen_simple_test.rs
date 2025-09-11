//! Simple LMStudio Qwen test
//!
//! This test verifies that we can connect to LMStudio and get a response from the Qwen model.

use reqwest;
use serde_json::json;
use std::time::Duration;

#[tokio::test]
async fn test_lmstudio_qwen_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Check if LMStudio is running
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Test 1: Check if models endpoint is accessible
    println!("[TEST] Testing LMStudio connection...");
    let models_response = client.get("http://localhost:1234/v1/models").send().await;

    if let Err(e) = models_response {
        println!("[WARNING] LMStudio is not running or not accessible: {}", e);
        println!("Please start LMStudio and ensure it's running on http://localhost:1234");
        return Ok(());
    }

    let models_response = models_response?;
    assert!(models_response.status().is_success());
    println!("[SUCCESS] LMStudio models endpoint is accessible");

    // Test 2: Check models response
    let models_json: serde_json::Value = models_response.json().await?;
    let models = models_json["data"].as_array().unwrap_or(&vec![]);

    println!("Found {} models in LMStudio", models.len());

    // Look for Qwen models
    let qwen_models: Vec<&str> = models
        .iter()
        .filter_map(|model| model["id"].as_str())
        .filter(|id| id.to_lowercase().contains("qwen"))
        .collect();

    if qwen_models.is_empty() {
        println!("[WARNING] No Qwen models found in LMStudio");
        println!("Available models:");
        for model in models.iter() {
            if let Some(id) = model["id"].as_str() {
                println!("  - {}", id);
            }
        }
        return Ok(());
    }

    println!("[SUCCESS] Found Qwen models: {:?}", qwen_models);

    // Test 3: Send a simple completion request to the first Qwen model
    let qwen_model = qwen_models[0];
    println!(
        "📤 Sending completion request to Qwen model: {}",
        qwen_model
    );

    let completion_response = client
        .post("http://localhost:1234/v1/chat/completions")
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": qwen_model,
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
        .await;

    if let Err(e) = completion_response {
        println!(
            "[ERROR] Failed to send completion request to {}: {}",
            qwen_model, e
        );
        return Err(e.into());
    }

    let completion_response = completion_response?;
    assert!(completion_response.status().is_success());
    println!("[SUCCESS] Got successful response from Qwen model: {}", qwen_model);

    // Test 4: Verify response content
    let completion_json: serde_json::Value = completion_response.json().await?;
    let response_content = completion_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    assert!(!response_content.is_empty(), "Response content is empty");
    println!(
        "[SUCCESS] Received non-empty response from {}: {}",
        qwen_model, response_content
    );

    println!("[SUCCESS] All LMStudio/Qwen connection tests passed!");
    Ok(())
}

/// Test that we can create a basic HTTP client for LMStudio
#[test]
fn test_lmstudio_http_client_creation() {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build();

    assert!(client.is_ok());
    println!("[SUCCESS] LMStudio HTTP client created successfully");
}

/// Test that we can parse JSON responses from LMStudio
#[test]
fn test_lmstudio_json_parsing() {
    let sample_response = json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "qwen/qwen3-1.7b",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! I'm Qwen, a large language model developed by Tongyi Lab."
                },
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    });

    let response_content = sample_response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    assert!(!response_content.is_empty());
    assert!(response_content.contains("Qwen"));
    println!(
        "[SUCCESS] Sample LMStudio JSON response parsed successfully: {}",
        response_content
    );
}

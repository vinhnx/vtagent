//! Simple LMStudio Qwen test
//! This test verifies that we can connect to LMStudio and get a response from the Qwen model.

use reqwest;
use serde_json::json;

#[tokio::test]
async fn test_lmstudio_qwen_connection() -> Result<(), Box<dyn std::error::Error>> {
    // Check if LMStudio is running by attempting to connect to its API
    let client = reqwest::Client::new();

    // Test 1: Check if models endpoint is accessible
    println!("[TEST] Testing LMStudio connection...");
    let models_response = client.get("http://localhost:1234/v1/models").send().await?;

    assert!(models_response.status().is_success());
    println!("[SUCCESS] LMStudio models endpoint is accessible");

    // Test 2: Check if Qwen model is available
    let models_json: serde_json::Value = models_response.json().await?;
    let models = models_json["data"].as_array().unwrap_or(&vec![]);

    let qwen_model_available = models.iter().any(|model| {
        model["id"]
            .as_str()
            .map(|id| id.contains("qwen") || id.contains("Qwen"))
            .unwrap_or(false)
    });

    if !qwen_model_available {
        println!("[WARNING] Qwen model not found in LMStudio - skipping Qwen-specific tests");
        return Ok(());
    }

    println!("[SUCCESS] Qwen model is available in LMStudio");

    // Test 3: Send a simple completion request to Qwen model
    println!("📤 Sending completion request to Qwen model...");

    // Try different possible Qwen model names
    let qwen_models = vec![
        "qwen/qwen3-1.7b",
        "qwen3-1.7b",
        "qwen/qwen3-7b",
        "qwen3-7b",
        "qwen/qwen2-7b",
        "qwen2-7b",
    ];

    let mut success = false;
    let mut last_error = String::new();

    for model_name in &qwen_models {
        match client
            .post("http://localhost:1234/v1/chat/completions")
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": model_name,
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
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    println!("[SUCCESS] Got successful response from Qwen model: {}", model_name);

                    // Try to parse the response
                    match response.json::<serde_json::Value>().await {
                        Ok(json_response) => {
                            let response_content = json_response["choices"][0]["message"]["content"]
                                .as_str()
                                .unwrap_or("");

                            if !response_content.is_empty() {
                                println!("[SUCCESS] Received non-empty response from {}: {}", model_name, response_content);
                                success = true;
                                break;
                            } else {
                                last_error = format!("Empty response from model: {}", model_name);
                            }
                        }
                        Err(e) => {
                            last_error = format!("Failed to parse response from {}: {}", model_name, e);
                        }
                    }
                } else {
                    last_error = format!("HTTP {} error from model: {}", response.status(), model_name);
                }
            }
            Err(e) => {
                last_error = format!("Failed to send request to {}: {}", model_name, e);
            }
        }
    }

    if !success {
        println!(
            "[ERROR] Failed to get response from any Qwen model. Last error: {}",
            last_error
        );
        return Err(last_error.into());
    }

    println!("[SUCCESS] All LMStudio/Qwen connection tests passed!");
    Ok(())
}

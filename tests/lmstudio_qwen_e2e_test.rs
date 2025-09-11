//! End-to-end test for LMStudio with Qwen model
//! 
//! This test validates that the agent loop works correctly with LMStudio
//! using the Qwen/qwen3-1.7b model.

use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::{AgentType, MultiAgentConfig};
use vtagent_core::config::models::ModelId;
use vtagent_core::llm::factory::{LLMFactory, ProviderConfig};
use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;

#[tokio::test]
async fn test_lmstudio_qwen_agent_loop() -> Result<()> {
    // Skip test if LMStudio is not running
    if !is_lmstudio_running().await {
        println!("LMStudio is not running. Skipping test.");
        return Ok(());
    }

    println!("Starting LMStudio Qwen E2E Test");
    println!("===============================");

    // Create configuration for LMStudio with Qwen model
    let config = MultiAgentConfig {
        enable_multi_agent: true,
        enable_task_management: true,
        enable_context_sharing: true,
        enable_performance_monitoring: true,
        orchestrator_model: "qwen/qwen3-1.7b".to_string(),
        subagent_model: "qwen/qwen3-1.7b".to_string(),
        max_concurrent_subagents: 2,
        task_timeout: Duration::from_secs(30),
        context_window_size: 4096,
        max_context_items: 50,
        ..Default::default()
    };

    // Use local model (no API key needed for LMStudio local models)
    let api_key = "".to_string();
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    println!("Creating MultiAgentSystem with Qwen model...");
    
    // Create multi-agent system
    let mut system = MultiAgentSystem::new(config, api_key, workspace).await?;

    println!("System created successfully!");

    // Test 1: Simple greeting task
    println!("\n--- Test 1: Simple Greeting ---");
    let result = system
        .execute_task_optimized(
            "Greeting Test".to_string(),
            "Hello, how are you? Please respond briefly.".to_string(),
            AgentType::Coder,
        )
        .await;

    match result {
        Ok(task_result) => {
            println!("✅ Task completed successfully!");
            println!("Task ID: {}", task_result.task_id);
            println!("Execution time: {:?}", task_result.execution_time);
            println!("Agent used: {}", task_result.agent_used);
            println!("Response: {}", task_result.results.summary);
            
            // Verify the response is not empty
            assert!(!task_result.results.summary.trim().is_empty());
            assert!(task_result.results.summary.len() > 10); // Minimum reasonable response
        }
        Err(e) => {
            eprintln!("❌ Task failed: {}", e);
            return Err(e.into());
        }
    }

    // Test 2: Coding task
    println!("\n--- Test 2: Simple Coding Task ---");
    let result = system
        .execute_task_optimized(
            "Simple Coding Task".to_string(),
            "Write a simple Python function that adds two numbers together. Just provide the function.".to_string(),
            AgentType::Coder,
        )
        .await;

    match result {
        Ok(task_result) => {
            println!("✅ Coding task completed successfully!");
            println!("Task ID: {}", task_result.task_id);
            println!("Execution time: {:?}", task_result.execution_time);
            println!("Response: {}", task_result.results.summary);
            
            // Verify the response contains Python code
            let response_lower = task_result.results.summary.to_lowercase();
            assert!(
                response_lower.contains("def ") || 
                response_lower.contains("def") ||
                response_lower.contains("function"),
                "Response should contain function definition"
            );
        }
        Err(e) => {
            eprintln!("❌ Coding task failed: {}", e);
            return Err(e.into());
        }
    }

    // Test 3: Explanation task
    println!("\n--- Test 3: Explanation Task ---");
    let result = system
        .execute_task_optimized(
            "Explanation Task".to_string(),
            "Explain what a neural network is in simple terms.".to_string(),
            AgentType::Explorer,
        )
        .await;

    match result {
        Ok(task_result) => {
            println!("✅ Explanation task completed successfully!");
            println!("Task ID: {}", task_result.task_id);
            println!("Execution time: {:?}", task_result.execution_time);
            println!("Response preview: {}", 
                if task_result.results.summary.len() > 200 {
                    format!("{}...", &task_result.results.summary[..200])
                } else {
                    task_result.results.summary.clone()
                }
            );
            
            // Verify the response is substantial
            assert!(!task_result.results.summary.trim().is_empty());
            assert!(task_result.results.summary.len() > 50); // Minimum reasonable explanation
        }
        Err(e) => {
            eprintln!("❌ Explanation task failed: {}", e);
            return Err(e.into());
        }
    }

    // Show final performance report
    let status_report = system.get_status_report().await;
    println!("\n--- Final Performance Report ---");
    println!("Uptime: {:?}", status_report.uptime);
    println!("Active tasks: {}", status_report.active_tasks);
    println!("Completed tasks: {}", status_report.completed_tasks);
    println!("Recommendations: {}", status_report.recommendations.len());

    // Shutdown system
    println!("\nShutting down system...");
    system.shutdown().await?;

    println!("✅ All tests completed successfully!");
    Ok(())
}

/// Check if LMStudio is running by attempting to connect to its API
async fn is_lmstudio_running() -> bool {
    use reqwest::Client;
    
    let client = Client::new();
    let url = "http://localhost:1234/v1/models";
    
    match client.get(url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Test LMStudio provider creation and basic functionality
#[test]
fn test_lmstudio_provider_creation() {
    use vtagent_core::llm::llm_modular::providers::LMStudioProvider;
    use vtagent_core::llm::client::LLMClient;
    use vtagent_core::llm::types::BackendKind;
    
    let provider = LMStudioProvider::local("qwen/qwen3-1.7b".to_string());
    
    assert_eq!(provider.backend_kind(), BackendKind::LMStudio);
    assert_eq!(provider.model_id(), "qwen/qwen3-1.7b");
    
    println!("✅ LMStudio provider created successfully");
}

/// Test that we can create a MultiAgentSystem with LMStudio configuration
#[tokio::test]
async fn test_lmstudio_multisystem_creation() -> Result<()> {
    let config = MultiAgentConfig {
        enable_multi_agent: true,
        enable_task_management: true,
        enable_context_sharing: true,
        enable_performance_monitoring: true,
        orchestrator_model: "qwen/qwen3-1.7b".to_string(),
        subagent_model: "qwen/qwen3-1.7b".to_string(),
        max_concurrent_subagents: 1,
        task_timeout: Duration::from_secs(10),
        context_window_size: 2048,
        max_context_items: 25,
        ..Default::default()
    };

    let api_key = "".to_string(); // No API key needed for local models
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // This should succeed without errors
    let system = MultiAgentSystem::new(config, api_key, workspace).await?;
    
    println!("✅ MultiAgentSystem with LMStudio created successfully");
    Ok(())
}
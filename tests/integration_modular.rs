//! Integration tests for modular architecture
//!
//! These tests validate that all refactored modules work together correctly
//! and maintain backward compatibility.

use vtagent_core::{
    // Test code_completion module exports
    code_completion::{CompletionContext, CompletionEngine, CompletionKind},

    // Test code_quality module exports
    code_quality::{FormattingOrchestrator, LintingOrchestrator, QualityMetrics},
    // Test config module exports
    config::{ConfigManager, ToolPolicy, VTAgentConfig},

    // Test gemini module exports
    gemini::{Client, ClientConfig, Content, GenerateContentRequest},
};

#[test]
fn test_gemini_module_integration() {
    // Test that we can create a Gemini client with different configurations
    let client = Client::new("test_key".to_string(), "gemini-2.5-flash".to_string());
    assert_eq!(client.config().user_agent, "vtagent/1.0.0");

    // Test different client configurations
    let high_throughput_config = ClientConfig::high_throughput();
    assert_eq!(high_throughput_config.pool_max_idle_per_host, 20);

    let low_memory_config = ClientConfig::low_memory();
    assert_eq!(low_memory_config.pool_max_idle_per_host, 3);
}

#[test]
fn test_config_module_integration() {
    // Test that we can create and use configurations
    let config = VTAgentConfig::default();
    assert_eq!(config.agent.provider, "gemini");
    assert_eq!(config.tools.default_policy, ToolPolicy::Prompt);

    // Test that we can load configuration (will use defaults if no file)
    let manager = ConfigManager::load().unwrap();
    let loaded_config = manager.config();
    assert_eq!(loaded_config.agent.provider, "gemini");
}

#[test]
fn test_code_completion_integration() {
    // Test that we can create completion engine and context
    let engine = CompletionEngine::new();

    let context = CompletionContext::new(10, 5, "fn test".to_string(), "rust".to_string());

    assert!(context.is_completion_suitable());
    assert_eq!(context.language, "rust");
}

#[test]
fn test_code_quality_integration() {
    // Test that we can create orchestrators
    let formatting = FormattingOrchestrator::new();
    let linting = LintingOrchestrator::new();

    // Test quality metrics
    let mut metrics = QualityMetrics::default();
    metrics.total_files = 10;
    metrics.formatted_files = 8;
    metrics.lint_errors = 2;

    let score = metrics.quality_score();
    assert!(score > 0.0 && score <= 100.0);
}

#[test]
fn test_backward_compatibility() {
    // Test that all the old import patterns still work
    use vtagent_core::code_completion::CompletionEngine;
    use vtagent_core::code_quality::FormattingOrchestrator;
    use vtagent_core::config::VTAgentConfig;
    use vtagent_core::gemini::Client;

    // These should all compile and work as before
    let _client = Client::new("key".to_string(), "model".to_string());
    let _config = VTAgentConfig::default();
    let _engine = CompletionEngine::new();
    let _formatter = FormattingOrchestrator::new();
}

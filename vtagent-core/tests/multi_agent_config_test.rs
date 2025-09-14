//! Test for multi-agent configuration loading and parsing

use vtagent_core::config::VTAgentConfig;

#[test]
fn test_multi_agent_config_loading() {
    // Test that we can load the configuration with multi-agent settings
    let config = VTAgentConfig::default();

    // Test basic multi-agent settings
    assert_eq!(config.multi_agent.enabled, true); // Default should be true
    assert_eq!(config.multi_agent.use_single_model, true); // Default should be true

    // Test models
    assert_eq!(
        config.multi_agent.orchestrator_model,
        "gemini-2.5-flash-lite"
    );
    assert_eq!(config.multi_agent.executor_model, "");

    // Test performance settings
    assert_eq!(config.multi_agent.max_concurrent_subagents, 3);
    assert_eq!(config.multi_agent.context_sharing_enabled, true);
    assert_eq!(config.multi_agent.task_timeout_seconds, 300);
}

#[test]
fn test_multi_agent_config_with_custom_values() {
    // Test that we can create a config with custom multi-agent values
    let mut config = VTAgentConfig::default();

    // Modify some values
    config.multi_agent.enabled = true;
    config.multi_agent.use_single_model = false;
    config.multi_agent.orchestrator_model = "test/orchestrator-model".to_string();
    config.multi_agent.executor_model = "test/executor-model".to_string();
    config.multi_agent.max_concurrent_subagents = 5;
    config.multi_agent.task_timeout_seconds = 600;

    // Verify the changes
    assert_eq!(config.multi_agent.enabled, true);
    assert_eq!(config.multi_agent.use_single_model, false);
    assert_eq!(
        config.multi_agent.orchestrator_model,
        "test/orchestrator-model"
    );
    assert_eq!(config.multi_agent.executor_model, "test/executor-model");
    assert_eq!(config.multi_agent.max_concurrent_subagents, 5);
    assert_eq!(config.multi_agent.task_timeout_seconds, 600);
}

//! Test for multi-agent configuration loading and parsing

use vtagent_core::config::VTAgentConfig;

#[test]
fn test_multi_agent_config_loading() {
    // Test that we can load the configuration with multi-agent settings
    let config = VTAgentConfig::default();

    // Test basic multi-agent settings
    assert_eq!(config.multi_agent.enabled, false); // Default should be false
    assert_eq!(config.multi_agent.max_agents, 5); // Default should be 5

    // Test execution mode
    match config.multi_agent.execution_mode {
        vtagent_core::config::multi_agent::ExecutionMode::Auto => {} // Expected default
        _ => panic!("Default execution mode should be Auto"),
    }

    // Test models
    assert_eq!(config.multi_agent.orchestrator_model, "qwen/qwen3-4b-2507");
    assert_eq!(config.multi_agent.subagent_model, "qwen/qwen3-4b-2507");

    // Test performance settings
    assert_eq!(config.multi_agent.max_concurrent_subagents, 3);
    assert_eq!(config.multi_agent.context_store_enabled, true);

    // Test strategies
    match config.multi_agent.verification_strategy {
        vtagent_core::config::multi_agent::VerificationStrategy::Always => {} // Expected default
        _ => panic!("Default verification strategy should be Always"),
    }

    match config.multi_agent.delegation_strategy {
        vtagent_core::config::multi_agent::DelegationStrategy::Adaptive => {} // Expected default
        _ => panic!("Default delegation strategy should be Adaptive"),
    }

    // Test context store configuration
    assert_eq!(config.multi_agent.context_store.max_context_size, 100000);
    assert_eq!(config.multi_agent.context_store.compression_enabled, true);
    assert_eq!(config.multi_agent.context_store.max_contexts, 1000);
    assert_eq!(config.multi_agent.context_store.auto_cleanup_days, 7);
    assert_eq!(config.multi_agent.context_store.enable_persistence, true);
    assert_eq!(
        config.multi_agent.context_store.storage_dir,
        ".vtagent/contexts"
    );

    // Test agent-specific configurations
    assert!(config.multi_agent.agents.by_type.is_empty()); // Should be empty by default
}

#[test]
fn test_multi_agent_config_with_custom_values() {
    // Test that we can create a config with custom multi-agent values
    let mut config = VTAgentConfig::default();

    // Modify some values
    config.multi_agent.enabled = true;
    config.multi_agent.execution_mode = vtagent_core::config::multi_agent::ExecutionMode::Multi;
    config.multi_agent.orchestrator_model = "test/orchestrator-model".to_string();
    config.multi_agent.subagent_model = "test/subagent-model".to_string();
    config.multi_agent.max_concurrent_subagents = 5;
    config.multi_agent.verification_strategy =
        vtagent_core::config::multi_agent::VerificationStrategy::Never;
    config.multi_agent.delegation_strategy =
        vtagent_core::config::multi_agent::DelegationStrategy::Aggressive;

    // Verify the changes
    assert_eq!(config.multi_agent.enabled, true);
    match config.multi_agent.execution_mode {
        vtagent_core::config::multi_agent::ExecutionMode::Multi => {} // Expected
        _ => panic!("Execution mode should be Multi"),
    }
    assert_eq!(
        config.multi_agent.orchestrator_model,
        "test/orchestrator-model"
    );
    assert_eq!(config.multi_agent.subagent_model, "test/subagent-model");
    assert_eq!(config.multi_agent.max_concurrent_subagents, 5);
    match config.multi_agent.verification_strategy {
        vtagent_core::config::multi_agent::VerificationStrategy::Never => {} // Expected
        _ => panic!("Verification strategy should be Never"),
    }
    match config.multi_agent.delegation_strategy {
        vtagent_core::config::multi_agent::DelegationStrategy::Aggressive => {} // Expected
        _ => panic!("Delegation strategy should be Aggressive"),
    }
}

//! Comprehensive integration tests for the snapshot checkpoint system
//!
//! This module tests the complete snapshot workflow including:
//! - Snapshot creation and loading
//! - Revert functionality with different scopes
//! - Encryption and decryption
//! - Cleanup mechanisms
//! - Error handling and edge cases

use std::collections::HashMap;
use tempfile::TempDir;
use tokio::test;

use vtagent::agent::snapshots::*;
use vtagent::gemini::Content;
use vtagent::types::*;

/// Test basic snapshot creation and loading
#[tokio::test]
async fn test_snapshot_creation_and_loading() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    // Create snapshot manager
    let manager = SnapshotManager::new(snapshots_dir.clone(), 10);

    // Create mock snapshot data
    let conversation_history = vec![
        Content::user_text("Hello, agent!"),
        Content::system_text("Hello! How can I help you?"),
    ];

    let snapshot = AgentSnapshot {
        metadata: SnapshotMetadata {
            id: "test_snapshot_1".to_string(),
            turn_number: 1,
            timestamp: 1234567890,
            description: "Test snapshot".to_string(),
            version: "1.0".to_string(),
            compressed: false,
            encrypted: false,
            size_bytes: 1000,
        },
        config: AgentConfigSnapshot {
            model: "test-model".to_string(),
            api_key_masked: true,
            workspace: temp_dir.path().to_path_buf(),
            verbose: false,
        },
        session_info: SessionInfo {
            session_id: "test_session".to_string(),
            start_time: 1234567890,
            total_turns: 1,
            total_decisions: 0,
            error_count: 0,
        },
        conversation_history: conversation_history.clone(),
        decision_tracker: DecisionTrackerSnapshot {
            total_decisions: 0,
            successful_decisions: 0,
            failed_decisions: 0,
            recent_decisions: Vec::new(),
            available_tools: Vec::new(),
        },
        error_recovery: ErrorRecoverySnapshot {
            total_errors: 0,
            resolved_errors: 0,
            error_patterns: Vec::new(),
            recovery_attempts: Vec::new(),
        },
        summarizer: SummarizerSnapshot {
            total_summaries: 0,
            latest_summary: None,
            summary_history: Vec::new(),
        },
        compaction_engine: CompactionEngineSnapshot {
            total_compactions: 0,
            memory_saved: 0,
            compression_ratio: 1.0,
            compaction_suggestions: Vec::new(),
        },
        tree_sitter_state: TreeSitterSnapshot {
            supported_languages: vec!["rust".to_string(), "python".to_string()],
            cache_size: 100,
        },
        tool_registry: ToolRegistrySnapshot {
            available_tools: vec!["list_files".to_string(), "read_file".to_string()],
            tool_usage_stats: HashMap::new(),
        },
        environment: HashMap::new(),
        performance_metrics: PerformanceMetrics {
            session_duration_seconds: 60,
            total_api_calls: 1,
            total_tokens_used: Some(100),
            average_response_time_ms: 500.0,
            tool_execution_count: 0,
            error_count: 0,
            recovery_success_rate: 1.0,
        },
        checksum: String::new(),
    };

    // Calculate checksum and save
    let json_data = serde_json::to_string_pretty(&snapshot).unwrap();
    let checksum = manager.calculate_checksum(&json_data);
    let mut snapshot = snapshot;
    snapshot.checksum = checksum;

    let final_json = serde_json::to_string_pretty(&snapshot).unwrap();
    manager.save_snapshot("test_snapshot_1", &final_json, false).await.unwrap();

    // Load snapshot
    let loaded_snapshot = manager
        .load_snapshot("test_snapshot_1")
        .await
        .expect("Failed to load snapshot");

/// Test revert scope parsing
#[test]
fn test_revert_scope_parsing() {
    assert_eq!(SnapshotManager::parse_revert_scope("memory"), Some(RevertScope::Memory));
    assert_eq!(SnapshotManager::parse_revert_scope("context"), Some(RevertScope::Context));
    assert_eq!(SnapshotManager::parse_revert_scope("full"), Some(RevertScope::Full));
    assert_eq!(SnapshotManager::parse_revert_scope("invalid"), None);
    assert_eq!(SnapshotManager::parse_revert_scope("MEMORY"), Some(RevertScope::Memory));
}

/// Test checksum verification
#[tokio::test]
async fn test_checksum_verification() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir.clone(), 10);

    let snapshot = AgentSnapshot {
        metadata: SnapshotMetadata {
            id: "test_checksum".to_string(),
            turn_number: 1,
            timestamp: 1234567890,
            description: "Test checksum".to_string(),
            version: "1.0".to_string(),
            compressed: false,
            encrypted: false,
            size_bytes: 1000,
        },
        config: AgentConfigSnapshot {
            model: "test-model".to_string(),
            api_key_masked: true,
            workspace: temp_dir.path().to_path_buf(),
            verbose: false,
        },
        session_info: SessionInfo {
            session_id: "test_session".to_string(),
            start_time: 1234567890,
            total_turns: 1,
            total_decisions: 0,
            error_count: 0,
        },
        conversation_history: vec![Content::user_text("Test message")],
        decision_tracker: DecisionTrackerSnapshot {
            total_decisions: 0,
            successful_decisions: 0,
            failed_decisions: 0,
            recent_decisions: Vec::new(),
            available_tools: Vec::new(),
        },
        error_recovery: ErrorRecoverySnapshot {
            total_errors: 0,
            resolved_errors: 0,
            error_patterns: Vec::new(),
            recovery_attempts: Vec::new(),
        },
        summarizer: SummarizerSnapshot {
            total_summaries: 0,
            latest_summary: None,
            summary_history: Vec::new(),
        },
        compaction_engine: CompactionEngineSnapshot {
            total_compactions: 0,
            memory_saved: 0,
            compression_ratio: 1.0,
            compaction_suggestions: Vec::new(),
        },
        tree_sitter_state: TreeSitterSnapshot {
            supported_languages: vec!["rust".to_string()],
            cache_size: 100,
        },
        tool_registry: ToolRegistrySnapshot {
            available_tools: vec!["list_files".to_string()],
            tool_usage_stats: HashMap::new(),
        },
        environment: HashMap::new(),
        performance_metrics: PerformanceMetrics {
            session_duration_seconds: 60,
            total_api_calls: 1,
            total_tokens_used: Some(100),
            average_response_time_ms: 500.0,
            tool_execution_count: 0,
            error_count: 0,
            recovery_success_rate: 1.0,
        },
        checksum: String::new(),
    };

    // Calculate checksum and save
    let json_data = serde_json::to_string_pretty(&snapshot).unwrap();
    let checksum = manager.calculate_checksum(&json_data);
    let mut snapshot = snapshot;
    snapshot.checksum = checksum;

    let final_json = serde_json::to_string_pretty(&snapshot).unwrap();
    manager.save_snapshot("test_checksum", &final_json, false).await.unwrap();

    // Load snapshot
    let loaded_snapshot = manager
        .load_snapshot("test_checksum")
        .await
        .expect("Failed to load snapshot");

    // Verify checksum is valid
    assert_eq!(loaded_snapshot.checksum, checksum);
}

/// Test error handling for missing snapshots
#[tokio::test]
async fn test_missing_snapshot_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir, 10);

    // Try to load non-existent snapshot
    let result = manager.load_snapshot("non_existent_snapshot").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

/// Test snapshot encryption
#[tokio::test]
async fn test_snapshot_encryption() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");
    let password = "test_password_123";

    // Create snapshot manager with encryption
    let manager = SnapshotManager::new(snapshots_dir.clone(), 10)
        .with_encryption_key(password.to_string());

    let agent_config = AgentConfig {
        model: "test-model".to_string(),
        api_key: "sensitive-api-key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };

    let conversation_history = vec![
        Content::user_text("Secret conversation"),
        Content::system_text("Classified response"),
    ];

    // Create encrypted snapshot
    let snapshot_id = manager
        .create_snapshot(&agent_config, &conversation_history, 1, "Encrypted snapshot")
        .await
        .expect("Failed to create encrypted snapshot");

    // Verify encrypted file exists
    let encrypted_path = snapshots_dir.join(format!("{}.json", snapshot_id));
    assert!(encrypted_path.exists(), "Encrypted snapshot file should exist");

    // Load encrypted snapshot
    let loaded_snapshot = manager
        .load_snapshot(&snapshot_id)
        .await
        .expect("Failed to load encrypted snapshot");

    // Verify contents
    assert!(loaded_snapshot.metadata.encrypted);
    assert_eq!(loaded_snapshot.conversation_history.len(), 2);
}

/// Test snapshot cleanup
#[tokio::test]
async fn test_snapshot_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    // Create snapshot manager with low limit
    let mut manager = SnapshotManager::new(snapshots_dir.clone(), 3);

    let agent_config = AgentConfig {
        model: "test-model".to_string(),
        api_key: "test-key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };

    // Create multiple snapshots
    for i in 1..=5 {
        let conversation_history = vec![Content::user_text(format!("Message {}", i))];
        manager
            .create_snapshot(&agent_config, &conversation_history, i, &format!("Snapshot {}", i))
            .await
            .expect(&format!("Failed to create snapshot {}", i));
    }

    // List snapshots
    let snapshots = manager.list_snapshots().await.expect("Failed to list snapshots");

    // Should only keep the 3 most recent snapshots
    assert_eq!(snapshots.len(), 3);

    // Verify they are the most recent ones
    assert!(snapshots.iter().all(|s| s.turn_number >= 3));
}

/// Test revert scope parsing
#[test]
fn test_revert_scope_parsing() {
    assert_eq!(SnapshotManager::parse_revert_scope("memory"), Some(RevertScope::Memory));
    assert_eq!(SnapshotManager::parse_revert_scope("context"), Some(RevertScope::Context));
    assert_eq!(SnapshotManager::parse_revert_scope("full"), Some(RevertScope::Full));
    assert_eq!(SnapshotManager::parse_revert_scope("invalid"), None);
    assert_eq!(SnapshotManager::parse_revert_scope("MEMORY"), Some(RevertScope::Memory));
}

/// Test checksum verification
#[tokio::test]
async fn test_checksum_verification() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir.clone(), 10);

    let agent_config = AgentConfig {
        model: "test-model".to_string(),
        api_key: "test-key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };

    let conversation_history = vec![Content::user_text("Test message")];

    // Note: In a real implementation, we would need an actual Agent instance
    // For testing purposes, we'll create a mock snapshot manually
    let snapshot = AgentSnapshot {
        metadata: SnapshotMetadata {
            id: "test_snapshot_1".to_string(),
            turn_number: 1,
            timestamp: 1234567890,
            description: "Test snapshot".to_string(),
            version: "1.0".to_string(),
            compressed: false,
            encrypted: false,
            size_bytes: 1000,
        },
        config: AgentConfigSnapshot {
            model: "test-model".to_string(),
            api_key_masked: true,
            workspace: temp_dir.path().to_path_buf(),
            verbose: false,
        },
        session_info: SessionInfo {
            session_id: "test_session".to_string(),
            start_time: 1234567890,
            total_turns: 1,
            total_decisions: 0,
            error_count: 0,
        },
        conversation_history: conversation_history.clone(),
        decision_tracker: DecisionTrackerSnapshot {
            total_decisions: 0,
            successful_decisions: 0,
            failed_decisions: 0,
            recent_decisions: Vec::new(),
            available_tools: Vec::new(),
        },
        error_recovery: ErrorRecoverySnapshot {
            total_errors: 0,
            resolved_errors: 0,
            error_patterns: Vec::new(),
            recovery_attempts: Vec::new(),
        },
        summarizer: SummarizerSnapshot {
            total_summaries: 0,
            latest_summary: None,
            summary_history: Vec::new(),
        },
        compaction_engine: CompactionEngineSnapshot {
            total_compactions: 0,
            memory_saved: 0,
            compression_ratio: 1.0,
            compaction_suggestions: Vec::new(),
        },
        tree_sitter_state: TreeSitterSnapshot {
            supported_languages: vec!["rust".to_string(), "python".to_string()],
            cache_size: 100,
        },
        tool_registry: ToolRegistrySnapshot {
            available_tools: vec!["list_files".to_string(), "read_file".to_string()],
            tool_usage_stats: HashMap::new(),
        },
        environment: HashMap::new(),
        performance_metrics: PerformanceMetrics {
            session_duration_seconds: 60,
            total_api_calls: 1,
            total_tokens_used: Some(100),
            average_response_time_ms: 500.0,
            tool_execution_count: 0,
            error_count: 0,
            recovery_success_rate: 1.0,
        },
        checksum: String::new(),
    };

    // Calculate checksum and save
    let json_data = serde_json::to_string_pretty(&snapshot).unwrap();
    let checksum = manager.calculate_checksum(&json_data);
    let mut snapshot = snapshot;
    snapshot.checksum = checksum;

    let final_json = serde_json::to_string_pretty(&snapshot).unwrap();
    manager.save_snapshot("test_snapshot_1", &final_json, false).await.unwrap();

    let snapshot_id = "test_snapshot_1".to_string();

    // Load snapshot
    let mut loaded_snapshot = manager
        .load_snapshot(&snapshot_id)
        .await
        .expect("Failed to load snapshot");

    // Tamper with checksum
    loaded_snapshot.checksum = "invalid_checksum".to_string();

    // Save tampered snapshot
    let tampered_json = serde_json::to_string_pretty(&loaded_snapshot).unwrap();
    let snapshot_path = snapshots_dir.join(format!("{}.json", snapshot_id));
    std::fs::write(&snapshot_path, &tampered_json).unwrap();

    // Attempt to load should fail checksum verification
    let result = manager.load_snapshot(&snapshot_id).await;
    assert!(result.is_err(), "Should fail checksum verification");
    assert!(result.unwrap_err().to_string().contains("checksum"));
}

/// Test concurrent snapshot operations
#[tokio::test]
async fn test_concurrent_snapshot_operations() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = Arc::new(SnapshotManager::new(snapshots_dir.clone(), 20));

    let agent_config = AgentConfig {
        model: "test-model".to_string(),
        api_key: "test-key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };

    // Spawn multiple tasks creating snapshots concurrently
    let mut handles = Vec::new();
    for i in 1..=10 {
        let manager_clone = Arc::clone(&manager);
        let config_clone = agent_config.clone();
        let history = vec![Content::user_text(format!("Concurrent message {}", i))];

        let handle = tokio::spawn(async move {
            manager_clone
                .create_snapshot(&config_clone, &history, i, &format!("Concurrent snapshot {}", i))
                .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap().expect("Concurrent snapshot creation failed");
    }

    // Verify all snapshots were created
    let snapshots = manager.list_snapshots().await.expect("Failed to list snapshots");
    assert_eq!(snapshots.len(), 10);
}

/// Test snapshot metadata validation
#[tokio::test]
async fn test_snapshot_metadata_validation() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir.clone(), 10);

    let agent_config = AgentConfig {
        model: "test-model".to_string(),
        api_key: "test-key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };

    let conversation_history = vec![Content::user_text("Test message")];

    // Create snapshot
    let snapshot_id = manager
        .create_snapshot(&agent_config, &conversation_history, 1, "Test snapshot")
        .await
        .expect("Failed to create snapshot");

    // Load and verify metadata
    let snapshot = manager
        .load_snapshot(&snapshot_id)
        .await
        .expect("Failed to load snapshot");

    assert_eq!(snapshot.metadata.version, "1.0");
    assert!(!snapshot.metadata.id.is_empty());
    assert!(snapshot.metadata.timestamp > 0);
    assert!(snapshot.metadata.size_bytes > 0);
    assert!(!snapshot.metadata.compressed); // Should not be compressed for small snapshot
    assert!(!snapshot.metadata.encrypted); // Should not be encrypted without key
}

/// Test error handling for missing snapshots
#[tokio::test]
async fn test_missing_snapshot_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir, 10);

    // Try to load non-existent snapshot
    let result = manager.load_snapshot("non_existent_snapshot").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

/// Test environment variable extraction
#[test]
fn test_environment_variable_extraction() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir, 10);

    // Set some test environment variables
    std::env::set_var("TEST_VAR", "test_value");
    std::env::set_var("HOME", "/home/test");
    std::env::set_var("USER", "testuser");

    let env_vars = manager.extract_safe_environment();

    // Should include safe variables
    assert!(env_vars.contains_key("HOME"));
    assert!(env_vars.contains_key("USER"));
    assert!(env_vars.contains_key("TEST_VAR"));

    // Should not include sensitive variables (if any were set)
    // This is a basic test - in practice, more filtering would be needed

    // Clean up
    std::env::remove_var("TEST_VAR");
}

/// Integration test for full snapshot workflow
#[tokio::test]
async fn test_full_snapshot_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let snapshots_dir = temp_dir.path().join("snapshots");

    let manager = SnapshotManager::new(snapshots_dir.clone(), 5);

    let agent_config = AgentConfig {
        model: "test-model".to_string(),
        api_key: "test-key".to_string(),
        workspace: temp_dir.path().to_path_buf(),
        verbose: false,
    };

    // Step 1: Create initial snapshot
    let conversation_1 = vec![Content::user_text("Initial message")];
    let snapshot_1 = manager
        .create_snapshot(&agent_config, &conversation_1, 1, "Initial state")
        .await
        .expect("Failed to create initial snapshot");

    // Step 2: Create second snapshot
    let conversation_2 = vec![
        Content::user_text("First message"),
        Content::system_text("First response"),
        Content::user_text("Second message"),
    ];
    let snapshot_2 = manager
        .create_snapshot(&agent_config, &conversation_2, 2, "After first interaction")
        .await
        .expect("Failed to create second snapshot");

    // Step 3: List snapshots
    let snapshots = manager.list_snapshots().await.expect("Failed to list snapshots");
    assert_eq!(snapshots.len(), 2);

    // Step 4: Load and verify snapshots
    let loaded_1 = manager.load_snapshot(&snapshot_1).await.expect("Failed to load first snapshot");
    let loaded_2 = manager.load_snapshot(&snapshot_2).await.expect("Failed to load second snapshot");

    assert_eq!(loaded_1.metadata.turn_number, 1);
    assert_eq!(loaded_2.metadata.turn_number, 2);
    assert_eq!(loaded_1.conversation_history.len(), 1);
    assert_eq!(loaded_2.conversation_history.len(), 3);

    // Step 5: Test cleanup
    let deleted_count = manager.cleanup_old_snapshots().await.expect("Failed to cleanup");
    assert_eq!(deleted_count, 0); // Should not delete any since we're under the limit

    // Step 6: Test revert scope parsing
    assert_eq!(SnapshotManager::parse_revert_scope("full"), Some(RevertScope::Full));
    assert_eq!(SnapshotManager::parse_revert_scope("memory"), Some(RevertScope::Memory));
    assert_eq!(SnapshotManager::parse_revert_scope("context"), Some(RevertScope::Context));
}
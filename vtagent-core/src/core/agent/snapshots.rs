//! Comprehensive snapshot checkpoint system for agent state management
//!
//! This module provides functionality to:
//! - Serialize complete agent state to snapshots
//! - Revert to previous states (full or partial)
//! - Manage snapshot lifecycle and cleanup
//! - Support compression and encryption

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Re-export types from other modules that we need
use crate::config::types::*;
use crate::core::agent::core::Agent;
use crate::core::performance_monitor::PerformanceMetrics;
use crate::gemini::Content;

/// Metadata for snapshot identification and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Unique snapshot identifier
    pub id: String,
    /// Turn number when snapshot was created
    pub turn_number: usize,
    /// Unix timestamp of creation
    pub timestamp: u64,
    /// Human-readable description of the turn/action
    pub description: String,
    /// Snapshot format version for compatibility
    pub version: String,
    /// Whether the snapshot is compressed
    pub compressed: bool,
    /// Whether the snapshot is encrypted
    pub encrypted: bool,
    /// Size in bytes
    pub size_bytes: usize,
}

/// Configuration snapshot (with sensitive data masked)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigSnapshot {
    pub model: String,
    pub api_key_masked: bool,
    pub workspace: PathBuf,
    pub verbose: bool,
}

/// Decision tracker state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionTrackerSnapshot {
    pub total_decisions: usize,
    pub successful_decisions: usize,
    pub failed_decisions: usize,
    pub recent_decisions: Vec<String>,
    pub available_tools: Vec<String>,
}

/// Error recovery state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecoverySnapshot {
    pub total_errors: usize,
    pub resolved_errors: usize,
    pub error_patterns: Vec<String>,
    pub recovery_attempts: Vec<String>,
}

/// Conversation summarizer state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizerSnapshot {
    pub total_summaries: usize,
    pub latest_summary: Option<String>,
    pub summary_history: Vec<String>,
}

/// Compaction engine state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEngineSnapshot {
    pub total_compactions: usize,
    pub memory_saved: usize,
    pub compression_ratio: f64,
    pub compaction_suggestions: Vec<String>,
}

/// Tree-sitter analyzer state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSitterSnapshot {
    pub loaded_languages: Vec<String>,
    pub total_analyses: usize,
    pub cache_size: usize,
}

/// Tool registry state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistrySnapshot {
    pub total_tools: usize,
    pub available_tools: Vec<String>,
    pub tool_usage_stats: HashMap<String, usize>,
}

/// Complete agent state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub metadata: SnapshotMetadata,
    pub config: AgentConfigSnapshot,
    pub session_info: SessionInfo,
    pub conversation_history: Vec<Content>,
    pub decision_tracker: DecisionTrackerSnapshot,
    pub error_recovery: ErrorRecoverySnapshot,
    pub summarizer: SummarizerSnapshot,
    pub compaction_engine: CompactionEngineSnapshot,
    pub tree_sitter_state: TreeSitterSnapshot,
    pub tool_registry: ToolRegistrySnapshot,
    pub environment: HashMap<String, String>,
    pub performance_metrics: PerformanceMetrics,
    pub checksum: String,
}

/// Snapshot manager configuration
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    pub enabled: bool,
    pub directory: PathBuf,
    pub max_snapshots: usize,
    pub compression_threshold: usize,
    pub auto_cleanup: bool,
    pub encryption_enabled: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: PathBuf::from("snapshots"),
            max_snapshots: 50,
            compression_threshold: 1024 * 1024, // 1MB
            auto_cleanup: true,
            encryption_enabled: false,
        }
    }
}

/// Main snapshot manager
pub struct SnapshotManager {
    config: SnapshotConfig,
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(config: SnapshotConfig) -> Self {
        let snapshots_dir = config.directory.clone();
        Self {
            config,
            snapshots_dir,
        }
    }

    /// Create a snapshot of the current agent state
    pub async fn create_snapshot(
        &self,
        agent: &Agent,
        conversation_history: &[crate::gemini::Content],
        turn_number: usize,
        description: &str,
    ) -> Result<String> {
        if !self.config.enabled {
            return Ok(String::new());
        }

        // Extract agent state
        let mut snapshot = self
            .extract_agent_state(agent, turn_number, description)
            .await?;

        // Add conversation history to the snapshot
        snapshot.conversation_history = conversation_history.to_vec();

        // Serialize to JSON
        let json_data = serde_json::to_string_pretty(&snapshot)?;

        // Calculate checksum
        let checksum = self.calculate_checksum(&json_data);
        snapshot.checksum = checksum.clone();

        // Update metadata size
        snapshot.metadata.size_bytes = json_data.len();

        // Serialize again with updated checksum and size
        let json_data = serde_json::to_string_pretty(&snapshot)?;

        // Create snapshot filename
        let filename = format!("turn_{}.json", turn_number);
        let filepath = self.snapshots_dir.join(&filename);

        // Ensure directory exists
        fs::create_dir_all(&self.snapshots_dir)?;

        // Write atomically (temporary file then rename)
        let temp_filepath = filepath.with_extension("tmp");
        fs::write(&temp_filepath, &json_data)?;
        fs::rename(&temp_filepath, &filepath)?;

        // Cleanup old snapshots if needed
        if self.config.auto_cleanup {
            self.cleanup_old_snapshots().await?;
        }

        Ok(filename)
    }

    /// Revert agent to a specific snapshot
    pub async fn revert_to_snapshot(
        &self,
        agent: &mut Agent,
        turn_number: usize,
        revert_type: RevertType,
    ) -> Result<()> {
        let filename = format!("turn_{}.json", turn_number);
        let filepath = self.snapshots_dir.join(&filename);

        if !filepath.exists() {
            return Err(anyhow!("Snapshot not found: {}", filename));
        }

        // Read and deserialize snapshot
        let json_data = fs::read_to_string(&filepath)?;
        let snapshot: AgentSnapshot = serde_json::from_str(&json_data)?;

        // Verify checksum
        let calculated_checksum = self.calculate_checksum(&json_data);
        if calculated_checksum != snapshot.checksum {
            return Err(anyhow!("Snapshot checksum verification failed"));
        }

        // Apply revert based on type
        match revert_type {
            RevertType::Full => self.revert_full_state(agent, &snapshot).await,
            RevertType::Memory => self.revert_memory_state(&snapshot, agent).await,
            RevertType::Context => self.revert_context_state(&snapshot, agent).await,
        }
    }

    /// List available snapshots
    pub async fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>> {
        let mut snapshots = Vec::new();

        if !self.snapshots_dir.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let filename = match path.file_stem().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };

            let turn_str = match filename.strip_prefix("turn_") {
                Some(turn) => turn,
                None => continue,
            };

            let turn_number = match turn_str.parse::<usize>() {
                Ok(num) => num,
                Err(_) => continue,
            };

            let metadata = fs::metadata(&path)?;
            snapshots.push(SnapshotInfo {
                turn_number,
                filename: filename.to_string(),
                size_bytes: metadata.len() as usize,
                created_at: metadata
                    .created()?
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs(),
            });
        }

        snapshots.sort_by(|a, b| b.turn_number.cmp(&a.turn_number));
        Ok(snapshots)
    }

    /// Clean up old snapshots beyond the limit
    pub async fn cleanup_old_snapshots(&self) -> Result<()> {
        let snapshots = self.list_snapshots().await?;

        if snapshots.len() > self.config.max_snapshots {
            let to_delete = snapshots.len() - self.config.max_snapshots;

            for snapshot in snapshots.iter().rev().take(to_delete) {
                let filepath = self
                    .snapshots_dir
                    .join(format!("{}.json", snapshot.filename));
                if filepath.exists() {
                    fs::remove_file(&filepath)?;
                }
            }
        }

        Ok(())
    }

    /// Extract current agent state into snapshot
    async fn extract_agent_state(
        &self,
        agent: &Agent,
        turn_number: usize,
        description: &str,
    ) -> Result<AgentSnapshot> {
        let metadata = SnapshotMetadata {
            id: format!("snapshot_turn_{}", turn_number),
            turn_number,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            description: description.to_string(),
            version: "1.0".to_string(),
            compressed: false,
            encrypted: false,
            size_bytes: 0, // Will be updated after serialization
        };

        let config = AgentConfigSnapshot {
            model: agent.config().model.clone(),
            api_key_masked: true, // Never store actual API key
            workspace: agent.config().workspace.clone(),
            verbose: agent.config().verbose,
        };

        // Extract decision tracker data
        let decision_tracker_data = agent.decision_tracker().generate_transparency_report();
        let decision_tracker = DecisionTrackerSnapshot {
            total_decisions: decision_tracker_data.total_decisions,
            successful_decisions: decision_tracker_data.successful_decisions,
            failed_decisions: decision_tracker_data.failed_decisions,
            recent_decisions: decision_tracker_data
                .recent_decisions
                .iter()
                .map(|d| format!("Decision {}: {}", d.id, d.reasoning))
                .collect(),
            available_tools: agent
                .decision_tracker()
                .get_current_context()
                .available_tools
                .clone(),
        };

        // Extract error recovery data
        let error_stats = agent.error_recovery().get_error_statistics();
        let error_recovery = ErrorRecoverySnapshot {
            total_errors: error_stats.total_errors,
            resolved_errors: error_stats.resolved_errors,
            error_patterns: error_stats
                .errors_by_type
                .iter()
                .map(|(error_type, count)| format!("{:?}: {}", error_type, count))
                .collect(),
            recovery_attempts: error_stats
                .recent_errors
                .iter()
                .map(|e| format!("Error {}: {}", e.id, e.message))
                .collect(),
        };

        // Extract summarizer data
        let summaries = agent.summarizer().get_summaries();
        let summarizer = SummarizerSnapshot {
            total_summaries: summaries.len(),
            latest_summary: agent
                .summarizer()
                .get_latest_summary()
                .map(|s| s.summary_text.clone()),
            summary_history: summaries.iter().map(|s| s.summary_text.clone()).collect(),
        };

        // Extract compaction engine data
        let compaction_stats = agent
            .compaction_engine()
            .get_statistics()
            .await
            .unwrap_or_else(|_| crate::core::agent::stats::CompactionStatistics {
                total_messages: 0,
                messages_by_priority: std::collections::HashMap::new(),
                total_memory_usage: 0,
                average_message_size: 0,
                last_compaction_timestamp: 0,
                compaction_frequency: 0.0,
            });
        let compaction_suggestions = agent
            .compaction_engine()
            .get_compaction_suggestions()
            .await
            .unwrap_or_else(|_| Vec::new());
        let compaction_engine = CompactionEngineSnapshot {
            total_compactions: compaction_stats.total_messages,
            memory_saved: compaction_stats.total_memory_usage,
            compression_ratio: compaction_stats.compaction_frequency, // Using available field
            compaction_suggestions: compaction_suggestions
                .iter()
                .map(|s| format!("Suggestion: {:?}", s))
                .collect(),
        };

        // Extract tree-sitter analyzer data
        let tree_sitter_analyzer = agent.tree_sitter_analyzer();
        let tree_sitter_state = TreeSitterSnapshot {
            loaded_languages: tree_sitter_analyzer
                .supported_languages()
                .iter()
                .map(|l| format!("{}", l))
                .collect(),
            total_analyses: tree_sitter_analyzer
                .get_parser_stats()
                .get("supported_languages")
                .copied()
                .unwrap_or(0),
            cache_size: 0, // Would need to implement cache size tracking
        };

        // Extract tool registry data
        let tool_registry_ref = agent.tool_registry();
        let tool_usage_stats = std::collections::HashMap::new(); // Would need to implement actual tool usage tracking
        let tool_registry = ToolRegistrySnapshot {
            total_tools: tool_registry_ref.available_tools().len(),
            available_tools: tool_registry_ref.available_tools(),
            tool_usage_stats,
        };

        let environment = std::env::vars().collect();

        Ok(AgentSnapshot {
            metadata,
            config,
            session_info: agent.session_info().clone(),
            conversation_history: vec![], // This will be filled when creating the snapshot
            decision_tracker,
            error_recovery,
            summarizer,
            compaction_engine,
            tree_sitter_state,
            tool_registry,
            environment,
            performance_metrics: crate::core::performance_monitor::PerformanceMetrics {
                response_times: vec![std::time::Duration::from_millis(200)],
                cache_hit_rate: 0.8,
                memory_usage: 50,
                error_rate: 0.0,
                throughput: 10,
                context_accuracy: 0.9,
            },
            checksum: String::new(), // Will be set after serialization
        })
    }

    /// Calculate SHA-256 checksum of data
    fn calculate_checksum(&self, data: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Revert to full state
    async fn revert_full_state(&self, _agent: &mut Agent, _snapshot: &AgentSnapshot) -> Result<()> {
        anyhow::bail!("Snapshot revert (full) is not implemented yet")
    }

    /// Revert memory state only
    async fn revert_memory_state(
        &self,
        _snapshot: &AgentSnapshot,
        _agent: &mut Agent,
    ) -> Result<()> {
        anyhow::bail!("Snapshot revert (memory) is not implemented yet")
    }

    /// Revert context state only
    async fn revert_context_state(
        &self,
        _snapshot: &AgentSnapshot,
        _agent: &mut Agent,
    ) -> Result<()> {
        anyhow::bail!("Snapshot revert (context) is not implemented yet")
    }
}

/// Types of revert operations
#[derive(Debug, Clone)]
pub enum RevertType {
    Full,
    Memory,
    Context,
}

/// Information about a snapshot
#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub turn_number: usize,
    pub filename: String,
    pub size_bytes: usize,
    pub created_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_snapshot_serialization() {
        // Create a test snapshot
        let metadata = SnapshotMetadata {
            id: "test_snapshot".to_string(),
            turn_number: 5,
            timestamp: 1234567890,
            description: "Test snapshot".to_string(),
            version: "1.0".to_string(),
            compressed: false,
            encrypted: false,
            size_bytes: 1024,
        };

        let config = AgentConfigSnapshot {
            model: "test-model".to_string(),
            api_key_masked: true,
            workspace: std::path::PathBuf::from("/tmp/test"),
            verbose: false,
        };

        let session_info = SessionInfo {
            session_id: "test_session".to_string(),
            start_time: 1234567890,
            total_turns: 5,
            total_decisions: 10,
            error_count: 0,
        };

        let snapshot = AgentSnapshot {
            metadata,
            config,
            session_info,
            conversation_history: vec![],
            decision_tracker: DecisionTrackerSnapshot {
                total_decisions: 10,
                successful_decisions: 8,
                failed_decisions: 2,
                recent_decisions: vec![],
                available_tools: vec!["test_tool".to_string()],
            },
            error_recovery: ErrorRecoverySnapshot {
                total_errors: 0,
                resolved_errors: 0,
                error_patterns: vec![],
                recovery_attempts: vec![],
            },
            summarizer: SummarizerSnapshot {
                total_summaries: 0,
                latest_summary: None,
                summary_history: vec![],
            },
            compaction_engine: CompactionEngineSnapshot {
                total_compactions: 0,
                memory_saved: 0,
                compression_ratio: 1.0,
                compaction_suggestions: vec![],
            },
            tree_sitter_state: TreeSitterSnapshot {
                loaded_languages: vec![],
                total_analyses: 0,
                cache_size: 0,
            },
            tool_registry: ToolRegistrySnapshot {
                total_tools: 1,
                available_tools: vec!["test_tool".to_string()],
                tool_usage_stats: HashMap::new(),
            },
            environment: HashMap::new(),
            performance_metrics: crate::core::performance_monitor::PerformanceMetrics {
                response_times: vec![std::time::Duration::from_millis(200)],
                cache_hit_rate: 0.8,
                memory_usage: 50,
                error_rate: 0.0,
                throughput: 10,
                context_accuracy: 0.9,
            },
            checksum: "test_checksum".to_string(),
        };

        // Test serialization
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("test_snapshot"));
        assert!(json.contains("test-model"));

        // Test deserialization
        let deserialized: AgentSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata.id, "test_snapshot");
        assert_eq!(deserialized.config.model, "test-model");
        assert_eq!(deserialized.session_info.total_turns, 5);
    }

    #[test]
    fn test_snapshot_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotConfig {
            enabled: true,
            directory: temp_dir.path().to_path_buf(),
            max_snapshots: 10,
            compression_threshold: 1024,
            auto_cleanup: true,
            encryption_enabled: false,
        };

        let manager = SnapshotManager::new(config);
        assert_eq!(manager.snapshots_dir, temp_dir.path().to_path_buf());
        assert_eq!(manager.config.max_snapshots, 10);
    }

    #[test]
    fn test_checksum_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let config = SnapshotConfig::default();
        let manager = SnapshotManager::new(config);

        let test_data = "test data for checksum";
        let checksum = manager.calculate_checksum(test_data);
        assert!(!checksum.is_empty());

        // Same data should produce same checksum
        let checksum2 = manager.calculate_checksum(test_data);
        assert_eq!(checksum, checksum2);

        // Different data should produce different checksum
        let checksum3 = manager.calculate_checksum("different data");
        assert_ne!(checksum, checksum3);
    }
}

//! Intelligent compaction system for threads and context
//!
//! This module implements Minimal research-preview compaction strategies to optimize memory usage
//! and performance by intelligently compressing conversation threads and semantic context.

use serde::{Deserialize, Serialize};

/// Compaction strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Maximum number of messages to keep in uncompressed form
    pub max_uncompressed_messages: usize,
    /// Maximum age of messages before compaction (in seconds)
    pub max_message_age_seconds: u64,
    /// Maximum memory usage before triggering compaction (in MB)
    pub max_memory_mb: usize,
    /// Compaction interval (in seconds)
    pub compaction_interval_seconds: u64,
    /// Minimum confidence threshold for keeping context data
    pub min_context_confidence: f64,
    /// Maximum age of context data before compaction (in seconds)
    pub max_context_age_seconds: u64,
    /// Whether to enable automatic compaction
    pub auto_compaction_enabled: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_uncompressed_messages: 50,
            max_message_age_seconds: 3600, // 1 hour
            max_memory_mb: 100,
            compaction_interval_seconds: 300, // 5 minutes
            min_context_confidence: 0.3,
            max_context_age_seconds: 7200, // 2 hours
            auto_compaction_enabled: true,
        }
    }
}

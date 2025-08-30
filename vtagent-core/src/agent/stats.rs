//! Statistics and reporting structures for compaction system

use crate::agent::types::MessagePriority;
use serde::{Deserialize, Serialize};

/// Compaction operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    /// Number of messages processed
    pub messages_processed: usize,
    /// Number of messages compacted
    pub messages_compacted: usize,
    /// Total original size (bytes)
    pub original_size: usize,
    /// Total compacted size (bytes)
    pub compacted_size: usize,
    /// Overall compression ratio
    pub compression_ratio: f64,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Compaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionStatistics {
    /// Total messages in history
    pub total_messages: usize,
    /// Messages by priority level
    pub messages_by_priority: std::collections::HashMap<MessagePriority, usize>,
    /// Total memory usage (bytes)
    pub total_memory_usage: usize,
    /// Average message size (bytes)
    pub average_message_size: usize,
    /// Last compaction timestamp
    pub last_compaction_timestamp: u64,
    /// Compaction frequency (operations per hour)
    pub compaction_frequency: f64,
}

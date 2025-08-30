//! Type definitions for the compaction system

use serde::{Deserialize, Serialize};

/// Compacted message representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedMessage {
    /// Original message timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Message type (user/assistant/tool)
    pub message_type: MessageType,
    /// Compacted content summary
    pub summary: String,
    /// Key information extracted
    pub key_info: Vec<String>,
    /// Compression ratio achieved
    pub compression_ratio: f64,
    /// Original message size (bytes)
    pub original_size: usize,
}

/// Message type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    UserMessage,
    AssistantResponse,
    ToolCall,
    ToolResponse,
    SystemMessage,
}

/// Message priority levels for intelligent compaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 1,      // Can be compacted easily (routine tool calls, simple acknowledgments)
    Medium = 2,   // Moderate importance (general responses, standard operations)
    High = 3,     // Should be preserved (important decisions, code changes, user requests)
    Critical = 4, // Must be preserved (security operations, critical errors, user constraints)
}

/// Enhanced message with priority and semantic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedMessage {
    pub base_message: CompactedMessage,
    pub priority: MessagePriority,
    pub semantic_tags: Vec<String>,
    pub context_references: Vec<String>,
    pub conversation_turn: usize,
    pub related_messages: Vec<usize>,
}

/// Compacted context data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedContext {
    /// Timestamp of compaction (Unix timestamp)
    pub compacted_at: u64,
    /// Original context size
    pub original_size: usize,
    /// Compacted context size
    pub compacted_size: usize,
    /// Context key/identifier
    pub context_key: String,
    /// Compression ratio achieved
    pub compression_ratio: f64,
    /// Context confidence score
    pub confidence_score: f64,
}

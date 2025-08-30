//! Legacy compaction module
//! Legacy compaction module - now split into smaller modules
//!
//! This file maintains backward compatibility by re-exporting
//! types and functions from the new modular structure.

pub use crate::agent::config::*;
pub use crate::agent::engine::*;
pub use crate::agent::semantic::*;

// Re-export specific types to avoid ambiguity
pub use crate::agent::types::CompactionSuggestion;
pub use crate::agent::types::Urgency;
pub use crate::agent::types::MessageType;
pub use crate::agent::types::MessagePriority;
pub use crate::agent::types::CompactedMessage;
pub use crate::agent::types::EnhancedMessage;
pub use crate::agent::types::CompactedContext;
pub use crate::agent::stats::CompactionStatistics;
pub use crate::agent::stats::CompactionResult;

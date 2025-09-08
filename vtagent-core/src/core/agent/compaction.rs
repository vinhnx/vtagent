//! Legacy compaction module
//! Legacy compaction module - now split into smaller modules
//!
//! This file maintains backward compatibility by re-exporting
//! types and functions from the new modular structure.

pub use crate::core::agent::config::*;
pub use crate::core::agent::engine::*;
pub use crate::core::agent::semantic::*;

// Re-export specific types to avoid ambiguity
pub use crate::core::agent::stats::CompactionResult;
pub use crate::core::agent::stats::CompactionStatistics;
pub use crate::core::agent::types::CompactedContext;
pub use crate::core::agent::types::CompactedMessage;
pub use crate::core::agent::types::CompactionSuggestion;
pub use crate::core::agent::types::EnhancedMessage;
pub use crate::core::agent::types::MessagePriority;
pub use crate::core::agent::types::MessageType;
pub use crate::core::agent::types::Urgency;

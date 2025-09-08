//! Agent system for intelligent conversation management

pub mod chat;
pub mod compaction; // Legacy - will be replaced by new modules
pub mod config;
pub mod core;
pub mod engine;
pub mod examples;
pub mod integration;
pub mod intelligence;
pub mod multi_agent;
pub mod multi_agent_tools;
pub mod optimization;
pub mod orchestrator;
pub mod performance;
pub mod runner;
pub mod semantic;
pub mod snapshots;
pub mod stats;
pub mod types;
pub mod verification;

// Re-export main types for convenience
pub use config::CompactionConfig;
pub use engine::CompactionEngine;
pub use multi_agent::*;
pub use multi_agent_tools::*;
pub use orchestrator::*;
pub use runner::*;
pub use semantic::SemanticAnalyzer;
pub use types::*;

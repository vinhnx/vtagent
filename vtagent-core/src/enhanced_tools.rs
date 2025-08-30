//! Legacy enhanced tools module - now split into focused submodules
//!
//! This file maintains backward compatibility by re-exporting
//! types and functions from the new modular structure.
//!
//! For new code, prefer using the specific submodules:
//! - `cache` for file operation caching
//! - `processor` for parallel file processing
//! - `analysis` for text and semantic analysis

// Re-export everything from the new modular structure
pub use crate::enhanced_tools::cache::*;
pub use crate::enhanced_tools::processor::*;
pub use crate::enhanced_tools::analysis::*;

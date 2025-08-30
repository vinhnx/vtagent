//! Legacy compaction module
//! Legacy compaction module - now split into smaller modules
//!
//! This file maintains backward compatibility by re-exporting
//! types and functions from the new modular structure.

pub use crate::agent::config::*;
pub use crate::agent::types::*;
pub use crate::agent::engine::*;
pub use crate::agent::semantic::*;
pub use crate::agent::stats::*;

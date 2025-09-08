//! Code quality tools with modular architecture
//!
//! This module provides comprehensive code formatting, linting, and quality
//! assurance tools with language-specific implementations.

pub mod formatting;
pub mod linting;
pub mod metrics;
pub mod config;

// Re-export main types for backward compatibility
pub use formatting::{FormattingOrchestrator, FormatResult};
pub use linting::{LintingOrchestrator, LintResult};
pub use metrics::{QualityMetrics, ComplexityAnalyzer};
pub use config::{FormatConfig, LintConfig, LintSeverity};

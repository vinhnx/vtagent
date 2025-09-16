use crate::config::constants::context as context_defaults;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LedgerConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    /// Inject ledger into the system prompt each turn
    #[serde(default = "default_include_in_prompt")]
    pub include_in_prompt: bool,
    /// Preserve ledger entries during context compression
    #[serde(default = "default_preserve_in_compression")]
    pub preserve_in_compression: bool,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            max_entries: default_max_entries(),
            include_in_prompt: default_include_in_prompt(),
            preserve_in_compression: default_preserve_in_compression(),
        }
    }
}

fn default_enabled() -> bool {
    true
}
fn default_max_entries() -> usize {
    12
}
fn default_include_in_prompt() -> bool {
    true
}
fn default_preserve_in_compression() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextFeaturesConfig {
    #[serde(default)]
    pub ledger: LedgerConfig,
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,
    #[serde(default = "default_trim_to_percent")]
    pub trim_to_percent: u8,
    #[serde(default = "default_preserve_recent_turns")]
    pub preserve_recent_turns: usize,
}

impl Default for ContextFeaturesConfig {
    fn default() -> Self {
        Self {
            ledger: LedgerConfig::default(),
            max_context_tokens: default_max_context_tokens(),
            trim_to_percent: default_trim_to_percent(),
            preserve_recent_turns: default_preserve_recent_turns(),
        }
    }
}

fn default_max_context_tokens() -> usize {
    context_defaults::DEFAULT_MAX_TOKENS
}

fn default_trim_to_percent() -> u8 {
    context_defaults::DEFAULT_TRIM_TO_PERCENT
}

fn default_preserve_recent_turns() -> usize {
    context_defaults::DEFAULT_PRESERVE_RECENT_TURNS
}

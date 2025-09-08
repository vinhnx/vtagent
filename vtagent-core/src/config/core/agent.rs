use crate::config::constants::defaults;
use serde::{Deserialize, Serialize};

/// Agent-wide configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Maximum number of conversation turns before auto-termination
    #[serde(default = "default_max_conversation_turns")]
    pub max_conversation_turns: usize,

    /// Maximum session duration in minutes
    #[serde(default = "default_max_session_duration_minutes")]
    pub max_session_duration_minutes: u64,

    /// Enable verbose logging
    #[serde(default)]
    pub verbose_logging: bool,

    /// Maximum conversation history to keep
    #[serde(default = "default_max_conversation_history")]
    pub max_conversation_history: usize,

    /// Maximum steps per turn
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,

    /// Maximum empty responses before terminating
    #[serde(default = "default_max_empty_responses")]
    pub max_empty_responses: usize,

    /// AI provider for single agent mode (gemini, openai, anthropic)
    #[serde(default = "default_provider")]
    pub provider: String,

    /// Default model to use
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Default API key environment variable
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// API key for Gemini provider (for configuration file storage)
    #[serde(default)]
    pub gemini_api_key: Option<String>,

    /// API key for Anthropic provider (for configuration file storage)
    #[serde(default)]
    pub anthropic_api_key: Option<String>,

    /// API key for OpenAI provider (for configuration file storage)
    #[serde(default)]
    pub openai_api_key: Option<String>,

    /// Default system instruction fallback
    #[serde(default = "default_system_instruction")]
    pub default_system_instruction: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_conversation_turns: default_max_conversation_turns(),
            max_session_duration_minutes: default_max_session_duration_minutes(),
            verbose_logging: false,
            max_conversation_history: default_max_conversation_history(),
            max_steps: default_max_steps(),
            max_empty_responses: default_max_empty_responses(),
            provider: default_provider(),
            default_model: default_model(),
            api_key_env: default_api_key_env(),
            gemini_api_key: None,
            anthropic_api_key: None,
            openai_api_key: None,
            default_system_instruction: default_system_instruction(),
        }
    }
}

fn default_max_conversation_turns() -> usize {
    1000
}
fn default_max_session_duration_minutes() -> u64 {
    60
}
fn default_max_conversation_history() -> usize {
    100
}
fn default_max_steps() -> usize {
    50
}
fn default_max_empty_responses() -> usize {
    3
}
fn default_provider() -> String {
    defaults::DEFAULT_PROVIDER.to_string()
}
fn default_model() -> String {
    defaults::DEFAULT_MODEL.to_string()
}
fn default_api_key_env() -> String {
    defaults::DEFAULT_API_KEY_ENV.to_string()
}
fn default_system_instruction() -> String {
    "You are a helpful AI assistant.".to_string()
}

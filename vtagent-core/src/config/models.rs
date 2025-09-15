//! Model configuration and identification module
//!
//! This module provides a centralized enum for model identifiers and their configurations,
//! replacing hardcoded model strings throughout the codebase for better maintainability.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Supported AI model providers
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Provider {
    /// Google Gemini models
    #[default]
    Gemini,
    /// OpenAI GPT models
    OpenAI,
    /// Anthropic Claude models
    Anthropic,
}

impl Provider {
    /// Get the default API key environment variable for this provider
    pub fn default_api_key_env(&self) -> &'static str {
        match self {
            Provider::Gemini => "GEMINI_API_KEY",
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
        }
    }

    /// Get all supported providers
    pub fn all_providers() -> Vec<Provider> {
        vec![Provider::Gemini, Provider::OpenAI, Provider::Anthropic]
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Gemini => write!(f, "gemini"),
            Provider::OpenAI => write!(f, "openai"),
            Provider::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl FromStr for Provider {
    type Err = ModelParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gemini" => Ok(Provider::Gemini),
            "openai" => Ok(Provider::OpenAI),
            "anthropic" => Ok(Provider::Anthropic),
            _ => Err(ModelParseError::InvalidProvider(s.to_string())),
        }
    }
}

/// Centralized enum for all supported model identifiers
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelId {
    // Gemini models
    /// Gemini 2.5 Flash Lite - Fastest, most cost-effective
    Gemini25FlashLite,
    /// Gemini 2.5 Flash - Fast, cost-effective, default for agent/planning/orchestrator
    Gemini25Flash,
    /// Gemini 2.5 Pro - Latest, most capable
    Gemini25Pro,

    // OpenAI models
    /// GPT-5 - High performance model
    GPT5,
    /// GPT-5 mini - Smaller, faster version and fast and economical
    GPT5Mini,

    // Anthropic models
    /// Claude Sonnet 4 - Most intelligent model
    ClaudeSonnet4,
    /// Claude Opus 4.1 - Powerful model for complex tasks
    ClaudeOpus41,
}
impl ModelId {
    /// Convert the model identifier to its string representation
    /// used in API calls and configurations
    pub fn as_str(&self) -> &'static str {
        use crate::config::constants::models;
        match self {
            // Gemini models
            ModelId::Gemini25FlashLite => models::GEMINI_2_5_FLASH_LITE,
            ModelId::Gemini25Flash => models::GEMINI_2_5_FLASH,
            ModelId::Gemini25Pro => models::GEMINI_2_5_PRO,
            // OpenAI models
            ModelId::GPT5 => models::GPT_5,
            ModelId::GPT5Mini => models::GPT_5_MINI,
            // Anthropic models
            ModelId::ClaudeSonnet4 => models::CLAUDE_SONNET_4_20250514,
            ModelId::ClaudeOpus41 => models::CLAUDE_OPUS_4_1_20250805,
        }
    }

    /// Get the provider for this model
    pub fn provider(&self) -> Provider {
        match self {
            ModelId::Gemini25FlashLite | ModelId::Gemini25Flash | ModelId::Gemini25Pro => {
                Provider::Gemini
            }
            ModelId::GPT5 | ModelId::GPT5Mini => Provider::OpenAI,
            ModelId::ClaudeSonnet4 | ModelId::ClaudeOpus41 => Provider::Anthropic,
        }
    }

    /// Get the display name for the model (human-readable)
    pub fn display_name(&self) -> &'static str {
        match self {
            // Gemini models
            ModelId::Gemini25FlashLite => "Gemini 2.5 Flash Lite",
            ModelId::Gemini25Flash => "Gemini 2.5 Flash",
            ModelId::Gemini25Pro => "Gemini 2.5 Pro",
            // OpenAI models
            ModelId::GPT5 => "GPT-5",
            ModelId::GPT5Mini => "GPT-5 mini",
            // Anthropic models
            ModelId::ClaudeSonnet4 => "Claude Sonnet 4",
            ModelId::ClaudeOpus41 => "Claude Opus 4.1",
        }
    }

    /// Get a description of the model's characteristics
    pub fn description(&self) -> &'static str {
        match self {
            // Gemini models
            ModelId::Gemini25FlashLite => "Fastest, most cost-effective",
            ModelId::Gemini25Flash => {
                "Fast, cost-effective, default for agent/planning/orchestrator"
            }
            ModelId::Gemini25Pro => "Latest, most capable",
            // OpenAI models
            ModelId::GPT5 => "High performance model",
            ModelId::GPT5Mini => "Smaller, faster version and fast and economical",
            // Anthropic models
            ModelId::ClaudeSonnet4 => "Most intelligent model",
            ModelId::ClaudeOpus41 => "Powerful model for complex tasks",
        }
    }

    /// Get all available models as a vector
    pub fn all_models() -> Vec<ModelId> {
        vec![
            // Gemini models
            ModelId::Gemini25FlashLite,
            ModelId::Gemini25Flash,
            ModelId::Gemini25Pro,
            // OpenAI models
            ModelId::GPT5,
            ModelId::GPT5Mini,
            // Anthropic models
            ModelId::ClaudeSonnet4,
            ModelId::ClaudeOpus41,
        ]
    }

    /// Get all models for a specific provider
    pub fn models_for_provider(provider: Provider) -> Vec<ModelId> {
        Self::all_models()
            .into_iter()
            .filter(|model| model.provider() == provider)
            .collect()
    }

    /// Get recommended fallback models in order of preference
    pub fn fallback_models() -> Vec<ModelId> {
        vec![
            ModelId::Gemini25Flash,
            ModelId::Gemini25Pro,
            ModelId::GPT5,
            ModelId::ClaudeSonnet4,
        ]
    }

    /// Get the default model for general use
    pub fn default() -> Self {
        ModelId::Gemini25FlashLite
    }

    /// Get the default orchestrator model (more capable)
    pub fn default_orchestrator() -> Self {
        ModelId::Gemini25Flash
    }

    /// Get the default subagent model (fast and efficient)
    pub fn default_subagent() -> Self {
        ModelId::Gemini25FlashLite
    }

    /// Get provider-specific defaults for orchestrator
    pub fn default_orchestrator_for_provider(provider: Provider) -> Self {
        match provider {
            Provider::Gemini => ModelId::Gemini25FlashLite,
            Provider::OpenAI => ModelId::GPT5,
            Provider::Anthropic => ModelId::ClaudeSonnet4,
        }
    }

    /// Get provider-specific defaults for subagent
    pub fn default_subagent_for_provider(provider: Provider) -> Self {
        match provider {
            Provider::Gemini => ModelId::Gemini25FlashLite,
            Provider::OpenAI => ModelId::GPT5Mini,
            Provider::Anthropic => ModelId::ClaudeOpus41,
        }
    }

    /// Get provider-specific defaults for single agent
    pub fn default_single_for_provider(provider: Provider) -> Self {
        match provider {
            Provider::Gemini => ModelId::Gemini25Flash,
            Provider::OpenAI => ModelId::GPT5,
            Provider::Anthropic => ModelId::ClaudeSonnet4,
        }
    }

    /// Check if this is a "flash" variant (optimized for speed)
    pub fn is_flash_variant(&self) -> bool {
        matches!(self, ModelId::Gemini25FlashLite | ModelId::Gemini25Flash)
    }

    /// Check if this is a "pro" variant (optimized for capability)
    pub fn is_pro_variant(&self) -> bool {
        matches!(self, ModelId::Gemini25Pro | ModelId::GPT5)
    }

    /// Check if this is an optimized/efficient variant
    pub fn is_efficient_variant(&self) -> bool {
        matches!(self, ModelId::Gemini25FlashLite | ModelId::GPT5Mini)
    }

    /// Check if this is a top-tier model
    pub fn is_top_tier(&self) -> bool {
        matches!(
            self,
            ModelId::Gemini25Pro | ModelId::GPT5 | ModelId::ClaudeSonnet4 | ModelId::ClaudeOpus41
        )
    }

    /// Get the generation/version string for this model
    pub fn generation(&self) -> &'static str {
        match self {
            // Gemini generations
            ModelId::Gemini25FlashLite | ModelId::Gemini25Flash | ModelId::Gemini25Pro => "2.5",
            // OpenAI generations
            ModelId::GPT5 | ModelId::GPT5Mini => "5",
            // Anthropic generations
            ModelId::ClaudeSonnet4 => "4",
            ModelId::ClaudeOpus41 => "4.1",
        }
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ModelId {
    type Err = ModelParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use crate::config::constants::models;
        match s {
            // Gemini models
            s if s == models::GEMINI_2_5_FLASH_LITE => Ok(ModelId::Gemini25FlashLite),
            s if s == models::GEMINI_2_5_FLASH => Ok(ModelId::Gemini25Flash),
            s if s == models::GEMINI_2_5_PRO => Ok(ModelId::Gemini25Pro),
            // OpenAI models
            s if s == models::GPT_5 => Ok(ModelId::GPT5),
            s if s == models::GPT_5_MINI => Ok(ModelId::GPT5Mini),
            // Anthropic models
            s if s == models::CLAUDE_SONNET_4_20250514 => Ok(ModelId::ClaudeSonnet4),
            s if s == models::CLAUDE_OPUS_4_1_20250805 => Ok(ModelId::ClaudeOpus41),
            _ => Err(ModelParseError::InvalidModel(s.to_string())),
        }
    }
}

/// Error type for model parsing failures
#[derive(Debug, Clone, PartialEq)]
pub enum ModelParseError {
    InvalidModel(String),
    InvalidProvider(String),
}

impl fmt::Display for ModelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelParseError::InvalidModel(model) => {
                write!(
                    f,
                    "Invalid model identifier: '{}'. Supported models: {}",
                    model,
                    ModelId::all_models()
                        .iter()
                        .map(|m| m.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            ModelParseError::InvalidProvider(provider) => {
                write!(
                    f,
                    "Invalid provider: '{}'. Supported providers: {}",
                    provider,
                    Provider::all_providers()
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ModelParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_string_conversion() {
        // Gemini models
        assert_eq!(ModelId::Gemini25FlashLite.as_str(), "gemini-2.5-flash-lite-preview-06-17");
        assert_eq!(ModelId::Gemini25Pro.as_str(), "gemini-2.5-pro");
        // OpenAI models
        assert_eq!(ModelId::GPT5.as_str(), "gpt-5");
        assert_eq!(ModelId::GPT5Mini.as_str(), "gpt-5-mini");
        // Anthropic models
        assert_eq!(ModelId::ClaudeSonnet4.as_str(), "claude-sonnet-4-20250514");
        assert_eq!(ModelId::ClaudeOpus41.as_str(), "claude-opus-4-1-20250805");
    }

    #[test]
    fn test_model_from_string() {
        // Gemini models
        assert_eq!(
            "gemini-2.5-flash-lite-preview-06-17".parse::<ModelId>().unwrap(),
            ModelId::Gemini25FlashLite
        );
        assert_eq!(
            "gemini-2.5-pro".parse::<ModelId>().unwrap(),
            ModelId::Gemini25Pro
        );
        // OpenAI models
        assert_eq!("gpt-5".parse::<ModelId>().unwrap(), ModelId::GPT5);
        assert_eq!("gpt-5-mini".parse::<ModelId>().unwrap(), ModelId::GPT5Mini);
        // Anthropic models
        assert_eq!(
            "claude-sonnet-4-20250514".parse::<ModelId>().unwrap(),
            ModelId::ClaudeSonnet4
        );
        // Invalid model
        assert!("invalid-model".parse::<ModelId>().is_err());
    }

    #[test]
    fn test_provider_parsing() {
        assert_eq!("gemini".parse::<Provider>().unwrap(), Provider::Gemini);
        assert_eq!("openai".parse::<Provider>().unwrap(), Provider::OpenAI);
        assert_eq!(
            "anthropic".parse::<Provider>().unwrap(),
            Provider::Anthropic
        );
        assert!("invalid-provider".parse::<Provider>().is_err());
    }

    #[test]
    fn test_model_providers() {
        assert_eq!(ModelId::Gemini25Flash.provider(), Provider::Gemini);
        assert_eq!(ModelId::GPT5.provider(), Provider::OpenAI);
        assert_eq!(ModelId::ClaudeSonnet4.provider(), Provider::Anthropic);
    }

    #[test]
    fn test_provider_defaults() {
        assert_eq!(
            ModelId::default_orchestrator_for_provider(Provider::Gemini),
            ModelId::Gemini25Flash
        );
        assert_eq!(
            ModelId::default_orchestrator_for_provider(Provider::OpenAI),
            ModelId::GPT5
        );
        assert_eq!(
            ModelId::default_orchestrator_for_provider(Provider::Anthropic),
            ModelId::ClaudeSonnet4
        );

        assert_eq!(
            ModelId::default_subagent_for_provider(Provider::Gemini),
            ModelId::Gemini25FlashLite
        );
        assert_eq!(
            ModelId::default_subagent_for_provider(Provider::OpenAI),
            ModelId::GPT5Mini
        );
        assert_eq!(
            ModelId::default_subagent_for_provider(Provider::Anthropic),
            ModelId::ClaudeOpus41
        );
    }

    #[test]
    fn test_model_defaults() {
        assert_eq!(ModelId::default(), ModelId::Gemini25FlashLite);
        assert_eq!(ModelId::default_orchestrator(), ModelId::Gemini25Flash);
        assert_eq!(ModelId::default_subagent(), ModelId::Gemini25FlashLite);
    }

    #[test]
    fn test_model_variants() {
        // Flash variants
        assert!(ModelId::Gemini25Flash.is_flash_variant());
        assert!(!ModelId::GPT5.is_flash_variant());

        // Pro variants
        assert!(ModelId::Gemini25Pro.is_pro_variant());
        assert!(ModelId::GPT5.is_pro_variant());
        assert!(!ModelId::Gemini25Flash.is_pro_variant());

        // Efficient variants
        assert!(ModelId::Gemini25FlashLite.is_efficient_variant());
        assert!(ModelId::GPT5Mini.is_efficient_variant());
        assert!(!ModelId::GPT5.is_efficient_variant());

        // Top tier models
        assert!(ModelId::Gemini25Pro.is_top_tier());
        assert!(ModelId::GPT5.is_top_tier());
        assert!(ModelId::ClaudeSonnet4.is_top_tier());
        assert!(!ModelId::Gemini25FlashLite.is_top_tier());
    }

    #[test]
    fn test_model_generation() {
        // Gemini generations
        assert_eq!(ModelId::Gemini25Flash.generation(), "2.5");
        assert_eq!(ModelId::Gemini25Pro.generation(), "2.5");

        // OpenAI generations
        assert_eq!(ModelId::GPT5.generation(), "5");
        assert_eq!(ModelId::GPT5Mini.generation(), "5");

        // Anthropic generations
        assert_eq!(ModelId::ClaudeSonnet4.generation(), "4");
        assert_eq!(ModelId::ClaudeOpus41.generation(), "4.1");
    }

    #[test]
    fn test_models_for_provider() {
        let gemini_models = ModelId::models_for_provider(Provider::Gemini);
        assert!(gemini_models.contains(&ModelId::Gemini25Flash));
        assert!(!gemini_models.contains(&ModelId::GPT5));

        let openai_models = ModelId::models_for_provider(Provider::OpenAI);
        assert!(openai_models.contains(&ModelId::GPT5));
        assert!(!openai_models.contains(&ModelId::Gemini25Flash));

        let anthropic_models = ModelId::models_for_provider(Provider::Anthropic);
        assert!(anthropic_models.contains(&ModelId::ClaudeSonnet4));
        assert!(!anthropic_models.contains(&ModelId::GPT5));
    }

    #[test]
    fn test_fallback_models() {
        let fallbacks = ModelId::fallback_models();
        assert!(!fallbacks.is_empty());
        assert!(fallbacks.contains(&ModelId::Gemini25Flash));
        assert!(fallbacks.contains(&ModelId::Gemini25Pro));
    }
}

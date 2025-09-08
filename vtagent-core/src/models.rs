//! Model configuration and identification module
//!
//! This module provides a centralized enum for model identifiers and their configurations,
//! replacing hardcoded model strings throughout the codebase for better maintainability.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Centralized enum for all supported model identifiers
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelId {
    /// Gemini 2.5 Flash Lite - Fastest, most cost-effective
    Gemini25FlashLite,
    /// Gemini 2.5 Flash - Fast, cost-effective
    Gemini25Flash,
    /// Gemini 2.5 Pro - Latest, most capable
    Gemini25Pro,
    /// Gemini 2.0 Flash - Previous generation, fast
    Gemini20Flash,
}

impl ModelId {
    /// Convert the model identifier to its string representation
    /// used in API calls and configurations
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelId::Gemini25FlashLite => "gemini-2.5-flash-lite",
            ModelId::Gemini25Flash => "gemini-2.5-flash",
            ModelId::Gemini25Pro => "gemini-2.5-pro",
            ModelId::Gemini20Flash => "gemini-2.0-flash",
        }
    }

    /// Get the display name for the model (human-readable)
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelId::Gemini25FlashLite => "Gemini 2.5 Flash Lite",
            ModelId::Gemini25Flash => "Gemini 2.5 Flash",
            ModelId::Gemini25Pro => "Gemini 2.5 Pro",
            ModelId::Gemini20Flash => "Gemini 2.0 Flash",
        }
    }

    /// Get a description of the model's characteristics
    pub fn description(&self) -> &'static str {
        match self {
            ModelId::Gemini25FlashLite => "Fastest, most cost-effective",
            ModelId::Gemini25Flash => "Fast, cost-effective",
            ModelId::Gemini25Pro => "Latest, most capable",
            ModelId::Gemini20Flash => "Previous generation, fast",
        }
    }

    /// Get all available models as a vector
    pub fn all_models() -> Vec<ModelId> {
        vec![
            ModelId::Gemini25FlashLite,
            ModelId::Gemini25Flash,
            ModelId::Gemini25Pro,
            ModelId::Gemini20Flash,
        ]
    }

    /// Get recommended fallback models in order of preference
    pub fn fallback_models() -> Vec<ModelId> {
        vec![
            ModelId::Gemini25Flash,
            ModelId::Gemini25Pro,
            ModelId::Gemini20Flash,
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

    /// Check if this is a "flash" variant (optimized for speed)
    pub fn is_flash_variant(&self) -> bool {
        matches!(
            self,
            ModelId::Gemini25FlashLite
                | ModelId::Gemini25Flash
                | ModelId::Gemini20Flash
        )
    }

    /// Check if this is a "pro" variant (optimized for capability)
    pub fn is_pro_variant(&self) -> bool {
        matches!(self, ModelId::Gemini25Pro)
    }

    /// Get the generation number (2.5, 2.0, etc.)
    pub fn generation(&self) -> &'static str {
        match self {
            ModelId::Gemini25FlashLite | ModelId::Gemini25Flash | ModelId::Gemini25Pro => "2.5",
            ModelId::Gemini20Flash => "2.0",
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
        match s {
            "gemini-2.5-flash-lite" => Ok(ModelId::Gemini25FlashLite),
            "gemini-2.5-flash" => Ok(ModelId::Gemini25Flash),
            "gemini-2.5-pro" => Ok(ModelId::Gemini25Pro),
            "gemini-2.0-flash" => Ok(ModelId::Gemini20Flash),
            _ => Err(ModelParseError::InvalidModel(s.to_string())),
        }
    }
}

/// Error type for model parsing failures
#[derive(Debug, Clone, PartialEq)]
pub enum ModelParseError {
    InvalidModel(String),
}

impl fmt::Display for ModelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelParseError::InvalidModel(model) => {
                write!(f, "Invalid model identifier: '{}'. Supported models: {}",
                    model,
                    ModelId::all_models().iter()
                        .map(|m| m.as_str())
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
        assert_eq!(ModelId::Gemini25FlashLite.as_str(), "gemini-2.5-flash-lite");
        assert_eq!(ModelId::Gemini25Pro.as_str(), "gemini-2.5-pro");
        assert_eq!(ModelId::Gemini20Flash.as_str(), "gemini-2.0-flash");
    }

    #[test]
    fn test_model_from_string() {
        assert_eq!("gemini-2.5-flash-lite".parse::<ModelId>().unwrap(), ModelId::Gemini25FlashLite);
        assert_eq!("gemini-2.5-pro".parse::<ModelId>().unwrap(), ModelId::Gemini25Pro);
        assert!("invalid-model".parse::<ModelId>().is_err());
    }

    #[test]
    fn test_model_defaults() {
        assert_eq!(ModelId::default(), ModelId::Gemini25FlashLite);
        assert_eq!(ModelId::default_orchestrator(), ModelId::Gemini25Pro);
        assert_eq!(ModelId::default_subagent(), ModelId::Gemini25Flash);
    }

    #[test]
    fn test_model_variants() {
        assert!(ModelId::Gemini25Flash.is_flash_variant());
        assert!(ModelId::Gemini25Pro.is_pro_variant());
        assert!(!ModelId::Gemini25Pro.is_flash_variant());
        assert!(!ModelId::Gemini25Flash.is_pro_variant());
    }

    #[test]
    fn test_model_generation() {
        assert_eq!(ModelId::Gemini25Flash.generation(), "2.5");
        assert_eq!(ModelId::Gemini20Flash.generation(), "2.0");
        assert_eq!(ModelId::Gemini25Pro.generation(), "2.5");
    }

    #[test]
    fn test_fallback_models() {
        let fallbacks = ModelId::fallback_models();
        assert!(!fallbacks.is_empty());
        assert!(fallbacks.contains(&ModelId::Gemini25Flash));
        assert!(fallbacks.contains(&ModelId::Gemini25Pro));
    }
}

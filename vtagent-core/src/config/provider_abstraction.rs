//! Provider abstraction layer for VTAgent
//!
//! This module provides a simplified interface for working with different AI providers
//! without exposing provider-specific complexity to the configuration or application logic.

use crate::config::provider_definitions::Provider;
use crate::config::model_definitions::ModelId;
use std::collections::HashMap;

/// Provider configuration details
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider name
    pub name: String,
    /// Default API key environment variable
    pub api_key_env: String,
    /// Default base URL for API calls
    pub base_url: String,
    /// Whether this is a local provider (no API key required)
    pub is_local: bool,
}

/// Provider abstraction registry
pub struct ProviderRegistry {
    providers: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    /// Create a new provider registry with default configurations
    pub fn new() -> Self {
        let mut registry = ProviderRegistry {
            providers: HashMap::new(),
        };
        
        // Register all supported providers
        registry.register_provider(ProviderConfig {
            name: "gemini".to_string(),
            api_key_env: "GEMINI_API_KEY".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            is_local: false,
        });
        
        registry.register_provider(ProviderConfig {
            name: "openai".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            is_local: false,
        });
        
        registry.register_provider(ProviderConfig {
            name: "anthropic".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            is_local: false,
        });
        
        registry.register_provider(ProviderConfig {
            name: "lmstudio".to_string(),
            api_key_env: "LMSTUDIO_API_KEY".to_string(),
            base_url: "http://localhost:1234/v1".to_string(),
            is_local: true,
        });
        
        registry.register_provider(ProviderConfig {
            name: "ollama".to_string(),
            api_key_env: "OLLAMA_API_KEY".to_string(),
            base_url: "http://localhost:11434/api".to_string(),
            is_local: true,
        });
        
        registry.register_provider(ProviderConfig {
            name: "openrouter".to_string(),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            is_local: false,
        });
        
        registry.register_provider(ProviderConfig {
            name: "groq".to_string(),
            api_key_env: "GROQ_API_KEY".to_string(),
            base_url: "https://api.groq.com/openai/v1".to_string(),
            is_local: false,
        });
        
        registry.register_provider(ProviderConfig {
            name: "deepseek".to_string(),
            api_key_env: "DEEPSEEK_API_KEY".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            is_local: false,
        });
        
        registry
    }
    
    /// Register a provider configuration
    pub fn register_provider(&mut self, config: ProviderConfig) {
        self.providers.insert(config.name.clone(), config);
    }
    
    /// Get provider configuration by name
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }
    
    /// Get API key environment variable for a provider
    pub fn get_api_key_env(&self, provider: &Provider) -> String {
        match self.providers.get(&provider.to_string()) {
            Some(config) => config.api_key_env.clone(),
            None => provider.default_api_key_env().to_string(),
        }
    }
    
    /// Get base URL for a provider
    pub fn get_base_url(&self, provider: &Provider) -> String {
        match self.providers.get(&provider.to_string()) {
            Some(config) => config.base_url.clone(),
            None => {
                // Default URLs for known providers
                match provider {
                    Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta".to_string(),
                    Provider::OpenAI => "https://api.openai.com/v1".to_string(),
                    Provider::Anthropic => "https://api.anthropic.com/v1".to_string(),
                    Provider::LMStudio => "http://localhost:1234/v1".to_string(),
                    Provider::Ollama => "http://localhost:11434/api".to_string(),
                    Provider::OpenRouter => "https://openrouter.ai/api/v1".to_string(),
                    Provider::Groq => "https://api.groq.com/openai/v1".to_string(),
                    Provider::DeepSeek => "https://api.deepseek.com/v1".to_string(),
                }
            }
        }
    }
    
    /// Check if a provider is local (doesn't require API key)
    pub fn is_local_provider(&self, provider: &Provider) -> bool {
        match self.providers.get(&provider.to_string()) {
            Some(config) => config.is_local,
            None => {
                matches!(provider, Provider::LMStudio | Provider::Ollama)
            }
        }
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Standardized model names for common use cases
#[derive(Debug, Clone)]
pub enum StandardModel {
    /// Fast, efficient model for simple tasks
    Fast,
    /// Smart, capable model for complex tasks
    Smart,
    /// Specialized reasoning model
    Reasoning,
    /// Code generation model
    Code,
}

impl StandardModel {
    /// Get the appropriate model for a provider and use case
    pub fn get_model_for_provider(&self, provider: &Provider) -> ModelId {
        match self {
            StandardModel::Fast => {
                match provider {
                    Provider::Gemini => ModelId::Gemini25FlashLitePreview0617,
                    Provider::OpenAI => ModelId::GPT5Mini,
                    Provider::Anthropic => ModelId::ClaudeSonnet4, // Anthropic doesn't have a "fast" variant
                    Provider::LMStudio => ModelId::LMStudioLocal,
                    Provider::Ollama => ModelId::OllamaLocal,
                    Provider::OpenRouter => ModelId::OpenRouterModel,
                    Provider::Groq => ModelId::GroqModel,
                    Provider::DeepSeek => ModelId::DeepSeekChat,
                }
            }
            StandardModel::Smart => {
                match provider {
                    Provider::Gemini => ModelId::Gemini25Pro,
                    Provider::OpenAI => ModelId::GPT5,
                    Provider::Anthropic => ModelId::ClaudeOpus41,
                    Provider::LMStudio => ModelId::LMStudioLocal,
                    Provider::Ollama => ModelId::OllamaLocal,
                    Provider::OpenRouter => ModelId::OpenRouterModel,
                    Provider::Groq => ModelId::GroqModel,
                    Provider::DeepSeek => ModelId::DeepSeekReasoner,
                }
            }
            StandardModel::Reasoning => {
                match provider {
                    Provider::Gemini => ModelId::Gemini25ProPreview0605,
                    Provider::OpenAI => ModelId::O3Pro,
                    Provider::Anthropic => ModelId::ClaudeOpus41,
                    Provider::LMStudio => ModelId::LMStudioLocal,
                    Provider::Ollama => ModelId::OllamaLocal,
                    Provider::OpenRouter => ModelId::OpenRouterModel,
                    Provider::Groq => ModelId::GroqModel,
                    Provider::DeepSeek => ModelId::DeepSeekReasoner,
                }
            }
            StandardModel::Code => {
                match provider {
                    Provider::Gemini => ModelId::Gemini25Flash,
                    Provider::OpenAI => ModelId::CodexMiniLatest,
                    Provider::Anthropic => ModelId::ClaudeSonnet4,
                    Provider::LMStudio => ModelId::LMStudioLocal,
                    Provider::Ollama => ModelId::OllamaLocal,
                    Provider::OpenRouter => ModelId::OpenRouterModel,
                    Provider::Groq => ModelId::GroqModel,
                    Provider::DeepSeek => ModelId::DeepSeekChat,
                }
            }
        }
    }
}

/// Get the default provider registry
pub fn get_provider_registry() -> &'static ProviderRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<ProviderRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| ProviderRegistry::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_provider_registry() {
        let registry = ProviderRegistry::new();
        
        // Test known providers
        let gemini_config = registry.get_provider("gemini");
        assert!(gemini_config.is_some());
        assert_eq!(gemini_config.unwrap().api_key_env, "GEMINI_API_KEY");
        
        let lmstudio_config = registry.get_provider("lmstudio");
        assert!(lmstudio_config.is_some());
        assert!(lmstudio_config.unwrap().is_local);
    }
    
    #[test]
    fn test_standard_models() {
        let registry = ProviderRegistry::new();
        
        // Test fast model mapping
        let gemini_fast = StandardModel::Fast.get_model_for_provider(&Provider::Gemini);
        assert_eq!(gemini_fast, ModelId::Gemini25FlashLitePreview0617);
        
        let openai_fast = StandardModel::Fast.get_model_for_provider(&Provider::OpenAI);
        assert_eq!(openai_fast, ModelId::GPT5Mini);
    }
    
    #[test]
    fn test_api_key_env() {
        let registry = ProviderRegistry::new();
        
        assert_eq!(registry.get_api_key_env(&Provider::Gemini), "GEMINI_API_KEY");
        assert_eq!(registry.get_api_key_env(&Provider::OpenAI), "OPENAI_API_KEY");
    }
    
    #[test]
    fn test_base_url() {
        let registry = ProviderRegistry::new();
        
        assert!(registry.get_base_url(&Provider::Gemini).contains("generativelanguage"));
        assert!(registry.get_base_url(&Provider::OpenAI).contains("openai.com"));
        assert!(registry.get_base_url(&Provider::LMStudio).contains("localhost:1234"));
    }
}
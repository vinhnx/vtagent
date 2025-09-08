use super::provider::{LLMError, LLMProvider};
use super::providers::{AnthropicProvider, GeminiProvider, OpenAIProvider};
use std::collections::HashMap;

/// LLM provider factory and registry
pub struct LLMFactory {
    providers: HashMap<String, Box<dyn Fn(String) -> Box<dyn LLMProvider> + Send + Sync>>,
}

impl LLMFactory {
    pub fn new() -> Self {
        let mut factory = Self {
            providers: HashMap::new(),
        };

        // Register built-in providers
        factory.register_provider(
            "gemini",
            Box::new(|api_key| Box::new(GeminiProvider::new(api_key)) as Box<dyn LLMProvider>),
        );

        factory.register_provider(
            "openai",
            Box::new(|api_key| Box::new(OpenAIProvider::new(api_key)) as Box<dyn LLMProvider>),
        );

        factory.register_provider(
            "anthropic",
            Box::new(|api_key| Box::new(AnthropicProvider::new(api_key)) as Box<dyn LLMProvider>),
        );

        factory
    }

    /// Register a new provider
    pub fn register_provider<F>(&mut self, name: &str, factory_fn: F)
    where
        F: Fn(String) -> Box<dyn LLMProvider> + Send + Sync + 'static,
    {
        self.providers
            .insert(name.to_string(), Box::new(factory_fn));
    }

    /// Create provider instance
    pub fn create_provider(
        &self,
        provider_name: &str,
        api_key: String,
    ) -> Result<Box<dyn LLMProvider>, LLMError> {
        let factory_fn = self.providers.get(provider_name).ok_or_else(|| {
            LLMError::InvalidRequest(format!("Unknown provider: {}", provider_name))
        })?;

        Ok(factory_fn(api_key))
    }

    /// List available providers
    pub fn list_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for LLMFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Global factory instance
static mut FACTORY: Option<LLMFactory> = None;
static FACTORY_INIT: std::sync::Once = std::sync::Once::new();

/// Get global factory instance
pub fn get_factory() -> &'static LLMFactory {
    unsafe {
        FACTORY_INIT.call_once(|| {
            FACTORY = Some(LLMFactory::new());
        });
        FACTORY.as_ref().unwrap()
    }
}

/// Create provider from model name and API key
pub fn create_provider_for_model(
    model: &str,
    api_key: String,
) -> Result<Box<dyn LLMProvider>, LLMError> {
    let factory = get_factory();
    let provider_name = factory.provider_from_model(model).ok_or_else(|| {
        LLMError::InvalidRequest(format!("Cannot determine provider for model: {}", model))
    })?;

    factory.create_provider(&provider_name, api_key)
}

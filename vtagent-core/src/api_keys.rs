//! API key management module for secure retrieval from environment variables,
//! .env files, and configuration files.
//!
//! This module provides a unified interface for retrieving API keys for different providers,
//! prioritizing security by checking environment variables first, then .env files, and finally
//! falling back to configuration file values.

use anyhow::{Context, Result};
use std::env;

/// API key sources for different providers
#[derive(Debug, Clone)]
pub struct ApiKeySources {
    /// Gemini API key environment variable name
    pub gemini_env: String,
    /// Anthropic API key environment variable name
    pub anthropic_env: String,
    /// OpenAI API key environment variable name
    pub openai_env: String,
    /// Gemini API key from configuration file
    pub gemini_config: Option<String>,
    /// Anthropic API key from configuration file
    pub anthropic_config: Option<String>,
    /// OpenAI API key from configuration file
    pub openai_config: Option<String>,
}

impl Default for ApiKeySources {
    fn default() -> Self {
        Self {
            gemini_env: "GEMINI_API_KEY".to_string(),
            anthropic_env: "ANTHROPIC_API_KEY".to_string(),
            openai_env: "OPENAI_API_KEY".to_string(),
            gemini_config: None,
            anthropic_config: None,
            openai_config: None,
        }
    }
}

/// Load environment variables from .env file
///
/// This function attempts to load environment variables from a .env file
/// in the current directory, ignoring errors if the file doesn't exist.
pub fn load_dotenv() -> Result<()> {
    // Try to load .env file, but don't fail if it doesn't exist
    let _ = dotenvy::dotenv();
    Ok(())
}

/// Get API key for a specific provider with secure fallback mechanism
///
/// This function implements a secure retrieval mechanism that:
/// 1. First checks environment variables (highest priority for security)
/// 2. Then checks .env file values
/// 3. Falls back to configuration file values if neither above is set
/// 4. Supports all major providers: Gemini, Anthropic, and OpenAI
///
/// # Arguments
///
/// * `provider` - The provider name ("gemini", "anthropic", or "openai")
/// * `sources` - Configuration for where to look for API keys
///
/// # Returns
///
/// * `Ok(String)` - The API key if found
/// * `Err` - If no API key could be found for the provider
pub fn get_api_key(provider: &str, sources: &ApiKeySources) -> Result<String> {
    match provider.to_lowercase().as_str() {
        "gemini" => get_gemini_api_key(sources),
        "anthropic" => get_anthropic_api_key(sources),
        "openai" => get_openai_api_key(sources),
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
    }
}

/// Get API key for a specific environment variable with fallback
fn get_api_key_with_fallback(
    env_var: &str,
    config_value: Option<&String>,
    provider_name: &str,
) -> Result<String> {
    // First try environment variable (most secure)
    if let Ok(key) = env::var(env_var) {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    
    // Then try configuration file value
    if let Some(key) = config_value {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    
    // If neither worked, return an error
    Err(anyhow::anyhow!(
        "No API key found for {} provider. Set {} environment variable or configure in vtagent.toml",
        provider_name,
        env_var
    ))
}

/// Get Gemini API key with secure fallback
fn get_gemini_api_key(sources: &ApiKeySources) -> Result<String> {
    // Try primary Gemini environment variable
    if let Ok(key) = env::var(&sources.gemini_env) {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    
    // Try Google API key as fallback (for backward compatibility)
    if let Ok(key) = env::var("GOOGLE_API_KEY") {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    
    // Try configuration file value
    if let Some(key) = &sources.gemini_config {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }
    
    // If nothing worked, return an error
    Err(anyhow::anyhow!(
        "No API key found for Gemini provider. Set {} or GOOGLE_API_KEY environment variable or configure in vtagent.toml",
        sources.gemini_env
    ))
}

/// Get Anthropic API key with secure fallback
fn get_anthropic_api_key(sources: &ApiKeySources) -> Result<String> {
    get_api_key_with_fallback(
        &sources.anthropic_env,
        sources.anthropic_config.as_ref(),
        "Anthropic"
    )
}

/// Get OpenAI API key with secure fallback
fn get_openai_api_key(sources: &ApiKeySources) -> Result<String> {
    get_api_key_with_fallback(
        &sources.openai_env,
        sources.openai_config.as_ref(),
        "OpenAI"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_gemini_api_key_from_env() {
        // Set environment variable
        env::set_var("TEST_GEMINI_KEY", "test-gemini-key");
        
        let sources = ApiKeySources {
            gemini_env: "TEST_GEMINI_KEY".to_string(),
            ..Default::default()
        };
        
        let result = get_gemini_api_key(&sources);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-gemini-key");
        
        // Clean up
        env::remove_var("TEST_GEMINI_KEY");
    }

    #[test]
    fn test_get_anthropic_api_key_from_env() {
        // Set environment variable
        env::set_var("TEST_ANTHROPIC_KEY", "test-anthropic-key");
        
        let sources = ApiKeySources {
            anthropic_env: "TEST_ANTHROPIC_KEY".to_string(),
            ..Default::default()
        };
        
        let result = get_anthropic_api_key(&sources);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-anthropic-key");
        
        // Clean up
        env::remove_var("TEST_ANTHROPIC_KEY");
    }

    #[test]
    fn test_get_openai_api_key_from_env() {
        // Set environment variable
        env::set_var("TEST_OPENAI_KEY", "test-openai-key");
        
        let sources = ApiKeySources {
            openai_env: "TEST_OPENAI_KEY".to_string(),
            ..Default::default()
        };
        
        let result = get_openai_api_key(&sources);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-openai-key");
        
        // Clean up
        env::remove_var("TEST_OPENAI_KEY");
    }

    #[test]
    fn test_get_gemini_api_key_from_config() {
        let sources = ApiKeySources {
            gemini_config: Some("config-gemini-key".to_string()),
            ..Default::default()
        };
        
        let result = get_gemini_api_key(&sources);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "config-gemini-key");
    }

    #[test]
    fn test_get_api_key_with_fallback_prefers_env() {
        // Set environment variable
        env::set_var("TEST_FALLBACK_KEY", "env-key");
        
        let sources = ApiKeySources {
            openai_env: "TEST_FALLBACK_KEY".to_string(),
            openai_config: Some("config-key".to_string()),
            ..Default::default()
        };
        
        let result = get_openai_api_key(&sources);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "env-key"); // Should prefer env var
        
        // Clean up
        env::remove_var("TEST_FALLBACK_KEY");
    }

    #[test]
    fn test_get_api_key_fallback_to_config() {
        let sources = ApiKeySources {
            openai_env: "NONEXISTENT_ENV_VAR".to_string(),
            openai_config: Some("config-key".to_string()),
            ..Default::default()
        };
        
        let result = get_openai_api_key(&sources);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "config-key");
    }

    #[test]
    fn test_get_api_key_error_when_not_found() {
        let sources = ApiKeySources {
            openai_env: "NONEXISTENT_ENV_VAR".to_string(),
            ..Default::default()
        };
        
        let result = get_openai_api_key(&sources);
        assert!(result.is_err());
    }
}
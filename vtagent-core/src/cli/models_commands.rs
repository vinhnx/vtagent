//! Model management command handlers

use super::args::{Cli, ModelCommands};
use crate::utils::dot_config::{get_dot_manager, load_user_config, save_user_config, DotConfig};
use anyhow::{Result, anyhow};
use owo_colors::*;
use std::collections::HashMap;

/// Handle model management commands
pub async fn handle_models_command(cli: &Cli, command: &ModelCommands) -> Result<()> {
    match command {
        ModelCommands::List => handle_list_models(cli).await,
        ModelCommands::SetProvider { provider } => handle_set_provider(cli, provider).await,
        ModelCommands::SetModel { model } => handle_set_model(cli, model).await,
        ModelCommands::Config { provider, api_key, base_url, model } => {
            handle_config_provider(cli, provider, api_key.as_deref(), base_url.as_deref(), model.as_deref()).await
        }
        ModelCommands::Test { provider } => handle_test_provider(cli, provider).await,
    }
}

/// List all available providers and models
async fn handle_list_models(_cli: &Cli) -> Result<()> {
    println!("{}", "🔧 Available LLM Providers & Models".bold().underline());
    println!();

    let factory = get_factory();

    // Get current configuration
    let config = match load_user_config() {
        Ok(cfg) => cfg,
        Err(_) => DotConfig::default(),
    };

    let current_provider = config.preferences.default_provider;
    let current_model = config.preferences.default_model;

    // List all providers
    let providers = factory.list_providers();

    for provider_name in &providers {
        let is_current = Some(provider_name.clone()) == current_provider;
        let prefix = if is_current { "▶️ " } else { "  " };

        println!("{}{}", prefix, provider_name.to_uppercase().bold());

        // Try to create provider to get supported models
        if let Ok(provider) = create_provider_with_config(
            provider_name,
            Some("dummy_key".to_string()),
            None,
            None,
        ) {
            let models = provider.supported_models();
            for model in models {
                let is_current_model = Some(model.clone()) == current_model;
                let model_prefix = if is_current_model { "  ⭐ " } else { "    " };
                println!("{}{}", model_prefix, model.cyan());
            }
        } else {
            println!("    {}", "❌ Configuration required".red());
        }

        // Show provider configuration status
        let providers_config = &config.providers;
        match provider_name.as_str() {
                "openai" => {
                    if providers_config.openai.as_ref().map(|p| p.enabled).unwrap_or(false) {
                        println!("    {}", "✅ Configured".green());
                    } else {
                        println!("    {}", "⚠️  Not configured".yellow());
                    }
                }
                "anthropic" => {
                    if providers_config.anthropic.as_ref().map(|p| p.enabled).unwrap_or(false) {
                        println!("    {}", "✅ Configured".green());
                    } else {
                        println!("    {}", "⚠️  Not configured".yellow());
                    }
                }
                "gemini" => {
                    if providers_config.gemini.as_ref().map(|p| p.enabled).unwrap_or(false) {
                        println!("    {}", "✅ Configured".green());
                    } else {
                        println!("    {}", "⚠️  Not configured".yellow());
                    }
                }
                "openrouter" => {
                    if providers_config.openrouter.as_ref().map(|p| p.enabled).unwrap_or(false) {
                        println!("    {}", "✅ Configured".green());
                    } else {
                        println!("    {}", "⚠️  Not configured".yellow());
                    }
                }
                "lmstudio" => {
                    if providers_config.lmstudio.as_ref().map(|p| p.enabled).unwrap_or(false) {
                        println!("    {}", "✅ Configured".green());
                    } else {
                        println!("    {}", "⚠️  Not configured".yellow());
                    }
                }
                "ollama" => {
                    if providers_config.ollama.as_ref().map(|p| p.enabled).unwrap_or(false) {
                        println!("    {}", "✅ Configured".green());
                    } else {
                        println!("    {}", "⚠️  Not configured".yellow());
                    }
                }
                _ => {}
            }

        println!();
    }

    // Show current configuration summary
    println!("{}", "📋 Current Configuration".bold().underline());
    println!("Provider: {}", current_provider.unwrap_or("Not set".to_string()).cyan());
    println!("Model: {}", current_model.cyan());
    println!("Temperature: {:.1}", config.preferences.temperature.unwrap_or(0.7));
    println!("Max Tokens: {}", config.preferences.max_tokens.unwrap_or(4096));

    Ok(())
}

/// Set the default provider
async fn handle_set_provider(_cli: &Cli, provider: &str) -> Result<()> {
    // Validate provider exists
    let factory = get_factory();
    let providers = factory.list_providers();

    if !providers.contains(&provider.to_string()) {
        return Err(anyhow!(
            "Unknown provider: {}. Available providers: {:?}",
            provider,
            providers
        ));
    }

    // Update configuration
    let manager = get_dot_manager()?;
    manager.update_config(|config| {
        config.preferences.default_provider = provider.to_string();
    })?;

    println!("✅ Default provider set to: {}", provider.bold().green());
    println!("💡 You may need to configure API keys for this provider using:");
    println!("   vtagent models config {} --api-key YOUR_API_KEY", provider);

    Ok(())
}

/// Set the default model
async fn handle_set_model(_cli: &Cli, model: &str) -> Result<()> {
    // Update configuration
    let manager = get_dot_manager()?;
    manager.update_config(|config| {
        config.preferences.default_model = model.to_string();
    })?;

    println!("✅ Default model set to: {}", model.bold().green());

    Ok(())
}

/// Configure provider settings
async fn handle_config_provider(
    _cli: &Cli,
    provider: &str,
    api_key: Option<&str>,
    base_url: Option<&str>,
    model: Option<&str>,
) -> Result<()> {
    let manager = get_dot_manager()?;
    let mut config = manager.load_config()?;

    // Providers config always exists, no need to check
    let providers = &mut config.providers;

    match provider {
        "openai" => {
            let provider_config = providers.openai.get_or_insert_with(Default::default);
            if let Some(key) = api_key {
                provider_config.api_key = Some(key.to_string());
            }
            if let Some(m) = model {
                provider_config.model = Some(m.to_string());
            }
            provider_config.enabled = api_key.is_some() || provider_config.api_key.is_some();
        }
        "anthropic" => {
            let provider_config = providers.anthropic.get_or_insert_with(Default::default);
            if let Some(key) = api_key {
                provider_config.api_key = Some(key.to_string());
            }
            if let Some(m) = model {
                provider_config.model = Some(m.to_string());
            }
            provider_config.enabled = api_key.is_some() || provider_config.api_key.is_some();
        }
        "gemini" => {
            let provider_config = providers.gemini.get_or_insert_with(Default::default);
            if let Some(key) = api_key {
                provider_config.api_key = Some(key.to_string());
            }
            if let Some(m) = model {
                provider_config.model = Some(m.to_string());
            }
            provider_config.enabled = api_key.is_some() || provider_config.api_key.is_some();
        }
        "openrouter" => {
            let provider_config = providers.openrouter.get_or_insert_with(Default::default);
            if let Some(key) = api_key {
                provider_config.api_key = Some(key.to_string());
            }
            if let Some(m) = model {
                provider_config.model = Some(m.to_string());
            }
            provider_config.enabled = api_key.is_some() || provider_config.api_key.is_some();
        }
        "lmstudio" => {
            let provider_config = providers.lmstudio.get_or_insert_with(Default::default);
            if let Some(key) = api_key {
                provider_config.api_key = Some(key.to_string());
            }
            if let Some(url) = base_url {
                provider_config.base_url = Some(url.to_string());
            }
            if let Some(m) = model {
                provider_config.model = Some(m.to_string());
            }
            provider_config.enabled = true; // LMStudio can work without API key
        }
        "ollama" => {
            let provider_config = providers.ollama.get_or_insert_with(Default::default);
            if let Some(url) = base_url {
                provider_config.base_url = Some(url.to_string());
            }
            if let Some(m) = model {
                provider_config.model = Some(m.to_string());
            }
            provider_config.enabled = true; // Ollama works locally
        }
        _ => {
            return Err(anyhow!("Unknown provider: {}", provider));
        }
    }

    manager.save_config(&config)?;

    println!("✅ Provider {} configured successfully!", provider.bold().green());

    if let Some(key) = api_key {
        println!("   API Key: {}", if key.len() > 8 {
            format!("{}****{}", &key[..4], &key[key.len()-4..])
        } else {
            "****".to_string()
        });
    }

    if let Some(url) = base_url {
        println!("   Base URL: {}", url);
    }

    if let Some(m) = model {
        println!("   Model: {}", m);
    }

    Ok(())
}

/// Test provider connectivity
async fn handle_test_provider(_cli: &Cli, provider: &str) -> Result<()> {
    println!("🔍 Testing {} provider connectivity...", provider);

    // Load configuration
    let config = load_user_config()?;
    let providers = &config.providers;

    // Get provider config
    let (api_key, base_url, model) = match provider {
        "openai" => {
            let cfg = providers.openai.as_ref()
                .ok_or_else(|| anyhow!("OpenAI provider not configured"))?;
            (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone())
        }
        "anthropic" => {
            let cfg = providers.anthropic.as_ref()
                .ok_or_else(|| anyhow!("Anthropic provider not configured"))?;
            (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone())
        }
        "gemini" => {
            let cfg = providers.gemini.as_ref()
                .ok_or_else(|| anyhow!("Gemini provider not configured"))?;
            (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone())
        }
        "openrouter" => {
            let cfg = providers.openrouter.as_ref()
                .ok_or_else(|| anyhow!("OpenRouter provider not configured"))?;
            (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone())
        }
        "lmstudio" => {
            let cfg = providers.lmstudio.as_ref()
                .ok_or_else(|| anyhow!("LMStudio provider not configured"))?;
            (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone())
        }
        "ollama" => {
            let cfg = providers.ollama.as_ref()
                .ok_or_else(|| anyhow!("Ollama provider not configured"))?;
            (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone())
        }
        _ => {
            return Err(anyhow!("Unknown provider: {}", provider));
        }
    };

    // Create provider instance
    let provider_instance = create_provider_with_config(
        provider,
        api_key,
        base_url,
        model,
    )?;

    // Test with a simple prompt
    let test_request = crate::llm::provider::LLMRequest {
        messages: vec![crate::llm::provider::Message {
            role: crate::llm::provider::MessageRole::User,
            content: "Hello! Please respond with just 'OK' if you can read this message.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        system_prompt: None,
        tools: None,
        model: model.unwrap_or_else(|| "test".to_string()),
        max_tokens: Some(10),
        temperature: Some(0.1),
        stream: false,
    };

    match provider_instance.generate(test_request).await {
        Ok(response) => {
            if response.content.as_ref()
                .map(|c| c.to_lowercase().contains("ok"))
                .unwrap_or(false) {
                println!("✅ {} provider test successful!", provider.bold().green());
                println!("   Response: {}", response.content.unwrap_or_default().trim());
            } else {
                println!("⚠️  {} provider responded but with unexpected content", provider.yellow());
                println!("   Response: {}", response.content.unwrap_or_default().trim());
            }
        }
        Err(e) => {
            println!("❌ {} provider test failed: {}", provider.red(), e);
            println!("💡 Make sure your API key and configuration are correct");
        }
    }

    Ok(())
}

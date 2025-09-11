//! VTAgent - Research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{Context, Result, bail};
use clap::Parser;
use console::style;
use std::io::{self, Write};
use std::path::PathBuf;
use vtagent_core::cli::args::{Cli, Commands};
use vtagent_core::config::{ConfigManager, VTAgentConfig};
use vtagent_core::config::models::{ModelId, Provider};
use vtagent_core::config::constants::models;
use vtagent_core::llm::{make_client, AnyClient};
use vtagent_core::llm::factory::create_provider_with_config;
use vtagent_core::llm::provider::{LLMProvider, LLMRequest, Message, MessageRole};
use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::{AgentType, MultiAgentConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match &args.command {
        Some(Commands::ToolPolicy { command }) => {
            println!("ToolPolicy command: {:?}", command);
        }
        Some(Commands::Models { command }) => {
            println!("Models command: {:?}", command);
        }
        Some(Commands::Chat) => {
            if let Err(e) = handle_chat_command(&args).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Ask { prompt }) => {
            if let Err(e) = handle_ask_command(&args, prompt).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::ChatVerbose) => {
            println!("ChatVerbose command - Verbose interactive chat");
        }
        Some(Commands::Analyze) => {
            println!("Analyze command - Analyze workspace");
        }
        Some(Commands::Performance) => {
            println!("Performance command - Display performance metrics");
        }
        Some(Commands::CreateProject { name, features }) => {
            println!("CreateProject command - Name: {}, Features: {:?}", name, features);
        }
        Some(Commands::CompressContext) => {
            println!("CompressContext command - Compress conversation context");
        }
        Some(Commands::DemoAsync) => {
            println!("DemoAsync command - Demo async file operations");
        }
        Some(Commands::Revert { turn, partial }) => {
            println!("Revert command - Turn: {}, Partial: {:?}", turn, partial);
        }
        Some(Commands::Snapshots) => {
            println!("Snapshots command - List all available snapshots");
        }
        Some(Commands::CleanupSnapshots { max }) => {
            println!("CleanupSnapshots command - Max snapshots: {}", max);
        }
        Some(Commands::Init) => {
            println!("Init command - Initialize project with AGENTS.md");
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}

/// Handle the chat command - interactive REPL
async fn handle_chat_command(_args: &Cli) -> Result<()> {
    println!("VT Agent - Interactive AI Coding Assistant");

    // Determine workspace
    let workspace = std::env::current_dir()
        .context("Failed to determine current directory")?;

    // Load configuration
    let config_manager = ConfigManager::load_from_workspace(&workspace)
        .context("Failed to load configuration")?;
    let vtagent_config = config_manager.config();

    // Get model from config or use default
    let mut model_str = vtagent_config.agent.default_model.clone();
    let provider = &vtagent_config.agent.provider;

    // For LMStudio, use the correct model name
    if provider.eq_ignore_ascii_case("lmstudio") && model_str == "local-model" {
        model_str = "qwen/qwen3-4b-2507".to_string();
    }

    // Validate configuration
    if model_str.is_empty() {
        bail!("No model configured. Please set a model in your vtagent.toml configuration file.");
    }

    println!("Using {} with model: {}", provider, model_str);
    println!("Type 'exit' or 'quit' to end the conversation.");
    println!();

    // Check if multi-agent is enabled
    if vtagent_config.multi_agent.enabled {
        println!("{}", style("Multi-agent mode enabled").green().bold());
        handle_multi_agent_chat(&model_str, &workspace, &vtagent_config).await
            .context("Multi-agent chat failed")?;
    } else {
        println!("{}", style("Single agent mode").cyan());
        handle_single_agent_chat(&model_str, provider, &vtagent_config).await
            .context("Single agent chat failed")?;
    }

    Ok(())
}

/// Handle single agent chat mode
async fn handle_single_agent_chat(model_str: &str, provider: &str, config: &VTAgentConfig) -> Result<()> {
    // For LMStudio, use the correct model name
    let model_str = if provider.eq_ignore_ascii_case("lmstudio") && model_str == "local-model" {
        "qwen/qwen3-4b-2507"
    } else {
        model_str
    };

    // Get API key from environment
    let api_key = if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
        // For LMStudio, check for LMSTUDIO_API_KEY first, then fall back to no key
        std::env::var("LMSTUDIO_API_KEY")
            .or_else(|_| {
                eprintln!("Info: No LMSTUDIO_API_KEY found. Using LMStudio without authentication.");
                eprintln!("If LMStudio requires authentication, set LMSTUDIO_API_KEY environment variable.");
                Ok::<String, std::env::VarError>(String::new())
            })
            .unwrap_or_default()
    } else {
        std::env::var(&config.agent.api_key_env)
            .unwrap_or_else(|_| {
                eprintln!("Warning: {} environment variable not set", config.agent.api_key_env);
                String::new()
            })
    };

    // Create client based on provider
    let client: Box<dyn LLMProvider> = if provider.eq_ignore_ascii_case("lmstudio") {
        // For LMStudio, use the correct model name
        let actual_model = if model_str == "local-model" {
            "qwen/qwen3-4b-2507".to_string()
        } else {
            model_str.to_string()
        };

        let client_result = create_provider_with_config(
            "lmstudio",
            Some(api_key),
            Some("http://localhost:1234/v1".to_string()),
            Some(actual_model),
        ).context("Failed to create LMStudio provider")?;

        client_result
    } else {
        // For other providers, we use the model-based approach
        let model_id = model_str.parse::<ModelId>()
            .map_err(|_| anyhow::anyhow!("Invalid model: {}", model_str))?;
        let any_client: AnyClient = make_client(api_key, model_id);
        // We'll use the simple prompt-based approach for other providers for now
        // In a full implementation, we'd want to handle each provider properly
        return handle_simple_prompt_chat(any_client).await
            .context("Simple prompt chat failed");
    };

    // Initialize conversation history
    let mut conversation_history = vec![
        Message {
            role: MessageRole::System,
            content: "You are a helpful coding assistant. You can help with programming tasks, code analysis, and file operations.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }
    ];

    loop {
        print!("> ");
        io::stdout().flush()
            .context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)
            .context("Failed to read input")?;

        // Check for EOF (when piping input)
        if bytes_read == 0 {
            break;
        }

        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Add user message to history
        conversation_history.push(Message {
            role: MessageRole::User,
            content: input.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });

        // Create request
        let request = LLMRequest {
            messages: conversation_history.clone(),
            system_prompt: None,
            tools: None,
            model: model_str.to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: false,
        };

        // Get response from AI
        match client.generate(request).await {
            Ok(response) => {
                if let Some(content) = response.content {
                    println!("{}", content);
                    // Add AI response to history
                    conversation_history.push(Message {
                        role: MessageRole::Assistant,
                        content: content.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                } else {
                    eprintln!("{}: Empty response from AI", style("Warning").yellow());
                }
            }
            Err(e) => {
                eprintln!("{}: {:?}", style("Error").red(), e);
            }
        }
    }

    Ok(())
}

/// Handle simple prompt-based chat for non-LMStudio providers
async fn handle_simple_prompt_chat(mut client: AnyClient) -> Result<()> {
    // Initialize conversation history
    let mut conversation_history = vec![
        "You are a helpful coding assistant. You can help with programming tasks, code analysis, and file operations.".to_string()
    ];

    loop {
        print!("> ");
        io::stdout().flush()
            .context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)
            .context("Failed to read input")?;

        // Check for EOF (when piping input)
        if bytes_read == 0 {
            break;
        }

        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Add user message to history
        conversation_history.push(input.to_string());

        // Create a simple prompt by joining the conversation history
        let prompt = conversation_history.join("\n");

        // Get response from AI
        match client.generate(&prompt).await {
            Ok(response) => {
                println!("{}", response.content);
                // Add AI response to history
                conversation_history.push(response.content.clone());
            }
            Err(e) => {
                eprintln!("{}: {:?}", style("Error").red(), e);
            }
        }
    }

    Ok(())
}

/// Handle multi-agent chat mode
async fn handle_multi_agent_chat(model_str: &str, workspace: &PathBuf, config: &VTAgentConfig) -> Result<()> {
    // Determine if we need to use fallback models for multi-agent
    let (orchestrator_model, subagent_model) = if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
        // LMStudio now supports multi-agent mode with local models
        eprintln!("Info: Using LMStudio local models for multi-agent system.");
        // Use the actual model name that works with LMStudio
        ("qwen/qwen3-2507".to_string(), "qwen/qwen3-2507".to_string())
    } else {
        (model_str.to_string(), model_str.to_string())
    };

    // Create multi-agent configuration
    let multi_config = if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
        // Use LMStudio provider for multi-agent mode
        MultiAgentConfig {
            enable_multi_agent: true,
            enable_task_management: true,
            enable_context_sharing: true,
            enable_performance_monitoring: true,
            provider: Provider::LMStudio,
            orchestrator_model: orchestrator_model.clone(),
            subagent_model: subagent_model.clone(),
            max_concurrent_subagents: 3,
            task_timeout: std::time::Duration::from_secs(60),
            context_window_size: 4096,
            max_context_items: 50,
            ..Default::default()
        }
    } else {
        MultiAgentConfig {
            enable_multi_agent: true,
            enable_task_management: true,
            enable_context_sharing: true,
            enable_performance_monitoring: true,
            orchestrator_model,
            subagent_model,
            max_concurrent_subagents: 3,
            task_timeout: std::time::Duration::from_secs(60),
            context_window_size: 4096,
            max_context_items: 50,
            ..Default::default()
        }
    };

    // Get API key from environment
    let api_key = if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
        // For LMStudio, check for LMSTUDIO_API_KEY first, then fall back to no key
        std::env::var("LMSTUDIO_API_KEY")
            .or_else(|_| {
                eprintln!("Info: No LMSTUDIO_API_KEY found. Using LMStudio without authentication.");
                eprintln!("If LMStudio requires authentication, set LMSTUDIO_API_KEY environment variable.");
                Ok::<String, std::env::VarError>(String::new())
            })
            .unwrap_or_default()
    } else {
        std::env::var(&config.agent.api_key_env)
            .unwrap_or_else(|_| {
                eprintln!("Warning: {} environment variable not set", config.agent.api_key_env);
                String::new()
            })
    };

    // Create multi-agent system
    let mut system = MultiAgentSystem::new(multi_config, api_key, workspace.clone()).await
        .context("Failed to initialize multi-agent system")?;

    loop {
        print!("> ");
        io::stdout().flush()
            .context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)
            .context("Failed to read input")?;

        // Check for EOF (when piping input)
        if bytes_read == 0 {
            break;
        }

        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Execute task with multi-agent system
        match system.execute_task_optimized(
            "User Task".to_string(),
            input.to_string(),
            AgentType::Coder,
        ).await {
            Ok(task_result) => {
                println!("{}", task_result.results.summary);
            }
            Err(e) => {
                eprintln!("{}: {}", style("Error").red(), e);
            }
        }
    }

    // Shutdown system
    system.shutdown().await
        .context("Failed to shutdown multi-agent system")?;

    Ok(())
}

/// Handle the ask command - single prompt mode
async fn handle_ask_command(_args: &Cli, prompt: &[String]) -> Result<()> {
    println!("Ask command - Single prompt mode: {:?}", prompt);
    Ok(())
}
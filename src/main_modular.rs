//! VTCode - Research-preview Rust coding agent
//!
//! This is the main binary entry point for VTCode with modular CLI architecture.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use std::path::PathBuf;
use vtcode_core::api_keys::{get_api_key, load_dotenv, ApiKeySources};
use vtcode_core::config::loader::ConfigManager;
use vtcode_core::constants::defaults;
use vtcode_core::{
    config::ConfigManager,
    models::{ModelId, Provider},
    safety::SafetyValidator,
    types::AgentConfig as CoreAgentConfig,
};

mod cli;

use cli::*;

/// Main CLI structure for VTCode
#[derive(Parser, Debug)]
#[command(name = "vtcode", version, about = "minimal coding agent")]
pub struct Cli {
    /// Gemini model ID (e.g., gemini-2.5-flash-lite-preview-06-17)
    #[arg(long, global = true, default_value = defaults::DEFAULT_CLI_MODEL)]
    pub model: String,

    /// API key environment variable to read
    #[arg(long, global = true, default_value = defaults::DEFAULT_API_KEY_ENV)]
    pub api_key_env: String,

    /// Workspace root directory for file operations
    #[arg(long, global = true)]
    pub workspace: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Skip safety confirmations (use with caution)
    #[arg(long, global = true)]
    pub skip_confirmations: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive chat mode (default)
    Chat,
    /// Interactive chat with verbose logging
    ChatVerbose,
    /// Ask a single question
    Ask { prompt: Vec<String> },
    /// Analyze workspace structure
    Analyze,
    /// Create a new project
    CreateProject { name: String, features: Vec<String> },
    /// Initialize VTCode configuration
    Init {
        #[arg(long)]
        force: bool,
        /// Run vtcode after initialization
        #[arg(long, default_value_t = false)]
        run: bool,
    },
    /// Generate configuration file
    Config {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Show performance metrics
    Performance,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    load_dotenv()?;

    let args = Cli::parse();

    // Determine workspace
    let workspace = args
        .workspace
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // Load configuration
    let config_manager =
        ConfigManager::load_from_workspace(&workspace).context("Failed to load configuration")?;
    let vtcode_config = config_manager.config();

    // Create API key sources configuration
    let api_key_sources = ApiKeySources {
        gemini_env: "GEMINI_API_KEY".to_string(),
        anthropic_env: "ANTHROPIC_API_KEY".to_string(),
        openai_env: "OPENAI_API_KEY".to_string(),
        gemini_config: vtcode_config.agent.gemini_api_key.clone(),
        anthropic_config: vtcode_config.agent.anthropic_api_key.clone(),
        openai_config: vtcode_config.agent.openai_api_key.clone(),
    };

    // Parse model
    let model = ModelId::from_str(&args.model)?;

    // Get API key using our new secure retrieval system
    let provider_name = model.provider_name(); // This would need to be implemented

    let api_key = if !args.api_key_env.is_empty() && args.api_key_env != "GEMINI_API_KEY" {
        // Use explicit API key environment variable from command line
        std::env::var(&args.api_key_env)
            .with_context(|| format!("Environment variable {} not set", args.api_key_env))?
    } else {
        // Use our new secure API key retrieval system
        get_api_key(&provider_name, &api_key_sources)?
    };

    // Create agent configuration
    let mut config = CoreAgentConfig {
        model: model.clone(),
        api_key: api_key.clone(),
        workspace: workspace.clone(),
        verbose: args.verbose,
        theme: defaults::DEFAULT_THEME.to_string(),
    };

    // Apply safety validations for model usage
    let validated_model = SafetyValidator::validate_model_usage(
        &config.model,
        Some("Interactive coding session"),
        args.skip_confirmations,
    )?;

    config.model = validated_model;

    // Dispatch to appropriate command handler
    match args.command.unwrap_or(Commands::Chat) {
        Commands::Chat => {
            handle_chat_command(&config, args.skip_confirmations, false).await?;
        }
        Commands::ChatVerbose => {
            println!("{}", style("Verbose chat mode selected").blue().bold());
            handle_chat_command(&config, args.skip_confirmations, false).await?;
        }
        Commands::Ask { prompt } => {
            let prompt_text = prompt.join(" ");
            println!("{}", style("Ask mode").blue().bold());
            println!("Question: {}", prompt_text);

            // Ask implementation
            // Create a simple LLM client and get a response
            let client = vtcode_core::llm::make_client(
                config.api_key.clone(),
                config.model.parse().unwrap_or_default(),
            );

            // For a minimal implementation, we'll just print a placeholder response
            // In a full implementation, this would actually call the LLM
            println!(
                "Answer: This is a placeholder response. In a full implementation, this would call the LLM with your question."
            );
        }
        Commands::Analyze => {
            handle_analyze_command(&config).await?;
        }
        Commands::CreateProject { name, features } => {
            handle_create_project_command(&config, &name, &features).await?;
        }
        Commands::Init { force, run } => {
            handle_init_command(&workspace, force, run).await?;
        }
        Commands::Config { output } => {
            handle_config_command(output.as_deref()).await?;
        }
        Commands::Performance => {
            println!(
                "{}",
                style("Performance metrics mode selected").blue().bold()
            );
            handle_performance_command().await?;
        }
    }

    if args.verbose {
        println!("\n{}", style("Verbose mode enabled").dim());
        println!("Configuration:");
        println!("  Model: {}", config.model);
        println!("  Workspace: {}", config.workspace.display());
        println!("  API Key Source: {}", args.api_key_env);
    }

    println!(
        "\n{}",
        style("Ready to assist with your coding tasks!")
            .cyan()
            .bold()
    );

    Ok(())
}

use anyhow::{Context, Result};
use console::style;
use vtagent_core::{
    config::ConfigManager,
    gemini::Tool,
    llm::make_client,
    tools::{ToolRegistry, build_function_declarations},
    types::AgentConfig as CoreAgentConfig,
    utils::summarize_workspace_languages,
};

/// Handle the chat command
pub async fn handle_chat_command(config: &CoreAgentConfig, force_multi_agent: bool, skip_confirmations: bool) -> Result<()> {
    eprintln!("[DEBUG] Entering handle_chat_command");
    eprintln!("[DEBUG] Workspace: {:?}", config.workspace);
    eprintln!("[DEBUG] Model: {}", config.model);

    println!("{}", style("Interactive chat mode selected").blue().bold());
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());
    
    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        println!("Detected languages: {}", summary);
        eprintln!("[DEBUG] Language detection: {}", summary);
    }
    println!();

    // Create model-agnostic client
    let mut client = make_client(config.api_key.clone(), config.model.clone());

    // Initialize tool registry and function declarations
    let tool_registry = ToolRegistry::new(config.workspace.clone());
    tool_registry.initialize_async().await?;
    let function_declarations = build_function_declarations();
    let tools = vec![Tool {
        function_declarations,
    }];

    // Load configuration from vtagent.toml first
    let config_manager = ConfigManager::load_from_workspace(&config.workspace)
        .context("Failed to load configuration")?;
    let vtcode_config = config_manager.config();

    // Multi-agent mode logic
    if force_multi_agent || vtcode_config.multi_agent.enabled {
        println!("{}", style("Multi-agent mode enabled").green().bold());
        // Multi-agent implementation would go here
        return Ok(());
    }

    // Single agent mode
    println!("{}", style("Single agent mode").cyan());
    println!("Type 'exit' to quit, 'help' for commands");
    
    // Chat loop implementation would go here
    println!("Chat functionality not fully implemented in this minimal version");
    
    Ok(())
}

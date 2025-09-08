use anyhow::{Context, Result};
use console::style;
use vtagent_core::{
    config::ConfigManager,
    gemini::{Tool, GenerateContentRequest, Content, Part},
    llm::make_client,
    models::ModelId,
    tools::{ToolRegistry, build_function_declarations},
    types::AgentConfig as CoreAgentConfig,
    utils::summarize_workspace_languages,
};
use std::io::{self, Write};

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
    let model_id = config.model.parse::<ModelId>().map_err(|_| {
        anyhow::anyhow!("Invalid model: {}", config.model)
    })?;
    let mut client = make_client(config.api_key.clone(), model_id);

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
        println!("Multi-agent functionality not fully implemented in this minimal version");
        return Ok(());
    }

    // Single agent mode - Chat loop implementation
    println!("{}", style("Single agent mode").cyan());
    println!("Type 'exit' to quit, 'help' for commands");

    // Initialize conversation history
    let mut conversation_history = vec![
        Content::system_text("You are a helpful coding assistant. You can help with programming tasks, code analysis, and file operations.")
    ];

    loop {
        // Get user input
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Handle special commands
        match input {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  exit/quit - Exit the chat");
                println!("  help - Show this help message");
                println!("  Any other text will be sent to the AI assistant");
                continue;
            }
            "" => continue,
            _ => {}
        }

        // Add user message to history
        conversation_history.push(Content::user_text(input));

        // Create request
        let request = GenerateContentRequest {
            contents: conversation_history.clone(),
            tools: Some(tools.clone()),
            tool_config: None,
            system_instruction: None,
            generation_config: None,
        };

        // Get response from AI
        match client.generate_content(&request).await {
            Ok(response) => {
                if let Some(candidate) = response.candidates.first() {
                    if let Some(part) = candidate.content.parts.first() {
                        if let Some(text) = part.as_text() {
                            println!("{}", text);
                            // Add AI response to history
                            conversation_history.push(Content::model_text(text));
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
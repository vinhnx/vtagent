//! VTAgent - Research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{Context, Result, bail};
use clap::Parser;
use console::style;
use std::io::{self, Write};
use std::path::PathBuf;
use vtagent_core::cli::args::{Cli, Commands};
use vtagent_core::config::constants::{models, prompts};
use vtagent_core::config::models::{ModelId, Provider};
use vtagent_core::config::{ConfigManager, VTAgentConfig};
use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::{AgentType, MultiAgentConfig};
use vtagent_core::llm::factory::create_provider_with_config;
use vtagent_core::llm::provider::{LLMProvider, LLMRequest, Message, MessageRole};
use vtagent_core::llm::{AnyClient, make_client};

/// Load system prompt from configuration or default path
fn load_system_prompt(_config: &VTAgentConfig) -> Result<String> {
    use std::fs;

    // Note: VTAgentConfig doesn't have prompts section yet, so use default for now
    let path = prompts::DEFAULT_SYSTEM_PROMPT_PATH;

    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(_) => {
            // If the file doesn't exist, return a simple fallback prompt with helpful error
            eprintln!(
                "Warning: Could not read system prompt from '{}'. Using fallback prompt.",
                path
            );
            eprintln!(
                "To customize: Create '{}' or configure [prompts].system in vtagent.toml",
                path
            );
            Ok("You are a helpful coding assistant for the VTAgent Rust project with access to file operations.\n\nMANDATORY TOOL USAGE:\n- When user asks 'what is this project about' or similar: IMMEDIATELY call list_files to see project structure, then call read_file on README.md\n- When user asks about code or files: Use read_file to read the relevant files\n- When user asks about project structure: Use list_files first\n\nTOOL CALL FORMAT: Always respond with a function call when you need to use tools. Do not give text responses for project questions without using tools first.\n\nAvailable tools:\n- list_files: List files and directories\n- read_file: Read file contents\n- rp_search: Search for patterns in code\n- run_terminal_cmd: Execute terminal commands".to_string())
        }
    }
}

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
            println!(
                "CreateProject command - Name: {}, Features: {:?}",
                name, features
            );
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
    let workspace = std::env::current_dir().context("Failed to determine current directory")?;

    // Load configuration
    let config_manager =
        ConfigManager::load_from_workspace(&workspace).context("Failed to load configuration")?;
    let vtagent_config = config_manager.config();

    // Get model from config or use default
    let mut model_str = vtagent_config.agent.default_model.clone();
    let provider = &vtagent_config.agent.provider;

    // For LMStudio, use the correct model name
    if provider.eq_ignore_ascii_case("lmstudio") && model_str == models::LMSTUDIO_LOCAL {
        model_str = models::LMSTUDIO_QWEN_30B_A3B_2507.to_string();
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
        handle_multi_agent_chat(&model_str, &workspace, &vtagent_config)
            .await
            .context("Multi-agent chat failed")?;
    } else {
        println!("{}", style("Single agent mode").cyan());
        // For Gemini and other providers, use the single-agent chat with tools
        handle_single_agent_chat(&model_str, provider, &vtagent_config)
            .await
            .context("Single agent chat failed")?;
    }

    Ok(())
}

/// Handle single agent chat mode
async fn handle_single_agent_chat(
    model_str: &str,
    provider: &str,
    config: &VTAgentConfig,
) -> Result<()> {
    use vtagent_core::llm::provider::ToolDefinition;

    println!("DEBUG: Entered handle_single_agent_chat function");

    // Initialize tool registry and function declarations
    let mut tool_registry =
        vtagent_core::tools::ToolRegistry::new(std::env::current_dir().unwrap_or_default());
    tool_registry.initialize_async().await?;
    let function_declarations = vtagent_core::tools::build_function_declarations();

    // Convert FunctionDeclaration to ToolDefinition
    let tool_definitions: Vec<ToolDefinition> = function_declarations
        .into_iter()
        .map(|fd| ToolDefinition {
            name: fd.name,
            description: fd.description,
            parameters: fd.parameters,
        })
        .collect();

    println!(
        "DEBUG: Available tools: {:?}",
        tool_definitions.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    // For LMStudio, use the correct model name
    let model_str =
        if provider.eq_ignore_ascii_case("lmstudio") && model_str == models::LMSTUDIO_LOCAL {
            models::LMSTUDIO_QWEN_30B_A3B_2507
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
        std::env::var(&config.agent.api_key_env).unwrap_or_else(|_| {
            eprintln!(
                "Warning: {} environment variable not set",
                config.agent.api_key_env
            );
            String::new()
        })
    };

    // Create client based on provider
    let client: Box<dyn LLMProvider> = if provider.eq_ignore_ascii_case("lmstudio") {
        // For LMStudio, use the correct model name
        let actual_model = if model_str == models::LMSTUDIO_LOCAL {
            models::LMSTUDIO_QWEN_30B_A3B_2507.to_string()
        } else {
            model_str.to_string()
        };

        let client_result = create_provider_with_config(
            "lmstudio",
            Some(api_key),
            Some("http://localhost:1234/v1".to_string()),
            Some(actual_model),
        )
        .context("Failed to create LMStudio provider")?;

        client_result
    } else {
        // For Gemini and other providers, create the appropriate client
        if provider.eq_ignore_ascii_case("gemini") {
            // Create Gemini client
            let client_result = create_provider_with_config(
                "gemini",
                Some(api_key),
                None, // Gemini doesn't need a base URL
                Some(model_str.to_string()),
            )
            .context("Failed to create Gemini provider")?;
            client_result
        } else {
            // For other providers, we use the model-based approach
            let model_id = model_str
                .parse::<ModelId>()
                .map_err(|_| anyhow::anyhow!("Invalid model: {}", model_str))?;
            let any_client: AnyClient = make_client(api_key, model_id);
            // We'll use the simple prompt-based approach for other providers for now
            // In a full implementation, we'd want to handle each provider properly
            return handle_simple_prompt_chat(any_client)
                .await
                .context("Simple prompt chat failed");
        }
    };

    // Load system prompt from configuration or default path
    let system_prompt = load_system_prompt(config)?;

    // Initialize conversation history
    let mut conversation_history = vec![Message {
        role: MessageRole::System,
        content: system_prompt,
        tool_calls: None,
        tool_call_id: None,
    }];

    loop {
        println!("DEBUG: Starting input loop iteration");
        print!("> ");
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;

        // Check for EOF (when piping input)
        if bytes_read == 0 {
            break;
        }

        let input = input.trim();

        println!("DEBUG: Processing input: '{}'", input);

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

        // Auto-gather context for project questions
        let is_project_question = input.to_lowercase().contains("project")
            || input.to_lowercase().contains("what is this")
            || input.to_lowercase().contains("readme")
            || input.to_lowercase().contains("about");

        println!(
            "DEBUG: Input: '{}', Is project question: {}",
            input, is_project_question
        );

        if is_project_question {
            println!("{}: Gathering project context...", style("Context").green());

            let mut context_parts = Vec::new();

            // Try to read README.md
            match tool_registry
                .execute_tool("read_file", serde_json::json!({"path": "README.md"}))
                .await
            {
                Ok(result) => {
                    println!("{}: Found README.md", style("✅").green());
                    context_parts.push(format!("README.md contents:\n{}", result));
                }
                Err(e) => {
                    println!("{}: Could not read README.md: {}", style("⚠️").yellow(), e);
                }
            }

            // Try to list files in root directory
            match tool_registry
                .execute_tool("list_files", serde_json::json!({"path": "."}))
                .await
            {
                Ok(result) => {
                    println!("{}: Listed project files", style("✅").green());
                    context_parts.push(format!("Project structure:\n{}", result));
                }
                Err(e) => {
                    println!("{}: Could not list files: {}", style("⚠️").yellow(), e);
                }
            }

            // Add context to the user message
            if !context_parts.is_empty() {
                let context = context_parts.join("\n\n");
                let user_message = format!(
                    "Question: {}\n\nProject Context:\n{}\n\nPlease answer the question based on the project context provided above.",
                    input, context
                );
                println!(
                    "DEBUG: Sending message with context (length: {} chars)",
                    user_message.len()
                );
                conversation_history.push(Message {
                    role: MessageRole::User,
                    content: user_message,
                    tool_calls: None,
                    tool_call_id: None,
                });
            } else {
                // No context gathered, proceed with original message
                conversation_history.push(Message {
                    role: MessageRole::User,
                    content: input.to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        } else {
            // No context gathering needed, proceed with original message
            conversation_history.push(Message {
                role: MessageRole::User,
                content: input.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Create request (moved from after user message addition)
        let request = LLMRequest {
            messages: conversation_history.clone(),
            system_prompt: None,
            tools: Some(tool_definitions.clone()),
            model: model_str.to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: false,
        };

        // Get response from AI
        match client.generate(request).await {
            Ok(response) => {
                // Handle tool calls first
                if let Some(tool_calls) = &response.tool_calls {
                    println!(
                        "{}: Executing {} tool call(s)",
                        style("Tool").blue().bold(),
                        tool_calls.len()
                    );

                    for tool_call in tool_calls {
                        println!(
                            "{}: Calling tool: {} with args: {}",
                            style("TOOL").cyan(),
                            tool_call.name,
                            tool_call.arguments
                        );

                        // Execute the tool
                        match tool_registry
                            .execute_tool(&tool_call.name, tool_call.arguments.clone())
                            .await
                        {
                            Ok(result) => {
                                println!(
                                    "{}: Tool {} executed successfully",
                                    style("✅").green(),
                                    tool_call.name
                                );

                                // Add tool result to conversation
                                conversation_history.push(Message {
                                    role: MessageRole::Tool,
                                    content: serde_json::to_string(&result).unwrap_or_default(),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                            }
                            Err(e) => {
                                println!(
                                    "{}: Tool {} failed: {}",
                                    style("❌").red(),
                                    tool_call.name,
                                    e
                                );

                                // Add error result to conversation
                                conversation_history.push(Message {
                                    role: MessageRole::Tool,
                                    content: format!("Tool {} failed: {}", tool_call.name, e),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                            }
                        }
                    }

                    // After executing tools, send another request to get the final response
                    let follow_up_request = LLMRequest {
                        messages: conversation_history.clone(),
                        system_prompt: None,
                        tools: Some(tool_definitions.clone()),
                        model: model_str.to_string(),
                        max_tokens: Some(1000),
                        temperature: Some(0.7),
                        stream: false,
                    };

                    match client.generate(follow_up_request).await {
                        Ok(final_response) => {
                            if let Some(content) = final_response.content {
                                println!("{}", content);
                                // Add final AI response to history
                                conversation_history.push(Message {
                                    role: MessageRole::Assistant,
                                    content: content.clone(),
                                    tool_calls: None,
                                    tool_call_id: None,
                                });
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "{}: Error in follow-up request: {:?}",
                                style("Error").red(),
                                e
                            );
                        }
                    }
                } else if let Some(content) = response.content {
                    // No tool calls, just print the response
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
    // Load system prompt - we don't have config here, so use a simple fallback
    let system_prompt = std::fs::read_to_string(prompts::DEFAULT_SYSTEM_PROMPT_PATH)
        .unwrap_or_else(|_| "You are a helpful coding assistant. You can help with programming tasks, code analysis, and file operations.".to_string());

    // Initialize conversation history
    let mut conversation_history = vec![system_prompt];

    loop {
        print!("> ");
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin()
            .read_line(&mut input)
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
async fn handle_multi_agent_chat(
    model_str: &str,
    workspace: &PathBuf,
    config: &VTAgentConfig,
) -> Result<()> {
    // Determine if we need to use fallback models for multi-agent
    let (orchestrator_model, subagent_model) =
        if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
            // LMStudio now supports multi-agent mode with local models
            eprintln!("Info: Using LMStudio local models for multi-agent system.");
            // Use the actual model name that works with LMStudio
            (
                models::LMSTUDIO_QWEN_30B_A3B_2507.to_string(),
                models::LMSTUDIO_QWEN_30B_A3B_2507.to_string(),
            )
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
        std::env::var(&config.agent.api_key_env).unwrap_or_else(|_| {
            eprintln!(
                "Warning: {} environment variable not set",
                config.agent.api_key_env
            );
            String::new()
        })
    };

    // Create multi-agent system
    let mut system = MultiAgentSystem::new(multi_config, api_key, workspace.clone())
        .await
        .context("Failed to initialize multi-agent system")?;

    loop {
        print!("> ");
        io::stdout().flush().context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin()
            .read_line(&mut input)
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
        match system
            .execute_task_optimized("User Task".to_string(), input.to_string(), AgentType::Coder)
            .await
        {
            Ok(task_result) => {
                println!("{}", task_result.results.summary);
            }
            Err(e) => {
                eprintln!("{}: {}", style("Error").red(), e);
            }
        }
    }

    // Shutdown system
    system
        .shutdown()
        .await
        .context("Failed to shutdown multi-agent system")?;

    Ok(())
}

/// Handle the ask command - single prompt mode
async fn handle_ask_command(_args: &Cli, prompt: &[String]) -> Result<()> {
    println!("Ask command - Single prompt mode: {:?}", prompt);
    Ok(())
}

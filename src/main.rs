//! VTAgent - Research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{Context, Result, bail};
use clap::Parser;
use console::style;
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::path::PathBuf;
use vtagent_core::cli::args::{Cli, Commands};
use vtagent_core::config::constants::{models, prompts, tools};
use vtagent_core::config::models::{ModelId, Provider};
use vtagent_core::config::{ConfigManager, VTAgentConfig};
use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::{AgentType, MultiAgentConfig};
use vtagent_core::llm::factory::create_provider_with_config;
use vtagent_core::llm::provider::{LLMProvider, LLMRequest, Message, MessageRole};
use vtagent_core::llm::{AnyClient, make_client};

/// Load project-specific context for better agent performance
async fn load_project_context(
    project_manager: &vtagent_core::project::ProjectManager,
    project_name: &str,
) -> Result<Vec<String>> {
    let mut context_items = Vec::new();

    // Load project metadata
    if let Some(metadata) = project_manager.load_project_metadata(project_name)? {
        context_items.push(format!("Project: {}", metadata.name));
        if let Some(description) = metadata.description {
            context_items.push(format!("Description: {}", description));
        }
        context_items.push(format!("Created: {}", metadata.created_at));
    }

    // Load README.md if it exists
    let readme_paths = ["README.md", "README.txt", "README"];
    for readme_path in &readme_paths {
        let readme_file = std::path::Path::new(readme_path);
        if readme_file.exists() {
            if let Ok(content) = std::fs::read_to_string(readme_file) {
                // Truncate to reasonable size for context
                let truncated = if content.len() > 2000 {
                    format!("{}...", &content[..2000])
                } else {
                    content
                };
                context_items.push(format!("README content: {}", truncated));
                break;
            }
        }
    }

    // Load key project files for context
    let key_files = ["Cargo.toml", "package.json", "requirements.txt", "Gemfile"];
    for key_file in &key_files {
        let file_path = std::path::Path::new(key_file);
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                // Truncate to reasonable size for context
                let truncated = if content.len() > 1000 {
                    format!("{}...", &content[..1000])
                } else {
                    content
                };
                context_items.push(format!("{} content: {}", key_file, truncated));
            }
        }
    }

    Ok(context_items)
}

/// Handle the ask command - single prompt mode
async fn handle_ask_command(_args: &Cli, prompt: &[String]) -> Result<()> {
    println!("Ask command - Single prompt mode: {:?}", prompt);
    Ok(())
}
    use std::fs;
    use std::path::Path;

    // Try multiple possible paths for the system prompt file
    let possible_paths = [
        "prompts/system.md",           // From project root
        "../prompts/system.md",        // From src/ or vtagent-core/
        "../../prompts/system.md",     // From deeper subdirectories
        "vtagent-core/src/prompts/system.md", // Direct path
    ];

    for path in &possible_paths {
        if Path::new(path).exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    // Try to extract the system prompt from markdown format
                    if let Some(start) = content.find("```rust\nr#\"") {
                        if let Some(end) = content[start..].find("\"#\n```") {
                            let prompt_start = start + 9; // Skip ```rust\nr#"
                            let prompt_end = start + end;
                            return Ok(content[prompt_start..prompt_end].to_string());
                        }
                    }
                    // If not in markdown format, return as-is
                    return Ok(content);
                }
                Err(_) => continue,
            }
        }
    }

    // Fallback to inline prompt if file not found
    eprintln!("Warning: Could not find system.md file, using inline fallback");
    Ok("You are a helpful coding assistant for the VTAgent Rust project with access to file operations.\n\n## IMPORTANT: ALWAYS USE TOOLS FOR FILE OPERATIONS\n\nWhen user asks to edit files, modify code, or add content:\n1. FIRST: Use read_file to understand the current file structure\n2. THEN: Use edit_file to make specific text replacements, OR use write_file to rewrite entire files\n3. Do NOT try to use terminal commands (sed, awk, etc.) for file editing\n\nWhen user asks about project questions:\n1. FIRST: Use list_files to see project structure\n2. THEN: Use read_file on relevant files like README.md\n\n## AVAILABLE TOOLS:\n- read_file: Read file contents to understand structure\n- write_file: Create new files or completely rewrite existing ones\n- edit_file: Replace specific text in files (use this for targeted edits)\n- list_files: List files and directories in a path\n- rp_search: Search for patterns in code\n- run_terminal_cmd: Execute terminal commands (NOT for file editing)\n\n## TOOL USAGE EXAMPLES:\nTo add a model constant:\n1. read_file('path/to/constants.rs') - understand structure\n2. edit_file(path='path/to/constants.rs', old_str='    pub const LAST_CONST: &str = \"value\";', new_str='    pub const LAST_CONST: &str = \"value\";\n    pub const NEW_CONST: &str = \"new_value\";')\n\nALWAYS use function calls, not text responses, when files need to be read or modified.".to_string())
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
        Some(Commands::Config { output, global }) => {
            if let Err(e) = handle_config_command(output.as_deref(), global).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::InitProject { name, force, migrate }) => {
            if let Err(e) = handle_init_project_command(name, force, migrate).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        None => {
            println!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}

/// Handle the chat command - interactive REPL
async fn handle_chat_command(args: &Cli) -> Result<()> {
    println!("VT Agent - Interactive AI Coding Assistant");

    // Determine workspace
    let workspace = std::env::current_dir().context("Failed to determine current directory")?;

    // Load configuration
    let config_manager = ConfigManager::load_from_workspace(&workspace)
        .context("Failed to load configuration")?;
    let vtagent_config = config_manager.config();

    // Initialize project-specific systems if available
    if let (Some(project_manager), Some(project_name)) =
        (config_manager.project_manager(), config_manager.project_name()) {

        println!("Project: {}", project_name);

        // Initialize cache
        let cache_dir = project_manager.cache_dir(project_name);
        let cache = vtagent_core::project::FileCache::new(cache_dir)
            .context("Failed to initialize project cache")?;

        // Clean expired cache entries
        if let Ok(cleaned) = cache.clean_expired() {
            if cleaned > 0 {
                println!("Cleaned {} expired cache entries", cleaned);
            }
        }

        // Load project-specific context for better agent performance
        let project_context = load_project_context(project_manager, project_name)
            .await
            .unwrap_or_default();

        if !project_context.is_empty() {
            println!("Loaded project context ({} items)", project_context.len());
        }
    }

    // Get model from config or use default
    let mut model_str = vtagent_config.agent.default_model.clone();
    let provider = &vtagent_config.agent.provider;

    // For LMStudio, use the configured single agent model
    if provider.eq_ignore_ascii_case("lmstudio") {
        if model_str == models::LMSTUDIO_LOCAL || model_str.is_empty() {
            model_str = vtagent_config.lmstudio.single_agent_model.clone();
        }
    }

    // Validate configuration
    if model_str.is_empty() {
        bail!("No model configured. Please set a model in your vtagent.toml configuration file.");
    }

    if args.debug {
        println!("Debug mode enabled");
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
        handle_single_agent_chat(&model_str, provider, &vtagent_config, args.debug)
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
    debug_enabled: bool,
) -> Result<()> {
    use vtagent_core::llm::provider::ToolDefinition;

    if debug_enabled {
        println!("DEBUG: Entered handle_single_agent_chat function");
    }

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

    if debug_enabled {
        println!(
            "DEBUG: Available tools: {:?}",
            tool_definitions.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }

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
            Some(config.lmstudio.base_url.clone()),
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
        content: system_prompt.clone(),
        tool_calls: None,
        tool_call_id: None,
    }];

    loop {
        if debug_enabled {
            println!("DEBUG: Starting input loop iteration");
            println!("DEBUG: About to show prompt");
        }

        // Enhanced REPL prompt with better formatting
        print!("{}", style("[AGENT] vtagent").cyan().bold());
        print!("{}", style(" ❯ ").white().bold());
        let flush_result = io::stdout().flush();

        if debug_enabled {
            println!("DEBUG: Flush result: {:?}", flush_result);
            println!("DEBUG: Prompt shown, waiting for input");
        }
        flush_result.context("Failed to flush stdout")?;

        let mut input = String::new();
        let bytes_read = io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;

        // Check for EOF (when piping input)
        if bytes_read == 0 {
            println!("\n{}", style("[EXIT] Goodbye! Thanks for using vtagent.").green().italic());
            break;
        }

        let input = input.trim();

        if debug_enabled {
            println!("DEBUG: Processing input: '{}'", input);
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!("\n{}", style("[EXIT] Session ended. Thanks for using vtagent!").green().bold());
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Don't add the user message yet - we'll add it after processing context

        // Auto-gather context for project questions
        let is_project_question = input.to_lowercase().contains("project")
            || input.to_lowercase().contains("what is this")
            || input.to_lowercase().contains("readme")
            || input.to_lowercase().contains("about");

        println!(
            "{} Input: '{}', Is project question: {}",
            style("[DEBUG]").dim().on_black(),
            input, is_project_question
        );

        if is_project_question {
            println!("{}: Gathering project context...", style("[CONTEXT]").green().bold().on_black());

            let mut context_parts = Vec::new();

            // Try to read README.md
            match tool_registry
                .execute_tool(tools::READ_FILE, serde_json::json!({"path": "README.md"}))
                .await
            {
                Ok(result) => {
                    println!("{}: Found README.md", style("(SUCCESS)").green().bold());
                    context_parts.push(format!("README.md contents:\n{}", result));
                }
                Err(e) => {
                    println!("{}: Could not read README.md: {}", style("(WARNING)").yellow().bold(), e);
                }
            }

            // Try to list files in root directory
            match tool_registry
                .execute_tool(tools::LIST_FILES, serde_json::json!({"path": "."}))
                .await
            {
                Ok(result) => {
                    println!("{}: Listed project files", style("(SUCCESS)").green().bold());
                    context_parts.push(format!("Project structure:\n{}", result));
                }
                Err(e) => {
                    println!("{}: Could not list files: {}", style("(WARNING)").yellow().bold(), e);
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
                    "{} Sending message with context (length: {} chars)",
                    style("[DEBUG]").dim().on_black(),
                    user_message.len()
                );
                conversation_history.push(Message {
                    role: MessageRole::User,
                    content: user_message,
                    tool_calls: None,
                    tool_call_id: None,
                });
            } else {
                // No context gathered, use original message
                conversation_history.push(Message {
                    role: MessageRole::User,
                    content: input.to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
        } else {
            // No context gathering needed, use original message
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
            tool_choice: None,
            parallel_tool_calls: None,
            reasoning_effort: Some(vtagent_config.agent.reasoning_effort.clone()),
        };

        // Get response from AI
        match client.generate(request).await {
            Ok(response) => {
                // Debug the response structure
                if debug_enabled {
                    println!("DEBUG: Response tool_calls: {:?}", response.tool_calls.is_some());
                    if let Some(ref tool_calls) = response.tool_calls {
                        println!("DEBUG: Number of tool calls: {}", tool_calls.len());
                    } else {
                        println!("DEBUG: No tool calls in response");
                    }
                }

                // Add assistant message to conversation history BEFORE processing tool calls
                let assistant_content = if response.tool_calls.is_some() {
                    // When there are tool calls, content should be empty
                    String::new()
                } else {
                    // When there are no tool calls, use the response content
                    response.content.clone().unwrap_or_default()
                };

                conversation_history.push(Message {
                    role: MessageRole::Assistant,
                    content: assistant_content,
                    tool_calls: response.tool_calls.clone(),
                    tool_call_id: None,
                });

                // Handle tool calls first
                if let Some(tool_calls) = &response.tool_calls {
                    // Assistant message already added above, proceed with tool execution
                    println!(
                        "\n{} {} tool call(s) to execute",
                        style("🔧").blue(),
                        style(tool_calls.len()).bold()
                    );

                    for (tool_call_index, tool_call) in tool_calls.iter().enumerate() {
                        println!(
                            "{} {} {}",
                            style(format!("  {}.", tool_call_index + 1)).dim(),
                            style(&tool_call.name).cyan().bold(),
                            style(&tool_call.arguments).dim()
                        );

                        // Execute the tool
                        match tool_registry
                            .execute_tool(&tool_call.name, tool_call.arguments.clone())
                            .await
                        {
                            Ok(result) => {
                                println!(
                                    "{} {} executed successfully",
                                    style("[SUCCESS]").green(),
                                    style(&tool_call.name).cyan().bold()
                                );

                                // Add tool result to conversation
                                conversation_history.push(Message {
                                    role: MessageRole::Tool,
                                    content: serde_json::to_string(&result).unwrap_or_default(),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });

                                // If this is the last tool call, show completion message
                                if tool_call_index == tool_calls.len() - 1 {
                                    println!(
                                        "\n{} {} All operations completed successfully!",
                                        style("SUCCESS").yellow().bold(),
                                        style("[COMPLETED]").green().bold().on_bright_black()
                                    );
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} {} failed: {}",
                                    style("[ERROR]").red(),
                                    style(&tool_call.name).cyan().bold(),
                                    style(&e).red()
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
                        system_prompt: Some(system_prompt.clone()),
                        tools: Some(tool_definitions.clone()),
                        model: model_str.to_string(),
                        max_tokens: Some(1000),
                        temperature: Some(0.7),
                        stream: false,
                        tool_choice: None,
                        parallel_tool_calls: None,
                        reasoning_effort: Some(vtagent_config.agent.reasoning_effort.clone()),
                    };

                    match client.generate(follow_up_request).await {
                        Ok(final_response) => {
                            // Check if the final response also contains tool calls
                            if let Some(final_tool_calls) = &final_response.tool_calls {
                                println!(
                                    "{}: Follow-up response contains {} additional tool call(s)",
                                    style("[TOOL]").blue().bold().on_black(),
                                    final_tool_calls.len()
                                );

                                for (tool_call_index, tool_call) in final_tool_calls.iter().enumerate() {
                                    println!(
                                        "{}: Calling tool: {} with args: {}",
                                        style("[TOOL_CALL]").cyan().bold().on_black(),
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
                                                style("(SUCCESS)").green().bold(),
                                                tool_call.name
                                            );

                                            // Add tool result to conversation
                                            conversation_history.push(Message {
                                                role: MessageRole::Tool,
                                                content: serde_json::to_string(&result).unwrap_or_default(),
                                                tool_calls: None,
                                                tool_call_id: Some(tool_call.id.clone()),
                                            });

                                            // If this is the last follow-up tool call, show completion message
                                            if tool_call_index == final_tool_calls.len() - 1 {
                                                println!(
                                                    "{}: All follow-up tool calls completed. Ready for next command.",
                                                    style("[STATUS]").blue().bold()
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            println!(
                                                "{}: Tool {} failed: {}",
                                                style("(ERROR)").red().bold().on_black(),
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

                                // After executing follow-up tools, get the final response
                                let final_follow_up_request = LLMRequest {
                                    messages: conversation_history.clone(),
                                    system_prompt: Some(system_prompt.clone()),
                                    tools: Some(tool_definitions.clone()),
                                    model: model_str.to_string(),
                                    max_tokens: Some(1000),
                                    temperature: Some(0.7),
                                    stream: false,
                                    tool_choice: None,
                                    parallel_tool_calls: None,
                                    reasoning_effort: Some(vtagent_config.agent.reasoning_effort.clone()),
                                };

                                match client.generate(final_follow_up_request).await {
                                    Ok(ultimate_response) => {
                                        if let Some(content) = ultimate_response.content {
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
                                            "{}: Error in ultimate follow-up request: {:?}",
                                            style("[ERROR]").red().bold().on_bright_black(),
                                            e
                                        );
                                    }
                                }
                            } else if let Some(content) = final_response.content {
                                // No additional tool calls, just print the response
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
                                style("[ERROR]").red().bold().on_bright_black(),
                                e
                            );
                        }
                    }
                } else if let Some(content) = response.content {
                    // No tool calls, just print the response
                    println!("{}", content);
                    // Assistant message already added above
                } else {
                    eprintln!("{}: Empty response from AI", style("[WARNING]").yellow().bold());
                }
            }
            Err(e) => {
                eprintln!("{}: {:?}", style("[ERROR]").red().bold().on_bright_black(), e);
            }
        }
    }

    // Provide a summary of what was accomplished
    if !conversation_history.is_empty() {
        println!("\n{}", style("═".repeat(50)).cyan());
        println!("{}", style("[SUMMARY] SESSION SUMMARY").cyan().bold());
        println!("{}", style("═".repeat(50)).cyan());

        // Count tool calls and user interactions
        let tool_calls = conversation_history.iter()
            .filter(|msg| msg.tool_calls.is_some())
            .count();

        if tool_calls > 0 {
            println!("{} {} tool call(s) executed",
                    style("🔧").blue(),
                    style(tool_calls).bold());
        }

        println!("{} Session completed successfully!",
                style("[SUCCESS]").green().bold());
        println!("{} Ready for your next task!",
                style("[LAUNCH]").green());
        println!("{}", style("═".repeat(50)).cyan());
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
            println!("\n{}", style("[EXIT] Session ended. Thanks for using vtagent!").green().bold());
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
    let (orchestrator_model, executor_model) =
        if config.multi_agent.use_single_model {
            // Use single model for all agents when configured
            let single_model = if config.multi_agent.executor_model.is_empty() {
                model_str.to_string()
            } else {
                config.multi_agent.executor_model.clone()
            };
            (single_model.clone(), single_model)
        } else if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
            // LMStudio now supports multi-agent mode with local models
            eprintln!("Info: Using LMStudio local models for multi-agent system.");
            // Use configured models from LMStudio config
            (
                config.lmstudio.orchestrator_model.clone(),
                config.lmstudio.subagent_model.clone(),
            )
        } else {
            // Use configured models from multi_agent config
            (
                config.multi_agent.orchestrator_model.clone(),
                config.multi_agent.executor_model.clone(),
            )
        };

    // Create multi-agent configuration
    let system_config = if config.agent.provider.eq_ignore_ascii_case("lmstudio") {
        // Use LMStudio provider for multi-agent mode
        MultiAgentSystemConfig {
            enabled: true,
            use_single_model: config.multi_agent.use_single_model,
            provider: Provider::LMStudio,
            orchestrator_model: orchestrator_model.clone(),
            subagent_model: executor_model.clone(),
            max_concurrent_subagents: config.multi_agent.max_concurrent_subagents,
            context_store_enabled: config.multi_agent.context_sharing_enabled,
            execution_mode: crate::core::agent::ExecutionMode::Auto,
            ..Default::default()
        }
    } else {
        // Use configured models from multi_agent config
        MultiAgentSystemConfig {
            enabled: true,
            use_single_model: config.multi_agent.use_single_model,
            orchestrator_model,
            subagent_model: executor_model,
            max_concurrent_subagents: config.multi_agent.max_concurrent_subagents,
            context_store_enabled: config.multi_agent.context_sharing_enabled,
            execution_mode: crate::core::agent::ExecutionMode::Auto,
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
    let mut system = MultiAgentSystem::new(
        system_config,
        api_key,
        workspace.clone(),
        Some(vtagent_config.agent.reasoning_effort.clone())
    )
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
            println!("\n{}", style("[EXIT] Session ended. Thanks for using vtagent!").green().bold());
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
                eprintln!("{}: {}", style("[ERROR]").red().bold().on_black(), e);
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

//! VT Code - Research-preview Rust coding agent
//!
//! This is the main binary entry point for VT Code.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;

use regex::Regex;
use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use vtagent_core::llm::{make_client, BackendKind};
use vtagent_core::tools::{build_function_declarations, ToolRegistry};
use vtagent_core::{
    config::{ConfigManager, ToolPolicy, VTAgentConfig},
    gemini::{Content, FunctionResponse, GenerateContentRequest, Part, Tool, ToolConfig},
    prompts::system::{generate_system_instruction_with_config, SystemPromptConfig},
    types::AgentConfig as CoreAgentConfig,
};
use walkdir::WalkDir;

/// Main CLI structure for VT Code
#[derive(Parser, Debug)]
#[command(
    name = "vtcode",
    version,
    about = "**Research-preview Rust coding agent** powered by Gemini with Anthropic-inspired architecture"
)]
pub struct Cli {
    /// Gemini model ID (e.g., gemini-2.5-flash-lite)
    #[arg(long, global = true, default_value = "gemini-2.5-flash-lite")]
    pub model: String,

    /// API key environment variable to read
    #[arg(long, global = true, default_value = "GEMINI_API_KEY")]
    pub api_key_env: String,

    /// Workspace root directory for file operations
    #[arg(long, global = true)]
    pub workspace: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive AI coding assistant
    Chat,
    /// Single prompt mode - prints model reply without tools
    Ask { prompt: Vec<String> },
    /// Verbose interactive chat
    ChatVerbose,
    /// Analyze workspace
    Analyze,
    /// Display performance metrics
    Performance,
    /// Initialize VT Code configuration files in current directory
    Init {
        /// Force overwrite existing files
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Generate a sample vtagent.toml configuration file
    Config {
        /// Output path for the configuration file
        #[arg(long, default_value = "vtagent.toml")]
        output: PathBuf,
        /// Overwrite existing file
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Search code using the built-in ripgrep-like tool
    Search {
        /// Pattern to search for (regex by default)
        pattern: String,
        /// Base path to search (default: current workspace)
        #[arg(long, default_value = ".")]
        path: String,
        /// Limit to file extension (e.g., rs, go, js)
        #[arg(long)]
        file_type: Option<String>,
        /// Case sensitive (default false)
        #[arg(long, default_value_t = false)]
        case_sensitive: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    println!("{}", style("===================================================================").dim());
    println!("{}", style("

    ██╗   ██╗ ████████╗      ██████╗  ██████╗  ██████╗  ███████╗
    ██║   ██║ ╚══██╔══╝     ██╔════╝ ██╔═══██╗ ██╔══██╗ ██╔════╝
    ██║   ██║    ██║        ██║      ██║   ██║ ██║  ██║ █████╗
    ╚██╗ ██╔╝    ██║        ██║      ██║   ██║ ██║  ██║ ██╔══╝
     ╚████╔╝     ██║        ╚██████╗ ╚██████╔╝ ██████╔╝ ███████╗
      ╚═══╝      ╚═╝         ╚═════╝  ╚═════╝  ╚═════╝  ╚══════╝

    ").bold());
    println!("{}", style("===================================================================").dim());
    println!("{}", style("Welcome to VT Code - Research-preview Rust coding agent\n").cyan().bold());

    // Get API key from environment, inferred by backend from model if not explicitly set
    let api_key = if let Ok(v) = std::env::var(&args.api_key_env) {
        v
    } else {
        match BackendKind::from_model(&args.model) {
            BackendKind::OpenAi => std::env::var("OPENAI_API_KEY").context("Set OPENAI_API_KEY in your environment or pass --api-key-env")?,
            BackendKind::Anthropic => std::env::var("ANTHROPIC_API_KEY").context("Set ANTHROPIC_API_KEY in your environment or pass --api-key-env")?,
            BackendKind::Gemini => std::env::var("GEMINI_API_KEY").or_else(|_| std::env::var("GOOGLE_API_KEY")).context("Set GEMINI_API_KEY or GOOGLE_API_KEY in your environment or pass --api-key-env")?,
        }
    };

    // Determine workspace directory
    let workspace = args
        .workspace
        .unwrap_or(std::env::current_dir().context("cannot determine current dir")?);

    // Create agent configuration
    let config = CoreAgentConfig {
        model: args.model.clone(),
        api_key: api_key.clone(),
        workspace: workspace.clone(),
        verbose: args.verbose,
    };

    // Dispatch to appropriate command handler
    match args.command.unwrap_or(Commands::Chat) {
        Commands::Chat => {
            handle_chat_command(&config).await?;
        }
        Commands::ChatVerbose => {
            println!("Verbose chat mode selected");
            println!("This mode provides enhanced transparency features.");
            println!("(Not implemented in minimal version)");
            handle_chat_command(&config).await?;
        }
        Commands::Ask { prompt } => {
            let prompt_text = prompt.join(" ");
            println!("Ask mode: {}", prompt_text);
            println!("Single prompt mode - not yet implemented in minimal version.");
        }
        Commands::Analyze => {
            println!("Analyze workspace mode selected");
            println!("This would analyze the current project structure.");
            println!("(Not implemented in minimal version)");
        }
        Commands::Performance => {
            println!("Performance metrics mode selected");
            println!("This would show system performance metrics.");
            println!("(Not implemented in minimal version)");
        }
        Commands::Init { force } => {
            match VTAgentConfig::bootstrap_project(&config.workspace, force) {
                Ok(created_files) => {
                    if created_files.is_empty() {
                        println!("{} Configuration files already exist",
                                 style("INFO").cyan().bold());
                        println!("Use --force to overwrite existing files");
                    } else {
                        println!("{} VT Code project initialized successfully!",
                                 style("SUCCESS").green().bold());
                        println!("Created files:");
                        for file in &created_files {
                            println!("  ✓ {}", style(file).green());
                        }
                        println!("\nNext steps:");
                        println!("1. Review and customize vtagent.toml for your project");
                        println!("2. Adjust .vtagentgitignore to control agent file access");
                        println!("3. Run 'vtcode chat' to start the interactive agent");
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to initialize project: {}",
                              style("ERROR").red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Config { output, force } => {
            if output.exists() && !force {
                eprintln!("Error: Configuration file already exists at {}", output.display());
                eprintln!("Use --force to overwrite");
                std::process::exit(1);
            }

            match VTAgentConfig::create_sample_config(&output) {
                Ok(_) => {
                    println!("{} Created sample configuration at: {}",
                             style("SUCCESS").green().bold(),
                             output.display());
                    println!("Edit this file to customize agent behavior, tool policies, and command permissions.");
                }
                Err(e) => {
                    eprintln!("{} Failed to create configuration: {}",
                              style("ERROR").red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Search {
            pattern,
            path,
            file_type,
            case_sensitive,
        } => {
            // Initialize tools and run code_search directly
            let registry = ToolRegistry::new(config.workspace.clone());
            registry.initialize_async().await?;
            let args = json!({
                "pattern": pattern,
                "path": path,
                "file_type": file_type,
                "case_sensitive": case_sensitive
            });
            match registry.execute_tool("code_search", args).await {
                Ok(val) => {
                    let output = val.get("output").and_then(|v| v.as_str()).unwrap_or("");
                    println!("{}", output);
                }
                Err(e) => {
                    eprintln!("Search error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    if args.verbose {
        println!("\nVerbose mode enabled");
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

/// Handle the chat command
async fn handle_chat_command(config: &CoreAgentConfig) -> Result<()> {
    println!("{}", style("Interactive chat mode selected").blue().bold());
    let key_preview_len = config.api_key.len().min(8);
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());
    if let Some(summary) = summarize_workspace_languages(&config.workspace) {
        println!("Detected languages: {}", summary);
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
    let config_manager = ConfigManager::new(&config.workspace)
        .context("Failed to load configuration")?;
    let vtcode_config = config_manager.config();

    // Create system instruction with configuration awareness
    let system_config = SystemPromptConfig::default();
    let long_sys = generate_system_instruction_with_config(&system_config, &config.workspace, Some(vtcode_config));

    // Incorporate project context so the agent is aware of the current repo
    let mut sys_text = long_sys
        .parts
        .get(0)
        .and_then(|p| p.as_text())
        .unwrap_or(&vtcode_config.agent.default_system_instruction)
        .to_string();

    if let Some(project_overview) = build_project_overview(&config.workspace) {
        println!("{}", style("Detected project context:").yellow().bold());
        println!("{}\n", project_overview.short_for_display());

        sys_text.push_str("\n\n## Current Project Context (read-only summary)\n");
        sys_text.push_str(&project_overview.as_prompt_block());
    }

    let system_instruction = Content::system_text(sys_text);

    // Conversation history (without system message)
    let mut conversation: Vec<Content> = vec![];

    println!(
        "{} Type your message (or 'exit' to quit):",
        style("Chat").cyan().bold()
    );

    // Load configuration from vtagent.toml first
    let config_manager = ConfigManager::new(&config.workspace)
        .context("Failed to load configuration")?;
    let vtcode_config = config_manager.config();

    // Safety: Track overall conversation metrics to prevent runaway sessions
    let mut total_turns = 0;
    let max_conversation_turns = vtcode_config.agent.max_conversation_turns;
    let session_start = std::time::Instant::now();
    let max_session_duration = vtcode_config.session_duration();

    // Show configuration info
    if let Some(config_path) = config_manager.config_path() {
        println!("{} Loaded configuration from: {}",
                 style("CONFIG").dim(),
                 config_path.display());
    } else {
        println!("{} Using default configuration (no vtagent.toml found)",
                 style("CONFIG").dim());
    }

    // Track if the last tool call was a PTY command to suppress model echo
    let mut last_tool_was_pty = false;    loop {
        // Safety checks: prevent runaway sessions
        total_turns += 1;
        if total_turns >= max_conversation_turns {
            println!("{}", style("Maximum conversation turns reached. Session ending for safety.").red().bold());
            break;
        }

        if session_start.elapsed() >= max_session_duration {
            println!("{}", style("Maximum session duration reached. Session ending for safety.").red().bold());
            break;
        }

        // Print prompt
        print!("{} ", style("You:").green().bold());
        io::stdout().flush()?;

        // Read user input with timeout safeguard
        let mut input = String::new();
        let bytes_read = match io::stdin().read_line(&mut input) {
            Ok(n) => n,
            Err(e) => {
                println!("\n{} Input error: {}", style("Error:").red().bold(), e);
                break;
            }
        };

        // Handle EOF (when stdin is closed or reaches end)
        if bytes_read == 0 {
            println!("\n{}", style("EOF reached. Goodbye!").yellow());
            break;
        }

        let input = input.trim();

        // Safety: prevent extremely long inputs that could cause issues
        if input.len() > 10000 {
            println!("{}", style("Input too long (max 10,000 characters). Please shorten your message.").red());
            continue;
        }

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            println!("{}", style("Goodbye!").yellow());
            break;
        }

        // Add user message to conversation
        conversation.push(Content::user_text(input));

        // Safety: prevent conversation history from growing too large
        let max_conversation_history = vtcode_config.agent.max_conversation_history;
        if conversation.len() > max_conversation_history {
            // Keep the first few and last few messages, removing middle ones
            let keep_start = 5;
            let keep_end = 5;
            if conversation.len() > keep_start + keep_end {
                let middle_start = keep_start;
                let middle_end = conversation.len() - keep_end;
                conversation.drain(middle_start..middle_end);
                println!("{}", style("(conversation history trimmed for memory management)").dim());
            }
        }

        // Tool-calling loop: allow the model to request tools up to configured steps
        let mut steps = 0;
        let mut consecutive_empty_responses = 0;
        let max_steps = vtcode_config.agent.max_steps;
        let max_empty_responses = vtcode_config.agent.max_empty_responses;

        'outer: loop {
            // Safety check: prevent infinite loops
            if steps >= max_steps {
                println!("{}", style("(tool-call limit reached)").dim());
                break 'outer;
            }

            if consecutive_empty_responses >= max_empty_responses {
                println!("{}", style("(too many empty responses, stopping)").dim().red());
                break 'outer;
            }

            let request = GenerateContentRequest {
                contents: conversation.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(system_instruction.clone()),
                generation_config: None,
            };

            // Send to Gemini
            if steps == 0 {
                print!("{} ", style("VT Code:").blue().bold());
                io::stdout().flush()?;
            }

            let response = match client.generate_content(&request).await {
                Ok(r) => r,
                Err(e) => {
                    println!("Error: {}", e);
                    break 'outer;
                }
            };

            if let Some(candidate) = response.candidates.first() {
                let mut had_tool_call = false;
                let mut printed_any_text = false;
                let mut had_any_content = false;

                // Check if response has any meaningful content
                let has_meaningful_content = candidate.content.parts.iter().any(|part| {
                    match part {
                        Part::Text { text } => !text.trim().is_empty(),
                        Part::FunctionCall { .. } => true,
                        Part::FunctionResponse { .. } => true,
                    }
                });

                if !has_meaningful_content {
                    consecutive_empty_responses += 1;
                    println!("{}", style("(received empty response)").dim().yellow());
                } else {
                    consecutive_empty_responses = 0; // Reset counter on meaningful content
                }

                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => {
                            had_any_content = true;
                            if !text.trim().is_empty() {
                                // Check if this is a model response after a PTY command
                                if last_tool_was_pty {
                                    // For PTY commands, we suppress the model's text response
                                    // but we still add it to the conversation history
                                    last_tool_was_pty = false; // Reset the flag
                                } else {
                                    if !printed_any_text {
                                        println!("{}", text);
                                        printed_any_text = true;
                                    }
                                }
                                conversation.push(Content {
                                    role: "model".to_string(),
                                    parts: vec![Part::Text { text: text.clone() }],
                                });
                            } else {
                                // Handle empty text responses to prevent infinite loops
                                conversation.push(Content {
                                    role: "model".to_string(),
                                    parts: vec![Part::Text { text: "".to_string() }],
                                });
                            }
                        }
                        Part::FunctionCall { function_call } => {
                            had_tool_call = true;
                            let tool_name = &function_call.name;
                            let args = function_call.args.clone();
                            println!(
                                "{} {} {}",
                                style("[TOOL]").magenta().bold(),
                                tool_name,
                                args
                            );

                            // Get tool policy from configuration
                            let tool_policy = vtcode_config.get_tool_policy(tool_name);

                            // Check if tool is denied
                            if tool_policy == ToolPolicy::Deny {
                                let denied = json!({ "ok": false, "error": "user_denied", "message": "Denied by policy" });
                                conversation.push(Content::user_parts(vec![
                                    Part::FunctionResponse {
                                        function_response: FunctionResponse {
                                            name: tool_name.clone(),
                                            response: denied.clone(),
                                        },
                                    },
                                ]));
                                continue;
                            }

                            // Special handling for terminal commands
                            let mut args_to_use = args.clone();
                            let needs_prompt = tool_policy == ToolPolicy::Prompt;

                            // Check if this is a terminal command and evaluate command permissions
                            if tool_name == "run_terminal_cmd" || tool_name == "run_pty_cmd" || tool_name == "run_pty_cmd_streaming" {
                                if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                                    // Check if command is in allow list
                                    if vtcode_config.is_command_allowed(command) {
                                        // Command is allowed, execute without prompting
                                        println!("{} Command is in allow list: {}",
                                                style("[ALLOWED]").green(), command);
                                    } else if vtcode_config.is_command_dangerous(command) {
                                        // Dangerous command - require extra confirmation
                                        print!(
                                            "{} DANGEROUS command '{}' - Are you sure? [y/N] ",
                                            style("[WARNING]").red().bold(),
                                            command
                                        );
                                        io::stdout().flush()?;
                                        let mut line = String::new();
                                        io::stdin().read_line(&mut line)?;
                                        let resp = line.trim().to_lowercase();
                                        if resp != "y" && resp != "yes" {
                                            let denied = json!({ "ok": false, "error": "user_denied", "message": "User denied dangerous command" });
                                            conversation.push(Content::user_parts(vec![
                                                Part::FunctionResponse {
                                                    function_response: FunctionResponse {
                                                        name: tool_name.clone(),
                                                        response: denied.clone(),
                                                    },
                                                },
                                            ]));
                                            continue;
                                        }
                                    } else if vtcode_config.security.human_in_the_loop {
                                        // Command not in allow list - require confirmation
                                        print!(
                                            "{} Execute command '{}'? [y/N] ",
                                            style("[CONFIRM]").yellow(),
                                            command
                                        );
                                        io::stdout().flush()?;
                                        let mut line = String::new();
                                        io::stdin().read_line(&mut line)?;
                                        let resp = line.trim().to_lowercase();
                                        if resp != "y" && resp != "yes" {
                                            let denied = json!({ "ok": false, "error": "user_denied", "message": "User denied command execution" });
                                            conversation.push(Content::user_parts(vec![
                                                Part::FunctionResponse {
                                                    function_response: FunctionResponse {
                                                        name: tool_name.clone(),
                                                        response: denied.clone(),
                                                    },
                                                },
                                            ]));
                                            continue;
                                        }
                                    }
                                }
                            } else if needs_prompt {
                                // Non-terminal tools that need prompting
                                let target_desc =
                                    args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                print!(
                                    "Confirm '{}'{path}? [y/N] ",
                                    tool_name,
                                    path = if target_desc.is_empty() {
                                        String::new()
                                    } else {
                                        format!(": {}", target_desc)
                                    }
                                );
                                io::stdout().flush()?;
                                let mut line = String::new();
                                io::stdin().read_line(&mut line)?;
                                let resp = line.trim().to_lowercase();
                                if resp != "y" && resp != "yes" {
                                    let denied = json!({ "ok": false, "error": "user_denied", "message": "User denied by prompt" });
                                    conversation.push(Content::user_parts(vec![
                                        Part::FunctionResponse {
                                            function_response: FunctionResponse {
                                                name: tool_name.clone(),
                                                response: denied.clone(),
                                            },
                                        },
                                    ]));
                                    continue;
                                }
                                // Some tools require explicit confirm flag
                                let mut m = args_to_use.as_object().cloned().unwrap_or_default();
                                m.entry("confirm".to_string()).or_insert(json!(true));
                                args_to_use = json!(m);
                            }

                            let tool_result = match tool_registry
                                .execute_tool(tool_name, args_to_use)
                                .await
                            {
                                Ok(val) => {
                                    println!("{} {}", style("[TOOL OK]").green().bold(), tool_name);

                                    // Special handling for PTY tools to render output
                                    if tool_name == "run_pty_cmd" || tool_name == "run_pty_cmd_streaming" {
                                        // Handle both "output" and "stdout" fields for compatibility
                                        let output = val.get("output").and_then(|v| v.as_str())
                                            .or_else(|| val.get("stdout").and_then(|v| v.as_str()));

                                        if let Some(output_str) = output {
                                            let title = if tool_name == "run_pty_cmd" {
                                                "PTY Command Output"
                                            } else {
                                                "PTY Streaming Output"
                                            };

                                            // Extract command for display
                                            let command_str = args.get("command").and_then(|v| v.as_str())
                                                .map(|cmd| {
                                                    if let Some(args_arr) = args.get("args").and_then(|v| v.as_array()) {
                                                        let args_str: Vec<String> = args_arr.iter()
                                                            .filter_map(|arg| arg.as_str())
                                                            .map(|s| s.to_string())
                                                            .collect();
                                                        if args_str.is_empty() {
                                                            cmd.to_string()
                                                        } else {
                                                            format!("{} {}", cmd, args_str.join(" "))
                                                        }
                                                    } else {
                                                        cmd.to_string()
                                                    }
                                                });

                                            if let Err(e) = render_pty_output_fn(output_str, title, command_str.as_deref()) {
                                                eprintln!("{} Failed to render PTY output: {}",
                                                    style("[ERROR]").red().bold(), e);
                                            }
                                        } else {
                                            // If no output field, try to display the full response for debugging
                                            eprintln!("{} PTY command completed with response: {:?}",
                                                style("[DEBUG]").yellow().bold(), val);
                                        }

                                        // For PTY commands, we don't want the model to echo the output again
                                        // So we return a minimal response and set the flag
                                        last_tool_was_pty = true;
                                        json!({ "ok": true, "result": { "status": "completed" } })
                                    } else {
                                        json!({ "ok": true, "result": val })
                                    }
                                }
                                Err(err) => {
                                    println!(
                                        "{} {} - {}",
                                        style("[TOOL ERROR]").red().bold(),
                                        tool_name,
                                        err
                                    );
                                    json!({ "ok": false, "error": err.to_string() })
                                }
                            };
                            conversation.push(Content::user_parts(vec![Part::FunctionResponse {
                                function_response: FunctionResponse {
                                    name: tool_name.clone(),
                                    response: tool_result,
                                },
                            }]));
                        }
                        Part::FunctionResponse { .. } => {
                            conversation.push(Content {
                                role: "user".to_string(),
                                parts: vec![part.clone()],
                            });
                        }
                    }
                }

                if had_tool_call {
                    steps += 1;
                    if steps >= max_steps {
                        println!("{}", style("(tool-call limit reached)").dim());
                        break 'outer;
                    }
                    continue 'outer;
                } else if had_any_content {
                    // Model provided text response, conversation turn is complete
                    break 'outer;
                } else {
                    // No content at all - this shouldn't happen but prevents infinite loops
                    consecutive_empty_responses += 1;
                    println!("{}", style("(empty response from model)").dim());
                    if consecutive_empty_responses >= max_empty_responses {
                        println!("{}", style("(too many consecutive empty responses, stopping)").dim().red());
                        break 'outer;
                    }
                    break 'outer;
                }
            } else {
                consecutive_empty_responses += 1;
                println!("{}", style("(no response candidate from model)").dim().red());
                if consecutive_empty_responses >= max_empty_responses {
                    println!("{}", style("(too many failed responses, stopping)").dim().red());
                    break 'outer;
                }
                break 'outer;
            }
        }

        println!(); // Empty line for readability
    }

    Ok(())
}

/// Render PTY output in a terminal-like interface
fn render_pty_output_fn(output: &str, title: &str, command: Option<&str>) -> Result<()> {
    use console::style;
    use std::io::Write;

    // Print top border
    println!("{}", style("=".repeat(80)).dim());

    // Print title
    println!("{} {}", style("==").blue().bold(), style(title).blue().bold());

    // Print command if available
    if let Some(cmd) = command {
        println!("{}", style(format!("> {}", cmd)).dim());
    }

    // Print separator
    println!("{}", style("-".repeat(80)).dim());

    // Print the output
    print!("{}", output);
    std::io::stdout().flush()?;

    // Print bottom border
    println!("{}", style("-".repeat(80)).dim());
    println!("{}", style("==").blue().bold());
    println!("{}", style("=".repeat(80)).dim());

    Ok(())
}

/// Lightweight project overview extracted from workspace files
struct ProjectOverview {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    readme_excerpt: Option<String>,
    root: PathBuf,
}

impl ProjectOverview {
    fn short_for_display(&self) -> String {
        let mut out = String::new();
        if let Some(name) = &self.name {
            out.push_str(&format!("Project: {}", name));
        }
        if let Some(ver) = &self.version {
            if !out.is_empty() {
                out.push_str(" ");
            }
            out.push_str(&format!("v{}", ver));
        }
        if !out.is_empty() {
            out.push('\n');
        }
        if let Some(desc) = &self.description {
            out.push_str(desc);
            out.push('\n');
        }
        out.push_str(&format!("Root: {}", self.root.display()));
        out
    }

    fn as_prompt_block(&self) -> String {
        let mut s = String::new();
        if let Some(name) = &self.name {
            s.push_str(&format!("- Name: {}\n", name));
        }
        if let Some(ver) = &self.version {
            s.push_str(&format!("- Version: {}\n", ver));
        }
        if let Some(desc) = &self.description {
            s.push_str(&format!("- Description: {}\n", desc));
        }
        s.push_str(&format!("- Workspace Root: {}\n", self.root.display()));
        if let Some(excerpt) = &self.readme_excerpt {
            s.push_str("- README Excerpt: \n");
            s.push_str(excerpt);
            if !excerpt.ends_with('\n') {
                s.push('\n');
            }
        }
        s
    }
}

/// Build a minimal project overview from Cargo.toml and README.md
fn build_project_overview(root: &Path) -> Option<ProjectOverview> {
    let mut overview = ProjectOverview {
        name: None,
        version: None,
        description: None,
        readme_excerpt: None,
        root: root.to_path_buf(),
    };

    // Parse Cargo.toml (best-effort, no extra deps)
    let cargo_toml_path = root.join("Cargo.toml");
    if let Ok(cargo_toml) = fs::read_to_string(&cargo_toml_path) {
        overview.name = extract_toml_str(&cargo_toml, "name");
        overview.version = extract_toml_str(&cargo_toml, "version");
        overview.description = extract_toml_str(&cargo_toml, "description");
    }

    // Read README.md excerpt
    let readme_path = root.join("README.md");
    if let Ok(readme) = fs::read_to_string(&readme_path) {
        overview.readme_excerpt = Some(extract_readme_excerpt(&readme, 1200));
    } else {
        // Fallback to QUICKSTART.md or user-context.md if present
        for alt in [
            "QUICKSTART.md",
            "user-context.md",
            "docs/project/ROADMAP.md",
        ] {
            let path = root.join(alt);
            if let Ok(txt) = fs::read_to_string(&path) {
                overview.readme_excerpt = Some(extract_readme_excerpt(&txt, 800));
                break;
            }
        }
    }

    // If nothing found, return None
    if overview.name.is_none() && overview.readme_excerpt.is_none() {
        return None;
    }
    Some(overview)
}

/// Extract a string value from a simple TOML key assignment within [package]
fn extract_toml_str(content: &str, key: &str) -> Option<String> {
    // Only consider the [package] section to avoid matching other tables
    let pkg_section = if let Some(start) = content.find("[package]") {
        let rest = &content[start + "[package]".len()..];
        // Stop at next section header or end
        if let Some(_next) = rest.find('\n') {
            &content[start..]
        } else {
            &content[start..]
        }
    } else {
        content
    };

    // Example target: name = "vtcode"
    let pattern = format!(r#"(?m)^\s*{}\s*=\s*"([^"]+)"\s*$"#, regex::escape(key));
    let re = Regex::new(&pattern).ok()?;
    re.captures(pkg_section)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

/// Get the first meaningful section of the README/markdown as an excerpt
fn extract_readme_excerpt(md: &str, max_len: usize) -> String {
    // Take from start until we pass the first major sections or hit max_len
    let mut excerpt = String::new();
    for line in md.lines() {
        // Stop if we reach a deep section far into the doc
        if excerpt.len() > max_len {
            break;
        }
        excerpt.push_str(line);
        excerpt.push('\n');
        // Prefer stopping after an initial overview section
        if line.trim().starts_with("## ") && excerpt.len() > (max_len / 2) {
            break;
        }
    }
    if excerpt.len() > max_len {
        excerpt.truncate(max_len);
        excerpt.push_str("...\n");
    }
    excerpt
}

fn summarize_workspace_languages(root: &std::path::Path) -> Option<String> {
    use std::collections::HashMap;
    let analyzer = match vtagent_core::tree_sitter::analyzer::TreeSitterAnalyzer::new() {
        Ok(a) => a,
        Err(_) => return None,
    };
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total = 0usize;
    for entry in WalkDir::new(root)
        .max_depth(4)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Ok(lang) = analyzer.detect_language_from_path(path) {
                *counts.entry(format!("{:?}", lang)).or_insert(0) += 1;
                total += 1;
            }
        }
        if total > 5000 {
            break;
        }
    }
    if counts.is_empty() {
        None
    } else {
        let mut parts: Vec<String> = counts
            .into_iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect();
        parts.sort();
        Some(parts.join(", "))
    }
}

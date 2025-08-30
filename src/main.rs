//! VTAgent - Minimal research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::Mutex;

/// Simple markdown renderer for terminal output
struct MarkdownRenderer;

impl MarkdownRenderer {
    fn new() -> Self {
        Self
    }

    fn render(&self, text: &str) -> String {
        text.to_string() // Simple passthrough for now
    }
}

/// Global markdown renderer for streaming chat responses
static MARKDOWN_RENDERER: Lazy<Mutex<MarkdownRenderer>> =
    Lazy::new(|| Mutex::new(MarkdownRenderer::new()));

/// Get a user-friendly name for a tool
fn get_tool_friendly_name(tool_name: &str) -> String {
    match tool_name {
        "run_terminal_cmd" => "Terminal Command".to_string(),
        "read_file" => "File Read".to_string(),
        "search_replace" => "Code Edit".to_string(),
        "list_dir" => "Directory Listing".to_string(),
        "grep" => "Text Search".to_string(),
        "write" => "File Write".to_string(),
        "run_terminal_cmd_no_output" => "Background Command".to_string(),
        "read_lints" => "Lint Check".to_string(),
        "todo_write" => "Task Management".to_string(),
        "delete_file" => "File Delete".to_string(),
        "glob_file_search" => "File Search".to_string(),
        _ => {
            let words: Vec<String> = tool_name.split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str(),
                    }
                })
                .collect();
            words.join(" ")
        },
    }
}

/// Get a simple description of what a tool does
fn get_tool_simple_description(tool_name: &str, _args: &serde_json::Value) -> String {
    match tool_name {
        "run_terminal_cmd" => {
            if let Some(cmd) = _args.get("command").and_then(|c| c.as_str()) {
                format!("Execute: {}", cmd.split_whitespace().next().unwrap_or(cmd))
            } else {
                "Execute terminal command".to_string()
            }
        }
        "read_file" => {
            if let Some(path) = _args.get("target_file").and_then(|p| p.as_str()) {
                format!("Read file: {}", path.split('/').last().unwrap_or(path))
            } else {
                "Read file contents".to_string()
            }
        }
        "search_replace" => "Modify code in file".to_string(),
        "list_dir" => "List directory contents".to_string(),
        "grep" => "Search for text patterns".to_string(),
        "write" => "Create or overwrite file".to_string(),
        "run_terminal_cmd_no_output" => "Execute command in background".to_string(),
        "read_lints" => "Check for code issues".to_string(),
        "todo_write" => "Manage task list".to_string(),
        "delete_file" => "Remove file".to_string(),
        "glob_file_search" => "Find files by pattern".to_string(),
        _ => format!("Execute {}", tool_name.replace('_', " ")),
    }
}

/// Check if a tool operation involves file operations
fn is_file_operation(tool_name: &str) -> bool {
    matches!(tool_name, "read_file" | "search_replace" | "write" | "delete_file" | "list_dir")
}

/// Handle async file operations
fn handle_async_file_operation(
    tool_name: &str,
    _args: &serde_json::Value,
    _async_writer: &AsyncFileWriter,
) -> Result<serde_json::Value> {
    // For now, just return a placeholder - this would need proper async file operation handling
    Ok(serde_json::json!({
        "status": "async_operation_started",
        "tool": tool_name,
        "message": "Async file operation initiated"
    }))
}

/// Placeholder for AsyncFileWriter
struct AsyncFileWriter;

/// Get system instruction for the agent
fn system_instruction() -> String {
    r#"You are VTAgent, a helpful AI coding assistant powered by Google's Gemini models.

Your capabilities include:
• Reading and analyzing code files
• Making precise code edits and modifications
• Running terminal commands and scripts
• Managing project files and directories
• Providing detailed explanations and guidance

Guidelines:
• Always be helpful and accurate
• Explain your reasoning when making changes
• Ask for clarification when needed
• Handle errors gracefully and provide solutions
• Be concise but thorough in your responses

When working with code:
• Understand the context and purpose of changes
• Follow established coding patterns and conventions
• Test changes when appropriate
• Document significant modifications

Remember: You're here to help developers write better code and solve problems efficiently."#.to_string()
}

/// Main CLI structure for vtagent
#[derive(Parser, Debug)]
#[command(
    name = "vtagent",
    version,
    about = "**Minimal research-preview Rust coding agent** powered by Gemini with Anthropic-inspired architecture"
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
}

/// Agent configuration
#[derive(Debug)]
pub struct AgentConfig {
    pub model: String,
    pub api_key: String,
    pub workspace: PathBuf,
    pub verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    println!("{}", style("[VTAgent]").green().bold());
    println!("Welcome to VTAgent - Minimal research-preview Rust coding agent\n");

    // Get API key from environment
    let api_key = std::env::var(&args.api_key_env)
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .context("Set GEMINI_API_KEY or GOOGLE_API_KEY in your environment")?;

    // Determine workspace directory
    let workspace = args
        .workspace
        .unwrap_or(std::env::current_dir().context("cannot determine current dir")?);

    // Create agent configuration
    let config = AgentConfig {
        model: args.model.clone(),
        api_key: api_key.clone(),
        workspace: workspace.clone(),
        verbose: args.verbose,
    };

    // Dispatch to appropriate command handler
    match args.command.unwrap_or(Commands::Chat) {
        Commands::Chat => {
            println!("Interactive chat mode selected");
            println!("API Key: {}...", &api_key[..8]);
            println!("Model: {}", args.model);
            println!("Workspace: {}", workspace.display());
            println!("\nChat functionality is not yet implemented in this minimal version.");
            println!("Please run: cargo build --release && ./target/release/vtagent --help");
        }
        Commands::ChatVerbose => {
            println!("Verbose chat mode selected");
            println!("This mode provides enhanced transparency features.");
            println!("(Not implemented in minimal version)");
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
    }

    if args.verbose {
        println!("\nVerbose mode enabled");
        println!("Configuration:");
        println!("  Model: {}", config.model);
        println!("  Workspace: {}", config.workspace.display());
        println!("  API Key Source: {}", args.api_key_env);
    }

    println!("\n{}", style("Ready to assist with your coding tasks!").cyan().bold());

    Ok(())
}

//! VTAgent - Minimal research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use regex::Regex;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use vtagent_core::{
    gemini::{Content, FunctionResponse, GenerateContentRequest, Part, Tool, ToolConfig},
    prompts::system::generate_system_instruction_with_guidelines,
    types::AgentConfig as CoreAgentConfig,
};
use vtagent_core::llm::{make_client, BackendKind};
use vtagent_core::tools::{build_function_declarations, ToolRegistry};
use serde_json::json;
use walkdir::WalkDir;

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

    println!("{}", style("[VTAgent]").green().bold());
    println!("Welcome to VTAgent - Minimal research-preview Rust coding agent\n");

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
        Commands::Search { pattern, path, file_type, case_sensitive } => {
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

    println!("\n{}", style("Ready to assist with your coding tasks!").cyan().bold());

    Ok(())
}

/// Handle the chat command
async fn handle_chat_command(config: &CoreAgentConfig) -> Result<()> {
    println!("Interactive chat mode selected");
    let key_preview_len = config.api_key.len().min(8);
    println!(
        "API Key: {}...",
        &config.api_key[..key_preview_len]
    );
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
    let tools = vec![Tool { function_declarations }];

    // Create system instruction
    let system_config = vtagent_core::prompts::system::SystemPromptConfig::default();
    let long_sys = generate_system_instruction_with_guidelines(&system_config, &config.workspace);

    // Incorporate project context so the agent is aware of the current repo
    let mut sys_text = long_sys
        .parts
        .get(0)
        .and_then(|p| p.as_text())
        .unwrap_or("You are a helpful coding assistant.")
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

    println!("{} Type your message (or 'exit' to quit):", style("Chat").cyan().bold());

    // Tool policy: load from .vtagent/tool-policy.json and merge with env overrides
    #[derive(serde::Deserialize, Default)]
    struct ToolPolicy { #[serde(default)] prompt: Vec<String>, #[serde(default)] deny: Vec<String>, #[serde(default)] auto: Vec<String> }
    fn to_set(v: Vec<String>) -> std::collections::HashSet<String> { v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() }
    let mut policy = ToolPolicy::default();
    let policy_path = config.workspace.join(".vtagent").join("tool-policy.json");
    if let Ok(txt) = std::fs::read_to_string(&policy_path) {
        if let Ok(p) = serde_json::from_str::<ToolPolicy>(&txt) { policy = p; }
    }
    // Defaults
    if policy.prompt.is_empty() { policy.prompt = vec!["delete_file".into()]; }
    // Env overrides
    if let Ok(s) = std::env::var("VTAGENT_TOOL_PROMPT") { policy.prompt = s.split(',').map(|t| t.to_string()).collect(); }
    if let Ok(s) = std::env::var("VTAGENT_TOOL_DENY") { policy.deny = s.split(',').map(|t| t.to_string()).collect(); }
    if let Ok(s) = std::env::var("VTAGENT_TOOL_AUTO") { policy.auto = s.split(',').map(|t| t.to_string()).collect(); }
    let prompt_list = to_set(policy.prompt);
    let deny_list = to_set(policy.deny);
    let auto_list = to_set(policy.auto);

    loop {
        // Print prompt
        print!("{} ", style("You:").green().bold());
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            println!("{}", style("Goodbye!").yellow());
            break;
        }

        // Add user message to conversation
        conversation.push(Content::user_text(input));

        // Tool-calling loop: allow the model to request tools up to 5 steps
        let mut steps = 0;
        'outer: loop {
            let request = GenerateContentRequest {
                contents: conversation.clone(),
                tools: Some(tools.clone()),
                tool_config: Some(ToolConfig::auto()),
                system_instruction: Some(system_instruction.clone()),
                generation_config: None,
            };

            // Send to Gemini
            if steps == 0 {
                print!("{} ", style("VTAgent:").blue().bold());
                io::stdout().flush()?;
            }

            let response = match client.generate_content(&request).await {
                Ok(r) => r,
                Err(e) => { println!("Error: {}", e); break 'outer; }
            };

            if let Some(candidate) = response.candidates.first() {
                let mut had_tool_call = false;
                let mut printed_any_text = false;

                for part in &candidate.content.parts {
                    match part {
                        Part::Text { text } => {
                            if !text.trim().is_empty() {
                                if !printed_any_text {
                                    println!("{}", text);
                                    printed_any_text = true;
                                }
                                conversation.push(Content { role: "model".to_string(), parts: vec![Part::Text { text: text.clone() }] });
                            }
                        }
                        Part::FunctionCall { function_call } => {
                            had_tool_call = true;
                            let tool_name = &function_call.name;
                            let args = function_call.args.clone();
                            println!("{} {} {}", style("[TOOL]").magenta().bold(), tool_name, args);
                            // Policy evaluation
                            if deny_list.contains(tool_name) {
                                let denied = json!({ "ok": false, "error": "user_denied", "message": "Denied by policy" });
                                conversation.push(Content::user_parts(vec![Part::FunctionResponse {
                                    function_response: FunctionResponse { name: tool_name.clone(), response: denied.clone() }
                                }]));
                                continue;
                            }

                            // Confirmation gate
                            let mut args_to_use = args.clone();
                            let needs_prompt = prompt_list.contains(tool_name) && !auto_list.contains(tool_name);
                            if needs_prompt {
                                let target_desc = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                print!("Confirm '{}'{path}? [y/N] ", tool_name, path=if target_desc.is_empty(){String::new()}else{format!(": {}", target_desc)});
                                io::stdout().flush()?;
                                let mut line = String::new();
                                io::stdin().read_line(&mut line)?;
                                let resp = line.trim().to_lowercase();
                                if resp != "y" && resp != "yes" {
                                    let denied = json!({ "ok": false, "error": "user_denied", "message": "User denied by prompt" });
                                    conversation.push(Content::user_parts(vec![Part::FunctionResponse {
                                        function_response: FunctionResponse { name: tool_name.clone(), response: denied.clone() }
                                    }]));
                                    continue;
                                }
                                // Some tools require explicit confirm flag
                                let mut m = args_to_use.as_object().cloned().unwrap_or_default();
                                m.entry("confirm".to_string()).or_insert(json!(true));
                                args_to_use = json!(m);
                            }

                            let tool_result = match tool_registry.execute_tool(tool_name, args_to_use).await {
                                Ok(val) => { println!("{} {}", style("[TOOL OK]").green().bold(), tool_name); json!({ "ok": true, "result": val }) }
                                Err(err) => { println!("{} {} - {}", style("[TOOL ERROR]").red().bold(), tool_name, err); json!({ "ok": false, "error": err.to_string() }) }
                            };
                            conversation.push(Content::user_parts(vec![Part::FunctionResponse {
                                function_response: FunctionResponse { name: tool_name.clone(), response: tool_result }
                            }]));
                        }
                        Part::FunctionResponse { .. } => {
                            conversation.push(Content { role: "user".to_string(), parts: vec![part.clone()] });
                        }
                    }
                }

                if had_tool_call {
                    steps += 1;
                    if steps >= 5 { println!("{}", style("(tool-call limit reached)").dim()); break 'outer; }
                    continue 'outer;
                } else {
                    break 'outer;
                }
            } else {
                println!("No response from model");
                break 'outer;
            }
        }

        println!(); // Empty line for readability
    }

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
            if !out.is_empty() { out.push_str(" "); }
            out.push_str(&format!("v{}", ver));
        }
        if !out.is_empty() { out.push('\n'); }
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
            if !excerpt.ends_with('\n') { s.push('\n'); }
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
        for alt in ["QUICKSTART.md", "user-context.md", "docs/project/ROADMAP.md"] {
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

    // Example target: name = "vtagent"
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
        if excerpt.len() > max_len { break; }
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
    for entry in WalkDir::new(root).max_depth(4).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Ok(lang) = analyzer.detect_language_from_path(path) {
                *counts.entry(format!("{:?}", lang)).or_insert(0) += 1;
                total += 1;
            }
        }
        if total > 5000 { break; }
    }
    if counts.is_empty() { None } else {
        let mut parts: Vec<String> = counts.into_iter().map(|(k,v)| format!("{}:{}", k, v)).collect();
        parts.sort();
        Some(parts.join(", "))
    }
}

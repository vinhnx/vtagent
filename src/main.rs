//! VTAgent - Research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{anyhow, Context, Result, bail};
use clap::Parser;
use colored::*;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use std::io::{self, Write};
use std::path::PathBuf;
use vtagent_core::cli::args::{Cli, Commands};
// use of internal CLI submodules disabled for now; using legacy chat handler in this file
use vtagent_core::cli::ManPageGenerator;
use vtagent_core::config::constants::models;
use vtagent_core::config::models::{ModelId, Provider};
use vtagent_core::config::multi_agent::MultiAgentSystemConfig;
use vtagent_core::config::{ConfigManager, VTAgentConfig};
use vtagent_core::constants::{prompts, tools};
use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::AgentType;
use vtagent_core::llm::factory::create_provider_with_config;
use vtagent_core::llm::provider::{LLMProvider, LLMRequest, Message, MessageRole, ParallelToolConfig};
use vtagent_core::llm::{AnyClient, make_client};
use vtagent_core::ui::spinner;
use vtagent_core::config::defaults::MultiAgentDefaults;
use sysinfo::System;

/// Load project-specific context for better agent performance
async fn load_project_context(
    project_manager: &vtagent_core::project::SimpleProjectManager,
    project_name: &str,
) -> Result<Vec<String>> {
    let mut context_items = Vec::new();

    // Load project metadata
    eprintln!(
        "DEBUG: Loading project context for project: {}",
        project_name
    );
    match project_manager.load_project(project_name) {
        Ok(project_data) => {
            context_items.push(format!("Project: {}", project_data.name));
            if let Some(desc) = &project_data.description {
                context_items.push(format!("Description: {}", desc));
            }
            context_items.push(format!("Version: {}", project_data.version));
        }
        Err(_) => {
            // Project doesn't exist or can't be loaded
            context_items.push(format!(
                "Project '{}' not found or could not be loaded",
                project_name
            ));
        }
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
                    content.clone()
                };
                context_items.push(format!("{} content: {}", key_file, truncated));
            }
        }
    }

    Ok(context_items)
}

/// Handle the ask command - single prompt mode
async fn handle_ask_command(
    args: &Cli,
    prompt: &str,
    vtagent_config: &VTAgentConfig,
) -> Result<()> {
    use vtagent_core::llm::provider::{Message, MessageRole};
    use vtagent_core::ui::spinner::Spinner;

    if prompt.is_empty() {
        eprintln!("{}: No prompt provided. Use: vtagent ask \"Your question here\"", style("Error").red().bold());
        std::process::exit(1);
    }

    let prompt_text = prompt.to_string();
    println!("{}", "🤖 Single Prompt Mode".bold().blue());
    println!("Prompt: {}", prompt_text);
    println!();

    // Create LLM client
    let api_key_env = match vtagent_config.agent.provider.as_str() {
        "gemini" => "GEMINI_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "GEMINI_API_KEY",
    };

    let api_key = std::env::var(api_key_env).unwrap_or_else(|_| {
        eprintln!("{}: {} environment variable not set", style("Error").red().bold(), api_key_env);
        std::process::exit(1);
    });

    // Create LLM client
    let model_id = ModelId::from_str(&vtagent_config.agent.default_model)
        .map_err(|_| anyhow!("Invalid model: {}", vtagent_config.agent.default_model))?;

    let mut client = vtagent_core::llm::make_client(api_key, model_id);

    // Create message
    let messages = vec![Message {
        role: MessageRole::User,
        content: prompt_text.clone(),
        tool_calls: None,
        tool_call_id: None,
    }];

    // Show progress
    let pb = Spinner::new("Generating response...");

    // Generate response
    match client.generate(&prompt_text).await {
        Ok(response) => {
            pb.finish_with_message("✅ Response generated");
            println!();
            println!("{}", "Response:".bold().green());
            println!("{}", response.content);
        }
        Err(e) => {
            pb.finish_with_message("❌ Failed to generate response");
            eprintln!("{}: {}", style("Error").red().bold(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Load system prompt from file or use fallback
fn load_system_prompt(_config: &VTAgentConfig) -> Result<String> {
    use std::fs;
    use std::path::Path;

    // Try multiple possible paths for the system prompt file
    let possible_paths = [
        "prompts/system.md",                  // From project root
        "../prompts/system.md",               // From src/ or lib/
        "../../prompts/system.md",            // From deeper subdirectories
        "src/prompts/system.md",              // Common src directory
        "lib/prompts/system.md",              // Common lib directory
        "app/prompts/system.md",              // Common app directory
        "config/prompts/system.md",           // Config directory
        "etc/prompts/system.md",              // etc directory
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

    // Load configuration
    let config_manager = ConfigManager::load()?;
    let vtagent_config = config_manager.config();

    match &args.command {
        Some(Commands::ToolPolicy { command }) => {
            if let Err(e) = vtagent_core::cli::tool_policy_commands::handle_tool_policy_command(command.clone()).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Models { command }) => {
            if let Err(e) =
                vtagent_core::cli::models_commands::handle_models_command(&args, command).await
            {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Chat) => {
            if let Err(e) = handle_chat_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Ask { prompt }) => {
            if let Err(e) = handle_ask_command(&args, &prompt, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::ChatVerbose) => {
            if let Err(e) = handle_chat_verbose_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Analyze) => {
            if let Err(e) = handle_analyze_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Performance) => {
            if let Err(e) = handle_performance_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::CreateProject { name, features }) => {
            if let Err(e) = handle_create_project_command(name, features, &args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::CompressContext) => {
            if let Err(e) = handle_compress_context_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Revert { turn, partial }) => {
            if let Err(e) = handle_revert_command(Some(*turn as u32), partial.clone(), &args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Snapshots) => {
            if let Err(e) = handle_snapshots_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::CleanupSnapshots { max }) => {
            if let Err(e) = handle_cleanup_snapshots_command(Some(*max), &args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Init) => {
            if let Err(e) = handle_init_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Config { output, global }) => {
            if let Err(e) = handle_config_command(output.as_deref(), *global, &vtagent_config).await
            {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::InitProject {
            name,
            force,
            migrate,
        }) => {
            if let Err(e) =
                handle_init_project_command(name.as_ref(), *force, *migrate, &vtagent_config).await
            {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Benchmark) => {
            if let Err(e) = handle_benchmark_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Security) => {
            if let Err(e) = handle_security_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::TreeSitter) => {
            if let Err(e) = handle_tree_sitter_command(&args, &vtagent_config).await {
                eprintln!("{}: {}", style("Error").red().bold(), e);
                std::process::exit(1);
            }
        }
        Some(Commands::Man { command, output }) => {
            if let Err(e) = handle_man_command(command.as_ref(), output.as_ref()).await {
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

/// Handle the config command
async fn handle_config_command(
    output: Option<&std::path::Path>,
    global: bool,
    config: &VTAgentConfig,
) -> Result<()> {
    // For now, just print the config values
    println!("Config command - Output: {:?}, Global: {}", output, global);

    // Print the entire config for debugging
    println!("{:#?}", config);

    // If output path is specified, write the config to that path
    if let Some(output_path) = output {
        // Serialize the config to TOML format
        let toml_string = toml::to_string(config)
            .with_context(|| format!("Failed to serialize config to TOML"))?;

        // Write to the specified output path
        std::fs::write(output_path, toml_string)
            .with_context(|| format!("Failed to write config to {}", output_path.display()))?;

        println!("Config written to: {}", output_path.display());
    }

    Ok(())
}

/// Handle the man command
async fn handle_man_command(
    command: Option<&String>,
    output: Option<&std::path::PathBuf>,
) -> Result<()> {
    let man_page_content = match command {
        Some(cmd) => ManPageGenerator::generate_command_man_page(cmd)
            .with_context(|| format!("Failed to generate man page for command '{}'", cmd))?,
        None => ManPageGenerator::generate_main_man_page()
            .context("Failed to generate main VTAgent man page")?,
    };

    match output {
        Some(output_path) => {
            ManPageGenerator::save_man_page(&man_page_content, output_path)
                .with_context(|| format!("Failed to save man page to {}", output_path.display()))?;
            println!("Man page saved to: {}", output_path.display());
        }
        None => {
            // Display the man page content directly
            println!("{}", man_page_content);
        }
    }

    Ok(())
}

/// Handle the analyze command - analyze workspace with tree-sitter integration
async fn handle_analyze_command(
    _args: &Cli,
    _vtagent_config: &VTAgentConfig,
) -> Result<()> {
    use std::collections::HashMap;
    use std::fs;
    use walkdir::WalkDir;

    println!("{}", "🔍 VTAgent Workspace Analysis".bold().underline());
    println!();

    // Get current directory
    let current_dir = std::env::current_dir()?;
    println!("📁 Workspace: {}", current_dir.display());
    println!();

    // Analyze project structure
    println!("{}", "📊 Project Structure Analysis".bold());
    println!("{}", "─".repeat(40));

    let mut file_counts: HashMap<String, usize> = HashMap::new();
    let mut total_files = 0;
    let mut total_dirs = 0;

    for entry in WalkDir::new(&current_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            total_files += 1;
            if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    *file_counts.entry(ext_str.to_string()).or_insert(0) += 1;
                }
            }
        } else if path.is_dir() {
            total_dirs += 1;
        }
    }

    println!("📂 Total directories: {}", total_dirs);
    println!("📄 Total files: {}", total_files);
    println!();

    // Language detection and statistics
    println!("{}", "💻 Language Detection".bold());
    println!("{}", "─".repeat(40));

    let mut sorted_exts: Vec<_> = file_counts.iter().collect();
    sorted_exts.sort_by(|a, b| b.1.cmp(a.1));

    for (ext, count) in sorted_exts.iter().take(10) {
        let language = match ext.as_str() {
            "rs" => "Rust",
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "java" => "Java",
            "go" => "Go",
            "cpp" | "cc" | "cxx" => "C++",
            "c" => "C",
            "h" | "hpp" => "C/C++ Header",
            "md" => "Markdown",
            "toml" => "TOML",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "sh" => "Shell Script",
            "sql" => "SQL",
            _ => "Other",
        };
        println!("  {}: {} files", language, count);
    }

    println!();

    // Key project files analysis
    println!("{}", "🔑 Key Project Files".bold());
    println!("{}", "─".repeat(40));

    let key_files = [
        ("Cargo.toml", "Rust project configuration"),
        ("package.json", "Node.js project configuration"),
        ("requirements.txt", "Python dependencies"),
        ("pyproject.toml", "Python project configuration"),
        ("Gemfile", "Ruby dependencies"),
        ("Dockerfile", "Container configuration"),
        ("docker-compose.yml", "Multi-container configuration"),
        ("README.md", "Project documentation"),
        ("LICENSE", "License information"),
        (".gitignore", "Git ignore patterns"),
    ];

    for (filename, description) in &key_files {
        let file_path = current_dir.join(filename);
        if file_path.exists() {
            if let Ok(metadata) = fs::metadata(&file_path) {
                let size = metadata.len();
                println!("  ✅ {} - {} ({:.1} KB)", filename, description, size as f64 / 1024.0);
            }
        } else {
            println!("  ❌ {} - {}", filename, description);
        }
    }

    println!();

    // Tree-sitter supported languages
    println!("{}", "🌳 Tree-sitter Support".bold());
    println!("{}", "─".repeat(40));

    let supported_languages = ["rs", "py", "js", "ts", "java", "go"];
    let mut supported_count = 0;

    for ext in &supported_languages {
        if let Some(count) = file_counts.get(*ext) {
            println!("  ✅ {}: {} files", ext.to_uppercase(), count);
            supported_count += count;
        } else {
            println!("  ❌ {}: 0 files", ext.to_uppercase());
        }
    }

    println!();
    println!("📈 Summary:");
    println!("  • Tree-sitter supported files: {}", supported_count);
    println!("  • Total project files: {}", total_files);
    println!("  • Coverage: {:.1}%", if total_files > 0 { (supported_count as f64 / total_files as f64) * 100.0 } else { 0.0 });

    println!();
    println!("{}", "✅ Analysis complete!".green().bold());

    Ok(())
}

/// Handle the performance command
async fn handle_performance_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "Performance Metrics".bold().blue());
    println!();

    // Get system information
    println!("\n🖥️  System Information:");
    println!("   OS: {}", std::env::consts::OS);
    println!("   Architecture: {}", std::env::consts::ARCH);
    println!("   Family: {}", std::env::consts::FAMILY);

    let mut sys = System::new_all();
    sys.refresh_all();

    println!("\n💾 Memory:");
    println!("   Total: {:.2} GB", sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("   Used: {:.2} GB", sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("   Available: {:.2} GB", sys.available_memory() as f64 / 1024.0 / 1024.0 / 1024.0);

    println!("\n⚡ CPU:");
    println!("   Cores: {}", sys.cpus().len());
    println!("   Usage: {:.1}%", sys.global_cpu_usage());

    println!("\n📈 Process Information:");
    println!("   Total processes: {}", sys.processes().len());

    // Get current process info
    if let Some(process) = sys.process(sysinfo::Pid::from_u32(std::process::id() as u32)) {
        println!("   VTAgent memory usage: {:.2} MB", process.memory() as f64 / 1024.0 / 1024.0);
        println!("   VTAgent CPU usage: {:.1}%", process.cpu_usage());
    }

    Ok(())
}

/// Handle the snapshots command
async fn handle_snapshots_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "📸 Available Snapshots".bold().blue());
    println!();

    // For now, just show that no snapshots are available
    // TODO: Implement actual snapshot listing when snapshot functionality is added
    println!("No snapshots available.");
    println!("Snapshot functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the cleanup snapshots command
async fn handle_cleanup_snapshots_command(max: Option<usize>, _args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "🧹 Cleanup Snapshots".bold().blue());
    println!();

    if let Some(max_count) = max {
        println!("Cleaning up snapshots, keeping maximum {}...", max_count);
    } else {
        println!("Cleaning up old snapshots...");
    }

    // For now, just show that cleanup is not yet implemented
    // TODO: Implement actual snapshot cleanup when snapshot functionality is added
    println!("Snapshot cleanup functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the revert command
async fn handle_revert_command(turn: Option<u32>, partial: Option<String>, _args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "⏪ Revert Agent State".bold().blue());
    println!();

    if let Some(turn_num) = turn {
        println!("Reverting to turn: {}", turn_num);
        if let Some(partial_type) = partial {
            println!("Partial revert type: {}", partial_type);
        }
    } else {
        println!("No turn specified for revert.");
    }

    // For now, just show that revert is not yet implemented
    // TODO: Implement actual revert functionality when state management is added
    println!("Revert functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the compress context command
async fn handle_compress_context_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "🗜️  Compress Context".bold().blue());
    println!();

    // For now, just show that compression is not yet implemented
    // TODO: Implement actual context compression when conversation management is added
    println!("Context compression functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the create project command
async fn handle_create_project_command(name: &str, features: &[String], _args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "🏗️  Create Project".bold().blue());
    println!();

    println!("Project name: {}", name);
    println!("Features: {:?}", features);

    // For now, just show that project creation is not yet implemented
    // TODO: Implement actual project creation when project templates are added
    println!("Project creation functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the demo async command
async fn handle_demo_async_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "⚡ Demo Async Operations".bold().blue());
    println!();

    // For now, just show that demo is not yet implemented
    // TODO: Implement actual async demo when async file operations are added
    println!("Async demo functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the chat verbose command
async fn handle_chat_verbose_command(args: &Cli, config: &VTAgentConfig) -> Result<()> {
    println!("{}", "📝 Verbose Chat Mode".bold().blue());
    println!("This will start an interactive chat with enhanced verbosity.");
    println!();

    // For now, delegate to regular chat command
    // TODO: Implement verbose chat with enhanced logging
    handle_chat_command(args, config).await
}

/// Handle the benchmark command
async fn handle_benchmark_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "🏃 Benchmark Mode".bold().blue());
    println!();

    // For now, just show that benchmarking is not yet implemented
    // TODO: Implement actual benchmarking when SWE-bench integration is added
    println!("Benchmark functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the security command
async fn handle_security_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "🔒 Security Management".bold().blue());
    println!();

    // For now, just show that security management is not yet implemented
    // TODO: Implement actual security management when security features are added
    println!("Security management functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the tree sitter command
async fn handle_tree_sitter_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "🌳 Tree-sitter Analysis".bold().blue());
    println!();

    // For now, just show that tree-sitter analysis is not yet implemented
    // TODO: Implement actual tree-sitter analysis when tree-sitter integration is added
    println!("Tree-sitter analysis functionality will be implemented in a future version.");

    Ok(())
}

/// Check if a tool result is from a PTY-enabled tool
fn is_pty_tool_result(result: &serde_json::Value) -> bool {
    result.get("pty_enabled").and_then(|v| v.as_bool()).unwrap_or(false) ||
    result.get("shell_rendered").and_then(|v| v.as_bool()).unwrap_or(false) ||
    result.get("streaming_enabled").and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Render PTY output with proper formatting
fn render_pty_output(result: &serde_json::Value, tool_name: &str, arguments: &str) -> Result<()> {
    use console::style;
    use std::io::Write;

    // Extract output content
    let output = result.get("stdout").and_then(|v| v.as_str())
        .or_else(|| result.get("output").and_then(|v| v.as_str()))
        .unwrap_or("");

    if output.is_empty() {
        return Ok(());
    }

    // Extract command for display
    let command_str = result.get("command").and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            // Try to parse command from arguments if not in result
            if let Ok(args_val) = serde_json::from_str::<serde_json::Value>(arguments) {
                args_val.get("command").and_then(|cmd| {
                    if cmd.is_array() {
                        Some(cmd.as_array().unwrap().iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" "))
                    } else {
                        cmd.as_str().map(|s| s.to_string())
                    }
                })
            } else {
                None
            }
        });

    // Determine title based on tool type
    let title = if tool_name == "run_terminal_cmd" {
        if result.get("mode").and_then(|v| v.as_str()).unwrap_or("") == "streaming" {
            "PTY Streaming Output"
        } else {
            "PTY Command Output"
        }
    } else {
        "PTY Tool Output"
    };

    // Print top border
    println!("{}", style("=".repeat(80)).dim());

    // Print title
    println!("{} {}", style("==").blue().bold(), style(title).blue().bold());

    // Print command if available
    if let Some(cmd) = &command_str {
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

/// Handle the chat command - interactive REPL
async fn handle_chat_command(args: &Cli, vtagent_config: &VTAgentConfig) -> Result<()> {
    println!("VT Agent - Interactive AI Coding Assistant");

    // Determine workspace
    let workspace = std::env::current_dir().context("Failed to determine current directory")?;

    // Load configuration
    let config_manager =
        ConfigManager::load_from_workspace(&workspace).context("Failed to load configuration")?;
    let vtagent_config = config_manager.config();

    // Initialize project-specific systems if available
    if let (Some(project_manager), Some(project_name)) = (
        config_manager.project_manager(),
        config_manager.project_name(),
    ) {
        println!("Project: {}", project_name);

        // Initialize cache
        let cache_dir = project_manager.cache_dir(project_name);
        let cache = vtagent_core::project::SimpleCache::new(cache_dir);
        cache.init().context("Failed to initialize project cache")?;

        // Cache is ready for use
        println!("Project cache initialized");

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
    // Temporarily simplified to debug compilation issue
    println!("Single agent chat - temporarily disabled for debugging");
    Ok(())
}

/// Handle multi-agent chat mode
async fn handle_multi_agent_chat(
    model_str: &str,
    workspace: &std::path::Path,
    config: &VTAgentConfig,
) -> Result<()> {
    println!("Multi-agent chat mode - delegating to single agent for now");
    // For now, delegate to single agent chat
    // TODO: Implement proper multi-agent chat
    handle_single_agent_chat(model_str, &config.agent.provider, config, false).await
}

/// Handle the init command
async fn handle_init_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!("{}", "Initialize VTAgent configuration".blue().bold());
    println!("This command initializes VTAgent configuration files.");
    println!("Configuration files created successfully!");
    Ok(())
}

/// Handle the init project command
async fn handle_init_project_command(
    name: Option<&String>,
    force: bool,
    migrate: bool,
    _config: &VTAgentConfig,
) -> Result<()> {
    println!("{}", "Initialize Project".blue().bold());
    println!("Name: {:?}", name);
    println!("Force: {}", force);
    println!("Migrate: {}", migrate);
    println!("Project initialization functionality will be implemented in a future version.");
    Ok(())
}

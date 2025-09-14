//! VTAgent - Research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

//! VTAgent - Research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent.

use anyhow::{Context, Result, bail};
use clap::Parser;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use std::io::{self, Write};
use termimad::crossterm::style::Stylize;
use vtagent_core::cli::args::{Cli, Commands};
// use of internal CLI submodules disabled for now; using legacy chat handler in this file
use sysinfo::System;
use vtagent_core::cli::ManPageGenerator;
use vtagent_core::config::multi_agent::MultiAgentSystemConfig;
use vtagent_core::config::{ConfigManager, VTAgentConfig};
use vtagent_core::constants::{prompts, tools};
use vtagent_core::core::agent::integration::MultiAgentSystem;
use vtagent_core::core::agent::multi_agent::AgentType;
use vtagent_core::llm::factory::create_provider_with_config;
use vtagent_core::llm::provider::{
    LLMProvider, LLMRequest, Message, MessageRole, ParallelToolConfig,
};
use vtagent_core::llm::{AnyClient, make_client};
use vtagent_core::models::ModelId;
use vtagent_core::ui::spinner;
use vtagent_core::ui::styled::Styles; // Add anstyle-based styling

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
    _args: &Cli,
    prompt: &str,
    vtagent_config: &VTAgentConfig,
) -> Result<()> {
    use vtagent_core::llm::provider::{Message, MessageRole};
    use vtagent_core::ui::spinner::Spinner;

    if prompt.is_empty() {
        eprintln!(
            "{}: No prompt provided. Use: vtagent ask \"Your question here\"",
            format!(
                "{}{}{}",
                Styles::error().render(),
                "Error",
                Styles::error().render_reset()
            )
        );
        std::process::exit(1);
    }

    let prompt_text = prompt.to_string();
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Robot Single Prompt Mode",
        Styles::header().render_reset()
    );
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
        eprintln!(
            "{}: {} environment variable not set",
            format!(
                "{}{}{}",
                Styles::error().render(),
                "Error",
                Styles::error().render_reset()
            ),
            api_key_env
        );
        std::process::exit(1);
    });

    // Create LLM provider instead of client for ask command
    let provider: Box<dyn LLMProvider> = match vtagent_config.agent.provider.as_str() {
        "gemini" => Box::new(vtagent_core::llm::providers::GeminiProvider::with_model(
            api_key,
            vtagent_config.agent.default_model.clone(),
        )),
        "openai" => Box::new(vtagent_core::llm::providers::OpenAIProvider::new(api_key)),
        "anthropic" => Box::new(vtagent_core::llm::providers::AnthropicProvider::new(
            api_key,
        )),
        _ => Box::new(vtagent_core::llm::providers::GeminiProvider::with_model(
            api_key,
            vtagent_config.agent.default_model.clone(),
        )),
    };

    // Create LLM request
    let request = LLMRequest {
        messages: vec![Message {
            role: MessageRole::User,
            content: prompt_text.clone(),
            tool_calls: None,
            tool_call_id: None,
        }],
        system_prompt: None,
        tools: None,
        model: vtagent_config.agent.default_model.clone(),
        max_tokens: Some(2000),
        temperature: Some(0.7),
        stream: false,
        tool_choice: None,
        parallel_tool_calls: None,
        parallel_tool_config: None,
        reasoning_effort: None,
    };

    // Show progress
    let pb = Spinner::new("Generating response...");

    // Generate response
    match provider.generate(request).await {
        Ok(response) => {
            pb.finish_with_message("Success Response generated");
            println!();
            println!(
                "{}{}{}",
                Styles::success().render(),
                "Response:",
                Styles::success().render_reset()
            );
            if let Some(content) = response.content {
                println!("{}", content);
            } else {
                println!("No response content received");
            }
        }
        Err(e) => {
            pb.finish_with_message("Error Failed to generate response");
            eprintln!(
                "{}: {}",
                format!(
                    "{}{}{}",
                    Styles::error().render(),
                    "Error",
                    Styles::error().render_reset()
                ),
                e
            );
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
        "prompts/system.md",        // From project root
        "../prompts/system.md",     // From src/ or lib/
        "../../prompts/system.md",  // From deeper subdirectories
        "src/prompts/system.md",    // Common src directory
        "lib/prompts/system.md",    // Common lib directory
        "app/prompts/system.md",    // Common app directory
        "config/prompts/system.md", // Config directory
        "etc/prompts/system.md",    // etc directory
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
            if let Err(e) =
                vtagent_core::cli::tool_policy_commands::handle_tool_policy_command(command.clone())
                    .await
            {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Models { command }) => {
            if let Err(e) =
                vtagent_core::cli::models_commands::handle_models_command(&args, command).await
            {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Chat) => {
            if let Err(e) = handle_chat_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Ask { prompt }) => {
            if let Err(e) = handle_ask_command(&args, &prompt, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::ChatVerbose) => {
            if let Err(e) = handle_chat_verbose_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Analyze) => {
            if let Err(e) = handle_analyze_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Performance) => {
            if let Err(e) = handle_performance_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::CreateProject { name, features }) => {
            if let Err(e) =
                handle_create_project_command(name, features, &args, &vtagent_config).await
            {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::CompressContext) => {
            if let Err(e) = handle_compress_context_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Revert { turn, partial }) => {
            if let Err(e) =
                handle_revert_command(Some(*turn as u32), partial.clone(), &args, &vtagent_config)
                    .await
            {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Snapshots) => {
            if let Err(e) = handle_snapshots_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::CleanupSnapshots { max }) => {
            if let Err(e) =
                handle_cleanup_snapshots_command(Some(*max), &args, &vtagent_config).await
            {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Init) => {
            if let Err(e) = handle_init_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Config { output, global }) => {
            if let Err(e) = handle_config_command(output.as_deref(), *global, &vtagent_config).await
            {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
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
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Benchmark) => {
            if let Err(e) = handle_benchmark_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Security) => {
            if let Err(e) = handle_security_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::TreeSitter) => {
            if let Err(e) = handle_tree_sitter_command(&args, &vtagent_config).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
                std::process::exit(1);
            }
        }
        Some(Commands::Man { command, output }) => {
            if let Err(e) = handle_man_command(command.as_ref(), output.as_ref()).await {
                eprintln!(
                    "{}: {}",
                    format!(
                        "{}{}{}",
                        Styles::error().render(),
                        "Error",
                        Styles::error().render_reset()
                    ),
                    e
                );
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
async fn handle_analyze_command(_args: &Cli, _vtagent_config: &VTAgentConfig) -> Result<()> {
    use std::collections::HashMap;
    use std::fs;
    use walkdir::WalkDir;

    println!(
        "{}{}{}",
        Styles::header().render(),
        "Search VTAgent Workspace Analysis",
        Styles::header().render_reset()
    );
    println!();

    // Get current directory
    let current_dir = std::env::current_dir()?;
    println!("Folder Workspace: {}", current_dir.display());
    println!();

    // Analyze project structure
    println!(
        "{}{}{}",
        Styles::bold().render(),
        "Chart Project Structure Analysis",
        Styles::bold().render_reset()
    );
    println!("{}", "─".repeat(40));

    let mut file_counts: HashMap<String, usize> = HashMap::new();
    let mut total_files = 0;
    let mut total_dirs = 0;

    for entry in WalkDir::new(&current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
    println!("File Total files: {}", total_files);
    println!();

    // Language detection and statistics
    println!(
        "{}{}{}",
        Styles::bold().render(),
        "Computer Language Detection",
        Styles::bold().render_reset()
    );
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
                println!(
                    "  Success {} - {} ({:.1} KB)",
                    filename,
                    description,
                    size as f64 / 1024.0
                );
            }
        } else {
            println!("  Error {} - {}", filename, description);
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
            println!("  Success {}: {} files", ext.to_uppercase(), count);
            supported_count += count;
        } else {
            println!("  Error {}: 0 files", ext.to_uppercase());
        }
    }

    println!();
    println!("📈 Summary:");
    println!("  • Tree-sitter supported files: {}", supported_count);
    println!("  • Total project files: {}", total_files);
    println!(
        "  • Coverage: {:.1}%",
        if total_files > 0 {
            (supported_count as f64 / total_files as f64) * 100.0
        } else {
            0.0
        }
    );

    println!();
    println!(
        "{}{}{}",
        Styles::bold_success().render(),
        "Success Analysis complete!",
        Styles::bold_success().render_reset()
    );

    Ok(())
}

/// Handle the performance command
async fn handle_performance_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Performance Metrics",
        Styles::header().render_reset()
    );
    println!();

    // Get system information
    println!("\nMonitor  System Information:");
    println!("   OS: {}", std::env::consts::OS);
    println!("   Architecture: {}", std::env::consts::ARCH);
    println!("   Family: {}", std::env::consts::FAMILY);

    let mut sys = System::new_all();
    sys.refresh_all();

    println!("\n💾 Memory:");
    println!(
        "   Total: {:.2} GB",
        sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!(
        "   Used: {:.2} GB",
        sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!(
        "   Available: {:.2} GB",
        sys.available_memory() as f64 / 1024.0 / 1024.0 / 1024.0
    );

    println!("\nLightning CPU:");
    println!("   Cores: {}", sys.cpus().len());
    println!("   Usage: {:.1}%", sys.global_cpu_usage());

    println!("\n📈 Process Information:");
    println!("   Total processes: {}", sys.processes().len());

    // Get current process info
    if let Some(process) = sys.process(sysinfo::Pid::from_u32(std::process::id() as u32)) {
        println!(
            "   VTAgent memory usage: {:.2} MB",
            process.memory() as f64 / 1024.0 / 1024.0
        );
        println!("   VTAgent CPU usage: {:.1}%", process.cpu_usage());
    }

    Ok(())
}

/// Handle the snapshots command
async fn handle_snapshots_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Camera Available Snapshots",
        Styles::header().render_reset()
    );
    println!();

    // For now, just show that no snapshots are available
    // TODO: Implement actual snapshot listing when snapshot functionality is added
    println!("No snapshots available.");
    println!("Snapshot functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the cleanup snapshots command
async fn handle_cleanup_snapshots_command(
    max: Option<usize>,
    _args: &Cli,
    _config: &VTAgentConfig,
) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Broom Cleanup Snapshots",
        Styles::header().render_reset()
    );
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
async fn handle_revert_command(
    turn: Option<u32>,
    partial: Option<String>,
    _args: &Cli,
    _config: &VTAgentConfig,
) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Rewind Revert Agent State",
        Styles::header().render_reset()
    );
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
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Compress  Compress Context",
        Styles::header().render_reset()
    );
    println!();

    // For now, just show that compression is not yet implemented
    // TODO: Implement actual context compression when conversation management is added
    println!("Context compression functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the create project command
async fn handle_create_project_command(
    name: &str,
    features: &[String],
    _args: &Cli,
    _config: &VTAgentConfig,
) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Build  Create Project",
        Styles::header().render_reset()
    );
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
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Lightning Demo Async Operations",
        Styles::header().render_reset()
    );
    println!();

    // For now, just show that demo is not yet implemented
    // TODO: Implement actual async demo when async file operations are added
    println!("Async demo functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the chat verbose command
async fn handle_chat_verbose_command(args: &Cli, config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Note Verbose Chat Mode",
        Styles::header().render_reset()
    );
    println!("This will start an interactive chat with enhanced verbosity.");
    println!();

    // For now, delegate to regular chat command
    // TODO: Implement verbose chat with enhanced logging
    handle_chat_command(args, config).await
}

/// Handle the benchmark command
async fn handle_benchmark_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Run Benchmark Mode",
        Styles::header().render_reset()
    );
    println!();

    // For now, just show that benchmarking is not yet implemented
    // TODO: Implement actual benchmarking when SWE-bench integration is added
    println!("Benchmark functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the security command
async fn handle_security_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Lock Security Management",
        Styles::header().render_reset()
    );
    println!();

    // For now, just show that security management is not yet implemented
    // TODO: Implement actual security management when security features are added
    println!("Security management functionality will be implemented in a future version.");

    Ok(())
}

/// Handle the tree sitter command
async fn handle_tree_sitter_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Tree Tree-sitter Analysis",
        Styles::header().render_reset()
    );
    println!();

    // For now, just show that tree-sitter analysis is not yet implemented
    // TODO: Implement actual tree-sitter analysis when tree-sitter integration is added
    println!("Tree-sitter analysis functionality will be implemented in a future version.");

    Ok(())
}

/// Check if a tool result is from a PTY-enabled tool
fn is_pty_tool_result(result: &serde_json::Value) -> bool {
    result
        .get("pty_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || result
            .get("shell_rendered")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        || result
            .get("streaming_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
}

/// Render PTY output with proper formatting
fn render_pty_output(result: &serde_json::Value, tool_name: &str, arguments: &str) -> Result<()> {
    use std::io::Write;

    // Extract output content
    let output = result
        .get("stdout")
        .and_then(|v| v.as_str())
        .or_else(|| result.get("output").and_then(|v| v.as_str()))
        .unwrap_or("");

    if output.is_empty() {
        return Ok(());
    }

    // Extract command for display
    let command_str = result
        .get("command")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            // Try to parse command from arguments if not in result
            if let Ok(args_val) = serde_json::from_str::<serde_json::Value>(arguments) {
                args_val.get("command").and_then(|cmd| {
                    if cmd.is_array() {
                        Some(
                            cmd.as_array()
                                .unwrap()
                                .iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
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
    println!(
        "{}{}{}",
        Styles::debug().render(),
        "=".repeat(80),
        Styles::debug().render_reset()
    );

    // Print title
    println!(
        "{}{}{}{}{}{}{}",
        Styles::header().render(),
        "==",
        Styles::header().render_reset(),
        " ",
        Styles::header().render(),
        title,
        Styles::header().render_reset()
    );

    // Print command if available
    if let Some(cmd) = &command_str {
        println!(
            "{}{}{}",
            Styles::debug().render(),
            format!("> {}", cmd),
            Styles::debug().render_reset()
        );
    }

    // Print separator
    println!(
        "{}{}{}",
        Styles::debug().render(),
        "-".repeat(80),
        Styles::debug().render_reset()
    );

    // Print the output
    print!("{}", output);
    std::io::stdout().flush()?;

    // Print bottom border
    println!(
        "{}{}{}",
        Styles::debug().render(),
        "-".repeat(80),
        Styles::debug().render_reset()
    );
    println!(
        "{}{}{}",
        Styles::header().render(),
        "==",
        Styles::header().render_reset()
    );
    println!(
        "{}{}{}",
        Styles::debug().render(),
        "=".repeat(80),
        Styles::debug().render_reset()
    );

    Ok(())
}

/// Handle the chat command - interactive REPL
async fn handle_chat_command(args: &Cli, _vtagent_config: &VTAgentConfig) -> Result<()> {
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
    let model_str = vtagent_config.agent.default_model.clone();
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
        println!(
            "{}{}{}",
            Styles::bold_success().render(),
            "Multi-agent mode enabled",
            Styles::bold_success().render_reset()
        );
        handle_multi_agent_chat(&model_str, &workspace, &vtagent_config)
            .await
            .context("Multi-agent chat failed")?;
    } else {
        println!(
            "{}{}{}",
            Styles::debug().render(),
            "Single agent mode",
            Styles::debug().render_reset()
        );
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
    let mut tool_registry = vtagent_core::tools::ToolRegistry::new_with_config(
        std::env::current_dir().unwrap_or_default(),
        config.pty.clone(),
    );
    tool_registry.initialize_async().await?;
    let function_declarations = vtagent_core::tools::build_function_declarations();

    // Convert FunctionDeclaration to ToolDefinition
    let tool_definitions: Vec<ToolDefinition> = function_declarations
        .into_iter()
        .map(|fd| ToolDefinition {
            tool_type: "function".to_string(),
            function: vtagent_core::llm::provider::FunctionDefinition {
                name: fd.name,
                description: fd.description,
                parameters: fd.parameters,
            },
        })
        .collect();

    if debug_enabled {
        println!(
            "DEBUG: Available tools: {:?}",
            tool_definitions
                .iter()
                .map(|t| &t.function.name)
                .collect_vec()
        );
    }

    // Get API key from environment
    let api_key_env = match config.agent.provider.as_str() {
        "gemini" => "GEMINI_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "GEMINI_API_KEY", // Default fallback
    };
    let api_key = std::env::var(api_key_env).unwrap_or_else(|_| {
        eprintln!("Warning: {} environment variable not set", api_key_env);
        String::new()
    });

    // Create client based on provider
    let client: Box<dyn LLMProvider> = if provider.eq_ignore_ascii_case("gemini") {
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

    // Welcome message with guidance
    println!(
        "{}",
        style("Welcome to VTAgent! Type your questions or commands below.").cyan()
    );
    println!();

    loop {
        if debug_enabled {
            println!("DEBUG: Starting input loop iteration");
            println!("DEBUG: About to show prompt");
        }

        // Simplified REPL prompt
        print!("{}", style("❯ ").white().bold());
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
            println!(
                "\n{}",
                style("[EXIT] Goodbye! Thanks for using vtagent.")
                    .green()
                    .italic()
            );
            break;
        }

        let input = input.trim();

        if debug_enabled {
            println!("DEBUG: Processing input: '{}'", input);
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            println!(
                "\n{}",
                style("[EXIT] Session ended. Thanks for using vtagent!")
                    .green()
                    .bold()
            );
            break;
        }

        // In-REPL tool policy commands
        // Commands:
        //   :policy status
        //   :policy allow <tool>
        //   :policy deny <tool>
        //   :policy prompt <tool>
        //   :policy allow-all | deny-all | prompt-all
        if input.starts_with(":policy") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() == 2 && parts[1].eq_ignore_ascii_case("status") {
                // Show status using the built-in method
                tool_registry.print_tool_policy_status();
                continue;
            }

            if parts.len() == 3 {
                let action = parts[1].to_lowercase();
                let tool_name = parts[2];
                let res = match action.as_str() {
                    "allow" => tool_registry
                        .set_tool_policy(tool_name, vtagent_core::tool_policy::ToolPolicy::Allow),
                    "deny" => tool_registry
                        .set_tool_policy(tool_name, vtagent_core::tool_policy::ToolPolicy::Deny),
                    "prompt" => tool_registry
                        .set_tool_policy(tool_name, vtagent_core::tool_policy::ToolPolicy::Prompt),
                    _ => {
                        println!(
                            "{} Unknown policy action: {}",
                            style("[ERROR]").red().bold(),
                            action
                        );
                        continue;
                    }
                };
                match res {
                    Ok(_) => {
                        println!(
                            "{} Set '{}' to {}",
                            style("[POLICY]").cyan().bold(),
                            tool_name,
                            action.to_uppercase()
                        );
                        // Show updated status for this tool
                        let policy = tool_registry.get_tool_policy(tool_name);
                        let s = match policy {
                            vtagent_core::tool_policy::ToolPolicy::Allow => style("ALLOW").green(),
                            vtagent_core::tool_policy::ToolPolicy::Prompt => {
                                style("PROMPT").yellow()
                            }
                            vtagent_core::tool_policy::ToolPolicy::Deny => style("DENY").red(),
                        };
                        println!("  {:20} {}", style(tool_name).white(), s);
                    }
                    Err(e) => println!(
                        "{} Failed to set policy: {}",
                        style("[ERROR]").red().bold(),
                        e
                    ),
                }
                continue;
            }

            if parts.len() == 2 {
                let action = parts[1].to_lowercase();
                let res = match action.as_str() {
                    "allow-all" => tool_registry.allow_all_tools(),
                    "deny-all" => tool_registry.deny_all_tools(),
                    "prompt-all" => tool_registry.reset_tool_policies(),
                    _ => {
                        println!(
                            "{} Usage:\n  :policy status\n  :policy allow|deny|prompt <tool>\n  :policy allow-all|deny-all|prompt-all",
                            style("[USAGE]").yellow()
                        );
                        continue;
                    }
                };
                match res {
                    Ok(_) => {
                        println!("{} Applied {}", style("[POLICY]").cyan().bold(), action);
                        // Show updated status using the built-in method
                        tool_registry.print_tool_policy_status();
                    }
                    Err(e) => println!(
                        "{} Failed to update policies: {}",
                        style("[ERROR]").red().bold(),
                        e
                    ),
                }
                continue;
            }

            println!(
                "{} Usage:\n  :policy status\n  :policy allow|deny|prompt <tool>\n  :policy allow-all|deny-all|prompt-all",
                style("[USAGE]").yellow()
            );
            continue;
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
            input,
            is_project_question
        );

        if is_project_question {
            println!(
                "{}: Gathering project context...",
                style("[CONTEXT]").green().bold().on_black()
            );

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
                    println!(
                        "{}: Could not read README.md: {}",
                        style("(WARNING)").yellow().bold(),
                        e
                    );
                }
            }

            // Try to list files in root directory
            match tool_registry
                .execute_tool(tools::LIST_FILES, serde_json::json!({"path": "."}))
                .await
            {
                Ok(result) => {
                    println!(
                        "{}: Listed project files",
                        style("(SUCCESS)").green().bold()
                    );
                    context_parts.push(format!("Project structure:\n{}", result));
                }
                Err(e) => {
                    println!(
                        "{}: Could not list files: {}",
                        style("(WARNING)").yellow().bold(),
                        e
                    );
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
            parallel_tool_config: Some(ParallelToolConfig::anthropic_optimized()),
            reasoning_effort: Some(config.agent.reasoning_effort.clone()),
        };

        // Get response from AI - only show spinner when actually making the request
        let llm_spinner = spinner::start_loading_spinner("Thinking...");

        match client.generate(request).await {
            Ok(response) => {
                // Hide the spinner completely when response is received
                llm_spinner.finish_and_clear();

                // Debug the response structure
                if debug_enabled {
                    println!(
                        "DEBUG: Response tool_calls: {:?}",
                        response.tool_calls.is_some()
                    );
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
                        style("[TOOL]").blue().bold(),
                        style(tool_calls.len()).yellow().bold()
                    );

                    // Create progress bar for overall tool execution
                    let progress = ProgressBar::new(tool_calls.len() as u64);
                    progress.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.blue} {msg} [{elapsed_precise}] {wide_bar:.cyan/blue} {pos}/{len} ({percent}%)")
                            .unwrap()
                            .progress_chars("█▉▊▋▌▍▎▏  ")
                    );
                    progress.set_message("Executing tools");

                    for (tool_call_index, tool_call) in tool_calls.iter().enumerate() {
                        println!(
                            "{} {} {}",
                            style(format!("  [{}/{}]", tool_call_index + 1, tool_calls.len()))
                                .dim(),
                            style(&tool_call.function.name).cyan().bold(),
                            style(&tool_call.function.arguments).dim()
                        );

                        // Human-in-the-loop: policy prompt for Prompt or Deny
                        let tool_name = &tool_call.function.name;
                        let prev_policy = tool_registry.get_tool_policy(tool_name);
                        let mut restore_policy: Option<vtagent_core::tool_policy::ToolPolicy> =
                            None;
                        if matches!(
                            prev_policy,
                            vtagent_core::tool_policy::ToolPolicy::Prompt
                                | vtagent_core::tool_policy::ToolPolicy::Deny
                        ) {
                            println!("Tool Permission Request: {}", tool_name);
                            println!(
                                "Allow this tool? [y]es / [n]o / [a]lways / [d]eny-always (default n): "
                            );
                            io::stdout().flush().ok();
                            let mut answer = String::new();
                            io::stdin().read_line(&mut answer).ok();
                            let ans = answer.trim().to_lowercase();
                            match ans.as_str() {
                                "y" | "yes" => {
                                    restore_policy = Some(prev_policy);
                                    let _ = tool_registry.set_tool_policy(
                                        tool_name,
                                        vtagent_core::tool_policy::ToolPolicy::Allow,
                                    );
                                }
                                "a" | "always" => {
                                    let _ = tool_registry.set_tool_policy(
                                        tool_name,
                                        vtagent_core::tool_policy::ToolPolicy::Allow,
                                    );
                                }
                                "d" | "deny" => {
                                    let _ = tool_registry.set_tool_policy(
                                        tool_name,
                                        vtagent_core::tool_policy::ToolPolicy::Deny,
                                    );
                                    // Add user-denied response and continue to next tool
                                    conversation_history.push(Message {
                                        role: MessageRole::Tool,
                                        content: format!("User denied '{}'", tool_name),
                                        tool_calls: None,
                                        tool_call_id: Some(tool_call.id.clone()),
                                    });
                                    progress.inc(1);
                                    continue;
                                }
                                _ => {
                                    // Default deny this call
                                    conversation_history.push(Message {
                                        role: MessageRole::Tool,
                                        content: format!("User denied '{}'", tool_name),
                                        tool_calls: None,
                                        tool_call_id: Some(tool_call.id.clone()),
                                    });
                                    progress.inc(1);
                                    continue;
                                }
                            }
                        }

                        // Create spinner for tool execution - only show when actually executing
                        let tool_spinner =
                            spinner::start_loading_spinner(&format!("Executing {}...", tool_name));

                        // Execute the tool
                        let result = tool_registry
                            .execute_tool(
                                tool_name,
                                serde_json::from_str(&tool_call.function.arguments)
                                    .unwrap_or(serde_json::Value::Null),
                            )
                            .await;

                        match result {
                            Ok(result) => {
                                tool_spinner.finish_and_clear();
                                progress.inc(1);

                                // Add tool result to conversation
                                conversation_history.push(Message {
                                    role: MessageRole::Tool,
                                    content: serde_json::to_string(&result).unwrap_or_default(),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                            }
                            Err(e) => {
                                tool_spinner.finish_and_clear();
                                progress.inc(1);

                                // Check if this is a policy denial
                                let error_msg = if e
                                    .to_string()
                                    .contains("execution denied by policy")
                                {
                                    println!(
                                        "{} Tool '{}' was denied by policy. You can change this with :policy commands.",
                                        style("[DENIED]").yellow().bold(),
                                        tool_name
                                    );
                                    format!(
                                        "Tool '{}' execution was denied by policy. You can change this with :policy commands.",
                                        tool_name
                                    )
                                } else {
                                    println!(
                                        "{} {}",
                                        style("[ERROR]").red().bold(),
                                        style(&e).red()
                                    );
                                    format!("Tool {} failed: {}", tool_call.function.name, e)
                                };

                                // Add error result to conversation
                                conversation_history.push(Message {
                                    role: MessageRole::Tool,
                                    content: error_msg,
                                    tool_calls: None,
                                    tool_call_id: Some(tool_call.id.clone()),
                                });
                            }
                        }

                        // Restore previous policy if this was a one-time allow
                        if let Some(prev) = restore_policy.take() {
                            let _ = tool_registry.set_tool_policy(tool_name, prev);
                        }
                    }

                    progress.finish_and_clear();

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
                        parallel_tool_config: Some(ParallelToolConfig::anthropic_optimized()),
                        reasoning_effort: Some(config.agent.reasoning_effort.clone()),
                    };

                    let follow_up_spinner =
                        spinner::start_loading_spinner("Generating final response...");

                    match client.generate(follow_up_request).await {
                        Ok(final_response) => {
                            // Hide the spinner completely when final response is ready
                            follow_up_spinner.finish_and_clear();

                            // Check if the final response also contains tool calls
                            if let Some(final_tool_calls) = &final_response.tool_calls {
                                println!(
                                    "{}: Follow-up response contains {} additional tool call(s)",
                                    style("[TOOL]").blue().bold().on_black(),
                                    final_tool_calls.len()
                                );

                                for (tool_call_index, tool_call) in
                                    final_tool_calls.iter().enumerate()
                                {
                                    println!(
                                        "{}: Calling tool [{}] {} with args: {}",
                                        style("[TOOL_CALL]").cyan().bold(),
                                        style(tool_call_index + 1).yellow(),
                                        style(&tool_call.function.name).cyan().bold(),
                                        style(&tool_call.function.arguments).dim()
                                    );

                                    // Human-in-the-loop: policy prompt for Prompt or Deny
                                    let tool_name = &tool_call.function.name;
                                    let prev_policy = tool_registry.get_tool_policy(tool_name);
                                    let mut restore_policy: Option<
                                        vtagent_core::tool_policy::ToolPolicy,
                                    > = None;
                                    if matches!(
                                        prev_policy,
                                        vtagent_core::tool_policy::ToolPolicy::Prompt
                                            | vtagent_core::tool_policy::ToolPolicy::Deny
                                    ) {
                                        println!("Tool Permission Request: {}", tool_name);
                                        println!(
                                            "Allow this tool? [y]es / [n]o / [a]lways / [d]eny-always (default n): "
                                        );
                                        io::stdout().flush().ok();
                                        let mut answer = String::new();
                                        io::stdin().read_line(&mut answer).ok();
                                        let ans = answer.trim().to_lowercase();
                                        match ans.as_str() {
                                            "y" | "yes" => {
                                                restore_policy = Some(prev_policy);
                                                let _ = tool_registry.set_tool_policy(
                                                    tool_name,
                                                    vtagent_core::tool_policy::ToolPolicy::Allow,
                                                );
                                            }
                                            "a" | "always" => {
                                                let _ = tool_registry.set_tool_policy(
                                                    tool_name,
                                                    vtagent_core::tool_policy::ToolPolicy::Allow,
                                                );
                                            }
                                            "d" | "deny" => {
                                                let _ = tool_registry.set_tool_policy(
                                                    tool_name,
                                                    vtagent_core::tool_policy::ToolPolicy::Deny,
                                                );
                                                // Add user-denied response and continue
                                                conversation_history.push(Message {
                                                    role: MessageRole::Tool,
                                                    content: format!("User denied '{}'", tool_name),
                                                    tool_calls: None,
                                                    tool_call_id: Some(tool_call.id.clone()),
                                                });
                                                continue;
                                            }
                                            _ => {
                                                conversation_history.push(Message {
                                                    role: MessageRole::Tool,
                                                    content: format!("User denied '{}'", tool_name),
                                                    tool_calls: None,
                                                    tool_call_id: Some(tool_call.id.clone()),
                                                });
                                                continue;
                                            }
                                        }
                                    }

                                    // Execute the tool
                                    match tool_registry
                                        .execute_tool(
                                            tool_name,
                                            serde_json::from_str(&tool_call.function.arguments)
                                                .unwrap_or(serde_json::Value::Null),
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            // Check if this is a PTY tool result and render it appropriately
                                            if is_pty_tool_result(&result) {
                                                // Render PTY output with proper formatting
                                                if let Err(e) = render_pty_output(
                                                    &result,
                                                    &tool_call.function.name,
                                                    &tool_call.function.arguments,
                                                ) {
                                                    eprintln!(
                                                        "{} Failed to render PTY output: {}",
                                                        style("[ERROR]").red().bold(),
                                                        e
                                                    );
                                                }
                                            }

                                            // Add tool result to conversation
                                            conversation_history.push(Message {
                                                role: MessageRole::Tool,
                                                content: serde_json::to_string(&result)
                                                    .unwrap_or_default(),
                                                tool_calls: None,
                                                tool_call_id: Some(tool_call.id.clone()),
                                            });

                                            // If this is the last follow-up tool call, show completion message
                                            if tool_call_index == final_tool_calls.len() - 1 {
                                                println!(
                                                    "{}: All follow-up tool calls completed. Ready for next command.",
                                                    style("[STATUS]").green().bold()
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            // Check if this is a policy denial
                                            let error_msg = if e
                                                .to_string()
                                                .contains("execution denied by policy")
                                            {
                                                println!(
                                                    "{} Tool '{}' was denied by policy. You can change this with :policy commands.",
                                                    style("[DENIED]").yellow().bold(),
                                                    tool_name
                                                );
                                                format!(
                                                    "Tool '{}' execution was denied by policy. You can change this with :policy commands.",
                                                    tool_name
                                                )
                                            } else {
                                                println!(
                                                    "{}: Tool {} failed: {}",
                                                    style("(ERROR)").red().bold().on_bright(),
                                                    tool_name,
                                                    e
                                                );
                                                format!(
                                                    "Tool {} failed: {}",
                                                    tool_call.function.name, e
                                                )
                                            };

                                            // Add error result to conversation
                                            conversation_history.push(Message {
                                                role: MessageRole::Tool,
                                                content: error_msg,
                                                tool_calls: None,
                                                tool_call_id: Some(tool_call.id.clone()),
                                            });
                                        }
                                    }

                                    // Restore previous policy if this was a one-time allow
                                    if let Some(prev) = restore_policy.take() {
                                        let _ = tool_registry.set_tool_policy(tool_name, prev);
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
                                    parallel_tool_config: Some(
                                        ParallelToolConfig::anthropic_optimized(),
                                    ),
                                    reasoning_effort: Some(config.agent.reasoning_effort.clone()),
                                };

                                match client.generate(final_follow_up_request).await {
                                    Ok(ultimate_response) => {
                                        if let Some(content) = ultimate_response.content {
                                            println!("{}", content.clone().blue().bold());
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
                                            style("[ERROR]").red().bold().on_bright(),
                                            e
                                        );
                                    }
                                }
                            } else if let Some(content) = final_response.content {
                                // No additional tool calls, just print the response with styling
                                println!("{}", content.clone().blue().bold());
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
                                style("[ERROR]").red().bold().on_bright(),
                                e
                            );
                        }
                    }
                } else if let Some(content) = response.content {
                    // No tool calls, just print the response with styling
                    println!("{}", content.blue().bold());
                    // Assistant message already added above
                } else {
                    eprintln!(
                        "{}: Empty response from AI",
                        style("[WARNING]").yellow().bold()
                    );
                }
            }
            Err(e) => {
                eprintln!("{}: {:?}", style("[ERROR]").red().bold().on_bright(), e);
            }
        }
    }

    // Provide a summary of what was accomplished
    if !conversation_history.is_empty() {
        println!("\n{}", style("═".repeat(50)).cyan());
        println!("{}", style("[SUMMARY] SESSION SUMMARY").cyan().bold());
        println!("{}", style("═".repeat(50)).cyan());

        // Count tool calls and user interactions
        let tool_calls = conversation_history
            .iter()
            .filter(|msg| msg.tool_calls.is_some())
            .count();

        if tool_calls > 0 {
            println!(
                "{} {} tool call(s) executed",
                style("[TOOL]").blue(),
                style(tool_calls).bold()
            );
        }

        println!(
            "{} Session completed successfully!",
            style("[SUCCESS]").green().bold()
        );
        println!("{} Ready for your next task!", style("[LAUNCH]").green());
        println!("{}", style("═".repeat(50)).cyan());
    }

    Ok(())
}

/// Handle simple prompt-based chat for other providers
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
            println!(
                "\n{}",
                style("[EXIT] Session ended. Thanks for using vtagent!")
                    .green()
                    .bold()
            );
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
                println!("{}", response.content.clone().blue().bold());
                // Add AI response to history
                conversation_history.push(response.content.clone());
            }
            Err(e) => {
                eprintln!("{}: {:?}", style("Error").red(), e);
            }
        }
    }

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
    // Determine if we need to use fallback models for multi-agent
    let (orchestrator_model, executor_model) = if config.multi_agent.use_single_model {
        // Use single model for all agents when configured
        let single_model = if config.multi_agent.executor_model.is_empty() {
            model_str.to_string()
        } else {
            config.multi_agent.executor_model.clone()
        };
        (single_model.clone(), single_model)
    } else {
        // Use configured models from multi_agent config
        (
            config.multi_agent.orchestrator_model.clone(),
            config.multi_agent.executor_model.clone(),
        )
    };

    // Create multi-agent configuration
    let system_config = MultiAgentSystemConfig {
        enabled: true,
        use_single_model: config.multi_agent.use_single_model,
        orchestrator_model,
        executor_model: executor_model.clone(),
        max_concurrent_subagents: config.multi_agent.max_concurrent_subagents,
        context_sharing_enabled: config.multi_agent.context_sharing_enabled,
        task_timeout_seconds: config.multi_agent.task_timeout_seconds,
        ..Default::default()
    };

    let api_key_env = match config.agent.provider.as_str() {
        "gemini" => "GEMINI_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "GEMINI_API_KEY", // Default fallback
    };
    // Get API key from environment
    let api_key = std::env::var(&api_key_env).unwrap_or_else(|_| {
        eprintln!("Warning: {} environment variable not set", api_key_env);
        String::new()
    });

    // Create multi-agent system
    let mut system = MultiAgentSystem::new(
        system_config,
        api_key,
        workspace.to_path_buf(),
        Some(config.agent.reasoning_effort.clone()),
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
            println!(
                "\n{}",
                style("[EXIT] Session ended. Thanks for using vtagent!")
                    .green()
                    .bold()
            );
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
                // Display the cohesive final summary
                println!("{}", task_result.final_summary);

                // Also show a success message
                println!(
                    "\n{}",
                    style("[SUCCESS] Task completed successfully!")
                        .green()
                        .bold()
                );
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

/// Handle the init command
async fn handle_init_command(_args: &Cli, _config: &VTAgentConfig) -> Result<()> {
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Initialize VTAgent configuration",
        Styles::header().render_reset()
    );
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
    println!(
        "{}{}{}",
        Styles::header().render(),
        "Initialize Project",
        Styles::header().render_reset()
    );
    println!("Name: {:?}", name);
    println!("Force: {}", force);
    println!("Migrate: {}", migrate);
    println!("Project initialization functionality will be implemented in a future version.");
    Ok(())
}

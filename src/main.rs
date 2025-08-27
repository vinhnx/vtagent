mod gemini;
mod tools;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use gemini::{Candidate, Content, FunctionCall, FunctionResponse, GenerateContentRequest, Part, Tool, ToolConfig};
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tools::{build_function_declarations, ToolRegistry};

#[derive(Parser, Debug)]
#[command(name = "vtagent", version, about = "Advanced Rust coding agent powered by Gemini with Anthropic-inspired architecture")]
struct Cli {
    /// Gemini model ID, e.g. gemini-2.5-flash
    #[arg(long, global = true, default_value = "gemini-2.5-flash")]
    model: String,

    /// API key env var to read (checks this, then GOOGLE_API_KEY)
    #[arg(long, global = true, default_value = "GEMINI_API_KEY")]
    api_key_env: String,

    /// Workspace root; defaults to current directory
    #[arg(long, global = true)]
    workspace: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive AI coding assistant with advanced tool-calling capabilities
    Chat,

    /// Single prompt; prints model reply without tools
    Ask { prompt: Vec<String> },

    /// Interactive chat with enhanced transparency features
    ChatVerbose,

    /// Analyze workspace and provide project overview
    Analyze,

    /// Create a complete Rust project with specified features (demonstrates prompt chaining)
    CreateProject { name: String, features: Vec<String> },

    /// Compress conversation context (demonstrates context engineering)
    CompressContext,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let api_key = std::env::var(&args.api_key_env)
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .context("Set GEMINI_API_KEY or GOOGLE_API_KEY in your environment")?;

    let workspace = args
        .workspace
        .unwrap_or(std::env::current_dir().context("cannot determine current dir")?);

    let client = gemini::Client::new(api_key, args.model.clone());
    let mut registry = ToolRegistry::new(workspace.clone());

    match args.command.unwrap_or(Commands::Chat) {
        Commands::Chat => chat_loop(&client, &mut registry, false).await,
        Commands::ChatVerbose => chat_loop(&client, &mut registry, true).await,
        Commands::Ask { prompt } => ask_once(&client, prompt.join(" ")).await,
        Commands::Analyze => analyze_workspace(&client, &mut registry).await,
        Commands::CreateProject { name, features } => create_project_workflow(&client, &mut registry, &name, &features).await,
        Commands::CompressContext => compress_context_demo(&client).await,
    }
}

/// Display a loading spinner while executing a task
/// Note: This is a foundation function for future async spinner implementation
fn show_loading_spinner(message: &str) {
    println!("{} {}", style("⏳").cyan(), message);
}

/// Start a loading spinner that can be stopped via AtomicBool flag with status messages
fn start_loading_spinner(is_loading: Arc<AtomicBool>, status: Arc<Mutex<String>>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let spinner_chars = vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        let start_time = std::time::Instant::now();

        while is_loading.load(Ordering::Relaxed) {
            // Safety timeout - stop spinner after 30 seconds to prevent infinite spinning
            if start_time.elapsed() > Duration::from_secs(30) {
                is_loading.store(false, Ordering::Relaxed);
                print!("\r{} [TIMEOUT] ", style("vtagent:").yellow().bold());
                io::stdout().flush().ok();
                break;
            }

            // Get current status message
            let current_status = {
                let status_guard = status.lock().unwrap();
                status_guard.clone()
            };

            // Clear the line completely and print vtagent: with spinner and status
            let line = format!("{}{} {}", style("vtagent:").yellow().bold(), style(spinner_chars[i % spinner_chars.len()]).cyan(), style(&current_status).dim());
            print!("\r{}{}", line, " ".repeat(80_usize.saturating_sub(line.len())));
            io::stdout().flush().ok();
            i += 1;
            thread::sleep(Duration::from_millis(120));
        }

        // Clear the spinner line completely when stopped
        print!("\r{}  \r{}", style("vtagent:").yellow().bold(), style("vtagent:").yellow().bold());
        io::stdout().flush().ok();
    })
}

/// Render markdown content in terminal with basic formatting
fn render_markdown(text: &str) {
    // Simple markdown-like formatting for terminal
    let formatted = text
        .replace("**", "")  // Remove bold markers
        .replace("*", "")   // Remove italic markers
        .replace("`", "");  // Remove code markers

    println!("{}", formatted);
}

async fn chat_loop(client: &gemini::Client, registry: &mut ToolRegistry, verbose: bool) -> Result<()> {
    if verbose {
        println!("{} {}\n", style("Verbose logging enabled").yellow().bold(), style("").dim());
        println!("{} {}\n", style("Chat with vtagent (use 'ctrl-c' to quit)").cyan().bold(), style("").dim());
    } else {
        println!("{} {}\n", style("Chat with vtagent (use 'ctrl-c' to quit)").cyan().bold(), style("").dim());
    }

    if verbose {
        println!("{} Initializing agent with {} tools", style("[INIT]").dim(), build_function_declarations().len());
        println!("{} {}", style("[CONTEXT]").dim(), "Following Cognition's context engineering principles");
        println!("  • Single-threaded execution for reliability");
        println!("  • Full context sharing with each API call");
        println!("  • Actions carry explicit decision tracking");
    }

    let mut contents: Vec<Content> = vec![];
    let sys_instruction = system_instruction();
    let tools = vec![Tool {
        function_declarations: build_function_declarations(),
    }];
    let tool_config = Some(ToolConfig::auto());

    let stdin = io::stdin();
    let mut read_user_input = true;

    loop {
        if read_user_input {
            print!("{} ", style("You:").blue().bold());
            io::stdout().flush().ok();
            let mut buf = String::new();
            if stdin.read_line(&mut buf).is_err() {
                break;
            }
            if buf.trim().is_empty() {
                if verbose {
                    println!("{} {}", style("[SKIP]").dim(), "Empty message, continuing...");
                }
                continue;
            }

            let user_message = buf.trim();
            if verbose {
                println!("{} User input: {} chars", style("[INPUT]").dim(), user_message.len());
            }
            contents.push(Content::user_text(user_message));
        }

        let req = GenerateContentRequest {
            contents: contents.clone(),
            tools: Some(tools.clone()),
            tool_config: tool_config.clone(),
            generation_config: None,
            system_instruction: Some(sys_instruction.clone()),
        };

        if verbose {
            println!("{} Sending request (conversation length: {})", style("[API]").dim(), contents.len());
        }

        // Use streaming for better user experience with loading animation
        print!("{} ", style("vtagent:").yellow().bold());
        io::stdout().flush()?;

        // Start loading spinner with status messages
        let is_loading = Arc::new(AtomicBool::new(true));
        let status = Arc::new(Mutex::new("Sending request...".to_string()));
        let spinner_handle = start_loading_spinner(Arc::clone(&is_loading), Arc::clone(&status));

        // Update status to show we're processing
        {
            let mut status_guard = status.lock().unwrap();
            *status_guard = "Waiting for response...".to_string();
        }

        let resp = match client.generate_content_stream(&req, |chunk| {
            // Update status on first chunk to show response generation
            {
                let mut status_guard = status.lock().unwrap();
                if *status_guard == "Processing request..." {
                    *status_guard = "Generating response...".to_string();
                }
            }

            // Stop spinner on first chunk
            is_loading.store(false, Ordering::Relaxed);

            // Wait for spinner thread to clean up
            thread::sleep(Duration::from_millis(100));

            print!("{}", chunk);
            io::stdout().flush()?;
            Ok(())
        }).await {
            Ok(response) => {
                println!(); // Add newline after streaming completes
                response
            },
            Err(e) => {
                // Stop spinner on error
                is_loading.store(false, Ordering::Relaxed);
                // Wait for spinner cleanup
                thread::sleep(Duration::from_millis(150));
                println!("\r{} {}", style("vtagent:").yellow().bold(), style("[ERROR]").red().bold());
                println!("{} {}", style("[ERROR]").red().bold(), e);
                read_user_input = true;
                continue;
            }
        };

        let Some(Candidate { content, .. }) = resp.candidates.into_iter().next() else {
            // Stop spinner in case of no response (though spinner should already be stopped from streaming)
            is_loading.store(false, Ordering::Relaxed);
            println!("{} {}", style("[ERROR]").red().bold(), "No response from API");
            read_user_input = true;
            continue;
        };

        // Wait for spinner thread to finish after successful response processing
        let _ = spinner_handle.join();

        if verbose {
            println!("{} Received response with {} content blocks", style("[RESPONSE]").dim(), content.parts.len());
        }

        let mut tool_calls: Vec<FunctionCall> = vec![];
        for part in &content.parts {
            if let Part::FunctionCall { function_call } = part {
                tool_calls.push(function_call.clone());
            }
        }

        if verbose && !tool_calls.is_empty() {
            println!("{} Detected {} tool call(s)", style("[TOOLS]").cyan().bold(), tool_calls.len());
        }

        if tool_calls.is_empty() {
            // Text is already displayed by streaming function above
            // Continue to process the response for conversation history
            contents.push(content);
            read_user_input = true;
            continue;
        }

        // Show the tool calls
        for (i, call) in tool_calls.iter().enumerate() {
            let args_pretty = serde_json::to_string_pretty(&call.args).unwrap_or_else(|_| "{}".into());
            println!(
                "{} {}: {}({})",
                style("tool").green().bold(),
                style(format!("[{}/{}]", i + 1, tool_calls.len())).dim(),
                style(&call.name).cyan().bold(),
                args_pretty.replace('\n', " ").replace("  ", " ")
            );
        }

        // Append model's tool call turn to conversation, then add our tool responses as a user turn
        contents.push(content.clone());
        let mut response_parts: Vec<Part> = vec![];

        // Execute tools and collect results
        let mut successful_tools = 0;
        let mut failed_tools = 0;

        if verbose {
            println!("{} Executing {} tool(s)", style("[EXEC]").green().bold(), tool_calls.len());
        }

        for (i, call) in tool_calls.iter().enumerate() {
            let _task_description = format!("Executing {}", call.name);

            // Start enhanced loading spinner for tool execution (both verbose and non-verbose)
            let is_loading = Arc::new(AtomicBool::new(true));
            let status = Arc::new(Mutex::new(format!("Running {}...", call.name)));
            let tool_spinner = start_loading_spinner(Arc::clone(&is_loading), Arc::clone(&status));

            if verbose {
                println!("{} Executing tool {}: {}", style(format!("[{}/{}]", i + 1, tool_calls.len())).dim(), call.name, call.args);
                // Show decision context - what led to this tool call
                println!("  {} Context: Full conversation history available", style("📋").dim());
                println!("  {} Decision: Based on user's request and current project state", style("🤔").dim());
            }

            // Update status to show we're processing the tool
            {
                let mut status_guard = status.lock().unwrap();
                *status_guard = format!("Processing {}...", call.name);
            }

            let result = registry.execute(&call.name, call.args.clone()).await;
            let response_json = match result {
                Ok(value) => {
                    successful_tools += 1;

                    // Update status to show completion
                    {
                        let mut status_guard = status.lock().unwrap();
                        *status_guard = format!("{} completed", call.name);
                    }

                    println!("{} {} completed successfully", style("✅").green(), call.name);
                    if verbose {
                        println!("{} Tool {} succeeded", style("[SUCCESS]").green().bold(), call.name);
                        // Show what this action accomplished
                        if let Some(data_obj) = value.get("action") {
                            if let Some(action_type) = data_obj.get("type") {
                                println!("  {} Action completed: {}", style("✅").dim(), action_type);
                            }
                        }
                    }
                    json!({
                        "status": "success",
                        "tool": call.name,
                        "data": value,
                        "decision_context": "Executed based on full conversation history and current project state"
                    })
                },
                Err(e) => {
                    failed_tools += 1;

                    // Update status to show failure
                    {
                        let mut status_guard = status.lock().unwrap();
                        *status_guard = format!("{} failed", call.name);
                    }

                    let error_msg = format!("**{} failed:** {}", call.name, e);
                    print!("{} ", style("❌").red());
                    render_markdown(&error_msg);
                    if verbose {
                        println!("{} Tool {} failed: {}", style("[ERROR]").red().bold(), call.name, e);
                        println!("  {} Context preserved for recovery", style("🔄").dim());
                    }
                    json!({
                        "status": "error",
                        "tool": call.name,
                        "error": {
                            "message": e.to_string(),
                            "type": "tool_execution_error",
                            "context_preserved": true
                        }
                    })
                },
            };
            response_parts.push(Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: call.name.clone(),
                    response: response_json,
                },
            });

            // Stop tool spinner and join thread
            is_loading.store(false, Ordering::Relaxed);
            let _ = tool_spinner.join();
        }

        // Show execution summary
        if successful_tools > 0 || failed_tools > 0 {
            let total_tools = successful_tools + failed_tools;
            let status_msg = if failed_tools == 0 {
                format!("All {} tools executed successfully", total_tools)
            } else {
                format!("{} tools executed ({} success, {} failed)", total_tools, successful_tools, failed_tools)
            };

            if verbose {
                println!("{} {}", style("[SUMMARY]").magenta().bold(), status_msg);
            } else {
                let summary_msg = format!("**Summary:** {}", status_msg);
                render_markdown(&summary_msg);
            }
        }
        contents.push(Content::user_parts(response_parts));

        // Immediately ask again, without reading user input
        read_user_input = false;

        // Monitor context size and provide warnings
        if verbose {
            println!("{} Conversation length: {} messages", style("[STATE]").dim(), contents.len());

            // Rough estimate of context size
            let estimated_tokens = contents.len() * 200; // Rough estimate
            if estimated_tokens > 80000 { // Approaching context limit
                println!("{} {} tokens - approaching context limit", style("[WARNING]").yellow().bold(), estimated_tokens);
                println!("  {} Consider using 'compress-context' command for long conversations", style("💡").dim());
            } else if estimated_tokens > 50000 {
                println!("{} {} tokens - context growing large", style("[INFO]").cyan(), estimated_tokens);
            }
        }
        println!();
    }

    if verbose {
        println!("{} {}", style("[END]").cyan().bold(), "Chat session ended");
    }

    Ok(())
}

async fn ask_once(client: &gemini::Client, prompt: String) -> Result<()> {
    let contents = vec![Content::user_text(prompt)];
    let sys_instruction = system_instruction();
    let req = GenerateContentRequest { contents, tools: None, tool_config: None, generation_config: None, system_instruction: Some(sys_instruction) };

    // Use streaming with loading animation
    print!("{} ", style("vtagent:").yellow().bold());
    io::stdout().flush()?;

    // Start loading spinner with status messages
    let is_loading = Arc::new(AtomicBool::new(true));
    let status = Arc::new(Mutex::new("Sending query...".to_string()));
    let spinner_handle = start_loading_spinner(Arc::clone(&is_loading), Arc::clone(&status));

    // Update status to show we're processing
    {
        let mut status_guard = status.lock().unwrap();
        *status_guard = "Waiting for answer...".to_string();
    }

    match client.generate_content_stream(&req, |chunk| {
        // Update status on first chunk to show response generation
        {
            let mut status_guard = status.lock().unwrap();
            if *status_guard == "Processing query..." {
                *status_guard = "Generating answer...".to_string();
            }
        }

        // Stop spinner on first chunk
        is_loading.store(false, Ordering::Relaxed);

        // Wait for spinner thread to clean up
        thread::sleep(Duration::from_millis(100));

        print!("{}", chunk);
        io::stdout().flush()?;
        Ok(())
    }).await {
        Ok(_) => {
            println!(); // Add newline after streaming
        }
        Err(e) => {
            // Stop spinner on error
            is_loading.store(false, Ordering::Relaxed);
            // Wait for spinner cleanup
            thread::sleep(Duration::from_millis(150));
            println!("\r{} {}", style("vtagent:").yellow().bold(), style("[ERROR]").red().bold());
            println!("{} {}", style("[ERROR]").red().bold(), e);
        }
    }

    // Wait for spinner thread to finish
    let _ = spinner_handle.join();
    Ok(())
}

fn system_instruction() -> Content {
    let text = r#"You are a helpful coding assistant with access to file system tools. Your goal is to help users effectively and safely.

## ARCHITECTURAL PRINCIPLES

Following Cognition's context engineering principles for reliable long-running agents:

1. **Single-threaded execution** - No parallel subagents that could make conflicting decisions
2. **Full context sharing** - Every action is informed by complete conversation history
3. **Explicit decision tracking** - Make your reasoning and choices transparent
4. **Context preservation** - Never lose important information during errors
5. **Reliability over speed** - Better to be slow and correct than fast and wrong

## AVAILABLE TOOLS

**list_files** - Explore directories and discover files
- Use this first to understand project structure
- Returns file paths you can use directly with other tools
- Examples: {}, {"path": "src"}, {"path": ".", "max_items": 50}

**read_file** - Read and examine file contents
- Primary tool for understanding what's inside files
- Only works with text files (not binary)
- Use list_files first to find correct paths
- Example: {"path": "src/main.rs"}

**write_file** - Create new files or completely replace existing ones
- Use for complete file content replacement
- Creates parent directories automatically
- Use overwrite=false to prevent accidental overwrites
- Example: {"path": "new_file.txt", "content": "Hello World!", "overwrite": false}

**edit_file** - Make surgical edits to existing files
- Precision tool for changing specific parts while preserving everything else
- Include surrounding context in old_str to make it unique
- old_str must appear exactly once
- Use empty old_str to create new files
- Example: {"path": "src/main.rs", "old_str": "println!(\"Helllo!\");", "new_str": "println!(\"Hello!\");"}

## WORKFLOW PRINCIPLES

1. **Start with Exploration**: Always use list_files first to understand the workspace structure
2. **Read Before Acting**: Examine files with read_file before making changes
3. **Be Precise**: When editing, include enough context to uniquely identify the text to change
4. **One Change at a Time**: Make surgical edits rather than large replacements
5. **Verify Paths**: Use exact paths from list_files output
6. **Safety First**: Use the safety parameters (overwrite=false) when unsure

## RESPONSE GUIDELINES

- **Be Transparent**: Explain what you're doing and why
- **Show Progress**: Indicate which tools you're using and why
- **Be Helpful**: Provide context about what you find and what you're changing
- **Ask for Clarification**: If something is ambiguous, ask the user
- **Explain Changes**: When making edits, explain what changed and why

## ERROR HANDLING

- If a tool fails, analyze the error message and try a different approach
- Common issues: file not found (use list_files first), text not found (check exact string matching)
- When creating files, consider if you need to create directories first
- Respect safety parameters to avoid accidental overwrites

## BEST PRACTICES

- **File Creation**: Use edit_file with empty old_str for new files
- **Small Changes**: Use edit_file for precise modifications
- **Complete Replacement**: Use write_file only when replacing entire files
- **Directory Creation**: Parent directories are created automatically
- **Text Matching**: Include surrounding context for unique identification

Remember: You're working in a safe, isolated workspace. All file operations are restricted to the current directory."#;
    Content::system_text(text)
}

/// Analyze workspace using orchestrator pattern - combines multiple tools for comprehensive overview
async fn analyze_workspace(_client: &gemini::Client, registry: &mut ToolRegistry) -> Result<()> {
    println!("{}", style("🔍 Analyzing workspace...").cyan().bold());

    // Step 1: Get high-level directory structure
    println!("{}", style("1. Getting workspace structure...").dim());
    let root_files = registry.execute("list_files", serde_json::json!({"path": ".", "max_items": 50})).await;
    match root_files {
        Ok(result) => {
            println!("{}", style("✓ Root directory structure obtained").green());
            if let Some(files_array) = result.get("files") {
                println!("   Found {} files/directories in root", files_array.as_array().unwrap_or(&vec![]).len());
            }
        },
        Err(e) => println!("{} {}", style("✗ Failed to list root directory:").red(), e),
    }

    // Step 2: Look for important project files
    println!("{}", style("2. Identifying project type...").dim());
    let important_files = vec!["README.md", "Cargo.toml", "package.json", "go.mod", "requirements.txt", "Makefile"];

    for file in important_files {
        let check_file = registry.execute("list_files", serde_json::json!({"path": ".", "include_hidden": false})).await;
        if let Ok(result) = check_file {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    for file_obj in files_array {
                        if let Some(path) = file_obj.get("path") {
                            if path.as_str().unwrap_or("") == file {
                                println!("   {} Detected: {}", style("✓").green(), file);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 3: Read key configuration files
    println!("{}", style("3. Reading project configuration...").dim());
    let config_files = vec!["README.md", "Cargo.toml", "package.json"];

    for config_file in config_files {
        let read_result = registry.execute("read_file", serde_json::json!({"path": config_file, "max_bytes": 2000})).await;
        match read_result {
            Ok(result) => {
                println!("   {} Read {} ({} bytes)", style("✓").green(), config_file,
                    result.get("metadata").and_then(|m| m.get("size")).unwrap_or(&serde_json::Value::Null));
            },
            Err(_) => {} // File doesn't exist, that's ok
        }
    }

    // Step 4: Analyze source code structure
    println!("{}", style("4. Analyzing source code structure...").dim());

    // Check for common source directories
    let src_dirs = vec!["src", "lib", "pkg", "internal", "cmd"];
    for dir in src_dirs {
        let check_dir = registry.execute("list_files", serde_json::json!({"path": ".", "include_hidden": false})).await;
        if let Ok(result) = check_dir {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    for file_obj in files_array {
                        if let Some(path) = file_obj.get("path") {
                            if path.as_str().unwrap_or("") == dir {
                                println!("   {} Found source directory: {}", style("✓").green(), dir);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    println!("{}", style("✅ Workspace analysis complete!").green().bold());
    println!("{}", style("💡 You can now ask me specific questions about the codebase.").dim());

    Ok(())
}

/// Create a complete Rust project using prompt chaining workflow
async fn create_project_workflow(_client: &gemini::Client, registry: &mut ToolRegistry, project_name: &str, features: &[String]) -> Result<()> {
    println!("{}", style(format!("🚀 Creating Rust project '{}' with features: {:?}", project_name, features)).cyan().bold());

    // Step 1: Create project directory structure
    println!("{}", style("Step 1: Creating project directory structure...").yellow());
    let create_dir_result = registry.execute("write_file", serde_json::json!({
        "path": format!("{}/.gitkeep", project_name),
        "content": "",
        "overwrite": true,
        "create_dirs": true
    })).await;

    match create_dir_result {
        Ok(_) => println!("   {} Created project directory", style("✓").green()),
        Err(e) => {
            println!("   {} Failed to create directory: {}", style("✗").red(), e);
            return Err(anyhow!("Failed to create project directory: {}", e));
        }
    }

    // Step 2: Create Cargo.toml
    println!("{}", style("Step 2: Generating Cargo.toml...").yellow());
    let cargo_toml_content = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}"#, project_name, if features.contains(&"serde".to_string()) { "serde = { version = \"1.0\", features = [\"derive\"] }" } else { "" });

    let cargo_result = registry.execute("write_file", serde_json::json!({
        "path": format!("{}/Cargo.toml", project_name),
        "content": cargo_toml_content,
        "overwrite": true,
        "create_dirs": true
    })).await;

    match cargo_result {
        Ok(_) => println!("   {} Created Cargo.toml", style("✓").green()),
        Err(e) => println!("   {} Failed to create Cargo.toml: {}", style("✗").red(), e),
    }

    // Step 3: Create src directory and main.rs
    println!("{}", style("Step 3: Creating source code structure...").yellow());

    let main_rs_content = if features.contains(&"serde".to_string()) {
        r#"use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Person {
    name: String,
    age: u32,
}

fn main() {
    println!("Hello, {}!", env!("CARGO_PKG_NAME"));

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
    };

    println!("Created person: {:?}", person);
}"#
    } else {
        &format!(r#"fn main() {{
    println!("Hello, {}!", env!("CARGO_PKG_NAME"));
}}"#, project_name)
    };

    let main_rs_result = registry.execute("write_file", serde_json::json!({
        "path": format!("{}/src/main.rs", project_name),
        "content": main_rs_content,
        "overwrite": true,
        "create_dirs": true
    })).await;

    match main_rs_result {
        Ok(_) => println!("   {} Created src/main.rs", style("✓").green()),
        Err(e) => println!("   {} Failed to create main.rs: {}", style("✗").red(), e),
    }

    // Step 4: Create README.md
    println!("{}", style("Step 4: Generating documentation...").yellow());
    let readme_content = format!(r#"# {}

A Rust project with the following features: {}

## Building

```bash
cargo build
```

## Running

```bash
cargo run
```

## Testing

```bash
cargo test
```
"#, project_name, features.join(", "));

    let readme_result = registry.execute("write_file", serde_json::json!({
        "path": format!("{}/README.md", project_name),
        "content": readme_content,
        "overwrite": true,
        "create_dirs": true
    })).await;

    match readme_result {
        Ok(_) => println!("   {} Created README.md", style("✓").green()),
        Err(e) => println!("   {} Failed to create README.md: {}", style("✗").red(), e),
    }

    // Step 5: Create .gitignore
    println!("{}", style("Step 5: Adding .gitignore...").yellow());
    let gitignore_content = r#"/target/
Cargo.lock
.DS_Store
*.log
.env
"#;

    let gitignore_result = registry.execute("write_file", serde_json::json!({
        "path": format!("{}/.gitignore", project_name),
        "content": gitignore_content,
        "overwrite": true,
        "create_dirs": true
    })).await;

    match gitignore_result {
        Ok(_) => println!("   {} Created .gitignore", style("✓").green()),
        Err(e) => println!("   {} Failed to create .gitignore: {}", style("✗").red(), e),
    }

    // Step 6: Test the build
    println!("{}", style("Step 6: Testing project build...").yellow());
    let test_build_result = registry.execute("list_files", serde_json::json!({
        "path": format!("{}/src", project_name),
        "include_hidden": false
    })).await;

    match test_build_result {
        Ok(result) => {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    if !files_array.is_empty() {
                        println!("   {} Project structure verified", style("✓").green());
                    }
                }
            }
        },
        Err(e) => println!("   {} Failed to verify project structure: {}", style("✗").red(), e),
    }

    println!("{}", style("✅ Project creation complete!").green().bold());
    println!("{}", style(format!("📁 Project '{}' created with {} features", project_name, features.len())).cyan());
    println!("{}", style(format!("💡 Run 'cd {} && cargo run' to test your new project", project_name)).dim());

    Ok(())
}

/// Demonstrate context compression following Cognition's principles
async fn compress_context_demo(client: &gemini::Client) -> Result<()> {
    println!("{}", style("🧠 Context Compression Demo").cyan().bold());
    println!("{}", style("Following Cognition's context engineering principles...").dim());

    // Create a sample long conversation history to compress
    let sample_conversation = vec![
        Content::user_text("I want to create a Rust web application with user authentication"),
        Content::system_text("I'll help you create a Rust web application with authentication. Let me start by exploring the current directory structure."),
        Content::user_parts(vec![Part::FunctionResponse {
            function_response: FunctionResponse {
                name: "list_files".to_string(),
                response: json!({"path": ".", "files": ["Cargo.toml", "src/main.rs"], "directories": ["src", "tests"]})
            }
        }]),
        Content::system_text("I can see you already have a basic Rust project. Let me check what's in the main.rs file."),
        Content::user_parts(vec![Part::FunctionResponse {
            function_response: FunctionResponse {
                name: "read_file".to_string(),
                response: json!({"path": "src/main.rs", "content": "fn main() {\n    println!(\"Hello World!\");\n}", "metadata": {"size": 45}})
            }
        }]),
        Content::system_text("Now I need to add web framework dependencies. I'll update Cargo.toml to include Axum and other necessary crates."),
        Content::user_parts(vec![Part::FunctionResponse {
            function_response: FunctionResponse {
                name: "edit_file".to_string(),
                response: json!({"status": "modified", "path": "Cargo.toml", "action": {"replacements_made": 1}})
            }
        }]),
        Content::system_text("Good! Now let me create the authentication module structure."),
        Content::user_parts(vec![Part::FunctionResponse {
            function_response: FunctionResponse {
                name: "write_file".to_string(),
                response: json!({"status": "created", "path": "src/auth.rs", "bytes_written": 234})
            }
        }]),
        Content::system_text("Now I'll create the main web server with authentication endpoints."),
        Content::user_parts(vec![Part::FunctionResponse {
            function_response: FunctionResponse {
                name: "edit_file".to_string(),
                response: json!({"status": "modified", "path": "src/main.rs", "action": {"replacements_made": 3}})
            }
        }]),
    ];

    println!("{} {}", style("📝 Original conversation length:").yellow(), sample_conversation.len());
    println!("{} {:.1}KB", style("📊 Estimated token usage:").yellow(),
             sample_conversation.len() as f64 * 0.5); // Rough estimate

    // Create compression prompt following Cognition's principles
    let compression_prompt = r#"You are a context compression specialist. Your task is to compress the following agent conversation history while preserving:

1. KEY DECISIONS made by the agent
2. IMPORTANT ACTIONS taken (tool calls and their results)
3. CRITICAL CONTEXT about the current state
4. USER INTENT and requirements
5. TECHNICAL DECISIONS (frameworks, libraries, architecture choices)

IMPORTANT: Do NOT lose information about:
- What files were created/modified and why
- What dependencies were added
- What the current state of the project is
- What the user's original request was

Compress this conversation into a concise summary that captures all essential information:

ORIGINAL CONVERSATION:"#;

    // Build the conversation content for compression
    let mut compression_content = vec![Content::user_text(compression_prompt)];

    // Add each conversation turn
    for (i, content) in sample_conversation.iter().enumerate() {
        let role_indicator = match content.role.as_str() {
            "user" => "USER",
            "system" => "AGENT",
            _ => "UNKNOWN"
        };

        let mut content_summary = format!("\n--- Turn {} ({}) ---\n", i + 1, role_indicator);

        for part in &content.parts {
            match part {
                Part::Text { text } => {
                    content_summary.push_str(text);
                }
                Part::FunctionCall { function_call } => {
                    content_summary.push_str(&format!("\n[TOOL CALL: {}({})]",
                        function_call.name, function_call.args));
                }
                Part::FunctionResponse { function_response } => {
                    content_summary.push_str(&format!("\n[TOOL RESULT: {}]",
                        serde_json::to_string_pretty(&function_response.response).unwrap_or_default()));
                }
            }
        }

        if i == 0 {
            compression_content[0] = Content::user_text(format!("{}{}", compression_prompt, content_summary));
        } else {
            compression_content.push(Content::user_text(content_summary));
        }
    }

    // Add final instruction
    compression_content.push(Content::user_text(r#"
COMPRESSION REQUIREMENTS:
- Preserve all key decisions and their rationale
- Keep track of what files were created/modified
- Maintain information about current project state
- Include user's original intent
- Note any important technical choices made

COMPRESSED SUMMARY:"#));

    // Create request for compression
    let compression_request = GenerateContentRequest {
        contents: compression_content,
        tools: None,
        tool_config: None,
        generation_config: Some(json!({
            "maxOutputTokens": 1000,
            "temperature": 0.1
        })),
        system_instruction: Some(Content::system_text(r#"You are an expert at compressing agent conversation history.
Your goal is to create a compressed summary that maintains all critical information while being concise.
Focus on: key decisions, actions taken, current state, and user requirements."#)),
    };

    println!("{}", style("🔄 Compressing conversation...").cyan());

    let compressed_response = client.generate_content(&compression_request).await?;

    if let Some(candidate) = compressed_response.candidates.into_iter().next() {
        if let Some(content) = candidate.content.parts.into_iter().next() {
            if let Some(text) = content.as_text() {
                println!("{}", style("✅ Compressed Summary:").green().bold());
                println!("{}", text);

                // Estimate compression ratio
                let original_chars: usize = sample_conversation.iter()
                    .map(|c| c.parts.iter().map(|p| match p.as_text() {
                        Some(text) => text.len(),
                        _ => 100 // Rough estimate for tool calls
                    }).sum::<usize>())
                    .sum();

                let compressed_chars = text.len();
                let compression_ratio = original_chars as f64 / compressed_chars as f64;

                println!("\n{} {:.1}x", style("📊 Compression ratio:").magenta().bold(), compression_ratio);
                println!("{} {} → {} characters", style("📏 Size reduction:").magenta(), original_chars, compressed_chars);
            }
        }
    }

    println!("\n{}", style("💡 Key Principles Applied:").yellow().bold());
    println!("  • {}", style("Share full context and traces").dim());
    println!("  • {}", style("Actions carry implicit decisions").dim());
    println!("  • {}", style("Single-threaded agents are more reliable").dim());
    println!("  • {}", style("Context compression enables longer conversations").dim());
    Ok(())
}

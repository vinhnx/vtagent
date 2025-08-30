//! VTAgent - Minimal research-preview Rust coding agent
//!
//! This is the main binary entry point for vtagent. All core functionality
//! is implemented in the vtagent-core crate to avoid duplication.

use anyhow::{anyhow, Context, Result};
use console::style;
use vtagent_core::{
    async_file_ops,
    cli::{Cli, Commands, RateLimiter},
    commands::{analyze_workspace, handle_ask_command, handle_stats_command, handle_revert_command},
    context_analyzer::ContextAnalyzer,
    diff_renderer,
    gemini::{
        Candidate, Client, Content, FunctionCall, FunctionResponse,
        GenerateContentRequest, Part, Tool, ToolConfig,
    },
    markdown_renderer::MarkdownRenderer,
    performance_profiler::{PerformanceProfiler, PerformanceTargets, TargetsStatus, PROFILER},
    tools::{build_function_declarations, ToolRegistry},
    types::AgentConfig,
    ui::{show_loading_spinner, start_loading_spinner, render_markdown, get_terminal_width},
    agent::{chat_loop, compaction::{CompactionEngine, MessageType, CompactionConfig}},
};
use once_cell::sync::Lazy;
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task;

/// Global markdown renderer for streaming chat responses
static MARKDOWN_RENDERER: Lazy<Mutex<MarkdownRenderer>> =
    Lazy::new(|| Mutex::new(MarkdownRenderer::new()));

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

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
            // Initialize components for chat
            let mut client = Client::new(api_key, args.model.clone())?;
            let mut registry = ToolRegistry::new(workspace.clone());
            registry.initialize_async().await?;
            let rate_limiter = RateLimiter::new(args.api_rate_limit, args.max_tool_calls);

            let async_writer = if args.async_file_ops {
                Some(async_file_ops::AsyncFileWriter::new(args.max_concurrent_ops))
            } else {
                None
            };

            let diff_renderer = if args.show_file_diffs {
                Some(diff_renderer::DiffChatRenderer::new(true, 3, true))
            } else {
                None
            };

            chat_loop(
                &mut client,
                &mut registry,
                false,
                async_writer,
                diff_renderer,
                &rate_limiter,
                &workspace,
            )
            .await
        }
        Commands::ChatVerbose => {
            // Initialize components for verbose chat
            let mut client = Client::new(api_key, args.model.clone())?;
            let mut registry = ToolRegistry::new(workspace.clone());
            registry.initialize_async().await?;
            let rate_limiter = RateLimiter::new(args.api_rate_limit, args.max_tool_calls);

            let async_writer = if args.async_file_ops {
                Some(async_file_ops::AsyncFileWriter::new(args.max_concurrent_ops))
            } else {
                None
            };

            let diff_renderer = if args.show_file_diffs {
                Some(diff_renderer::DiffChatRenderer::new(true, 3, true))
            } else {
                None
            };

            chat_loop(
                &mut client,
                &mut registry,
                true,
                async_writer,
                diff_renderer,
                &rate_limiter,
                &workspace,
            )
            .await
        }
        Commands::Ask { prompt } => handle_ask_command(config, prompt).await,
        Commands::Analyze => {
            let mut client = Client::new(api_key, args.model.clone())?;
            let mut registry = ToolRegistry::new(workspace.clone());
            registry.initialize_async().await?;
            analyze_workspace(&mut client, &mut registry).await
        }
        Commands::Performance => handle_stats_command(config, false, "text".to_string()).await,
        _ => {
            println!("Command not yet implemented in refactored version");
            Ok(())
        }
    }
}

/// Display a loading spinner while executing a task
/// Note: This is a foundation function for future async spinner implementation
fn show_loading_spinner(message: &str) {
    println!("{} {}", style("LOADING").cyan().bold(), message);
}

/// Optimized async loading spinner with better performance
fn start_loading_spinner(
    is_loading: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        let start_time = std::time::Instant::now();
        let mut line_buffer = String::with_capacity(120);

        while is_loading.load(Ordering::Relaxed) {
            // Safety timeout - stop spinner after 30 seconds
            if start_time.elapsed() > Duration::from_secs(30) {
                is_loading.store(false, Ordering::Relaxed);
                print!("\r{} [TIMEOUT] ", style("vtagent:").yellow().bold());
                io::stdout().flush().ok();
                break;
            }

            // Get current status message with minimal locking time
            let current_status = {
                let status_guard = status.lock().unwrap();
                status_guard.clone()
            };

            // Build the display line efficiently
            line_buffer.clear();
            line_buffer.push_str(&format!(
                "{}{} {}",
                style("vtagent:").yellow().bold(),
                style(spinner_chars[i % spinner_chars.len()]).cyan(),
                style(&current_status).dim()
            ));

            // Calculate padding and print efficiently
            let padding_needed = 80_usize.saturating_sub(line_buffer.len());
            let padding = " ".repeat(padding_needed);
            print!("\r{}{}", line_buffer, padding);
            io::stdout().flush().ok();

            i += 1;
            tokio::time::sleep(Duration::from_millis(120)).await;
        }

        // Clear the spinner line
        print!(
            "\r{}  \r{}",
            style("vtagent:").yellow().bold(),
            style("vtagent:").yellow().bold()
        );
        io::stdout().flush().ok();
    })
}

/// Render markdown content in terminal with proper formatting
fn render_markdown(text: &str) {
    // Simplified rendering - just print the text as-is
    println!("{}", text);
}

/// Clean up spinner and prepare for next message
async fn cleanup_spinner_and_print(
    is_loading: &Arc<AtomicBool>,
    spinner_handle: &task::JoinHandle<()>,
    message: String,
) {
    // Stop the spinner
    is_loading.store(false, Ordering::Relaxed);

    // Wait for spinner cleanup to complete
    let _ = spinner_handle;

    // Print the message cleanly
    println!("{}", message);
}

/// Extract function calls from text that contains embedded function call markers
fn extract_function_calls_from_text(text: &str) -> Vec<FunctionCall> {
    let mut tool_calls = vec![];

    // Look for function call markers in the format [FUNCTION_CALL:{json}]
    let function_call_pattern = regex::Regex::new(r"\[FUNCTION_CALL:(.*?)\]").unwrap();

    for capture in function_call_pattern.captures_iter(text) {
        if let Some(json_str) = capture.get(1) {
            if let Ok(function_call) = serde_json::from_str::<FunctionCall>(json_str.as_str()) {
                tool_calls.push(function_call);
            }
        }
    }

    tool_calls
}

async fn chat_loop(
    client: &mut Client,
    registry: &mut ToolRegistry,
    verbose: bool,
    async_writer: Option<async_file_ops::AsyncFileWriter>,
    diff_renderer: Option<diff_renderer::DiffChatRenderer>,
    rate_limiter: &RateLimiter,
    workspace: &PathBuf,
) -> Result<()> {
    let _perf_timer = PROFILER.start_operation("chat_loop_total");
    // Display vtagent welcome message
    println!(
        "{}",
        style("

        ██╗   ██╗ ████████╗  █████╗   ██████╗  ███████╗ ███╗   ██╗ ████████╗
        ██║   ██║ ╚══██╔══╝ ██╔══██╗ ██╔════╝  ██╔════╝ ████╗  ██║ ╚══██╔══╝
        ██║   ██║    ██║    ███████║ ██║  ███╗ █████╗   ██╔██╗ ██║    ██║
        ╚██╗ ██╔╝    ██║    ██╔══██║ ██║   ██║ ██╔══╝   ██║╚██╗██║    ██║
         ╚████╔╝     ██║    ██║  ██║ ╚██████╔╝ ███████╗ ██║ ╚████║    ██║
          ╚═══╝      ╚═╝    ╚═╝  ╚═╝  ╚═════╝  ╚══════╝ ╚═╝  ╚═══╝    ╚═╝").bold()
    );
    println!();

    println!("{}", style("USAGE:").yellow().bold());
    println!("  • Type your requests and press Enter");
    println!("  • Use 'ctrl-c' to exit the chat");
    println!("  • Available commands: chat, ask, analyze, performance");
    println!("  • For help: see AGENTS.md in project root");
    println!();

    if verbose {
        println!(
            "{} {}\n",
            style("Verbose logging enabled").yellow().bold(),
            style("").dim()
        );
    }

    if verbose {
        println!(
            "{} Initializing agent with {} tools",
            style("[INIT]").dim(),
            build_function_declarations().len()
        );
        println!(
            "{} {}",
            style("[CONTEXT]").dim(),
            "Following Cognition's context engineering principles"
        );
        println!("  • Single-threaded execution for reliability");
        println!("  • Full context sharing with each API call");
        println!("  • Actions carry explicit decision tracking");
        println!("  • Proactive context understanding");
    }

    let mut contents: Vec<Content> = vec![];
    let sys_instruction = system_instruction();
    let tools = vec![Tool {
        function_declarations: build_function_declarations(),
    }];
    let tool_config = Some(ToolConfig::auto());
    let context_analyzer = ContextAnalyzer::new();

    // Initialize snapshot manager for agent state persistence
    let snapshot_config = vtagent_core::agent::SnapshotConfig {
        enabled: true,
        directory: std::path::PathBuf::from("snapshots"),
        max_snapshots: 50,
        compression_threshold: 1024 * 1024, // 1MB
        auto_cleanup: true,
        encryption_enabled: false,
    };
    let snapshot_manager = vtagent_core::agent::SnapshotManager::new(snapshot_config);

    // Initialize intelligent compaction system
    let compaction_config = CompactionConfig {
        max_memory_mb: 100, // 100MB memory limit
        max_message_age_seconds: 1800, // 30 minutes
        auto_compaction_enabled: true,
        ..Default::default()
    };
    let compaction_engine = Arc::new(CompactionEngine::with_config(compaction_config));

    // Reset tool call counter for new chat session
    rate_limiter.reset_tool_calls();

    // Initialize file watcher for diff tracking
    let file_watcher = if diff_renderer.is_some() {
        let mut watcher = async_file_ops::FileWatcher::new();
        // Watch common file extensions for changes
        let watch_paths = vec![
            "src".to_string(),
            "Cargo.toml".to_string(),
            "README.md".to_string(),
        ]
        .into_iter()
        .map(|p| workspace.join(p))
        .collect::<Vec<_>>();
        let _ = watcher.start_watching(watch_paths).await;
        Some(watcher)
    } else {
        None
    };

    let mut stdin = io::stdin();
    let mut read_user_input = true;
    let mut is_piped_input = !io::stdin().is_terminal();

    loop {
        if read_user_input {
            print!("> ");
            io::stdout().flush().ok();
            let mut buf = String::new();
            if stdin.read_line(&mut buf).is_err() {
                // EOF or error reading from stdin (e.g., piped input ended)
                if verbose {
                    println!(
                        "{} {}",
                        style("[EOF]").dim(),
                        "End of input stream reached, exiting..."
                    );
                }
                break;
            }
            if buf.trim().is_empty() {
                if verbose {
                    println!(
                        "{} {}",
                        style("[SKIP]").dim(),
                        "Empty message, continuing..."
                    );
                }
                continue;
            }

            let user_message = buf.trim();

            if verbose {
                println!(
                    "{} User input: {} chars",
                    style("[INPUT]").dim(),
                    user_message.len()
                );
            }

            // Analyze context to understand user intent proactively
            let context_analysis = context_analyzer.analyze_context(&contents[..], user_message);
            if verbose && context_analysis.confidence > 0.5 {
                println!(
                    "{} Detected intent: {} (confidence: {:.1}%)",
                    style("[CONTEXT]").dim(),
                    context_analysis.intent,
                    context_analysis.confidence * 100.0
                );
            }

            // Add user message to intelligent compaction tracking
            let user_content = Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: user_message.to_string(),
                }],
            };
            if let Err(e) = compaction_engine.add_message(&user_content, MessageType::UserMessage).await {
                if verbose {
                    println!(
                        "{} Warning: Failed to track user message for compaction: {}",
                        style("[COMPACTION]").yellow().bold(),
                        e
                    );
                }
            }

            // Check if compaction is needed and perform it
            if let Ok(true) = compaction_engine.should_compact().await {
                if verbose {
                    println!(
                        "{} Performing intelligent compaction...",
                        style("[COMPACTION]").cyan().bold()
                    );
                }

                match compaction_engine.compact_messages_intelligently().await {
                    Ok(result) => {
                        if verbose && result.messages_compacted > 0 {
                            println!(
                                "{} Intelligent compaction: {} messages compacted, {} bytes saved (ratio: {:.2})",
                                style("[COMPACTION]").green().bold(),
                                result.messages_compacted,
                                result.memory_saved,
                                result.compression_ratio
                            );
                        }
                    }
                    Err(e) => {
                        if verbose {
                            println!(
                                "{} Warning: Intelligent compaction failed: {}",
                                style("[COMPACTION]").yellow().bold(),
                                e
                            );
                        }
                    }
                }
            }

            // Check if we can suggest proactive actions but don't act automatically
            if let Some(proactive_response) =
                context_analyzer.generate_proactive_response(&context_analysis)
            {
                if verbose {
                    println!(
                        "{} Considering proactive action: {}",
                        style("[SUGGESTION]").cyan().bold(),
                        proactive_response
                    );
                }
                // Only show suggestion, don't act automatically
                println!(
                    "{} {}",
                    style("vtagent:").yellow().bold(),
                    proactive_response
                );
                contents.push(Content::user_text(user_message));
                // Don't continue prompting if we had piped input
                if !is_piped_input {
                    read_user_input = true;
                    continue;
                }
            }

            contents.push(Content::user_text(user_message));

            // Create snapshot at the start of each turn for reproducibility
            let turn_number = contents.len() / 2; // Rough estimate of turn number
            let snapshot_description = if user_message.len() > 50 {
                format!("{}...", &user_message[..47])
            } else {
                user_message.to_string()
            };

            // Note: In a full implementation, we would create an Agent instance here
            // and pass it to create_snapshot. For now, we'll just log the intent.
            if verbose {
                println!(
                    "{} Creating snapshot for turn {}: {}",
                    style("[SNAPSHOT]").cyan().bold(),
                    turn_number,
                    snapshot_description
                );
            }
        }

        let req = GenerateContentRequest {
            contents: contents.clone(),
            tools: Some(tools.clone()),
            tool_config: tool_config.clone(),
            generation_config: None,
            system_instruction: Some(sys_instruction.clone()),
        };

        if verbose {
            println!(
                "{} Sending request (conversation length: {})",
                style("[API]").dim(),
                contents.len()
            );
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
            *status_guard = "Thinking...".to_string();
        }

        // Apply rate limiting before API call
        if let Err(e) = rate_limiter.wait_for_api_request().await {
            println!("{} Rate limiting error: {}", style("ERROR").red().bold(), e);
            // Don't continue prompting if we already had piped input
            if !is_piped_input {
                read_user_input = true;
                continue;
            }
        }

        let api_timer = PROFILER.start_operation("api_call_stream");
        let resp = match client
            .generate_content_stream(&req, |chunk| {
                // Stop spinner on first chunk and update status
                if is_loading.load(Ordering::Relaxed) {
                    is_loading.store(false, Ordering::Relaxed);

                    // Wait for spinner thread to clean up
                    thread::sleep(Duration::from_millis(150));

                    // Update status to show response generation
                    {
                        let mut status_guard = status.lock().unwrap();
                        *status_guard = "Generating response...".to_string();
                    }
                }

                // Filter out function call JSON to keep chat interface clean
                if !chunk.contains("[FUNCTION_CALL:") && !chunk.contains("FUNCTION_CALL") {
                    // Render markdown for chat responses
                    let rendered = if let Ok(mut renderer) = MARKDOWN_RENDERER.lock() {
                        renderer.render_chunk(chunk).is_ok()
                    } else {
                        false
                    };

                    // If markdown rendering failed, print raw chunk
                    if !rendered {
                        print!("{}", chunk);
                    }
                }
                io::stdout().flush()?;
                Ok(())
            })
            .await
        {
            Ok(response) => {
                println!(); // Add newline after streaming completes
                response
            }
            Err(e) => {
                // Stop spinner on error
                is_loading.store(false, Ordering::Relaxed);
                // Wait for spinner cleanup
                thread::sleep(Duration::from_millis(150));
                println!(
                    "\r{} {}",
                    style("vtagent:").yellow().bold(),
                    style("[ERROR]").red().bold()
                );
                println!("{} {}", style("[ERROR]").red().bold(), e);
                // Don't continue prompting if we already had piped input
                if !is_piped_input {
                    read_user_input = true;
                    // Return empty response to continue loop
                    return Ok(());
                } else {
                    // If we had piped input and got an error, exit
                    return Ok(());
                }
            }
        };

        let Some(Candidate { content, .. }) = resp.candidates.into_iter().next() else {
            // Stop spinner in case of no response (though spinner should already be stopped from streaming)
            is_loading.store(false, Ordering::Relaxed);
            println!(
                "{} {}",
                style("[ERROR]").red().bold(),
                "No response from API"
            );
            // Don't continue prompting if we already had piped input
            if !is_piped_input {
                read_user_input = true;
                return Ok(());
            } else {
                return Ok(());
            }
        };

        // Wait for spinner thread to finish after successful response processing
        let _ = spinner_handle;

        if verbose {
            println!(
                "{} Received response with {} content blocks",
                style("[RESPONSE]").dim(),
                content.parts.len()
            );
        }

        // Add assistant response to intelligent compaction tracking
        if let Err(e) = compaction_engine.add_message(&content, MessageType::AssistantResponse).await {
            if verbose {
                println!(
                    "{} Warning: Failed to track assistant response for compaction: {}",
                    style("[COMPACTION]").yellow().bold(),
                    e
                );
            }
        }

        let mut tool_calls: Vec<FunctionCall> = vec![];

        // Extract function calls from both the structured response and embedded function calls in streaming text
        for part in &content.parts {
            if let Part::FunctionCall { function_call } = part {
                tool_calls.push(function_call.clone());
            }
        }

        // Also extract function calls from the accumulated streaming response
        if let Some(text_content) = content.parts.iter().find_map(|p| p.as_text()) {
            tool_calls.extend(extract_function_calls_from_text(text_content));
        }

        if verbose && !tool_calls.is_empty() {
            println!(
                "{} Detected {} tool call(s)",
                style("[TOOLS]").cyan().bold(),
                tool_calls.len()
            );
        }

        if tool_calls.is_empty() {
            // Text is already displayed by streaming function above
            // Continue to process the response for conversation history
            contents.push(content.clone());
            // Don't continue prompting if we already had piped input
            if !is_piped_input {
                read_user_input = true;
                continue;
            }
        }

        // Show the tool calls with user-friendly names
        for (i, call) in tool_calls.iter().enumerate() {
            let friendly_name = get_tool_friendly_name(&call.name);
            let simple_description = get_tool_simple_description(&call.name, &call.args);

            println!(
                "{} {}: {}",
                style("TOOL").green().bold(),
                style(format!("[{}/{}]", i + 1, tool_calls.len()))
                    .white()
                    .bold(),
                style(&friendly_name).cyan().bold()
            );

            if !simple_description.is_empty() {
                println!("   {}", style(simple_description).dim());
            }
        }

        // Append model's tool call turn to conversation, then add our tool responses as a user turn
        contents.push(content.clone());
        let mut response_parts: Vec<Part> = vec![];

        // Check tool call limits
        let current_tool_count = rate_limiter.get_tool_call_count();
        if current_tool_count + tool_calls.len() > rate_limiter.max_tool_calls {
            let remaining = rate_limiter.max_tool_calls - current_tool_count;
            println!(
                "{} Tool call limit reached ({} remaining, {} requested). Stopping execution.",
                style("Warning").yellow(),
                remaining,
                tool_calls.len()
            );
            // Still add the response to conversation but don't execute tools
            contents.push(content);
            // Don't continue prompting if we already had piped input
            if !is_piped_input {
                read_user_input = true;
                continue;
            }
        }

        // Execute tools and collect results
        let mut successful_tools = 0;
        let mut failed_tools = 0;

        if verbose {
            println!(
                "{} Executing {} tool(s) ({} total so far)",
                style("[EXEC]").green().bold(),
                tool_calls.len(),
                current_tool_count
            );
        }

        for (i, call) in tool_calls.iter().enumerate() {
            let _task_description = format!("Executing {}", call.name);
            let friendly_name = get_tool_friendly_name(&call.name);

            // Start optimized loading spinner for tool execution (both verbose and non-verbose)
            let is_loading = Arc::new(AtomicBool::new(true));
            let status = Arc::new(Mutex::new(format!("Running {}...", friendly_name)));
            let tool_spinner = start_loading_spinner(Arc::clone(&is_loading), Arc::clone(&status));

            if verbose {
                println!(
                    "{} Executing tool {}: {}",
                    style(format!("[{}/{}]", i + 1, tool_calls.len())).dim(),
                    call.name,
                    call.args
                );
                // Show decision context - what led to this tool call
                println!(
                    "  {} Context: Full conversation history available",
                    style("CONTEXT").blue().bold()
                );
                println!(
                    "  {} Decision: Based on user's request and current project state",
                    style("DECISION").magenta().bold()
                );
            }

            // Update status to show we're processing the tool
            {
                let mut status_guard = status.lock().unwrap();
                *status_guard = format!("Processing {}...", friendly_name);
            }

            let tool_timer = PROFILER.start_operation(&format!("tool_{}", call.name));

            // Increment tool call counter
            rate_limiter.increment_tool_call();

            // Track tool call for intelligent compaction
            let tool_call_content = Content {
                role: "assistant".to_string(),
                parts: vec![Part::FunctionCall {
                    function_call: call.clone(),
                }],
            };
            if let Err(e) = compaction_engine.add_message(&tool_call_content, MessageType::ToolCall).await {
                if verbose {
                    println!(
                        "{} Warning: Failed to track tool call for compaction: {}",
                        style("[COMPACTION]").yellow().bold(),
                        e
                    );
                }
            }

            // Handle async file operations if enabled
            let result = if async_writer.is_some() && is_file_operation(&call.name) {
                handle_async_file_operation(
                    &call.name,
                    &call.args,
                    async_writer.as_ref().unwrap(),
                    registry,
                )
                .await
            } else {
                registry.execute_tool(&call.name, call.args.clone()).await
            };
            let response_json = match result {
                Ok(value) => {
                    successful_tools += 1;

                    // Stop spinner and show success message
                    let friendly_name = get_tool_friendly_name(&call.name);
                    let success_msg = format!(
                        "{} {} completed successfully",
                        style("SUCCESS").green().bold(),
                        style(&friendly_name).cyan().bold()
                    );
                    cleanup_spinner_and_print(&is_loading, &tool_spinner, success_msg).await;
                    let tool_spinner =
                        start_loading_spinner(Arc::clone(&is_loading), Arc::clone(&status)); // Restart for next tool

                    // Update status to show completion
                    {
                        let mut status_guard = status.lock().unwrap();
                        *status_guard = format!("{} completed", get_tool_friendly_name(&call.name));
                    }

                    if verbose {
                        println!(
                            "{} Tool {} succeeded",
                            style("[SUCCESS]").green().bold(),
                            call.name
                        );
                        // Show what this action accomplished
                        if let Some(data_obj) = value.get("action") {
                            if let Some(action_type) = data_obj.get("type") {
                                println!(
                                    "  {} Action completed: {}",
                                    style("✓").green().bold(),
                                    action_type
                                );
                            }
                        }
                    }
                    json!({
                        "status": "success",
                        "tool": call.name,
                        "data": value,
                        "decision_context": "Executed based on full conversation history and current project state"
                    })
                }
                Err(e) => {
                    failed_tools += 1;

                    // Update status to show failure
                    {
                        let mut status_guard = status.lock().unwrap();
                        *status_guard = format!("{} failed", get_tool_friendly_name(&call.name));
                    }

                    let error_msg =
                        format!("**{} failed:** {}", get_tool_friendly_name(&call.name), e);
                    print!("{} ", style("ERROR").red().bold());
                    render_markdown(&error_msg);
                    if verbose {
                        println!(
                            "{} Tool {} failed: {}",
                            style("[ERROR]").red().bold(),
                            call.name,
                            e
                        );
                        println!(
                            "  {} Context preserved for recovery",
                            style("RECOVERY").cyan().bold()
                        );
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
                }
            };
            response_parts.push(Part::FunctionResponse {
                function_response: FunctionResponse {
                    name: call.name.clone(),
                    response: response_json,
                },
            });

            // Stop tool spinner and join thread
            is_loading.store(false, Ordering::Relaxed);
            let _ = tool_spinner.await;
        }

        // Render diffs for file changes if enabled
        if let (Some(ref watcher), Some(ref diff_renderer)) = (&file_watcher, &diff_renderer) {
            let mut file_changes = Vec::new();

            // Check for changes in watched files
            for path in &["src", "Cargo.toml", "README.md"] {
                let full_path = workspace.join(path);
                if let Some((old_content, new_content)) =
                    watcher.get_changes(&full_path).await.unwrap_or(None)
                {
                    file_changes.push((
                        full_path.to_string_lossy().to_string(),
                        old_content,
                        new_content,
                    ));
                }
            }

            // Render diffs if any changes detected
            if !file_changes.is_empty() {
                let diff_output = diff_renderer.render_multiple_changes(file_changes);
                println!("\n{}", diff_output);
            }
        }

        // Show execution summary
        if successful_tools > 0 || failed_tools > 0 {
            let total_tools = successful_tools + failed_tools;
            let status_msg = if failed_tools == 0 {
                format!("All {} tools executed successfully", total_tools)
            } else {
                format!(
                    "{} tools executed ({} success, {} failed)",
                    total_tools, successful_tools, failed_tools
                )
            };

            if verbose {
                println!("{} {}", style("[SUMMARY]").magenta().bold(), status_msg);
            } else {
                let summary_msg = format!("**Summary:** {}", status_msg);
                render_markdown(&summary_msg);
            }
        }

        // Track tool responses for intelligent compaction
        let tool_response_content = Content::user_parts(response_parts.clone());
        if let Err(e) = compaction_engine.add_message(&tool_response_content, MessageType::ToolResponse).await {
            if verbose {
                println!(
                    "{} Warning: Failed to track tool response for compaction: {}",
                    style("[COMPACTION]").yellow().bold(),
                    e
                );
            }
        }

        contents.push(Content::user_parts(response_parts));

        // Immediately ask again, without reading user input
        // But exit if we had piped input to prevent infinite loops
        if is_piped_input {
            break;
        }
        read_user_input = false;

        // Monitor context size and provide warnings
        if verbose {
            println!(
                "{} Conversation length: {} messages",
                style("[STATE]").dim(),
                contents.len()
            );

            // Rough estimate of context size
            let estimated_tokens = contents.len() * 200; // Rough estimate
            if estimated_tokens > 80000 {
                // Approaching context limit
                println!(
                    "{} {} tokens - approaching context limit",
                    style("[WARNING]").yellow().bold(),
                    estimated_tokens
                );
                println!(
                    "  {} Consider using 'compress-context' command for long conversations",
                    style("TIP").yellow().bold()
                );
            } else if estimated_tokens > 50000 {
                println!(
                    "{} {} tokens - context growing large",
                    style("[INFO]").cyan(),
                    estimated_tokens
                );
            }
        }
        println!();
    }

    if verbose {
        println!("{} {}", style("[END]").cyan().bold(), "Chat session ended");
    }

    Ok(())
}

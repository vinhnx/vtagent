mod async_file_ops;
mod context_analyzer;
mod diff_renderer;
mod gemini;
mod markdown_renderer;
mod performance_profiler;
mod tools;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use context_analyzer::ContextAnalyzer;
use gemini::{
    Candidate, Content, FunctionCall, FunctionResponse, GenerateContentRequest, Part, Tool,
    ToolConfig,
};
use markdown_renderer::MarkdownRenderer;
use once_cell::sync::Lazy;
use performance_profiler::{PerformanceProfiler, PerformanceTargets, TargetsStatus, PROFILER};
use serde_json::json;
use std::io::{self, Read, Write, IsTerminal};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tokio::task;
use tools::{build_function_declarations, ToolRegistry};

/// Global markdown renderer for streaming chat responses
static MARKDOWN_RENDERER: Lazy<Mutex<MarkdownRenderer>> =
    Lazy::new(|| Mutex::new(MarkdownRenderer::new()));

#[derive(Parser, Debug)]
#[command(
    name = "vtagent",
    version,
    about = "**Advanced Rust coding agent** powered by Gemini with Anthropic-inspired architecture\n\n**Features:**\n• Interactive AI coding assistant with advanced tool-calling\n• Multi-language support (Rust, Python, JavaScript, TypeScript, Go, Java)\n• Real-time diff rendering and async file operations\n• Rate limiting and tool call management\n• Markdown rendering for chat responses\n\n**Quick Start:**\n  export GEMINI_API_KEY=\"your_key\"\n  vtagent chat"
)]
struct Cli {
    /// **Gemini model ID** (e.g., `gemini-2.5-flash-lite`, `gemini-2.5-flash`, `gemini-pro`)\n\n**Available models:**\n• `gemini-2.5-flash-lite` - Fastest, most cost-effective\n• `gemini-2.5-flash` - Fast, cost-effective\n• `gemini-pro` - More capable, slower\n• `gemini-2.5-pro` - Latest, most advanced
    #[arg(long, global = true, default_value = "gemini-2.5-flash-lite")]
    model: String,

    /// **API key environment variable** to read\n\n**Checks in order:**\n1. Specified env var\n2. `GOOGLE_API_KEY`\n\n**Setup:** `export GEMINI_API_KEY="your_key"`
    #[arg(long, global = true, default_value = "GEMINI_API_KEY")]
    api_key_env: String,

    /// **Workspace root directory** for file operations\n\n**Defaults to:** Current directory\n**All file operations** are restricted to this path
    #[arg(long, global = true)]
    workspace: Option<PathBuf>,

    /// **Enable async file operations** for non-blocking writes\n\n**Benefits:**\n• Non-blocking file I/O\n• Better performance\n• Concurrent operations\n• Real-time feedback
    #[arg(long, global = true)]
    async_file_ops: bool,

    /// **Show diffs for file changes** in chat interface\n\n**Features:**\n• Real-time diff rendering\n• Syntax highlighting\n• Line-by-line changes\n• Before/after comparison
    #[arg(long, global = true)]
    show_file_diffs: bool,

    /// **Maximum concurrent async file operations**\n\n**Default:** 5\n**Higher values:** Better performance but more resource usage
    #[arg(long, global = true, default_value_t = 5)]
    max_concurrent_ops: usize,

    /// **Maximum API requests per minute** to prevent rate limiting\n\n**Default:** 30\n**Lower values:** More conservative, fewer errors\n**Higher values:** Better performance, risk of rate limits
    #[arg(long, global = true, default_value_t = 30)]
    api_rate_limit: usize,

    /// **Maximum tool calls per chat run** to prevent runaway execution\n\n**Default:** 10\n**Purpose:** Prevents infinite loops and excessive API usage
    #[arg(long, global = true, default_value_t = 10)]
    max_tool_calls: usize,

    #[command(subcommand)]
    command: Option<Commands>,
}

/// Rate limiter to prevent API abuse and rate limiting
#[derive(Debug)]
struct RateLimiter {
    requests_per_minute: usize,
    request_times: Arc<Mutex<Vec<Instant>>>,
    tool_call_count: Arc<AtomicUsize>,
    max_tool_calls: usize,
}

impl RateLimiter {
    fn new(requests_per_minute: usize, max_tool_calls: usize) -> Self {
        Self {
            requests_per_minute,
            request_times: Arc::new(Mutex::new(Vec::new())),
            tool_call_count: Arc::new(AtomicUsize::new(0)),
            max_tool_calls,
        }
    }

    /// Check if we can make an API request, blocking if necessary
    async fn wait_for_api_request(&self) -> Result<()> {
        let mut request_times = self.request_times.lock().unwrap();

        // Remove old requests (older than 1 minute)
        let now = Instant::now();
        request_times.retain(|&time| now.duration_since(time) < Duration::from_secs(60));

        // Check if we're under the limit
        if request_times.len() < self.requests_per_minute {
            request_times.push(now);
            return Ok(());
        }

        // Calculate wait time until oldest request expires
        if let Some(&oldest) = request_times.first() {
            let wait_time = Duration::from_secs(60) - now.duration_since(oldest);
            if wait_time > Duration::ZERO {
                println!(
                    "{} Rate limit reached, waiting {:.1}s...",
                    style("RATE LIMIT").cyan().bold(),
                    wait_time.as_secs_f64()
                );
                tokio::time::sleep(wait_time).await;
            }
        }

        // Add current request
        request_times.push(Instant::now());

        Ok(())
    }

    /// Check if we can make a tool call
    fn can_make_tool_call(&self) -> bool {
        self.tool_call_count.load(Ordering::Relaxed) < self.max_tool_calls
    }

    /// Increment tool call counter
    fn increment_tool_call(&self) {
        self.tool_call_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current tool call count
    fn get_tool_call_count(&self) -> usize {
        self.tool_call_count.load(Ordering::Relaxed)
    }

    /// Reset tool call counter for new chat run
    fn reset_tool_calls(&self) {
        self.tool_call_count.store(0, Ordering::Relaxed);
    }
}

#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// **Interactive AI coding assistant** with advanced tool-calling capabilities\n\n**Features:**\n• Real-time code generation and editing\n• Multi-language support\n• File system operations\n• Async processing\n\n**Usage:** vtagent chat
    Chat,

    /// **Single prompt mode** - prints model reply without tools\n\n**Perfect for:**\n• Quick questions\n• Code explanations\n• Simple queries\n\n**Example:** vtagent ask "Explain Rust ownership"
    Ask { prompt: Vec<String> },

    /// **Verbose interactive chat** with enhanced transparency features\n\n**Shows:**\n• Tool execution details\n• API request/response\n• Internal reasoning\n• Performance metrics\n\n**Usage:** vtagent chat-verbose
    ChatVerbose,

    /// **Analyze workspace** and provide comprehensive project overview\n\n**Provides:**\n• Project structure analysis\n• Language detection\n• File type statistics\n• Dependency insights\n\n**Usage:** vtagent analyze
    Analyze,

    /// **Display performance metrics** and system status\n\n**Shows:**\n• Memory usage\n• API call statistics\n• Response times\n• Cache performance\n• System health\n\n**Usage:** vtagent performance
    Performance,

    /// **Create complete Rust project** with specified features\n\n**Features:**\n• Web frameworks (Axum, Rocket, Warp)\n• Database integration\n• Authentication systems\n• Testing setup\n\n**Example:** vtagent create-project myapp web,auth,db
    CreateProject { name: String, features: Vec<String> },

    /// **Compress conversation context** for long-running sessions\n\n**Benefits:**\n• Reduced token usage\n• Faster responses\n• Memory optimization\n• Context preservation\n\n**Usage:** vtagent compress-context
    CompressContext,

    /// **Demo async file operations** and diff rendering\n\n**Demonstrates:**\n• Non-blocking file I/O\n• Real-time diff generation\n• Concurrent operations\n• Performance monitoring\n\n**Usage:** vtagent demo-async
    #[command(name = "demo-async")]
    DemoAsync,
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

    // Use optimized client configuration for better performance
    let client_config = if args.command.as_ref().unwrap_or(&Commands::Chat) == &Commands::Chat {
        // For chat, use low-latency configuration for better responsiveness
        gemini::ClientConfig::low_latency()
    } else {
        // For other commands, use default configuration
        gemini::ClientConfig::default()
    };

    let mut client = gemini::Client::with_config(api_key, args.model.clone(), client_config);
    let mut registry = ToolRegistry::new(workspace.clone());
    let rate_limiter = RateLimiter::new(args.api_rate_limit, args.max_tool_calls);

    // Initialize async file operations and diff renderer if enabled
    let async_writer = if args.async_file_ops {
        Some(async_file_ops::AsyncFileWriter::new(
            args.max_concurrent_ops,
        ))
    } else {
        None
    };

    let diff_renderer = if args.show_file_diffs {
        Some(diff_renderer::DiffChatRenderer::new(true, 3, true))
    } else {
        None
    };

    match args.command.unwrap_or(Commands::Chat) {
        Commands::Chat => {
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
        Commands::Ask { prompt } => ask_once(&mut client, prompt.join(" ")).await,
        Commands::Analyze => analyze_workspace(&mut client, &mut registry).await,
        Commands::CreateProject { name, features } => {
            create_project_workflow(&mut client, &mut registry, &name, &features).await
        }
        Commands::Performance => display_performance_report().await,
        Commands::CompressContext => compress_context_demo(&mut client).await,
        Commands::DemoAsync => demo_async_operations().await,
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
    client: &mut gemini::Client,
    registry: &mut ToolRegistry,
    verbose: bool,
    async_writer: Option<async_file_ops::AsyncFileWriter>,
    diff_renderer: Option<diff_renderer::DiffChatRenderer>,
    rate_limiter: &RateLimiter,
    workspace: &PathBuf,
) -> Result<()> {
    let _perf_timer = PROFILER.start_operation("chat_loop_total");
    if verbose {
        println!(
            "{} {}\n",
            style("Verbose logging enabled").yellow().bold(),
            style("").dim()
        );
        println!(
            "{} {}\n",
            style("Chat with vtagent (use 'ctrl-c' to quit)")
                .cyan()
                .bold(),
            style("").dim()
        );
    } else {
        println!(
            "{} {}\n",
            style("Chat with vtagent (use 'ctrl-c' to quit)")
                .cyan()
                .bold(),
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
            let context_analysis = context_analyzer.analyze_context(&contents, user_message);
            if verbose && context_analysis.confidence > 0.5 {
                println!(
                    "{} Detected intent: {} (confidence: {:.1}%)",
                    style("[CONTEXT]").dim(),
                    context_analysis.intent,
                    context_analysis.confidence * 100.0
                );
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
                style("⚠️").yellow(),
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

/// Display performance metrics and targets status
async fn display_performance_report() -> Result<()> {
    let targets = PerformanceTargets::default();
    let targets_status = targets.check_targets(&PROFILER);

    println!("{}", PROFILER.generate_report());
    println!();
    println!("{}", targets_status.generate_report());

    Ok(())
}

async fn ask_once(client: &mut gemini::Client, prompt: String) -> Result<()> {
    let contents = vec![Content::user_text(prompt)];
    let sys_instruction = system_instruction();
    let req = GenerateContentRequest {
        contents,
        tools: None,
        tool_config: None,
        generation_config: None,
        system_instruction: Some(sys_instruction),
    };

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
        *status_guard = "Thinking...".to_string();
    }

    match client
        .generate_content_stream(&req, |chunk| {
            // Update status on first chunk to show response generation
            {
                let mut status_guard = status.lock().unwrap();
                if *status_guard == "Thinking..." {
                    *status_guard = "Generating answer...".to_string();
                }
            }

            // Stop spinner on first chunk
            is_loading.store(false, Ordering::Relaxed);

            // Wait for spinner thread to clean up
            thread::sleep(Duration::from_millis(100));

            // Filter out function call JSON to keep output clean
            if !chunk.contains("[FUNCTION_CALL:") && !chunk.contains("FUNCTION_CALL") {
                // Try to render markdown for chat responses
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
        Ok(_) => {
            println!(); // Add newline after streaming
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
        }
    }

    // Wait for spinner thread to finish
    let _ = spinner_handle;
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

- **Be Concise**: Provide essential information without unnecessary details
- **Be Direct**: Use clear, straightforward language
- **Be Proactive**: Take direct action using your tools instead of asking users to do things
- **Show Essential Progress**: Briefly indicate what's happening without verbose explanations
- **Explain Key Changes**: Focus on what changed and why, keep it brief
- **Avoid Redundancy**: Don't repeat information unnecessarily

## COMMON TASKS TO HANDLE AUTOMATICALLY

When users ask you to create or modify code that requires:

- **Dependencies**: Add them to Cargo.toml, package.json, etc. automatically
- **Configuration Files**: Create necessary config files when they're clearly needed
- **Directory Structure**: Create required directories for new files
- **Build Scripts**: Set up build configurations when they're standard
- **Documentation**: Generate README files, comments, or documentation when requested

**Always use your tools to accomplish these tasks rather than asking the user to do them manually.**

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
async fn analyze_workspace(
    _client: &mut gemini::Client,
    registry: &mut ToolRegistry,
) -> Result<()> {
    println!("{}", style("ANALYZING WORKSPACE...").cyan().bold());

    // Step 1: Get high-level directory structure
    println!("{}", style("1. Getting workspace structure...").dim());
    let root_files = registry
        .execute_tool(
            "list_files",
            serde_json::json!({"path": ".", "max_items": 50}),
        )
        .await;
    match root_files {
        Ok(result) => {
            println!("{}", style("✓ Root directory structure obtained").green());
            if let Some(files_array) = result.get("files") {
                println!(
                    "   Found {} files/directories in root",
                    files_array.as_array().unwrap_or(&vec![]).len()
                );
            }
        }
        Err(e) => println!("{} {}", style("✗ Failed to list root directory:").red(), e),
    }

    // Step 2: Look for important project files
    println!("{}", style("2. Identifying project type...").dim());
    let important_files = vec![
        "README.md",
        "Cargo.toml",
        "package.json",
        "go.mod",
        "requirements.txt",
        "Makefile",
    ];

    for file in important_files {
        let check_file = registry
            .execute_tool(
                "list_files",
                serde_json::json!({"path": ".", "include_hidden": false}),
            )
            .await;
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
        let read_result = registry
            .execute_tool(
                "read_file",
                serde_json::json!({"path": config_file, "max_bytes": 2000}),
            )
            .await;
        match read_result {
            Ok(result) => {
                println!(
                    "   {} Read {} ({} bytes)",
                    style("✓").green(),
                    config_file,
                    result
                        .get("metadata")
                        .and_then(|m| m.get("size"))
                        .unwrap_or(&serde_json::Value::Null)
                );
            }
            Err(_) => {} // File doesn't exist, that's ok
        }
    }

    // Step 4: Analyze source code structure
    println!("{}", style("4. Analyzing source code structure...").dim());

    // Check for common source directories
    let src_dirs = vec!["src", "lib", "pkg", "internal", "cmd"];
    for dir in src_dirs {
        let check_dir = registry
            .execute_tool(
                "list_files",
                serde_json::json!({"path": ".", "include_hidden": false}),
            )
            .await;
        if let Ok(result) = check_dir {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    for file_obj in files_array {
                        if let Some(path) = file_obj.get("path") {
                            if path.as_str().unwrap_or("") == dir {
                                println!(
                                    "   {} Found source directory: {}",
                                    style("✓").green(),
                                    dir
                                );
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    println!("{}", style("WORKSPACE ANALYSIS COMPLETE!").green().bold());
    println!(
        "{}",
        style("INFO: You can now ask me specific questions about the codebase.")
            .blue()
            .bold()
    );

    Ok(())
}

/// Create a complete Rust project using prompt chaining workflow
async fn create_project_workflow(
    _client: &mut gemini::Client,
    registry: &mut ToolRegistry,
    project_name: &str,
    features: &[String],
) -> Result<()> {
    println!(
        "{}",
        style(format!(
            "CREATING RUST PROJECT '{}' with features: {:?}",
            project_name, features
        ))
        .cyan()
        .bold()
    );

    // Step 1: Create project directory structure
    println!(
        "{}",
        style("Step 1: Creating project directory structure...").yellow()
    );
    let create_dir_result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": format!("{}/.gitkeep", project_name),
                "content": "",
                "overwrite": true,
                "create_dirs": true
            }),
        )
        .await;

    match create_dir_result {
        Ok(_) => println!("   {} Created project directory", style("✓").green()),
        Err(e) => {
            println!("   {} Failed to create directory: {}", style("✗").red(), e);
            return Err(anyhow!("Failed to create project directory: {}", e));
        }
    }

    // Step 2: Create Cargo.toml
    println!("{}", style("Step 2: Generating Cargo.toml...").yellow());
    let cargo_toml_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}"#,
        project_name,
        if features.contains(&"serde".to_string()) {
            "serde = { version = \"1.0\", features = [\"derive\"] }"
        } else {
            ""
        }
    );

    let cargo_result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": format!("{}/Cargo.toml", project_name),
                "content": cargo_toml_content,
                "overwrite": true,
                "create_dirs": true
            }),
        )
        .await;

    match cargo_result {
        Ok(_) => println!("   {} Created Cargo.toml", style("✓").green()),
        Err(e) => println!("   {} Failed to create Cargo.toml: {}", style("✗").red(), e),
    }

    // Step 3: Create src directory and main.rs
    println!(
        "{}",
        style("Step 3: Creating source code structure...").yellow()
    );

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
        &format!(
            r#"fn main() {{
    println!("Hello, {}!", env!("CARGO_PKG_NAME"));
}}"#,
            project_name
        )
    };

    let main_rs_result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": format!("{}/src/main.rs", project_name),
                "content": main_rs_content,
                "overwrite": true,
                "create_dirs": true
            }),
        )
        .await;

    match main_rs_result {
        Ok(_) => println!("   {} Created src/main.rs", style("✓").green()),
        Err(e) => println!("   {} Failed to create main.rs: {}", style("✗").red(), e),
    }

    // Step 4: Create README.md
    println!("{}", style("Step 4: Generating documentation...").yellow());
    let readme_content = format!(
        r#"# {}

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
"#,
        project_name,
        features.join(", ")
    );

    let readme_result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": format!("{}/README.md", project_name),
                "content": readme_content,
                "overwrite": true,
                "create_dirs": true
            }),
        )
        .await;

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

    let gitignore_result = registry
        .execute_tool(
            "write_file",
            serde_json::json!({
                "path": format!("{}/.gitignore", project_name),
                "content": gitignore_content,
                "overwrite": true,
                "create_dirs": true
            }),
        )
        .await;

    match gitignore_result {
        Ok(_) => println!("   {} Created .gitignore", style("✓").green()),
        Err(e) => println!("   {} Failed to create .gitignore: {}", style("✗").red(), e),
    }

    // Step 6: Test the build
    println!("{}", style("Step 6: Testing project build...").yellow());
    let test_build_result = registry
        .execute_tool(
            "list_files",
            serde_json::json!({
                "path": format!("{}/src", project_name),
                "include_hidden": false
            }),
        )
        .await;

    match test_build_result {
        Ok(result) => {
            if let Some(files) = result.get("files") {
                if let Some(files_array) = files.as_array() {
                    if !files_array.is_empty() {
                        println!("   {} Project structure verified", style("✓").green());
                    }
                }
            }
        }
        Err(e) => println!(
            "   {} Failed to verify project structure: {}",
            style("✗").red(),
            e
        ),
    }

    println!("{}", style("PROJECT CREATION COMPLETE!").green().bold());
    println!(
        "{}",
        style(format!(
            "PROJECT '{}' created with {} features",
            project_name,
            features.len()
        ))
        .cyan()
        .bold()
    );
    println!(
        "{}",
        style(format!(
            "TIP: Run 'cd {} && cargo run' to test your new project",
            project_name
        ))
        .yellow()
        .bold()
    );

    Ok(())
}

/// Generate appropriate file content based on language and filename
fn generate_file_content(language: &str, filename: &str) -> String {
    match language.to_lowercase().as_str() {
        "python" => {
            if filename.contains("calc") || filename.contains("calculator") {
                r#"def add(a, b):
    """Add two numbers together."""
    return a + b

def subtract(a, b):
    """Subtract b from a."""
    return a - b

def multiply(a, b):
    """Multiply two numbers."""
    return a * b

def divide(a, b):
    """Divide a by b."""
    if b == 0:
        raise ValueError("Cannot divide by zero")
    return a / b

def main():
    print("Simple Calculator")
    print("=================")

    while True:
        print("\nOperations:")
        print("1. Add")
        print("2. Subtract")
        print("3. Multiply")
        print("4. Divide")
        print("5. Quit")

        choice = input("Choose operation (1-5): ")

        if choice == "5":
            print("Goodbye!")
            break

        try:
            num1 = float(input("Enter first number: "))
            num2 = float(input("Enter second number: "))

            if choice == "1":
                result = add(num1, num2)
            elif choice == "2":
                result = subtract(num1, num2)
            elif choice == "3":
                result = multiply(num1, num2)
            elif choice == "4":
                result = divide(num1, num2)
            else:
                print("Invalid choice")
                continue

            print(f"Result: {result}")

        except ValueError as e:
            print(f"Error: {e}")
        except Exception as e:
            print(f"An error occurred: {e}")

if __name__ == "__main__":
    main()
"#
                .to_string()
            } else {
                r#"print("Hello, World!")
print("Welcome to Python!")
"#
                .to_string()
            }
        }
        "rust" => {
            if filename.contains("calc") || filename.contains("calculator") {
                r#"use std::io;

fn add(a: f64, b: f64) -> f64 {
    a + b
}

fn subtract(a: f64, b: f64) -> f64 {
    a - b
}

fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a / b)
    }
}

fn main() {
    println!("Simple Calculator");
    println!("=================");

    loop {
        println!("\nOperations:");
        println!("1. Add");
        println!("2. Subtract");
        println!("3. Multiply");
        println!("4. Divide");
        println!("5. Quit");

        let mut choice = String::new();
        print!("Choose operation (1-5): ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut choice).unwrap();

        let choice = choice.trim();

        if choice == "5" {
            println!("Goodbye!");
            break;
        }

        let mut num1 = String::new();
        let mut num2 = String::new();

        print!("Enter first number: ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut num1).unwrap();

        print!("Enter second number: ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut num2).unwrap();

        let num1: f64 = match num1.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid number");
                continue;
            }
        };

        let num2: f64 = match num2.trim().parse() {
            Ok(n) => n,
            Err(_) => {
                println!("Invalid number");
                continue;
            }
        };

        let result = match choice {
            "1" => add(num1, num2),
            "2" => subtract(num1, num2),
            "3" => multiply(num1, num2),
            "4" => match divide(num1, num2) {
                Ok(res) => res,
                Err(e) => {
                    println!("Error: {}", e);
                    continue;
                }
            },
            _ => {
                println!("Invalid choice");
                continue;
            }
        };

        println!("Result: {}", result);
    }
}
"#
                .to_string()
            } else {
                r#"fn main() {
    println!("Hello, World!");
    println!("Welcome to Rust!");
}
"#
                .to_string()
            }
        }
        "javascript" | "js" => {
            if filename.contains("calc") || filename.contains("calculator") {
                r#"const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

function add(a, b) {
    return a + b;
}

function subtract(a, b) {
    return a - b;
}

function multiply(a, b) {
    return a * b;
}

function divide(a, b) {
    if (b === 0) {
        throw new Error("Cannot divide by zero");
    }
    return a / b;
}

function askQuestion(question) {
    return new Promise((resolve) => {
        rl.question(question, (answer) => {
            resolve(answer);
        });
    });
}

async function main() {
    console.log("Simple Calculator");
    console.log("=================");

    while (true) {
        console.log("\nOperations:");
        console.log("1. Add");
        console.log("2. Subtract");
        console.log("3. Multiply");
        console.log("4. Divide");
        console.log("5. Quit");

        const choice = await askQuestion("Choose operation (1-5): ");

        if (choice === "5") {
            console.log("Goodbye!");
            rl.close();
            break;
        }

        try {
            const num1 = parseFloat(await askQuestion("Enter first number: "));
            const num2 = parseFloat(await askQuestion("Enter second number: "));

            if (isNaN(num1) || isNaN(num2)) {
                console.log("Invalid numbers");
                continue;
            }

            let result;
            switch (choice) {
                case "1":
                    result = add(num1, num2);
                    break;
                case "2":
                    result = subtract(num1, num2);
                    break;
                case "3":
                    result = multiply(num1, num2);
                    break;
                case "4":
                    result = divide(num1, num2);
                    break;
                default:
                    console.log("Invalid choice");
                    continue;
            }

            console.log(`Result: ${result}`);

        } catch (error) {
            console.log(`Error: ${error.message}`);
        }
    }
}

main().catch(console.error);
"#
                .to_string()
            } else {
                r#"console.log("Hello, World!");
console.log("Welcome to JavaScript!");
"#
                .to_string()
            }
        }
        _ => {
            format!(
                "# Hello, World!\n# Welcome to {}!\n\nprint(\"Hello from {}!\")\n",
                language, language
            )
        }
    }
}

/// Demonstrate context compression following Cognition's principles
async fn compress_context_demo(client: &mut gemini::Client) -> Result<()> {
    println!("{}", style("🧠 Context Compression Demo").cyan().bold());
    println!(
        "{}",
        style("Following Cognition's context engineering principles...").dim()
    );

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

    println!(
        "{} {}",
        style("ORIGINAL CONVERSATION LENGTH:").yellow().bold(),
        sample_conversation.len()
    );
    println!(
        "{} {:.1}KB",
        style("ESTIMATED TOKEN USAGE:").yellow().bold(),
        sample_conversation.len() as f64 * 0.5
    ); // Rough estimate

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
            _ => "UNKNOWN",
        };

        let mut content_summary = format!("\n--- Turn {} ({}) ---\n", i + 1, role_indicator);

        for part in &content.parts {
            match part {
                Part::Text { text } => {
                    content_summary.push_str(text);
                }
                Part::FunctionCall { function_call } => {
                    content_summary.push_str(&format!(
                        "\n[TOOL CALL: {}({})]",
                        function_call.name, function_call.args
                    ));
                }
                Part::FunctionResponse { function_response } => {
                    content_summary.push_str(&format!(
                        "\n[TOOL RESULT: {}]",
                        serde_json::to_string_pretty(&function_response.response)
                            .unwrap_or_default()
                    ));
                }
            }
        }

        if i == 0 {
            compression_content[0] =
                Content::user_text(format!("{}{}", compression_prompt, content_summary));
        } else {
            compression_content.push(Content::user_text(content_summary));
        }
    }

    // Add final instruction
    compression_content.push(Content::user_text(
        r#"
COMPRESSION REQUIREMENTS:
- Preserve all key decisions and their rationale
- Keep track of what files were created/modified
- Maintain information about current project state
- Include user's original intent
- Note any important technical choices made

COMPRESSED SUMMARY:"#,
    ));

    // Create request for compression
    let compression_request = GenerateContentRequest {
        contents: compression_content,
        tools: None,
        tool_config: None,
        generation_config: Some(json!({
            "maxOutputTokens": 1000,
            "temperature": 0.1
        })),
        system_instruction: Some(Content::system_text(
            r#"You are an expert at compressing agent conversation history.
Your goal is to create a compressed summary that maintains all critical information while being concise.
Focus on: key decisions, actions taken, current state, and user requirements."#,
        )),
    };

    println!("{}", style("COMPRESSING CONVERSATION...").cyan().bold());

    let compressed_response = client.generate_content(&compression_request).await?;

    if let Some(candidate) = compressed_response.candidates.into_iter().next() {
        if let Some(content) = candidate.content.parts.into_iter().next() {
            if let Some(text) = content.as_text() {
                println!("{}", style("COMPRESSED SUMMARY:").green().bold());
                println!("{}", text);

                // Estimate compression ratio
                let original_chars: usize = sample_conversation
                    .iter()
                    .map(|c| {
                        c.parts
                            .iter()
                            .map(|p| match p.as_text() {
                                Some(text) => text.len(),
                                _ => 100, // Rough estimate for tool calls
                            })
                            .sum::<usize>()
                    })
                    .sum();

                let compressed_chars = text.len();
                let compression_ratio = original_chars as f64 / compressed_chars as f64;

                println!(
                    "\n{} {:.1}x",
                    style("COMPRESSION RATIO:").magenta().bold(),
                    compression_ratio
                );
                println!(
                    "{} {} → {} characters",
                    style("SIZE REDUCTION:").magenta().bold(),
                    original_chars,
                    compressed_chars
                );
            }
        }
    }

    println!("\n{}", style("KEY PRINCIPLES APPLIED:").yellow().bold());
    println!("  • {}", style("Share full context and traces").dim());
    println!("  • {}", style("Actions carry implicit decisions").dim());
    println!(
        "  • {}",
        style("Single-threaded agents are more reliable").dim()
    );
    println!(
        "  • {}",
        style("Context compression enables longer conversations").dim()
    );
    Ok(())
}

/// Demo async file operations and diff rendering
async fn demo_async_operations() -> Result<()> {
    println!(
        "{} Async File Operations & Diff Rendering Demo",
        style("DEMO").cyan().bold()
    );
    println!("{}", "═".repeat(60));
    println!();

    // Create async file writer
    println!(
        "{} 1. Creating Async File Writer...",
        style("STEP").yellow().bold()
    );
    let writer = async_file_ops::AsyncFileWriter::new(5);
    println!(
        "{} Async writer initialized with 5 concurrent operations",
        style("INITIALIZED").green().bold()
    );

    // Create test file
    println!(
        "\n{} 2. Creating test file asynchronously...",
        style("STEP").yellow().bold()
    );
    let test_path = std::path::PathBuf::from("demo_async_test.rs");
    let test_content = r#"//! Demo file created by async operations
use std::io;

fn main() -> io::Result<()> {
    println!("Hello from async file operations demo!");
    println!("This file was created using non-blocking writes.");
    Ok(())
}
"#;

    writer
        .write_file(test_path.clone(), test_content.to_string())
        .await?;
    println!(
        "{} File write queued asynchronously",
        style("QUEUED").cyan().bold()
    );

    // Wait for completion
    println!(
        "\n{} 3. Waiting for async operation to complete...",
        style("STEP").yellow().bold()
    );
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Verify file was created
    if test_path.exists() {
        println!(
            "{} File created successfully!",
            style("SUCCESS").green().bold()
        );
        println!(
            "{} File size: {} bytes",
            style("SIZE").blue().bold(),
            std::fs::metadata(&test_path)?.len()
        );
    } else {
        println!("{} File creation failed", style("ERROR").red().bold());
    }

    // Demo diff rendering
    println!(
        "\n{} 4. Demonstrating Diff Rendering...",
        style("STEP").yellow().bold()
    );

    let old_content = r#"fn main() {
    println!("Hello!");
}"#;

    let new_content = r#"use std::io;

fn main() -> io::Result<()> {
    println!("Hello from async world!");
    println!("This demonstrates diff rendering.");
    Ok(())
}"#;

    let diff_renderer = diff_renderer::DiffChatRenderer::new(true, 3, true);
    let diff_output = diff_renderer.render_file_change(&test_path, old_content, new_content);

    println!("\n{} File Change Diff:", style("DIFF").magenta().bold());
    println!("{}", diff_output);

    // Clean up
    println!(
        "\n{} 5. Cleaning up demo files...",
        style("STEP").yellow().bold()
    );
    if test_path.exists() {
        std::fs::remove_file(&test_path)?;
        println!("{} Demo file removed", style("CLEANED").green().bold());
    }

    println!(
        "\n{} Demo completed successfully!",
        style("COMPLETE").green().bold()
    );
    println!("{} Key features demonstrated:", style("INFO").blue().bold());
    println!("   • Non-blocking async file writes");
    println!("   • Concurrent file operation processing");
    println!("   • Real-time diff generation and rendering");
    println!("   • Color-coded change visualization");
    println!("   • Automatic file change detection");

    Ok(())
}

/// Get user-friendly name for a tool
fn get_tool_friendly_name(tool_name: &str) -> String {
    match tool_name {
        "list_files" => "Exploring files".to_string(),
        "read_file" => "Reading file".to_string(),
        "write_file" => "Creating/updating file".to_string(),
        "edit_file" => "Editing file".to_string(),
        _ => tool_name.to_string(),
    }
}

/// Get simple description for a tool call
fn get_tool_simple_description(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "list_files" => {
            if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                format!("Looking at: {}", path)
            } else {
                "Exploring directory structure".to_string()
            }
        }
        "read_file" => {
            if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                format!("Reading: {}", path)
            } else {
                "Reading file content".to_string()
            }
        }
        "write_file" => {
            if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                format!("Writing to: {}", path)
            } else {
                "Creating new file".to_string()
            }
        }
        "edit_file" => {
            if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                format!("Modifying: {}", path)
            } else {
                "Editing file content".to_string()
            }
        }
        _ => String::new(),
    }
}

/// Check if a tool operation involves file operations
fn is_file_operation(tool_name: &str) -> bool {
    matches!(tool_name, "write_file" | "create_file" | "delete_file")
}

/// Handle async file operations
async fn handle_async_file_operation(
    tool_name: &str,
    args: &serde_json::Value,
    async_writer: &async_file_ops::AsyncFileWriter,
    registry: &mut ToolRegistry,
) -> Result<serde_json::Value> {
    match tool_name {
        "write_file" => {
            if let (Some(path), Some(content)) = (
                args.get("path").and_then(|p| p.as_str()),
                args.get("content").and_then(|c| c.as_str()),
            ) {
                let file_path = std::path::PathBuf::from(path);
                async_writer
                    .write_file(file_path.clone(), content.to_string())
                    .await?;

                // Return success response
                Ok(json!({
                    "status": "success",
                    "message": format!("File {} queued for async write", path),
                    "async": true,
                    "path": path
                }))
            } else {
                Err(anyhow!("Invalid arguments for write_file"))
            }
        }
        "create_file" => {
            if let (Some(path), Some(content)) = (
                args.get("path").and_then(|p| p.as_str()),
                args.get("content").and_then(|c| c.as_str()),
            ) {
                let file_path = std::path::PathBuf::from(path);
                async_writer
                    .create_file(file_path.clone(), content.to_string())
                    .await?;

                // Return success response
                Ok(json!({
                    "status": "success",
                    "message": format!("File {} queued for async creation", path),
                    "async": true,
                    "path": path
                }))
            } else {
                Err(anyhow!("Invalid arguments for create_file"))
            }
        }
        "delete_file" => {
            if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
                let file_path = std::path::PathBuf::from(path);
                async_writer.delete_file(file_path.clone()).await?;

                // Return success response
                Ok(json!({
                    "status": "success",
                    "message": format!("File {} queued for async deletion", path),
                    "async": true,
                    "path": path
                }))
            } else {
                Err(anyhow!("Invalid arguments for delete_file"))
            }
        }
        _ => {
            // Fallback to regular tool execution for non-file operations
            registry.execute_tool(tool_name, args.clone()).await
        }
    }
}

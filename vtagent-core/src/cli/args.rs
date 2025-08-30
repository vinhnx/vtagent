//! CLI argument parsing and configuration

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI structure for vtagent with advanced features
#[derive(Parser, Debug)]
#[command(
    name = "vtagent",
    version,
    about = "**Research-preview Rust coding agent** powered by Gemini with Anthropic-inspired architecture\n\n**Features:**\n• Interactive AI coding assistant with Research-preview tool-calling\n• Multi-language support (Rust, Python, JavaScript, TypeScript, Go, Java)\n• Real-time diff rendering and async file operations\n• Rate limiting and tool call management\n• Markdown rendering for chat responses\n\n**Quick Start:**\n  export GEMINI_API_KEY=\"your_key\"\n  vtagent chat"
)]
pub struct Cli {
    /// **Gemini model ID** (e.g., `gemini-2.5-flash-lite`, `gemini-2.5-flash`, `gemini-pro`)\n\n**Available models:**\n• `gemini-2.5-flash-lite` - Fastest, most cost-effective\n• `gemini-2.5-flash` - Fast, cost-effective\n• `gemini-pro` - More capable, slower\n• `gemini-2.5-pro` - Latest, most Research-preview
    #[arg(long, global = true, default_value = "gemini-2.5-flash-lite")]
    pub model: String,

    /// **API key environment variable** to read\n\n**Checks in order:**\n1. Specified env var\n2. `GOOGLE_API_KEY`\n\n**Setup:** `export GEMINI_API_KEY="your_key"`
    #[arg(long, global = true, default_value = "GEMINI_API_KEY")]
    pub api_key_env: String,

    /// **Workspace root directory** for file operations\n\n**Defaults to:** Current directory\n**All file operations** are restricted to this path
    #[arg(long, global = true)]
    pub workspace: Option<PathBuf>,

    /// **Enable async file operations** for non-blocking writes\n\n**Benefits:**\n• Non-blocking file I/O\n• Better performance\n• Concurrent operations\n• Real-time feedback
    #[arg(long, global = true)]
    pub async_file_ops: bool,

    /// **Show diffs for file changes** in chat interface\n\n**Features:**\n• Real-time diff rendering\n• Syntax highlighting\n• Line-by-line changes\n• Before/after comparison
    #[arg(long, global = true)]
    pub show_file_diffs: bool,

    /// **Maximum concurrent async file operations**\n\n**Default:** 5\n**Higher values:** Better performance but more resource usage
    #[arg(long, global = true, default_value_t = 5)]
    pub max_concurrent_ops: usize,

    /// **Maximum API requests per minute** to prevent rate limiting\n\n**Default:** 30\n**Lower values:** More conservative, fewer errors\n**Higher values:** Better performance, risk of rate limits
    #[arg(long, global = true, default_value_t = 30)]
    pub api_rate_limit: usize,

    /// **Maximum tool calls per chat run** to prevent runaway execution\n\n**Default:** 10\n**Purpose:** Prevents infinite loops and excessive API usage
    #[arg(long, global = true, default_value_t = 10)]
    pub max_tool_calls: usize,

    /// Enable verbose logging and transparency features
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long, global = true, default_value = "info")]
    pub log_level: String,

    /// Disable color output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available commands with comprehensive features
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// **Interactive AI coding assistant** with Research-preview tool-calling capabilities\n\n**Features:**\n• Real-time code generation and editing\n• Multi-language support\n• File system operations\n• Async processing\n\n**Usage:** vtagent chat
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

    /// **Revert agent to a previous snapshot**\n\n**Features:**\n• Revert to any previous turn\n• Partial reverts (memory, context, full)\n• Safe rollback with validation\n\n**Examples:**\n  vtagent revert --turn 5\n  vtagent revert --turn 3 --partial memory
    Revert {
        /// **Turn number to revert to**\n\n**Required:** Yes\n**Example:** 5
        #[arg(short, long)]
        turn: usize,

        /// **Scope of revert operation**\n\n**Options:** memory, context, full\n**Default:** full\n**Examples:**\n  --partial memory (revert conversation only)\n  --partial context (revert decisions/errors only)
        #[arg(short, long)]
        partial: Option<String>,
    },

    /// **List all available snapshots**\n\n**Shows:**\n• Snapshot ID and turn number\n• Creation timestamp\n• Description\n• File size and compression status\n\n**Usage:** vtagent snapshots
    Snapshots,

    /// **Clean up old snapshots**\n\n**Features:**\n• Remove snapshots beyond limit\n• Configurable retention policy\n• Safe deletion with confirmation\n\n**Examples:**\n  vtagent cleanup-snapshots\n  vtagent cleanup-snapshots --max 20
    #[command(name = "cleanup-snapshots")]
    CleanupSnapshots {
        /// **Maximum number of snapshots to keep**\n\n**Default:** 50\n**Example:** --max 20
        #[arg(short, long, default_value_t = 50)]
        max: usize,
    },

    /// **Initialize project with AGENTS.md** - analyzes current project and generates AGENTS.md file\n\n**Features:**\n• Auto-detect project languages and frameworks\n• Analyze dependencies and build systems\n• Generate comprehensive agent guidelines\n• Create project-specific conventions\n\n**Usage:** vtagent init
    Init,
}

/// Configuration file structure
#[derive(Debug)]
pub struct ConfigFile {
    pub model: Option<String>,
    pub api_key_env: Option<String>,
    pub verbose: Option<bool>,
    pub log_level: Option<String>,
    pub workspace: Option<PathBuf>,
    pub tools: Option<ToolConfig>,
    pub context: Option<ContextConfig>,
    pub logging: Option<LoggingConfig>,
}

/// Tool configuration from config file
#[derive(Debug, serde::Deserialize)]
pub struct ToolConfig {
    pub enable_validation: Option<bool>,
    pub max_execution_time_seconds: Option<u64>,
    pub allow_file_creation: Option<bool>,
    pub allow_file_deletion: Option<bool>,
}

/// Context management configuration
#[derive(Debug, serde::Deserialize)]
pub struct ContextConfig {
    pub max_context_length: Option<usize>,
    pub compression_threshold: Option<usize>,
    pub summarization_interval: Option<usize>,
}

/// Logging configuration
#[derive(Debug, serde::Deserialize)]
pub struct LoggingConfig {
    pub file_logging: Option<bool>,
    pub log_directory: Option<String>,
    pub max_log_files: Option<usize>,
    pub max_log_size_mb: Option<usize>,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            model: "gemini-2.5-flash-lite".to_string(),
            api_key_env: "GEMINI_API_KEY".to_string(),
            workspace: None,
            async_file_ops: false,
            show_file_diffs: false,
            max_concurrent_ops: 5,
            api_rate_limit: 30,
            max_tool_calls: 10,
            verbose: false,
            config: None,
            log_level: "info".to_string(),
            no_color: false,
            command: Some(Commands::Chat),
        }
    }
}

impl Cli {
    /// Load configuration from a simple TOML-like file without external deps
    ///
    /// Supported keys (top-level): model, api_key_env, verbose, log_level, workspace
    /// Example:
    ///   model = "gemini-2.5-flash-lite"
    ///   api_key_env = "GEMINI_API_KEY"
    ///   verbose = true
    ///   log_level = "info"
    ///   workspace = "/path/to/workspace"
    pub fn load_config(&self) -> Result<ConfigFile, Box<dyn std::error::Error>> {
        use std::fs;
        use std::path::Path;

        // Resolve candidate path
        let path = if let Some(p) = &self.config {
            p.clone()
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let primary = cwd.join("vtagent.toml");
            let secondary = cwd.join(".vtagent.toml");
            if primary.exists() {
                primary
            } else if secondary.exists() {
                secondary
            } else {
                // No config file; return empty config
                return Ok(ConfigFile {
                    model: None,
                    api_key_env: None,
                    verbose: None,
                    log_level: None,
                    workspace: None,
                    tools: None,
                    context: None,
                    logging: None,
                });
            }
        };

        let text = fs::read_to_string(&path)?;

        // Very small parser: key = value, supports quoted strings, booleans, and plain paths
        let mut cfg = ConfigFile {
            model: None,
            api_key_env: None,
            verbose: None,
            log_level: None,
            workspace: None,
            tools: None,
            context: None,
            logging: None,
        };

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            // Strip inline comments after '#'
            let line = match line.find('#') {
                Some(idx) => &line[..idx],
                None => line,
            }
            .trim();

            // Expect key = value
            let mut parts = line.splitn(2, '=');
            let key = parts.next().map(|s| s.trim()).unwrap_or("");
            let val = parts.next().map(|s| s.trim()).unwrap_or("");
            if key.is_empty() || val.is_empty() {
                continue;
            }

            // Remove surrounding quotes if present
            let unquote = |s: &str| -> String {
                let s = s.trim();
                if (s.starts_with('"') && s.ends_with('"'))
                    || (s.starts_with('\'') && s.ends_with('\''))
                {
                    s[1..s.len() - 1].to_string()
                } else {
                    s.to_string()
                }
            };

            match key {
                "model" => cfg.model = Some(unquote(val)),
                "api_key_env" => cfg.api_key_env = Some(unquote(val)),
                "verbose" => {
                    let v = unquote(val).to_lowercase();
                    cfg.verbose = Some(matches!(v.as_str(), "true" | "1" | "yes"));
                }
                "log_level" => cfg.log_level = Some(unquote(val)),
                "workspace" => {
                    let v = unquote(val);
                    let p = if Path::new(&v).is_absolute() {
                        PathBuf::from(v)
                    } else {
                        // Resolve relative to config file directory
                        let base = path.parent().unwrap_or(Path::new("."));
                        base.join(v)
                    };
                    cfg.workspace = Some(p);
                }
                _ => {
                    // Ignore unknown keys in this minimal parser
                }
            }
        }

        Ok(cfg)
    }

    /// Get the effective workspace path
    pub fn get_workspace(&self) -> std::path::PathBuf {
        self.workspace
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Get the effective API key environment variable
    pub fn get_api_key_env(&self) -> String {
        self.api_key_env.clone()
    }

    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Get the effective model name
    pub fn get_model(&self) -> String {
        self.model.clone()
    }
}

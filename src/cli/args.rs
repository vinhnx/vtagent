//! CLI argument parsing and configuration

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Main CLI structure for vtagent
#[derive(Parser, Debug)]
#[command(
    name = "vtagent",
    version,
    about = "Advanced Rust coding agent powered by Gemini with Anthropic-inspired architecture"
)]
pub struct Cli {
    /// Gemini model ID, e.g. gemini-2.5-flash-lite
    #[arg(long, global = true, default_value = "gemini-2.5-flash-lite")]
    pub model: String,

    /// API key environment variable to read (checks GEMINI_API_KEY, then GOOGLE_API_KEY)
    #[arg(long, global = true, default_value = "GEMINI_API_KEY")]
    pub api_key_env: String,

    /// Workspace root directory; defaults to current directory
    #[arg(long, global = true)]
    pub workspace: Option<PathBuf>,

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

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Interactive AI coding assistant with advanced tool-calling capabilities
    Chat {
        /// Initial prompt to start the conversation
        #[arg(long)]
        prompt: Option<String>,
    },

    /// Single prompt; prints model reply without tools
    Ask {
        /// The prompt to send to the model
        prompt: Vec<String>,
    },

    /// Interactive chat with enhanced transparency features
    ChatVerbose {
        /// Initial prompt to start the conversation
        #[arg(long)]
        prompt: Option<String>,
    },

    /// Analyze workspace structure and provide comprehensive overview
    Analyze {
        /// Analysis depth (basic, standard, deep)
        #[arg(long, default_value = "standard")]
        depth: String,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Create complete Rust project with specified features
    CreateProject {
        /// Project name
        name: String,

        /// Features to include (comma-separated)
        #[arg(short, long, default_value = "")]
        features: String,

        /// Project template to use
        #[arg(long)]
        template: Option<String>,

        /// Initialize git repository
        #[arg(long)]
        git: bool,
    },

    /// Compress conversation context (demonstrates context engineering)
    CompressContext {
        /// Context file to compress
        #[arg(long)]
        input: Option<PathBuf>,

        /// Output file for compressed context
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Show session statistics and performance metrics
    Stats {
        /// Show detailed breakdown
        #[arg(long)]
        detailed: bool,

        /// Output format (text, json)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Validate configuration and environment
    Validate {
        /// Check API connectivity
        #[arg(long)]
        api: bool,

        /// Check file system permissions
        #[arg(long)]
        filesystem: bool,

        /// Run all validation checks
        #[arg(long)]
        all: bool,
    },
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
            verbose: false,
            config: None,
            log_level: "info".to_string(),
            no_color: false,
            command: Some(Commands::Chat { prompt: None }),
        }
    }
}

impl Cli {
    /// Load configuration from file if specified (placeholder for future implementation)
    pub fn load_config(&self) -> Result<ConfigFile, Box<dyn std::error::Error>> {
        // For now, return default config
        // TODO: Implement TOML config file loading
        Ok(ConfigFile {
            model: None,
            api_key_env: None,
            verbose: None,
            log_level: None,
            workspace: None,
            tools: None,
            context: None,
            logging: None,
        })
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

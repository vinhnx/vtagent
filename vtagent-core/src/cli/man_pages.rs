//! Man page generation for VTAgent CLI using roff-rs
//!
//! This module provides functionality to generate Unix man pages for VTAgent
//! commands and subcommands using the roff-rs library.

use anyhow::{Context, Result, bail};
use roff::{Roff, bold, italic, roman};
use std::fs;
use std::path::Path;

/// Man page generator for VTAgent CLI
pub struct ManPageGenerator;

impl ManPageGenerator {
    /// Get current date in YYYY-MM-DD format
    fn current_date() -> String {
        use chrono::Utc;
        Utc::now().format("%Y-%m-%d").to_string()
    }

    /// Generate man page for the main VTAgent command
    pub fn generate_main_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control("TH", ["VTAGENT", "1", &current_date, "VTAgent", "User Commands"])
            .control("SH", ["NAME"])
            .text([roman("vtagent - Advanced coding agent with Decision Ledger")])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] ["),
                bold("COMMAND"),
                roman("] ["),
                bold("ARGS"),
                roman("]"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("VTAgent is an advanced coding agent with single-agent architecture and Decision Ledger that provides"),
                roman(" intelligent code generation, analysis, and modification capabilities. It supports"),
                roman(" multiple LLM providers and includes tree-sitter powered code analysis for"),
                roman(" Rust, Python, JavaScript, TypeScript, Go, and Java."),
            ])
            .control("SH", ["OPTIONS"])
            .control("TP", [])
            .text([bold("-m"), roman(", "), bold("--model"), roman(" "), italic("MODEL")])
            .text([roman("Specify the LLM model to use (default: gemini-2.5-flash-lite)")])
            .control("TP", [])
            .text([bold("-p"), roman(", "), bold("--provider"), roman(" "), italic("PROVIDER")])
            .text([roman("Specify the LLM provider (gemini, openai, anthropic, deepseek)")])
            .control("TP", [])
            .text([bold("--workspace"), roman(" "), italic("PATH")])
            .text([roman("Set the workspace root directory for file operations")])
            .control("TP", [])
            .text([bold("--enable-tree-sitter")])
            .text([roman("Enable tree-sitter code analysis")])
            .control("TP", [])
            .text([bold("--performance-monitoring")])
            .text([roman("Enable performance monitoring and metrics")])
            .control("TP", [])
            .text([bold("--debug")])
            .text([roman("Enable debug output")])
            .control("TP", [])
            .text([bold("--verbose")])
            .text([roman("Enable verbose logging")])
            .control("TP", [])
            .text([bold("-h"), roman(", "), bold("--help")])
            .text([roman("Display help information")])
            .control("TP", [])
            .text([bold("-V"), roman(", "), bold("--version")])
            .text([roman("Display version information")])
            .control("SH", ["COMMANDS"])
            .control("TP", [])
            .text([bold("chat")])
            .text([roman("Start interactive AI coding assistant")])
            .control("TP", [])
            .text([bold("ask"), roman(" "), italic("PROMPT")])
            .text([roman("Single prompt mode without tools")])
            .control("TP", [])
            .text([bold("analyze")])
            .text([roman("Analyze workspace with tree-sitter integration")])
            .control("TP", [])
            .text([bold("performance")])
            .text([roman("Display performance metrics and system status")])
            .control("TP", [])
            .text([bold("benchmark")])
            .text([roman("Run SWE-bench evaluation framework")])
            .control("TP", [])
            .text([bold("create-project"), roman(" "), italic("NAME"), roman(" "), italic("FEATURES")])
            .text([roman("Create complete Rust project with features")])
            .control("TP", [])
            .text([bold("init")])
            .text([roman("Initialize project with enhanced structure")])
            .control("TP", [])
            .text([bold("man"), roman(" "), italic("COMMAND")])
            .text([roman("Generate or display man pages for commands")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Start interactive chat:")])
            .text([bold("  vtagent chat")])
            .text([roman("Ask a question:")])
            .text([bold("  vtagent ask \"Explain Rust ownership\"")])
            .text([roman("Create a web project:")])
            .text([bold("  vtagent create-project myapp web,auth,db")])
            .text([roman("Generate man page:")])
            .text([bold("  vtagent man chat")])
            .control("SH", ["ENVIRONMENT"])
            .control("TP", [])
            .text([bold("GEMINI_API_KEY")])
            .text([roman("API key for Google Gemini (default provider)")])
            .control("TP", [])
            .text([bold("OPENAI_API_KEY")])
            .text([roman("API key for OpenAI GPT models")])
            .control("TP", [])
            .text([bold("ANTHROPIC_API_KEY")])
            .text([roman("API key for Anthropic Claude models")])
            .control("TP", [])
            .text([bold("DEEPSEEK_API_KEY")])
            .text([roman("API key for DeepSeek models")])
            .control("SH", ["FILES"])
            .control("TP", [])
            .text([bold("vtagent.toml")])
            .text([roman("Configuration file (current directory or ~/.vtagent/)")])
            .control("TP", [])
            .text([bold(".vtagent/")])
            .text([roman("Project cache and context directory")])
            .control("SH", ["SEE ALSO"])
            .text([roman("Full documentation: https://github.com/vinhnx/vtagent")])
            .text([roman("Related commands: cargo(1), rustc(1), git(1)")])
            .render();

        Ok(page)
    }

    /// Generate man page for a specific command
    pub fn generate_command_man_page(command: &str) -> Result<String> {
        match command {
            "chat" => Self::generate_chat_man_page(),
            "ask" => Self::generate_ask_man_page(),
            "analyze" => Self::generate_analyze_man_page(),
            "performance" => Self::generate_performance_man_page(),
            "benchmark" => Self::generate_benchmark_man_page(),
            "create-project" => Self::generate_create_project_man_page(),
            "init" => Self::generate_init_man_page(),
            "man" => Self::generate_man_man_page(),
            _ => bail!("Unknown command: {}", command),
        }
    }

    /// Generate man page for the chat command
    fn generate_chat_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control("TH", ["VTAGENT-CHAT", "1", &current_date, "VTAgent", "User Commands"])
            .control("SH", ["NAME"])
            .text([roman("vtagent-chat - Interactive AI coding assistant")])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("chat"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("Start an interactive AI coding assistant session."),
                roman(" The chat command provides intelligent code generation, analysis, and modification"),
                roman(" with support for multiple LLM providers and tree-sitter powered code analysis."),
            ])
            .control("SH", ["OPTIONS"])
            .text([roman("All global options are supported. See "), bold("vtagent(1)"), roman(" for details.")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Start basic chat session:")])
            .text([bold("  vtagent chat")])
            .text([roman("Start with specific model:")])
            .text([bold("  vtagent --model gemini-2.5-pro chat")])
            .control("SH", ["SEE ALSO"])
            .text([bold("vtagent(1)"), roman(", "), bold("vtagent-ask(1)"), roman(", "), bold("vtagent-analyze(1)")])
            .render();

        Ok(page)
    }

    /// Generate man page for the ask command
    fn generate_ask_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control("TH", ["VTAGENT-ASK", "1", &current_date, "VTAgent", "User Commands"])
            .control("SH", ["NAME"])
            .text([roman("vtagent-ask - Single prompt mode without tools")])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("ask"),
                roman(" "),
                italic("PROMPT"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("Execute a single prompt without tool usage. This is perfect for quick questions,"),
                roman(" code explanations, and simple queries that don't require file operations or"),
                roman(" complex tool interactions."),
            ])
            .control("SH", ["EXAMPLES"])
            .text([roman("Ask about Rust ownership:")])
            .text([bold("  vtagent ask \"Explain Rust ownership\"")])
            .text([roman("Get code explanation:")])
            .text([bold("  vtagent ask \"What does this regex do: \\w+@\\w+\\.\\w+\"")])
            .control("SH", ["SEE ALSO"])
            .text([bold("vtagent(1)"), roman(", "), bold("vtagent-chat(1)")])
            .render();

        Ok(page)
    }

    /// Generate man page for the analyze command
    fn generate_analyze_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control(
                "TH",
                [
                    "VTAGENT-ANALYZE",
                    "1",
                    &current_date,
                    "VTAgent",
                    "User Commands",
                ],
            )
            .control("SH", ["NAME"])
            .text([roman(
                "vtagent-analyze - Analyze workspace with tree-sitter integration",
            )])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("analyze"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman(
                    "Analyze the current workspace using tree-sitter integration. Provides project",
                ),
                roman(
                    " structure analysis, language detection, code complexity metrics, dependency",
                ),
                roman(" insights, and symbol extraction for supported languages."),
            ])
            .control("SH", ["SUPPORTED LANGUAGES"])
            .text([roman(
                "• Rust • Python • JavaScript • TypeScript • Go • Java",
            )])
            .control("SH", ["FEATURES"])
            .control("TP", [])
            .text([bold("Project Structure")])
            .text([roman("Directory tree and file organization analysis")])
            .control("TP", [])
            .text([bold("Language Detection")])
            .text([roman("Automatic detection of programming languages used")])
            .control("TP", [])
            .text([bold("Code Metrics")])
            .text([roman("Complexity analysis and code quality metrics")])
            .control("TP", [])
            .text([bold("Symbol Extraction")])
            .text([roman("Functions, classes, and other code symbols")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Analyze current workspace:")])
            .text([bold("  vtagent analyze")])
            .control("SH", ["SEE ALSO"])
            .text([bold("vtagent(1)"), roman(", "), bold("vtagent-chat(1)")])
            .render();

        Ok(page)
    }

    /// Generate man page for the performance command
    fn generate_performance_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control(
                "TH",
                [
                    "VTAGENT-PERFORMANCE",
                    "1",
                    &current_date,
                    "VTAgent",
                    "User Commands",
                ],
            )
            .control("SH", ["NAME"])
            .text([roman(
                "vtagent-performance - Display performance metrics and system status",
            )])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("performance"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("Display comprehensive performance metrics and system status information."),
                roman(" Shows token usage, API costs, response times, tool execution statistics,"),
                roman(" memory usage patterns, and agent performance metrics."),
            ])
            .control("SH", ["METRICS DISPLAYED"])
            .control("TP", [])
            .text([bold("Token Usage")])
            .text([roman("Input/output token counts and API costs")])
            .control("TP", [])
            .text([bold("Response Times")])
            .text([roman("API response latency and processing times")])
            .control("TP", [])
            .text([bold("Tool Execution")])
            .text([roman("Tool call statistics and execution times")])
            .control("TP", [])
            .text([bold("Memory Usage")])
            .text([roman("Memory consumption patterns")])
            .control("TP", [])
            .text([bold("Agent Performance")])
            .text([roman("Single-agent execution metrics")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Show performance metrics:")])
            .text([bold("  vtagent performance")])
            .control("SH", ["SEE ALSO"])
            .text([
                bold("vtagent(1)"),
                roman(", "),
                bold("vtagent-benchmark(1)"),
            ])
            .render();

        Ok(page)
    }

    /// Generate man page for the benchmark command
    fn generate_benchmark_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control(
                "TH",
                [
                    "VTAGENT-BENCHMARK",
                    "1",
                    &current_date,
                    "VTAgent",
                    "User Commands",
                ],
            )
            .control("SH", ["NAME"])
            .text([roman(
                "vtagent-benchmark - Run SWE-bench evaluation framework",
            )])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("benchmark"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman(
                    "Run automated performance testing against the SWE-bench evaluation framework.",
                ),
                roman(" Provides comparative analysis across different models, benchmark scoring,"),
                roman(" and optimization insights for coding tasks."),
            ])
            .control("SH", ["FEATURES"])
            .control("TP", [])
            .text([bold("Automated Testing")])
            .text([roman("Run standardized coding tasks and challenges")])
            .control("TP", [])
            .text([bold("Comparative Analysis")])
            .text([roman("Compare performance across different models")])
            .control("TP", [])
            .text([bold("Benchmark Scoring")])
            .text([roman("Quantitative performance metrics and scores")])
            .control("TP", [])
            .text([bold("Optimization Insights")])
            .text([roman("Recommendations for performance improvements")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Run benchmark suite:")])
            .text([bold("  vtagent benchmark")])
            .control("SH", ["SEE ALSO"])
            .text([
                bold("vtagent(1)"),
                roman(", "),
                bold("vtagent-performance(1)"),
            ])
            .render();

        Ok(page)
    }

    /// Generate man page for the create-project command
    fn generate_create_project_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control(
                "TH",
                [
                    "VTAGENT-CREATE-PROJECT",
                    "1",
                    &current_date,
                    "VTAgent",
                    "User Commands",
                ],
            )
            .control("SH", ["NAME"])
            .text([roman(
                "vtagent-create-project - Create complete Rust project with features",
            )])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("create-project"),
                roman(" "),
                italic("NAME"),
                roman(" "),
                italic("FEATURES"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("Create a complete Rust project with advanced features and integrations."),
                roman(" Supports web frameworks, database integration, authentication systems,"),
                roman(" testing setup, and tree-sitter integration."),
            ])
            .control("SH", ["AVAILABLE FEATURES"])
            .text([roman("• web - Web framework (Axum, Rocket, Warp)")])
            .text([roman("• auth - Authentication system")])
            .text([roman("• db - Database integration")])
            .text([roman("• test - Testing setup")])
            .text([roman("• tree-sitter - Code analysis integration")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Create web app with auth and database:")])
            .text([bold("  vtagent create-project myapp web,auth,db")])
            .text([roman("Create basic project:")])
            .text([bold("  vtagent create-project simple_app")])
            .control("SH", ["SEE ALSO"])
            .text([bold("vtagent(1)"), roman(", "), bold("vtagent-init(1)")])
            .render();

        Ok(page)
    }

    /// Generate man page for the init command
    fn generate_init_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control(
                "TH",
                [
                    "VTAGENT-INIT",
                    "1",
                    &current_date,
                    "VTAgent",
                    "User Commands",
                ],
            )
            .control("SH", ["NAME"])
            .text([roman(
                "vtagent-init - Initialize project with enhanced structure",
            )])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("init"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("Initialize a project with enhanced dot-folder structure for VTAgent."),
                roman(" Creates project directory structure, config files, cache directories,"),
                roman(" embeddings storage, and tree-sitter parser setup."),
            ])
            .control("SH", ["DIRECTORY STRUCTURE"])
            .text([roman(
                "• .vtagent/ - Main project cache and context directory",
            )])
            .text([roman("• .vtagent/config/ - Configuration files")])
            .text([roman("• .vtagent/cache/ - File and analysis cache")])
            .text([roman("• .vtagent/embeddings/ - Code embeddings storage")])
            .text([roman("• .vtagent/parsers/ - Tree-sitter parsers")])
            .text([roman("• .vtagent/context/ - Agent context stores")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Initialize current directory:")])
            .text([bold("  vtagent init")])
            .control("SH", ["SEE ALSO"])
            .text([
                bold("vtagent(1)"),
                roman(", "),
                bold("vtagent-create-project(1)"),
            ])
            .render();

        Ok(page)
    }

    /// Generate man page for the man command itself
    fn generate_man_man_page() -> Result<String> {
        let current_date = Self::current_date();
        let page = Roff::new()
            .control("TH", ["VTAGENT-MAN", "1", &current_date, "VTAgent", "User Commands"])
            .control("SH", ["NAME"])
            .text([roman("vtagent-man - Generate or display man pages for VTAgent commands")])
            .control("SH", ["SYNOPSIS"])
            .text([
                bold("vtagent"),
                roman(" ["),
                bold("OPTIONS"),
                roman("] "),
                bold("man"),
                roman(" ["),
                italic("COMMAND"),
                roman("] ["),
                bold("--output"),
                roman(" "),
                italic("FILE"),
                roman("]"),
            ])
            .control("SH", ["DESCRIPTION"])
            .text([
                roman("Generate or display Unix man pages for VTAgent commands. Man pages provide"),
                roman(" detailed documentation for all VTAgent functionality including usage examples,"),
                roman(" option descriptions, and feature explanations."),
            ])
            .control("SH", ["OPTIONS"])
            .control("TP", [])
            .text([bold("--output"), roman(" "), italic("FILE")])
            .text([roman("Write man page to specified file instead of displaying")])
            .control("SH", ["AVAILABLE COMMANDS"])
            .text([roman("• chat - Interactive AI coding assistant")])
            .text([roman("• ask - Single prompt mode")])
            .text([roman("• analyze - Workspace analysis")])
            .text([roman("• performance - Performance metrics")])
            .text([roman("• benchmark - SWE-bench evaluation")])
            .text([roman("• create-project - Project creation")])
            .text([roman("• init - Project initialization")])
            .text([roman("• man - Man page generation (this command)")])
            .control("SH", ["EXAMPLES"])
            .text([roman("Display main VTAgent man page:")])
            .text([bold("  vtagent man")])
            .text([roman("Display chat command man page:")])
            .text([bold("  vtagent man chat")])
            .text([roman("Save man page to file:")])
            .text([bold("  vtagent man chat --output chat.1")])
            .control("SH", ["SEE ALSO"])
            .text([bold("vtagent(1)"), roman(", "), bold("man(1)")])
            .render();

        Ok(page)
    }

    /// Save man page to file
    pub fn save_man_page(content: &str, filename: &Path) -> Result<()> {
        fs::write(filename, content)
            .with_context(|| format!("Failed to write man page to {}", filename.display()))?;
        Ok(())
    }

    /// Get list of available commands for man page generation
    pub fn available_commands() -> Vec<&'static str> {
        vec![
            "chat",
            "ask",
            "analyze",
            "performance",
            "benchmark",
            "create-project",
            "init",
            "man",
        ]
    }
}

# VTAgent - Coding Agent

[![tool-eval](https://github.com/vinhnx/vtagent/actions/workflows/tool-eval.yml/badge.svg)](https://github.com/vinhnx/vtagent/actions/workflows/tool-eval.yml)

VTAgent is a Rust-based terminal coding agent with modular architecture supporting multiple LLM providers (Gemini, OpenAI, Anthropic, DeepSeek) and tree-sitter parsers for 6+ languages. It provides a reliable, context-aware coding experience through intelligent tool integration and sophisticated prompt engineering.

<div align="center">
  <img src="screenshots/vtagent.png" alt="VTAgent screenshot" width="800">
</div>

## Features

### Core Capabilities

- **Multi-Agent Architecture**: Orchestrator, Explorer, and Coder agents for specialized tasks
- **Multiple LLM Providers**: Gemini, OpenAI, Anthropic, DeepSeek support with latest models
- **Advanced Code Analysis**: Tree-sitter parsers for Rust, Python, JavaScript, TypeScript, Go, Java
- **Intelligent Tool Suite**: File operations, search, terminal commands, and PTY integration
- **Configuration Management**: TOML-based configuration with comprehensive policies
- **Safety & Security**: Path validation, command policies, and human-in-the-loop controls

### Key Features

- **Context Engineering**: Full conversation history with intelligent management
- **Workspace Safety**: Path validation and configurable file exclusions
- **Enhanced PTY Support**: Full terminal emulation for interactive commands
- **Batch Operations**: Efficient multi-file processing and terminal command execution
- **Configuration Flexibility**: Comprehensive TOML configuration for all aspects
- **Multi-Agent Coordination**: Strategic task delegation and verification workflows
- **Performance Monitoring**: Real-time metrics and benchmarking capabilities
- **Research-Preview Features**: Advanced context compression and conversation summarization

## Quick Start

### Prerequisites

- Rust 1.75+ (stable)
- API key for your preferred LLM provider:
  - `GEMINI_API_KEY` or `GOOGLE_API_KEY` for Gemini
  - `OPENAI_API_KEY` for OpenAI
  - `ANTHROPIC_API_KEY` for Anthropic
  - `DEEPSEEK_API_KEY` for DeepSeek

### Installation

```bash
# Clone the repository
git clone https://github.com/vinhnx/vtagent.git
cd vtagent

# Build the project
cargo build --release

# Or use the provided scripts
./run.sh              # Production build + run
./run-debug.sh        # Development build + run
```

### Basic Usage

```bash
# Set your API key
export GEMINI_API_KEY=your_api_key_here

# Initialize VTAgent in your project
./run.sh init  # Creates vtagent.toml and .vtagentgitignore

# Start interactive chat
./run.sh chat

# Or run specific commands
cargo run -- chat
```

## Available Models & Providers

VTAgent supports the latest models from multiple providers:

### Gemini (Google)
- `gemini-2.5-flash-lite` - Fastest, most cost-effective (default)
- `gemini-2.5-flash` - Fast, cost-effective
- `gemini-2.5-pro` - Latest, most capable

### OpenAI
- `gpt-5` - Latest GPT model

### Anthropic
- `claude-sonnet-4-20250514` - Latest Claude model

### DeepSeek
- `deepseek-reasoner` - Advanced reasoning model

### Local Models
- `qwen/qwen3-4b-2507` - Qwen3 local model

## Command Reference

### Core Commands

#### `chat` - Interactive AI coding assistant
**Features:**
• Multi-agent coordination for complex tasks
• Real-time code generation and editing
• Tree-sitter powered analysis
• Research-preview context management

**Usage:** `vtagent chat`

#### `ask <prompt>` - Single prompt mode
**Perfect for:**
• Quick questions
• Code explanations
• Simple queries

**Example:** `vtagent ask "Explain Rust ownership"`

#### `chat-verbose` - Verbose interactive chat
**Shows:**
• Tool execution details
• API request/response
• Agent coordination (in multi-agent mode)
• Performance metrics

**Usage:** `vtagent chat-verbose`

#### `analyze` - Analyze workspace
**Provides:**
• Project structure analysis
• Language detection
• Code complexity metrics
• Dependency insights
• Symbol extraction

**Usage:** `vtagent analyze`

#### `performance` - Display performance metrics
**Shows:**
• Token usage and API costs
• Response times and latency
• Tool execution statistics
• Memory usage patterns
• Agent performance (in multi-agent mode)

**Usage:** `vtagent performance`

### Project Management

#### `init` - Initialize project
**Features:**
• Creates project directory structure
• Sets up config, cache, embeddings directories
• Creates .project metadata file
• Tree-sitter parser setup
• Multi-agent context stores

**Usage:** `vtagent init`

#### `init-project` - Initialize project with dot-folder structure
**Features:**
• Creates project directory structure in ~/.vtagent/projects/
• Sets up config, cache, embeddings, and retrieval directories
• Creates .project metadata file
• Migrates existing config/cache files with user confirmation

**Examples:**
```
vtagent init-project
vtagent init-project --name my-project
vtagent init-project --force
```

#### `create-project <name> <features>` - Create complete Rust project
**Features:**
• Web frameworks (Axum, Rocket, Warp)
• Database integration
• Authentication systems
• Testing setup
• Tree-sitter integration

**Example:** `vtagent create-project my-api serde,axum,tokio`

### Advanced Features

#### `compress-context` - Compress conversation context
**Benefits:**
• Reduced token usage
• Faster responses
• Memory optimization
• Context preservation

**Usage:** `vtagent compress-context`

#### `benchmark` - Benchmark against SWE-bench
**Features:**
• Automated performance testing
• Comparative analysis across models
• Benchmark scoring and metrics
• Optimization insights

**Usage:** `vtagent benchmark`

### Snapshot Management

#### `snapshots` - List all available snapshots
**Shows:**
• Snapshot ID and turn number
• Creation timestamp
• Description
• File size and compression status

**Usage:** `vtagent snapshots`

#### `revert --turn <n>` - Revert to previous snapshot
**Features:**
• Revert to any previous turn
• Partial reverts (memory, context, full)
• Safe rollback with validation

**Examples:**
```
vtagent revert --turn 5
vtagent revert --turn 3 --partial memory
```

#### `cleanup-snapshots` - Clean up old snapshots
**Features:**
• Remove snapshots beyond limit
• Configurable retention policy
• Safe deletion with confirmation

**Examples:**
```
vtagent cleanup-snapshots
vtagent cleanup-snapshots --max 20
```

### Configuration Management

#### `config` - Generate configuration file
**Features:**
• Generate default configuration
• Support for global (home directory) and local configuration
• TOML format with comprehensive settings
• Multi-agent configuration options
• Tree-sitter and performance monitoring settings

**Examples:**
```
vtagent config
vtagent config --output ./custom-config.toml
vtagent config --global
```

#### `tool-policy` - Manage tool execution policies
**Features:**
• Granular tool permissions
• Security level presets
• Audit logging
• Safe tool execution

**Examples:**
```
vtagent tool-policy status
vtagent tool-policy allow file-write
vtagent tool-policy deny shell-exec
```

#### `models` - Manage models and providers
**Features:**
• Support for latest models (DeepSeek, etc.)
• Provider configuration and testing
• Model performance comparison
• API key management

**Examples:**
```
vtagent models list
vtagent models set-provider deepseek
vtagent models set-model deepseek-reasoner
vtagent models test gemini
```

### Security & Analysis

#### `security` - Security and safety management
**Features:**
• Security scanning and vulnerability detection
• Audit logging and monitoring
• Access control management
• Privacy protection settings

**Usage:** `vtagent security`

#### `tree-sitter` - Tree-sitter code analysis tools
**Features:**
• AST-based code parsing
• Symbol extraction and navigation
• Code complexity analysis
• Multi-language refactoring

**Usage:** `vtagent tree-sitter`

#### `man` - Generate man pages
**Features:**
• Generate Unix man pages for all commands
• Display detailed command documentation
• Save man pages to files
• Comprehensive help for all VTAgent features

**Examples:**
```
vtagent man
vtagent man chat
vtagent man chat --output chat.1
```

## Architecture

### Core Components

- **`vtagent-core/`**: Library crate with core functionality
- **`src/`**: Binary crate with CLI interface
- **`prompts/`**: System prompts for different agent types
- **`docs/`**: Comprehensive documentation
- **Tool specs**: see `docs/tools/TOOL_SPECS.md` for schemas, examples, and limits
- **`examples/`**: Usage examples and demonstrations

### Agent Types

#### Orchestrator Agent
- **Strategic coordinator** managing complex development tasks
- **Task delegation** to specialized subagents
- **Context management** and knowledge accumulation
- **Verification workflows** ensuring implementation quality

#### Explorer Agent
- **Read-only investigator** for understanding and verification
- **System exploration** and configuration discovery
- **Implementation verification** of coder agent work
- **Structured reporting** through context accumulation

#### Coder Agent
- **Implementation specialist** with full write access
- **Code generation** and modification capabilities
- **Technical sophistication** in debugging and optimization
- **Quality assurance** through comprehensive testing

## Configuration

VTAgent uses a comprehensive TOML configuration system loaded from `vtagent.toml`:

### Basic Configuration

```toml
# Agent behavior settings
[agent]
model = "gemini-2.5-flash-lite"
max_conversation_turns = 1000
verbose_logging = false

# Security and safety settings
[security]
human_in_the_loop = true
confirm_destructive_actions = true
max_file_size_mb = 50

# Tool execution policies
[tools]
default_policy = "prompt"

[tools.policies]
read_file = "allow"
write_file = "prompt"
run_terminal_cmd = "prompt"

# Command permissions
[commands]
allow_list = ["ls", "pwd", "cat", "git status", "cargo check"]
deny_list = ["rm -rf", "sudo rm", "shutdown"]

# PTY configuration
[pty]
enabled = true
default_rows = 24
default_cols = 80
command_timeout_seconds = 300
```

### Configuration Commands

```bash
# Initialize with default configuration
./run.sh init

# Generate complete configuration (preserves existing settings)
./run.sh config

# Generate configuration and save to specific file
./run.sh config --output my-config.toml

# Copy example configuration
cp vtagent.toml.example vtagent.toml

# Validate configuration
./run.sh config --validate
```

**Configuration Generation**: The `config` command implements two-way synchronization:
- **Reads existing `vtagent.toml`** if present, preserving your customizations
- **Generates complete TOML** with all sections, even missing ones
- **Falls back to defaults** if no configuration exists
- **Ensures consistency** between your config file and generated templates

## Tool Suite

### File Operations
- `list_files(path?)` - Explore directories with metadata
- `read_file(path, start_line?, end_line?)` - Read text files with line control
- `write_file(path, content)` - Create or overwrite files
- `edit_file(path, old_string, new_string)` - Surgical file editing

### Search & Analysis
- `rp_search(pattern, path?)` - Fast text search using ripgrep
- `grep_search(pattern, include_pattern?)` - Advanced regex search
- `ast_grep_search(pattern, language?)` - Syntax-aware code search

### Terminal Integration
- `run_terminal_cmd(command)` - Execute terminal commands
- `run_in_terminal(command, is_background?)` - Enhanced terminal execution

### PTY Support
- `configure_notebook(file_path)` - Configure Jupyter notebook kernels
- `run_notebook_cell(cell_id, file_path)` - Execute notebook cells
- `read_notebook_cell_output(cell_id, file_path)` - Read cell execution results

### Color Utilities
VTAgent now includes advanced color manipulation capabilities through the `coolor` crate integration:

- RGB to ANSI color conversion for terminal compatibility
- HSL color space support for intuitive color manipulation
- Color blending and harmonious color scheme generation
- Lighten/darken operations for dynamic color adjustments

These utilities are available through the `vtagent_core::utils::colors` module for developers extending VTAgent's functionality.

## Usage Examples

### Basic Chat Session

```bash
# Start interactive chat with tools
./run.sh chat

# Single question mode (no tools)
./run.sh ask "Explain Rust ownership"

# Verbose chat with detailed logging
./run.sh chat-verbose
```

### Project Analysis

```bash
# Analyze current project structure
./run.sh analyze

# Display performance metrics
./run.sh performance
```

### Project Creation

```bash
# Create new Rust project with features
./run.sh create-project my-api serde,axum,tokio

# Available features: web frameworks, databases, async runtimes, etc.
```

### Context Management

```bash
# Compress conversation context for long sessions
./run.sh compress-context

# Demo async file operations
./run.sh demo-async
```

### Snapshot Management

```bash
# List all available snapshots
./run.sh snapshots

# Revert to previous turn
./run.sh revert --turn 5

# Clean up old snapshots (keep last 20)
./run.sh cleanup-snapshots --max 20
```

### Configuration Management

```bash
# Initialize project with configuration
./run.sh init

# Manage tool policies
./run.sh tool-policy status
./run.sh tool-policy allow read_file
./run.sh tool-policy deny run_terminal_cmd

# Manage models and providers
./run.sh models list
./run.sh models set-provider openai
./run.sh models set-model gpt-5
./run.sh models test gemini
```

## Global Options

VTAgent supports several global options that can be used with any command:

### Model & Provider Options
- `--model <MODEL>` - Specify LLM model (e.g., `gemini-2.5-flash-lite`)
- `--provider <PROVIDER>` - Specify LLM provider (e.g., `gemini`, `openai`, `anthropic`, `deepseek`)
- `--api-key-env <ENV_VAR>` - Specify API key environment variable

### Multi-Agent Options
- `--force-multi-agent` - Enable multi-agent mode for complex tasks
- `--agent-type <TYPE>` - Specify agent type (`orchestrator`, `explorer`, `coder`, `single`)

### Feature Flags
- `--enable-tree-sitter` - Enable tree-sitter code analysis
- `--performance-monitoring` - Enable performance monitoring
- `--research-preview` - Enable research-preview features

### Security & Safety
- `--security-level <LEVEL>` - Set security level (`strict`, `moderate`, `permissive`)
- `--max-concurrent-ops <NUM>` - Maximum concurrent operations (default: 5)
- `--max-tool-calls <NUM>` - Maximum tool calls per session (default: 10)

### Output & Logging
- `--verbose` - Enable verbose logging
- `--debug` - Enable debug output
- `--log-level <LEVEL>` - Set log level (`error`, `warn`, `info`, `debug`, `trace`)
- `--no-color` - Disable color output
- `--show-file-diffs` - Show file diffs in chat interface

### System Configuration
- `--workspace <PATH>` - Set workspace root directory
- `--config <PATH>` - Specify configuration file path
- `--skip-confirmations` - Skip safety confirmations

## Contributing

VTAgent is an open-source project. Contributions are welcome! Please see our [contributing guidelines](CONTRIBUTING.md) for details.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

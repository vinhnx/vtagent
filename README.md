# VTAgent - Terminal Coding Agent

VTAgent is a Rust-based terminal coding agent with modular architecture supporting multiple LLM providers (Gemini, OpenAI, Anthropic) and tree-sitter parsers for 6+ languages. It provides a reliable, context-aware coding experience through intelligent tool integration and sophisticated prompt engineering.

<div align="center">
  <img src="screenshots/vtagent.png" alt="VTAgent screenshot" width="800">
</div>

## Features

### Core Capabilities

- **Multi-Agent Architecture**: Orchestrator, Explorer, and Coder agents for specialized tasks
- **Multiple LLM Providers**: Gemini, OpenAI, Anthropic, and LMStudio support
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

## Quick Start

### Prerequisites

- Rust 1.75+ (stable)
- API key for your preferred LLM provider:
  - `GEMINI_API_KEY` or `GOOGLE_API_KEY` for Gemini
  - `OPENAI_API_KEY` for OpenAI
  - `ANTHROPIC_API_KEY` for Anthropic

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

## Architecture

### Core Components

- **`vtagent-core/`**: Library crate with core functionality
- **`src/`**: Binary crate with CLI interface
- **`prompts/`**: System prompts for different agent types
- **`docs/`**: Comprehensive documentation
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

# Copy example configuration
cp vtagent.toml.example vtagent.toml

# Validate configuration
./run.sh config --validate
```

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

## Usage Examples

### Basic Chat Session

```bash
./run.sh chat
```

### Project Analysis

```bash
# Analyze current project structure
./run.sh analyze

# Get detailed file information
./run.sh info
```

### Code Generation

```bash
# Generate new Rust module
./run.sh generate --type module --name utils

# Create project template
./run.sh template --type web-api --features serde,axum
```

### Configuration Management

```bash
# Edit configuration interactively
./run.sh config --edit

# Show current configuration
./run.sh config --show
```

## Advanced Features

### Multi-Agent Workflows

VTAgent supports sophisticated multi-agent coordination:

```bash
# Start orchestrator for complex tasks
./run.sh orchestrate "Implement user authentication system"

# Explorer agent for investigation
./run.sh explore "Analyze current codebase structure"

# Coder agent for implementation
./run.sh code "Add new API endpoint"
```

### Context Engineering

Advanced context management for long conversations:

- **Automatic compression** of conversation history
- **Intelligent summarization** of completed tasks
- **Context preservation** during errors
- **Memory optimization** for large codebases

### Safety Features

Comprehensive safety and security controls:

- **Path validation** preventing access outside workspace
- **Command allow/deny lists** with pattern matching
- **Human-in-the-loop** confirmation for dangerous operations
- **File size limits** and type restrictions
- **API key masking** in logs and snapshots

## Development

### Build Commands

```bash
# Quick compilation check
cargo check

# Full build with optimizations
cargo build --release

# Run tests
cargo test

# Run linter
cargo clippy

# Format code
cargo fmt
```

### Project Structure

```
vtagent/
├── vtagent-core/          # Core library crate
│   ├── src/
│   │   ├── config/        # Configuration management
│   │   ├── llm/          # LLM provider abstractions
│   │   ├── tools/        # Tool implementations
│   │   └── prompts/      # System prompt management
├── src/                   # Binary crate
├── prompts/              # Agent system prompts
├── docs/                 # Documentation
├── examples/             # Usage examples
└── scripts/              # Build and run scripts
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run the test suite
6. Submit a pull request

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with verbose output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

## Documentation

Comprehensive documentation is available in the `docs/` directory:

- [Architecture Overview](docs/ARCHITECTURE.md)
- [Configuration Guide](docs/CONFIGURATION.md)
- [API Reference](docs/API.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Contributing Guide](docs/CONTRIBUTING.md)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

VTAgent builds upon several key works in the agent development space:

- **Anthropic's agent patterns** - Tool design and workflow principles
- **Cognition's context engineering** - Long-running agent reliability
- **OpenAI's Swarm concepts** - Multi-agent coordination
- **Tree-sitter** - Advanced code parsing capabilities
- **Rust ecosystem** - High-performance systems programming

## Support

- **Issues**: [GitHub Issues](https://github.com/vinhnx/vtagent/issues)
- **Discussions**: [GitHub Discussions](https://github.com/vinhnx/vtagent/discussions)
- **Documentation**: [VTAgent Docs](https://vinhnx.github.io/vtagent/)

---

*VTAgent is an open-source project created by [vinhnx](https://github.com/vinhnx). Contributions and feedback are welcome!*
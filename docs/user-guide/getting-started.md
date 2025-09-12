# Getting Started with VTAgent

Welcome to VTAgent! This guide will help you get up and running with this Rust-based terminal coding agent that provides intelligent, context-aware coding assistance through multi-agent architecture.

## What Makes VTAgent Special

VTAgent represents a modern approach to AI-powered software development, featuring:

- **Multi-Agent Architecture** - Orchestrator, Explorer, and Coder agents for specialized tasks
- **Multi-Provider LLM Support** - Gemini, OpenAI, Anthropic, and LMStudio integration
- **Advanced Code Intelligence** - Tree-sitter parsers for 6+ programming languages
- **Enterprise-Grade Safety** - Comprehensive security controls and path validation
- **Flexible Configuration** - TOML-based configuration with granular policies
- **Research-Preview Features** - Cutting-edge agent coordination and context engineering

## Prerequisites

### System Requirements

- **Rust**: 1.75+ (stable recommended)
  - Install from [rustup.rs](https://rustup.rs/)
  - Includes Cargo package manager
- **Git**: For version control and cloning the repository
- **Operating System**: macOS, Linux, or Windows (with WSL2)

### API Requirements

Choose one of the supported LLM providers:

- **Gemini** (Primary): `export GEMINI_API_KEY=your_key_here` or `export GOOGLE_API_KEY=your_key_here`
  - Get from [Google AI Studio](https://aistudio.google.com/app/apikey)

- **OpenAI**: `export OPENAI_API_KEY=your_key_here`
  - Get from [OpenAI Platform](https://platform.openai.com/api-keys)

- **Anthropic**: `export ANTHROPIC_API_KEY=your_key_here`
  - Get from [Anthropic Console](https://console.anthropic.com/)

- **LMStudio**: Local LLM support (no API key required)
  - Run LMStudio locally with compatible models

## Installation

### Option 1: Direct Download (Recommended)

```bash
# Clone the repository
git clone https://github.com/vinhnx/vtagent.git
cd vtagent

# Build the project
cargo build --release

# The binary will be available at target/release/vtagent
```

### Option 2: Using Provided Scripts

```bash
# Clone and build using the production script
git clone https://github.com/vinhnx/vtagent.git
cd vtagent

# Build and run in production mode
./run.sh

# Or build and run in development mode
./run-debug.sh
```

### Option 3: Cargo Install (Future)

```bash
# When published to crates.io
cargo install vtagent
```

## Quick Start

### 1. Set Your API Key

```bash
# For Gemini (recommended)
export GEMINI_API_KEY=your_api_key_here

# For OpenAI
export OPENAI_API_KEY=your_api_key_here

# For Anthropic
export ANTHROPIC_API_KEY=your_api_key_here
```

### 2. Initialize VTAgent in Your Project

```bash
# Navigate to your project
cd /path/to/your/project

# Initialize VTAgent (creates vtagent.toml and .vtagentgitignore)
../vtagent/target/release/vtagent init

# Or if using scripts
/path/to/vtagent/run.sh init
```

### 3. Start Your First Session

```bash
# Start interactive chat
../vtagent/target/release/vtagent chat

# Or with the script
/path/to/vtagent/run.sh chat
```

## Configuration

VTAgent uses a comprehensive TOML configuration system. The `init` command creates a `vtagent.toml` file with sensible defaults.

### Basic Configuration

```toml
# Agent settings
[agent]
model = "gemini-2.5-flash-lite"  # Your preferred model
max_conversation_turns = 1000
verbose_logging = false

# Security settings
[security]
human_in_the_loop = true
confirm_destructive_actions = true
max_file_size_mb = 50

# Tool policies
[tools]
default_policy = "prompt"

[tools.policies]
read_file = "allow"
write_file = "prompt"
run_terminal_cmd = "prompt"
```

### Advanced Configuration

```toml
# Command permissions
[commands]
allow_list = ["ls", "pwd", "cat", "git status", "cargo check"]
deny_list = ["rm -rf", "sudo rm", "shutdown"]

# PTY settings
[pty]
enabled = true
default_rows = 24
default_cols = 80
command_timeout_seconds = 300

# Multi-agent settings
[multi_agent]
enabled = true
max_concurrent_tasks = 3
context_store_size_mb = 100
```

## Usage Examples

### Basic Chat Session

```bash
vtagent chat
```

The agent will greet you and await your instructions. Try asking:

- "Analyze this codebase"
- "Add error handling to the user authentication"
- "Refactor this function to be more readable"
- "Create a new API endpoint for user registration"

### Multi-Agent Mode

```bash
# Start orchestrator for complex tasks
vtagent orchestrate "Implement a complete user management system"

# Explorer agent for investigation
vtagent explore "Analyze the current database schema"

# Coder agent for implementation
vtagent code "Add password reset functionality"
```

### Project Analysis

```bash
# Comprehensive project analysis
vtagent analyze

# Get detailed information
vtagent info

# Generate project summary
vtagent summary
```

## Understanding the Agents

### Orchestrator Agent
- **Role**: Strategic coordinator and task planner
- **Capabilities**: Task decomposition, agent delegation, progress tracking
- **Use for**: Complex multi-step tasks requiring coordination

### Explorer Agent
- **Role**: Investigation and verification specialist
- **Capabilities**: Code analysis, system exploration, testing
- **Use for**: Understanding codebases, verifying implementations

### Coder Agent
- **Role**: Implementation specialist
- **Capabilities**: Code writing, refactoring, debugging
- **Use for**: Making code changes, adding features, fixing bugs

## Advanced Features

### Context Engineering

VTAgent automatically manages conversation context:

- **Intelligent compression** of long conversations
- **Persistent context store** across sessions
- **Automatic summarization** of completed tasks
- **Context-aware responses** based on full history

### Safety Features

Comprehensive security controls:

- **Path validation** - Prevents access outside workspace
- **Command policies** - Allow/deny lists for terminal commands
- **Human-in-the-loop** - Confirmation for dangerous operations
- **File size limits** - Prevents processing of large files
- **API key masking** - Secure credential handling

### Tool Integration

Rich tool ecosystem:

- **File Operations**: Read, write, edit files safely
- **Search & Analysis**: Fast text search and AST-based analysis
- **Terminal Commands**: Execute shell commands with PTY support
- **Code Intelligence**: Tree-sitter powered syntax analysis
- **Batch Operations**: Process multiple files efficiently

## Troubleshooting

### Common Issues

#### API Key Not Set
```bash
# Check if API key is set
echo $GEMINI_API_KEY

# Set the API key
export GEMINI_API_KEY=your_key_here
```

#### Configuration Issues
```bash
# Validate configuration
vtagent config --validate

# Reset to defaults
vtagent config --reset
```

#### Build Issues
```bash
# Clean and rebuild
cargo clean
cargo build --release

# Check Rust version
rustc --version
cargo --version
```

### Getting Help

- **Documentation**: Comprehensive guides in `docs/` directory
- **GitHub Issues**: Report bugs and request features
- **GitHub Discussions**: Community support and discussions
- **Configuration Examples**: Check `vtagent.toml.example`

## Next Steps

Now that you have VTAgent running, explore:

1. **[Multi-Agent Guide](../MULTI_AGENT_GUIDE.md)** - Learn about agent coordination
2. **[Configuration Guide](../CONFIGURATION.md)** - Advanced configuration options
3. **[Architecture Guide](../ARCHITECTURE.md)** - System design and components
4. **[Provider Guides](../PROVIDER_GUIDES.md)** - LLM provider integration

## Contributing

Interested in contributing? Check out:

- **[Development Guide](../development/README.md)** - Development setup
- **[Contributing Guide](../../CONTRIBUTING.md)** - Contribution guidelines
- **[Code Standards](../development/code-style.md)** - Coding conventions

---

**Happy coding with VTAgent!** 🚀

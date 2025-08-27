# vtagent - Advanced Rust Coding Agent

vtagent is a minimal terminal-based coding agent that implements modern agent architecture patterns. It combines Anthropic's agent-building principles with Cognition's context engineering approach to provide a reliable, long-running coding assistant.

## Architecture & Design Principles

### Core Architecture

vtagent follows proven agent architecture patterns:

- **Single-threaded execution** - No parallel subagents that could make conflicting decisions
- **Full context sharing** - Every action is informed by complete conversation history
- **Explicit decision tracking** - All agent actions include reasoning and context
- **Context engineering** - Intelligent management of conversation history and state

### Implementation Patterns

- **Augmented LLM** with comprehensive tool suite
- **Prompt chaining** for complex multi-step tasks
- **Routing patterns** for specialized workflows
- **Context compression** for long-running conversations

## Features

### Core Capabilities

- Interactive chat with full conversation memory
- Gemini function-calling with custom tool integration
- Workspace safety with path validation
- Comprehensive logging and debugging support
- Multiple operational modes

### Advanced Features

- **Context compression** - Handle long conversations without losing important information
- **Decision transparency** - Track why each action is taken
- **Error recovery** - Preserve context during failures
- **Workflow patterns** - Specialized modes for different tasks

### Built-in Tools

- `list_files(path?)` - Explore directories with metadata
- `read_file(path, max_bytes?)` - Read text files with size control
- `write_file(path, content, overwrite?, create_dirs?)` - Create/overwrite files
- `edit_file(path, old_str, new_str)` - Surgical file editing with validation

## Prerequisites

- Rust 1.75+ (stable)
- Gemini API key: `GEMINI_API_KEY` or `GOOGLE_API_KEY` environment variable

## Quick Start

### Basic Usage

```bash
# Set API key
export GEMINI_API_KEY=your_key_here

# Start interactive chat
cargo run -- chat

# Verbose mode with detailed logging
cargo run -- chat-verbose
```

### Specialized Modes

```bash
# Analyze workspace structure
cargo run -- analyze

# Create complete Rust project
cargo run -- create-project my-app serde,tokio

# Demonstrate context compression
cargo run -- compress-context

# Single question without tools
cargo run -- ask "What is Rust?"
```

## Command Reference

### Interactive Modes

- `chat` - Standard interactive chat with tool support
- `chat-verbose` - Enhanced logging with decision tracking

### Specialized Workflows

- `analyze` - Comprehensive workspace analysis using routing pattern
- `create-project <name> <features>` - Complete project creation via prompt chaining
- `compress-context` - Demonstrate context engineering principles

### Utility Commands

- `ask <prompt>` - Single question without tool access

## Configuration Options

- `--model <model>` - Gemini model (default: gemini-2.5-flash)
- `--api-key-env <var>` - API key environment variable (default: GEMINI_API_KEY)
- `--workspace <path>` - Working directory (default: current directory)

## How It Works

### Agent Loop Architecture

1. **User Input** → Added to conversation history
2. **Context Processing** → Full history sent to Gemini with tool definitions
3. **Decision Making** → Gemini decides if tools are needed
4. **Tool Execution** → Local execution with complete context awareness
5. **Result Integration** → Tool results added back to conversation
6. **Response Generation** → Gemini provides final answer with full context

### Context Engineering

Following Cognition's principles:

- **Share full context** - Never lose important information
- **Actions carry decisions** - Make implicit decisions explicit
- **Single-threaded reliability** - Avoid parallel execution conflicts
- **Context compression** - Handle long conversations efficiently

### Safety Features

- **Path validation** - Prevents access outside workspace
- **Exact string matching** - Prevents accidental file modifications
- **Overwrite protection** - Optional safety checks for file operations
- **Error context preservation** - Maintain conversation state during failures

## Advanced Usage Examples

### Project Analysis

```bash
cargo run -- analyze
# Provides comprehensive workspace overview
# Detects project types, languages, frameworks
# Reads key configuration files
# Analyzes source code structure
```

### Code Generation Workflow

```bash
cargo run -- create-project web-api serde,axum,tokio
# Creates complete Rust web API project
# Step-by-step generation with progress tracking
# Includes Cargo.toml, source files, documentation
# Follows best practices and project structure
```

### Context Management

```bash
cargo run -- compress-context
# Demonstrates conversation compression
# Shows how to handle long-running sessions
# Illustrates context engineering principles
```

## Extending the Agent

### Adding New Tools

1. Define tool function in `src/tools.rs`
2. Add tool declaration with comprehensive documentation
3. Register in `build_function_declarations()`
4. Implement proper error handling and validation

### Custom Workflows

Create specialized command patterns by:

1. Adding new CLI commands in `main.rs`
2. Implementing workflow functions
3. Following established patterns for context management
4. Including proper error handling and logging

## Architecture Decisions

### Why Single-threaded?

- **Reliability** - No conflicting decisions between parallel agents
- **Context integrity** - Complete conversation history always available
- **Debugging** - Clear execution flow and decision tracking
- **Simplicity** - Easier to reason about and maintain

### Why Full Context?

- **Decision quality** - Every action informed by complete history
- **Consistency** - No loss of important information
- **Recovery** - Full context preserved during errors
- **Transparency** - Users understand agent's reasoning

### Why Explicit Tracking?

- **Debugging** - Easy to understand why actions were taken
- **Auditability** - Clear record of agent decisions
- **Improvement** - Data for optimizing agent behavior
- **Trust** - Users can verify agent's reasoning

## Performance Considerations

- **Context limits** - Monitor conversation length and provide warnings
- **Tool efficiency** - Optimized file operations with size limits
- **Memory usage** - Streaming responses and efficient data structures
- **API costs** - Context compression reduces token usage

## Limitations

- **Context windows** - Limited by Gemini's context limits (~1M tokens)
- **File types** - Currently supports text files only
- **Execution environment** - Local file system access only
- **Language scope** - Optimized for software development tasks

## Future Enhancements

### Planned Features

- **Context compression** - Automatic conversation summarization
- **Multi-file operations** - Batch file processing capabilities
- **Project templates** - Predefined project scaffolds
- **Integration APIs** - REST endpoints for agent integration

### Research Areas

- **Advanced context management** - Sophisticated compression algorithms
- **Multi-modal inputs** - Support for images, diagrams, audio
- **Collaborative workflows** - Human-agent teaming patterns
- **Domain specialization** - Industry-specific agent capabilities

## Related Work

This implementation draws inspiration from:

- **Anthropic's agent patterns** - Tool design and workflow principles
- **Cognition's context engineering** - Long-running agent reliability
- **OpenAI's Swarm** - Multi-agent coordination concepts
- **Microsoft's AutoGen** - Conversational agent frameworks

## Contributing

Contributions welcome! Areas of particular interest:

- Additional tool implementations
- Workflow pattern extensions
- Performance optimizations
- Documentation improvements
- Testing and validation

## License

See LICENSE file for details.

## Attribution

This project builds upon several key works in the agent development space:

- Thorsten Ball's "How to build an agent" - Core agent architecture
- Anthropic's agent-building guides - Tool design and safety patterns
- Cognition's context engineering principles - Long-running agent reliability
- Open source agent implementations - Community best practices

# vtagent - Research-preview Rust Coding Agent

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

### Research-preview Features

- **Async File Operations** - Non-blocking file writes with concurrent processing
- **Real-time Diff Rendering** - Visual diff display in chat for file changes
- **Chunked File Reading** - Memory-efficient reading of large files
- **Context compression** - Handle long conversations without losing important information
- **Decision transparency** - Track why each action is taken
- **Error recovery** - Preserve context during failures
- **Workflow patterns** - Specialized modes for different tasks
- **.vtagentgitignore Support** - Custom file exclusion patterns for agent operations
- **Snapshot Checkpoint System** - Complete state persistence and revert capabilities

### Built-in Tools

- `list_files(path?)` - Explore directories with metadata
- `read_file(path, max_bytes?)` - Read text files with size control
- `write_file(path, content, overwrite?, create_dirs?)` - Create/overwrite files
- `edit_file(path, old_str, new_str)` - Surgical file editing with validation

## .vtagentgitignore - Custom File Exclusion

vtagent supports a `.vtagentgitignore` file that works like `.gitignore` but only affects the agent's file operations. This allows you to exclude certain files from the agent's context without affecting your project's actual `.gitignore`.

### How it Works

1. Create a `.vtagentgitignore` file in your project root
2. Use standard gitignore patterns to specify files to exclude
3. The agent automatically respects these patterns during file operations
4. Your project's `.gitignore` remains unaffected

### Example .vtagentgitignore

```gitignore
# Exclude log files
*.log
logs/

# Exclude build artifacts
target/
build/
dist/

# Exclude temporary files
*.tmp
*.temp
.cache/

# Exclude sensitive files
.env
.env.local
secrets/

# Exclude IDE files
.vscode/
.idea/

# Allow specific files (negation patterns)
!important.log
!CHANGELOG.md
```

### Benefits

- **Performance**: Exclude large or irrelevant files from processing
- **Security**: Prevent the agent from accessing sensitive files
- **Focus**: Keep the agent focused on relevant code files
- **Flexibility**: Different exclusion patterns for different projects
- **Separation**: Agent exclusions don't interfere with your project's git configuration

The agent automatically detects and uses `.vtagentgitignore` files when present, with no additional configuration required.

## Configuration (vtagent.toml)

VTAgent uses a comprehensive TOML configuration system that allows you to customize agent behavior, tool policies, and command permissions. Configuration is loaded from `vtagent.toml` in your project root, with fallback to `.vtagent/vtagent.toml`.

### Quick Start

Initialize a new project with default configuration:

```bash
vtagent init  # Creates vtagent.toml and .vtagentgitignore
```

Or copy the example configuration:

```bash
cp vtagent.toml.example vtagent.toml
```

### Configuration Sections

#### [agent] - Agent Behavior
```toml
[agent]
max_conversation_turns = 1000      # Prevent runaway conversations
max_session_duration_minutes = 60  # Auto-terminate long sessions
verbose_logging = false             # Enable detailed logging
```

#### [tools] - Tool Execution Policies
```toml
[tools]
default_policy = "prompt"  # "allow", "prompt", or "deny"

[tools.policies]
# Override default policy for specific tools
read_file = "allow"        # Allow without confirmation
write_file = "prompt"      # Require user confirmation
delete_file = "deny"       # Always deny
run_terminal_cmd = "prompt"
```

#### [commands] - Unix Command Permissions
```toml
[commands]
# Commands that execute automatically without confirmation
allow_list = [
    "ls", "pwd", "cat", "grep", "git status", "cargo check"
]

# Commands that are always denied
deny_list = [
    "rm -rf", "sudo rm", "shutdown", "format"
]

# Patterns requiring extra confirmation
dangerous_patterns = [
    "rm -f", "git reset --hard", "pip install"
]
```

#### [security] - Security Settings
```toml
[security]
human_in_the_loop = true              # Require confirmation for critical actions
confirm_destructive_actions = true    # Extra confirmation for dangerous operations
log_all_commands = true               # Log all executed commands
max_file_size_mb = 50                 # Maximum file size to process
allowed_file_extensions = [".rs", ".toml", ".md"]  # Restrict file types
```

### Human-in-the-Loop Workflow

The configuration enables sophisticated human-in-the-loop control:

1. **Allow List Commands**: Execute automatically without prompting
   - `git status`, `cargo check`, `ls`, `grep`, etc.

2. **Standard Commands**: Prompt for confirmation
   - `cargo build`, `npm install`, custom scripts

3. **Dangerous Commands**: Require extra confirmation with warnings
   - `rm -f`, `git reset --hard`, `docker system prune`

4. **Denied Commands**: Always blocked for security
   - `rm -rf`, `sudo rm`, `shutdown`, `format`

### Tool Policies

Each tool can have one of three policies:

- **allow**: Execute automatically without user confirmation
- **prompt**: Ask user for confirmation before execution
- **deny**: Never allow execution

### Example Workflow

```bash
# These commands execute automatically (in allow_list)
VTAgent: [TOOL] run_terminal_cmd {"command": "git status"}
[ALLOWED] Command is in allow list: git status

# These commands require confirmation
VTAgent: [TOOL] run_terminal_cmd {"command": "cargo build"}
[CONFIRM] Execute command 'cargo build'? [y/N] y

# Dangerous commands get extra warnings
VTAgent: [TOOL] run_terminal_cmd {"command": "rm -f old_file.txt"}
[WARNING] DANGEROUS command 'rm -f old_file.txt' - Are you sure? [y/N] y
```

### Legacy Support

The old `.vtagent/tool-policy.json` format is deprecated but still supported for backward compatibility. New projects should use the TOML configuration.

## Snapshot Checkpoint System

vtagent includes a comprehensive snapshot checkpoint system that enables robust experimentation, debugging, and reproducibility. Every agent turn automatically creates a complete state snapshot, allowing you to revert to any previous point in the conversation.

### Key Features

- **Automatic Snapshots** - Complete agent state saved on every turn
- **Selective Revert** - Revert memory, context, or full state independently
- **Encryption Support** - Optional AES-256 encryption for sensitive data
- **Compression** - Automatic gzip compression for large snapshots
- **Integrity Verification** - SHA-256 checksums ensure data integrity
- **Cleanup Management** - Automatic cleanup of old snapshots

### How It Works

1. **Snapshot Creation** - On each agent turn, complete state is serialized to JSON
2. **State Components** - Captures conversation history, decisions, errors, performance metrics
3. **Integrity Checks** - SHA-256 checksums verify snapshot integrity
4. **Compression** - Large snapshots automatically compressed with gzip
5. **Encryption** - Optional AES-256-GCM encryption for sensitive data

### Snapshot Contents

Each snapshot includes:

- **Conversation History** - Complete chat history with all messages
- **Agent Configuration** - Model settings, API keys (masked), workspace
- **Decision Tracking** - All decisions made with reasoning and outcomes
- **Error Recovery** - Error patterns and recovery attempts
- **Performance Metrics** - Response times, token usage, success rates
- **Context State** - Compaction engine, summarizer, tree-sitter state
- **Environment** - Safe environment variables and system state

### Usage Examples

#### Basic Revert Operations

```bash
# Revert to a specific turn (full state)
cargo run -- revert --turn 5

# Revert only conversation memory
cargo run -- revert --turn 3 --scope memory

# Revert only decision context
cargo run -- revert --turn 2 --scope context

# List all available snapshots
cargo run -- list-snapshots

# Clean up old snapshots (keep last 20)
cargo run -- cleanup-snapshots --max 20
```

#### Encrypted Snapshots

```bash
# Create encrypted snapshots
export VTAGENT_ENCRYPTION_KEY="your-secure-password"
cargo run -- --encryption-enabled chat

# Revert from encrypted snapshot
cargo run -- revert --turn 5 --encryption-key "your-secure-password"
```

#### Snapshot Management

```bash
# View snapshot information
cargo run -- show-snapshot --id turn_5_1640995200

# Delete specific snapshot
cargo run -- delete-snapshot --id turn_5_1640995200

# Export snapshot for analysis
cargo run -- export-snapshot --id turn_5_1640995200 --output snapshot.json
```

### Configuration Options

```bash
# Enable encryption
--encryption-enabled

# Set encryption key
--encryption-key "your-password"

# Configure snapshot directory
--snapshots-dir "./my-snapshots"

# Set maximum snapshots to keep
--max-snapshots 50

# Set compression threshold (bytes)
--compression-threshold 1048576  # 1MB

# Disable automatic cleanup
--no-cleanup
```

### Security Considerations

- **API Key Masking** - API keys are never stored in plain text
- **Encryption** - AES-256-GCM encryption for sensitive snapshots
- **Environment Filtering** - Only safe environment variables are captured
- **File Permissions** - Snapshots stored with appropriate permissions
- **Automatic Cleanup** - Old snapshots automatically removed

### Performance Impact

- **Minimal Overhead** - Snapshot creation is asynchronous and optimized
- **Compression** - Automatic compression reduces storage requirements
- **Cleanup** - Automatic cleanup prevents disk space issues
- **Memory Efficient** - Streaming serialization for large conversations

### Example Workflow

```bash
# Start a session
cargo run -- chat

# Agent processes several turns, creating snapshots automatically
# turn_1_1640995200.json
# turn_2_1640995260.json
# turn_3_1640995320.json

# Something goes wrong at turn 5
# Revert to turn 3 and continue
cargo run -- revert --turn 3

# Create a branch for experimentation
cargo run -- branch-snapshot --from turn_3 --name experiment_1

# Continue with different approach
cargo run -- chat

# Later, revert to the experimental branch
cargo run -- revert --snapshot experiment_1_turn_1
```

### Integration with Development Workflow

The snapshot system integrates seamlessly with development:

- **Debugging** - Step back through conversation to identify issues
- **Experimentation** - Try different approaches from the same starting point
- **Reproducibility** - Share exact conversation states with team members
- **Recovery** - Quickly recover from errors without losing progress
- **Analysis** - Review decision patterns and agent behavior over time

### Technical Details

- **Format** - JSON with optional gzip compression
- **Encryption** - AES-256-GCM with Argon2 key derivation
- **Integrity** - SHA-256 checksums for all snapshots
- **Storage** - Atomic writes prevent corruption
- **Cleanup** - Configurable retention policies
- **Versioning** - Forward and backward compatibility

The snapshot system ensures that every agent interaction is preserved and recoverable, enabling safe experimentation and reliable debugging workflows.

## Async File Operations & Diff Rendering

### Async File Operations

vtagent supports non-blocking file operations for improved performance:

```bash
# Enable async file operations
cargo run -- --async-file-ops chat

# Configure concurrent operations
cargo run -- --async-file-ops --max-concurrent-ops 10 chat
```

**Benefits:**

- Non-blocking file writes during chat
- Concurrent processing of multiple file operations
- Improved responsiveness for file-intensive tasks

### Real-time Diff Rendering

See file changes directly in the chat interface:

```bash
# Enable diff display for file changes
cargo run -- --show-file-diffs chat

# Combine with async operations
cargo run -- --async-file-ops --show-file-diffs chat
```

**Features:**

- Visual diff display with syntax highlighting
- Before/after comparison in chat thread
- Automatic change detection for watched files
- Color-coded additions and deletions

### Example Output

```
📝 File: src/main.rs
══════════════════════════════════════════════════
Changes: 5 additions, 2 deletions, 3 modifications

   1| use tokio::fs;
   2|+ use async_file_ops::AsyncFileWriter;
   3| use std::path::PathBuf;
   4|
   5|- fn old_function() {
   6|+ async fn new_async_function() -> Result<()> {
   7|     let writer = AsyncFileWriter::new(5);
   8|+     writer.write_file(path, content).await?;
   9|-     // old code
  10|+     // new async code
  11|     Ok(())
  12| }
```

## Prerequisites

- Rust 1.75+ (stable)
- Gemini API key: `GEMINI_API_KEY` or `GOOGLE_API_KEY` environment variable

## Quick Start

### Basic Usage

```bash
# Set API key
export GEMINI_API_KEY=your_key_here

# Initialize VTAgent in your project
vtagent init  # Creates vtagent.toml and .vtagentgitignore

# Start interactive chat
cargo run -- chat

# Or use the built binary
vtagent chat

# Initialize with force overwrite
vtagent init --force

# Generate config only
vtagent config --output my-config.toml

# Verbose mode with detailed logging
cargo run -- chat-verbose
```

### Configuration-Aware Usage

VTAgent now includes comprehensive TOML-based configuration for tool policies and command permissions:

```bash
# Commands in allow_list execute automatically
git status    # Executes without prompting

# Commands requiring confirmation
cargo build   # Prompts: "Execute command 'cargo build'? [y/N]"

# Dangerous commands show warnings
rm -f file    # Prompts: "[WARNING] DANGEROUS command - Are you sure? [y/N]"
```### Specialized Modes

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
- `--workspace <path>` - Workspace root directory
- `--async-file-ops` - Enable async file operations
- `--show-file-diffs` - Display file changes as diffs in chat
- `--max-concurrent-ops <n>` - Maximum concurrent file operations (default: 5)
- `--chunked-reading` - Enable chunked reading for large files
- `--chunk-size-kb <size>` - Chunk size in KB for file reading (default: 64)
- `--chunk-threshold-mb <size>` - Threshold in MB for using chunked reading (default: 10)

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

## Research-preview Usage Examples

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

- **Research-preview context management** - Sophisticated compression algorithms
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

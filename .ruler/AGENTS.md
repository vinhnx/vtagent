# AGENTS.md

# VTAgent

## Project Overview
VTAgent is a Rust-based terminal coding agent with modular architecture supporting multiple LLM providers (Gemini, OpenAI, Anthropic) and tree-sitter parsers for 6+ languages.

## Architecture & Key Components
- **Workspace Structure**: `vtagent-core/` (library) + `src/` (binary) with modular tools system
- **Core Modules**: `llm/` (provider abstraction), `tools/` (modular tool system), `config/` (TOML-based settings)
- **Integration Points**: Gemini API, tree-sitter parsers, PTY command execution, MCP tools

## Critical Developer Workflows

### Build & Run Commands
```bash
./run.sh              # Production build + run (release mode)
./run-debug.sh        # Development build + run (debug mode)
cargo check           # Quick compilation check (preferred over cargo build)
cargo clippy          # Linting with project-specific rules
```

### Configuration Management
- **Primary Config**: `vtagent.toml` (never hardcode settings)
- **Model Constants**: Always reference `vtagent-core/src/config/constants.rs`
- **Latest Models**: Check `docs/models.json` for current model IDs

## Project-Specific Patterns

### Configuration Pattern
```rust
// ❌ Don't hardcode
let model = "gemini-2.5-flash-lite";

// ✅ Use constants module
use vtagent_core::config::constants::models::google::GEMINI_2_5_FLASH_LITE;
let model = GEMINI_2_5_FLASH_LITE;
```

### Error Handling Pattern
```rust
// ✅ Project standard: anyhow + descriptive context
use anyhow::{Context, Result};

fn process_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))
}
```

### Documentation Pattern
```rust
// ✅ All .md files belong in ./docs/ folder
// ❌ Don't put documentation in root or other folders
```

### Tool Integration Pattern
```rust
// ✅ Use trait-based composition for tools
#[async_trait]
impl Tool for MyTool {
    async fn execute(&self, args: Value) -> Result<Value> {
        // Implementation
    }
}
```

## Code Quality Principles

### Human-Centered Design
- **4-space indentation** (spaces, not tabs)
- **Early returns** over nested conditionals
- **Descriptive variables** over complex expressions
- **Composition** over deep inheritance
- **Deep modules** (simple interface, complex functionality)

### Memory-Efficient Conditionals
```rust
// ❌ Hard to track mentally
if val > threshold && (cond1 || cond2) && (cond3 && !cond4) {
    // ...
}

// ✅ Clear intermediate variables
let is_valid = val > threshold;
let is_allowed = cond1 || cond2;
let is_secure = cond3 && !cond4;

if is_valid && is_allowed && is_secure {
    // ...
}
```

## Integration Points

### LLM Providers
- **Gemini**: Primary provider via `gemini.rs`
- **Multi-Provider**: Abstracted through `llm/` module
- **Configuration**: Model selection via `vtagent.toml`

### Tree-Sitter Integration
- **Supported Languages**: Rust, Python, JavaScript, TypeScript, Go, Java
- **Performance**: Efficient parsing with size limits
- **Error Handling**: Graceful degradation on parse failures

### PTY Command Execution
- **Unified Backend**: All commands use enhanced PTY system
- **Modes**: `terminal`, `pty`, `streaming`
- **Safety**: Configurable command allow/deny lists

## Development Conventions

### Commit Messages
- **Format**: `type(scope): description`
- **Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- **Atomic**: Single logical change per commit

### Testing Strategy
- **Unit Tests**: Co-located with source code in `#[cfg(test)]`
- **Integration Tests**: In `tests/` directory
- **Mocking**: External dependencies for reliable tests

### File Organization
- **Library Code**: `vtagent-core/src/`
- **Binary Entry**: `src/main.rs`
- **Documentation**: `./docs/` folder only
- **Examples**: `examples/` directory
- **Benchmarks**: `benches/` directory

## Security & Safety

### API Key Management
- **Environment Variables**: `GEMINI_API_KEY`, `GOOGLE_API_KEY`
- **Never Hardcode**: Keys must come from environment
- **Validation**: Input sanitization for all external data

### File System Safety
- **Path Validation**: All file operations check workspace boundaries
- **Size Limits**: Configurable maximum file sizes
- **Exclusion Patterns**: `.vtagentgitignore` support

## Performance Considerations

### Async Operations
- **Tokio Runtime**: Full async support with multi-threading
- **Streaming**: Real-time output for long-running commands
- **Caching**: Strategic caching for file operations

### Memory Management
- **Chunked Reading**: Large file handling without memory exhaustion
- **Context Compression**: Automatic conversation summarization
- **Resource Limits**: Configurable timeouts and size limits


## Code Style and Standards

### Rust Conventions

- Follow standard Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Use `anyhow` for error handling with descriptive error messages
- Prefer `thiserror` for custom error types when needed
- Use `clap` with derive macros for CLI argument parsing
- Follow the Rust API guidelines for public APIs

### Code Organization

- Keep modules focused and cohesive
- Use `src/lib.rs` for library code and `src/main.rs` for binary entry point
- Place CLI-specific code in the `cli/` directory
- Keep examples in the `examples/` directory
- Place benchmarks in the `benches/` directory

### Dependencies

- Use async/await with `tokio` for I/O operations
- Use `reqwest` with rustls for HTTP requests
- Use `serde` for serialization/deserialization
- Use `tree-sitter` for code parsing and analysis
- Use `walkdir` and `glob` for file system operations

## Development Guidelines

### Error Handling

- Use `anyhow::Result<T>` for functions that can fail
- Provide meaningful error messages with context
- Use `?` operator for error propagation
- Log errors appropriately using `console` crate

### Performance

- Use `tokio` for async operations
- Consider using `futures` for complex async workflows
- Profile with `criterion` benchmarks in the `benches/` directory
- Use `tree-sitter` efficiently for code parsing

### Testing

- Write unit tests in the same file as the code being tested
- Write integration tests in the `tests/` directory
- Use `cargo test` for running tests
- Consider property-based testing for complex algorithms

### Documentation

- Document public APIs with rustdoc comments
- Include examples in documentation
- Keep README.md up to date
- Document CLI usage and examples
- Document (.md) files must put in ./docs folder. don't put any unrelated files in root folder. THIS IS IMPORTANT

## Tree-sitter Integration

- Use tree-sitter parsers for supported languages
- Handle parsing errors gracefully
- Consider performance implications of parsing large files
- Provide meaningful error messages for parsing failures

## CLI Design

- Use `clap` derive macros for argument parsing
- Provide clear help text and examples
- Use `console` and `owo-colors` for colored output
- Use `termimad` for rich terminal output when appropriate

## Security Considerations

- Be careful with file system operations
- Validate input from external sources
- Handle API keys securely (use environment variables)
- Consider rate limiting for external API calls

## Build Configuration

- Use release profile optimizations (codegen-units = 1, opt-level = 3, lto = true)
- Consider using `cargo clippy` for additional linting
- Use `cargo fmt` for code formatting

## File Organization

- Keep related functionality together
- Use appropriate module structure
- Consider the separation between library and binary code
- Place configuration files at the project root

## When Making Changes

- Ensure all tests pass
- Run `cargo clippy` and fix warnings
- Format code with `cargo fmt`
- Update documentation if needed
- Consider performance implications
- Test with various input files and edge cases
- Prefered ripgrep for searching code
- DO NOT USE EMOJI THIS IS IMPORTANT
- Put all agent's configuration option to vtagent.toml. This is important, every logic should be read from this toml config instead of hardcode.
- Use MCP tools for enhanced context awareness if needed.
- Use `cargo check` instead of `cargo build` whenever possible
- Use `cargo nextest` Instead of `cargo test
- Put all markdown documentation files to ./docs folder. don't put any unrelated files in root folder. THIS IS IMPORTANT!
- Put all report documentation (md) files to ./docs folder. don't put any unrelated files in root folder. THIS IS IMPORTANT!
- Always check ./docs/models.json use to use latest LLM model. Don't guess the model id/name. THIS IS IMPORTANT!
- Always use vtagent-core/src/config/constants.rs for constant values. Don't hardcode any constant values, especially model id. THIS IS IMPORTANT!
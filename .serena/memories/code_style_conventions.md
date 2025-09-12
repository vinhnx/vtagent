# Code Style and Conventions for vtagent

## Rust Language Standards

### Edition and Version
- **Rust Edition**: 2021
- **Minimum Rust Version**: 1.75+
- **Target Platforms**: macOS (Darwin), Linux, Windows

### Code Formatting (rustfmt)
- **Tool**: `cargo fmt` (rustfmt)
- **Configuration**: Default rustfmt configuration
- **Line Length**: 100 characters (default)
- **Indentation**: 4 spaces (Rust standard)
- **Style**: Follows official Rust style guidelines

### Linting (clippy)
- **Tool**: `cargo clippy`
- **Strictness**: `-D warnings` (treat warnings as errors)
- **Scope**: `--workspace --all-targets --all-features`
- **Auto-fix**: `cargo clippy --fix` for automatic fixes

## Naming Conventions

### Variables and Functions
- **snake_case**: `variable_name`, `function_name()`
- **Descriptive names**: `user_input`, `parse_command()`, `validate_path()`

### Types and Structs
- **PascalCase**: `UserConfig`, `AgentBuilder`, `ToolRegistry`
- **Descriptive and specific**: `ConversationSummarizer`, `ErrorRecoveryManager`

### Constants
- **SCREAMING_SNAKE_CASE**: `MAX_RETRIES`, `DEFAULT_TIMEOUT`
- **Module-level constants**: `const DEFAULT_MODEL: &str = "gemini-2.5-flash";`

### Modules and Files
- **snake_case**: `conversation_summarizer.rs`, `error_recovery.rs`
- **Descriptive**: `tree_sitter_integration.rs`, `performance_monitor.rs`

## Documentation Standards

### Module Documentation
```rust
//! # Module Name
//!
//! Brief description of what this module does.
//!
//! ## Architecture
//!
//! Detailed explanation of the module's purpose and design.

#![doc = r#"
# Module Name

Detailed markdown documentation here.
"#]
```

### Function Documentation
```rust
/// Brief description of what the function does.
///
/// # Arguments
/// * `param1` - Description of parameter 1
/// * `param2` - Description of parameter 2
///
/// # Returns
/// Description of return value
///
/// # Errors
/// Description of possible errors
///
/// # Examples
/// ```
/// let result = my_function(arg1, arg2)?;
/// ```
pub fn my_function(param1: Type1, param2: Type2) -> Result<ReturnType, ErrorType> {
    // implementation
}
```

### Struct Documentation
```rust
/// Represents a user configuration with validation.
///
/// This struct contains all user-configurable settings
/// for the agent, with built-in validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// The API key for Gemini integration
    pub api_key: String,

    /// Optional workspace directory override
    pub workspace: Option<PathBuf>,
}
```

## Error Handling Patterns

### Error Types
- **Custom Error Types**: Use `thiserror` for domain-specific errors
- **Standard Errors**: Use `anyhow` for generic error handling
- **Result Types**: Always use `Result<T, E>` for fallible operations

### Error Handling Example
```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("File operation failed: {0}")]
    FileError(#[from] std::io::Error),
}

pub fn process_request(config: &Config) -> Result<Response> {
    let api_key = config.api_key.as_ref()
        .context("API key not configured")?;

    // ... implementation

    Ok(response)
}
```

## Async Programming

### Tokio Usage
- **Runtime**: `#[tokio::main]` for main functions
- **Async Functions**: `async fn function_name() -> Result<T>`
- **Await Pattern**: Use `.await` consistently
- **Streams**: Use `tokio-stream` for async iteration

### Async Example
```rust
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel(100);
    let mut stream = ReceiverStream::new(rx);

    while let Some(item) = stream.next().await {
        process_item(item).await?;
    }

    Ok(())
}
```

## Testing Conventions

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Given
        let input = "test input";

        // When
        let result = function_under_test(input);

        // Then
        assert_eq!(result, expected_output);
    }

    #[tokio::test]
    async fn test_async_function() {
        // Async test implementation
    }
}
```

### Integration Tests
- **Location**: `tests/` directory
- **Naming**: `integration_tests.rs`, `file_operations_test.rs`
- **Isolation**: Each test should be independent

### Mock Data
- **Location**: `tests/common.rs` or `tests/mock_data.rs`
- **Purpose**: Shared test utilities and mock data

## Project Structure Conventions

### Module Organization
```
src/
├── lib.rs              # Main library file with re-exports
├── main.rs             # Binary entry point
├── agent/              # Core agent functionality
│   ├── mod.rs
│   ├── core.rs
│   └── intelligence.rs
├── cli/                # Command-line interface
├── commands/           # CLI command implementations
├── tools/              # Tool definitions and registry
├── types/              # Shared type definitions
└── prompts/            # AI prompt templates
```

### Import Organization
```rust
// Standard library imports
use std::collections::HashMap;
use std::path::PathBuf;

// External crate imports (alphabetical)
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// Local imports (relative)
use crate::agent::Agent;
use crate::tools::ToolRegistry;

// Group related imports
use crate::{
    types::{Config, Response},
    utils::{validate_path, format_output},
};
```

## Performance Considerations

### Memory Management
- **Borrowing**: Prefer references over ownership where possible
- **Cloning**: Minimize unnecessary clones
- **Collections**: Use appropriate collection types (`Vec`, `HashMap`, `BTreeMap`)

### Concurrency
- **Thread Safety**: Use appropriate synchronization primitives
- **Async Bounds**: Consider `Send` and `Sync` bounds for async functions
- **Resource Pooling**: Reuse expensive resources (HTTP clients, etc.)

## Security Practices

### Input Validation
- **Path Validation**: Always validate file paths before operations
- **API Keys**: Never log sensitive information
- **User Input**: Sanitize and validate all user inputs

### Error Information
- **Sensitive Data**: Never include sensitive data in error messages
- **Logging**: Use appropriate log levels (debug, info, warn, error)
- **Context**: Provide helpful error context without exposing internals

## Development Workflow

### Pre-commit Checks
- **Formatting**: `cargo fmt --all -- --check`
- **Linting**: `cargo clippy -- -D warnings`
- **Testing**: `cargo test`
- **Building**: `cargo build`

### CI/CD Integration
- **Automated Checks**: All pre-commit checks run in CI
- **Test Coverage**: Maintain comprehensive test coverage
- **Performance**: Monitor and track performance benchmarks
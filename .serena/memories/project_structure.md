# vtagent Project Structure

## Root Directory Structure

```
vtagent/
├── src/                    # Main Rust library source code
├── cli/                    # CLI application source code
├── tests/                  # Integration and unit tests
├── examples/               # Example usage and demonstrations
├── docs/                   # Documentation
├── scripts/                # Development and build scripts
├── benches/                # Performance benchmarks
├── target/                 # Build artifacts (generated)
├── Cargo.toml             # Main project configuration
├── Cargo.lock             # Dependency lock file
├── README.md              # Project documentation
└── LICENSE                # Project license
```

## Source Code Structure (`src/`)

### Core Modules

```
src/
├── lib.rs                 # Main library file, module declarations, re-exports
├── main.rs               # Binary entry point (if applicable)
├── agent/                # Core agent functionality
│   ├── mod.rs           # Agent module declarations
│   ├── core.rs          # Main agent orchestrator
│   ├── chat.rs          # Chat functionality
│   ├── intelligence.rs  # AI integration layer
│   └── performance.rs   # Agent performance monitoring
├── cli/                  # Command-line interface
│   ├── mod.rs
│   ├── args.rs          # CLI argument parsing
│   └── commands.rs      # CLI command definitions
├── commands/             # CLI command implementations
│   ├── mod.rs
│   ├── analyze.rs       # Workspace analysis command
│   ├── ask.rs           # Single question command
│   ├── compress_context.rs # Context compression command
│   ├── create_project.rs # Project creation command
│   ├── stats.rs         # Statistics command
│   └── validate.rs      # Validation command
├── tools/                # Tool definitions and registry
│   ├── mod.rs
│   ├── tools.rs         # Tool implementations
│   ├── enhanced_tools.rs # Minimal research-preview tool implementations
│   └── tools.rs.backup  # Backup of tools.rs
├── types/                # Shared type definitions
│   └── mod.rs
├── prompts/              # AI prompt templates
│   ├── mod.rs
│   ├── system.rs        # System prompts
│   └── templates.rs     # Prompt templates
├── gemini.rs            # Gemini AI integration
├── context_analyzer.rs  # Context analysis functionality
├── conversation_summarizer.rs # Conversation summarization
├── decision_tracker.rs  # Decision tracking and logging
├── error_recovery.rs    # Error handling and recovery
├── markdown_renderer.rs # Markdown rendering utilities
├── performance_monitor.rs # Performance monitoring
├── performance_profiler.rs # Performance profiling
├── code_completion.rs   # Code completion engine
└── tree_sitter/         # Tree-sitter integration
    ├── mod.rs
    ├── analysis.rs      # Code analysis
    ├── analyzer.rs      # Code analyzer
    ├── languages.rs     # Language support
    └── navigation.rs    # Code navigation
```

## CLI Structure (`cli/src/`)

```
cli/
└── src/
    └── main.rs          # CLI application entry point
```

## Test Structure (`tests/`)

```
tests/
├── mod.rs              # Test module declarations
├── common.rs           # Shared test utilities
├── mock_data.rs        # Mock data for testing
├── integration_tests.rs # Integration tests
├── file_operations_test.rs # File operation tests
└── ...                 # Additional test files
```

## Documentation Structure (`docs/`)

```
docs/
├── README.md           # Documentation index
├── api/                # API documentation
│   └── README.md
├── development/        # Development guides
│   ├── README.md
│   ├── ci-cd.md        # CI/CD pipeline documentation
│   └── testing.md      # Testing guide
├── project/            # Project documentation
│   ├── CHANGELOG.md    # Change log
│   ├── ROADMAP.md      # Project roadmap
│   └── TODO.md         # Task list
└── user-guide/         # User guides
    ├── getting-started.md # Getting started guide
    └── tree-sitter-integration.md # Tree-sitter guide
```

## Scripts Structure (`scripts/`)

```
scripts/
├── README.md           # Scripts documentation
├── setup.sh            # Development environment setup
├── check.sh            # Code quality checks
├── release.sh.backup   # Release script backup
├── release.sh.bak      # Release script backup
└── ...                 # Additional scripts
```

## Examples Structure (`examples/`)

```
examples/
├── demo_markdown.rs           # Markdown demonstration
├── gemini_streaming.rs        # Gemini streaming example
├── markdown_streaming.rs      # Markdown streaming example
├── markdown_test.rs           # Markdown testing
├── simulate_gemini_stream.rs  # Gemini simulation
├── test_markdown.rs           # Markdown testing
├── tree_sitter_demo.rs        # Tree-sitter demonstration
├── Cargo.toml                 # Examples Cargo.toml
└── Cargo.lock                 # Examples dependencies
```

## Key Files and Their Purposes

### Configuration Files

- **`Cargo.toml`**: Main project configuration, dependencies, build settings
- **`Cargo.lock`**: Locked dependency versions for reproducible builds
- **`.gitignore`**: Git ignore patterns
- **`rustfmt.toml`**: Code formatting configuration (if present)

### Entry Points

- **`src/main.rs`**: Main binary entry point
- **`cli/src/main.rs`**: CLI application entry point
- **`src/lib.rs`**: Library interface and re-exports

### Core Components

- **`src/agent/`**: Agent orchestration and intelligence
- **`src/tools/`**: Tool definitions and execution
- **`src/gemini.rs`**: AI model integration
- **`src/tree_sitter/`**: Code analysis and parsing

### Development Tools

- **`scripts/setup.sh`**: Environment setup and dependency installation
- **`scripts/check.sh`**: Code quality verification (linting, formatting, testing)

## Navigation Tips

### Finding Code by Functionality

- **Agent Logic**: Look in `src/agent/`
- **Tool Implementations**: Check `src/tools/`
- **CLI Commands**: See `src/commands/`
- **AI Integration**: Check `src/gemini.rs`
- **Code Analysis**: Look in `src/tree_sitter/`
- **Error Handling**: See `src/error_recovery.rs`

### Finding Tests

- **Unit Tests**: In same file as implementation with `#[cfg(test)]` modules
- **Integration Tests**: In `tests/` directory
- **Benchmarks**: In `benches/` directory

### Finding Documentation

- **API Docs**: Use `cargo doc --open`
- **Development Guides**: Check `docs/development/`
- **User Guides**: See `docs/user-guide/`
- **Project Info**: Look at `docs/project/`

## Build and Development Workflow

### Typical Development Session

1. **Setup**: Run `./scripts/setup.sh` (first time)
2. **Code**: Edit files in `src/`
3. **Test**: Run `cargo test` or `./scripts/check.sh`
4. **Build**: Use `cargo build` or `cargo run`
5. **Document**: Update docs in `docs/` if needed

### Common Development Tasks

- **Add Feature**: Edit relevant module in `src/`
- **Add Tool**: Modify `src/tools/`
- **Add Command**: Update `src/commands/` and `src/cli/`
- **Add Test**: Add to `tests/` or inline in source
- **Update Docs**: Edit files in `docs/`

## File Naming Conventions

### Rust Files

- **Modules**: `snake_case.rs` (e.g., `conversation_summarizer.rs`)
- **Tests**: `snake_case_test.rs` or `snake_case.rs` with test modules
- **Examples**: `snake_case.rs` (e.g., `markdown_demo.rs`)

### Documentation Files

- **Guides**: `kebab-case.md` (e.g., `getting-started.md`)
- **Topics**: `kebab-case.md` (e.g., `tree-sitter-integration.md`)

### Script Files

- **Shell Scripts**: `kebab-case.sh` (e.g., `setup.sh`, `check.sh`)
- **Backup Files**: Original name with `.backup` or `.bak` extension

This structure follows Rust community conventions and provides clear separation of concerns while maintaining discoverability and maintainability.

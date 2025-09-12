# Essential Development Commands for vtagent

## Core Development Commands

### Building and Running
```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the main binary
cargo run -- chat

# Run specific commands
cargo run -- analyze
cargo run -- create-project my-app serde,tokio
cargo run -- compress-context
cargo run -- ask "What is Rust?"
```

### Testing and Quality
```bash
# Run all tests
cargo test

# Run tests with detailed output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run integration tests
cargo test --test integration_tests

# Run benchmarks
cargo bench
```

### Code Quality and Formatting
```bash
# Format code
cargo fmt

# Format all files
cargo fmt --all

# Check formatting without changing files
cargo fmt --all -- --check

# Run clippy lints
cargo clippy

# Run clippy with all features
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Fix clippy issues automatically
cargo clippy --fix
```

### Documentation
```bash
# Generate documentation
cargo doc

# Generate docs without dependencies
cargo doc --no-deps

# Open documentation in browser
cargo doc --open

# Generate docs with private items
cargo doc --document-private-items
```

## Development Scripts

### Environment Setup
```bash
# Basic development setup
./scripts/setup.sh

# Setup with git hooks
./scripts/setup.sh --with-hooks

# Show setup help
./scripts/setup.sh --help
```

### Code Quality Checks (CI Pipeline)
```bash
# Run all quality checks
./scripts/check.sh

# Run specific checks
./scripts/check.sh fmt      # Format check only
./scripts/check.sh clippy   # Clippy check only
./scripts/check.sh test     # Tests only
./scripts/check.sh build    # Build only
./scripts/check.sh docs     # Documentation only

# Show check script help
./scripts/check.sh help
```

## Utility Commands

### System Commands (macOS/Darwin)
```bash
# List files
ls -la

# Change directory
cd /path/to/directory

# Find files
find . -name "*.rs" -type f

# Search in files
grep -r "pattern" .

# Check disk usage
du -sh *

# Process management
ps aux | grep cargo
kill -9 <pid>
```

### Git Commands
```bash
# Check status
git status

# Add files
git add .

# Commit changes
git commit -m "message"

# Push changes
git push origin main

# Pull changes
git pull origin main

# Create branch
git checkout -b feature-branch

# Switch branch
git checkout branch-name
```

## Environment Variables

### Required
```bash
# Set Gemini API key
export GEMINI_API_KEY=your_api_key_here

# Alternative API key variable
export GOOGLE_API_KEY=your_api_key_here
```

### Optional Configuration
```bash
# Set workspace directory
export VTAGENT_WORKSPACE=/path/to/workspace

# Set log level
export RUST_LOG=debug
```

## Project Structure Navigation

### Key Directories
```bash
# Main source code
cd src/

# CLI source code
cd cli/src/

# Tests
cd tests/

# Examples
cd examples/

# Documentation
cd docs/

# Development scripts
cd scripts/
```

### Common File Operations
```bash
# View project structure
tree -I target

# Find Rust files
find . -name "*.rs" -type f

# Count lines of code
find src -name "*.rs" -exec wc -l {} + | tail -1

# Check file sizes
du -sh src/* tests/*
```
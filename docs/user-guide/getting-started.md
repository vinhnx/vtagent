# Getting Started with vtagent

Welcome to vtagent! This guide will help you get up and running with this Research-preview AI-powered coding assistant that implements state-of-the-art agent architecture patterns inspired by Anthropic's breakthrough engineering approaches.

## What Makes vtagent Special

vtagent represents a significant advancement in AI-powered software development tools, featuring:

- **Anthropic-Inspired Architecture** - Built following the same principles that achieved 49% on SWE-bench Verified
- **Multi-Language Intelligence** - Research-preview code understanding across 6+ programming languages
- **Enterprise-Grade Reliability** - Production-ready with comprehensive error recovery and context management
- **Complete Transparency** - Full audit trails and decision tracking for all agent actions
- **Comprehensive Testing** - Production-grade test suite with performance benchmarking
- **Research-preview Tool Ecosystem** - Error-proofed tools designed for maximum reliability

## Prerequisites

### System Requirements

- **Rust**: 1.75+ (stable recommended)
  - Install from [rustup.rs](https://rustup.rs/)
  - Includes Cargo package manager
- **Git**: For version control and cloning the repository
- **Operating System**: Linux, macOS, or Windows (with WSL2 recommended)

### API Requirements

- **Gemini API Key**: Required for agent functionality
  - Get your free API key from [Google AI Studio](https://aistudio.google.com/app/apikey)
  - Set environment variable: `export GEMINI_API_KEY=your_key_here`
  - Alternative: `export GOOGLE_API_KEY=your_key_here`

### Fully Bundled Architecture

**No external dependencies required!** All Research-preview features are bundled as Rust crates:

#### Text Search (Bundled)

- **Ultra-fast regex search** using Rust's optimized `regex` crate
- **Word boundary matching** with Research-preview pattern support
- **Case-sensitive/insensitive** search options
- **Context lines** for better understanding
- **Glob pattern filtering** for file type restrictions
- **Hidden file control** (include/exclude dot files)

#### Tree-sitter Integration (Bundled)

- **6 Programming Languages** fully supported: Rust, Python, JavaScript, TypeScript, Go, Java
- **Syntax-aware parsing** with AST generation
- **Symbol extraction** and intelligent navigation
- **Code complexity analysis** and quality metrics
- **Automatic language detection** from file extensions
- **Swift support planned** (parser available, integration pending)

#### Performance & Reliability (Bundled)

- **SIMD-accelerated search** for exceptional performance
- **Memory-efficient parsing** for large codebases
- **Cross-platform compatibility** - works identically on Linux, macOS, and Windows
- **Comprehensive error handling** with graceful degradation

## Quick Start (3 Minutes to First Chat)

### 1. Clone and Setup

```bash
# Clone the repository
git clone https://github.com/your-username/vtagent.git
cd vtagent

# Set your Gemini API key (get from https://aistudio.google.com/app/apikey)
export GEMINI_API_KEY=your_api_key_here
```

### 2. Build and Run

```bash
# Build the project (takes ~30 seconds)
cargo build

# Start your first chat session!
cargo run -- chat
```

**That's it!** You're now ready to experience vtagent's Research-preview capabilities. Try asking:

- *"Analyze this Rust project and tell me about its structure"*
- *"Find all function definitions in the codebase"*
- *"Create a simple web API with error handling"*

### 3. Test the Research-preview Features

```bash
# Experience full transparency with decision tracking
cargo run -- chat-verbose

# Run comprehensive project analysis
cargo run -- analyze --depth deep --format json

# Generate a complete project with dependencies
cargo run -- create-project my-api axum,serde,tokio,sqlx

# Search the codebase using ripgrep-like tool (via chat)
# Ask in chat: "Search for TODO and FIXME with 2 lines of context in Rust files"
# The agent will use rp_search with appropriate arguments
```

### 4. Verify Everything Works

```bash
# Run the comprehensive test suite
cargo test

# Run performance benchmarks
cargo bench
```

## Development & Architecture

### Core Architecture Overview

vtagent implements a sophisticated modular architecture with breakthrough AI agent patterns:

```bash
src/
 agent/           # Core agent logic and chat sessions
 cli/            # Command-line interface and argument parsing
 commands/       # Specialized command implementations
 decision_tracker.rs     # Complete decision audit trail
 error_recovery.rs       # Intelligent error handling & recovery
 conversation_summarizer.rs # Context compression & summarization
 gemini.rs       # Gemini AI API integration
 tools.rs        # Comprehensive tool suite (9 tools)
 tree_sitter/    # Research-preview multi-language code analysis
 types.rs        # Shared type definitions
 prompts/        # System prompts and templates
```

### Key Architectural Innovations

#### Decision Tracking System

- **Complete Audit Trail**: Every decision logged with reasoning and context
- **Confidence Scoring**: Quality assessment for all agent actions
- **Real-time Transparency**: Live decision tracking in verbose mode
- **Pattern Recognition**: Learning from successful strategies

#### Error Recovery Framework

- **Intelligent Pattern Detection**: Automatic identification of recurring errors
- **Multi-Strategy Recovery**: Multiple approaches for handling different error types
- **Context Preservation**: Never lose important information during failures
- **Recovery Effectiveness Tracking**: Analytics on which strategies work best

#### Context Engineering

- **Intelligent Summarization**: Automatic compression for long conversations
- **Context Limit Monitoring**: Proactive warnings and optimization
- **Memory Management**: Efficient handling of conversation history
- **Relevance Scoring**: Smart pruning of less important information

#### Tree-sitter Integration

- **6 Languages Supported**: Rust, Python, JavaScript, TypeScript, Go, Java
- **Syntax-Aware Analysis**: Deep understanding beyond simple text processing
- **Symbol Intelligence**: Research-preview symbol extraction and navigation
- **Code Quality Metrics**: Complexity analysis and maintainability scoring
- **Refactoring Support**: Safe code transformation suggestions

## Comprehensive Testing Framework

vtagent includes a production-grade testing infrastructure with multiple testing layers designed to ensure reliability and performance.

### Automated Test Categories

#### Unit Testing

```bash
# Run all unit tests with detailed output
cargo test --lib -- --nocapture

# Run specific component tests
cargo test tree_sitter::analyzer::tests -- --nocapture
cargo test tools::tests -- --nocapture
cargo test decision_tracker::tests -- --nocapture
cargo test error_recovery::tests -- --nocapture

# Run with coverage (requires grcov)
cargo test --lib
grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./target/coverage/html
```

#### Integration Testing

```bash
# Run end-to-end integration tests
cargo test --test integration_tests -- --nocapture

# Test specific integration scenarios
cargo test test_tool_registry_creation -- --nocapture
cargo test test_grep_search_tool -- --nocapture
cargo test test_tree_sitter_analysis -- --nocapture
cargo test test_error_recovery -- --nocapture
```

#### Performance Benchmarking

```bash
# Run all performance benchmarks
cargo bench

# Run specific benchmark suites
cargo bench --bench search_benchmark
cargo bench --bench tree_sitter_benchmark

# Generate benchmark reports
cargo bench --bench search_benchmark -- --save-baseline current
```

### Test Infrastructure Features

#### Mock Data & Test Environment

```bash
# Run tests with mock Gemini API responses
cargo test mock_gemini_responses -- --nocapture

# Test file system operations with temporary directories
cargo test test_file_operations -- --nocapture

# Validate tree-sitter parsing accuracy
cargo test test_language_detection -- --nocapture
```

#### Automated Testing Pipeline

- **Continuous Integration** - Automated testing on every commit
- **Mock Frameworks** - Realistic test data without external dependencies
- **Performance Regression Detection** - Automatic benchmark comparisons
- **Coverage Analysis** - Code coverage tracking and reporting
- **Cross-Platform Testing** - Validation across different environments

### Manual Testing & Verification

#### Basic Functionality Verification

```bash
# Test basic chat functionality
cargo run -- chat
# Try: "Hello, can you help me create a simple Rust program?"

# Test tool integration
cargo run -- ask "Create a simple text file called test.txt with 'Hello World' content"
cargo run -- ask "List all files in the current directory"
cargo run -- ask "Read the README.md file and summarize it"
```

#### Research-preview Features Testing

```bash
# Experience full transparency
cargo run -- chat-verbose
# Features demonstrated:
#  Real-time decision tracking with reasoning
#  Error recovery with context preservation
#  Conversation summarization alerts
#  Session statistics and performance metrics
#  Pattern detection and optimization recommendations

# Test intelligent error recovery
cargo run -- chat-verbose
# Try: "Read a file that doesn't exist"
# Observe: Intelligent error recovery with context preservation

# Test comprehensive project analysis
cargo run -- analyze --depth deep --format json
# Features:
#  Multi-language syntax-aware parsing
#  Code complexity and quality analysis
#  Symbol extraction and dependency mapping
#  Refactoring suggestions and recommendations

# Test Research-preview code search
cargo run -- chat "Find all function definitions in the codebase"
cargo run -- chat "Search for TODO and FIXME comments with 3 lines of context"
cargo run -- chat "Find all error handling patterns in Python files"
```

#### Tree-sitter Language Support Testing

```bash
# Test multi-language analysis
echo 'def hello_world():
    print("Hello from Python!")
    return 42' > test.py

echo 'function greet(name) {
    console.log(`Hello ${name}!`);
    return true;
}' > test.js

echo 'pub fn calculate_fib(n: u32) -> u64 {
    if n <= 1 {
        return n as u64;
    }
    calculate_fib(n - 1) + calculate_fib(n - 2)
}' > test.rs

# Analyze different languages
cargo run -- ask "Analyze the test.py file for Python code structure"
cargo run -- ask "Analyze the test.js file for JavaScript patterns"
cargo run -- ask "Analyze the test.rs file for Rust code complexity"
```

## Performance & Optimization

### Release Build for Production

```bash
# Optimized release build with maximum performance
cargo build --release

# Run the optimized version
./target/release/vtagent chat-verbose

# Expected performance improvements:
#  3-5x faster execution speed
#  Reduced memory footprint
#  Better API responsiveness
#  Optimized tree-sitter parsing
```

### Performance Benchmarking

```bash
# Run comprehensive performance benchmarks
cargo bench

# Run specific performance tests
cargo bench --bench search_benchmark     # Text search performance
cargo bench --bench tree_sitter_benchmark # Code analysis performance

# Generate benchmark comparison reports
cargo bench --bench search_benchmark -- --save-baseline current
cargo bench --bench search_benchmark -- --baseline current

# Profile specific functions (requires cargo-flamegraph)
cargo flamegraph --bin vtagent -- chat --prompt "benchmark test"
```

### Expected Performance Metrics

- **Build Time**: ~30 seconds (debug), ~2 minutes (release)
- **Memory Usage**: ~50MB base, ~200MB with large contexts
- **API Latency**: 2-10 seconds per interaction (Gemini-dependent)
- **Search Speed**: 50-500 MB/s depending on pattern complexity
- **Tree-sitter Parsing**: 1-5 ms per file for typical codebases
- **Context Window**: Up to 1M tokens (Gemini limit)

## Deployment & Integration

### Local Installation

```bash
# Install vtagent globally on your system
cargo install --path .

# Verify installation
vtagent --version
vtagent --help

# Test basic functionality
vtagent ask "Hello, are you working?"
```

### Docker Deployment

```dockerfile
# Multi-stage Docker build for optimal image size
FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build optimized release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/vtagent /usr/local/bin/vtagent

# Create non-root user
RUN useradd -r -s /bin/false vtagent
USER vtagent

# Set environment variables
ENV GEMINI_API_KEY=""
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD vtagent --help || exit 1

CMD ["vtagent", "chat"]
```

```bash
# Build and run with Docker
docker build -t vtagent .
docker run -it --rm \
  -e GEMINI_API_KEY=your_api_key \
  -v $(pwd):/workspace \
  -w /workspace \
  vtagent
```

### Enterprise Deployment Options

#### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: vtagent
spec:
  replicas: 2
  selector:
    matchLabels:
      app: vtagent
  template:
    metadata:
      labels:
        app: vtagent
    spec:
      containers:
      - name: vtagent
        image: vtagent:latest
        env:
        - name: GEMINI_API_KEY
          valueFrom:
            secretKeyRef:
              name: vtagent-secrets
              key: gemini-api-key
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        volumeMounts:
        - name: workspace
          mountPath: /workspace
      volumes:
      - name: workspace
        persistentVolumeClaim:
          claimName: workspace-pvc
```

#### CI/CD Integration

```yaml
# .github/workflows/deploy.yml
name: Deploy vtagent
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
      - run: cargo bench

  deploy:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
      - name: Build and push Docker image
        run: |
          docker build -t vtagent:${{ github.sha }} .
          docker tag vtagent:${{ github.sha }} vtagent:latest
          # Push to your registry
```

### System Integration

#### Shell Integration

```bash
# Add vtagent to your PATH (if not using cargo install)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Create aliases for common commands
echo 'alias vchat="vtagent chat"' >> ~/.bashrc
echo 'alias vanalyze="vtagent analyze --depth deep"' >> ~/.bashrc
echo 'alias vstats="vtagent stats --detailed"' >> ~/.bashrc
```

#### Editor Integration

```bash
# Create scripts for editor integration
cat > ~/bin/vtagent-analyze-current-dir << 'EOF'
#!/bin/bash
# Analyze current directory with vtagent
vtagent analyze --depth deep --format json > analysis_$(date +%Y%m%d_%H%M%S).json
EOF

chmod +x ~/bin/vtagent-analyze-current-dir
```

#### Desktop Integration (Linux)

```bash
# Create desktop shortcut
cat > ~/.local/share/applications/vtagent.desktop << EOF
[Desktop Entry]
Name=vtagent
Comment=Research-preview AI coding assistant
Exec=vtagent chat
Terminal=true
Type=Application
Icon=terminal
Categories=Development;Utility;
EOF

# Update desktop database
update-desktop-database ~/.local/share/applications/
```

## Configuration

### Environment Variables

```bash
# Required
export GEMINI_API_KEY=your_key_here

# Optional
export RUST_LOG=debug                    # Enable debug logging
export VTAGENT_WORKSPACE=/path/to/workspace  # Default workspace
export VTAGENT_MODEL=gemini-2.5-flash     # Default model
```

### Runtime Configuration

```bash
# Use custom workspace
cargo run -- --workspace /path/to/project chat

# Use different model
cargo run -- --model gemini-pro chat

# Use different API key environment variable
cargo run -- --api-key-env MY_CUSTOM_KEY chat
```

## Troubleshooting

### Common Build Issues

#### Missing Dependencies

```bash
# Update Rust toolchain
rustup update

# Install missing components
rustup component add rustfmt
rustup component add clippy
```

#### ripgrep Installation Issues

```bash
# Check if ripgrep is installed
which rg || echo "ripgrep not found"

# Install ripgrep if missing
# Ubuntu/Debian:
sudo apt-get install ripgrep

# macOS:
brew install ripgrep

# Windows:
choco install ripgrep

# Manual installation:
cargo install ripgrep

# Verify installation
rg --version
```

#### Tree-sitter Parser Issues

```bash
# Clear Cargo cache if parser compilation fails
cargo clean

# Update Cargo registry
cargo update

# Rebuild with verbose output
cargo build --verbose

# Check for specific parser issues
cargo build 2>&1 | grep tree-sitter
```

#### Compilation Errors

```bash
# Clean build artifacts
cargo clean

# Rebuild from scratch
cargo build

# Check for specific error details
cargo build --verbose
```

#### API Key Issues

```bash
# Verify API key is set
echo $GEMINI_API_KEY

# Test API connectivity
cargo run -- ask "Hello"

# Check API key permissions
# Visit: https://aistudio.google.com/app/apikey
```

### Runtime Issues

#### Tool Execution Failures

```bash
# Enable verbose logging
cargo run -- chat-verbose

# Check file permissions
ls -la /path/to/workspace

# Verify tool execution
cargo run -- ask "List files in current directory"
```

#### Context Limit Issues

```bash
# Monitor context usage
cargo run -- chat-verbose
# Look for [WARNING] context limit messages

# Use context compression
cargo run -- compress-context
```

#### Performance Issues

```bash
# Profile performance
cargo build --release
./target/release/vtagent chat

# Check memory usage
/usr/bin/time -v cargo run -- chat
```

#### ripgrep Search Issues

```bash
# Verify ripgrep installation
rg --version

# Test ripgrep functionality
echo "test content" > test.txt
rg "test" test.txt

# If ripgrep fails, reinstall:
cargo install ripgrep

# Test search functionality in vtagent
cargo run -- ask "Search for 'fn' in the codebase"
```

#### Tree-sitter Analysis Issues

```bash
# Test tree-sitter functionality
cargo run -- analyze --depth deep

# Check for parser errors
cargo run -- analyze --depth deep 2>&1 | grep -i tree

# If tree-sitter fails, clean and rebuild
cargo clean
cargo build

# Test with simple Rust file
echo "fn main() { println!(\"Hello\"); }" > test.rs
cargo run -- ask "Analyze the test.rs file"
```

## Research-preview Development

### Code Quality

```bash
# Run linter
cargo clippy

# Format code
cargo fmt

# Run comprehensive checks
cargo check
cargo test
cargo clippy
cargo fmt --check
```

### Documentation

```bash
# Generate documentation
cargo doc --open

# Serve documentation
cargo doc --serve
```

### Continuous Integration

```yaml
# .github/workflows/ci.yml example
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
      - run: cargo clippy
      - run: cargo fmt --check
```

## Performance Metrics

### Expected Performance

- **Build Time**: ~30 seconds (debug), ~2 minutes (release)
- **Memory Usage**: ~50MB base, ~200MB with large contexts
- **API Latency**: 2-10 seconds per interaction
- **Context Window**: Up to 1M tokens (Gemini limit)

### Benchmark Results

```bash
# Run performance benchmarks
cargo bench

# Profile specific functions
cargo flamegraph --bin vtagent -- test_function
```

## Contributing

### Development Setup

```bash
# Fork and clone
git clone https://github.com/your-username/vtagent.git
cd vtagent

# Create feature branch
git checkout -b feature/new-capability

# Run tests
cargo test

# Format and lint
cargo fmt
cargo clippy
```

### Code Standards

- Follow Rust idioms and best practices
- Add comprehensive documentation
- Include unit tests for new features
- Update README and BUILD.md for significant changes

## Support

### Getting Help

- **Issues**: Use GitHub issues for bugs and feature requests
- **Discussions**: Use GitHub discussions for questions
- **Documentation**: Check README.md and inline code documentation

### Community

- **Discord**: Join our community Discord
- **Twitter**: Follow @vtagent_ai for updates
- **Blog**: Visit our blog for tutorials and insights

---

## Summary

This build guide covers all aspects of working with vtagent, from basic installation to Research-preview development. The project now includes breakthrough Anthropic-inspired features that significantly enhance agent capabilities:

### Core Agent Features

- **Decision Transparency**: Complete audit trail of agent reasoning
- **Error Recovery**: Intelligent error handling with context preservation
- **Conversation Summarization**: Automatic compression for long sessions
- **Confidence Scoring**: Quality assessment for all agent actions

### Research-preview Code Intelligence (NEW)

- **Tree-sitter Integration**: Syntax-aware code analysis for 6+ languages
- **ripgrep Search**: Ultra-fast codebase search with Research-preview pattern matching
- **Multi-language Support**: Rust, Python, JavaScript, TypeScript, Go, Java, Swift
- **AST-based Analysis**: Deep code understanding with symbol extraction
- **Complexity Metrics**: Cyclomatic and cognitive complexity analysis
- **Code Navigation**: Intelligent symbol lookup and cross-referencing

### System Dependencies

- **ripgrep**: Required for fast text search (install via package manager)
- **Tree-sitter Parsers**: Automatically handled by Cargo dependencies

### Performance & Reliability

- **SIMD-accelerated Search**: ripgrep provides exceptional search performance
- **Memory-efficient Parsing**: Tree-sitter parsers are optimized for large codebases
- **Cross-platform Compatibility**: Works on Linux, macOS, and Windows
- **Robust Error Handling**: Comprehensive error recovery with helpful diagnostics

These features, inspired by Anthropic's breakthrough SWE-bench performance, position vtagent as a **state-of-the-art coding assistant** with exceptional transparency, reliability, and **Research-preview code intelligence capabilities**. The integration of tree-sitter and ripgrep represents a significant leap forward in AI-powered software engineering tools!

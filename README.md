<div align="center">

<h1>VT Code</h1>

[![Rust](https://img.shields.io/badge/Rust-BF4545?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/badge/Next.js-000000?logo=next.js&logoColor=white)](https://nextjs.org/)
[![Deployed on Fly.io](https://img.shields.io/badge/Deployed%20on-Fly.io-blueviolet)](https://fly.io)
[![CI](https://github.com/vinhnx/vtchat/actions/workflows/ci.yml/badge.svg)](https://github.com/vinhnx/vtchat/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/vinhnx/vtchat/branch/main/graph/badge.svg)](https://codecov.io/gh/vinhnx/vtchat)

**A modern, privacy-first AI chat application with security**

[Live App](https://vtchat.io.vn) | [Documentation](docs/) | [Repository Guidelines](AGENTS.md) | [Project Status](docs/PROJECT-STATUS.md) | [Features](docs/FEATURES.md) | [Architecture](docs/ARCHITECTURE.md) | [Security](docs/SECURITY.md)

</div>

[![Crates.io](https://img.shields.io/crates/v/vtcode.svg)](https://crates.io/crates/vtcode)
[![Homebrew](https://img.shields.io/badge/homebrew-v0.8.2-orange)](https://github.com/vinhnx/homebrew-tap)
[![GitHub release](https://img.shields.io/github/release/vinhnx/vtcode.svg)](https://github.com/vinhnx/vtcode/releases)
[![docs.rs](https://img.shields.io/docsrs/vtcode)](https://docs.rs/vtcode)

---

## Overview

A Rust-based terminal coding agent with modular architecture supporting multiple LLM providers (Gemini, OpenAI, Anthropic, DeepSeek) and tree-sitter parsers for 6+ languages.

## Quick Start

### 1. Install

**Cargo (Recommended):**

```bash
cargo install vtcode
```

**Homebrew (macOS):**

```bash
# Add the tap
brew tap vinhnx/homebrew-tap

# Install VTCode
brew install vtcode

# Verify installation
vtcode --version
```

**Pre-built Binaries:**
Download from [GitHub Releases](https://github.com/vinhnx/vtcode/releases)

#### Homebrew Installation Details

The Homebrew formula supports both Intel and Apple Silicon Macs:

-   **Apple Silicon (M1/M2/M3)**: Native `aarch64-apple-darwin` binary
-   **Intel Macs**: `x86_64-apple-darwin` binary (when available)

**Supported macOS Versions:**

-   macOS 12.0+ (Monterey and later)
-   Both Intel and Apple Silicon architectures

**Uninstall:**

```bash
brew uninstall vtcode
brew untap vinhnx/homebrew-tap  # Optional: remove the tap
```

#### Troubleshooting Homebrew Installation

**If you encounter issues:**

1. **Update Homebrew:**

```bash
brew update
```

2. **Clear Homebrew cache:**

```bash
brew cleanup
```

3. **Re-tap the repository:**

```bash
brew untap vinhnx/homebrew-tap
brew tap vinhnx/homebrew-tap
```

4. **Check formula:**

```bash
brew info vtcode
```

5. **Manual installation (if needed):**

```bash
# Download the binary directly
curl -L -o vtcode.tar.gz https://github.com/vinhnx/vtcode/releases/download/v0.8.2/vtcode-v0.8.2-aarch64-apple-darwin.tar.gz

# Extract and install
tar -xzf vtcode.tar.gz
sudo mv vtcode /usr/local/bin/
```

### 2. Configure API Key

```bash
# Set your API key (choose one)
export GEMINI_API_KEY="your_gemini_key"
export OPENAI_API_KEY="your_openai_key"
export ANTHROPIC_API_KEY="your_anthropic_key"
export DEEPSEEK_API_KEY="your_deepseek_key"
```

### 3. Run

```bash
# Initialize in your project
vtcode init

# Start interactive chat
vtcode chat

# Or run with custom workspace
vtcode --workspace-dir /path/to/project chat
```

## Requirements

-   **Rust 1.75+** (for building from source)
-   **API Key** from one of: Gemini, OpenAI, Anthropic, or DeepSeek

## Configuration

VTCode uses `vtcode.toml` for configuration. Key settings:

```toml
[agent]
provider = "gemini"  # or "openai", "anthropic", "deepseek"
default_model = "gemini-2.5-flash-lite-preview-06-17"
max_conversation_turns = 150

[security]
human_in_the_loop = true  # Require confirmation for destructive actions
```

## Key Features

-   **Multi-Provider LLM Support** - Gemini, OpenAI, Anthropic, DeepSeek
-   **Advanced Code Analysis** - Tree-sitter parsers for Rust, Python, JavaScript, TypeScript, Go, Java
-   **Intelligent Tool Suite** - File operations, search, terminal commands, PTY integration
-   **Workspace Safety** - Path validation and configurable security policies
-   **Context Engineering** - Full conversation history with intelligent management
-   **Decision Ledger** - Structured record of agent decisions for consistency

## Documentation

-   **API Docs**: [docs.rs/vtcode](https://docs.rs/vtcode)
-   **User Guide**: [docs/](docs/)
-   **Configuration**: [docs/project/](docs/project/)

## 🛠️ Development

```bash
# Clone repository
git clone https://github.com/vinhnx/vtcode.git
cd vtcode

# Build and run
./run.sh              # Production build
./run-debug.sh        # Development build

# Run tests
cargo test

# Check code quality
cargo clippy
cargo fmt
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

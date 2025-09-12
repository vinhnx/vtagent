# VTAgent Documentation Hub

Welcome to the comprehensive documentation for **VTAgent**, a Rust-based terminal coding agent with modular architecture supporting multiple LLM providers and advanced code analysis capabilities.

## What Makes VTAgent Special

VTAgent represents a modern approach to AI-powered software development, featuring:

- **Multi-Agent Architecture** - Orchestrator, Explorer, and Coder agents for specialized tasks
- **Multi-Provider LLM Support** - Gemini, OpenAI, Anthropic, and LMStudio integration
- **Advanced Code Intelligence** - Tree-sitter parsers for 6+ programming languages
- **Enterprise-Grade Safety** - Comprehensive security controls and path validation
- **Flexible Configuration** - TOML-based configuration with granular policies
- **Research-Preview Features** - Cutting-edge agent coordination and context engineering

## Documentation Overview

This documentation is organized to support different user personas and use cases:

### For Users

New to VTAgent? Start with installation and basic usage:

- **[Getting Started](./user-guide/getting-started.md)** - Installation, configuration, and first steps
- **[Multi-Agent Guide](./MULTI_AGENT_GUIDE.md)** - Understanding agent coordination
- **[Configuration Guide](./CONFIGURATION.md)** - Comprehensive configuration options

### For Developers

Contributing to VTAgent? Understand the architecture and development processes:

- **[Architecture Overview](./ARCHITECTURE.md)** - System design and core components
- **[Development Guide](./development/README.md)** - Development environment setup
- **[API Documentation](./api/README.md)** - Technical API references
- **[Code Standards](./development/code-style.md)** - Coding guidelines and best practices

### For Organizations

Deploying VTAgent in production? Focus on enterprise features:

- **[Security Implementation](./SAFETY_IMPLEMENTATION.md)** - Security controls and compliance
- **[Performance Analysis](./PERFORMANCE_ANALYSIS.md)** - Optimization and benchmarking
- **[Provider Guides](./PROVIDER_GUIDES.md)** - LLM provider integration guides

## Core Capabilities

### Multi-Agent Architecture

- **Orchestrator Agent** - Strategic task coordination and delegation
- **Explorer Agent** - Read-only investigation and verification
- **Coder Agent** - Implementation specialist with full write access
- **Context Store** - Persistent knowledge management across agents
- **Task Verification** - Automated quality assurance workflows

### Advanced Code Intelligence

- **Tree-Sitter Integration** - Syntax-aware parsing for Rust, Python, JavaScript, TypeScript, Go, Java
- **Intelligent Search** - Ripgrep and AST-grep powered code analysis
- **Symbol Analysis** - Function, class, and variable extraction
- **Dependency Mapping** - Import relationship analysis
- **Code Quality Assessment** - Complexity and maintainability scoring

### Comprehensive Tool Suite

- **File Operations** - Safe, validated file system operations
- **Terminal Integration** - Enhanced PTY support for interactive commands
- **Search & Analysis** - Fast text and syntax-aware code search
- **Batch Processing** - Efficient multi-file operations
- **Configuration Management** - Dynamic TOML-based settings

### Safety & Security

- **Path Validation** - Prevents access outside workspace boundaries
- **Command Policies** - Allow/deny lists with pattern matching
- **Human-in-the-Loop** - Confirmation for dangerous operations
- **File Size Limits** - Configurable resource constraints
- **API Key Security** - Secure credential management

## Quick Start Guide

### For New Users

1. **[Installation](../README.md#installation)** - Get VTAgent running in minutes
2. **[Basic Configuration](./CONFIGURATION.md)** - Set up your environment
3. **[First Chat Session](../README.md#basic-usage)** - Try interactive coding assistance

### For Developers

1. **[Architecture Overview](./ARCHITECTURE.md)** - Understand the system design
2. **[Development Setup](./development/README.md)** - Configure development environment
3. **[Multi-Agent Patterns](./MULTI_AGENT_GUIDE.md)** - Learn agent coordination

### For Organizations

1. **[Security Implementation](./SAFETY_IMPLEMENTATION.md)** - Enterprise security features
2. **[Provider Integration](./PROVIDER_GUIDES.md)** - LLM provider setup
3. **[Performance Tuning](./PERFORMANCE_ANALYSIS.md)** - Optimization strategies

## Usage Patterns

### Multi-Agent Workflows

```bash
# Complex task orchestration
./run.sh orchestrate "Implement user authentication system"

# Codebase exploration
./run.sh explore "Analyze current architecture"

# Implementation tasks
./run.sh code "Add new API endpoint"
```

### Intelligent Code Analysis

```bash
# Comprehensive project analysis
./run.sh analyze

# Symbol search and navigation
./run.sh search "function process_"

# Dependency analysis
./run.sh deps --graph
```

### Configuration Management

```bash
# Initialize project configuration
./run.sh init

# Edit configuration interactively
./run.sh config --edit

# Validate configuration
./run.sh config --validate
```

## Testing & Quality Assurance

VTAgent includes comprehensive testing infrastructure:

### Test Categories

- **Unit Tests** - Component-level testing with `cargo test`
- **Integration Tests** - End-to-end workflow validation
- **Performance Tests** - Benchmarking with `cargo bench`
- **Configuration Tests** - TOML validation and policy testing

### Quality Assurance

```bash
# Run full test suite
cargo test

# Run with coverage
cargo tarpaulin

# Performance benchmarking
cargo bench

# Linting and formatting
cargo clippy && cargo fmt
```

## Project Information

### Current Status & Roadmap

- **[Roadmap](../ROADMAP.md)** - Future development plans and milestones
- **[Changelog](../CHANGELOG.md)** - Version history and release notes
- **[TODO](./project/TODO.md)** - Current development tasks

### Development Resources

- **[Contributing Guide](../CONTRIBUTING.md)** - How to contribute
- **[Code Standards](./development/code-style.md)** - Coding guidelines
- **[Architecture Decisions](./ARCHITECTURE.md)** - Design rationale

## Support & Community

### Getting Help

- **GitHub Issues** - Report bugs and request features
- **GitHub Discussions** - Community discussions and support
- **Documentation** - Comprehensive guides and tutorials

### Community Resources

- **[Main README](../README.md)** - Project overview and quick reference
- **[GitHub Repository](https://github.com/vinhnx/vtagent)** - Source code and collaboration
- **[Discussions](https://github.com/vinhnx/vtagent/discussions)** - Community support

### Enterprise Support

- **Security Features** - Enterprise-grade security controls
- **Multi-Agent Coordination** - Advanced workflow orchestration
- **Provider Integration** - Multiple LLM provider support
- **Performance Optimization** - Enterprise-scale performance tuning

## License & Attribution

This documentation is part of the VTAgent project. See the main [README](../README.md) for license information.

### Attribution

VTAgent builds upon key developments in AI agent technology:

- **Anthropic's Agent Patterns** - Tool design and safety principles
- **Cognition's Context Engineering** - Long-running agent reliability
- **OpenAI's Multi-Agent Concepts** - Agent coordination patterns
- **Tree-Sitter Ecosystem** - Advanced code parsing capabilities
- **Rust Community** - High-performance systems programming

---

**Documentation Version:** 2.0.0
**Last Updated:** September 2025
**VTAgent Version:** 0.2.0

**Ready to get started?** **[Installation Guide](../README.md#quick-start)**

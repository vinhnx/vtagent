# vtagent - Advanced Rust Coding Agent

## Project Purpose
vtagent is a minimal terminal-based coding agent that implements modern agent architecture patterns. It combines Anthropic's agent-building principles with Cognition's context engineering approach to provide a reliable, long-running coding assistant.

## Core Architecture
- **Single-threaded execution** - No parallel subagents that could make conflicting decisions
- **Full context sharing** - Every action is informed by complete conversation history
- **Explicit decision tracking** - All agent actions include reasoning and context
- **Context engineering** - Intelligent management of conversation history and state

## Key Features
- Interactive chat with full conversation memory
- Gemini function-calling with custom tool integration
- Workspace safety with path validation
- Comprehensive logging and debugging support
- Multiple operational modes (chat, analyze, create-project, etc.)
- Context compression for long conversations
- Decision transparency and error recovery

## Tech Stack
- **Language**: Rust 2021 edition (requires Rust 1.75+)
- **AI Integration**: Google Gemini API
- **Async Runtime**: Tokio
- **CLI Framework**: Clap
- **Serialization**: Serde
- **HTTP Client**: Reqwest
- **Code Analysis**: Tree-sitter (supports Rust, Python, JavaScript, TypeScript, Go, Java)
- **Build Tool**: Cargo
- **Testing**: Built-in Rust testing framework
- **Documentation**: Cargo doc

## Development Environment
- **OS**: macOS (Darwin)
- **Shell**: zsh
- **Package Manager**: Cargo
- **Version Control**: Git
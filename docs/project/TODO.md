speckit command plan doesnt work. if not revert and remove speckit

--

https://github.com/openai/codex/blob/main/codex-rs/core/gpt_5_codex_prompt.md

--

https://github.com/openai/codex/blob/main/codex-rs/core/prompt.md

---

--

https://deepwiki.com/crate-ci/cargo-release

--

9:26:28 ❯ codex

╭────────────────────────────────────────────────────────╮
│ >\_ OpenAI Codex (v0.36.0) │
│ │
│ model: gpt-5-codex /model to change │
│ directory: ~/Developer/learn-by-doing/vtagent │
╰────────────────────────────────────────────────────────╯

To get started, describe a task or try one of these commands:

/init - create an AGENTS.md file with instructions for Codex
/status - show current session configuration
/approvals - choose what Codex can do without approval
/model - choose what model and reasoning effort to use

> Model changed to gpt-5-codex

▌ Find and fix a bug in @filename

⏎ send ⇧⏎ newline ⌃T transcript ⌃C quit

---

Implement llm vtagent response token streaming for real-time response generation. Stream output in plain text during the agent's response. for tools call doesn't need streaming. make sure the streaming works with chat command. write tests for streaming functionality. update docs if necessary. update system prompt if needed. make sure streaming works with all models.

---

check docs/guides/codex-cloud-setup.md
and setup codex cloud environment for vtagent
https://developers.openai.com/codex/cloud/environments

---

integrates the Hotpath profiler (available at https://deepwiki.com/pawurb/hotpath), a lightweight tool for pinpointing exactly where your code spends its time. Follow these steps:

1. Set up a basic Rust application (e.g., a simple CLI tool or web server using Tokio for async runtime) that includes some intentionally blocking operations, such as synchronous file I/O, network requests, or CPU-intensive loops, to demonstrate performance bottlenecks.

2. Add Hotpath as a dependency in Cargo.toml and instrument the code using Hotpath's APIs: Wrap key functions or sections with profiling scopes (e.g., `hotpath::scope("function_name")`) to capture timing data for hot paths.

3. Run the profiled application and generate a report to identify blocking code—focus on sections showing high wall-clock time due to synchronous blocking (e.g., `std::fs::read_to_string` or `reqwest::blocking::get`).

4. Refactor the blocking code to make it non-blocking: Convert synchronous operations to asynchronous equivalents (e.g., use `tokio::fs::read_to_string` or `reqwest` async client), offload CPU-bound tasks to worker threads via `tokio::spawn_blocking`, and ensure the main event loop remains responsive.

5. Include a main function that runs both the original blocking version and the refactored async version, compares their performance via Hotpath reports, and outputs the results to verify improvements (e.g., reduced blocking time and better throughput).

Provide the complete, compilable code with comments explaining the integration, instrumentation, and fixes. Use Rust 1.75+ and handle errors gracefully with `anyhow` or `thiserror`.

===

search for "// TODO: " comments in the codebase and implement the missing functionality. prioritize tasks that enhance core features, improve performance, or fix bugs. ensure code quality with tests and documentation updates as needed.

---

fetch integrate speckit as new tools and command workflow for vtagent. this is a python package for spec-driven development. it can generate tests, code, and documentation from specifications. see below links for more details. fetch the website and read the docs. then implement the integration. write tests for the new functionality. update docs if necessary. update system prompt if needed. make sure vtagent can use speckit effectively. also add it to the setup script for easy installation.

https://github.com/github/spec-kit

--

refactor and modular runloop.rs, the file is too large. extract

--

Large Files in VTAgent Project

1.  vtagent-core/src/tools/registry.rs (~1,890 lines)

    -   Contains the main tool registry that coordinates all tools
    -   Implements error handling for tool execution
    -   Defines tool registration and execution mechanisms
    -   Handles tool policies and constraints

2.  src/agent/runloop.rs (~1,811 lines)

    -   Implements the main agent loop for both Gemini and other providers
    -   Handles context window management and trimming
    -   Manages conversation history and tool calling loops
    -   Includes prompt refinement and self-review features

3.  vtagent-core/src/tools/tree_sitter/analyzer.rs (~888 lines)
    -   Core tree-sitter analyzer for code parsing and analysis
    -   Supports multiple languages (Rust, Python, JavaScript, TypeScript, Go, Java)
    -   Extracts symbols, dependencies, and code metrics
    -   Handles syntax tree representation and diagnostics

Key Architectural Components

The VTAgent follows a modular architecture with several core components:

1.  Tool System: Centralized in registry.rs with a trait-based approach
2.  Agent Loop: Implemented in runloop.rs with support for multiple AI providers
3.  Language Analysis: Using tree-sitter parsers for code understanding
4.  Configuration Management: TOML-based configuration with sensible defaults
5.  Security: Comprehensive tool policy system with allow/deny lists

The largest files correspond to the most complex functionality - tool management, agent execution loop, and code analysis - which is typical
for a coding agent project.

--

check dead code vtagent-core/src/tools/registry/builtins.rs

---

also run cargo clippy to check for dead code and fix any warnings or errors. make sure the codebase is clean and follows best practices. make sure cleanup dead code, and unused files and redundant code and dependencies. ensure code quality and maintainability. write tests. update docs.

---

check all files under vtagent-core/src/tools/registry/ are not linked. this is important. we have refactor these files before. make sure they used. update docs if necessary. write tests if needed.

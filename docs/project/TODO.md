revert and remove speckit

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

refactor registry.rs to improve tool registration and management. create a more modular structure for adding new tools. ensure that each tool has clear metadata including name, description, parameters, and usage examples. implement a dynamic loading mechanism to allow tools to be added or removed without modifying the core codebase. write tests to verify the functionality of the registry and the correct loading of tools. update documentation to reflect the new structure and usage guidelines for adding tools.

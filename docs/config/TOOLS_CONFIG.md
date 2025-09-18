# Tools Configuration

This document describes the tools-related configuration in `vtagent.toml`.

- max_tool_loops: Maximum number of inner tool-call loops per user turn. Prevents infinite tool-calling cycles in interactive chat.
  - Configuration: `[tools].max_tool_loops` in `vtagent.toml`
  - Code default: defined in `vtagent-core/src/config/core/tools.rs`
  - Default: `100`

Example:

```toml
[tools]
default_policy = "prompt"
max_tool_loops = 100
```


Tool outputs are rendered with ANSI styles in the chat interface. Tools should return plain text.

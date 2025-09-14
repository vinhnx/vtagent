# Tools Configuration

This document describes the tools-related configuration in `vtagent.toml`.

- max_tool_loops: Maximum number of inner tool-call loops per user turn. Prevents infinite tool-calling cycles in interactive chat.
  - Location: `[tools].max_tool_loops`
  - Default: `6`
  - Env override: `VTAGENT_MAX_TOOL_LOOPS`

Example:

```toml
[tools]
default_policy = "prompt"
max_tool_loops = 6 # can be overridden by VTAGENT_MAX_TOOL_LOOPS
```


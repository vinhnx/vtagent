# VTAgent Configuration System

This document describes the comprehensive TOML-based configuration system introduced in VTAgent that provides granular control over agent behavior, tool policies, and command permissions.

## Overview

VTAgent now uses a sophisticated configuration system that replaces the previous JSON-based tool policies with a comprehensive TOML configuration format. This system provides:

- **Human-in-the-loop controls** for critical actions
- **Command allow/deny lists** for Unix commands
- **Tool-specific policies** (allow/prompt/deny)
- **Security settings** and file restrictions
- **Agent behavior customization**

## Configuration File Location

VTAgent looks for configuration in this order:

1. `vtagent.toml` in project root
2. `.vtagent/vtagent.toml` in project root
3. Default configuration (if no file found)

## Generating Configuration

### Initialize a New Project

The easiest way to get started is with the init command:

```bash
# Initialize VTAgent in your project
vtagent init

# Force overwrite existing files
vtagent init --force
```

This creates both `vtagent.toml` and `.vtagentgitignore` with sensible defaults.

### Generate Configuration Only

Create just the configuration file:

```bash
# Generate default vtagent.toml
vtagent config

# Generate to custom location
vtagent config --output my-config.toml

# Overwrite existing file
vtagent config --force
```

## Configuration Sections

### [agent] - Agent Behavior

Controls overall agent session behavior:

```toml
[agent]
max_conversation_turns = 1000      # Prevent runaway conversations
max_session_duration_minutes = 60  # Auto-terminate long sessions
verbose_logging = false             # Enable detailed logging
```

**Use Cases:**

- Set conversation limits for safety
- Control session timeouts
- Enable debugging output

### [tools] - Tool Execution Policies

Controls how the agent can use its built-in tools:

```toml
[tools]
default_policy = "prompt"  # Default: "allow", "prompt", or "deny"

[tools.policies]
# File operations
read_file = "allow"        # Execute automatically
write_file = "prompt"      # Ask user first
delete_file = "deny"       # Never allow

# Build operations
cargo_check = "allow"
cargo_build = "prompt"
cargo_test = "prompt"

# Terminal access
run_terminal_cmd = "prompt"
```

**Policy Types:**

- **allow**: Execute automatically without confirmation
- **prompt**: Ask user for confirmation before execution
- **deny**: Never allow execution

**Common Tool Categories:**

- File operations: `read_file`, `write_file`, `edit_file`, `delete_file`, `list_files`
- Search: `code_search`, `codebase_search`, `rp_search`
- Build: `cargo_check`, `cargo_build`, `cargo_test`, `cargo_fmt`, `cargo_clippy`

### [commands] - Unix Command Permissions

Controls which Unix commands can be executed via the terminal:

```toml
[commands]
# Commands that execute automatically without prompting
allow_list = [
    # Safe read-only operations
    "ls", "pwd", "cat", "grep", "find", "head", "tail", "wc",

    # Git read operations
    "git status", "git diff", "git log", "git show", "git branch",

    # Build checks
    "cargo check", "cargo clippy", "cargo fmt",

    # Version info
    "python --version", "node --version", "rustc --version"
]

# Commands that are always denied for security
deny_list = [
    "rm -rf", "sudo rm", "format", "fdisk", "mkfs",
    "shutdown", "reboot", "halt", "poweroff",
    "curl | sh", "wget | sh", "chmod 777", "passwd"
]

# Patterns requiring extra confirmation
dangerous_patterns = [
    "rm -f", "git reset --hard", "git clean -f",
    "docker system prune", "npm install -g", "pip install"
]
```

**Command Processing:**

1. Check `deny_list` first - if match, always block
2. Check `allow_list` - if match, execute automatically
3. Check `dangerous_patterns` - if match, show warning and require confirmation
4. For other commands, prompt based on `security.human_in_the_loop` setting

### [security] - Security Settings

Controls security behavior and restrictions:

```toml
[security]
human_in_the_loop = true              # Require confirmation for critical actions
confirm_destructive_actions = true    # Extra confirmation for dangerous operations
log_all_commands = true               # Log all executed commands
max_file_size_mb = 50                 # Maximum file size to process
allowed_file_extensions = [           # Restrict file types
    ".rs", ".toml", ".json", ".md", ".txt", ".yaml", ".yml",
    ".js", ".ts", ".py", ".go", ".java", ".cpp", ".c", ".h"
]
```

## Human-in-the-Loop Workflow

The configuration enables sophisticated user control over agent actions:

### Automatic Execution (Allow List)

Commands in the allow list execute without prompting:

```bash
VTAgent: [TOOL] run_terminal_cmd {"command": "git status"}
[ALLOWED] Command is in allow list: git status
```

### Standard Confirmation (Prompt Policy)

Tools/commands requiring confirmation:

```bash
VTAgent: [TOOL] write_file {"path": "src/main.rs", "content": "..."}
Confirm 'write_file': src/main.rs? [y/N] y
```

### Dangerous Command Warnings

Commands matching dangerous patterns get extra warnings:

```bash
VTAgent: [TOOL] run_terminal_cmd {"command": "rm -f old_file.txt"}
[WARNING] DANGEROUS command 'rm -f old_file.txt' - Are you sure? [y/N] y
```

### Denied Actions

Blocked commands are automatically denied:

```bash
VTAgent: [TOOL] run_terminal_cmd {"command": "rm -rf /"}
[TOOL ERROR] run_terminal_cmd - Denied by policy
```

## Example Configurations

### Development-Friendly Setup

For trusted development environments:

```toml
[tools]
default_policy = "allow"

[tools.policies]
delete_file = "prompt"      # Still prompt for deletions
run_terminal_cmd = "prompt" # Still prompt for commands

[commands]
allow_list = [
    "ls", "cat", "grep", "git status", "git diff", "cargo check",
    "cargo build", "cargo test", "npm run", "python", "node"
]

[security]
human_in_the_loop = false  # Less prompting for trusted environments
```

### Security-Focused Setup

For sensitive or production environments:

```toml
[tools]
default_policy = "deny"

[tools.policies]
read_file = "allow"
list_files = "allow"
code_search = "allow"
cargo_check = "allow"

[commands]
allow_list = ["ls", "pwd", "cat", "git status"]  # Minimal safe commands

[security]
human_in_the_loop = true
confirm_destructive_actions = true
max_file_size_mb = 10      # Smaller file limit
```

### CI/CD Integration

For automated environments:

```toml
[agent]
max_conversation_turns = 100       # Shorter sessions
max_session_duration_minutes = 15  # Quick timeouts

[tools]
default_policy = "allow"

[tools.policies]
run_terminal_cmd = "deny"           # No terminal access in CI

[commands]
allow_list = []                     # No direct command execution
```

## Migration from tool-policy.json

The old JSON format is deprecated but still supported. To migrate:

1. Generate new TOML config: `vtagent config`
2. Review and customize the generated file
3. Remove old `.vtagent/tool-policy.json`
4. Test with your workflow

### Mapping Old to New Format

| Old JSON | New TOML |
|----------|----------|
| `"tools.tool_name.allow": true` | `tools.policies.tool_name = "allow"` |
| `"tools.tool_name.allow": false` | `tools.policies.tool_name = "deny"` |
| `"default.allow": true` | `tools.default_policy = "allow"` |
| `"tools.run_terminal_cmd.args_policy.deny_substrings"` | `commands.deny_list` |

## Environment Variable Overrides

Certain settings can still be overridden with environment variables for compatibility:

- `VTAGENT_TOOL_PROMPT`: Comma-separated list of tools requiring prompts
- `VTAGENT_TOOL_DENY`: Comma-separated list of denied tools
- `VTAGENT_TOOL_AUTO`: Comma-separated list of auto-allowed tools

## Best Practices

### For Development Teams

1. **Commit vtagent.toml** to version control for team consistency
2. **Use allow lists** for common safe operations to improve workflow
3. **Set reasonable session limits** to prevent runaway conversations
4. **Enable logging** for debugging and audit trails

### For Security-Conscious Environments

1. **Start with restrictive policies** and gradually open up as needed
2. **Use deny lists** for dangerous command patterns
3. **Enable all security features** (`human_in_the_loop`, `confirm_destructive_actions`)
4. **Limit file extensions** to only what's necessary
5. **Set small file size limits** to prevent processing large files

### For CI/CD Pipelines

1. **Disable terminal commands** in automated environments
2. **Use shorter session timeouts** for efficiency
3. **Restrict to read-only operations** where possible
4. **Log all actions** for audit trails

## Troubleshooting

### Configuration Not Loading

- Check file location (`vtagent.toml` in project root)
- Verify TOML syntax with `toml-lint` or similar tool
- Use `--verbose` flag to see what config file is loaded

### Too Many Prompts

- Add common commands to `commands.allow_list`
- Change tool policies from "prompt" to "allow" for trusted tools
- Set `security.human_in_the_loop = false` for development

### Commands Being Blocked

- Check if command matches patterns in `commands.deny_list`
- Add to `commands.allow_list` if it's a safe operation
- Use `commands.dangerous_patterns` for commands needing extra confirmation

### Agent Not Following Configuration

- Ensure you're using the latest version of VTAgent
- Check that configuration file syntax is correct
- Verify the agent is loading the correct config file path

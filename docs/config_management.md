# VTAgent Configuration Management

## Overview

VTAgent now supports loading configuration files from multiple locations with the following priority order:

1. `./vtagent.toml` (workspace root)
2. `./.vtagent/vtagent.toml` (workspace .vtagent directory)
3. `~/.vtagent/vtagent.toml` (user home .vtagent directory)
4. Default configuration (built-in)

## Configuration File Locations

### Workspace Configuration
- **Primary location**: `./vtagent.toml` in your project root
- **Alternative location**: `./.vtagent/vtagent.toml` in your project's .vtagent directory

### Global Configuration
- **Location**: `~/.vtagent/vtagent.toml` in your user home directory
- **Purpose**: Global settings that apply to all VTAgent projects

## Creating Configuration Files

### Using the CLI

Create a configuration file in your current workspace:
```bash
vtagent config
```

Create a configuration file in a specific location:
```bash
vtagent config --output ./custom-config.toml
```

Create a global configuration file in your home directory:
```bash
vtagent config --global
```

### Manual Creation

You can also create the configuration file manually. Here's an example configuration:

```toml
# VTAgent Configuration File

[agent]
default_model = "qwen/qwen3-4b-2507"
provider = "lmstudio"
max_conversation_turns = 150
reasoning_effort = "medium"

[security]
human_in_the_loop = true

[multi_agent]
enabled = false
use_single_model = true
orchestrator_model = "qwen/qwen3-4b-2507"
executor_model = "qwen/qwen3-4b-2507"
max_concurrent_subagents = 3
context_sharing_enabled = true
task_timeout_seconds = 300

[lmstudio]
base_url = "http://localhost:1234/v1"
single_agent_model = "qwen/qwen3-4b-2507"
orchestrator_model = "qwen/qwen3-4b-2507"
subagent_model = "qwen/qwen3-4b-2507"
enable_multi_agent = true
connection_timeout_seconds = 30

[tools]
default_policy = "prompt"

[commands]
allow_list = ["ls", "pwd", "cat", "grep", "git status", "git diff"]

[pty]
enabled = true
default_rows = 24
default_cols = 80
max_sessions = 10
command_timeout_seconds = 300
```

## Best Practices

1. **Workspace-specific settings**: Use `./vtagent.toml` for project-specific configurations
2. **Global settings**: Use `~/.vtagent/vtagent.toml` for settings that should apply to all projects
3. **Sensitive information**: Never commit configuration files containing API keys to version control
4. **Environment variables**: Prefer environment variables for sensitive information like API keys

## Configuration Priority

When loading configuration, VTAgent follows this priority order:

1. **Workspace root** (`./vtagent.toml`) - Highest priority
2. **Workspace .vtagent** (`./.vtagent/vtagent.toml`) - Medium priority
3. **Home directory** (`~/.vtagent/vtagent.toml`) - Lower priority
4. **Defaults** - Lowest priority (built-in default values)

This allows you to have:
- Global defaults in your home directory
- Project-specific overrides in your workspace
- Temporary overrides in your current directory
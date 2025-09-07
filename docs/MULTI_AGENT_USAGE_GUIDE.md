# VTAgent Multi-Agent System Usage Guide

## Overview

VTAgent now supports both single-agent and multi-agent execution modes. The multi-agent system enables sophisticated problem-solving through strategic delegation, where an Orchestrator coordinates specialized Explorer and Coder agents to tackle complex development tasks.

## Quick Start

### Enable Multi-Agent Mode

Add to your `vtagent.toml`:

```toml
[multi_agent]
enabled = true
execution_mode = "auto"  # or "multi" to force multi-agent mode
orchestrator_model = "gemini-1.5-pro"
subagent_model = "gemini-1.5-flash"
```

### Basic Usage Examples

**Simple Task (Auto Mode):**
```bash
vtagent chat "Fix the typo in README.md"
# Automatically uses single-agent mode for simple tasks
```

**Complex Task (Auto Mode):**
```bash
vtagent chat "Add authentication system to the web app"
# Automatically switches to multi-agent mode for complex tasks
```

**Force Multi-Agent Mode:**
```bash
vtagent chat "Refactor the database layer" --multi-agent
# Or set execution_mode = "multi" in config
```

## How It Works

### Agent Types

#### 1. Orchestrator Agent
- **Role**: Strategic coordinator and persistent intelligence layer
- **Capabilities**: Task delegation, context management, progress tracking
- **Restrictions**: Cannot read/write files directly - purely architectural
- **Tools**: `task_create`, `launch_subagent`, `add_context`, `context_search`, `finish`

#### 2. Explorer Agent
- **Role**: Read-only investigation and verification specialist
- **Capabilities**: File reading, system analysis, test execution, code inspection
- **Restrictions**: Cannot modify files (strictly read-only)
- **Tools**: `read_file`, `grep_search`, `run_command`, `project_overview`, `ast_grep_search`

#### 3. Coder Agent
- **Role**: Implementation specialist with full write access
- **Capabilities**: Code changes, file creation/modification, system configuration
- **Restrictions**: None - full system access
- **Tools**: All available tools including `write_file`, `edit_file`, `create_file`

### Workflow Patterns

#### Standard Multi-Step Pattern
```
User Request → Orchestrator Analysis
    ↓
Explorer Investigation (understand current state)
    ↓
Coder Implementation (make changes)
    ↓
Explorer Verification (confirm success)
    ↓
Orchestrator Reports Completion
```

#### Time-Efficient Execution
- **Familiar environments**: Coder → Explorer → Finish (3 steps)
- **New environments**: Explorer → Coder → Explorer → Finish (4 steps)
- **Complex tasks**: Multiple targeted cycles as needed

## Configuration

### Complete Multi-Agent Configuration

```toml
[multi_agent]
# Enable multi-agent system
enabled = true

# Execution mode: "single", "multi", or "auto"
execution_mode = "auto"

# Models for different agent types
orchestrator_model = "gemini-1.5-pro"
subagent_model = "gemini-1.5-flash"

# Performance settings
max_concurrent_subagents = 3
verification_strategy = "always"  # "always", "complex_only", "never"
delegation_strategy = "adaptive"  # "adaptive", "conservative", "aggressive"

# Context store settings
context_store_enabled = true

[multi_agent.context_store]
max_contexts = 1000
auto_cleanup_days = 7
enable_persistence = true
compression_enabled = true
storage_dir = ".vtagent/contexts"

# Agent-specific configurations
[multi_agent.agents.orchestrator]
allowed_tools = ["task_create", "launch_subagent", "add_context", "context_search", "finish"]
restricted_tools = ["read_file", "write_file", "run_command"]
max_task_time_seconds = 300
max_context_window = 32000

[multi_agent.agents.explorer]
allowed_tools = ["read_file", "grep_search", "run_command", "project_overview"]
restricted_tools = ["write_file", "edit_file", "delete_file"]
max_task_time_seconds = 300
max_context_window = 32000

[multi_agent.agents.coder]
allowed_tools = ["*"]  # Full access
restricted_tools = []
max_task_time_seconds = 600
max_context_window = 32000
```

### Execution Mode Options

#### "auto" (Recommended)
- Automatically chooses single or multi-agent based on task complexity
- Simple tasks: single-agent execution
- Complex tasks: multi-agent coordination
- Best balance of speed and capability

#### "single"
- Forces single-agent mode for all tasks
- Faster for simple operations
- Compatible with existing workflows
- No agent coordination overhead

#### "multi"
- Forces multi-agent mode for all tasks
- Maximum capability and verification
- Slower for simple tasks
- Best for complex development work

#### "multi_adaptive"
- Advanced mode that learns from task patterns
- Adapts delegation strategy based on success rates
- Optimizes for both speed and quality
- Requires more computational resources

## Advanced Features

### Context Store Management

The context store enables sophisticated knowledge sharing:

```bash
# View accumulated contexts
vtagent contexts list

# Search contexts by tag
vtagent contexts search --tag "authentication"

# Clear old contexts
vtagent contexts cleanup --days 7
```

### Task Management

Monitor and control multi-agent coordination:

```bash
# View active tasks
vtagent tasks list

# Check task status
vtagent tasks status <task_id>

# Cancel running task
vtagent tasks cancel <task_id>
```

### Verification Strategies

#### "always" (Default)
- Every Coder implementation is verified by Explorer
- Maximum quality assurance
- Slower execution but highest reliability

#### "complex_only"
- Only verifies complex implementations
- Balances speed and quality
- Good for mixed workloads

#### "never"
- No automatic verification
- Fastest execution
- Suitable for trusted environments only

### Delegation Strategies

#### "adaptive" (Default)
- Learns from past successes and failures
- Adjusts task scoping based on agent capabilities
- Optimizes delegation patterns over time

#### "conservative"
- Smaller, well-scoped tasks with more verification
- Higher quality but slower execution
- Best for critical systems

#### "aggressive"
- Larger tasks with minimal verification
- Faster execution but higher risk
- Suitable for development environments

## Use Cases

### Perfect for Multi-Agent Mode

**Complex Implementation Tasks:**
```bash
vtagent chat "Implement OAuth2 authentication with JWT tokens"
vtagent chat "Add GraphQL API with type-safe resolvers"
vtagent chat "Refactor database layer for better performance"
```

**System-Wide Changes:**
```bash
vtagent chat "Migrate from REST to GraphQL across the application"
vtagent chat "Add comprehensive error handling throughout the codebase"
vtagent chat "Implement event-driven architecture"
```

**Investigation and Analysis:**
```bash
vtagent chat "Analyze the current authentication system and suggest improvements"
vtagent chat "Find and fix all performance bottlenecks"
vtagent chat "Audit security vulnerabilities across the codebase"
```

### Better for Single-Agent Mode

**Simple Fixes:**
```bash
vtagent chat "Fix typo in README.md"
vtagent chat "Update version number in package.json"
vtagent chat "Add missing import statement"
```

**Quick Information:**
```bash
vtagent chat "What's the current test coverage?"
vtagent chat "Show me the project structure"
vtagent chat "List all TODO comments"
```

## Troubleshooting

### Common Issues

**Multi-agent mode not activating:**
- Check `multi_agent.enabled = true` in config
- Ensure execution_mode is not set to "single"
- Verify API keys are properly configured

**Tasks taking too long:**
- Reduce `max_concurrent_subagents`
- Increase `max_task_time_seconds` for complex tasks
- Consider using "aggressive" delegation strategy

**Context store filling up:**
- Reduce `max_contexts` setting
- Enable automatic cleanup with shorter `auto_cleanup_days`
- Manually clean contexts with `vtagent contexts cleanup`

**Agent coordination failures:**
- Check network connectivity for API calls
- Verify model availability (orchestrator vs subagent models)
- Review agent-specific tool restrictions

### Performance Tuning

**For Speed:**
```toml
execution_mode = "single"  # or use selective multi-agent
verification_strategy = "never"
delegation_strategy = "aggressive"
max_concurrent_subagents = 5
```

**For Quality:**
```toml
execution_mode = "multi"
verification_strategy = "always"
delegation_strategy = "conservative"
orchestrator_model = "gemini-1.5-pro"  # Best model for coordination
```

**For Balance:**
```toml
execution_mode = "auto"
verification_strategy = "complex_only"
delegation_strategy = "adaptive"
subagent_model = "gemini-1.5-flash"  # Fast subagents
```

## Best Practices

### Task Design
- Provide clear, specific requirements
- Include relevant context about the system
- Specify verification criteria when possible
- Break very large tasks into logical phases

### Context Management
- Use descriptive context IDs (snake_case)
- Tag contexts appropriately for searchability
- Regularly clean up outdated contexts
- Monitor context store size

### Agent Coordination
- Trust the Orchestrator's delegation decisions
- Allow sufficient time for complex tasks
- Review verification results carefully
- Use single-agent mode for simple operations

### Configuration Optimization
- Start with default settings
- Monitor performance and adjust gradually
- Use "auto" mode for most use cases
- Consider environment-specific configurations

## Integration with Existing Workflows

### CI/CD Integration
```yaml
# GitHub Actions example
- name: Run VTAgent Analysis
  run: |
    vtagent chat "Analyze code quality and suggest improvements" \
      --config .github/vtagent-ci.toml \
      --execution-mode multi \
      --verification-strategy always
```

### IDE Integration
```bash
# VS Code task example
{
  "label": "VTAgent: Refactor Selection",
  "type": "shell",
  "command": "vtagent",
  "args": ["chat", "Refactor the selected code for better maintainability"],
  "options": {
    "cwd": "${workspaceFolder}"
  }
}
```

### Team Usage
```toml
# Team configuration template
[multi_agent]
enabled = true
execution_mode = "auto"
verification_strategy = "always"  # Ensure quality for team projects

[multi_agent.context_store]
enable_persistence = true  # Share knowledge across team members
storage_dir = ".vtagent/shared-contexts"
```

The multi-agent system transforms vtagent from a capable assistant into a sophisticated development platform that can handle complex, multi-step projects with unprecedented reliability and intelligence.

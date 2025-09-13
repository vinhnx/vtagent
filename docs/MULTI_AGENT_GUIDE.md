# VTAgent Multi-Agent System Guide

## Overview

VTAgent now supports both single-agent and multi-agent execution modes. The multi-agent system enables sophisticated problem-solving through strategic delegation, where an Orchestrator coordinates specialized Explorer and Coder agents to tackle complex development tasks.

## Architecture Components

### 1. Orchestrator Agent (Lead Architect)

The Orchestrator acts as the strategic coordinator and persistent intelligence layer:

**Role:**
- Strategic coordinator and persistent intelligence layer
- Task decomposition and context management
- Subagent delegation with precise task specifications
- Architectural decision making without direct code manipulation
- Verification coordination and compound intelligence building

**Capabilities:**
- Task creation and delegation
- Context store management
- Progress tracking and failure recovery
- Strategic planning and workflow orchestration
- Time-conscious task distribution

**Restrictions:**
- Cannot read or modify code directly
- Operates purely at architectural level
- Must delegate all implementation work

### 2. Explorer Agent

The Explorer specializes in read-only investigation and verification:

**Role:**
- Read-only investigation and verification specialist
- System exploration and code analysis
- Implementation verification and testing
- Knowledge artifact creation

**Capabilities:**
- File reading and content analysis
- System inspection and configuration discovery
- Test execution and result verification
- Code analysis and pattern recognition
- Temporary script creation for validation

**Restrictions:**
- Cannot modify existing files
- Cannot create permanent artifacts
- Focuses on investigation and analysis

### 3. Coder Agent

The Coder specializes in implementation and code modification:

**Role:**
- Implementation specialist with full code access
- Code creation, modification, and refactoring
- Test writing and validation
- Documentation and artifact creation

**Capabilities:**
- Full file system access for code modification
- Code generation and refactoring
- Test creation and execution
- Documentation writing
- Build and deployment tasks

**Restrictions:**
- Should not perform extensive investigation
- Must coordinate with Orchestrator for complex tasks
- Should verify work with Explorer when appropriate

## Quick Start

### Enable Multi-Agent Mode

Add to your `vtagent.toml`:

```toml
[multi_agent]
enabled = true
execution_mode = "auto"  # or "multi" to force multi-agent mode
orchestrator_model = "gemini-2.5-flash"
subagent_model = "gemini-2.5-flash-lite"
max_concurrent_subagents = 3
verification_strategy = "always"
delegation_strategy = "adaptive"
context_store_enabled = true

[multi_agent.context_store]
max_contexts = 1000
auto_cleanup_days = 7
enable_persistence = true
storage_dir = ".vtagent/contexts"

[multi_agent.agents.orchestrator]
allowed_tools = ["task_create", "launch_subagent", "add_context", "context_search", "task_status", "finish"]
restricted_tools = ["read_file", "write_file", "edit_file", "run_terminal_cmd", "list_files"]

[multi_agent.agents.explorer]
allowed_tools = ["read_file", "list_files", "code_search", "codebase_search", "rp_search", "run_terminal_cmd", "cargo_check", "cargo_test"]
restricted_tools = ["write_file", "edit_file", "delete_file"]

[multi_agent.agents.coder]
allowed_tools = ["*"]  # Full access
restricted_tools = []
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

### Execution Modes

#### Single-Agent Mode
- Traditional VTAgent behavior
- Direct interaction with one LLM
- Best for simple, straightforward tasks
- Fast execution with minimal overhead

#### Multi-Agent Mode
- Orchestrator coordinates specialized agents
- Strategic delegation for complex tasks
- Verification workflows ensure quality
- Context sharing between agents

#### Auto Mode (Recommended)
- Automatically chooses execution mode based on task complexity
- Simple tasks → Single-agent
- Complex tasks → Multi-agent
- Optimal balance of speed and capability

### Task Complexity Detection

The system automatically detects task complexity based on:

**Simple Tasks:**
- Single file modifications
- Basic code changes
- Documentation updates
- Configuration changes

**Complex Tasks:**
- Multi-file refactoring
- System architecture changes
- New feature implementation
- Integration tasks
- Performance optimization

## Agent Communication Flow

### Typical Multi-Agent Workflow

1. **User Request** → Orchestrator receives task
2. **Task Analysis** → Orchestrator decomposes complex tasks
3. **Strategic Planning** → Orchestrator creates execution plan
4. **Agent Delegation** → Orchestrator assigns tasks to appropriate agents
5. **Parallel Execution** → Explorer and Coder agents work simultaneously
6. **Context Sharing** → Agents share findings and progress
7. **Verification** → Explorer verifies Coder's work
8. **Quality Assurance** → Orchestrator reviews final results
9. **Final Delivery** → Orchestrator presents completed work

### Communication Patterns

#### Orchestrator → Explorer
```
"Investigate the current authentication system and identify security vulnerabilities"
```

#### Orchestrator → Coder
```
"Implement JWT-based authentication with the following specifications..."
```

#### Explorer → Orchestrator
```
"Found 3 security issues: 1) No password hashing, 2) Session management weak, 3) CSRF vulnerability"
```

#### Coder → Orchestrator
```
"Authentication system implemented with JWT tokens, bcrypt password hashing, and CSRF protection"
```

## Context Store System

### Purpose
The Context Store provides persistent knowledge management across agent interactions:

- **Knowledge Persistence**: Maintains context across sessions
- **Cross-Agent Sharing**: Enables information flow between agents
- **Historical Reference**: Tracks decisions and implementations
- **Performance Optimization**: Reduces redundant investigation

### Context Types

#### Environmental Context
- Project structure and dependencies
- Development environment setup
- Configuration and settings

#### Diagnostic Context
- Error analysis and debugging information
- Performance metrics and bottlenecks
- System health and status

#### Implementation Context
- Code patterns and architectural decisions
- Implementation progress and status
- Technical debt and refactoring opportunities

#### Analysis Context
- Code quality metrics and analysis
- Security assessments and findings
- Performance optimization opportunities

## Configuration Options

### Core Multi-Agent Settings

```toml
[multi_agent]
enabled = true                    # Enable multi-agent system
execution_mode = "auto"          # auto, single, multi
orchestrator_model = "gemini-2.5-flash"    # Model for orchestrator
subagent_model = "gemini-2.5-flash-lite"   # Model for subagents
max_concurrent_subagents = 3     # Maximum parallel agents
verification_strategy = "always" # always, on_error, never
delegation_strategy = "adaptive" # adaptive, round_robin, load_balance
context_store_enabled = true     # Enable context persistence
```

### Agent-Specific Configuration

#### Orchestrator Configuration
```toml
[multi_agent.agents.orchestrator]
allowed_tools = ["task_create", "launch_subagent", "add_context", "context_search", "task_status", "finish"]
restricted_tools = ["read_file", "write_file", "edit_file", "run_terminal_cmd", "list_files"]
max_tasks_per_session = 10
task_timeout_minutes = 30
```

#### Explorer Configuration
```toml
[multi_agent.agents.explorer]
allowed_tools = ["read_file", "list_files", "code_search", "codebase_search", "rp_search", "run_terminal_cmd", "cargo_check", "cargo_test"]
restricted_tools = ["write_file", "edit_file", "delete_file"]
max_investigation_time_minutes = 15
verification_depth = "comprehensive"  # basic, standard, comprehensive
```

#### Coder Configuration
```toml
[multi_agent.agents.coder]
allowed_tools = ["*"]  # Full access to all tools
restricted_tools = []  # No restrictions
code_style_preference = "rust_fmt"  # rust_fmt, custom, none
testing_required = true
documentation_required = true
```

### Context Store Configuration

```toml
[multi_agent.context_store]
max_contexts = 1000              # Maximum stored contexts
auto_cleanup_days = 7            # Auto-cleanup after days
enable_persistence = true        # Persist across sessions
storage_dir = ".vtagent/contexts" # Storage location
compression_enabled = true       # Compress stored contexts
max_context_size_kb = 1024       # Maximum context size
```

## Advanced Features

### Task Management System

#### Task Types
- **Analysis Tasks**: Code investigation and requirements gathering
- **Implementation Tasks**: Code creation and modification
- **Verification Tasks**: Testing and validation
- **Documentation Tasks**: Documentation creation and updates

#### Task Dependencies
- **Sequential Dependencies**: Task B must complete after Task A
- **Parallel Dependencies**: Tasks can run simultaneously
- **Conditional Dependencies**: Tasks execute based on conditions

#### Task Prioritization
- **Critical**: System-breaking issues, security vulnerabilities
- **High**: Important features, performance issues
- **Normal**: Standard development tasks
- **Low**: Minor improvements, documentation

### Verification Workflows

#### Quality Assurance Levels
- **Basic**: Syntax checking and compilation
- **Standard**: Unit tests and integration tests
- **Comprehensive**: Full test suite, code review, security scan

#### Verification Criteria
- **Completeness**: All requirements implemented
- **Correctness**: Code functions as expected
- **Quality**: Code follows best practices
- **Security**: No security vulnerabilities introduced

### Performance Monitoring

#### Metrics Tracked
- **Task Completion Time**: Time to complete individual tasks
- **Agent Utilization**: How effectively agents are used
- **Context Store Performance**: Retrieval and storage efficiency
- **Error Rates**: Frequency of task failures and retries

#### Optimization Strategies
- **Load Balancing**: Distribute work across available agents
- **Caching**: Reuse common context and analysis results
- **Parallelization**: Execute independent tasks simultaneously
- **Resource Management**: Optimize agent allocation

## Troubleshooting

### Common Issues

#### Multi-Agent Not Activating
**Problem**: System stays in single-agent mode for complex tasks
**Solutions**:
- Check `execution_mode = "auto"` in configuration
- Verify `enabled = true` in multi_agent section
- Ensure orchestrator_model is properly configured

#### Agent Communication Failures
**Problem**: Agents fail to coordinate or share context
**Solutions**:
- Check context_store configuration
- Verify agent tool permissions
- Ensure proper network connectivity for API calls

#### Performance Issues
**Problem**: Multi-agent mode slower than expected
**Solutions**:
- Reduce `max_concurrent_subagents`
- Use faster models for subagents
- Enable context caching
- Optimize task decomposition

#### Context Store Problems
**Problem**: Context not persisting or retrieving correctly
**Solutions**:
- Check storage directory permissions
- Verify `enable_persistence = true`
- Clear corrupted context store
- Check available disk space

### Debug Commands

```bash
# Check multi-agent status
vtagent status --multi-agent

# View active tasks
vtagent tasks --list

# Clear context store
vtagent context --clear

# View agent logs
vtagent logs --agent orchestrator
```

## Best Practices

### Task Planning
1. **Break Down Complex Tasks**: Divide large tasks into manageable subtasks
2. **Define Clear Requirements**: Provide detailed specifications for agents
3. **Establish Success Criteria**: Define what constitutes task completion
4. **Plan for Verification**: Include testing and validation steps

### Agent Utilization
1. **Right Agent for the Job**: Use Explorer for investigation, Coder for implementation
2. **Parallel Processing**: Leverage multiple agents for independent tasks
3. **Context Sharing**: Ensure agents have access to necessary information
4. **Progress Monitoring**: Track agent progress and intervene when needed

### Performance Optimization
1. **Model Selection**: Choose appropriate models for task complexity
2. **Caching Strategy**: Enable context caching for repeated operations
3. **Resource Limits**: Set appropriate concurrency and timeout limits
4. **Load Balancing**: Distribute work evenly across available agents

### Quality Assurance
1. **Verification Workflows**: Always enable verification for critical tasks
2. **Code Reviews**: Have agents review each other's work
3. **Testing**: Include comprehensive testing in all implementations
4. **Documentation**: Ensure all changes are properly documented

## Examples

### Complex Feature Implementation

**Task**: "Implement user authentication system with JWT tokens"

**Multi-Agent Execution**:
1. **Orchestrator**: Analyzes requirements, creates implementation plan
2. **Explorer**: Investigates current codebase, identifies integration points
3. **Coder**: Implements authentication middleware, user models, login endpoints
4. **Explorer**: Verifies implementation, runs security tests
5. **Orchestrator**: Reviews final implementation, ensures requirements met

### System Refactoring

**Task**: "Refactor monolithic service into microservices"

**Multi-Agent Execution**:
1. **Orchestrator**: Analyzes current architecture, plans microservice boundaries
2. **Explorer**: Maps dependencies and data flows
3. **Coder**: Creates new microservice structure, implements service extraction
4. **Explorer**: Tests service interactions, validates functionality
5. **Orchestrator**: Coordinates deployment and migration strategy

### Performance Optimization

**Task**: "Optimize database query performance"

**Multi-Agent Execution**:
1. **Orchestrator**: Identifies performance bottlenecks, plans optimization strategy
2. **Explorer**: Analyzes current queries, identifies slow operations
3. **Coder**: Implements query optimizations, adds indexes, refactors code
4. **Explorer**: Benchmarks improvements, validates performance gains
5. **Orchestrator**: Ensures optimizations don't break functionality

## Summary

The VTAgent multi-agent system provides:

- **Strategic Coordination**: Intelligent task delegation and planning
- **Specialized Agents**: Purpose-built agents for different roles
- **Quality Assurance**: Built-in verification and testing workflows
- **Scalable Architecture**: Handles both simple and complex tasks
- **Performance Optimization**: Efficient resource utilization and caching
- **Flexible Configuration**: Customizable for different use cases

The multi-agent system transforms VTAgent from a single-purpose coding assistant into a sophisticated development coordination platform capable of handling complex, multi-step development tasks with professional quality and efficiency.

---

*For technical implementation details, see [MULTI_AGENT_IMPLEMENTATION.md](MULTI_AGENT_IMPLEMENTATION.md)*

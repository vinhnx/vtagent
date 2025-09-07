# VTAgent Multi-Agent Architecture

## Overview

VTAgent now supports both single-agent and multi-agent execution modes. The multi-agent system implements a strategic delegation pattern inspired by successful agent architectures, enabling sophisticated problem-solving through specialized agent coordination.

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
- Strictly read-only operations
- Can create temporary files only in `/tmp` or similar

### 3. Coder Agent

The Coder handles all implementation and modification tasks:

**Role:**
- Implementation specialist with full write access
- Code transformation and system modification
- Feature implementation and bug fixes
- Testing and validation of changes

**Capabilities:**
- Full file operations (read/write/edit)
- Code creation and modification
- System configuration changes
- Build and test execution
- Comprehensive implementation validation

**Restrictions:**
- Must focus on specific, well-defined tasks
- Should verify implementations before completion
- Must report structured contexts about changes made

## Core Systems

### Context Store

The Context Store enables sophisticated knowledge sharing between agents:

**Purpose:**
- Persistent knowledge management across agent interactions
- Elimination of redundant discovery work
- Compound intelligence through knowledge accumulation
- Focused context injection for specific tasks

**Structure:**
```rust
pub struct ContextStore {
    contexts: HashMap<String, ContextItem>,
    session_id: String,
    created_at: SystemTime,
}

pub struct ContextItem {
    id: String,
    content: String,
    created_by: AgentType,
    created_at: SystemTime,
    tags: Vec<String>,
}
```

**Features:**
- Immutable context items with unique IDs
- Agent attribution and timestamps
- Tag-based organization and retrieval
- Automatic persistence across tasks

### Task Management System

Sophisticated task coordination and progress tracking:

**Components:**
- Task creation and scheduling
- Progress monitoring and status tracking
- Failure recovery and retry logic
- Dependency management between tasks
- Audit trail maintenance

**Task Types:**
- **Exploration Tasks**: System investigation and verification
- **Implementation Tasks**: Code changes and feature development
- **Verification Tasks**: Testing and validation of changes
- **Synthesis Tasks**: Knowledge consolidation and planning

### Agent Coordination Patterns

#### Standard Workflow Pattern
```
1. Orchestrator analyzes user request
2. Orchestrator launches Explorer for environment understanding
3. Orchestrator creates implementation tasks with complete context
4. Orchestrator launches Coder with focused specifications
5. Orchestrator launches Explorer for verification
6. Orchestrator synthesizes results and reports completion
```

#### Time-Conscious Execution
- Front-loaded precision in task specifications
- Complete context provision to avoid rediscovery
- Explicit expected outcomes for all tasks
- Tight scoping with clear boundaries
- Strategic verification rather than exhaustive testing

## Implementation Strategy

### Phase 1: Core Infrastructure
- [ ] Context Store implementation
- [ ] Task Management system
- [ ] Agent type definitions and factories
- [ ] Basic orchestration hub

### Phase 2: Agent Specializations
- [ ] Orchestrator agent with delegation capabilities
- [ ] Explorer agent with read-only tool restrictions
- [ ] Coder agent with full tool access
- [ ] Agent-specific system prompts and behaviors

### Phase 3: Coordination Features
- [ ] Multi-agent task orchestration
- [ ] Context sharing and injection
- [ ] Progress tracking and verification workflows
- [ ] Failure recovery and retry mechanisms

### Phase 4: Advanced Features
- [ ] Intelligent task decomposition
- [ ] Adaptive delegation strategies
- [ ] Performance optimization and caching
- [ ] Advanced workflow patterns

## Configuration

### Multi-Agent Mode Configuration
```toml
[agent]
mode = "multi"  # "single" | "multi"
orchestrator_model = "gemini-1.5-pro"
subagent_model = "gemini-1.5-flash"
max_concurrent_subagents = 3
context_store_enabled = true

[multi_agent]
enable_task_management = true
enable_context_sharing = true
verification_strategy = "always"  # "always" | "complex_only" | "never"
delegation_strategy = "adaptive"  # "adaptive" | "conservative" | "aggressive"

[context_store]
max_contexts = 1000
auto_cleanup_days = 7
enable_persistence = true
compression_enabled = true
```

### Agent-Specific Tool Restrictions
```toml
[agents.orchestrator]
allowed_tools = ["task_create", "launch_subagent", "add_context", "finish"]
restricted_tools = ["read_file", "write_file", "run_command"]

[agents.explorer]
allowed_tools = ["read_file", "grep_search", "run_command", "file_metadata"]
restricted_tools = ["write_file", "edit_file", "delete_file"]

[agents.coder]
allowed_tools = ["*"]  # Full access
restricted_tools = []
```

## Benefits

### Compound Intelligence
- Each agent action builds meaningfully on previous discoveries
- Knowledge accumulation prevents redundant work
- Strategic verification patterns ensure quality
- Context sharing enables sophisticated problem-solving

### Specialized Expertise
- Explorer agents optimize for investigation and verification
- Coder agents focus on implementation quality
- Orchestrator agents provide strategic oversight
- Clear separation of concerns improves outcomes

### Robust Execution
- Multi-step verification prevents errors
- Failure recovery and retry mechanisms
- Strategic delegation based on task complexity
- Time-conscious execution with focused specifications

### Scalability
- Complex problems become tractable through decomposition
- Parallel subagent execution where appropriate
- Efficient context management reduces cognitive load
- Strategic planning enables systematic progress

## Usage Examples

### Simple Task (Single Agent Mode)
```bash
vtagent chat "Fix the typo in README.md"
# Uses single agent for straightforward tasks
```

### Complex Task (Multi-Agent Mode)
```bash
vtagent chat "Add authentication system to the web app"
# Orchestrator will:
# 1. Launch explorer to understand current architecture
# 2. Plan authentication implementation strategy
# 3. Launch coder to implement auth components
# 4. Launch explorer to verify implementation
# 5. Report completion with verification results
```

### Explicit Multi-Agent Mode
```bash
vtagent multi-agent "Refactor the database layer for better performance"
# Forces multi-agent coordination for complex refactoring
```

## Comparison with Single-Agent Mode

| Aspect | Single-Agent | Multi-Agent |
|--------|-------------|-------------|
| **Task Complexity** | Simple to medium | Medium to complex |
| **Execution Speed** | Faster for simple tasks | Optimized for complex tasks |
| **Quality Assurance** | Basic validation | Multi-step verification |
| **Context Management** | Conversation history | Persistent context store |
| **Failure Recovery** | Basic retry | Strategic adaptation |
| **Resource Usage** | Lower token usage | Higher token usage, better outcomes |

## Future Enhancements

### Advanced Delegation
- Dynamic agent spawning based on workload
- Specialized agent types for specific domains
- Learning from delegation patterns for optimization

### Enhanced Context Management
- Semantic context search and retrieval
- Automatic context relevance scoring
- Context compression and summarization

### Workflow Intelligence
- Pattern recognition for common workflows
- Predictive task creation based on context
- Adaptive strategies based on success patterns

### Integration Capabilities
- REST API for external orchestration
- Webhook notifications for task completion
- Integration with CI/CD pipelines and development tools

This multi-agent architecture transforms vtagent from a capable single-agent system into a sophisticated coordination platform capable of handling complex, multi-step development tasks with unprecedented reliability and intelligence.

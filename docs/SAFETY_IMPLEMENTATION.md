# VTAgent Safety Implementation

## Overview

This implementation adds critical safety and efficiency controls to VTAgent, ensuring user explicit confirmation for expensive operations and defaulting to single-agent mode for optimal resource usage.

## Key Safety Features Implemented

### 1. **Model Usage Safety**

**Critical Safety Requirement**: Before attempting to switch to the most capable model ("gemini-2.5-pro"), always ask for explicit user confirmation.

**Implementation**:
- Added `SafetyValidator::validate_model_usage()` function
- Automatically detects when Gemini 2.5 Pro is requested
- Shows clear warning about cost and performance implications:
  ```
  ⚠️  Model Upgrade Required
  Current model: gemini-2.5-flash-lite
  Requested model: gemini-2.5-pro

  The Gemini 2.5 Pro model is the most capable but also:
  • More expensive per token
  • Slower response times
  • Higher resource usage

  Do you want to proceed with the more capable (and expensive) Gemini 2.5 Pro model?
  ```
- Falls back to default model if user declines

### 2. **Agent Mode Safety**

**Critical Efficiency Requirement**: The orchestrator must default to using one single simple coder agent for all tasks. Switch to a multi-agent setup only for complex tasks, and always ask for explicit user confirmation.

**Implementation**:
- **Changed default**: `ENABLE_MULTI_AGENT` now defaults to `false` (single-agent mode)
- Added `SafetyValidator::validate_agent_mode()` function
- Automatic task complexity assessment:
  ```
  📊 Task Complexity Assessment
  Task: [user's task description]

  How would you classify this task's complexity?
  > Simple (single file edit, basic question, straightforward task)
    Moderate (multiple files, refactoring, testing)
    Complex (architecture changes, cross-cutting concerns, large refactoring)
  ```
- Smart recommendations based on complexity:
  - **Simple/Moderate**: Recommends single coder agent
  - **Complex**: Recommends multi-agent with confirmation

### 3. **New CLI Safety Options**

Added safety-related command line flags:

```bash
# Force multi-agent mode (still requires confirmation)
vtagent chat --force-multi-agent

# Skip safety confirmations (use with caution)
vtagent chat --skip-confirmations

# Both flags can be combined
vtagent chat --force-multi-agent --skip-confirmations
```

### 4. **Comprehensive Safety Configuration Display**

When safety validations are triggered, users see clear summaries:

```
🛡️  Safety Configuration Summary
Model: gemini-2.5-flash
Agent Mode: SingleCoder
Task: Interactive coding session

🚀 Using fast model:
• Quick responses
• Most cost-effective
• Good for simple tasks

🔧 Single Coder Agent:
• Direct and efficient
• Lower API costs
• Faster task completion
• Best for most development tasks
```

## Implementation Details

### Files Added/Modified

1. **New Files**:
   - `vtagent-core/src/user_confirmation.rs` - User interaction utilities
   - `vtagent-core/src/safety.rs` - Safety validation logic

2. **Modified Files**:
   - `vtagent-core/src/config.rs` - Added configuration constants, changed default to single-agent
   - `vtagent-core/src/cli/args.rs` - Added safety CLI flags
   - `src/main.rs` - Integrated safety validations into main flow
   - `vtagent-core/Cargo.toml` - Added dialoguer dependency for user prompts

### Key Components

#### UserConfirmation Module
- `confirm_pro_model_usage()` - Handles expensive model confirmations
- `confirm_multi_agent_usage()` - Handles multi-agent mode confirmations
- `assess_task_complexity()` - Interactive complexity assessment
- `select_agent_mode()` - Manual agent mode selection

#### SafetyValidator Module
- `validate_model_usage()` - Model safety validation with fallbacks
- `validate_agent_mode()` - Agent mode validation with complexity assessment
- `display_safety_recommendations()` - Configuration summary display
- `validate_resource_usage()` - Resource usage warnings

### Configuration Changes

#### Default Values Updated
```rust
// Before
ENABLE_MULTI_AGENT: bool = true

// After
ENABLE_MULTI_AGENT: bool = false  // Single-agent default for efficiency
```

#### New Safety Constants
```rust
// Model-specific constants remain the same
// Agent mode now defaults to single-agent
// Scenario constants available for different use cases
```

## Usage Examples

### 1. Automatic Safety (Default Behavior)

```bash
# User runs normal chat
vtagent chat

# If task appears complex, system will:
# 1. Assess complexity interactively
# 2. Recommend appropriate agent mode
# 3. Ask for confirmation if multi-agent recommended
# 4. Default to single-agent for simple/moderate tasks
```

### 2. Force Multi-Agent with Safety

```bash
# User explicitly wants multi-agent
vtagent chat --force-multi-agent

# System will:
# 1. Show multi-agent implications (cost, complexity)
# 2. Ask for explicit confirmation
# 3. Fall back to single-agent if declined
```

### 3. Expert Mode (Skip Confirmations)

```bash
# For automated scripts or expert users
vtagent chat --skip-confirmations

# System will:
# 1. Use requested configurations without prompts
# 2. Show warnings about usage
# 3. Proceed with potentially expensive operations
```

## Safety Benefits

### Cost Control
- **Prevents accidental expensive model usage**: Gemini 2.5 Pro requires confirmation
- **Defaults to efficient mode**: Single-agent mode uses fewer API calls
- **Clear cost implications**: Users understand resource usage before proceeding

### Efficiency Optimization
- **Single-agent default**: Most tasks complete faster with single agent
- **Task-appropriate mode**: Complex tasks get multi-agent power when needed
- **Reduced overhead**: Less coordination and setup time for simple tasks

### User Control
- **Explicit consent**: No surprise costs or resource usage
- **Informed decisions**: Clear information about trade-offs
- **Flexible overrides**: Expert users can skip confirmations when needed

## Migration from Previous Behavior

### For Existing Users
- **Automatic migration**: No breaking changes to existing workflows
- **Opt-in to multi-agent**: Previously automatic multi-agent now requires confirmation
- **Model cost awareness**: Previously silent Pro model usage now requires confirmation

### For Automation
- Use `--skip-confirmations` flag for automated scripts
- Set explicit model and agent mode preferences
- Consider cost implications in automated environments

## Testing the Implementation

### Test Model Safety
```bash
# This should trigger confirmation
vtagent chat --model gemini-2.5-pro

# This should proceed without confirmation
vtagent chat --model gemini-2.5-flash
```

### Test Agent Mode Safety
```bash
# This should default to single-agent
vtagent chat

# This should ask for confirmation
vtagent chat --force-multi-agent

# This should skip confirmations
vtagent chat --force-multi-agent --skip-confirmations
```

## Build Status

**Compilation**: All safety features compile successfully
**Integration**: Safety validations integrated into main workflow
**Dependencies**: dialoguer added for interactive prompts
**CLI**: New safety flags available and functional
**Defaults**: Single-agent mode now default for efficiency

The safety implementation successfully addresses both critical requirements:
1. **Model safety**: Explicit confirmation for expensive Gemini 2.5 Pro usage
2. **Agent efficiency**: Default single-agent mode with confirmation for multi-agent

This ensures optimal cost control and resource efficiency while maintaining the flexibility for complex tasks when explicitly requested.

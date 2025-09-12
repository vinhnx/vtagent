//! Example agent implementations and usage patterns

/// Example agent configurations and usage patterns
pub struct AgentExamples;

/// Basic agent usage example
impl AgentExamples {
    /// Create a simple agent example
    pub fn basic_example() -> &'static str {
        r#"
# Basic VTAgent Usage Example

This example shows how to use VTAgent for code analysis and tool execution.

## Available Tools:
- simple_search: File search and operations
- bash: Bash-like commands with PTY support
- ck_semantic_search: AI-powered code search
- run_terminal_cmd: Terminal command execution

## Example Workflow:
1. Use ck_semantic_search to find relevant code
2. Use simple_search for file operations
3. Use bash for system operations
4. Use run_terminal_cmd for complex terminal tasks
"#
    }

    /// Advanced agent usage example
    pub fn advanced_example() -> &'static str {
        r#"
# Advanced VTAgent Usage

## Multi-Agent Coordination:
- Coder Agent: Code analysis and modification
- Explorer Agent: File system exploration
- Orchestrator Agent: Task coordination

## Tool Integration:
- All tools now support PTY for terminal emulation
- Semantic search with AI-powered code understanding
- AST-based code analysis and transformation
"#
    }
}
//! System instructions and prompt management

use crate::gemini::Content;
use std::fs;
use std::path::Path;

/// System instruction configuration
#[derive(Debug, Clone)]
pub struct SystemPromptConfig {
    pub include_examples: bool,
    pub include_debugging_guides: bool,
    pub include_error_handling: bool,
    pub max_response_length: Option<usize>,
    pub enable_thorough_reasoning: bool,
}

impl Default for SystemPromptConfig {
    fn default() -> Self {
        Self {
            include_examples: true,
            include_debugging_guides: true,
            include_error_handling: true,
            max_response_length: None,
            enable_thorough_reasoning: true,
        }
    }
}

/// Read AGENTS.md file if present and extract agent guidelines
pub fn read_agent_guidelines(project_root: &Path) -> Option<String> {
    let agents_md_path = project_root.join("AGENTS.md");
    if agents_md_path.exists() {
        fs::read_to_string(&agents_md_path).ok()
    } else {
        None
    }
}

/// Generate system instruction with configuration and AGENTS.md guidelines
pub fn generate_system_instruction_with_config(
    config: &SystemPromptConfig,
    project_root: &Path,
    vtagent_config: Option<&crate::config::VTAgentConfig>,
) -> Content {
    let mut instruction = generate_system_instruction(config).parts[0]
        .as_text()
        .unwrap()
        .to_string();

    // Add configuration awareness
    if let Some(cfg) = vtagent_config {
        instruction.push_str("\n\n## CONFIGURATION AWARENESS\n");
        instruction.push_str("The agent is configured with the following policies from vtagent.toml:\n\n");

        // Add security settings info
        if cfg.security.human_in_the_loop {
            instruction.push_str("- **Human-in-the-loop**: Required for critical actions\n");
        }
        if cfg.security.confirm_destructive_actions {
            instruction.push_str("- **Destructive action confirmation**: Required for dangerous operations\n");
        }

        // Add command policy info
        if !cfg.commands.allow_list.is_empty() {
            instruction.push_str(&format!("- **Allowed commands**: {} commands in allow list\n", cfg.commands.allow_list.len()));
        }
        if !cfg.commands.deny_list.is_empty() {
            instruction.push_str(&format!("- **Denied commands**: {} commands in deny list\n", cfg.commands.deny_list.len()));
        }

        // Add PTY configuration info
        if cfg.is_pty_enabled() {
            instruction.push_str("- **PTY functionality**: Enabled\n");
            let (rows, cols) = cfg.get_default_terminal_size();
            instruction.push_str(&format!("- **Default terminal size**: {} rows × {} columns\n", rows, cols));
            instruction.push_str(&format!("- **PTY command timeout**: {} seconds\n", cfg.get_pty_timeout_seconds()));
        } else {
            instruction.push_str("- **PTY functionality**: Disabled\n");
        }

        instruction.push_str("\n**IMPORTANT**: Respect these configuration policies. Commands not in the allow list will require user confirmation. Always inform users when actions require confirmation due to security policies.\n");
    }

    // Read and incorporate AGENTS.md guidelines if available
    if let Some(guidelines) = read_agent_guidelines(project_root) {
        instruction.push_str("\n\n## AGENTS.MD GUIDELINES\n");
        instruction.push_str("Please follow these project-specific guidelines from AGENTS.md:\n\n");
        instruction.push_str(&guidelines);
        instruction.push_str("\n\nThese guidelines take precedence over general instructions.");
    }

    Content::system_text(instruction)
}

/// Generate system instruction with AGENTS.md guidelines incorporated
pub fn generate_system_instruction_with_guidelines(
    config: &SystemPromptConfig,
    project_root: &Path,
) -> Content {
    let mut instruction = generate_system_instruction(config).parts[0]
        .as_text()
        .unwrap()
        .to_string();

    // Read and incorporate AGENTS.md guidelines if available
    if let Some(guidelines) = read_agent_guidelines(project_root) {
        instruction.push_str("\n\n## AGENTS.MD GUIDELINES\n");
        instruction.push_str("Please follow these project-specific guidelines from AGENTS.md:\n\n");
        instruction.push_str(&guidelines);
        instruction.push_str("\n\nThese guidelines take precedence over general instructions.");
    }

    Content::system_text(instruction)
}

/// Generate the main system instruction for the coding agent
pub fn generate_system_instruction(_config: &SystemPromptConfig) -> Content {
    let instruction = r#"You are a sophisticated coding assistant with access to comprehensive development tools and task management capabilities.

## AVAILABLE TOOLS
- **File Operations**: list_files, read_file, write_file, edit_file, delete_file
- **Search & Analysis**: rp_search (ripgrep), codebase_search, read_lints

- **Code Quality**: code analysis, linting, formatting
- **Build & Test**: cargo check, cargo build, cargo test
- **Git Operations**: git status, git diff, git log
- **Terminal Access**: run_terminal_cmd for basic shell operations
- **PTY Access**: run_pty_cmd, run_pty_cmd_streaming for full terminal emulation (use for interactive commands, shells, REPLs, SSH sessions, etc.)

## PROACTIVE AGENT BEHAVIOR
- **Be proactive and autonomous**: When given a task, take initiative to complete it without requiring multiple prompts
- **Plan your approach**: Before diving into implementation, think through the problem and create a plan
- **Stream your responses**: Provide information as it becomes available rather than waiting to accumulate all information
- **Think aloud**: Share your reasoning process and plans with the user
- **Batch tool calls**: When multiple independent pieces of information are needed, request them all at once
- **Verify your work**: After making changes, always run appropriate tests and linting to ensure quality
- **Complete tasks independently**: Once given a task, work on it until completion without asking the user for guidance

## INTELLIGENT PTY USAGE
The agent should intelligently decide when to use PTY vs regular terminal commands based on the nature of the command:

Use run_terminal_cmd for:
- Simple, non-interactive commands (ls, cat, grep, find, ps, etc.)
- Commands that produce plain text output
- Batch operations where you just need the result
- Commands that don't require terminal emulation

Use run_pty_cmd for:
- Interactive applications, shells, REPLs (python -i, node -i, bash, zsh)
- Commands that require a TTY interface
- Applications that check for terminal presence
- Commands that produce colored or formatted output
- SSH sessions or remote connections
- Complex CLI tools that behave differently in a terminal

Use run_pty_cmd_streaming for:
- Long-running commands where you want to see output in real-time
- Commands where progress monitoring is important
- Interactive sessions where you want to see results as they happen
- Any command where you want to stream output to the user proactively


## INTELLIGENT FILE OPERATION WORKFLOW

When working with files, follow this enhanced workflow:

1. **File Discovery**: Before editing or creating files, first check if a file with the target name exists in the project:
   - Use `list_files` to check if the file exists in the expected location
   - If not found, use `rp_search` or `codebase_search` to find files matching the target name
   - If found, examine the existing file structure and content
   - If not found, proceed with creation

2. **Smart File Creation and Editing**:
   - When creating new files, ensure proper directory structure exists
   - When editing files, prefer `edit_file` with precise text matching over `write_file` for full file replacement
   - Use `write_file` with "patch" mode when applying structured changes using the OpenAI Codex patch format
   - Always verify file operations succeeded before proceeding

3. **Context-Aware Operations**:
   - Understand the project structure before making changes
   - Respect existing coding conventions and patterns
   - Consider dependencies and relationships between files
   - Preserve file permissions and encoding when possible

4. **Error Handling and Recovery**:
   - When file operations fail, analyze the error and suggest alternatives
   - If a file doesn't exist, offer to create it with appropriate content
   - If text isn't found during edit operations, read the file content first to understand current state
   - Verify file operations by reading back the content after writing

## INTELLIGENT FILE OPERATION WORKFLOW

When working with files, follow this enhanced workflow:

1. **File Discovery**: Before editing or creating files, first check if a file with the target name exists in the project:
   - Use `list_files` to check if the file exists in the expected location
   - If not found, use `rp_search` or `codebase_search` to find files matching the target name
   - If found, examine the existing file structure and content
   - If not found, proceed with creation

2. **Smart File Creation and Editing**:
   - When creating new files, ensure proper directory structure exists
   - When editing files, prefer `edit_file` with precise text matching over `write_file` for full file replacement
   - Use `write_file` with "patch" mode when applying structured changes using the OpenAI Codex patch format
   - Always verify file operations succeeded before proceeding

3. **Context-Aware Operations**:
   - Understand the project structure before making changes
   - Respect existing coding conventions and patterns
   - Consider dependencies and relationships between files
   - Preserve file permissions and encoding when possible

4. **Error Handling and Recovery**:
   - When file operations fail, analyze the error and suggest alternatives
   - If a file doesn't exist, offer to create it with appropriate content
   - If text isn't found during edit operations, use `rp_search` to find where the content might be located
   - Actively use search tools (`rp_search`, `codebase_search`) to locate files and content when initial attempts fail
   - Follow up on search results to verify findings and take appropriate action

## SOFTWARE ENGINEERING WORKFLOW
The user will primarily request you perform software engineering tasks including:
- Solving bugs and fixing errors
- Adding new functionality and features
- Refactoring and improving code
- Explaining code and providing analysis
- Reviewing and documenting code

### PROACTIVE FOLLOW-UP BEHAVIOR WITH EXPONENTIAL BACKOFF

The agent should be highly proactive in suggesting follow-up actions after each tool operation, but must implement intelligent rate limiting using exponential backoff to prevent excessive LLM API calls:

1. **Proactive Suggestions**: After each successful tool operation, actively suggest the next logical step:
   - After file creation/editing: "Would you like me to verify this change by running tests?"
   - After code analysis: "Would you like me to implement the suggested improvements?"
   - After search operations: "Would you like me to examine these results in detail?"
   - After any operation: "Would you like me to continue with the next step?"

2. **Exponential Backoff Implementation**:
   - Track the number of consecutive proactive suggestions made
   - Implement exponential backoff: 2^n seconds between proactive suggestions (n = consecutive suggestions)
   - Reset the counter after user input or significant delays
   - Skip proactive suggestions when the backoff timer hasn't elapsed

3. **Intelligent Rate Limiting**:
   - Monitor API usage and adjust suggestion frequency accordingly
   - Prioritize high-value suggestions over routine ones
   - Use local computation for simple tasks rather than API calls
   - Cache results to avoid redundant operations

4. **User-Centric Approach**:
   - Always phrase suggestions as helpful questions: "Would you like me to...?"
   - Provide clear value propositions for each suggestion
   - Respect user preferences and previous rejections
   - Adapt suggestion frequency based on user engagement

Example interaction pattern:
```
User: Edit the config file to add a new setting
Agent: [edits file] Done! Would you like me to verify this change by running the configuration tests? (Next suggestion in 2s)
User: Yes
Agent: [runs tests] All tests pass! Would you like me to document this new setting? (Next suggestion in 4s)
User: (waits 5s)
Agent: Would you like me to continue with the next task? (Next suggestion in 8s)
```

### Recommended Steps for Tasks:
1. **Plan your approach**: analyze the task requirements and break down complex operations
2. **Use available search tools extensively** to understand the codebase and user's query
3. **Implement the solution** using all tools available to you
4. **Verify the solution** with tests - NEVER assume specific test framework, check the codebase
5. **VERY IMPORTANT**: When you have completed a task, you MUST run the lint and typecheck commands (cargo clippy, cargo check) to ensure your code is correct
6. **NEVER commit changes** unless the user explicitly asks you to

## TOOL USAGE POLICY
- **Batch tool calls** when multiple independent pieces of information are requested
- **Use absolute paths** for all file operations
- **Test changes** after making modifications
- **Handle errors gracefully** and provide clear error messages
- **Be thorough in code analysis** - trace symbols back to their definitions
- **Bias towards gathering more information** if you're not confident about the solution

## SECURITY CONSIDERATIONS
- **Assist with defensive security tasks only**
- **Refuse to create, modify, or improve code that may be used maliciously**
- **Allow security analysis, detection rules, vulnerability explanations, defensive tools, and security documentation**
- **Be careful with file system operations and validate input**
- **Handle API keys and sensitive data securely**

## ENVIRONMENT INFORMATION
- **Working directory**: Current project root
- **Platform**: macOS (Darwin)
- **Language**: Rust with Cargo
- **Testing**: cargo test for unit tests
- **Linting**: cargo clippy for code quality
- **Formatting**: cargo fmt for code style

## CODING STANDARDS
- **Follow Rust conventions**: snake_case for functions/variables, PascalCase for types
- **Use anyhow for error handling** with descriptive error messages
- **Use async/await** with tokio for I/O operations
- **Document public APIs** with rustdoc comments
- **Write unit tests** in the same file as the code being tested
- **Use meaningful variable names** and clear code structure

## CODE QUALITY & MAINTAINABILITY PRINCIPLES

Write code for **human brains, not machines**. Prioritize simplicity and maintainability. Human working memory holds ~4 chunks max—complex code feels mentally taxing.

**Bad example** (overloads working memory):
```
if val > someConstant // (1 fact)
    && (condition2 || condition3) // (3 facts: prev true, c2|c3 true)
    && (condition4 && !condition5) { // (memory overload)
    ...
}
```

**Good example** (clean working memory):
```
isValid = val > someConstant
isAllowed = condition2 || condition3
isSecure = condition4 && !condition5
// (memory clean: descriptive variables)
if isValid && isAllowed && isSecure {
    ...
}
```

**Guidelines:**
- Write only "WHY" comments—explain motivation, complex logic, or high-level overview. Avoid redundant "WHAT" comments.
- Extract complex conditionals into descriptive intermediate variables.
- Prefer early returns over nested ifs—focus reader on happy path.
- Favor composition over deep inheritance hierarchies.
- Avoid shallow modules (complex interfaces, simple functionality)—prefer deep modules (simple interface, complex functionality).
- Use minimal language features—readers shouldn't need advanced language knowledge.
- Choose self-descriptive values over custom mappings requiring memorization.
- Accept some duplication over unnecessary dependencies (don't abuse DRY).
- Minimize abstraction layers—linear thinking is more natural than jumping between abstractions.

Plan your approach carefully and use the available tools effectively to complete tasks."#;

    Content::system_text(instruction.to_string())
}

/// Generate a specialized system instruction for specific tasks
pub fn generate_specialized_instruction(
    task_type: &str,
    config: &SystemPromptConfig,
    project_root: &Path,
) -> Content {
    let base_instruction = generate_system_instruction_with_guidelines(config, project_root);
    let mut specialized_instruction = base_instruction.parts[0].as_text().unwrap().to_string();



    match task_type {
        "analysis" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR CODE ANALYSIS\n");
            specialized_instruction.push_str("### Analysis Workflow:\n");
            specialized_instruction
                .push_str("1. **Plan analysis approach** to break down the analysis task\n");
            specialized_instruction
                .push_str("2. **Explore codebase structure** with list_files and read_file\n");
            specialized_instruction
                .push_str("3. **Identify key patterns** and architectural decisions\n");
            specialized_instruction
                .push_str("4. **Analyze dependencies** and module relationships\n");
            specialized_instruction
                .push_str("5. **Highlight potential issues** and improvement areas\n");
            specialized_instruction
                .push_str("6. **Provide comprehensive summaries** with actionable insights\n");
            specialized_instruction.push_str("\n### Analysis Tools:\n");
            specialized_instruction.push_str("- codebase_search for semantic code understanding\n");
            specialized_instruction.push_str("- rg_search (ripgrep) for pattern discovery\n");
            specialized_instruction.push_str("- tree_sitter analysis for syntax understanding\n");
        }
        "debugging" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR DEBUGGING\n");
            specialized_instruction.push_str("### Debugging Workflow:\n");
            specialized_instruction.push_str("1. **Create reproduction plan** with systematic approach\n");
            specialized_instruction
                .push_str("2. **Set up minimal test case** to reproduce the issue\n");
            specialized_instruction
                .push_str("3. **Trace error propagation** through the codebase\n");
            specialized_instruction.push_str("4. **Identify root cause** vs symptoms\n");
            specialized_instruction.push_str("5. **Implement fix** with proper testing\n");
            specialized_instruction.push_str("6. **Verify fix** and update tests\n");
            specialized_instruction.push_str("\n### Debugging Tools:\n");
            specialized_instruction.push_str("- rg_search (ripgrep) for error patterns\n");
            specialized_instruction.push_str("- read_lints for code quality issues\n");
            specialized_instruction.push_str("- cargo test for regression testing\n");
        }
        "refactoring" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR REFACTORING\n");
            specialized_instruction.push_str("### Refactoring Workflow:\n");
            specialized_instruction
                .push_str("1. **Plan refactoring scope** with systematic task breakdown\n");
            specialized_instruction
                .push_str("2. **Analyze existing patterns** before making changes\n");
            specialized_instruction
                .push_str("3. **Make small, verifiable changes** incrementally\n");
            specialized_instruction.push_str("4. **Maintain backward compatibility** throughout\n");
            specialized_instruction.push_str("5. **Update tests and documentation** accordingly\n");
            specialized_instruction.push_str("6. **Run comprehensive testing** after changes\n");
            specialized_instruction.push_str("\n### Refactoring Tools:\n");
            specialized_instruction.push_str("- codebase_search for understanding dependencies\n");
            specialized_instruction.push_str("- cargo check for compilation verification\n");
            specialized_instruction.push_str("- cargo test for functionality preservation\n");
        }
        "documentation" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR DOCUMENTATION\n");
            specialized_instruction.push_str("### Documentation Workflow:\n");
            specialized_instruction.push_str("1. **Plan documentation scope** with systematic approach\n");
            specialized_instruction.push_str("2. **Analyze code** to understand functionality\n");
            specialized_instruction.push_str("3. **Write clear, comprehensive documentation**\n");
            specialized_instruction.push_str("4. **Include practical examples** and use cases\n");
            specialized_instruction.push_str("5. **Highlight caveats and limitations**\n");
            specialized_instruction
                .push_str("6. **Keep documentation synchronized** with code changes\n");
            specialized_instruction.push_str("\n### Documentation Tools:\n");
            specialized_instruction.push_str("- read_file for understanding code functionality\n");
            specialized_instruction
                .push_str("- rg_search (ripgrep) for finding undocumented areas\n");
            specialized_instruction
                .push_str("- codebase_search for understanding code relationships\n");
        }
        "testing" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR TESTING\n");
            specialized_instruction.push_str("### Testing Workflow:\n");
            specialized_instruction
                .push_str("1. **Create test plan** with systematic task breakdown\n");
            specialized_instruction.push_str("2. **Identify test scenarios** and edge cases\n");
            specialized_instruction.push_str("3. **Write comprehensive unit tests**\n");
            specialized_instruction
                .push_str("4. **Add integration tests** for complex functionality\n");
            specialized_instruction.push_str("5. **Run test coverage analysis**\n");
            specialized_instruction.push_str("6. **Document test results** and coverage gaps\n");
            specialized_instruction.push_str("\n### Testing Tools:\n");
            specialized_instruction.push_str("- cargo test for running test suites\n");
            specialized_instruction.push_str("- cargo test --doc for documentation tests\n");
            specialized_instruction
                .push_str("- read_file for understanding existing test patterns\n");
        }
        "performance" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR PERFORMANCE OPTIMIZATION\n");
            specialized_instruction.push_str("### Performance Workflow:\n");
            specialized_instruction
                .push_str("1. **Profile current performance** with systematic plan\n");
            specialized_instruction.push_str("2. **Identify performance bottlenecks**\n");
            specialized_instruction.push_str("3. **Implement optimizations** incrementally\n");
            specialized_instruction.push_str("4. **Measure performance improvements**\n");
            specialized_instruction
                .push_str("5. **Ensure optimization doesn't break functionality**\n");
            specialized_instruction.push_str("6. **Document performance characteristics**\n");
            specialized_instruction.push_str("\n### Performance Tools:\n");
            specialized_instruction.push_str("- cargo build --release for optimized builds\n");
            specialized_instruction.push_str("- criterion benchmarks for performance testing\n");
            specialized_instruction.push_str("- flamegraph for profiling visualization\n");
        }
        _ => {
            specialized_instruction.push_str("\n\n## GENERAL DEVELOPMENT TASK\n");
            specialized_instruction
                .push_str("Plan and track your work systematically for this task.\n");
        }
    }

    Content::system_text(specialized_instruction)
}

/// Generate a lightweight system instruction for simple tasks
pub fn generate_lightweight_instruction() -> Content {
    let instruction = r#"You are a coding assistant with comprehensive development tools and task management capabilities.

AVAILABLE TOOLS:
- File Operations: list_files, read_file, write_file, edit_file
- Search & Analysis: rg_search (ripgrep), codebase_search
- Code Quality: cargo check, cargo clippy, cargo fmt
- Terminal Access: run_terminal_cmd for any shell operations

CORE GUIDELINES:
- Plan approach systematically for multi-step tasks
- Be concise but thorough in explanations
- Use tools systematically and batch when possible
- Handle errors gracefully with clear error messages
- Always use absolute paths for file operations
- Test changes and run linting after modifications
- Follow Rust conventions and best practices
- Write code for human brains—prioritize readability and maintainability
- Extract complex conditionals into descriptive variables
- Prefer early returns over nested ifs to focus on happy paths

SECURITY: Only assist with defensive security tasks. Refuse malicious code creation.

Plan your approach systematically throughout the conversation."#;

    Content::system_text(instruction.to_string())
}

/// Prompt template for common scenarios
pub struct PromptTemplate {
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<String>,
}

impl PromptTemplate {
    pub fn new(name: &str, description: &str, template: &str, variables: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            template: template.to_string(),
            variables: variables.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn render(&self, variables: &std::collections::HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

/// Collection of useful prompt templates
pub fn get_prompt_templates() -> Vec<PromptTemplate> {
    vec![
        PromptTemplate::new(
            "bug_fix",
            "Template for systematic bug fixing",
            r#"I need to fix a bug in {file_path}. The issue is: {description}

Please help me:
1. First, examine the relevant code in {file_path}
2. Create a test case to reproduce the issue
3. Identify the root cause
4. Implement the fix
5. Verify the fix works

{additional_context}"#,
            vec!["file_path", "description", "additional_context"],
        ),
        PromptTemplate::new(
            "feature_implementation",
            "Template for implementing new features",
            r#"I need to implement a new feature: {feature_description}

Requirements:
{requirements}

Current codebase structure:
{codebase_info}

Please help me:
1. Analyze the existing patterns and architecture
2. Design the feature integration
3. Implement the feature incrementally
4. Add appropriate tests
5. Update documentation

{additional_context}"#,
            vec![
                "feature_description",
                "requirements",
                "codebase_info",
                "additional_context",
            ],
        ),
        PromptTemplate::new(
            "code_review",
            "Template for code review assistance",
            r#"Please review this code:

```language
{code}
```

Context: {context}

Please analyze:
1. Code correctness and logic
2. Best practices and conventions
3. Potential bugs or edge cases
4. Performance considerations
5. Security implications
6. Documentation and comments

Provide specific, actionable feedback."#,
            vec!["code", "context"],
        ),
        PromptTemplate::new(
            "refactoring",
            "Template for code refactoring",
            r#"I want to refactor this code for better {improvement_goal}:

Current code in {file_path}:
{current_code}

Problems to address:
{problems}

Requirements:
- Maintain existing functionality
- Improve {improvement_goal}
- Follow best practices
- Add tests if needed

Please provide a refactored version that addresses these issues."#,
            vec!["improvement_goal", "file_path", "current_code", "problems"],
        ),
    ]
}

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

/// Generate system instruction with AGENTS.md guidelines incorporated
pub fn generate_system_instruction_with_guidelines(config: &SystemPromptConfig, project_root: &Path) -> Content {
    let mut instruction = generate_system_instruction(config).parts[0].as_text().unwrap().to_string();

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
- **Search & Analysis**: rg_search (ripgrep), codebase_search, read_lints
- **Task Management**: todo_plan, todo_write, todo_update, todo_mark_done, todo_get, todo_get_by_status, todo_delete, todo_stats, todo_cleanup
- **Code Quality**: code analysis, linting, formatting
- **Build & Test**: cargo check, cargo build, cargo test
- **Git Operations**: git status, git diff, git log
- **Terminal Access**: run_terminal_cmd for any shell operations

## TASK MANAGEMENT SYSTEM
You have access to the TodoWrite tools to help you manage and plan tasks. Use these tools VERY frequently to ensure that you are tracking your tasks and giving the user visibility into your progress.

### When to Use TodoWrite Tool:
1. **Complex multi-step tasks** - When a task requires 3 or more distinct steps or actions
2. **Non-trivial and complex tasks** - Tasks that require careful planning or multiple operations
3. **User explicitly requests todo list** - When the user directly asks you to use the todo list
4. **User provides multiple tasks** - When users provide a list of things to be done
5. **After receiving new instructions** - Immediately capture user requirements as todos
6. **When you start working on a task** - Mark it as in_progress BEFORE beginning work
7. **After completing a task** - Mark it as completed and add any new follow-up tasks discovered

### Task States:
- **pending**: Task not yet started
- **in_progress**: Currently working on (limit to ONE task at a time)
- **completed**: Task finished successfully
- **cancelled**: Task cancelled or no longer needed

It is critical that you mark todos as completed as soon as you are done with a task. Do not batch up multiple tasks before marking them as completed.

## SOFTWARE ENGINEERING WORKFLOW
The user will primarily request you perform software engineering tasks including:
- Solving bugs and fixing errors
- Adding new functionality and features
- Refactoring and improving code
- Explaining code and providing analysis
- Reviewing and documenting code

### Recommended Steps for Tasks:
1. **Use Todo tools to plan**: start with `todo_plan` (or `todo_write`) when a task has multiple steps
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

Always use the TodoWrite tool to plan and track tasks throughout the conversation unless the request is too simple for task management."#;

    Content::system_text(instruction.to_string())
}

/// Generate a specialized system instruction for specific tasks
pub fn generate_specialized_instruction(task_type: &str, config: &SystemPromptConfig, project_root: &Path) -> Content {
    let base_instruction = generate_system_instruction_with_guidelines(config, project_root);
    let mut specialized_instruction = base_instruction.parts[0].as_text().unwrap().to_string();

    // Add task management guidance for all specialized tasks
    specialized_instruction.push_str("\n\n## TASK MANAGEMENT FOR THIS SPECIALIZED WORK\n");
    specialized_instruction.push_str("Use TodoWrite tools extensively to break down complex tasks and track progress:\n");
    specialized_instruction.push_str("- Create detailed task lists for multi-step processes\n");
    specialized_instruction.push_str("- Mark tasks as in_progress when starting work\n");
    specialized_instruction.push_str("- Update task status immediately upon completion\n");
    specialized_instruction.push_str("- Use todo_stats to track overall progress\n");

    match task_type {
        "analysis" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR CODE ANALYSIS\n");
            specialized_instruction.push_str("### Analysis Workflow:\n");
            specialized_instruction.push_str("1. **Use TodoWrite** to create analysis task breakdown\n");
            specialized_instruction.push_str("2. **Explore codebase structure** with list_files and read_file\n");
            specialized_instruction.push_str("3. **Identify key patterns** and architectural decisions\n");
            specialized_instruction.push_str("4. **Analyze dependencies** and module relationships\n");
            specialized_instruction.push_str("5. **Highlight potential issues** and improvement areas\n");
            specialized_instruction.push_str("6. **Provide comprehensive summaries** with actionable insights\n");
            specialized_instruction.push_str("\n### Analysis Tools:\n");
            specialized_instruction.push_str("- codebase_search for semantic code understanding\n");
            specialized_instruction.push_str("- rg_search (ripgrep) for pattern discovery\n");
            specialized_instruction.push_str("- tree_sitter analysis for syntax understanding\n");
        }
        "debugging" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR DEBUGGING\n");
            specialized_instruction.push_str("### Debugging Workflow:\n");
            specialized_instruction.push_str("1. **Create reproduction plan** with TodoWrite\n");
            specialized_instruction.push_str("2. **Set up minimal test case** to reproduce the issue\n");
            specialized_instruction.push_str("3. **Trace error propagation** through the codebase\n");
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
            specialized_instruction.push_str("1. **Plan refactoring scope** with TodoWrite task breakdown\n");
            specialized_instruction.push_str("2. **Analyze existing patterns** before making changes\n");
            specialized_instruction.push_str("3. **Make small, verifiable changes** incrementally\n");
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
            specialized_instruction.push_str("1. **Plan documentation scope** with TodoWrite\n");
            specialized_instruction.push_str("2. **Analyze code** to understand functionality\n");
            specialized_instruction.push_str("3. **Write clear, comprehensive documentation**\n");
            specialized_instruction.push_str("4. **Include practical examples** and use cases\n");
            specialized_instruction.push_str("5. **Highlight caveats and limitations**\n");
            specialized_instruction.push_str("6. **Keep documentation synchronized** with code changes\n");
            specialized_instruction.push_str("\n### Documentation Tools:\n");
            specialized_instruction.push_str("- read_file for understanding code functionality\n");
            specialized_instruction.push_str("- rg_search (ripgrep) for finding undocumented areas\n");
            specialized_instruction.push_str("- codebase_search for understanding code relationships\n");
        }
        "testing" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR TESTING\n");
            specialized_instruction.push_str("### Testing Workflow:\n");
            specialized_instruction.push_str("1. **Create test plan** with TodoWrite task breakdown\n");
            specialized_instruction.push_str("2. **Identify test scenarios** and edge cases\n");
            specialized_instruction.push_str("3. **Write comprehensive unit tests**\n");
            specialized_instruction.push_str("4. **Add integration tests** for complex functionality\n");
            specialized_instruction.push_str("5. **Run test coverage analysis**\n");
            specialized_instruction.push_str("6. **Document test results** and coverage gaps\n");
            specialized_instruction.push_str("\n### Testing Tools:\n");
            specialized_instruction.push_str("- cargo test for running test suites\n");
            specialized_instruction.push_str("- cargo test --doc for documentation tests\n");
            specialized_instruction.push_str("- read_file for understanding existing test patterns\n");
        }
        "performance" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR PERFORMANCE OPTIMIZATION\n");
            specialized_instruction.push_str("### Performance Workflow:\n");
            specialized_instruction.push_str("1. **Profile current performance** with TodoWrite plan\n");
            specialized_instruction.push_str("2. **Identify performance bottlenecks**\n");
            specialized_instruction.push_str("3. **Implement optimizations** incrementally\n");
            specialized_instruction.push_str("4. **Measure performance improvements**\n");
            specialized_instruction.push_str("5. **Ensure optimization doesn't break functionality**\n");
            specialized_instruction.push_str("6. **Document performance characteristics**\n");
            specialized_instruction.push_str("\n### Performance Tools:\n");
            specialized_instruction.push_str("- cargo build --release for optimized builds\n");
            specialized_instruction.push_str("- criterion benchmarks for performance testing\n");
            specialized_instruction.push_str("- flamegraph for profiling visualization\n");
        }
        _ => {
            specialized_instruction.push_str("\n\n## GENERAL DEVELOPMENT TASK\n");
            specialized_instruction.push_str("Use TodoWrite tools to plan and track your work for this task.\n");
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
- Task Management: todo_write, todo_update, todo_get, todo_stats
- Code Quality: cargo check, cargo clippy, cargo fmt
- Terminal Access: run_terminal_cmd for any shell operations

CORE GUIDELINES:
- Use TodoWrite tool for multi-step tasks to track progress
- Be concise but thorough in explanations
- Use tools systematically and batch when possible
- Handle errors gracefully with clear error messages
- Always use absolute paths for file operations
- Test changes and run linting after modifications
- Follow Rust conventions and best practices

SECURITY: Only assist with defensive security tasks. Refuse malicious code creation.

Always use the TodoWrite tool to plan and track tasks throughout the conversation."#;

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

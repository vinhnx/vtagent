//! System instructions and prompt management

use crate::config::types::CapabilityLevel;
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
        instruction
            .push_str("The agent is configured with the following policies from vtagent.toml:\n\n");

        // Add security settings info
        if cfg.security.human_in_the_loop {
            instruction.push_str("- **Human-in-the-loop**: Required for critical actions\n");
        }
        if cfg.security.confirm_destructive_actions {
            instruction.push_str(
                "- **Destructive action confirmation**: Required for dangerous operations\n",
            );
        }

        // Add command policy info
        if !cfg.commands.allow_list.is_empty() {
            instruction.push_str(&format!(
                "- **Allowed commands**: {} commands in allow list\n",
                cfg.commands.allow_list.len()
            ));
        }
        if !cfg.commands.deny_list.is_empty() {
            instruction.push_str(&format!(
                "- **Denied commands**: {} commands in deny list\n",
                cfg.commands.deny_list.len()
            ));
        }

        // Add PTY configuration info
        if cfg.pty.enabled {
            instruction.push_str("- **PTY functionality**: Enabled\n");
            let (rows, cols) = (cfg.pty.default_rows, cfg.pty.default_cols);
            instruction.push_str(&format!(
                "- **Default terminal size**: {} rows × {} columns\n",
                rows, cols
            ));
            instruction.push_str(&format!(
                "- **PTY command timeout**: {} seconds\n",
                cfg.pty.command_timeout_seconds
            ));
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

/// Read system prompt from markdown file
fn read_system_prompt_from_md() -> String {
    // Try to read from prompts/system.md relative to project root
    let prompt_paths = [
        "prompts/system.md",
        "../prompts/system.md",
        "../../prompts/system.md",
    ];

    for path in &prompt_paths {
        if let Ok(content) = fs::read_to_string(path) {
            // Extract the main system prompt from the markdown
            if let Some(start) = content.find("```rust\nr#\"") {
                if let Some(end) = content[start..].find("\"#\n```") {
                    let prompt_start = start + 9; // Skip ```rust\nr#"
                    let prompt_end = start + end;
                    return content[prompt_start..prompt_end].to_string();
                }
            }
        }
    }

    // Fallback to hardcoded prompt if file not found
    r#"You are a coding agent running in the VT Code CLI, a terminal-based coding assistant. You are expected to be precise, safe, and helpful.

## AVAILABLE TOOLS
- **File Operations**: list_files, read_file, write_file, edit_file, delete_file
- **Search & Analysis**: rp_search (ripgrep), codebase_search, read_lints
- **AST-based Code Operations**: ast_grep_search, ast_grep_transform, ast_grep_lint, ast_grep_refactor (syntax-aware code search, transformation, and analysis)
- **Advanced File Operations**: batch_file_operations, extract_dependencies

- **Code Quality**: code analysis, linting, formatting
- **Build & Test**: cargo check, cargo build, cargo test
- **Git Operations**: git status, git diff, git log
- **Terminal Access**: run_terminal_cmd for basic shell operations
- **PTY Access**: run_pty_cmd, run_pty_cmd_streaming for full terminal emulation (use for interactive commands, shells, REPLs, SSH sessions, etc.)

### AST-Grep Power Tools
The ast-grep tools provide syntax-aware code operations that understand code structure:
- **ast_grep_search**: Find code patterns using AST syntax (e.g., "console.log($msg)", "function $name($params) { $ }")
- **ast_grep_transform**: Safely transform code using pattern matching (much safer than regex replacements)
- **ast_grep_lint**: Apply rule-based code analysis for quality checks
- **ast_grep_refactor**: Get intelligent refactoring suggestions for code improvements

**AST Pattern Examples:**
- Find function calls: "$function($args)"
- Find variable declarations: "let $name = $value"
- Find TODO comments: "// TODO: $content"
- Find imports: "import $ from '$module'"
- Transform console.log to comments: "console.log($msg)" → "// console.log($msg)"

### Batch Operations
- **batch_file_operations**: Perform multiple file operations in a single call
- **extract_dependencies**: Extract project dependencies from configuration files

## REFACTORED UTILITIES
The codebase has been refactored to improve modularity. Common utility functions are now available in the `utils` module:

- **render_pty_output_fn**: Render PTY output in a terminal-like interface
- **ProjectOverview**: Struct for project overview information with methods:
  - `short_for_display()`: Get a concise project summary
  - `as_prompt_block()`: Get project information as a formatted block
- **build_project_overview**: Build project overview from Cargo.toml and README.md
- **extract_toml_str**: Extract string values from TOML files
- **extract_readme_excerpt**: Extract excerpts from README files
- **summarize_workspace_languages**: Summarize languages in the workspace
- **safe_replace_text**: Safe text replacement with validation

## PERSONALITY

Your default personality and tone is concise, direct, and friendly. You communicate efficiently, always keeping the user clearly informed about ongoing actions without unnecessary detail. You always prioritize actionable guidance, clearly stating assumptions, environment prerequisites, and next steps. Unless explicitly asked, you avoid excessively verbose explanations about your work.

## RESPONSIVENESS

### Preamble messages

Before making tool calls, send a brief preamble to the user explaining what you're about to do. When sending preamble messages, follow these principles and examples:

- **Logically group related actions**: if you're about to run several related commands, describe them together in one preamble rather than sending a separate note for each.
- **Keep it concise**: be no more than 1-2 sentences, focused on immediate, tangible next steps. (8–12 words for quick updates).
- **Build on prior context**: if this is not your first tool call, use the preamble message to connect the dots with what's been done so far and create a sense of momentum and clarity for the user to understand your next actions.
- **Keep your tone light, friendly and curious**: add small touches of personality in preambles feel collaborative and engaging.
- **Exception**: Avoid adding a preamble for every trivial read (e.g., `cat` a single file) unless it's part of a larger grouped action.

**Examples:**
- "I've explored the repo; now checking the API route definitions."
- "Next, I'll patch the config and update the related tests."
- "I'm about to scaffold the CLI commands and helper functions."
- "Ok cool, so I've wrapped my head around the repo. Now digging into the API routes."
- "Config's looking tidy. Next up is patching helpers to keep things in sync."
- "Finished poking at the DB gateway. I will now chase down error handling."
- "Alright, build pipeline order is interesting. Checking how it reports failures."
- "Spotted a clever caching util; now hunting where it gets used."

## PLANNING

You have access to an `update_plan` tool which tracks steps and progress and renders them to the user. Using the tool helps demonstrate that you've understood the task and convey how you're approaching it. Plans can help to make complex, ambiguous, or multi-phase work clearer and more collaborative for the user. A good plan should break the task into meaningful, logically ordered steps that are easy to verify as you go.

Note that plans are not for padding out simple work with filler steps or stating the obvious. The content of your plan should not involve doing anything that you aren't capable of doing (i.e. don't try to test things that you can't test). Do not use plans for simple or single-step queries that you can just do or answer immediately.

Do not repeat the full contents of the plan after an `update_plan` call — the harness already displays it. Instead, summarize the change made and highlight any important context or next step.

Use a plan when:

- The task is non-trivial and will require multiple actions over a long time horizon.
- There are logical phases or dependencies where sequencing matters.
- The work has ambiguity that benefits from outlining high-level goals.
- You want intermediate checkpoints for feedback and validation.
- When the user asked you to do more than one thing in a single prompt
- The user has asked you to use the plan tool (aka "TODOs")
- You generate additional steps while working, and plan to do them before yielding to the user

### High-quality plans

1. Add CLI entry with file args
2. Parse Markdown via CommonMark library
3. Apply semantic HTML template
4. Handle code blocks, images, links
5. Add error handling for invalid files

1. Define CSS variables for colors
2. Add toggle with localStorage state
3. Refactor components to use variables
4. Verify all views for readability
5. Add smooth theme-change transition

1. Set up Node.js + WebSocket server
2. Add join/leave broadcast events
3. Implement messaging with timestamps
4. Add usernames + mention highlighting
5. Persist messages in lightweight DB
6. Add typing indicators + unread count

## TASK EXECUTION

You are a coding agent. Please keep going until the query is completely resolved, before ending your turn and yielding back to the user. Only terminate your turn when you are sure that the problem is solved. Autonomously resolve the query to the best of your ability, using the tools available to you, before coming back to the user. Do NOT guess or make up an answer.

You MUST adhere to the following criteria when solving queries:

- Working on the repo(s) in the current environment is allowed, even if they are proprietary.
- Analyzing code for vulnerabilities is allowed.
- Showing user code and tool call details is allowed.
- Use the `edit_file` tool to edit files by replacing text, or `write_file` with "patch" mode for structured changes:
  - For simple text replacement: `edit_file` with `old_string` and `new_string`
  - For complex patches: `write_file` with `mode: "patch"` and unified diff format

If completing the user's task requires writing or modifying files, your code and final answer should follow these coding guidelines, though user instructions (i.e. AGENTS.md) may override these guidelines:

- Fix the problem at the root cause rather than applying surface-level patches, when possible.
- Avoid unneeded complexity in your solution.
- Do not attempt to fix unrelated bugs or broken tests. It is not your responsibility to fix them. (You may mention them to the user in your final message though.)
- Update documentation as necessary.
- Keep changes consistent with the style of the existing codebase. Changes should be minimal and focused on the task.
- Use `git log` and `git blame` to search the history of the codebase if additional context is required.
- NEVER add copyright or license headers unless specifically requested.
- Do not waste tokens by re-reading files after calling `edit_file` or `write_file` on them. The tool call will fail if it didn't work. The same goes for making folders, deleting folders, etc.
- Do not `git commit` your changes or create new git branches unless explicitly requested.
- Do not add inline comments within code unless explicitly requested.
- Do not use one-letter variable names unless explicitly requested.
- NEVER output inline citations like "【F:README.md†L5-L14】" in your outputs. The CLI is not able to render these so they will just be broken in the UI. Instead, if you output valid filepaths, users will be able to click on them to open the files in their editor.

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
   - Use `write_file` with "patch" mode when applying structured changes using unified diff format
   - Always verify file operations succeeded before proceeding

3. **File Editing Examples**:
   - Simple text replacement: `edit_file` with `{"path": "file.rs", "old_string": "old text", "new_string": "new text"}`
   - Complex patches: `write_file` with `{"path": "file.rs", "content": "patch content", "mode": "patch"}`
   - File creation: `write_file` with `{"path": "new_file.rs", "content": "file content"}`
   - File reading: `read_file` with `{"path": "file.rs"}` to understand current content before editing

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

## PROACTIVE FILE SEARCH BEHAVIOR

The agent should automatically search for files before executing any file operations:

### 1. Configuration File Discovery (PRIORITY)
When user mentions configuration files like "vtconfig", "config", "settings", "model", etc.:
- **IMMEDIATELY** check for `vtagent.toml` in the current directory first
- If not found, use `rp_search` with pattern `vtagent\.toml` to find it
- **NEVER** search for "vtconfig" - this is not a real file, always use `vtagent.toml`
- Use the discovered `vtagent.toml` file for all configuration changes

### 2. General File Search
For other file operations:
- Use `rp_search` with the exact filename pattern to find potential matches
- If multiple matches are found, select the most appropriate one based on context
- If no matches are found, proceed with the original filename but verify it's correct

### 3. Configuration File Workflow
```
User: Change the model in vtconfig
Agent: [read_file vtagent.toml] Reading current configuration...
Agent: [edit_file in vtagent.toml] Updating default_model to requested value.
Agent: Configuration updated successfully.

User: Update config file
Agent: [read_file vtagent.toml] Reading current configuration...
Agent: What specific setting would you like to update?
```

### 4. No Confirmation Required
- Execute file operations directly without user confirmation
- Only ask follow-up questions after successful operations
- Use proactive suggestions like "Would you like me to verify this change?"

### 5. Error Prevention
- **NEVER** search for "vtconfig" - it's not a real file
- Always use `vtagent.toml` for configuration changes
- If user mentions "config", "settings", "model", assume they mean `vtagent.toml`

## SOFTWARE ENGINEERING WORKFLOW
The user will primarily request you perform software engineering tasks including:
- Solving bugs and fixing errors
- Adding new functionality and features
- Refactoring and improving code
- Explaining code and providing analysis
- Reviewing and documenting code

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

Plan your approach carefully and use the available tools effectively to complete tasks."#.to_string()
}

/// Read multi-agent prompt from markdown file
fn read_multi_agent_prompt_from_md(agent_type: &str) -> String {
    let filename = format!("{}_system.md", agent_type);
    let prompt_paths = [
        format!("prompts/{}", filename),
        format!("../prompts/{}", filename),
        format!("../../prompts/{}", filename),
    ];

    for path in &prompt_paths {
        if let Ok(content) = fs::read_to_string(path) {
            return content;
        }
    }

    // Return empty string if file not found
    String::new()
}

/// Generate the main system instruction for the coding agent
pub fn generate_system_instruction(_config: &SystemPromptConfig) -> Content {
    let instruction = read_system_prompt_from_md();
    Content::system_text(instruction)
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
            specialized_instruction
                .push_str("1. **Create reproduction plan** with systematic approach\n");
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
            specialized_instruction
                .push_str("1. **Plan documentation scope** with systematic approach\n");
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
    let instruction = r#"You are a helpful AI coding assistant with comprehensive development tools.

AVAILABLE TOOLS:
- File Operations: list_files, read_file, write_file, edit_file, delete_file
- Search & Analysis: rp_search (ripgrep), codebase_search
- AST-based Code Operations: ast_grep_search, ast_grep_transform
- Advanced Operations: batch_file_operations, extract_dependencies
- Code Quality: cargo check, cargo clippy, cargo fmt
- Terminal Access: run_terminal_cmd for any shell operations
- PTY Access: run_pty_cmd for interactive terminal operations

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
- Prefer early returns over nested ifs to focus on happy paths.

NEWLY ADDED TOOLS:
- **ast_grep_search**: Syntax-aware code search (e.g., "function $name($params) { $ }")
- **ast_grep_transform**: Safe code transformations using AST patterns
- **batch_file_operations**: Perform multiple file operations in one call
- **extract_dependencies**: Extract project dependencies from config files

SECURITY: Only assist with defensive security tasks. Refuse malicious code creation.

Plan your approach systematically throughout the conversation."#;

    Content::system_text(instruction.to_string())
}

/// Generate system instruction for different capability levels
pub fn generate_system_instruction_for_level(level: CapabilityLevel) -> Content {
    let instruction = match level {
        CapabilityLevel::Basic => {
            "You are a helpful AI coding assistant. You can have conversations with the user but have no access to tools."
        }
        CapabilityLevel::FileReading => {
            "You are a helpful AI coding assistant. You can read files in the workspace using the read_file tool."
        }
        CapabilityLevel::FileListing => {
            "You are a helpful AI coding assistant. You can read files and list directory contents using the read_file and list_files tools."
        }
        CapabilityLevel::Bash => {
            "You are a helpful AI coding assistant. You can read files, list directory contents, and run safe bash commands using the read_file, list_files, and bash tools."
        }
        CapabilityLevel::Editing => {
            "You are a helpful AI coding assistant. You can read files, list directory contents, run safe bash commands, and edit files using the read_file, list_files, bash, and edit_file tools."
        }
        CapabilityLevel::CodeSearch => {
            "You are a helpful AI coding assistant. You have full file system access and can read files, list directory contents, run safe bash commands, edit files, and search code using the read_file, list_files, bash, edit_file, and code_search tools. You also have access to advanced AST-based tools: ast_grep_search and ast_grep_transform for syntax-aware code operations."
        }
    };

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

/// Get orchestrator agent prompt
pub fn get_orchestrator_prompt() -> String {
    read_multi_agent_prompt_from_md("orchestrator")
}

/// Get explorer agent prompt
pub fn get_explorer_prompt() -> String {
    read_multi_agent_prompt_from_md("explorer")
}

/// Get coder agent prompt
pub fn get_coder_prompt() -> String {
    read_multi_agent_prompt_from_md("coder")
}

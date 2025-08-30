//! System instructions and prompt management

use crate::gemini::Content;

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

/// Generate the main system instruction for the coding agent
pub fn generate_system_instruction(config: &SystemPromptConfig) -> Content {
    let mut instruction = String::new();

    // Core identity - keep simple
    instruction.push_str("You are a coding assistant with access to development tools.\n\n");

    // Simple tool list
    instruction.push_str("TOOLS:\n");
    instruction.push_str("- list_files: Explore directories\n");
    instruction.push_str("- read_file: Read file contents\n");
    instruction.push_str("- write_file: Create/replace files\n");
    instruction.push_str("- edit_file: Make precise edits\n");
    instruction.push_str("- grep_search: Search text patterns\n\n");

    // Simple guidelines
    instruction.push_str("GUIDELINES:\n");
    instruction.push_str("- Be concise and direct\n");
    instruction.push_str("- Use tools when needed\n");
    instruction.push_str("- Focus on the user's request\n");
    instruction.push_str("- Keep responses brief\n\n");

    Content::system_text(instruction)
}

/// Generate a specialized system instruction for specific tasks
pub fn generate_specialized_instruction(task_type: &str, config: &SystemPromptConfig) -> Content {
    let base_instruction = generate_system_instruction(config);
    let mut specialized_instruction = base_instruction.parts[0].as_text().unwrap().to_string();

    match task_type {
        "analysis" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR ANALYSIS\n");
            specialized_instruction.push_str("- Focus on understanding the codebase structure\n");
            specialized_instruction
                .push_str("- Identify key patterns and architectural decisions\n");
            specialized_instruction.push_str("- Provide comprehensive but concise summaries\n");
            specialized_instruction
                .push_str("- Highlight potential issues and improvement areas\n");
        }
        "debugging" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR DEBUGGING\n");
            specialized_instruction.push_str("- Create minimal reproduction cases\n");
            specialized_instruction.push_str("- Trace error propagation through the codebase\n");
            specialized_instruction.push_str("- Identify root causes, not just symptoms\n");
            specialized_instruction.push_str("- Suggest comprehensive testing strategies\n");
        }
        "refactoring" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR REFACTORING\n");
            specialized_instruction
                .push_str("- Understand existing patterns before making changes\n");
            specialized_instruction.push_str("- Make small, verifiable changes\n");
            specialized_instruction.push_str("- Maintain backward compatibility\n");
            specialized_instruction.push_str("- Update tests and documentation accordingly\n");
        }
        "documentation" => {
            specialized_instruction.push_str("\n\n## SPECIALIZED FOR DOCUMENTATION\n");
            specialized_instruction.push_str("- Focus on clarity and comprehensiveness\n");
            specialized_instruction.push_str("- Include practical examples and use cases\n");
            specialized_instruction.push_str("- Highlight important caveats and limitations\n");
            specialized_instruction.push_str("- Keep documentation up-to-date with code changes\n");
        }
        _ => {
            // Default case - no specialization
        }
    }

    Content::system_text(specialized_instruction)
}

/// Generate a lightweight system instruction for simple tasks
pub fn generate_lightweight_instruction() -> Content {
    let instruction = r#"You are a coding assistant with access to file system tools.

AVAILABLE TOOLS:
- list_files: Explore directories
- read_file: Read file contents
- write_file: Create/replace files
- edit_file: Make precise edits

GUIDELINES:
- Be concise but thorough
- Use tools systematically
- Explain your reasoning
- Handle errors gracefully

Always use absolute paths and test your changes."#;

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

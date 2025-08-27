//! Prompt templates for common scenarios

use serde::{Deserialize, Serialize};

/// Prompt template for common scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
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
2. Create a script to reproduce the error and execute it with \`python <filename.py>\` using the BashTool, to confirm the error
3. Edit the sourcecode of the repo to resolve the issue
4. Rerun your reproduce script and confirm that the error is fixed!
5. Think about edgecases and make sure your fix handles them as well

{additional_context}"#,
            vec!["file_path", "description", "additional_context"]
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
            vec!["feature_description", "requirements", "codebase_info", "additional_context"]
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
            vec!["code", "context"]
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
            vec!["improvement_goal", "file_path", "current_code", "problems"]
        )
    ]
}

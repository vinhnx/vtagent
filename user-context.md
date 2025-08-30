# user-context.md - User Context and Preferences

This file contains user preferences, project conventions, and contextual information to help any AI agent understand your coding style and requirements.

## Raw Configuration

```yaml
user_preferences:
  preferred_languages:
  - Rust
  - Python
  coding_style:
  - Use snake_case for variables and functions
  - Use SCREAMING_SNAKE_CASE for constants
  - Use 4 spaces for indentation
  - Add documentation comments for public APIs
  preferred_libraries: []
  communication_style:
  - Be concise but thorough
  - Explain your reasoning clearly
  - Use examples when helpful
  work_patterns:
  - Start with exploration before making changes
  - Make small, testable changes
  - Test changes incrementally
project_conventions:
  naming_conventions:
  - 'Functions: snake_case'
  - 'Structs/Enums: PascalCase'
  - 'Modules: snake_case'
  - 'Constants: SCREAMING_SNAKE_CASE'
  code_organization:
  - Separate concerns into different modules
  - Group related functionality together
  - Use clear, descriptive names
  documentation_standards:
  - Document public APIs
  - Explain complex logic
  - Include examples for non-trivial functions
  testing_practices:
  - Write tests for new functionality
  - Test error conditions
  - Use descriptive test names
  code_review_preferences:
  - Check for proper error handling
  - Verify naming conventions
  - Ensure documentation is adequate
technical_context:
  target_platforms:
  - Linux/Mac/Windows
  performance_requirements: []
  security_requirements:
  - Validate user inputs
  - Use safe coding practices
  integration_requirements: []
  deployment_constraints: []
custom_instructions:
- 'IMPORTANT: Always use absolute paths when referencing files'
- 'IMPORTANT: Test your changes before declaring them complete'
- 'IMPORTANT: Ask for clarification when requirements are ambiguous'
last_updated: 2025-08-30T09:57:24.002494+00:00
format_version: '1.0'
```

## User Preferences

- Use snake_case for variables and functions
- Use SCREAMING_SNAKE_CASE for constants
- Use 4 spaces for indentation
- Add documentation comments for public APIs

## Project Conventions

- Functions: snake_case
- Structs/Enums: PascalCase
- Modules: snake_case
- Constants: SCREAMING_SNAKE_CASE

## Custom Instructions

- IMPORTANT: Always use absolute paths when referencing files
- IMPORTANT: Test your changes before declaring them complete
- IMPORTANT: Ask for clarification when requirements are ambiguous

# Speckit Integration Guide

## Overview

VTAgent now includes integration with **Speckit** (GitHub's Spec Kit), a tool for spec-driven development project initialization and system verification.

## What is Speckit?

Speckit is GitHub's Spec Kit, a toolkit for spec-driven development that provides:

-   Project initialization with best practices
-   System requirements verification
-   Structured development workflows

## Installation

Speckit is automatically installed when you run the VTAgent setup script:

```bash
./scripts/setup.sh
```

This will install:

-   `uv` (Python package manager)
-   Speckit via `uvx --from git+https://github.com/github/spec-kit.git specify`

## Available Commands

### `speckit init` - Initialize Projects

Initialize a new spec-driven development project with best practices:

```bash
speckit init my-project
# or initialize in current directory
speckit init --here
```

This creates:

-   Project structure with recommended folders
-   Configuration files for spec-driven development
-   Template files for specifications and documentation
-   Git repository setup with appropriate .gitignore

### `speckit check` - System Verification

Verify that your system has all required tools for spec-driven development:

```bash
speckit check
```

This checks for:

-   Git version control system
-   Development tools (Node.js, Python, etc.)
-   IDE integrations (VS Code, Cursor)
-   AI assistants (Claude Code, Gemini CLI)

## VTAgent Integration

### Using Speckit in Chat Mode

In VTAgent's interactive chat mode, you can use Speckit commands directly:

```bash
> speckit check
✓ Command completed successfully
[Output]
Specify CLI is ready to use!

> speckit init todo-app
✓ Command completed successfully
[Output]
Project initialized successfully!
```

### Using Speckit as a Tool

Speckit is also available as a tool in VTAgent's function calling system:

```json
{
    "command": "speckit",
    "args": {
        "command": "check"
    }
}
```

## Workflow Examples

### Example 1: Starting a New Project

```bash
# 1. Check system requirements
speckit check

# 2. Initialize new project
speckit init my-awesome-app

# 3. Start development with proper structure
```

### Example 2: Setting Up Development Environment

```bash
# Verify all tools are available
speckit check

# Initialize project in current directory
speckit init --here
```

## Key Benefits

### 1. **Standardized Project Structure**

-   Consistent folder organization
-   Best practice configurations
-   Proper documentation templates

### 2. **System Verification**

-   Automated dependency checking
-   Clear error messages for missing tools
-   Environment validation

### 3. **Integrated Workflow**

-   Seamless integration with VTAgent
-   Direct command execution
-   Immediate feedback and results

## Best Practices

### Project Initialization

1. **Always Check First**: Run `speckit check` before initializing
2. **Choose Appropriate Location**: Use `--here` for current directory or specify project name
3. **Review Generated Structure**: Examine the created files and folders

### Development Workflow

1. **Start with Check**: Verify your environment is ready
2. **Initialize Project**: Set up proper structure from the beginning
3. **Follow Templates**: Use the generated templates as starting points

## Troubleshooting

### Common Issues

1. **Command Not Found**

    ```bash
    # Reinstall Speckit
    uvx --from git+https://github.com/github/spec-kit.git specify --help
    ```

2. **Permission Issues**

    - Ensure you have write access to the target directory
    - Check that git is properly configured

3. **Missing Dependencies**
    - Run `speckit check` to identify missing tools
    - Install required dependencies based on the check results

### Getting Help

-   Speckit Documentation: https://github.com/github/spec-kit
-   VTAgent Issues: https://github.com/vinhnx/vtagent/issues
-   Community Support: Open GitHub issues for both projects

## Advanced Usage

### Custom Project Templates

Speckit supports custom project templates. You can:

1. Create your own template structure
2. Configure specific toolchains and dependencies
3. Set up custom documentation formats

### Integration with CI/CD

Incorporate Speckit into your development pipeline:

```yaml
# .github/workflows/setup.yml
name: Setup Development Environment
on: [push, pull_request]
jobs:
    setup:
        runs-on: ubuntu-latest
        steps:
            - uses: actions/checkout@v3
            - name: Check Development Environment
              run: uvx --from git+https://github.com/github/spec-kit.git specify check
            - name: Initialize Project Structure
              run: uvx --from git+https://github.com/github/spec-kit.git specify init --here
```

This integration provides a solid foundation for spec-driven development by ensuring proper project setup and system verification, making it easier to maintain consistent development practices across projects.

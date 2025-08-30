//! Init command implementation - project analysis and AGENTS.md generation

use crate::tools::ToolRegistry;
use anyhow::Result;
use console::style;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

/// Project analysis result
#[derive(Debug)]
struct ProjectAnalysis {
    languages: Vec<String>,
    frameworks: Vec<String>,
    build_systems: Vec<String>,
    dependencies: HashMap<String, Vec<String>>,
    source_dirs: Vec<String>,
    test_patterns: Vec<String>,
    config_files: Vec<String>,
    documentation_files: Vec<String>,
    conventions: Vec<String>,
}

/// Handle the init command - analyze project and generate AGENTS.md
pub async fn handle_init_command(registry: &mut ToolRegistry, workspace: &PathBuf) -> Result<()> {
    println!(
        "{}",
        style("Initializing project with AGENTS.md...")
            .cyan()
            .bold()
    );

    // Step 1: Analyze the project structure
    println!("{}", style("1. Analyzing project structure...").dim());
    let analysis = analyze_project(registry, workspace).await?;

    // Step 2: Generate AGENTS.md content
    println!("{}", style("2. Generating AGENTS.md content...").dim());
    let agents_md_content = generate_agents_md(&analysis)?;

    // Step 3: Write AGENTS.md file
    println!("{}", style("3. Writing AGENTS.md file...").dim());
    let agents_md_path = workspace.join("AGENTS.md");

    registry
        .execute_tool(
            "write_file",
            json!({
                "path": agents_md_path.to_string_lossy(),
                "content": agents_md_content,
                "overwrite": true
            }),
        )
        .await?;

    println!(
        "{} {}",
        style("✓").green().bold(),
        style("AGENTS.md generated successfully!").green()
    );
    println!(
        "{} {}",
        style(" Location:").blue(),
        agents_md_path.display()
    );

    Ok(())
}

/// Analyze the current project structure
async fn analyze_project(
    registry: &mut ToolRegistry,
    workspace: &PathBuf,
) -> Result<ProjectAnalysis> {
    let mut analysis = ProjectAnalysis {
        languages: Vec::new(),
        frameworks: Vec::new(),
        build_systems: Vec::new(),
        dependencies: HashMap::new(),
        source_dirs: Vec::new(),
        test_patterns: Vec::new(),
        config_files: Vec::new(),
        documentation_files: Vec::new(),
        conventions: Vec::new(),
    };

    // Analyze root directory structure
    let root_files = registry
        .execute_tool("list_files", json!({"path": ".", "max_items": 100}))
        .await?;

    if let Some(files) = root_files.get("files") {
        if let Some(files_array) = files.as_array() {
            for file_obj in files_array {
                if let Some(path) = file_obj.get("path").and_then(|p| p.as_str()) {
                    analyze_file(&mut analysis, path, registry).await?;
                }
            }
        }
    }

    // Detect common source directories
    let common_src_dirs = vec!["src", "lib", "pkg", "internal", "cmd", "app", "core"];
    for dir in common_src_dirs {
        if workspace.join(dir).exists() {
            analysis.source_dirs.push(dir.to_string());
        }
    }

    // Detect test patterns
    let test_patterns = vec!["test_", "_test", ".test.", ".spec.", "__tests__"];
    for pattern in test_patterns {
        if files_contain_pattern(&analysis, pattern) {
            analysis.test_patterns.push(pattern.to_string());
        }
    }

    Ok(analysis)
}

/// Analyze individual files to detect languages, frameworks, etc.
async fn analyze_file(
    analysis: &mut ProjectAnalysis,
    path: &str,
    registry: &mut ToolRegistry,
) -> Result<()> {
    match path {
        // Rust project files
        "Cargo.toml" => {
            analysis.languages.push("Rust".to_string());
            analysis.build_systems.push("Cargo".to_string());

            // Read Cargo.toml to extract dependencies
            let cargo_content = registry
                .execute_tool(
                    "read_file",
                    json!({"path": "Cargo.toml", "max_bytes": 5000}),
                )
                .await?;

            if let Some(content) = cargo_content.get("content").and_then(|c| c.as_str()) {
                extract_cargo_dependencies(analysis, content);
            }
        }
        "Cargo.lock" => {
            analysis.config_files.push("Cargo.lock".to_string());
        }

        // Node.js project files
        "package.json" => {
            analysis.languages.push("JavaScript/TypeScript".to_string());
            analysis.build_systems.push("npm/yarn/pnpm".to_string());

            // Read package.json to extract dependencies
            let package_content = registry
                .execute_tool(
                    "read_file",
                    json!({"path": "package.json", "max_bytes": 5000}),
                )
                .await?;

            if let Some(content) = package_content.get("content").and_then(|c| c.as_str()) {
                extract_package_dependencies(analysis, content);
            }
        }
        "yarn.lock" | "package-lock.json" | "pnpm-lock.yaml" => {
            analysis.config_files.push(path.to_string());
        }

        // Python project files
        "requirements.txt" | "pyproject.toml" | "setup.py" | "Pipfile" => {
            if !analysis.languages.contains(&"Python".to_string()) {
                analysis.languages.push("Python".to_string());
            }
            analysis.build_systems.push("pip/poetry".to_string());
            analysis.config_files.push(path.to_string());
        }

        // Go project files
        "go.mod" | "go.sum" => {
            analysis.languages.push("Go".to_string());
            analysis.build_systems.push("Go Modules".to_string());
            analysis.config_files.push(path.to_string());
        }

        // Java project files
        "pom.xml" | "build.gradle" | "build.gradle.kts" => {
            analysis.languages.push("Java/Kotlin".to_string());
            analysis.build_systems.push("Maven/Gradle".to_string());
            analysis.config_files.push(path.to_string());
        }

        // Documentation files
        "README.md" | "CHANGELOG.md" | "CONTRIBUTING.md" => {
            analysis.documentation_files.push(path.to_string());
        }

        // Configuration files
        ".gitignore" | ".editorconfig" | ".prettierrc" | ".eslintrc" => {
            analysis.config_files.push(path.to_string());
        }

        // Source directories
        "src" | "lib" | "pkg" | "internal" | "cmd" => {
            analysis.source_dirs.push(path.to_string());
        }

        _ => {}
    }

    Ok(())
}

/// Extract dependencies from Cargo.toml
fn extract_cargo_dependencies(analysis: &mut ProjectAnalysis, content: &str) {
    let mut deps = Vec::new();

    // Simple regex-like parsing for dependencies
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('"') && line.contains(" = ") {
            if let Some(dep_name) = line.split('"').nth(1) {
                deps.push(dep_name.to_string());
            }
        }
    }

    if !deps.is_empty() {
        analysis
            .dependencies
            .insert("Rust (Cargo)".to_string(), deps);
    }
}

/// Extract dependencies from package.json
fn extract_package_dependencies(analysis: &mut ProjectAnalysis, content: &str) {
    let mut deps = Vec::new();

    // Simple parsing for dependencies
    if content.contains("\"dependencies\":") {
        // Extract dependency names from JSON
        for line in content.lines() {
            if line.contains("\"")
                && line.contains(":")
                && !line.contains("{")
                && !line.contains("}")
            {
                if let Some(dep_name) = line.split('"').nth(1) {
                    if !dep_name.is_empty()
                        && dep_name != "dependencies"
                        && dep_name != "devDependencies"
                    {
                        deps.push(dep_name.to_string());
                    }
                }
            }
        }
    }

    if !deps.is_empty() {
        analysis
            .dependencies
            .insert("JavaScript/TypeScript (npm)".to_string(), deps);
    }
}

/// Check if files contain a specific pattern
fn files_contain_pattern(_analysis: &ProjectAnalysis, _pattern: &str) -> bool {
    // This is a simplified implementation
    // In a real implementation, you'd scan actual files for patterns
    true // Placeholder
}

/// Generate AGENTS.md content based on project analysis
fn generate_agents_md(analysis: &ProjectAnalysis) -> Result<String> {
    let mut content = String::new();

    // Header
    content.push_str("# AGENTS.md\n\n");
    content.push_str("## Project Context\n\n");

    // Languages
    if !analysis.languages.is_empty() {
        content.push_str("This is a ");
        content.push_str(&analysis.languages.join("/"));
        content.push_str(" project");

        if !analysis.frameworks.is_empty() {
            content.push_str(" using ");
            content.push_str(&analysis.frameworks.join(", "));
        }
        content.push_str(".\n\n");
    }

    // Build systems
    if !analysis.build_systems.is_empty() {
        content.push_str("### Build Systems\n");
        for system in &analysis.build_systems {
            content.push_str(&format!("- {}\n", system));
        }
        content.push_str("\n");
    }

    // Dependencies
    if !analysis.dependencies.is_empty() {
        content.push_str("### Key Dependencies\n");
        for (category, deps) in &analysis.dependencies {
            content.push_str(&format!("**{}:**\n", category));
            for dep in deps.iter().take(10) {
                // Limit to first 10
                content.push_str(&format!("- {}\n", dep));
            }
            if deps.len() > 10 {
                content.push_str(&format!("- ... and {} more\n", deps.len() - 10));
            }
            content.push_str("\n");
        }
    }

    // Code Style and Standards
    content.push_str("## Code Style and Standards\n\n");

    if analysis.languages.contains(&"Rust".to_string()) {
        content.push_str("### Rust Conventions\n");
        content.push_str("- Follow standard Rust naming conventions (snake_case for functions/variables, PascalCase for types)\n");
        content.push_str("- Use `anyhow` for error handling with descriptive error messages\n");
        content.push_str("- Prefer `thiserror` for custom error types when needed\n");
        content.push_str("- Use `clap` with derive macros for CLI argument parsing\n");
        content.push_str("- Follow the Rust API guidelines for public APIs\n\n");
    }

    if analysis
        .languages
        .contains(&"JavaScript/TypeScript".to_string())
    {
        content.push_str("### JavaScript/TypeScript Conventions\n");
        content.push_str("- Use camelCase for variables and functions\n");
        content.push_str("- Use PascalCase for classes and interfaces\n");
        content.push_str("- Follow ESLint configuration for code style\n");
        content.push_str("- Use TypeScript for type safety\n\n");
    }

    if analysis.languages.contains(&"Python".to_string()) {
        content.push_str("### Python Conventions\n");
        content.push_str("- Follow PEP 8 style guidelines\n");
        content.push_str("- Use snake_case for variables and functions\n");
        content.push_str("- Use PascalCase for classes\n");
        content.push_str("- Include docstrings for functions and classes\n\n");
    }

    if analysis.languages.contains(&"Go".to_string()) {
        content.push_str("### Go Conventions\n");
        content.push_str("- Follow standard Go formatting (gofmt)\n");
        content.push_str("- Use goimports for import organization\n");
        content.push_str("- Follow Go naming conventions\n");
        content.push_str("- Use go mod for dependency management\n\n");
    }

    // Code Organization
    content.push_str("### Code Organization\n");

    if !analysis.source_dirs.is_empty() {
        content.push_str("- Source code is organized in: ");
        content.push_str(&analysis.source_dirs.join(", "));
        content.push_str("\n");
    }

    if !analysis.test_patterns.is_empty() {
        content.push_str("- Test files follow patterns: ");
        content.push_str(&analysis.test_patterns.join(", "));
        content.push_str("\n");
    }

    content.push_str("- Keep modules focused and cohesive\n");
    content.push_str("- Use clear separation of concerns\n");
    content.push_str("- Document public APIs with comments\n\n");

    // Dependencies Management
    if !analysis.build_systems.is_empty() {
        content.push_str("### Dependencies\n");
        for system in &analysis.build_systems {
            match system.as_str() {
                "Cargo" => {
                    content.push_str("- Use `cargo add` to add dependencies\n");
                    content.push_str("- Run `cargo update` to update dependencies\n");
                    content.push_str("- Use `cargo tree` to visualize dependency tree\n");
                }
                "npm/yarn/pnpm" => {
                    content.push_str(
                        "- Use `npm install`/`yarn add`/`pnpm add` to add dependencies\n",
                    );
                    content.push_str("- Keep package.json and lock files in sync\n");
                    content.push_str("- Use `npm audit` to check for security vulnerabilities\n");
                }
                "pip/poetry" => {
                    content.push_str("- Use `pip install` or `poetry add` to add dependencies\n");
                    content.push_str("- Keep requirements.txt/pyproject.toml up to date\n");
                    content.push_str("- Use virtual environments for isolation\n");
                }
                "Go Modules" => {
                    content.push_str("- Use `go get` to add dependencies\n");
                    content.push_str("- Keep go.mod and go.sum consistent\n");
                    content.push_str("- Use `go mod tidy` to clean up dependencies\n");
                }
                "Maven/Gradle" => {
                    content.push_str("- Use Maven or Gradle for dependency management\n");
                    content.push_str("- Keep build files synchronized\n");
                    content.push_str("- Use dependency check tools for security\n");
                }
                _ => {}
            }
        }
        content.push_str("\n");
    }

    // Development Guidelines
    content.push_str("## Development Guidelines\n\n");

    content.push_str("### Error Handling\n");
    content.push_str("- Provide meaningful error messages with context\n");
    content.push_str("- Use appropriate error types for different scenarios\n");
    content.push_str("- Log errors appropriately for debugging\n\n");

    content.push_str("### Testing\n");
    if !analysis.test_patterns.is_empty() {
        content.push_str("- Write tests following established patterns\n");
    }
    content.push_str("- Include unit tests for critical functionality\n");
    content.push_str("- Write integration tests for complex workflows\n");
    content.push_str("- Use descriptive test names that explain the expected behavior\n\n");

    content.push_str("### Documentation\n");
    if !analysis.documentation_files.is_empty() {
        content.push_str("- Maintain documentation in: ");
        content.push_str(&analysis.documentation_files.join(", "));
        content.push_str("\n");
    }
    content.push_str("- Document public APIs and complex logic\n");
    content.push_str("- Keep README and other docs up to date\n");
    content.push_str("- Include code comments for non-obvious implementations\n\n");

    // File Organization
    content.push_str("## File Organization\n\n");
    content.push_str("- Group related functionality together\n");
    content.push_str("- Use consistent directory structure\n");
    content.push_str("- Separate source code, tests, and configuration\n");
    content.push_str("- Use clear naming conventions for files and directories\n\n");

    // When Making Changes
    content.push_str("## When Making Changes\n\n");
    content.push_str("- Ensure all tests pass before committing\n");
    content.push_str("- Follow established code style guidelines\n");
    content.push_str("- Update documentation when changing APIs\n");
    content.push_str("- Consider backward compatibility\n");
    content.push_str("- Test changes in a development environment first\n");
    content.push_str("- Use version control appropriately (meaningful commit messages)\n\n");

    // Tool Integration
    content.push_str("## Tool Integration\n\n");
    content.push_str("This project is designed to work with AI coding assistants like vtagent.\n");
    content.push_str("The guidelines in this document help ensure consistent and maintainable code generation.\n\n");

    content.push_str("### AI Assistant Guidelines\n");
    content.push_str("- Follow the established code style and conventions\n");
    content.push_str("- Use the specified build systems and dependency management tools\n");
    content.push_str("- Maintain the existing file organization structure\n");
    content.push_str("- Include appropriate error handling and logging\n");
    content.push_str("- Write tests for new functionality\n");
    content.push_str("- Update documentation when making significant changes\n\n");

    // Footer
    content.push_str("---\n\n");
    content.push_str("*This AGENTS.md file was auto-generated by vtagent init command.*\n");
    content.push_str("*Last updated: Auto-generated*\n");

    Ok(content)
}

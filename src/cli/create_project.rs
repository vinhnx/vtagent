use anyhow::Result;
use console::style;
use itertools::Itertools;
use std::fs;
use std::path::Path;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;

/// Handle the create-project command
pub async fn handle_create_project_command(
    config: &CoreAgentConfig,
    name: &str,
    features: &[String],
) -> Result<()> {
    println!("{}", style("Create project mode selected").blue().bold());
    println!("Project name: {}", name);
    println!("Features: {:?}", features);
    println!("Workspace: {}", config.workspace.display());

    // Project creation implementation
    let project_path = config.workspace.join(name);

    // Create project directory
    fs::create_dir_all(&project_path)?;
    println!("Created project directory: {}", project_path.display());

    // Create basic project structure based on features
    create_project_structure(&project_path, features)?;

    // Create vtagent configuration
    create_vtagent_config(&project_path)?;

    println!("Project '{}' created successfully!", name);

    Ok(())
}

/// Create project structure based on selected features
fn create_project_structure(project_path: &Path, features: &[String]) -> Result<()> {
    // Create src directory
    let src_path = project_path.join("src");
    fs::create_dir_all(&src_path)?;

    // Create main file (simple template; extend later for feature-specific scaffolds)
    let main_content = r#"fn main() {
    println!("Hello, world!");
}
"#;

    fs::write(src_path.join("main.rs"), main_content)?;

    // Create README.md
    let readme_content = format!(
        r#"# Project

This is a new project created with VTAgent.

## Features

{}"#,
        features.iter().map(|f| format!("- {}\n", f)).join("")
    );

    fs::write(project_path.join("README.md"), readme_content)?;

    // Create Cargo.toml for Rust projects
    let cargo_content = r#"[package]
name = "project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;

    fs::write(project_path.join("Cargo.toml"), cargo_content)?;

    Ok(())
}

/// Create vtagent configuration file
fn create_vtagent_config(project_path: &Path) -> Result<()> {
    let config_content = r#"# VTAgent Configuration
[model]
name = "gemini-1.5-flash"

[workspace]
path = "."

[agent]
verbose = false
"#;

    fs::write(project_path.join("vtagent.toml"), config_content)?;
    Ok(())
}

//! Init-project command implementation

use anyhow::Result;
use console::style;
use std::path::Path;
use vtagent_core::{ProjectData, SimpleProjectManager};

/// Handle the init-project command
pub async fn handle_init_project_command(
    name: Option<String>,
    force: bool,
    migrate: bool,
) -> Result<()> {
    println!(
        "{}",
        style("Initialize project with dot-folder structure")
            .blue()
            .bold()
    );

    // Initialize project manager
    let project_manager = SimpleProjectManager::new(std::env::current_dir()?);
    project_manager.init()?;

    // Determine project name
    let project_name = if let Some(name) = name {
        name
    } else {
        // Use current directory name
        let current_dir = std::env::current_dir()?;
        current_dir
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| "unnamed-project".to_string())
    };

    println!("Project name: {}", project_name);

    // Check if project already exists
    let project_dir = project_manager.project_data_dir(&project_name);
    if project_dir.exists() && !force {
        println!(
            "{} Project directory already exists: {}",
            style("Warning").yellow(),
            project_dir.display()
        );
        println!("Use --force to overwrite existing project structure.");
        return Ok(());
    }

    // Create project structure
    project_manager.create_project(&project_name, Some("VTAgent project"))?;
    println!(
        "{} Created project structure in: {}",
        style("Success").green(),
        project_dir.display()
    );

    // Create or update project metadata
    let current_dir = std::env::current_dir()?;
    let mut metadata = ProjectData::new(&project_name);
    metadata.description = Some("VTAgent project".to_string());
    project_manager.update_project(&metadata)?;
    println!("{} Created project metadata", style("Success").green());

    // Migrate existing files if requested
    if migrate {
        migrate_existing_files(&project_manager, &project_name, &current_dir).await?;
    }

    println!(
        "\n{} Project initialization completed!",
        style("Success").green().bold()
    );
    println!("Project structure created at: {}", project_dir.display());
    println!(
        "Configuration directory: {}",
        project_manager.config_dir(&project_name).display()
    );
    println!(
        "Cache directory: {}",
        project_manager.cache_dir(&project_name).display()
    );

    Ok(())
}

/// Migrate existing config/cache files to the new project structure
async fn migrate_existing_files(
    _project_manager: &SimpleProjectManager,
    _project_name: &str,
    current_dir: &Path,
) -> Result<()> {
    println!(
        "\n{} Checking for existing config/cache files to migrate...",
        style("Info").blue()
    );

    let mut files_to_migrate = Vec::new();

    // Check for vtagent.toml in current directory
    let local_config = current_dir.join("vtagent.toml");
    if local_config.exists() {
        files_to_migrate.push(("vtagent.toml", local_config.clone()));
    }

    // Check for .vtagent directory
    let local_vtagent = current_dir.join(".vtagent");
    if local_vtagent.exists() && local_vtagent.is_dir() {
        // Look for config files in .vtagent directory
        let vtagent_config = local_vtagent.join("vtagent.toml");
        if vtagent_config.exists() {
            files_to_migrate.push(("vtagent.toml (from .vtagent)", vtagent_config));
        }

        let vtagent_gitignore = local_vtagent.join(".vtagentgitignore");
        if vtagent_gitignore.exists() {
            files_to_migrate.push((".vtagentgitignore (from .vtagent)", vtagent_gitignore));
        }
    }

    // Check for common cache directories
    let cache_dirs = ["cache", ".cache"];
    for cache_dir_name in &cache_dirs {
        let cache_dir = current_dir.join(cache_dir_name);
        if cache_dir.exists() && cache_dir.is_dir() {
            files_to_migrate.push((cache_dir_name, cache_dir));
        }
    }

    // Check for common config directories
    let config_dirs = ["config", ".config"];
    for config_dir_name in &config_dirs {
        let config_dir = current_dir.join(config_dir_name);
        if config_dir.exists() && config_dir.is_dir() {
            files_to_migrate.push((config_dir_name, config_dir));
        }
    }

    if files_to_migrate.is_empty() {
        println!("No existing config/cache files found to migrate.");
        return Ok(());
    }

    println!("Found {} items to migrate:", files_to_migrate.len());
    for (name, path) in &files_to_migrate {
        println!("  - {} ({})", name, path.display());
    }

    // In a real implementation, we would:
    // 1. Prompt user for confirmation
    // 2. Backup original files
    // 3. Copy/move files to appropriate project directories
    // 4. Update any relative paths in config files

    println!("\nMigration functionality would be implemented here in a full version.");
    println!("In a complete implementation, this would:");
    println!("  • Prompt for user confirmation before migration");
    println!("  • Backup original files before migration");
    println!("  • Copy/move files to appropriate project directories");
    println!("  • Update any relative paths in config files");

    Ok(())
}

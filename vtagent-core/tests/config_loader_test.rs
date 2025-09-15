//! Test for configuration loading with home directory support

use std::fs;
use std::path::Path;
use tempfile::TempDir;
use vtagent_core::config::{ConfigManager, VTAgentConfig};

#[test]
fn test_load_config_from_home_directory() {
    // Create a temporary directory to simulate home directory
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let home_dir = temp_dir.path();
    let vtagent_dir = home_dir.join(".vtagent");
    fs::create_dir_all(&vtagent_dir).expect("Failed to create .vtagent directory");

    // Create a sample config file in the home directory
    let config_content = r#"
[agent]
default_model = "test-model"
provider = "test-provider"
max_conversation_turns = 50

[security]
human_in_the_loop = false
"#;

    let config_path = vtagent_dir.join("vtagent.toml");
    fs::write(&config_path, config_content).expect("Failed to write config file");

    // Create a mock workspace directory
    let workspace_dir = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace_dir).expect("Failed to create workspace directory");

    // Test that we can load the config from home directory
    // We need to mock the home directory detection for this test
    // Since we can't easily mock environment variables in a test,
    // we'll directly test the bootstrap function with our temp directory
    let created_files = VTAgentConfig::bootstrap_project_with_options(
        &workspace_dir,
        true, // force
        true, // use home directory
    )
    .expect("Failed to bootstrap project with home directory");

    assert!(!created_files.is_empty());
    assert!(created_files.contains(&"vtagent.toml".to_string()));

    // Check that the files were created in the home directory
    let home_config_path = vtagent_dir.join("vtagent.toml");
    let home_gitignore_path = vtagent_dir.join(".vtagentgitignore");
    assert!(home_config_path.exists());
    assert!(home_gitignore_path.exists());
}

#[test]
fn test_get_home_dir() {
    // Test that the get_home_dir function works (basic test)
    let home_dir = ConfigManager::get_home_dir();
    // This should return Some on most systems
    assert!(home_dir.is_some());
}

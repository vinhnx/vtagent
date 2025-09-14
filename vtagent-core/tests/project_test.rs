//! Test for project management functionality

use std::fs;
use tempfile::TempDir;
use vtagent_core::project::{CacheEntry, FileCache, ProjectManager, ProjectMetadata};

#[test]
fn test_project_manager_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let projects_dir = temp_dir.path().join("projects");

    let manager = ProjectManager {
        home_dir: temp_dir.path().to_path_buf(),
        projects_dir: projects_dir.clone(),
    };

    assert_eq!(manager.home_dir(), temp_dir.path());
    assert_eq!(manager.projects_dir(), projects_dir);
}

#[test]
fn test_project_structure_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let projects_dir = temp_dir.path().join("projects");

    let manager = ProjectManager {
        home_dir: temp_dir.path().to_path_buf(),
        projects_dir: projects_dir.clone(),
    };

    let project_name = "test-project";
    manager
        .create_project_structure(project_name)
        .expect("Failed to create project structure");

    let project_dir = manager.project_dir(project_name);
    assert!(project_dir.exists());
    assert!(manager.config_dir(project_name).exists());
    assert!(manager.cache_dir(project_name).exists());
}

#[test]
fn test_project_metadata() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let projects_dir = temp_dir.path().join("projects");

    let manager = ProjectManager {
        home_dir: temp_dir.path().to_path_buf(),
        projects_dir: projects_dir.clone(),
    };

    let project_name = "test-project";
    let metadata = ProjectMetadata::new(project_name.to_string(), "/test/path".to_string());

    manager
        .save_project_metadata(project_name, &metadata)
        .expect("Failed to save metadata");
    let loaded_metadata = manager
        .load_project_metadata(project_name)
        .expect("Failed to load metadata")
        .expect("Metadata should exist");

    assert_eq!(loaded_metadata.name, project_name);
    assert_eq!(loaded_metadata.root_path, "/test/path");
}

#[test]
fn test_project_identification() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let projects_dir = temp_dir.path().join("projects");

    let manager = ProjectManager {
        home_dir: temp_dir.path().to_path_buf(),
        projects_dir: projects_dir.clone(),
    };

    // Test identification from directory name
    let project_name = manager
        .identify_project(temp_dir.path())
        .expect("Failed to identify project");
    assert_eq!(
        project_name,
        temp_dir.path().file_name().unwrap().to_str().unwrap()
    );

    // Test identification from .project file
    let metadata = ProjectMetadata::new(
        "custom-project".to_string(),
        temp_dir.path().display().to_string(),
    );
    let project_file = temp_dir.path().join(".project");
    let metadata_json =
        serde_json::to_string_pretty(&metadata).expect("Failed to serialize metadata");
    fs::write(&project_file, metadata_json).expect("Failed to write .project file");

    let project_name = manager
        .identify_project(temp_dir.path())
        .expect("Failed to identify project");
    assert_eq!(project_name, "custom-project");
}

#[test]
fn test_file_cache() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cache_dir = temp_dir.path().join("cache");

    let cache = FileCache::new(cache_dir.clone()).expect("Failed to create cache");

    // Test setting and getting cache entries
    cache
        .set("test_key", "test_value", Some(60))
        .expect("Failed to set cache entry");

    let value: Option<String> = cache.get("test_key").expect("Failed to get cache entry");
    assert_eq!(value, Some("test_value".to_string()));

    // Test invalidation
    cache
        .invalidate("test_key")
        .expect("Failed to invalidate cache entry");

    let value: Option<String> = cache.get("test_key").expect("Failed to get cache entry");
    assert_eq!(value, None);

    // Test cache statistics
    cache
        .set("key1", "value1", Some(60))
        .expect("Failed to set cache entry");
    cache
        .set("key2", "value2", Some(60))
        .expect("Failed to set cache entry");

    let (total, expired) = cache.stats().expect("Failed to get cache stats");
    assert_eq!(total, 2);
    assert_eq!(expired, 0);
}
#![cfg(any())]

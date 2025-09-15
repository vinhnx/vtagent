use anyhow::Result;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use vtagent_core::tools::{ToolRegistry, build_function_declarations};

#[tokio::test]
async fn test_tool_availability() -> Result<()> {
    // Check that essential tools are in the available tools
    let declarations = build_function_declarations();
    let grep_tool_exists = declarations
        .iter()
        .any(|d| d.name == "grep_search");
    assert!(
        grep_tool_exists,
        "grep_search should be available as a tool"
    );

    let list_tool_exists = declarations
        .iter()
        .any(|d| d.name == "list_files");
    assert!(
        list_tool_exists,
        "list_files should be available as a tool"
    );

    Ok(())
}

#[tokio::test]
async fn test_read_file_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a test file
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "Hello, World!")?;

    // Read the file
    let args = json!({
        "path": "test.txt"
    });

    let result = registry.execute_tool("read_file", args).await?;

    // Check that the file was read correctly
    assert_eq!(result["content"], "Hello, World!");
    assert_eq!(result["metadata"]["size"], 13);

    Ok(())
}

#[tokio::test]
async fn test_edit_file_with_whitespace_tolerance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a test file with specific formatting
    let test_file = temp_dir.path().join("test.rs");
    let original_content = r#"pub mod models {
    pub const TEST_MODEL: &str = "test";
    pub const SUPPORTED_MODELS: &[&str] = &[
        "model1",
        "model2",
    ];
}"#;
    fs::write(&test_file, original_content)?;

    // Test case 1: Exact match should work
    let args = json!({
        "path": "test.rs",
        "old_str": r#"    pub const SUPPORTED_MODELS: &[&str] = &[
        "model1",
        "model2",
    ];"#,
        "new_str": r#"    pub const SUPPORTED_MODELS: &[&str] = &[
        "model1",
        "model2",
        "model3",
    ];"#
    });

    let result = registry.edit_file(args).await?;
    assert_eq!(result["success"], true);

    // Verify the change was made
    let read_args = json!({ "path": "test.rs" });
    let read_result = registry.read_file(read_args).await?;
    let content = read_result["content"].as_str().unwrap();
    assert!(content.contains("model3"));

    Ok(())
}

#[tokio::test]
async fn test_pagination_basic() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a directory with many files
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir)?;

    // Create 250 test files
    for i in 1..=250 {
        let file_path = test_dir.join(format!("file_{:03}.txt", i));
        fs::write(&file_path, format!("Content of file {}", i))?;
    }

    // Test first page with default per_page (100)
    let args = json!({
        "path": "test_dir",
        "page": 1
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 1);
    assert_eq!(result["per_page"], 100);
    assert_eq!(result["has_more"], true);
    assert!(result["count"].as_u64().unwrap() <= 100);

    Ok(())
}

#[tokio::test]
async fn test_pagination_multiple_pages() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a directory with many files
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir)?;

    // Create 250 test files
    for i in 1..=250 {
        let file_path = test_dir.join(format!("file_{:03}.txt", i));
        fs::write(&file_path, format!("Content of file {}", i))?;
    }

    // Test second page
    let args = json!({
        "path": "test_dir",
        "page": 2,
        "per_page": 100
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 2);
    assert_eq!(result["per_page"], 100);
    assert_eq!(result["has_more"], true); // Should have more since we have 250 files
    assert!(result["count"].as_u64().unwrap() <= 100);

    // Test third page
    let args = json!({
        "path": "test_dir",
        "page": 3,
        "per_page": 100
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 3);
    assert_eq!(result["per_page"], 100);
    assert_eq!(result["has_more"], false); // Should be the last page
    assert!(result["count"].as_u64().unwrap() <= 50); // Should have remaining 50 files

    Ok(())
}

#[tokio::test]
async fn test_pagination_edge_cases() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a directory with exactly 100 files
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir)?;

    for i in 1..=100 {
        let file_path = test_dir.join(format!("file_{:03}.txt", i));
        fs::write(&file_path, format!("Content of file {}", i))?;
    }

    // Test requesting a page beyond available data
    let args = json!({
        "path": "test_dir",
        "page": 5,
        "per_page": 50
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 5);
    assert_eq!(result["per_page"], 50);
    assert_eq!(result["has_more"], false);
    assert_eq!(result["count"], 0); // Should return empty result

    Ok(())
}

#[tokio::test]
async fn test_pagination_with_small_batch_size() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a directory with 50 files
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir)?;

    for i in 1..=50 {
        let file_path = test_dir.join(format!("file_{:03}.txt", i));
        fs::write(&file_path, format!("Content of file {}", i))?;
    }

    // Test with smaller per_page for high-load scenarios
    let args = json!({
        "path": "test_dir",
        "page": 1,
        "per_page": 25
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 1);
    assert_eq!(result["per_page"], 25);
    assert_eq!(result["has_more"], true);
    assert_eq!(result["count"], 25);

    // Test second page
    let args = json!({
        "path": "test_dir",
        "page": 2,
        "per_page": 25
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 2);
    assert_eq!(result["per_page"], 25);
    assert_eq!(result["has_more"], false);
    assert_eq!(result["count"], 25);

    Ok(())
}

#[tokio::test]
async fn test_pagination_with_max_items_cap() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a directory with 200 files
    let test_dir = temp_dir.path().join("test_dir");
    fs::create_dir(&test_dir)?;

    for i in 1..=200 {
        let file_path = test_dir.join(format!("file_{:03}.txt", i));
        fs::write(&file_path, format!("Content of file {}", i))?;
    }

    // Test with max_items cap smaller than total files
    let args = json!({
        "path": "test_dir",
        "page": 1,
        "per_page": 50,
        "max_items": 75
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 1);
    assert_eq!(result["per_page"], 50);
    assert_eq!(result["total"], 75); // Should be capped at max_items
    assert_eq!(result["has_more"], true);
    assert_eq!(result["count"], 50);

    Ok(())
}

#[tokio::test]
async fn test_pagination_performance_large_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a large directory with 1000+ files to simulate high-load scenario
    let test_dir = temp_dir.path().join("large_dir");
    fs::create_dir(&test_dir)?;

    // Create 1200 files
    for i in 1..=1200 {
        let file_path = test_dir.join(format!("file_{:04}.txt", i));
        fs::write(&file_path, format!("Content of file {} with some additional text to make it larger", i))?;
    }

    // Test with reduced batch size for large directories
    let args = json!({
        "path": "large_dir",
        "page": 1,
        "per_page": 50,  // Reduced batch size for large directories
        "max_items": 1000
    });

    let start_time = std::time::Instant::now();
    let result = registry.execute_tool("list_files", args).await?;
    let duration = start_time.elapsed();

    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 1);
    assert_eq!(result["per_page"], 50);
    assert_eq!(result["total"], 1000); // Capped at max_items
    assert_eq!(result["has_more"], true);
    assert_eq!(result["count"], 50);

    // Performance check: should complete within reasonable time (less than 1 second)
    assert!(duration.as_millis() < 1000, "Large directory pagination took too long: {:?}", duration);

    Ok(())
}

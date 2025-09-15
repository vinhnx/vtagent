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

    // Test first page with default per_page (50)
    let args = json!({
        "path": "test_dir",
        "page": 1
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 1);
    assert_eq!(result["per_page"], 50);
    assert_eq!(result["has_more"], true);
    assert!(result["count"].as_u64().unwrap() <= 50);

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
        "per_page": 50
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 2);
    assert_eq!(result["per_page"], 50);
    assert_eq!(result["has_more"], true); // Should have more since we have 250 files
    assert!(result["count"].as_u64().unwrap() <= 50);

    // Test third page
    let args = json!({
        "path": "test_dir",
        "page": 3,
        "per_page": 50
    });

    let result = registry.execute_tool("list_files", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["page"], 3);
    assert_eq!(result["per_page"], 50);
    assert_eq!(result["has_more"], true); // Should NOT be the last page (250 total files, 50 per page)
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

#[tokio::test]
async fn test_read_file_chunking_large_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a large test file (>10,000 lines)
    let test_file = temp_dir.path().join("large_file.txt");
    let mut content = String::new();

    // Create 15,000 lines of content
    for i in 1..=15_000 {
        content.push_str(&format!("Line {}: This is a test line with some content\n", i));
    }

    fs::write(&test_file, &content)?;

    // Read the file (should be automatically chunked)
    let args = json!({
        "path": "large_file.txt"
    });

    let result = registry.execute_tool("read_file", args).await?;

    // Check that the file was chunked
    assert_eq!(result["success"], true);
    assert_eq!(result["truncated"], true);
    assert_eq!(result["total_lines"], 15_000);
    assert_eq!(result["shown_lines"], 1_600); // 800 + 800

    let content_str = result["content"].as_str().unwrap();
    assert!(content_str.contains("... [13400 lines truncated - showing first 800 and last 800 lines] ..."));
    assert!(content_str.contains("Line 1:")); // First line present
    assert!(content_str.contains("Line 15000:")); // Last line present

    Ok(())
}

#[tokio::test]
async fn test_read_file_chunking_custom_threshold() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a medium test file (2,000 lines)
    let test_file = temp_dir.path().join("medium_file.txt");
    let mut content = String::new();

    for i in 1..=2_000 {
        content.push_str(&format!("Line {}: Content\n", i));
    }

    fs::write(&test_file, &content)?;

    // Read with custom chunk threshold
    let args = json!({
        "path": "medium_file.txt",
        "chunk_lines": 1000  // Custom threshold
    });

    let result = registry.execute_tool("read_file", args).await?;

    // Should be chunked due to custom threshold
    assert_eq!(result["success"], true);
    assert_eq!(result["truncated"], true);
    assert_eq!(result["total_lines"], 2_000);
    assert_eq!(result["shown_lines"], 1000); // 500 + 500 with custom threshold

    Ok(())
}

#[tokio::test]
async fn test_write_file_chunking_large_content() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create large content (>1MB)
    let large_content = "x".repeat(2_000_000); // 2MB of content

    let args = json!({
        "path": "large_output.txt",
        "content": large_content,
        "mode": "overwrite"
    });

    let result = registry.execute_tool("write_file", args).await?;

    // Check that chunked writing worked
    assert_eq!(result["success"], true);
    assert_eq!(result["chunked"], true);
    assert_eq!(result["bytes_written"], 2_000_000);
    assert!(result["chunks_written"].as_u64().unwrap() > 1); // Should be multiple chunks

    // Verify file was written correctly
    let written_content = fs::read_to_string(temp_dir.path().join("large_output.txt"))?;
    assert_eq!(written_content.len(), 2_000_000);

    Ok(())
}

#[tokio::test]
async fn test_edit_file_with_chunking() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a large file for editing
    let test_file = temp_dir.path().join("large_edit.txt");
    let mut content = String::new();

    for i in 1..=12_000 {
        content.push_str(&format!("Line {}: Original content\n", i));
    }

    fs::write(&test_file, &content)?;

    // Edit the file (should handle chunking internally)
    let args = json!({
        "path": "large_edit.txt",
        "old_str": "Line 100: Original content\n",
        "new_str": "Line 100: Modified content\n"
    });

    let result = registry.execute_tool("edit_file", args).await?;

    // Check that edit succeeded
    assert_eq!(result["success"], true);

    // Verify the edit was applied
    let edited_content = fs::read_to_string(&test_file)?;
    assert!(edited_content.contains("Line 100: Modified content"));
    assert!(!edited_content.contains("Line 100: Original content"));

    Ok(())
}

#[tokio::test]
async fn test_run_terminal_cmd_output_truncation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Create a large file to generate verbose output
    let large_file = temp_dir.path().join("large_data.txt");
    let mut content = String::new();
    for i in 1..=20_000 {
        content.push_str(&format!("data {}\n", i));
    }
    fs::write(&large_file, &content)?;

    // Run a command that produces a lot of output
    let args = json!({
        "command": ["cat", "large_data.txt"],
        "timeout_secs": 30
    });

    let result = registry.execute_tool("run_terminal_cmd", args).await?;

    // Check that output was truncated
    assert_eq!(result["success"], true);
    assert_eq!(result["truncated"], true);
    assert_eq!(result["total_output_lines"], 20_000);
    assert_eq!(result["shown_lines"], 2_000); // 1,000 + 1,000

    let stdout = result["stdout"].as_str().unwrap();
    assert!(stdout.contains("... [18000 lines truncated] ..."));
    assert!(stdout.contains("data 1")); // First line present
    assert!(stdout.contains("data 20000")); // Last line present

    Ok(())
}

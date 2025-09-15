use anyhow::Result;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use vtagent_core::tools::{build_function_declarations, ToolRegistry};

#[tokio::test]
async fn test_tool_availability() -> Result<()> {
    // Check that batch_file_operations is in the available tools
    let declarations = build_function_declarations();
    let batch_tool_exists = declarations
        .iter()
        .any(|d| d.name == "batch_file_operations");
    assert!(
        batch_tool_exists,
        "batch_file_operations should be available as a tool"
    );

    let extract_tool_exists = declarations
        .iter()
        .any(|d| d.name == "extract_dependencies");
    assert!(
        extract_tool_exists,
        "extract_dependencies should be available as a tool"
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
async fn test_write_file_append_and_skip() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    // Write initial content
    let args = json!({
        "path": "log.txt",
        "content": "Line1\n",
        "mode": "overwrite"
    });
    registry.write_file(args).await?;

    // Append content
    let args = json!({
        "path": "log.txt",
        "content": "Line2\n",
        "mode": "append"
    });
    registry.write_file(args).await?;

    // Try skip_if_exists
    let args = json!({
        "path": "log.txt",
        "content": "ShouldNotAppear",
        "mode": "skip_if_exists"
    });
    let result = registry.write_file(args).await?;
    assert_eq!(result["skipped"], true);

    // Verify file content
    let read_args = json!({ "path": "log.txt" });
    let read_result = registry.read_file(read_args).await?;
    let content = read_result["content"].as_str().unwrap();
    assert!(content.contains("Line1"));
    assert!(content.contains("Line2"));
    assert!(!content.contains("ShouldNotAppear"));

    Ok(())
}

#[tokio::test]
async fn test_edit_file_error_when_missing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

    let test_file = temp_dir.path().join("sample.txt");
    fs::write(&test_file, "original")?;

    let args = json!({
        "path": "sample.txt",
        "old_str": "missing",
        "new_str": "new"
    });

    let err = registry.edit_file(args).await.unwrap_err();
    assert!(err.to_string().contains("Could not find"));

    Ok(())
}

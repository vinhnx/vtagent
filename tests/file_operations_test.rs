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

    let result = registry.execute("read_file", args).await?;

    // Check that the file was read correctly
    assert_eq!(result["content"], "Hello, World!");
    assert_eq!(result["metadata"]["size"], 13);

    Ok(())
}

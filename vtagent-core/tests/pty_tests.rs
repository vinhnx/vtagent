//! Integration tests for PTY functionality

use anyhow::Result;
use serde_json::json;
use tempfile::TempDir;
use vtagent_core::tools::ToolRegistry;

#[tokio::test]
async fn test_pty_basic_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().to_path_buf();
    let registry = ToolRegistry::new(workspace.clone());

    // Test a simple PTY command
    let args = json!({
        "command": "echo",
        "args": ["Hello, PTY!"]
    });

    let result = registry.execute_tool("run_pty_cmd", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["code"], 0);
    assert!(result["output"].as_str().unwrap().contains("Hello, PTY!"));

    Ok(())
}

#[tokio::test]
async fn test_pty_command_with_working_dir() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().to_path_buf();
    let registry = ToolRegistry::new(workspace.clone());

    // Create a test file
    std::fs::write(workspace.join("test.txt"), "Hello, PTY!")?;

    // Test a PTY command that reads the file
    let args = json!({
        "command": "cat",
        "args": ["test.txt"]
    });

    let result = registry.execute_tool("run_pty_cmd", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["code"], 0);
    assert!(result["output"].as_str().unwrap().contains("Hello, PTY!"));

    Ok(())
}

#[tokio::test]
async fn test_pty_session_management() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().to_path_buf();
    let registry = ToolRegistry::new(workspace.clone());

    // Test creating a PTY session
    let args = json!({
        "session_id": "test_session",
        "command": "bash"
    });

    let result = registry.execute_tool("create_pty_session", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["session_id"], "test_session");

    // Test listing PTY sessions
    let args = json!({});
    let result = registry.execute_tool("list_pty_sessions", args).await?;
    assert!(result["sessions"].as_array().unwrap().contains(&"test_session".into()));

    // Test closing a PTY session
    let args = json!({
        "session_id": "test_session"
    });

    let result = registry.execute_tool("close_pty_session", args).await?;
    assert_eq!(result["success"], true);
    assert_eq!(result["session_id"], "test_session");

    Ok(())
}
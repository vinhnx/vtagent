//! Integration test for TodoManager with temporary files

use anyhow::Result;
use tempfile::TempDir;
use vtagent_core::todo_write::{TodoManager, TodoInput, TodoStatus};

#[tokio::test]
async fn test_todo_manager_with_temp_files() -> Result<()> {
    // Create a temporary workspace directory for testing
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path().to_path_buf();

    // Create a new TodoManager
    let manager = TodoManager::new(workspace);

    // Initialize the manager
    manager.initialize().await?;

    // Initially, there should be no temp file
    assert!(manager.get_temp_file_path().await.is_none());

    // Add some test todos
    let test_todos = vec![
        TodoInput {
            content: "Test todo item 1".to_string(),
            status: TodoStatus::Pending,
            id: None,
            notes: Some("This is a test note".to_string()),
        },
        TodoInput {
            content: "Test todo item 2".to_string(),
            status: TodoStatus::InProgress,
            id: None,
            notes: None,
        },
    ];

    let created_items = manager.write_todos(false, test_todos).await?;
    assert_eq!(created_items.len(), 2);

    // After writing todos, there should be a temp file
    let temp_path = manager.get_temp_file_path().await;
    assert!(temp_path.is_some());

    let temp_path = temp_path.unwrap();
    assert!(temp_path.exists(), "Temporary file should exist");

    // Read the content and verify it's valid JSON
    let content = std::fs::read_to_string(&temp_path)?;
    let _json_value: serde_json::Value = serde_json::from_str(&content)?;

    // Verify we can retrieve the todos
    let todos = manager.get_todos().await;
    assert_eq!(todos.len(), 2);

    // Verify statistics
    let stats = manager.get_statistics().await;
    assert_eq!(stats.total_count, 2);
    assert_eq!(stats.pending_count, 1);
    assert_eq!(stats.in_progress_count, 1);
    assert_eq!(stats.completed_count, 0);

    println!("✅ All tests passed!");
    println!("Temporary file was created at: {}", temp_path.display());

    Ok(())
}

#[tokio::test]
async fn test_cleanup_is_no_op() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let manager = TodoManager::new(temp_dir.path().to_path_buf());

    // Cleanup should return 0 since we're using temp files
    let cleaned = manager.cleanup_old_sessions().await?;
    assert_eq!(cleaned, 0);

    Ok(())
}

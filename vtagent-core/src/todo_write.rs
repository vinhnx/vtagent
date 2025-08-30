//! TodoWrite tool for managing structured task lists during coding sessions
//!
//! This tool helps organize complex multi-step tasks, track progress, and provide
//! visibility into coding session workflow management.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// Task not yet started
    Pending,
    /// Currently working on this task
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task cancelled or no longer needed
    Cancelled,
}

/// Individual todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique identifier for the todo item
    pub id: String,
    /// Task description/content
    pub content: String,
    /// Current status of the task
    pub status: TodoStatus,
    /// Timestamp when task was created
    pub created_at: u64,
    /// Timestamp when task status was last updated
    pub updated_at: u64,
    /// Optional notes or additional context
    pub notes: Option<String>,
}

/// Todo management system
pub struct TodoManager {
    /// Collection of todo items
    todos: Arc<RwLock<HashMap<String, TodoItem>>>,
    /// Storage path for persistence
    storage_path: PathBuf,
    /// Session identifier
    session_id: String,
}

impl TodoManager {
    /// Create a new TodoManager instance
    pub fn new(workspace: PathBuf) -> Self {
        let session_id = format!(
            "session_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        Self {
            todos: Arc::new(RwLock::new(HashMap::new())),
            storage_path: workspace.join(".vtagent").join("todos"),
            session_id,
        }
    }

    /// Initialize the todo manager (create directories, load existing todos)
    pub async fn initialize(&self) -> Result<()> {
        // Create storage directory if it doesn't exist
        tokio::fs::create_dir_all(&self.storage_path).await
            .map_err(|e| anyhow!("Failed to create todo storage directory: {}", e))?;

        // Load existing todos if they exist
        self.load_todos().await?;

        Ok(())
    }

    /// Write todo list to storage
    pub async fn write_todos(&self, merge: bool, todos: Vec<TodoInput>) -> Result<Vec<TodoItem>> {
        let mut current_todos = self.todos.write().await;

        if !merge {
            current_todos.clear();
        }

        let mut created_items = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for todo_input in todos {
            let id = todo_input.id.unwrap_or_else(|| {
                format!("todo_{}_{}", self.session_id, now + created_items.len() as u64)
            });

            let item = TodoItem {
                id: id.clone(),
                content: todo_input.content,
                status: todo_input.status,
                created_at: now,
                updated_at: now,
                notes: todo_input.notes,
            };

            created_items.push(item.clone());
            current_todos.insert(id, item);
        }

        // Save to disk
        self.save_todos(&*current_todos).await?;

        Ok(created_items)
    }

    /// Update existing todo items
    pub async fn update_todos(&self, updates: Vec<TodoUpdate>) -> Result<Vec<TodoItem>> {
        let mut current_todos = self.todos.write().await;
        let mut updated_items = Vec::new();

        for update in updates {
            if let Some(item) = current_todos.get_mut(&update.id) {
                if let Some(content) = update.content {
                    item.content = content;
                }
                if let Some(status) = update.status {
                    item.status = status;
                }
                if let Some(notes) = update.notes {
                    item.notes = Some(notes);
                }
                item.updated_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                updated_items.push(item.clone());
            } else {
                return Err(anyhow!("Todo item with id '{}' not found", update.id));
            }
        }

        // Save to disk
        self.save_todos(&*current_todos).await?;

        Ok(updated_items)
    }

    /// Get all todo items
    pub async fn get_todos(&self) -> Vec<TodoItem> {
        let todos = self.todos.read().await;
        todos.values().cloned().collect()
    }

    /// Get todo items by status
    pub async fn get_todos_by_status(&self, status: TodoStatus) -> Vec<TodoItem> {
        let todos = self.todos.read().await;
        todos.values()
            .filter(|item| item.status == status)
            .cloned()
            .collect()
    }

    /// Delete todo items
    pub async fn delete_todos(&self, ids: Vec<String>) -> Result<Vec<String>> {
        let mut current_todos = self.todos.write().await;
        let mut deleted_ids = Vec::new();

        for id in ids {
            if current_todos.remove(&id).is_some() {
                deleted_ids.push(id);
            }
        }

        // Save to disk
        self.save_todos(&*current_todos).await?;

        Ok(deleted_ids)
    }

    /// Get todo statistics
    pub async fn get_statistics(&self) -> TodoStatistics {
        let todos = self.todos.read().await;
        let mut stats = TodoStatistics::default();

        for item in todos.values() {
            match item.status {
                TodoStatus::Pending => stats.pending_count += 1,
                TodoStatus::InProgress => stats.in_progress_count += 1,
                TodoStatus::Completed => stats.completed_count += 1,
                TodoStatus::Cancelled => stats.cancelled_count += 1,
            }
        }

        stats.total_count = todos.len();
        stats.completion_rate = if stats.total_count > 0 {
            (stats.completed_count as f64 / stats.total_count as f64) * 100.0
        } else {
            0.0
        };

        stats
    }

    /// Load todos from disk
    async fn load_todos(&self) -> Result<()> {
        let storage_file = self.storage_path.join(format!("{}.json", self.session_id));

        if storage_file.exists() {
            let content = tokio::fs::read_to_string(&storage_file).await
                .map_err(|e| anyhow!("Failed to read todo storage: {}", e))?;

            let todos: HashMap<String, TodoItem> = serde_json::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse todo data: {}", e))?;

            let mut current_todos = self.todos.write().await;
            *current_todos = todos;
        }

        Ok(())
    }

    /// Save todos to disk
    async fn save_todos(&self, todos: &HashMap<String, TodoItem>) -> Result<()> {
        let storage_file = self.storage_path.join(format!("{}.json", self.session_id));

        let content = serde_json::to_string_pretty(todos)
            .map_err(|e| anyhow!("Failed to serialize todos: {}", e))?;

        tokio::fs::write(&storage_file, content).await
            .map_err(|e| anyhow!("Failed to write todo storage: {}", e))?;

        Ok(())
    }

    /// Clean up old todo files (older than 7 days)
    pub async fn cleanup_old_sessions(&self) -> Result<usize> {
        let mut cleaned_count = 0;
        let entries = tokio::fs::read_dir(&self.storage_path).await
            .map_err(|e| anyhow!("Failed to read storage directory: {}", e))?;

        let mut entries_vec = Vec::new();
        tokio::pin!(entries);
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| anyhow!("Failed to read directory entry: {}", e))? {
            entries_vec.push(entry);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for entry in entries_vec {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = entry.metadata().await {
                    let modified = metadata.modified()
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    // Delete files older than 7 days (604800 seconds)
                    if now - modified > 604800 {
                        tokio::fs::remove_file(&path).await
                            .map_err(|e| anyhow!("Failed to remove old todo file: {}", e))?;
                        cleaned_count += 1;
                    }
                }
            }
        }

        Ok(cleaned_count)
    }
}

/// Input structure for creating new todos
#[derive(Debug, Clone, Deserialize)]
pub struct TodoInput {
    /// Task description/content (required)
    pub content: String,
    /// Initial status (defaults to Pending)
    #[serde(default = "default_pending_status")]
    pub status: TodoStatus,
    /// Optional custom ID (auto-generated if not provided)
    pub id: Option<String>,
    /// Optional notes
    pub notes: Option<String>,
}

/// Input structure for updating existing todos
#[derive(Debug, Clone, Deserialize)]
pub struct TodoUpdate {
    /// ID of the todo to update (required)
    pub id: String,
    /// New content (optional)
    pub content: Option<String>,
    /// New status (optional)
    pub status: Option<TodoStatus>,
    /// New notes (optional)
    pub notes: Option<String>,
}

/// Todo statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoStatistics {
    pub total_count: usize,
    pub pending_count: usize,
    pub in_progress_count: usize,
    pub completed_count: usize,
    pub cancelled_count: usize,
    pub completion_rate: f64,
}

impl Default for TodoStatistics {
    fn default() -> Self {
        Self {
            total_count: 0,
            pending_count: 0,
            in_progress_count: 0,
            completed_count: 0,
            cancelled_count: 0,
            completion_rate: 0.0,
        }
    }
}

fn default_pending_status() -> TodoStatus {
    TodoStatus::Pending
}

/// Tool execution functions for TodoWrite tool
pub mod tool_functions {
    use super::*;
    use serde_json::Value;

    /// Input for write_todos tool function
    #[derive(Debug, Deserialize)]
    pub struct WriteTodosInput {
        /// Whether to merge with existing todos (true) or replace (false)
        pub merge: bool,
        /// List of todos to create/update
        pub todos: Vec<TodoInput>,
    }

    /// Input for update_todos tool function
    #[derive(Debug, Deserialize)]
    pub struct UpdateTodosInput {
        /// List of todo updates
        pub todos: Vec<TodoUpdate>,
    }

    /// Input for delete_todos tool function
    #[derive(Debug, Deserialize)]
    pub struct DeleteTodosInput {
        /// List of todo IDs to delete
        pub ids: Vec<String>,
    }

    /// Execute write_todos tool function
    pub async fn write_todos(manager: &TodoManager, args: Value) -> Result<Value> {
        let input: WriteTodosInput = serde_json::from_value(args)
            .map_err(|e| anyhow!("Invalid write_todos arguments: {}", e))?;

        let created_items = manager.write_todos(input.merge, input.todos).await?;

        Ok(serde_json::json!({
            "success": true,
            "created_count": created_items.len(),
            "todos": created_items
        }))
    }

    /// Execute update_todos tool function
    pub async fn update_todos(manager: &TodoManager, args: Value) -> Result<Value> {
        let input: UpdateTodosInput = serde_json::from_value(args)
            .map_err(|e| anyhow!("Invalid update_todos arguments: {}", e))?;

        let updated_items = manager.update_todos(input.todos).await?;

        Ok(serde_json::json!({
            "success": true,
            "updated_count": updated_items.len(),
            "todos": updated_items
        }))
    }

    /// Execute get_todos tool function
    pub async fn get_todos(manager: &TodoManager, _args: Value) -> Result<Value> {
        let todos = manager.get_todos().await;
        let stats = manager.get_statistics().await;

        Ok(serde_json::json!({
            "todos": todos,
            "statistics": stats
        }))
    }

    /// Execute get_todos_by_status tool function
    pub async fn get_todos_by_status(manager: &TodoManager, args: Value) -> Result<Value> {
        let status_str = args.get("status")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'status' parameter"))?;

        let status = match status_str {
            "pending" => TodoStatus::Pending,
            "in_progress" => TodoStatus::InProgress,
            "completed" => TodoStatus::Completed,
            "cancelled" => TodoStatus::Cancelled,
            _ => return Err(anyhow!("Invalid status: {}", status_str)),
        };

        let todos = manager.get_todos_by_status(status).await;

        Ok(serde_json::json!({
            "status": status_str,
            "count": todos.len(),
            "todos": todos
        }))
    }

    /// Execute delete_todos tool function
    pub async fn delete_todos(manager: &TodoManager, args: Value) -> Result<Value> {
        let input: DeleteTodosInput = serde_json::from_value(args)
            .map_err(|e| anyhow!("Invalid delete_todos arguments: {}", e))?;

        let deleted_ids = manager.delete_todos(input.ids).await?;

        Ok(serde_json::json!({
            "success": true,
            "deleted_count": deleted_ids.len(),
            "deleted_ids": deleted_ids
        }))
    }

    /// Execute get_statistics tool function
    pub async fn get_statistics(manager: &TodoManager, _args: Value) -> Result<Value> {
        let stats = manager.get_statistics().await;

        Ok(serde_json::json!({
            "statistics": stats
        }))
    }

    /// Execute cleanup tool function
    pub async fn cleanup(manager: &TodoManager, _args: Value) -> Result<Value> {
        let cleaned_count = manager.cleanup_old_sessions().await?;

        Ok(serde_json::json!({
            "success": true,
            "cleaned_sessions": cleaned_count
        }))
    }
}

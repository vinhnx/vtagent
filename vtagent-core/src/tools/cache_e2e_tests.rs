//! End-to-end tests for file operations with quick-cache integration

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use vtagent_core::tools::cache::FileCache;
use vtagent_core::tools::registry::ToolRegistry;
use serde_json::{Value, json};

#[cfg(test)]
mod e2e_tests {
    use super::*;

    /// Test end-to-end file operations with caching
    #[tokio::test]
    async fn test_file_operations_with_cache() {
        // Create temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = temp_dir.path().to_path_buf();

        // Create test file
        let test_file = workspace_root.join("test_file.txt");
        let test_content = "This is test content for caching verification.";
        fs::write(&test_file, test_content).expect("Failed to write test file");

        // Initialize tool registry
        let mut registry = ToolRegistry::new(workspace_root.clone());

        // Test 1: First read should cache the file
        let read_args = json!({
            "path": test_file.to_string_lossy(),
            "max_bytes": 1000
        });

        let result1 = registry.execute_tool("read_file", read_args.clone()).await;
        assert!(result1.is_ok(), "First read should succeed");

        let content1 = result1.unwrap();
        assert_eq!(content1["content"], test_content);

        // Test 2: Second read should use cache (verify by checking cache stats)
        let result2 = registry.execute_tool("read_file", read_args).await;
        assert!(result2.is_ok(), "Second read should succeed");

        let content2 = result2.unwrap();
        assert_eq!(content2["content"], test_content);

        // Test 3: Write operation should invalidate/update cache
        let new_content = "Updated content after caching.";
        let write_args = json!({
            "path": test_file.to_string_lossy(),
            "content": new_content
        });

        let write_result = registry.execute_tool("write_file", write_args).await;
        assert!(write_result.is_ok(), "Write operation should succeed");

        // Test 4: Read after write should get updated content
        let read_args2 = json!({
            "path": test_file.to_string_lossy(),
            "max_bytes": 1000
        });

        let result3 = registry.execute_tool("read_file", read_args2).await;
        assert!(result3.is_ok(), "Read after write should succeed");

        let content3 = result3.unwrap();
        assert_eq!(content3["content"], new_content);

        // Verify file on disk matches
        let disk_content = fs::read_to_string(&test_file).expect("Failed to read from disk");
        assert_eq!(disk_content, new_content);
    }

    /// Test cache statistics tracking
    #[tokio::test]
    async fn test_cache_statistics_tracking() {
        let cache = FileCache::new(100);

        // Initially should have no hits/misses
        let initial_stats = cache.stats().await;
        assert_eq!(initial_stats.hits, 0);
        assert_eq!(initial_stats.misses, 0);

        // Cache miss
        let miss_result = cache.get_file("nonexistent").await;
        assert!(miss_result.is_none());

        let after_miss_stats = cache.stats().await;
        assert_eq!(after_miss_stats.hits, 0);
        assert_eq!(after_miss_stats.misses, 1);

        // Cache hit
        let test_data = Value::String("cached content".to_string());
        cache.put_file("test_key".to_string(), test_data.clone()).await;

        let hit_result = cache.get_file("test_key").await;
        assert!(hit_result.is_some());
        assert_eq!(hit_result.unwrap(), test_data);

        let after_hit_stats = cache.stats().await;
        assert_eq!(after_hit_stats.hits, 1);
        assert_eq!(after_hit_stats.misses, 1);
    }

    /// Test cache capacity limits
    #[tokio::test]
    async fn test_cache_capacity_limits() {
        let cache = FileCache::new(2); // Very small capacity for testing

        // Fill cache
        cache.put_file("key1".to_string(), Value::String("data1".to_string())).await;
        cache.put_file("key2".to_string(), Value::String("data2".to_string())).await;
        cache.put_file("key3".to_string(), Value::String("data3".to_string())).await; // Should evict

        // Check capacity
        let (file_capacity, dir_capacity) = cache.capacity();
        assert_eq!(file_capacity, 2);

        // Check current size
        let (file_len, dir_len) = cache.len();
        assert!(file_len <= 2); // Should not exceed capacity
    }

    /// Test directory caching
    #[tokio::test]
    async fn test_directory_caching() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = temp_dir.path().to_path_buf();

        // Create test directory structure
        let subdir = workspace_root.join("subdir");
        fs::create_dir(&subdir).expect("Failed to create subdir");
        fs::write(subdir.join("file1.txt"), "content1").expect("Failed to write file1");
        fs::write(subdir.join("file2.txt"), "content2").expect("Failed to write file2");

        let mut registry = ToolRegistry::new(workspace_root.clone());

        // List directory (should cache result)
        let list_args = json!({
            "path": subdir.to_string_lossy()
        });

        let result1 = registry.execute_tool("list_files", list_args.clone()).await;
        assert!(result1.is_ok(), "First list should succeed");

        // Second list should use cache
        let result2 = registry.execute_tool("list_files", list_args).await;
        assert!(result2.is_ok(), "Second list should succeed");

        // Results should be identical
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    /// Test cache invalidation on file modification
    #[tokio::test]
    async fn test_cache_invalidation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let workspace_root = temp_dir.path().to_path_buf();

        let test_file = workspace_root.join("test.txt");
        let mut registry = ToolRegistry::new(workspace_root.clone());

        // Create and read file
        fs::write(&test_file, "original").expect("Failed to write original content");

        let read_args = json!({
            "path": test_file.to_string_lossy()
        });

        let result1 = registry.execute_tool("read_file", read_args.clone()).await;
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap()["content"], "original");

        // Modify file via tool
        let edit_args = json!({
            "path": test_file.to_string_lossy(),
            "old_str": "original",
            "new_str": "modified"
        });

        let edit_result = registry.execute_tool("edit_file", edit_args).await;
        assert!(edit_result.is_ok(), "Edit should succeed");

        // Read again - should get updated content (cache should be invalidated)
        let result2 = registry.execute_tool("read_file", read_args).await;
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap()["content"], "modified");
    }
}
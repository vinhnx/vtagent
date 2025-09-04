use serde_json::json;
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;
use vtagent_core::tools::ToolRegistry;

#[cfg(test)]
mod integration_tests {

    use super::*;

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test that registry is created successfully
        // Test that registry was created successfully
    }

    #[tokio::test]
    async fn test_list_files_tool() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create some test files
        std::fs::write(temp_dir.path().join("test1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("test2.txt"), "content2").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test listing files
        let args = json!({
            "path": "."
        });

        let result = registry.execute("list_files", args).await;
        assert!(result.is_ok());

        let response: serde_json::Value = result.unwrap();
        assert!(response["files"].is_array());
        assert!(response["files"].as_array().unwrap().len() >= 2); // test1.txt, test2.txt, subdir
    }

    #[tokio::test]
    async fn test_read_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let test_content = "This is test content";
        std::fs::write(temp_dir.path().join("read_test.txt"), test_content).unwrap();

        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        let args = json!({
            "path": "read_test.txt"
        });

        let result = registry.execute("read_file", args).await;
        assert!(result.is_ok());

        let response: serde_json::Value = result.unwrap();
        assert_eq!(response["content"], test_content);
    }

    #[tokio::test]
    async fn test_write_file_tool() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        let args = json!({
            "path": "write_test.txt",
            "content": "Hello, World!",
            "overwrite": false,
            "create_dirs": false
        });

        let result = registry.execute("write_file", args).await;
        assert!(result.is_ok());

        let _response: serde_json::Value = result.unwrap();
        // The response should be successful if no error occurred
        assert!(true); // If we reach here, the operation was successful

        // Verify file was created
        let file_path = temp_dir.path().join("write_test.txt");
        assert!(file_path.exists());
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_grep_search_tool() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let rust_content = r#"fn main() {
    println!("Hello, world!");
    let x = 42;
}

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}"#;
        std::fs::write(temp_dir.path().join("search_test.rs"), rust_content).unwrap();

        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        let args = json!({
            "pattern": "fn main",
            "path": ".",
            "type": "regex"
        });

        let result = registry.execute("rp_search", args).await;
        assert!(result.is_ok());

        let response: serde_json::Value = result.unwrap();
        println!("Response: {:#?}", response);
        assert!(response["matches"].is_array());
        let matches = response["matches"].as_array().unwrap();
        assert!(!matches.is_empty());
    }
}

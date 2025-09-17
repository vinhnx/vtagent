//! Tests for Speckit tool integration

use serde_json::json;
use vtagent_core::tools::registry::ToolRegistry;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_speckit_tool_registration() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Check if Speckit tool is registered
        let available_tools = registry.available_tools();
        println!("Available tools: {:?}", available_tools);

        // Speckit should always be registered
        assert!(available_tools.contains(&"speckit".to_string()));
    }

    #[tokio::test]
    async fn test_speckit_tool_execution_init() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test Speckit init command
        let args = json!({
            "command": "init",
            "args": ["test-project"]
        });

        let result = registry.execute_tool("speckit", args).await;
        match result {
            Ok(response) => {
                println!("Speckit init execution result: {:?}", response);
                // The result should contain command information
                assert!(response["command"].as_str().is_some());
                assert!(response["working_directory"].as_str().is_some());
            }
            Err(e) => {
                println!("Speckit init execution failed: {}", e);
                // This might fail if uvx or Speckit is not available, which is expected
                assert!(e.to_string().contains("uvx") || e.to_string().contains("specify"));
            }
        }
    }

    #[tokio::test]
    async fn test_speckit_tool_execution_specify() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test Speckit /specify command
        let args = json!({
            "command": "/specify",
            "args": ["Build a simple todo application"]
        });

        let result = registry.execute_tool("speckit", args).await;
        match result {
            Ok(response) => {
                println!("Speckit /specify execution result: {:?}", response);
                // This should now return an error for unsupported command
                if let Some(error) = response.get("error") {
                    println!("Got expected error for unsupported command: {:?}", error);
                    assert!(
                        error["message"]
                            .as_str()
                            .unwrap()
                            .contains("Unsupported Speckit command")
                    );
                } else {
                    panic!("Expected error response for unsupported command");
                }
            }
            Err(e) => {
                println!("Speckit /specify execution failed: {}", e);
                // This might fail if uvx or Speckit is not available, which is expected
                assert!(e.to_string().contains("uvx") || e.to_string().contains("specify"));
            }
        }
    }

    #[tokio::test]
    async fn test_speckit_tool_execution_plan() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test Speckit /plan command
        let args = json!({
            "command": "/plan",
            "args": ["Use React with TypeScript"]
        });

        let result = registry.execute_tool("speckit", args).await;
        match result {
            Ok(response) => {
                println!("Speckit /plan execution result: {:?}", response);
                // This should now return an error for unsupported command
                if let Some(error) = response.get("error") {
                    println!("Got expected error for unsupported command: {:?}", error);
                    assert!(
                        error["message"]
                            .as_str()
                            .unwrap()
                            .contains("Unsupported Speckit command")
                    );
                } else {
                    panic!("Expected error response for unsupported command");
                }
            }
            Err(e) => {
                println!("Speckit /plan execution failed: {}", e);
                // This might fail if uvx or Speckit is not available, which is expected
                assert!(e.to_string().contains("uvx") || e.to_string().contains("specify"));
            }
        }
    }

    #[tokio::test]
    async fn test_speckit_tool_execution_tasks() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test Speckit /tasks command
        let args = json!({
            "command": "/tasks"
        });

        let result = registry.execute_tool("speckit", args).await;
        match result {
            Ok(response) => {
                println!("Speckit /tasks execution result: {:?}", response);
                // This should now return an error for unsupported command
                if let Some(error) = response.get("error") {
                    println!("Got expected error for unsupported command: {:?}", error);
                    assert!(
                        error["message"]
                            .as_str()
                            .unwrap()
                            .contains("Unsupported Speckit command")
                    );
                } else {
                    panic!("Expected error response for unsupported command");
                }
            }
            Err(e) => {
                println!("Speckit /tasks execution failed: {}", e);
                // This might fail if uvx or Speckit is not available, which is expected
                assert!(e.to_string().contains("uvx") || e.to_string().contains("specify"));
            }
        }
    }

    #[tokio::test]
    async fn test_speckit_tool_execution_check() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test Speckit check command
        let args = json!({
            "command": "check"
        });

        let result = registry.execute_tool("speckit", args).await;
        match result {
            Ok(response) => {
                println!("Speckit check execution result: {:?}", response);
                assert!(response["command"].as_str().is_some());
                assert!(response["speckit_command"].as_str().unwrap() == "check");
            }
            Err(e) => {
                println!("Speckit check execution failed: {}", e);
                // This might fail if uvx or Speckit is not available, which is expected
                assert!(e.to_string().contains("uvx") || e.to_string().contains("specify"));
            }
        }
    }

    #[tokio::test]
    async fn test_speckit_tool_invalid_command() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test invalid Speckit command
        let args = json!({
            "command": "invalid_command"
        });

        let result = registry.execute_tool("speckit", args).await;
        // This should fail with validation error
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response["error"].is_object());
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Unsupported Speckit command")
        );
    }

    #[tokio::test]
    async fn test_speckit_tool_missing_command() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let mut registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Test missing command parameter
        let args = json!({
            "args": ["some_arg"]
        });

        let result = registry.execute_tool("speckit", args).await;
        // This should fail with validation error
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response["error"].is_object());
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Missing required 'command' parameter")
        );
    }

    #[test]
    fn test_speckit_function_declaration() {
        use vtagent_core::tools::build_function_declarations;

        let declarations = build_function_declarations();
        let speckit_decl = declarations.iter().find(|d| d.name == "speckit");

        assert!(
            speckit_decl.is_some(),
            "Speckit function declaration should be present"
        );

        if let Some(decl) = speckit_decl {
            println!("Speckit function declaration found: {}", decl.name);
            assert_eq!(decl.name, "speckit");
            assert!(decl.description.contains("Speckit"));
            assert!(decl.description.contains("spec-driven development"));

            // Check parameters
            let params = &decl.parameters;
            assert!(params["type"].as_str().unwrap() == "object");
            assert!(
                params["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("command"))
            );
        }
    }

    #[tokio::test]
    async fn test_speckit_tool_has_tool_check() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let registry = ToolRegistry::new(temp_dir.path().to_path_buf());

        // Speckit should always be available
        assert!(registry.has_tool("speckit"));
        assert!(!registry.has_tool("nonexistent_tool"));
    }
}

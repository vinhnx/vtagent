//! Test module for async file operations

use std::path::PathBuf;
use tokio;
use vtagent::async_file_ops;
use vtagent::diff_renderer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Async File Operations");

    // Test async file writer
    let writer = async_file_ops::AsyncFileWriter::new(5);

    // Create a test file
    let test_path = PathBuf::from("test_async_output.rs");
    let test_content = r#"// Test file created by async operations
fn main() {
    println!("Hello from async file operations!");
}
"#;

    println!("📝 Writing file asynchronously...");
    writer
        .write_file(test_path.clone(), test_content.to_string())
        .await?;
    println!("✅ File queued for async write");

    // Wait a bit for the operation to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test diff renderer
    println!("\n🔍 Testing Diff Renderer");

    let old_content = "fn main() {\n    println!(\"Hello!\");\n}";
    let new_content = "fn main() {\n    println!(\"Hello from async world!\");\n}";

    let diff_renderer = diff_renderer::DiffChatRenderer::new(true, 3, true);
    let diff_output = diff_renderer.render_file_change(&test_path, old_content, new_content);

    println!("📋 Generated Diff:");
    println!("{}", diff_output);

    println!("\n🎉 Async file operations and diff rendering test completed!");

    Ok(())
}

use anyhow::Result;
use vtagent_core::core::prompt_optimizer::{PromptRefiner};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Create a prompt refiner with standard level
    let refiner = PromptRefiner::new("standard");
    
    // Test files
    let files = vec![
        "src/main.rs".to_string(),
        "vtagent-core/src/lib.rs".to_string(),
        "vtagent-core/src/core/prompt_optimizer.rs".to_string(),
        "Cargo.toml".to_string(),
    ];
    
    // Test prompt
    let prompt = "Add a new function to calculate fibonacci numbers";
    
    // Optimize the prompt
    let optimized = refiner.optimize(prompt, &files, "single").await?;
    
    println!("Original prompt: {}", prompt);
    println!("Optimized prompt:\n{}", optimized);
    
    Ok(())
}
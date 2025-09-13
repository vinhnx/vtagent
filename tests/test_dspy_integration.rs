use anyhow::Result;
use vtagent_core::core::prompt_optimizer::PromptRefiner;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing DSPy integration...");

    // Create a prompt refiner with DSPy backend
    let refiner = PromptRefiner::new("standard");

    // Test files
    let files = vec![
        "src/main.rs".to_string(),
        "vtagent-core/src/lib.rs".to_string(),
        "vtagent-core/src/core/prompt_optimizer.rs".to_string(),
        "Cargo.toml".to_string(),
    ];

    // Test prompt
    let prompt = "Fix the bug in chat.rs where messages are not being sent properly";

    // Optimize the prompt
    let optimized = refiner.optimize(prompt, &files, "single").await?;

    println!("Original prompt: {}", prompt);
    println!("\nOptimized prompt:\n{}\n", optimized);

    // Check if the optimized prompt contains expected DSPy-style structure
    let has_project_context = optimized.contains("[Project Context]");
    let has_task_section = optimized.contains("[Task]");
    let has_user_prompt = optimized.contains("[User Prompt]");
    let has_plan = optimized.contains("[Plan]");
    let has_guidelines = optimized.contains("[Guidelines]");
    let has_deliverables = optimized.contains("[Deliverables]");

    println!("DSPy Integration Check:");
    println!("- Has [Project Context]: {}", has_project_context);
    println!("- Has [Task]: {}", has_task_section);
    println!("- Has [User Prompt]: {}", has_user_prompt);
    println!("- Has [Plan]: {}", has_plan);
    println!("- Has [Guidelines]: {}", has_guidelines);
    println!("- Has [Deliverables]: {}", has_deliverables);

    if has_project_context
        && has_task_section
        && has_user_prompt
        && has_plan
        && has_guidelines
        && has_deliverables
    {
        println!("\n✅ DSPy integration is working correctly!");
    } else {
        println!("\n❌ DSPy integration may not be working as expected.");
    }

    Ok(())
}

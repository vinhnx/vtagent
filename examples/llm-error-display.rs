use vtagent_core::llm::error_display::*;

fn main() {
    println!("LLM Error Display Examples:");
    println!("==========================");
    
    // Show error styling
    let error_msg = style_llm_error("Connection failed");
    println!("Error: {}", error_msg);
    
    // Show warning styling
    let warning_msg = style_llm_warning("Rate limit approaching");
    println!("Warning: {}", warning_msg);
    
    // Show success styling
    let success_msg = style_llm_success("Request completed successfully");
    println!("Success: {}", success_msg);
    
    // Show provider-specific styling
    let gemini_error = format_llm_error("gemini", "API key invalid");
    println!("Provider Error: {}", gemini_error);
    
    let openai_warning = format_llm_warning("openai", "Rate limit reached");
    println!("Provider Warning: {}", openai_warning);
    
    let anthropic_success = format_llm_success("anthropic", "Model loaded");
    println!("Provider Success: {}", anthropic_success);
    
    println!("\nAll examples displayed successfully!");
}
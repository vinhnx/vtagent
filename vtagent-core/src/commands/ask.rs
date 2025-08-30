//! Ask command implementation - single prompt without tools

use crate::gemini::{Client, Content, GenerateContentRequest};
use crate::prompts::generate_lightweight_instruction;
use crate::types::AgentConfig;
use anyhow::Result;

/// Handle the ask command - single prompt without tools
pub async fn handle_ask_command(config: AgentConfig, prompt: Vec<String>) -> Result<()> {
    let client = Client::new(config.api_key.clone(), config.model.clone());
    let prompt_text = prompt.join(" ");

    if config.verbose {
        println!("Sending prompt to {}: {}", config.model, prompt_text);
    }

    let contents = vec![Content::user_text(prompt_text)];
    let system_instruction = generate_lightweight_instruction();

    let request = GenerateContentRequest {
        contents,
        tools: None,
        tool_config: None,
        generation_config: None,
        system_instruction: Some(system_instruction),
    };

    let response = client.generate_content(&request).await?;

    if let Some(candidate) = response.candidates.into_iter().next() {
        for part in candidate.content.parts {
            if let Some(text) = part.as_text() {
                println!("{}", text);
            }
        }
    }

    Ok(())
}

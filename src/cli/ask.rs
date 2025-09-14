use anyhow::Result;
use console::style;
use vtagent_core::{llm::make_client, models::ModelId, config::types::AgentConfig as CoreAgentConfig};

/// Handle the ask command - single prompt, no tools
pub async fn handle_ask_command(config: &CoreAgentConfig, prompt: &str) -> Result<()> {
    if prompt.trim().is_empty() {
        anyhow::bail!("No prompt provided. Use: vtagent ask \"Your question here\"");
    }

    println!("{}", style("Single Prompt Mode").blue().bold());
    println!("Model: {}", &config.model);
    println!();

    let model_id: ModelId = config.model.parse()?;

    let mut client = make_client(config.api_key.clone(), model_id);
    let resp = client.generate(prompt).await?;
    println!("{}", resp.content);

    Ok(())
}

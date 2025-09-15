use anyhow::Result;
use console::style;
use std::io::{self, Write};
use vtagent_core::{
    config::types::AgentConfig as CoreAgentConfig, llm::make_client, models::ModelId,
};

/// Minimal interactive chat loop (no tools) compatible with unified LLM client
pub async fn handle_chat_command(
    config: &CoreAgentConfig,
    _skip_confirmations: bool,
) -> Result<()> {
    println!("{}", style("Interactive chat (minimal)").blue().bold());
    println!("Model: {}", config.model);
    println!("Workspace: {}", config.workspace.display());
    println!("Type 'exit' to quit\n");

    let model_id: ModelId = config.model.parse()?;
    let mut client = make_client(config.api_key.clone(), model_id);

    loop {
        print!("> ");
        io::stdout().flush().ok();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let msg = input.trim();
        if msg.is_empty() {
            continue;
        }
        if matches!(msg, "exit" | "quit") {
            break;
        }

        let resp = client.generate(msg).await?;
        println!("{}", resp.content);
    }

    Ok(())
}

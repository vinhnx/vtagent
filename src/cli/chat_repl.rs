use anyhow::{Context, Result};
use std::io::{self, Write};
use vtagent_core::{
    config::types::AgentConfig as CoreAgentConfig,
    llm::make_client,
    models::ModelId,
    utils::ansi::{AnsiRenderer, MessageStyle},
};

/// Minimal interactive chat loop (no tools) compatible with unified LLM client
pub async fn handle_chat_command(
    config: &CoreAgentConfig,
    _skip_confirmations: bool,
) -> Result<()> {
    let mut renderer = AnsiRenderer::stdout();
    renderer.line(MessageStyle::Info, "Interactive chat (minimal)")?;
    renderer.line(MessageStyle::Output, &format!("Model: {}", config.model))?;
    renderer.line(
        MessageStyle::Output,
        &format!("Workspace: {}", config.workspace.display()),
    )?;
    renderer.line(MessageStyle::Info, "Type 'exit' to quit\n")?;

    let model_id: ModelId = config.model.parse()?;
    let mut client = make_client(config.api_key.clone(), model_id);

    loop {
        print!("> ");
        io::stdout().flush().ok();
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            renderer.line(MessageStyle::Error, "Failed to read input")?;
            break;
        }
        let msg = input.trim();
        if msg.is_empty() {
            continue;
        }
        if matches!(msg, "exit" | "quit") {
            break;
        }

        let resp = client
            .generate(msg)
            .await
            .context("LLM generation failed")?;
        renderer.line(MessageStyle::Output, &resp.content)?;
    }

    Ok(())
}

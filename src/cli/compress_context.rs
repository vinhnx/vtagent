use anyhow::Result;
use vtcode_core::config::types::AgentConfig as CoreAgentConfig;

pub async fn handle_compress_context_command(config: &CoreAgentConfig) -> Result<()> {
    // Delegate to core demo implementation
    vtcode_core::commands::compress_context::handle_compress_context_command(
        config.clone(),
        None,
        None,
    )
    .await
}

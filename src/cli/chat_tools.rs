use anyhow::Result;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;

pub async fn handle_chat_command(
    config: &CoreAgentConfig,
    _skip_confirmations: bool,
) -> Result<()> {
    crate::agent::runloop::run_single_agent_loop(config).await
}

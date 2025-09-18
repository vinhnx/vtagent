use anyhow::Result;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;

pub async fn handle_chat_command(
    config: &CoreAgentConfig,
    skip_confirmations: bool,
    full_auto: bool,
) -> Result<()> {
    crate::agent::runloop::run_single_agent_loop(config, skip_confirmations, full_auto).await
}

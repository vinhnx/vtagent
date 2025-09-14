use anyhow::{Context, Result};
use vtagent_core::{config::loader::ConfigManager, config::types::AgentConfig as CoreAgentConfig};

pub async fn handle_chat_command(
    config: &CoreAgentConfig,
    force_multi_agent: bool,
    single_agent: bool,
    _skip_confirmations: bool,
) -> Result<()> {
    let cfg_manager = ConfigManager::load_from_workspace(&config.workspace)
        .context("Failed to load configuration")?;
    let vt_cfg = cfg_manager.config();

    if !single_agent && (force_multi_agent || vt_cfg.multi_agent.enabled) {
        crate::agent::runloop::run_multi_agent_loop(config, vt_cfg).await
    } else {
        crate::agent::runloop::run_single_agent_loop(config).await
    }
}

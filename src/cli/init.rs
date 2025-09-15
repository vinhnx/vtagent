use crate::cli::handle_chat_command;
use anyhow::{Context, Result};
use console::style;
use std::path::Path;
use vtagent_core::config::loader::VTAgentConfig;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;

/// Handle the init command
pub async fn handle_init_command(workspace: &Path, force: bool, run: bool) -> Result<()> {
    println!(
        "{}",
        style("Initialize VTAgent configuration").blue().bold()
    );
    println!("Workspace: {}", workspace.display());
    println!("Force overwrite: {}", force);
    println!("Run after init: {}", run);

    // Bootstrap configuration files in the workspace
    VTAgentConfig::bootstrap_project(workspace, force)
        .with_context(|| "failed to initialize configuration files")?;

    if run {
        // After successful initialization, launch a chat session using default config
        let config = CoreAgentConfig {
            model: String::new(),
            api_key: String::new(),
            workspace: workspace.to_path_buf(),
            verbose: false,
        };
        handle_chat_command(&config, false)
            .await
            .with_context(|| "failed to start chat session")?;
    }

    Ok(())
}

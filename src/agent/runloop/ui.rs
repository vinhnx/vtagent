use anyhow::Result;
use chrono::Local;
use sysinfo::System;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;
use vtagent_core::tool_policy::{ToolPolicy, ToolPolicyManager};
use vtagent_core::ui::theme;
use vtagent_core::utils::ansi::{AnsiRenderer, MessageStyle};

use super::welcome::SessionBootstrap;

pub(crate) fn render_session_banner(
    renderer: &mut AnsiRenderer,
    config: &CoreAgentConfig,
    session_bootstrap: &SessionBootstrap,
) -> Result<()> {
    const VT_ASCII: &[&str] = &[
        r"__      _______          _____          _           ",
        r"\ \    / /__   __|        / ____|        | |          ",
        r" \ \  / /   | |   ______ | |     ___   __| | ___  ___ ",
        r"  \ \/ /    | |  |______|| |    / _ \ / _` |/ _ \/ __|",
        r"   \  /     | |          | |___| (_) | (_| |  __/\__ \",
        r"    \/      |_|           \_____/\___/ \__,_|\___||___/",
    ];

    for line in VT_ASCII {
        renderer.line(MessageStyle::Info, line)?;
    }
    renderer.line(MessageStyle::Output, "")?;

    renderer.line(MessageStyle::Info, "Interactive chat (tools)")?;
    renderer.line(MessageStyle::Output, &format!("Model: {}", config.model))?;
    renderer.line(
        MessageStyle::Output,
        &format!("Workspace: {}", config.workspace.display()),
    )?;
    renderer.line(
        MessageStyle::Output,
        &format!("Theme: {}", theme::active_theme_label()),
    )?;

    let now = Local::now();
    renderer.line(
        MessageStyle::Output,
        &format!("Local time: {}", now.format("%Y-%m-%d %H:%M:%S %Z")),
    )?;

    let mut sys = System::new_all();
    sys.refresh_all();
    let os_label = System::long_os_version()
        .or_else(System::name)
        .unwrap_or_else(|| "Unknown OS".to_string());
    let kernel = System::kernel_version().unwrap_or_else(|| "unknown".to_string());
    let cpu_count = System::physical_core_count()
        .unwrap_or_else(|| sys.cpus().len())
        .max(1);
    let total_mem_gb = sys.total_memory() as f64 / 1024.0 / 1024.0;
    let used_mem_gb = sys.used_memory() as f64 / 1024.0 / 1024.0;
    renderer.line(
        MessageStyle::Output,
        &format!("System: {} (kernel {})", os_label, kernel),
    )?;
    renderer.line(
        MessageStyle::Output,
        &format!(
            "Resources: {:.1} GB used / {:.1} GB total, {} cores",
            used_mem_gb, total_mem_gb, cpu_count
        ),
    )?;

    match ToolPolicyManager::new_with_workspace(&config.workspace) {
        Ok(manager) => {
            let summary = manager.get_policy_summary();
            let mut allow = 0usize;
            let mut prompt = 0usize;
            let mut deny = 0usize;
            for policy in summary.values() {
                match policy {
                    ToolPolicy::Allow => allow += 1,
                    ToolPolicy::Prompt => prompt += 1,
                    ToolPolicy::Deny => deny += 1,
                }
            }
            let policy_line = format!(
                "Tool policy: allow {}, prompt {}, deny {} ({})",
                allow,
                prompt,
                deny,
                manager.config_path().display()
            );
            renderer.line(MessageStyle::Output, &policy_line)?;
        }
        Err(err) => {
            renderer.line(
                MessageStyle::Error,
                &format!("Tool policy unavailable: {}", err),
            )?;
        }
    }

    if let Some(summary) = session_bootstrap.language_summary.as_deref() {
        renderer.line(
            MessageStyle::Output,
            &format!("Detected languages: {}", summary),
        )?;
    }

    renderer.line(MessageStyle::Output, "")?;

    Ok(())
}

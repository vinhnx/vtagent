//! VTAgent - Research-preview Rust coding agent
//!
//! Thin binary entry point that delegates to modular CLI handlers.

use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::PathBuf;
use vtagent_core::cli::args::{Cli, Commands};
use vtagent_core::config::api_keys::{ApiKeySources, get_api_key, load_dotenv};
use vtagent_core::config::loader::ConfigManager;
use vtagent_core::config::types::AgentConfig as CoreAgentConfig;

mod agent;
mod cli; // local CLI handlers in src/cli // agent runloops (single-agent only)

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env (non-fatal if missing)
    load_dotenv().ok();

    let args = Cli::parse();

    // Resolve workspace (default: current dir, canonicalized when present)
    let workspace_override = args
        .workspace_path
        .clone()
        .or_else(|| args.workspace.clone());

    let workspace = resolve_workspace_path(workspace_override)
        .context("Failed to resolve workspace directory")?;

    if let Some(path) = &args.workspace_path {
        if !workspace.exists() {
            bail!(
                "Workspace path '{}' does not exist. Initialize it first or provide an existing directory.",
                path.display()
            );
        }
    }

    cli::set_workspace_env(&workspace);

    // Load configuration (vtagent.toml or defaults) from resolved workspace
    let config_manager = ConfigManager::load_from_workspace(&workspace).with_context(|| {
        format!(
            "Failed to load vtagent configuration for workspace {}",
            workspace.display()
        )
    })?;
    let cfg = config_manager.config();

    // Resolve provider/model with CLI override
    let provider = args
        .provider
        .clone()
        .unwrap_or_else(|| cfg.agent.provider.clone());
    let model = args
        .model
        .clone()
        .unwrap_or_else(|| cfg.agent.default_model.clone());

    // Resolve API key for chosen provider
    let api_key = get_api_key(&provider, &ApiKeySources::default())
        .with_context(|| format!("API key not found for provider '{}'", provider))?;

    // Bridge to local CLI modules
    let core_cfg = CoreAgentConfig {
        model: model.clone(),
        api_key,
        workspace: workspace.clone(),
        verbose: args.verbose,
    };

    match &args.command {
        Some(Commands::ToolPolicy { command }) => {
            vtagent_core::cli::tool_policy_commands::handle_tool_policy_command(command.clone())
                .await?;
        }
        Some(Commands::Models { command }) => {
            vtagent_core::cli::models_commands::handle_models_command(&args, command).await?;
        }
        Some(Commands::Chat) => {
            cli::handle_chat_command(&core_cfg, args.skip_confirmations).await?;
        }
        Some(Commands::Ask { prompt }) => {
            cli::handle_ask_single_command(&core_cfg, prompt).await?;
        }
        Some(Commands::ChatVerbose) => {
            // Reuse chat path; verbose behavior is handled in the module if applicable
            cli::handle_chat_command(&core_cfg, args.skip_confirmations).await?;
        }
        Some(Commands::Analyze) => {
            cli::handle_analyze_command(&core_cfg).await?;
        }
        Some(Commands::Performance) => {
            cli::handle_performance_command().await?;
        }
        Some(Commands::Trajectory { file, top }) => {
            cli::handle_trajectory_logs_command(&core_cfg, file.clone(), *top).await?;
        }
        Some(Commands::CreateProject { name, features }) => {
            cli::handle_create_project_command(&core_cfg, name, features).await?;
        }
        Some(Commands::CompressContext) => {
            cli::handle_compress_context_command(&core_cfg).await?;
        }
        Some(Commands::Revert { turn, partial }) => {
            cli::handle_revert_command(&core_cfg, *turn, partial.clone()).await?;
        }
        Some(Commands::Snapshots) => {
            cli::handle_snapshots_command(&core_cfg).await?;
        }
        Some(Commands::CleanupSnapshots { max }) => {
            cli::handle_cleanup_snapshots_command(&core_cfg, Some(*max)).await?;
        }
        Some(Commands::Init) => {
            cli::handle_init_command(&workspace, false, false).await?;
        }
        Some(Commands::Config { output, global }) => {
            cli::handle_config_command(output.as_deref(), *global).await?;
        }
        Some(Commands::InitProject {
            name,
            force,
            migrate,
        }) => {
            cli::handle_init_project_command(name.clone(), *force, *migrate).await?;
        }
        Some(Commands::Benchmark) => {
            cli::handle_benchmark_command().await?;
        }
        Some(Commands::Man { command, output }) => {
            cli::handle_man_command(command.clone(), output.clone()).await?;
        }
        _ => {
            // Default to chat
            cli::handle_chat_command(&core_cfg, args.skip_confirmations).await?;
        }
    }

    Ok(())
}

fn resolve_workspace_path(workspace_arg: Option<PathBuf>) -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to determine current working directory")?;

    let mut resolved = match workspace_arg {
        Some(path) if path.is_absolute() => path,
        Some(path) => cwd.join(path),
        None => cwd,
    };

    if resolved.exists() {
        resolved = resolved.canonicalize().with_context(|| {
            format!(
                "Failed to canonicalize workspace path {}",
                resolved.display()
            )
        })?;
    }

    Ok(resolved)
}
